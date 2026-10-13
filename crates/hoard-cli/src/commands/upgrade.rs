//! `hoard upgrade`: brings everything installed up to the latest release,
//! together.
//!
//! It does not update "the CLI": it updates the components the manifest
//! (`hoard_agent::install::Manifest`) says exist on this machine, all to the same
//! version. The same operation as `hoard install`, seen from afterwards: there it
//! decides what is needed, here it relieves what was already there.
//!
//! The core lives in one place only (`hoard_agent::install::layout`), so there is
//! no question of which copy to relieve: the one there, and whatever older copy
//! an earlier release left beside it comes off on the way.

use std::io::IsTerminal;

use anyhow::{bail, Result};

use hoard_agent::update;
use hoard_core::ipc::{UpdatePhase, UpdateState};
use hoardd::setup::{Plan, Source, Want};

/// `hoard upgrade` (no args): check, then upgrade only if there is something
/// newer. `--version` pins a specific release and always runs the installer, so
/// you can re-install or roll back.
///
/// Normally there is nothing to do here. Since the service updates itself
/// (`hoardd::updater`), this command is mostly the way to *not wait*: if something
/// is downloaded, it tells the service to apply it now, and with a person present
/// it can also open the privilege dialog the background cycle cannot. Only when
/// there is no service to ask does it fall back to the usual installer.
pub async fn run(version: Option<String>) -> Result<()> {
    let current = update::current();

    // Pinned: skip the "is there anything new" check, since the user asked for a
    // specific version explicitly (install, reinstall, downgrade). It does not go
    // through the service: the service only knows how to move forward, and this is
    // also how you go back.
    if let Some(v) = version {
        println!("hoard {current} → {v} (pinned)");
        return install(Some(&v)).await;
    }

    // The service holds every update back until an install from an older
    // release is moved, and what it reports meanwhile ("not downloaded yet",
    // "hasn't been able to check") would have this run again for nothing.
    if let Some(why) = repair_pending().await {
        println!("hoard {current} — this install has to be moved before it can update:");
        println!("  {why}");
        println!("Run `hoard install` to do it now; the service also does it on its own.");
        return Ok(());
    }

    if let Some(state) = crate::commands::link::update_state().await {
        return through_the_service(current, state).await;
    }

    println!("hoard {current} — checking for updates…");
    match update::fetch_latest().await {
        Some(latest) if update::is_newer(&latest, current) => {
            println!("new version available: {latest}\n");
            install(Some(&latest)).await
        }
        Some(latest) => {
            println!("already up to date (latest is {latest}).");
            Ok(())
        }
        None => {
            // Couldn't reach GitHub. Don't guess: tell the user and let them
            // force it if they want.
            bail!(
                "couldn't check the latest version (no network, or GitHub is \
                 unreachable). Retry, or force a reinstall with \
                 `hoard upgrade --version <x.y.z>`."
            );
        }
    }
}

/// The normal path: the service already knows what there is and has it
/// downloaded.
async fn through_the_service(current: &str, state: UpdateState) -> Result<()> {
    let Some(latest) = state.latest.clone() else {
        println!("hoard {current} — the sync service hasn't been able to check yet.");
        return Ok(());
    };
    if !update::is_newer(&latest, current) {
        println!("hoard {current} — already up to date.");
        return Ok(());
    }

    match state.phase {
        UpdatePhase::Managed => {
            println!("hoard {current} → {latest} is out, but this install is managed by your");
            println!("package manager. Update it with that.");
            return Ok(());
        }
        UpdatePhase::Downloading => {
            println!("hoard {current} → {latest} — the service is downloading it now.");
            println!("It applies itself when it's ready; nothing to do.");
            return Ok(());
        }
        UpdatePhase::Restarting => {
            println!("hoard {latest} is installed — the service is restarting on it.");
            return Ok(());
        }
        _ => {}
    }

    if state.staged.is_none() {
        println!("hoard {current} → {latest} — not downloaded yet.");
        println!("The service fetches it in the background; run this again in a minute.");
        return Ok(());
    }

    println!("hoard {current} → {latest} — applying the staged update…");
    let after = crate::commands::link::apply_update(Some(latest.clone())).await?;
    // Applying carries on after the service answers: a native installer takes
    // time, and a `pkexec` takes as long as the human does. We poll the state
    // rather than leave the request hanging and blocking the rest of this
    // connection.
    match wait_for(&latest).await {
        Applied::Done => {
            println!("\n✓ hoard {latest} installed. The service is restarting on it.");
            println!("Run `hoard --version` in a moment to confirm.");
            Ok(())
        }
        Applied::Failed(reason) => bail!(
            "the service couldn't apply it: {reason}\n\
             Try `hoard upgrade --version {latest}` to run the installer yourself."
        ),
        Applied::StillGoing => {
            println!("\nStill installing. Watch it with `hoard sync logs`.");
            let _ = after;
            Ok(())
        }
    }
}

