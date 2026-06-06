//! Background schedulers that back the sidebar's "Modo Automático" toggle.
//!
//! When the user flips the toggle on we persist `prefs.automatic_mode = true`
//! and start **two** independent Tokio tickers, because the work splits into a
//! cheap half and an expensive half:
//!
//! * **Scan** (`automatic-scan-tick`, default every 5 min) — a metadata-only
//!   disk walk that detects newly installed games and tracks the
//!   high-confidence ones. No file bytes read; safe to run often.
//! * **Backup sweep** (`automatic-backup-tick`, default every 1 h) — re-hashes
//!   tracked saves to catch changes the fs-watcher missed. Reads file bytes,
//!   so it's the costly half; the agent staggers the per-save work across an
//!   effective window so disk use never bursts.
//!
//! Each tick emits its Tauri event; the frontend's `initAutomaticListener()`
//! (in `lib/stores/automatic.ts`) reacts — scan-tick runs detection+tracking,
//! backup-tick fires the staggered sweep. We keep the heavy lifting in the UI
//! layer for the scan (it reads the catalog cache, dispatches toasts, boots the
//! live agent through stores that don't exist on the Rust side); the sweep's
//! staggering lives in the agent.
//!
//! Lifecycle:
//! * `start(app, scan_secs, backup_secs)` aborts any previous handles and
//!   spawns fresh tasks. Both fire **immediately** so flipping the toggle on
//!   (or saving a new interval) does something visible right away, then settle
//!   onto full-interval boundaries.
//! * `stop(app)` aborts both handles.
//! * `restart_if_enabled(app)` rehydrates from prefs on launch so a user who
//!   left the toggle on across reboots gets their schedulers back.
//!
//! The scheduler lives as a `tauri::State` singleton so the JoinHandles
//! survive between Tauri commands.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;
use tokio::time::interval;

use hoard_agent::prefs::Prefs;

/// Lower bounds so a hand-edited `prefs.json` can't spin a pathologically
/// tight loop. The scan is cheap but still touches the disk; the sweep is
/// expensive. These are floors, not defaults (defaults live in `prefs.rs`:
/// 300s scan / 3600s sweep).
const MIN_SCAN_INTERVAL_SECS: u64 = 30;
const MIN_BACKUP_INTERVAL_SECS: u64 = 60;

/// Managed singleton holding the currently-active scheduler tasks, if any.
/// Registered with `app.manage(AutomaticScheduler::default())` during Tauri
/// setup. One instance lives for the whole app lifetime; we mutate the inner
/// `Option<JoinHandle>`s to swap tasks on toggle/interval changes.
#[derive(Default)]
pub struct AutomaticScheduler {
    scan_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    backup_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Cancel any in-flight scheduler tasks and start fresh ones: the detection
/// scan every `scan_interval_secs`, the backup sweep every
/// `backup_interval_secs`. Safe to call repeatedly — each call cleanly
/// replaces the previous handles.
pub fn start(app: &AppHandle, scan_interval_secs: u64, backup_interval_secs: u64) {
    let scheduler = app.state::<AutomaticScheduler>();
    {
        let mut s = scheduler.scan_handle.lock().unwrap();
        if let Some(prev) = s.take() {
            prev.abort();
        }
        let mut b = scheduler.backup_handle.lock().unwrap();
        if let Some(prev) = b.take() {
            prev.abort();
        }
    }

    let scan_secs = scan_interval_secs.max(MIN_SCAN_INTERVAL_SECS);
    let backup_secs = backup_interval_secs.max(MIN_BACKUP_INTERVAL_SECS);
    tracing::info!(
        scan_secs,
        backup_secs,
        "automatic mode: starting scan + backup schedulers"
    );

    let scan = spawn_ticker(app.clone(), "automatic-scan-tick", scan_secs);
    let backup = spawn_ticker(app.clone(), "automatic-backup-tick", backup_secs);

    *scheduler.scan_handle.lock().unwrap() = Some(scan);
    *scheduler.backup_handle.lock().unwrap() = Some(backup);
}

/// Spawn one ticker task. Emits `event` immediately (so toggling on produces
/// a visible effect right away), then drives a `tokio::time::interval`,
/// consuming its built-in zero-delay first tick so subsequent emits land on
/// full-period boundaries.
fn spawn_ticker(app: AppHandle, event: &'static str, period_secs: u64) -> JoinHandle<()> {
    let period = Duration::from_secs(period_secs);
    tokio::task::spawn(async move {
        if let Err(e) = app.emit(event, ()) {
            tracing::warn!(error = %e, event, "automatic mode: couldn't emit initial tick");
        }
        let mut ticker = interval(period);
        ticker.tick().await; // consume the immediate first tick (we just fired manually)
        loop {
            ticker.tick().await;
            if let Err(e) = app.emit(event, ()) {
                tracing::warn!(error = %e, event, "automatic mode: couldn't emit tick");
            }
        }
    })
}

/// Abort both running schedulers, if any. No-op when nothing is scheduled —
/// safe to call from any wind-down path (toggle off, app shutdown).
pub fn stop(app: &AppHandle) {
    let scheduler = app.state::<AutomaticScheduler>();
    let mut stopped = false;
    if let Some(prev) = scheduler.scan_handle.lock().unwrap().take() {
        prev.abort();
        stopped = true;
    }
    if let Some(prev) = scheduler.backup_handle.lock().unwrap().take() {
        prev.abort();
        stopped = true;
    }
    if stopped {
        tracing::info!("automatic mode: schedulers stopped");
    }
}

/// Re-arm the schedulers if the user had Modo Automático enabled before the
/// app last closed. Called from the Tauri `setup` closure once the
/// `AutomaticScheduler` state is managed. Errors are non-fatal — the toggle
/// still shows the persisted value and the user can re-trigger by flipping it.
pub async fn restart_if_enabled(app: &AppHandle) -> anyhow::Result<()> {
    let (prefs, _) = Prefs::load_default()?;
    if prefs.automatic_mode {
        tracing::info!(
            scan_secs = prefs.automatic_scan_interval_secs,
            backup_secs = prefs.automatic_backup_interval_secs,
            "automatic mode: rehydrating schedulers from prefs"
        );
        start(
            app,
            prefs.automatic_scan_interval_secs,
            prefs.automatic_backup_interval_secs,
        );
    }
    Ok(())
}
