//! Background schedulers that back the sidebar's "Modo Automático" toggle.
//!
//! When the user flips the toggle on we persist `prefs.automatic_mode = true`
//! and start **two** independent Tokio tickers, because the work splits into a
//! cheap half and an expensive half:
//!
//! * **Scan** (default every 5 min) — a metadata-only disk walk that detects
//!   newly installed games and tracks the high-confidence ones, then boots the
//!   agent so they're watched. No file bytes read; safe to run often.
//! * **Backup sweep** (default every 1 h) — re-hashes tracked saves to catch
//!   changes the fs-watcher missed. Reads file bytes, so it's the costly half;
//!   the agent staggers the per-save work across an effective window so disk
//!   use never bursts.
//!
//! **Both halves run entirely in Rust** ([`run_scan`] / [`run_backup_sweep`]).
//! This used to live in the frontend (`automatic.ts` reacted to per-tick Tauri
//! events), but that only worked while the WebView was alive — exactly *not*
//! the case while the user is gaming with the window minimized to the tray
//! (WebView2 suspends background windows). The symptom was "nothing got
//! monitored no matter how long passed". Driving the work from the Tokio
//! tickers keeps it running headless. The UI is now a pure observer: we emit
//! `automatic-phase` (progress for the sidebar pulse) and
//! `automatic-scan-complete` (so it can toast + refresh the library).
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
use std::time::{Duration, Instant};

use hoard_agent::detection::Confidence;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use hoard_agent::prefs::Prefs;

use crate::state::AppState;

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
    /// When the last event-driven scan (`request_scan`) fired. Debounces a
    /// burst of `HeavyProcessDetected` signals into a single pass. Independent
    /// of the periodic ticker, which keeps its own cadence.
    last_event_scan: Arc<Mutex<Option<Instant>>>,
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

    let scan = spawn_worker(app.clone(), TickKind::Scan, scan_secs);
    let backup = spawn_worker(app.clone(), TickKind::Backup, backup_secs);

    *scheduler.scan_handle.lock().unwrap() = Some(scan);
    *scheduler.backup_handle.lock().unwrap() = Some(backup);
}

/// Minimum gap between event-driven scans. A burst of game launches (or a
/// process whose CPU flaps around the threshold across an agent restart)
/// coalesces into one detection pass instead of one per signal. The periodic
/// timer is the slow backstop; this is the fast path.
const EVENT_SCAN_DEBOUNCE_SECS: u64 = 60;

/// Fire a detection scan *now*, off the periodic timer, because the agent spotted
/// a heavy untracked game (`AgentEvent::HeavyProcessDetected`). No-op unless Modo
/// Automático is on — the periodic scanner only runs then, and auto-tracking
/// games the user never opted to monitor would surprise them. Debounced to
/// `EVENT_SCAN_DEBOUNCE_SECS` so repeated signals don't stack scans.
pub fn request_scan(app: AppHandle) {
    match Prefs::load_default() {
        Ok((prefs, _)) if prefs.automatic_mode => {}
        Ok(_) => return,
        Err(e) => {
            tracing::warn!(error = %e, "event scan: couldn't load prefs; skipping");
            return;
        }
    }

    let scheduler = app.state::<AutomaticScheduler>();
    {
        let mut last = scheduler.last_event_scan.lock().unwrap();
        let now = Instant::now();
        if let Some(prev) = *last {
            if now.duration_since(prev) < Duration::from_secs(EVENT_SCAN_DEBOUNCE_SECS) {
                tracing::debug!("event scan: debounced, a scan ran recently");
                return;
            }
        }
        *last = Some(now);
    }

    tracing::info!("event scan: heavy untracked game detected; scanning now");
    let app = app.clone();
    tokio::task::spawn(async move { run_scan(&app).await });
}

/// Which half of Modo Automático a worker task drives.
#[derive(Clone, Copy)]
enum TickKind {
    Scan,
    Backup,
}