/// What keeps the service from updating this install, when it is the same
/// thing that keeps it in `hoardd::updater`: a record of ours that points
/// outside the layout, or a service running a `hoardd` other than the core.
async fn repair_pending() -> Option<String> {
    let health = hoardd::setup::health().await;
    let manifest = health.manifest.as_ref()?;
    if !manifest.delivery.is_none_or(|d| d.is_ours()) {
        return None;
    }
    (!manifest.is_in_layout() || health.unit_elsewhere).then(|| health.describe())
}

/// How the wait ended.
enum Applied {
    Done,
    Failed(String),
    StillGoing,
}

/// Polls the service until the update reaches an outcome.
///
/// The cap is not generous for its own sake: underneath it there is a `dpkg` or a
/// polkit dialog waiting for somebody to type their password, and cutting that off
/// after ten seconds with a "it failed" would be lying about something that is
/// going fine.
async fn wait_for(version: &str) -> Applied {
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(180);
    const TICK: std::time::Duration = std::time::Duration::from_secs(2);
    let started = std::time::Instant::now();

    while started.elapsed() < DEADLINE {
        tokio::time::sleep(TICK).await;
        let Some(state) = crate::commands::link::update_state().await else {
            // The service stopped answering. With an update just applied that is
            // expected: it is relieving itself with the new binary, and the
            // socket drops meanwhile.
            return Applied::Done;
        };
        match state.phase {
            UpdatePhase::Restarting | UpdatePhase::UpToDate => return Applied::Done,
            UpdatePhase::Failed => {
                return Applied::Failed(
                    state
                        .last_error
                        .unwrap_or_else(|| "no reason given".to_string()),
                )
            }
            // It applied, but another one shipped meanwhile: there is still work
            // to do, and it is not this one's failure.
            UpdatePhase::Ready if state.staged.as_deref() != Some(version) => return Applied::Done,
            _ => {}
        }
    }
    Applied::StillGoing
}

/// Puts every installed component at `version`, the same way Hoard Setup does.
///
/// The pin stops being optional in practice even though the signature allows it:
/// if the core resolved against "latest" and the app against a specific number, a
/// release shipping between the two downloads would be enough to leave the machine
/// with pieces from different versions. One number for everything.
async fn install(version: Option<&str>) -> Result<()> {
    let outcome = hoardd::setup::run(Plan {
        version: version.map(str::to_string),
        want: Want::Keep,
        source: Source::Download,
        interactive: std::io::stdin().is_terminal(),
        setup_binary: None,
        app_file: None,
    })
    .await?;
    println!(
        "\n✓ hoard {} installed; the service is running on it.",
        outcome.version
    );
    if let Some(app) = &outcome.app {
        println!("✓ desktop app upgraded alongside it.");
        if outcome.app_was_running {
            hoardd::setup::reopen_app(app);
        }
    }
    for warning in &outcome.warnings {
        eprintln!("warning: {warning}");
    }
    for item in &outcome.left {
        eprintln!("still here, needs an administrator: {item}");
    }
    Ok(())
}
