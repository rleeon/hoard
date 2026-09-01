//! `hoard sync run`: the CLI attached to the service (ADR 0021, Slice 4c).
//!
//! No engine lives here. It belongs to `hoardd`, one per user, resident, and
//! outliving the app, and this command does what the desktop does: ensure the
//! service and attach to its journal, only printing lines instead of drawing a
//! window.
//!
//! ## Attaching is following, not re-reading
//!
//! We subscribe from the cursor the daemon reports in the `Welcome`, so only what
//! happens from now on gets printed. The desktop does ask for the whole backlog,
//! and rightly: it has state on screen to rebuild. There is no state here, and
//! dumping the ring on start would only pass yesterday's history off as current.
//! (The gap between the `Welcome` and the `Subscribe` does travel: the cursor is
//! the `Welcome`'s, not the moment of subscribing.)
//!
//! ## Stopping this stops sync, and stopping sync stops this
//!
//! On a signal we send `Shutdown` over IPC. It is the exception to "closing a
//! client never kills the engine", and it is still deliberate: whoever types
//! `hoard sync run` and cuts it asked to stop syncing. Until Slice 4d this
//! process *was* the unit's `ExecStart`, so units installed by earlier versions
//! still depend on it (from 4d the unit points at `hoardd`, which handles the
//! signal itself).
//!
//! And the other way round, since 4d: if the service says goodbye
//! ([`hoard_core::ipc::ServerFrame::Goodbye`]) this command *ends* rather than
//! reconnecting. Carrying on reconnecting would relaunch it, since the reconnect
//! is spawn-if-absent, and that would undo the `hoard sync stop` somebody just
//! issued.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use std::io::{Read, Seek, SeekFrom};

use hoard_agent::agent::AgentEvent;
use hoard_agent::config::CliConfig;
use hoard_core::ipc::events::TooLargeKind;
use hoard_core::ipc::{DaemonStatus, Payload, Request};
use hoardd::client::Push;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};

use crate::commands::link;

/// Wait between attach retries. Normally the service stays alive and this never
/// gets used; it covers a restart (an update) without leaving the stream mute
/// forever.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// How long to wait for the engine before applying `--backup-only`. The service
/// resolves the session before it has an engine, so at boot the order arrives
/// first: without this wait the flag would be lost in silence and the service
/// would write to disk at exactly the moment the user asked it not to.
const BACKUP_ONLY_DEADLINE: Duration = Duration::from_secs(120);

/// Path of the sync service log written by *this* process
/// (`<state_dir>/logs/sync.log`). Windows' Task Scheduler throws stdout and
/// stderr away, so without a file there would be no trace of the events we print;
/// launchd redirects through the plist and systemd captures into journald, where
/// the extra file is harmless. The *service's* own log is a different one
/// (`hoardd.log`); see [`super::service`].
pub fn sync_log_path() -> Option<std::path::PathBuf> {
    CliConfig::state_dir()
        .ok()
        .map(|d| d.join("logs").join("sync.log"))
}

/// A non-blocking file writer mirroring `tracing` events to [`sync_log_path`],
/// plus the `WorkerGuard` that must outlive the process to flush on exit.
/// `None` if the log dir can't be resolved or created. Wired in by `main`
/// only when running as `hoard sync run`.
pub fn sync_log_writer() -> Option<(NonBlocking, WorkerGuard)> {
    let path = sync_log_path()?;
    let dir = path.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let appender = tracing_appender::rolling::never(dir, "sync.log");
    Some(tracing_appender::non_blocking(appender))
}