/// Spawn one worker task. `tokio::time::interval`'s first tick is immediate, so
/// the work runs right away (toggling on / saving an interval does something
/// visible at once), then repeats every `period_secs`. We await the work
/// *inline* on each tick and use `MissedTickBehavior::Delay` so a scan/sweep
/// that runs long never re-fires back-to-back when it finishes.
fn spawn_worker(app: AppHandle, kind: TickKind, period_secs: u64) -> JoinHandle<()> {
    let period = Duration::from_secs(period_secs);
    tokio::task::spawn(async move {
        let mut ticker = interval(period);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match kind {
                TickKind::Scan => run_scan(&app).await,
                TickKind::Backup => run_backup_sweep(&app).await,
            }
        }
    })
}

/// Shape of the `automatic-phase` event — mirrors the frontend's
/// `AutomaticPhase` union so `automatic.ts` can drop it straight into
/// `automaticState` to animate the sidebar.
#[derive(Clone, Serialize)]
struct PhasePayload {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    done: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
}

/// Payload of `automatic-scan-complete` — the count of newly-tracked games so
/// the UI can toast and refresh the library list.
#[derive(Clone, Serialize)]
struct ScanComplete {
    tracked: usize,
}

fn emit_phase(app: &AppHandle, kind: &'static str, done: Option<usize>, total: Option<usize>) {
    let _ = app.emit("automatic-phase", PhasePayload { kind, done, total });
}

