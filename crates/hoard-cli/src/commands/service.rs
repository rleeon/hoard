//! `hoard sync`: the resident sync (the desktop app without a window), running as
//! a user service rather than as a foreground process.
//!
//! Since Slice 4d this file knows nothing about systemd, launchd or the Task
//! Scheduler: the unit is defined and managed by [`hoardd::autostart`], which is
//! where it belongs, since the desktop and the CLI install the same one and the
//! `ExecStart` is the service's binary. What stays here is what is proper to a
//! terminal: showing things (status, logs) and translating what the user types.
//!
//! What the user types (`start`, `stop`, `status`) governs the OS service
//! manager; what the *service* is doing is reported by the service over IPC.

use anyhow::{Context, Result};

use crate::commands::daemon;

#[derive(clap::Subcommand)]
pub enum SyncCommand {
    /// Install and start the sync service (runs now and at every login/boot)
    Start,
    /// Stop the service and remove it from autostart
    Stop,
    /// Restart the service
    Restart,
    /// Show the most recent service logs
    Logs,
    /// Follow the sync service's events in this terminal
    #[command(hide = true)]
    Run {
        /// Back up only: never restore or write to disk (global-sync off)
        #[arg(long)]
        backup_only: bool,
    },
}

/// `hoard sync [action]`. No action prints the status (like `systemctl status`).
pub async fn run(action: Option<SyncCommand>) -> Result<()> {
    match action {
        None => {
            // Paint the overall status panel first, then the service detail
            // below it: the banner gives cli, server, session and sync at a
            // glance.
            let _ = crate::commands::banner::show(false).await;
            service_detail().await;
            status().await
        }
        Some(SyncCommand::Start) => start().await,
        Some(SyncCommand::Stop) => stop().await,
        Some(SyncCommand::Restart) => restart().await,
        Some(SyncCommand::Logs) => {
            // Two halves, and both matter: the engine's diagnosis is in the
            // service's log (it runs detached when a client brings it up, so it
            // lands neither in the unit's journal nor in anybody's console), and
            // the chronicle of events is in what the client prints.
            service_logs();
            logs().await
        }
        // No longer the unit's `ExecStart` (that is `hoardd` since 4d): it stays
        // as the way to watch sync go by in a terminal, and as the command the
        // units installed by earlier versions point at.
        Some(SyncCommand::Run { backup_only }) => daemon::run(backup_only).await,
    }
}

/// `hoard sync start`: installs the unit and leaves the service running under it.
async fn start() -> Result<()> {
    let installed = hoardd::autostart::install()
        .await
        .context("installing the Hoard sync service")?;
    println!(
        "hoard sync started ({} · {}).",
        installed.manager, installed.id
    );
    if let Some(path) = installed.path {
        println!("  unit:   {}", path.display());
    }
    println!("  status: `hoard sync`   ·   logs: `hoard sync logs`");
    Ok(())
}

/// `hoard sync stop`: removes the autostart *and* stops the service.
///
/// Both steps, in this order. Removing the unit alone would leave `hoardd`
/// syncing (it outlives its clients by design, and a client may have started it
/// rather than the unit), so "stop" would have stopped meaning what it meant. And
/// the other way round: stopping the service without removing the unit would
/// resurrect it at the next login.
async fn stop() -> Result<()> {
    let removed = hoardd::autostart::uninstall()
        .await
        .context("removing the Hoard sync service")?;
    let was_running = stop_service().await;
    match (removed, was_running) {
        (true, _) => println!("hoard sync stopped and removed from autostart."),
        // No unit but a live service: a client brought it up (the app on open, a
        // `hoard track`). Stopping it is just as valid, but it is worth saying
        // there was nothing installed here to remove.
        (false, true) => println!("the Hoard service stopped (it wasn't set to start at login)."),
        (false, false) => println!("hoard sync wasn't running."),
    }
    Ok(())
}

/// Stops the service if it is up; returns whether there was one. The service
/// manager only kills the process it launched, and `hoardd` may have been brought
/// up by a client. It does not start it in order to stop it, obviously.
async fn stop_service() -> bool {
    let Some(mut client) = crate::commands::link::attached("stop").await else {
        return false;
    };
    if let Err(err) =
        crate::commands::link::ask(&mut client, hoard_core::ipc::Request::Shutdown).await
    {
        eprintln!("warning: the Hoard service didn't acknowledge the stop: {err:#}");
    }
    true
}

async fn restart() -> Result<()> {
    let installed = hoardd::autostart::restart()
        .await
        .context("restarting the Hoard sync service")?;
    println!(
        "hoard sync restarted ({} · {}).",
        installed.manager, installed.id
    );
    Ok(())
}

/// Bounce the resident sync service after `hoard upgrade` has swapped the binary,
/// so the daemon re-execs the new code. Best-effort and conservative:
/// - only when the OS service is actually installed on this machine, since an
///   upgrade must never install or start sync as a side effect;
/// - a restart hiccup is a warning, not a failure: the upgrade already succeeded,
///   so we never return `Err` here.
pub async fn reload_after_upgrade() {
    if !hoardd::autostart::installed().await {
        return;
    }
    println!("reloading the sync service to run the new binary…");
    if let Err(e) = restart().await {
        eprintln!("warning: couldn't restart the sync service automatically: {e:#}");
        eprintln!("  restart it yourself with `hoard sync restart`.");
    }
}

