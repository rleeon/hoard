//! `hoard upgrade`: brings everything installed up to the latest release,
//! together.
//!
//! It does not update "the CLI": it updates the components the manifest
//! (`hoard_agent::install::Manifest`) says exist on this machine, all to the same
//! version. The same operation as `hoard install`, seen from afterwards: there it
//! decides what is needed, here it relieves what was already there.
//!
//! ## Why it reads the manifest before touching anything
//!
//! The terminal installer leaves the core in a user directory (`~/.local/bin`); a
//! native package leaves it inside the app's bundle (`/usr/bin`). Re-running the
//! installer blindly in the second case updates nothing: it installs a *second*
//! core in the home that shadows the packaged one depending on `PATH` order, and
//! from then on which version runs depends on who started the process. It is the
//! same failure as the old `hoard-server` on `PATH`, which is why this asks first
//! who owns each piece.
//!
//! We do not overwrite our own running executable: the installer writes to the
//! standard directory and the new binary takes over on the next invocation.

use anyhow::{bail, Result};

use hoard_agent::install::{Component, Manifest};
use hoard_agent::update;
use hoard_core::ipc::{UpdatePhase, UpdateState};

/// Canonical installer host (same one printed by `install.sh`).
const BASE: &str = "https://hoard.services";

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

/// Relieves every installed component to `version`.
///
/// The pin stops being optional in practice even though the signature allows it:
/// if the core resolved against "latest" and the app against a specific number, a
/// release shipping between the two downloads would be enough to leave the machine
/// with pieces from different versions. One number for everything.
async fn install(version: Option<&str>) -> Result<()> {
    let manifest = Manifest::load_or_observe()?;

    // A core inside the app's bundle is relieved by the app's installer, not by
    // ours. Running the terminal installer here would not update the one running:
    // it would put another one beside it.
    if manifest.core_from_bundle {
        println!(
            "this core ships inside the desktop app ({}), so the app's own \
             installer owns it.",
            manifest
                .core_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("?"))
                .display()
        );
        return upgrade_desktop_only(&manifest, version).await;
    }

    println!("running the official installer from {BASE}…\n");

    let status = match installer_command(version).status() {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(missing_tool_hint()),
        Err(e) => return Err(e.into()),
    };

    if !status.success() {
        bail!(
            "the installer exited with {}. Nothing changed if it failed before \
             writing the binary; re-run `hoard upgrade` or install manually from {BASE}/cli.",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "a signal".into())
        );
    }

    println!("\n✓ core upgraded.");

    // Reload the resident daemon so it picks up the new binary. No-op (and no
    // noise) unless the sync service is actually installed here.
    crate::commands::service::reload_after_upgrade().await;

    // The installer ends by calling `hoard install`, which relieves the app if
    // there is one, so by the time we get here everything is at the same version
    // and all that is left is to say so.
    if manifest.has(Component::Desktop) {
        println!("✓ desktop app upgraded alongside it.");
    }
    println!("Run `hoard --version` to confirm.");
    Ok(())
}

/// The case where the core is not ours: the app gets relieved, and its bundle
/// brings the new core with it.
async fn upgrade_desktop_only(manifest: &Manifest, version: Option<&str>) -> Result<()> {
    let Some(delivery) = manifest.delivery else {
        bail!(
            "nothing here is ours to upgrade — this install is managed by your \
             package manager. Update it with that."
        );
    };
    if !delivery.is_ours() {
        bail!(
            "this install is managed by your package manager ({}). Update it with that.",
            delivery.as_str()
        );
    }
    let target = match version {
        Some(v) => v.trim_start_matches('v').to_string(),
        None => update::fetch_latest().await.ok_or_else(|| {
            anyhow::anyhow!("couldn't reach GitHub to resolve the latest version")
        })?,
    };
    crate::commands::install::run(crate::commands::install::Want::Detect, Some(target)).await
}

#[cfg(unix)]
fn installer_command(version: Option<&str>) -> std::process::Command {
    // Pipe the installer straight into a POSIX shell, same as
    // `curl -fsSL .../install.sh | sh`. `HOARD_VERSION` is read by the script.
    let mut script = format!("curl -fsSL {BASE}/install.sh | sh");
    if let Some(v) = version {
        script = format!("HOARD_VERSION={} {script}", shell_escape(v));
    }
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(script);
    cmd
}

#[cfg(not(unix))]
fn installer_command(version: Option<&str>) -> std::process::Command {
    // `irm .../install.ps1 | iex`, with the pin set as an env var beforehand.
    let mut ps = String::new();
    if let Some(v) = version {
        ps.push_str(&format!(
            "$env:HOARD_VERSION = '{}'; ",
            v.replace('\'', "''")
        ));
    }
    ps.push_str(&format!("irm {BASE}/install.ps1 | iex"));
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps]);
    cmd
}

/// Minimal single-quote escaping for a value passed to `sh -c`.
#[cfg(unix)]
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(unix)]
fn missing_tool_hint() -> String {
    format!("`sh` not found — can't run the installer. Install manually from {BASE}/cli.")
}

#[cfg(not(unix))]
fn missing_tool_hint() -> String {
    format!("`powershell` not found — can't run the installer. Install manually from {BASE}/cli.")
}