/// Headless detection + auto-track pass — the Rust-native replacement for what
/// the frontend's `runAutomaticSetup` used to do off `automatic-scan-tick`.
/// Runs detection, tracks every newly-detected high-confidence game with a real
/// save path, and boots the agent so those saves start being watched. Mirrors
/// the old frontend filter (`confidence == High && !found_paths.is_empty() &&
/// !already_tracked`). Errors are logged, never propagated — a background tick
/// must not take the app down.
pub async fn run_scan(app: &AppHandle) {
    emit_phase(app, "detecting", None, None);

    // `scan_library` runs detection, drops ignored slugs, and persists the
    // cache — same as the user pressing "re-escanear".
    let report = match crate::commands::library::scan_library(app.clone(), app.state()).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "automatic scan: detection failed");
            emit_phase(app, "idle", None, None);
            return;
        }
    };

    let tracked = match crate::commands::library::list_tracked_saves(app.state()).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "automatic scan: couldn't list tracked saves");
            emit_phase(app, "idle", None, None);
            return;
        }
    };
    // Slugs THIS machine actually tracks (has a local folder for). Orphan rows
    // are cloud saves from *another* machine with no local state here — they
    // must NOT count as tracked, or the slug looks monitored and auto-track
    // skips it (the cross-device bug: detected=1, tracked=0). They're collected
    // separately so a High detection can ADOPT them instead.
    let mut tracked_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut orphans_by_slug: std::collections::HashMap<
        String,
        crate::commands::library::TrackedSave,
    > = std::collections::HashMap::new();
    for t in tracked {
        if t.orphan {
            // Prefer the "main" label when a slug has several cloud branches;
            // otherwise keep the first seen.
            match orphans_by_slug.get(&t.game_slug) {
                Some(existing) if existing.label == "main" => {}
                _ => {
                    orphans_by_slug.insert(t.game_slug.clone(), t);
                }
            }
        } else {
            tracked_slugs.insert(t.game_slug);
        }
    }

    // One pass over the detected games splits them three ways:
    // * `adopt` — High + untracked here + already in the cloud as an orphan
    //   (another machine has it): bind this machine's detected folder to the
    //   existing save_id instead of forking a new branch (BUG 3).
    // * `candidates` — High + untracked + not in the cloud: brand-new save.
    // * `probe_dirs` — untracked + below High: fed to the agent for process↔
    //   write correlation (ADR 0020 fase 3) so a future scan promotes them.
    let mut candidates = Vec::new();
    let mut adopt: Vec<(crate::commands::library::TrackedSave, std::path::PathBuf)> = Vec::new();
    let mut probe_dirs: Vec<std::path::PathBuf> = Vec::new();
    for g in report.games {
        if tracked_slugs.contains(&g.slug) || g.found_paths.is_empty() {
            continue;
        }
        if g.confidence == Confidence::High {
            if let Some(orphan) = orphans_by_slug.remove(&g.slug) {
                adopt.push((orphan, g.found_paths[0].clone()));
            } else {
                candidates.push(g);
            }
        } else {
            probe_dirs.extend(g.found_paths.iter().cloned());
        }
    }

    let total = candidates.len() + adopt.len();
    let mut tracked_count = 0usize;
    let mut done = 0usize;

    // Adopt cloud orphans first: vincula la carpeta detectada al save existente.
    for (orphan, path) in adopt {
        emit_phase(app, "tracking", Some(done), Some(total));
        done += 1;
        let args = crate::commands::library::AdoptArgs {
            save_id: orphan.save_id.clone(),
            game_slug: orphan.game_slug.clone(),
            label: orphan.label.clone(),
            local_path: path.to_string_lossy().into_owned(),
        };
        match crate::commands::library::adopt_save(args, app.state()).await {
            Ok(_) => tracked_count += 1,
            Err(e) => {
                tracing::warn!(slug = %orphan.game_slug, error = %e, "automatic scan: couldn't adopt cloud save")
            }
        }
    }

    for game in candidates.into_iter() {
        emit_phase(app, "tracking", Some(done), Some(total));
        done += 1;
        let args = crate::commands::library::AddGameArgs {
            game_slug: game.slug.clone(),
            label: None,
            local_path: game.found_paths[0].to_string_lossy().into_owned(),
            display_name: Some(game.display_name),
            steam_app_id: game.steam_app_id.map(|v| v as i64),
        };
        // Don't abort the batch over one game — a single 422/409 on an old
        // server shouldn't stop the other nine from being tracked.
        match crate::commands::library::add_game_to_tracking(args, app.state()).await {
            Ok(_) => tracked_count += 1,
            Err(e) => {
                tracing::warn!(slug = %game.slug, error = %e, "automatic scan: couldn't track game")
            }
        }
    }

    // Boot the agent (no-op if already running) so newly-tracked saves get
    // watched immediately — and so a user who left games tracked from a prior
    // session gets them re-armed even when nothing new was found.
    emit_phase(app, "starting_agent", None, None);
    if let Err(e) = crate::commands::agent::start_agent(app.clone(), app.state()).await {
        tracing::warn!(error = %e, "automatic scan: couldn't boot agent");
    }

    // Hand the (now-running) agent the untracked candidates to probe for
    // process↔write correlation. Empty list is fine — it just clears the set.
    let handle = app.state::<AppState>().agent.lock().unwrap().clone();
    if let Some(h) = handle {
        let n = probe_dirs.len();
        if let Err(e) = h.set_probe_candidates(probe_dirs).await {
            tracing::warn!(error = %e, "automatic scan: couldn't set probe candidates");
        } else {
            tracing::debug!(count = n, "automatic scan: probe candidates sent to agent");
        }
    }

    emit_phase(app, "idle", None, None);
    let _ = app.emit(
        "automatic-scan-complete",
        ScanComplete {
            tracked: tracked_count,
        },
    );
    tracing::info!(tracked = tracked_count, "automatic scan: done");
}

/// Headless backup sweep — re-hashes every tracked save through the agent's
/// staggered window. Replaces the frontend's `runBackupSweep`. No-op (not an
/// error) when the agent isn't running; the next scan boots it.
pub async fn run_backup_sweep(app: &AppHandle) {
    if let Err(e) = crate::commands::agent::sweep_backups(app.state::<AppState>()).await {
        tracing::warn!(error = %e, "automatic backup sweep failed");
    }
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