/// What the *service* says about itself, which is different from what the service
/// manager knows: the manager only knows the process it launched, and `hoardd` may
/// have been brought up by a client. It does not start it; a status panel that
/// brings a service up would be the worst possible side effect.
async fn service_detail() {
    let Some(status) = crate::commands::link::status().await else {
        println!("  service: not running");
        return;
    };
    println!(
        "  service: hoardd {} · pid {} · up {}",
        status.daemon_version,
        status.pid,
        fmt_uptime(status.uptime_secs)
    );
    if status.engine.running {
        println!(
            "  engine:  up · {} save(s) · {}",
            status.slots.len().max(status.engine.watched),
            status.engine.server.as_deref().unwrap_or("unknown server")
        );
    } else {
        // An engine that is down *with a reason* is diagnosable; without one it
        // is the invisible failure that cost two sessions (D.11, D.12).
        println!(
            "  engine:  down · {}",
            status
                .engine
                .last_error
                .as_deref()
                .unwrap_or("still starting")
        );
    }
    // Who sends the native notices. Without this line, "nothing reaches me with
    // the app closed" cannot be told from "I have them switched off in Settings".
    println!(
        "  notify:  {}",
        if status.notifications {
            "the service sends them (even with the app closed)"
        } else {
            "the app sends them while it's open (no service backend on this OS yet)"
        }
    );
}

/// The last lines of `hoardd`'s log. It is the log that matters since Slice 4c:
/// when a client brings it up, the service is launched detached (its own session,
/// stdio to `null`), so its output appears neither in the unit's journal nor in
/// the terminal that started it, only in its file.
fn service_logs() {
    let Ok(path) = hoard_agent::config::CliConfig::logs_dir().map(|d| d.join("hoardd.log")) else {
        return;
    };
    if !path.exists() {
        println!("no service log yet at {}", path.display());
        return;
    }
    println!("── service · {} ──", path.display());
    match daemon::tail_last_n_lines(&path, 40) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(err) => eprintln!("warning: couldn't read the service log: {err:#}"),
    }
    println!();
}

fn fmt_uptime(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h{:02}m", s / 3600, (s % 3600) / 60),
        s => format!("{}d{:02}h", s / 86_400, (s % 86_400) / 3600),
    }
}

/// Run a command inheriting our stdio; return whether it succeeded.
async fn run_status(program: &str, args: &[&str]) -> Result<bool> {
    let st = tokio::process::Command::new(program)
        .args(args)
        .status()
        .await
        .with_context(|| format!("running `{program}`"))?;
    Ok(st.success())
}

// ---- what only makes sense in a terminal: showing the service manager's status
// and logs, exactly as it gives them
// =======================================================================

#[cfg(target_os = "linux")]
async fn status() -> Result<()> {
    if !hoardd::autostart::installed().await {
        println!("hoard sync is not installed. Run `hoard sync start`.");
        return Ok(());
    }
    // `status` exits non-zero when the unit is inactive; that's not our error.
    let _ = run_status(
        "systemctl",
        &["--user", "status", hoardd::autostart::UNIT_ID, "--no-pager"],
    )
    .await;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn logs() -> Result<()> {
    let _ = run_status(
        "journalctl",
        &[
            "--user",
            "-u",
            hoardd::autostart::UNIT_ID,
            "-n",
            "80",
            "--no-pager",
        ],
    )
    .await;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn status() -> Result<()> {
    if !hoardd::autostart::installed().await {
        println!("hoard sync is not installed. Run `hoard sync start`.");
        return Ok(());
    }
    let out = tokio::process::Command::new("id")
        .arg("-u")
        .output()
        .await
        .context("running `id -u`")?;
    let uid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let target = format!("gui/{uid}/{}", hoardd::autostart::UNIT_ID);
    let _ = run_status("launchctl", &["print", &target]).await;
    Ok(())
}

#[cfg(target_os = "macos")]
async fn logs() -> Result<()> {
    let log = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|h| h.join("Library").join("Logs").join("hoard-sync.log"));
    match log {
        Some(path) if path.exists() => {
            let _ = run_status("tail", &["-n", "80", &path.to_string_lossy()]).await;
        }
        Some(path) => println!("no logs yet at {}", path.display()),
        None => println!("no HOME in the environment"),
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn status() -> Result<()> {
    if hoardd::autostart::installed().await {
        let _ = run_status(
            "schtasks",
            &[
                "/Query",
                "/TN",
                hoardd::autostart::UNIT_ID,
                "/V",
                "/FO",
                "LIST",
            ],
        )
        .await;
        return Ok(());
    }
    // No installed task, which no longer means "no sync": the service may be up
    // because the desktop app (or a `hoard track`) asked for it. `service_detail`
    // above already printed what it's doing, so only say what's missing here.
    println!("hoard sync isn't installed as a logon task. Run `hoard sync start`.");
    Ok(())
}

#[cfg(target_os = "windows")]
async fn logs() -> Result<()> {
    // `hoard sync run` writes the events it prints to this file (Task Scheduler
    // drops stdout/stderr), so it's the client half of the story; the service's
    // own half went above.
    if let Some(path) = daemon::sync_log_path() {
        if path.exists() {
            for line in daemon::tail_last_n_lines(&path, 80)? {
                println!("{line}");
            }
            return Ok(());
        }
    }
    println!("no client logs yet at the expected path.");
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
async fn status() -> Result<()> {
    println!("no service backend for this OS — run `hoardd` manually.");
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
async fn logs() -> Result<()> {
    anyhow::bail!("no service backend for this OS")
}