/// Reads the last `n` lines of `path`. Efficient for large files: only the
/// trailing 256 KiB is read and the partial line at the chunk boundary is
/// dropped. Portable (no `tail` subprocess) so it works on Windows too, and since
/// Slice 4c it is what `hoard sync logs` uses on every platform to tail the
/// service's own log.
pub(crate) fn tail_last_n_lines(path: &Path, n: usize) -> Result<Vec<String>> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let len = file
        .metadata()
        .with_context(|| format!("reading metadata for {}", path.display()))?
        .len();
    // Read only the trailing chunk so a multi-MB log doesn't load fully.
    const TAIL: u64 = 256 * 1024;
    let start = len.saturating_sub(TAIL);
    file.seek(SeekFrom::Start(start))
        .with_context(|| format!("seeking {}", path.display()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .with_context(|| format!("reading {}", path.display()))?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.lines().collect();
    // If we sliced into the middle of the file, the first line is likely a
    // partial fragment, so drop it and return whole lines only.
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let drop = lines.len().saturating_sub(n);
    Ok(lines[drop..].iter().map(|s| s.to_string()).collect())
}

/// `hoard sync run`: ensures the service and follows its journal until we are
/// told to stop. `backup_only` asks the engine not to restore and not to open the
/// pull paths, so it only uploads and never writes to your disk.
pub async fn run(backup_only: bool) -> Result<()> {
    let mut commands = link::ensure("commands").await?;
    let welcome = commands.welcome().clone();
    println!(
        "hoard sync · service {} · pid {}",
        welcome.daemon_version, welcome.pid
    );
    match link::ask(&mut commands, Request::Status).await {
        Ok(Payload::Status(status)) => print_engine(&status),
        Ok(other) => println!("  (unexpected answer to status: {other:?})"),
        Err(err) => println!("  couldn't read the service status: {err:#}"),
    }

    if backup_only {
        // A flag from this process over an engine that belongs to another: it
        // applies as soon as there is an engine and is undone on stop, since we
        // stop the service on the way out.
        tokio::spawn(apply_backup_only());
        println!("  backup only: never restores or writes to your disk");
    }
    println!("Ctrl-C (or `hoard sync stop`) to stop.\n");

    // Two ways to end, and only one of them stops the service. If we are stopped
    // (Ctrl-C, or the `systemctl --user stop` of an inherited unit still pointing
    // at this command) the user asked to stop syncing; if it is the service that
    // stops, it is already stopped and there is nothing to ask it.
    let stop_the_service = tokio::select! {
        _ = shutdown_signal() => true,
        // Only returns when the service says goodbye; a dropped connection
        // retries on its own, because the service can restart (an update) with
        // this process having no other way to find out.
        _ = follow() => false,
    };

    if stop_the_service {
        println!("\nstopping the Hoard service…");
        stop_service().await;
    }
    Ok(())
}

/// The engine's status line inside the service. An engine that is down *with a
/// reason* is diagnosable; without one it is the invisible failure D.11 and D.12
/// cost.
fn print_engine(status: &DaemonStatus) {
    if status.engine.running {
        println!(
            "  engine up · {} save(s) · {}",
            status.slots.len().max(status.engine.watched),
            status.engine.server.as_deref().unwrap_or("unknown server")
        );
    } else {
        println!(
            "  engine down · {}",
            status
                .engine
                .last_error
                .as_deref()
                .unwrap_or("still starting")
        );
    }
}

/// Asks the engine for upload-only mode as soon as one exists. See
/// [`BACKUP_ONLY_DEADLINE`]: at boot the engine arrives after we do, and a flag
/// silently lost here means writing to the user's disk against what they asked
/// for.
async fn apply_backup_only() {
    let deadline = Instant::now() + BACKUP_ONLY_DEADLINE;
    loop {
        if let Some(mut client) = link::attached("backup-only").await {
            let sent = link::ask(&mut client, Request::SetAutoRestore { enabled: false })
                .await
                .and(link::ask(&mut client, Request::SetGlobalSync { enabled: false }).await);
            match sent {
                Ok(_) => return,
                Err(err) => {
                    if Instant::now() >= deadline {
                        eprintln!(
                            "warning: couldn't put the service in backup-only mode ({err:#}). \
                             It may restore saves to this machine."
                        );
                        return;
                    }
                }
            }
        } else if Instant::now() >= deadline {
            eprintln!("warning: couldn't reach the service to set backup-only mode.");
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Follows the service's journal and prints what arrives. Only returns when the
/// service says goodbye: a dropped connection is retried, because the service can
/// restart (an update) with this process having no other way to find out.
async fn follow() {
    loop {
        match follow_once().await {
            Ok(Followed::Farewell) => return,
            Ok(Followed::Disconnected) => {
                eprintln!("warning: the Hoard service closed the connection; reconnecting…")
            }
            Err(err) => eprintln!("warning: lost the Hoard service ({err:#}); reconnecting…"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Why the following ended.
enum Followed {
    /// The connection dropped, so retry.
    Disconnected,
    /// The service said it was being stopped. Nothing to follow, nothing to
    /// relaunch.
    Farewell,
}

async fn follow_once() -> Result<Followed> {
    let mut events = link::ensure("events").await?;
    // From the `Welcome`'s cursor: follow, don't re-read. Whatever happens
    // between the greeting and the subscription does come in the backlog, so
    // there is no gap.
    let since = events.welcome().cursor;
    let backlog = link::ask(&mut events, Request::Subscribe { since: Some(since) }).await?;
    if let Payload::Backlog(backlog) = backlog {
        for entry in backlog.entries {
            print_event(&entry.event);
        }
    }
    while let Some(push) = events.next_push().await? {
        match push {
            Push::Event(entry) => print_event(&entry.event),
            // We fell behind and the channel dropped rows. The daemon owns up to
            // it rather than leaving the gap invisible; for a terminal stream,
            // saying so and carrying on from the new cursor is enough.
            Push::Resync { cursor, dropped } => {
                eprintln!("warning: fell behind the service's events ({dropped} dropped)");
                let _ = link::ask(
                    &mut events,
                    Request::Subscribe {
                        since: Some(cursor),
                    },
                )
                .await?;
            }
            // Stopped on purpose. Carrying on reconnecting would mean waiting for
            // events from a sync that is not running, and relaunching it (our
            // reconnect is spawn-if-absent) would undo a `hoard sync stop`.
            Push::Goodbye { reason } => {
                println!("the Hoard service stopped ({reason}).");
                return Ok(Followed::Farewell);
            }
        }
    }
    Ok(Followed::Disconnected)
}

fn print_event(event: &AgentEvent) {
    if let Some(line) = render(event) {
        println!("{line}");
    }
}

/// Stops the service. A fresh connection on purpose: if the daemon restarted
/// while we were following its journal, the order has to reach the one running
/// now. It does not start it: bringing a service up in order to stop it makes no
/// sense.
async fn stop_service() {
    let Some(mut client) = link::attached("stop").await else {
        println!("the Hoard service wasn't running.");
        return;
    };
    match link::ask(&mut client, Request::Shutdown).await {
        Ok(_) => println!("stopped."),
        Err(err) => eprintln!("warning: the service didn't acknowledge the stop: {err:#}"),
    }
}

/// Resolves when the process is asked to stop: Ctrl-C anywhere, plus SIGTERM on
/// unix, the signal `systemctl --user stop` and `launchctl bootout` send. Without
/// the SIGTERM arm the service manager would have to SIGKILL us, skipping the
/// service's clean shutdown.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Human-readable line for an agent event. `None` means an internal event not
/// worth showing (scheduled, skipped, heavy process).
fn render(ev: &AgentEvent) -> Option<String> {
    use AgentEvent::*;
    Some(match ev {
        GameStarted { game_slug, .. } => format!("▶  {game_slug} running"),
        GameStopped { game_slug, .. } => format!("■  {game_slug} closed"),
        BackupStarted { label, .. } => format!("…  backing up {label}"),
        // `already_landed`: the content already was the server's head, so nothing
        // was uploaded (D.8.3). Saying "backup 0B" would be confusing; what
        // happened is that it was not needed.
        BackupSuccess {
            version_num,
            already_landed: true,
            ..
        } => format!("✓  already on the server as v{version_num}"),
        BackupSuccess {
            version_num,
            total_bytes,
            ..
        } => format!("✓  backup v{version_num} ({})", fmt_bytes(*total_bytes)),
        BackupFailed {
            game_slug,
            error,
            will_retry,
            ..
        } => format!(
            "✗  {game_slug} failed: {error}{}",
            if *will_retry { " (retrying)" } else { "" }
        ),
        BackupThrottled {
            game_slug,
            retry_after_secs,
            ..
        } => format!("⏱  {game_slug} waiting {retry_after_secs}s (bandwidth limit)"),
        BackupTooLarge {
            game_slug, kind, ..
        } => match kind {
            // Naming the wrong limit costs the user a search: a self-hoster's
            // knob is `storage.max_snapshot_size_mb`, and a proxy's body limit
            // isn't Hoard's at all.
            TooLargeKind::ServerLimit => format!(
                "✗  {game_slug} exceeds your server's per-snapshot limit \
                 (storage.max_snapshot_size_mb)"
            ),
            TooLargeKind::Proxy => format!(
                "✗  {game_slug} was refused as too large by something in front \
                 of your server (a reverse proxy or tunnel body-size limit)"
            ),
            TooLargeKind::PlanCap => format!("✗  {game_slug} exceeds your plan's limit"),
        },
        SaveAutoRestored {
            game_slug,
            version_num,
            files_extracted,
            ..
        } => format!("↺  {game_slug} restored v{version_num} ({files_extracted} files)"),
        SaveAutoRestoreFailed {
            game_slug, error, ..
        } => format!("✗  {game_slug} auto-restore failed: {error}"),
        SaveConflictsBackedUp { .. } => {
            "⚠  conflict: local copy saved before applying the remote".to_string()
        }
        RestoreDeferred { game_slug, .. } => {
            format!("⏸  {game_slug} update ready — waiting for the game to close")
        }
        SaveAutoRestoreStuck {
            game_slug,
            failures,
            error,
            ..
        } => format!("⚠  {game_slug}: cloud restore failing repeatedly ({failures}×) — {error}"),
        SaveAutoRestoreRecovered { game_slug, .. } => {
            format!("✓  {game_slug}: cloud restore working again")
        }
        _ => return None,
    })
}

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2}G", b / GB)
    } else if b >= MB {
        format!("{:.2}M", b / MB)
    } else if b >= KB {
        format!("{:.2}K", b / KB)
    } else {
        format!("{b}B")
    }
}

#[cfg(test)]
mod tests {
    use super::tail_last_n_lines;

    #[test]
    fn tail_returns_last_n_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.log");
        std::fs::write(&path, "one\ntwo\nthree\nfour\nfive\n").unwrap();
        let lines = tail_last_n_lines(&path, 3).unwrap();
        assert_eq!(
            lines,
            vec!["three".to_string(), "four".into(), "five".into()]
        );
    }

    #[test]
    fn tail_fewer_lines_than_n_returns_all() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.log");
        std::fs::write(&path, "only\n").unwrap();
        let lines = tail_last_n_lines(&path, 80).unwrap();
        assert_eq!(lines, vec!["only".to_string()]);
    }

    #[test]
    fn tail_drops_partial_first_line_on_large_file() {
        // Write well over the 256 KiB tail window so the read slices mid-file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.log");
        let mut content = String::from("header\n");
        for i in 0..1000u32 {
            content.push_str(&format!("{i:0300}\n"));
        }
        std::fs::write(&path, &content).unwrap();
        let lines = tail_last_n_lines(&path, 2).unwrap();
        // The last two whole lines are the last two indices (998, 999). The
        // numbers are zero-padded on the left, so the index sits at the end.
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with("999"));
        assert!(lines[0].ends_with("998"));
    }

    #[test]
    fn tail_handles_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.log");
        std::fs::write(&path, "a\nb\nc\n").unwrap();
        let lines = tail_last_n_lines(&path, 2).unwrap();
        assert_eq!(lines, vec!["b".to_string(), "c".into()]);
    }
}
