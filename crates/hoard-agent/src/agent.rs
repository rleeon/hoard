//! Long-running "live agent" that watches tracked saves and backs them up.
//!
//! Three independent loops cooperate inside one Tokio task:
//!
//! 1. **Filesystem watcher** — `notify-debouncer-mini` aggregates raw inotify
//!    events into a debounced stream. When a save folder settles for
//!    `debounce_secs`, we enqueue a backup.
//! 2. **Process watcher** — a periodic `sysinfo` poll asks "is any tracked
//!    game's executable running?" and emits `GameStarted` / `GameStopped`
//!    transitions. On stop we also enqueue an immediate backup, since the
//!    user just finished playing.
//! 3. **Backup scheduler** — drains the queue, runs `upload_directory` per
//!    entry, and applies exponential backoff (`2 ** retry` seconds, capped)
//!    on failure up to `max_retries`.
//!
//! Everything outside the agent talks to it through two channels:
//! - `AgentCommand` (mpsc, in)  — add/remove watched saves, shut down.
//! - `AgentEvent` (mpsc, out) — fire-and-forget notifications the desktop UI
//!   surfaces as Tauri events.
//!
//! The agent never panics on a missing path or a failed upload; those become
//! events the UI can show. Loss-of-network is the common case and we want it
//! to look like "we'll retry in a bit", not a crash.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant as TokioInstant;

use crate::api::{ApiClient, ApiError};
use crate::backup::{upload_directory_checked, BackupResult};

/// Configuration for the live agent. Defaults are tuned for v0.3's
/// "instant feel" priority:
///
/// - **5 s debounce**: short enough that auto-backup feels immediate
///   after a save, long enough to coalesce torn writes (Bethesda games,
///   Souls games re-write the save file mid-burst). v0.2's 30 s default
///   was much more conservative; product call to match the user's ask.
/// - **2 s process poll**: catches "I quit the game" within seconds
///   without hammering `/proc`.
/// - **5 retries** with exponential backoff covers "wifi blipped"
///   without pestering the user forever.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub debounce_secs: u64,
    pub poll_secs: u64,
    pub max_retries: u32,
    /// Mirror of `Prefs::auto_restore`. When `true`, every save the agent
    /// adopts (initial seed or live `AddSave`) is checked against the
    /// server: if the local path is missing or empty *and* the server has
    /// at least one snapshot, we restore the latest snapshot in the
    /// background and emit `AgentEvent::SaveAutoRestored`. Off by default
    /// because silently writing files under the user's `~` is the sort
    /// of side-effect that needs explicit opt-in.
    pub auto_restore: bool,
    /// Root directory the agent uses to park the *local* copy of a file
    /// before letting a newer remote version overwrite it (ADR 0014). The
    /// final path is `<conflict_root>/<save_id>/<rfc3339_ts>/<rel>`. When
    /// `None`, the agent falls back to 1.5.4 behaviour: a conflict where
    /// the remote is newer is *not* applied (we keep local) so data is
    /// never destroyed silently.
    pub conflict_root: Option<PathBuf>,
    /// Days to keep conflict backups under `conflict_root` before the
    /// per-tick sweep removes them. Mirrors `Prefs::conflict_retention_days`.
    pub conflict_retention_days: u32,
    /// Minimum seconds between two successful backups of the *same* save
    /// (ADR 0018, eje A — "ahorro de datos"). After a backup succeeds, the
    /// agent won't start another for this save until the interval elapses;
    /// intermediate writes coalesce into the next one (the final state is
    /// always uploaded). Kills the "one version per minute" cadence of games
    /// that autosave every few seconds (OpenTTD). `0` disables the floor
    /// (legacy behaviour — every settle backs up). The desktop derives this
    /// from `Prefs::data_saving` via `lerp(k, 5s, 600s)`.
    pub min_snapshot_interval_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            debounce_secs: 5,
            poll_secs: 2,
            max_retries: 5,
            auto_restore: false,
            conflict_root: None,
            conflict_retention_days: 14,
            min_snapshot_interval_secs: 0,
        }
    }
}

/// Map the user's `data_saving` knob (0..=1) to a minimum snapshot interval
/// in seconds via `lerp(k, 5, 600)` (ADR 0018, Decisión 4). `k=0` keeps the
/// eager 5 s floor; `k=1` waits up to 10 min between backups of a save.
pub fn min_snapshot_interval_for(data_saving: f64) -> u64 {
    let k = data_saving.clamp(0.0, 1.0);
    (5.0 + (600.0 - 5.0) * k).round() as u64
}

/// One save the agent is responsible for backing up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedSave {
    pub save_id: String,
    pub game_slug: String,
    pub display_name: String,
    pub label: String,
    pub local_path: PathBuf,
    /// Optional install directory (e.g. Steam's `steamapps/common/<game>`).
    /// Kept for the UI and for legacy install-dir-prefix matching as a
    /// fallback when [`Self::processes`] is empty.
    pub steam_install_dir: Option<PathBuf>,
    /// Process executable file names (case-insensitive, with extension on
    /// Windows). The agent's process poll matches against these to fire
    /// `GameStarted` / `GameStopped` transitions. Empty list = match by
    /// `steam_install_dir` only. Today the desktop never populates this
    /// — the curated TOML catalog that fed it was removed in 1.5.0 — so
    /// every save falls back to install-dir matching. The field stays
    /// in case a future catalog ships process names again.
    #[serde(default)]
    pub processes: Vec<String>,
    /// Resolved per-save sync overrides (from the save's preset). Empty by
    /// default = inherit every global `AgentConfig` setting. The agent reads
    /// `policy.<field>.unwrap_or(config.<field>)` at each decision point. See
    /// [`crate::presets`].
    #[serde(default)]
    pub policy: crate::presets::SavePolicy,
    /// Cloud version this device last committed or restored, read from
    /// `state.json` (`last_version_num`). Seeds the slot's `known_version`
    /// so the reconciliation sweep's version-gate is armed from the first
    /// tick after a restart: without it every restart re-downloads every
    /// snapshot to diff and drains the bandwidth quota. `None` for a
    /// freshly tracked save (nothing committed yet) is correct — the gate
    /// stays open so an empty/new device still pulls.
    #[serde(default)]
    pub known_version: Option<i64>,
}

/// Out-of-agent notifications. Frontend listens to these to drive the
/// dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    GameStarted {
        save_id: String,
        game_slug: String,
    },
    GameStopped {
        save_id: String,
        game_slug: String,
    },
    /// A backup will run after `delay_ms` unless cancelled. Used by the UI
    /// to show "next backup in 30s" pills.
    BackupScheduled {
        save_id: String,
        delay_ms: u64,
        reason: BackupReason,
    },
    BackupStarted {
        save_id: String,
    },
    BackupSuccess {
        save_id: String,
        version_num: i64,
        total_bytes: u64,
    },
    BackupFailed {
        save_id: String,
        error: String,
        will_retry: bool,
    },
    /// The agent detected that the save's local folder was missing or empty
    /// on add and `Prefs::auto_restore` was enabled, so it downloaded the
    /// latest server snapshot into the folder. The UI uses this to toast
    /// "We restored your save from the cloud" and to nudge the dashboard
    /// pill back to a synced state.
    SaveAutoRestored {
        save_id: String,
        game_slug: String,
        version_num: i64,
        files_extracted: u64,
        bytes_extracted: u64,
    },
    /// Auto-restore was attempted but failed (network error, sha mismatch,
    /// permission denied writing to the local path). Surfaced separately
    /// from `BackupFailed` because the user-visible message is different:
    /// the save is left untouched, no retry is scheduled, and we want
    /// the UI to suggest "restore manually" rather than "we'll try again".
    SaveAutoRestoreFailed {
        save_id: String,
        game_slug: String,
        error: String,
    },
    /// A scheduled backup landed but the local folder was empty (or gone)
    /// at upload time. We deliberately do **not** push an empty snapshot —
    /// that would silently destroy the user's last good save on the server
    /// the next time they look at History. Instead we surface this event so
    /// the UI can toast "we skipped backup because the folder is empty; turn
    /// on auto-restore in Settings if you wanted it pulled back".
    ///
    /// Since 1.4.3. Pairs with `SaveAutoRestored` when `auto_restore` is on:
    /// in that case the agent fires the restore *instead* of this event.
    BackupSkippedEmpty {
        save_id: String,
        game_slug: String,
    },
    /// The diff-based auto-restore found N files where the remote snapshot
    /// was newer than the local copy (ADR 0014). Before overwriting, the
    /// agent moved each local version into `conflict_dir`. The UI surfaces
    /// a toast so the user can recover manually if mtime decided wrong.
    SaveConflictsBackedUp {
        save_id: String,
        game_slug: String,
        count: u64,
        conflict_dir: PathBuf,
    },
}

/// Why we scheduled a backup. Useful in the UI to explain "the game just
/// closed, so I'm backing it up now" vs "the save folder changed".
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupReason {
    FilesystemSettled,
    GameStopped,
    Manual,
    /// One save inside a staggered "backup sweep" (Modo Automático's hourly
    /// hash pass). Spaced out across an effective window so disk I/O doesn't
    /// burst. Kept quiet in the activity feed — unlike a filesystem-settled
    /// backup there's no user-visible trigger, and N queued rows every hour
    /// would be noise — but the resulting upload still announces normally.
    SweepStaggered,
}

/// Per-slot diagnostic snapshot. Surfaced by the hidden Settings diagnostics
/// panel so a user can verify the watcher actually armed and is seeing fs
/// events. Serializable so the desktop can hand it straight to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSlotStatus {
    pub save_id: String,
    pub display_name: String,
    pub path: PathBuf,
    pub watcher_armed: bool,
    pub process_running: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_fs_event_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_scheduled_backup_at: Option<OffsetDateTime>,
}

/// Commands the host (Tauri command handlers, tests) sends to the agent.
enum AgentCommand {
    AddSave(WatchedSave),
    RemoveSave(String),
    BackupNow(String),
    /// Staggered "backup sweep": re-hash every tracked save to catch changes
    /// the fs-watcher missed, but spread the per-save work over time so disk
    /// use doesn't spike. `window_secs` is the nominal sweep interval (the
    /// hourly cadence); the agent grows it into a longer *effective* window
    /// when the total save footprint is large, and schedules each save at a
    /// size-proportional offset within it. Saves already queued for backup
    /// (fs event or a still-running previous sweep) are skipped so ticks
    /// don't pile up. Fired by the desktop "Modo Automático" backup
    /// scheduler.
    SweepAll {
        window_secs: u64,
    },
    /// Internal: an auto-restore task finished writing files into a slot's
    /// local path. The slot's fs watcher was either never armed (path was
    /// missing on AddSave) or armed against an empty directory — either
    /// way we re-arm it now so the freshly-restored save is being watched.
    /// Not exposed through `AgentHandle` because only the auto-restore
    /// task ever fires it.
    RearmWatcher(String),
    /// Internal: a spawned auto-restore task finished (success or failure).
    /// Clears `slot.restoring` so the reconciliation sweep can try again
    /// next tick. `not_on_server` is set when the restore failed with a 404
    /// (the save has no record/snapshot on the backend we're talking to) —
    /// the handler then parks the slot on a long backoff so the sweep stops
    /// hammering it every cooldown (saves tracked locally but absent from the
    /// current cloud account otherwise spam the log forever).
    AutoRestoreFinished {
        id: String,
        not_on_server: bool,
        /// The cloud version this slot is now synced to (the latest the restore
        /// observed), so the slot can remember it and the reconciliation sweep
        /// skips re-downloading the same version next tick. `None` when the
        /// attempt didn't reach a known version (404, transient failure).
        synced_version: Option<i64>,
    },
    /// Live-toggle `config.auto_restore` so the user's Settings change
    /// reaches the running agent without a restart. When flipped from
    /// `false → true` the agent also kicks an immediate reconciliation
    /// sweep so any tracked save with an empty local folder gets restored
    /// right away.
    SetAutoRestore(bool),
    /// DETECCIÓN (fase 3, ADR 0020): lista de carpetas candidatas detectadas
    /// pero AÚN NO rastreadas, que el escaneo del desktop quiere "sondear".
    /// El agente las vigila por mtime en cada tick de proceso: si una se
    /// reescribe mientras un juego está vivo, registra la correlación
    /// proceso↔escritura — la misma señal +0.50 que hoy sólo obtenían los
    /// saves ya rastreados. Rompe el huevo-y-gallina: jugar un juego no
    /// rastreado deja por fin rastro, y el siguiente escaneo lo asciende a
    /// `High` y lo auto-rastrea. Reemplaza el set entero en cada llamada.
    SetProbeCandidates(Vec<PathBuf>),
    QueryStatus(oneshot::Sender<Vec<AgentSlotStatus>>),
    Shutdown,
}

/// Handle returned by `spawn`. Cheap to clone (channel-cloning).
#[derive(Debug, Clone)]
pub struct AgentHandle {
    tx: mpsc::Sender<AgentCommand>,
}

impl AgentHandle {
    pub async fn add_save(&self, save: WatchedSave) -> Result<()> {
        self.tx.send(AgentCommand::AddSave(save)).await?;
        Ok(())
    }

    pub async fn remove_save(&self, save_id: impl Into<String>) -> Result<()> {
        self.tx
            .send(AgentCommand::RemoveSave(save_id.into()))
            .await?;
        Ok(())
    }

    /// Force an immediate backup attempt for `save_id`, bypassing debounce.
    /// Used by the "Back up now" button.
    pub async fn backup_now(&self, save_id: impl Into<String>) -> Result<()> {
        self.tx
            .send(AgentCommand::BackupNow(save_id.into()))
            .await?;
        Ok(())
    }

    /// Kick a staggered backup sweep across every tracked save. `window_secs`
    /// is the nominal sweep interval; the agent spreads each save's re-hash
    /// across an effective window (grown when there are tens of GB of saves)
    /// so disk I/O stays spread out. Replaces the frontend's old "loop
    /// `backup_now` over every save" burst.
    pub async fn sweep_all(&self, window_secs: u64) -> Result<()> {
        self.tx.send(AgentCommand::SweepAll { window_secs }).await?;
        Ok(())
    }

    /// Diagnostic snapshot of every tracked slot. Backs the hidden Settings
    /// "agent diagnostics" panel — surfaces the same internal state we'd
    /// otherwise only see in `tracing` logs (watcher armed, last fs event,
    /// next scheduled backup).
    pub async fn status(&self) -> Result<Vec<AgentSlotStatus>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(AgentCommand::QueryStatus(resp_tx)).await?;
        Ok(resp_rx.await?)
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.tx.send(AgentCommand::Shutdown).await?;
        Ok(())
    }

    /// Push a new `auto_restore` preference into the running agent. The
    /// agent loop applies it to its own copy of `AgentConfig` and, on a
    /// `false → true` flip, immediately re-scans every slot so any
    /// already-empty folder is restored right away (instead of waiting
    /// for the next fs event / process tick).
    pub async fn set_auto_restore(&self, enabled: bool) -> Result<()> {
        self.tx.send(AgentCommand::SetAutoRestore(enabled)).await?;
        Ok(())
    }

    /// Hand the agent the latest set of untracked candidate folders to probe
    /// for process↔write correlation (ADR 0020 fase 3). The desktop calls
    /// this after each automatic scan with the detected-but-untracked dirs.
    pub async fn set_probe_candidates(&self, dirs: Vec<PathBuf>) -> Result<()> {
        self.tx.send(AgentCommand::SetProbeCandidates(dirs)).await?;
        Ok(())
    }
}

/// Spawn the live agent. Returns a handle for sending commands and a task
/// handle the caller can `.abort()` for hard shutdown.
pub fn spawn(
    api: ApiClient,
    config: AgentConfig,
    initial_saves: Vec<WatchedSave>,
    events_tx: mpsc::Sender<AgentEvent>,
) -> (AgentHandle, JoinHandle<()>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(64);

    // Pre-seed the agent with already-tracked saves so the desktop app can
    // start watching as soon as login completes.
    let cmd_tx_seed = cmd_tx.clone();
    if !initial_saves.is_empty() {
        tokio::spawn(async move {
            for s in initial_saves {
                let _ = cmd_tx_seed.send(AgentCommand::AddSave(s)).await;
            }
        });
    }

    // The agent loop needs its own clone of `cmd_tx` so background tasks
    // it spawns (auto-restore is the only one today) can post commands
    // back to it — e.g. `RearmWatcher` after files land on disk.
    let cmd_tx_loop = cmd_tx.clone();
    let task = tokio::spawn(run_agent(api, config, cmd_rx, cmd_tx_loop, events_tx));
    (AgentHandle { tx: cmd_tx }, task)
}

/// Signal from a finished backup task back to the agent loop.
struct BackupDone {
    save_id: String,
    /// `Some` when a new snapshot was uploaded — carries the fresh set
    /// signature to cache on the slot. `None` when the backup was skipped
    /// (unchanged) or the folder was empty, so the slot keeps its previous
    /// signature.
    new_set_hash: Option<String>,
    /// `true` only when a real snapshot reached the server. The min-interval
    /// throttle anchors on `last_backup_at`, which must advance **only** on a
    /// genuine upload. A skip (unchanged bytes) or an empty/missing folder is
    /// not a backup: if it bumped the anchor, the next real change would be
    /// throttled a full `min_snapshot_interval_secs` out — and with
    /// auto-restore re-emptying the folder each cycle, the anchor would keep
    /// advancing on phantom "backups" and a short play session would never
    /// flush its progress before the game closed (R.E.P.O. regression).
    committed: bool,
    /// Version number of the snapshot just uploaded (`Some` only when
    /// `committed`). The agent advances the slot's `known_version` to this so
    /// the reconciliation sweep won't re-download a version this device itself
    /// just produced. `None` on skip/empty.
    version_num: Option<i64>,
}

/// Internal per-save bookkeeping.
struct SaveSlot {
    save: WatchedSave,
    /// Active fs debouncer. Armed in `handle_add` so the agent reacts to
    /// save-folder changes whether or not a game process is running. The
    /// pre-1.4 design built this lazily on `GameStarted`, which silently
    /// broke autobackup for any save without a matching process name
    /// (most non-Steam installs and most manifest entries without a
    /// `processes` field). See ADR / version1-5 §P1.4.0-0.
    watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    /// Tokio task that fires the debounced backup. Cancelled and recreated
    /// on every fs event so the timer effectively resets.
    pending: Option<tokio::task::JoinHandle<()>>,
    /// Currently-running guess from the last process poll. Drives
    /// GameStarted/Stopped transitions.
    is_running: bool,
    /// Has the save folder changed since the last successful backup?
    /// Drives the v0.3 "final-flush-only-if-pending" rule on `GameStopped`
    /// — no point re-uploading an unchanged save just because the user
    /// quit. Set on every fs event; cleared on backup success.
    has_pending: bool,
    /// Most recent debounced fs event observed for this slot. Surfaced via
    /// `AgentSlotStatus` so the diagnostics panel can prove the watcher
    /// is actually seeing writes.
    last_fs_event_at: Option<OffsetDateTime>,
    /// When the currently-pending backup will fire (UTC). `None` if no
    /// backup is scheduled. Recomputed in `schedule_backup`.
    next_scheduled_backup_at: Option<OffsetDateTime>,
    /// When the *oldest* un-flushed change in the current debounce window
    /// arrived (UTC). The notify debounce resets `next_scheduled_backup_at`
    /// on every write, so a game that autosaves every second (OpenTTD,
    /// factory builders) would reset the timer forever and never flush.
    /// This anchor lets the fs handler cap the total wait: once it's older
    /// than `MAX_BACKUP_WAIT_SECS`, we stop resetting and back up now.
    /// `None` when there are no pending changes; cleared on backup success.
    first_pending_event_at: Option<OffsetDateTime>,
    /// When this save was last successfully backed up (UTC). Anchors the
    /// `min_snapshot_interval_secs` floor (ADR 0018, eje A): a new backup is
    /// never scheduled to fire before `last_backup_at + interval`. `None`
    /// until the first success this session.
    last_backup_at: Option<OffsetDateTime>,
    /// `true` while a background auto-restore task is downloading into
    /// this slot's local path. Prevents the reconciliation sweep from
    /// firing the same restore twice. Cleared by
    /// `AgentCommand::AutoRestoreFinished` when the task ends (success
    /// or failure).
    restoring: bool,
    /// Earliest moment the reconciliation sweep is allowed to fire
    /// another auto-restore for this slot. Used as a 60-second cooldown
    /// after a failed attempt so a misbehaving server doesn't burn rate
    /// limits in a tight loop. `None` means "no cooldown active".
    next_auto_restore_at: Option<TokioInstant>,
    /// Skip-by-set-hash signature of the last successful upload this session
    /// (ADR 0019). Compared against the freshly-walked signature before each
    /// backup; an unchanged signature means the watcher fired on a no-op
    /// settle, so the upload is skipped. In-memory only — cross-restart
    /// persistence is the CLI/desktop's job via `state.json`.
    last_set_hash: Option<String>,
    /// Cloud version this slot is known to be synced to — advanced on a genuine
    /// upload commit and after a successful auto-restore. The reconciliation
    /// sweep passes it to `run_auto_restore`, which skips the download-to-diff
    /// when the server's latest version isn't newer than this. `None` until the
    /// first commit/restore this session (the first sweep then downloads once to
    /// establish the baseline). This is what stops the every-tick re-download
    /// that used to burn the cloud bandwidth quota: a real cross-device update
    /// (another device committed a higher version) still pulls; our own folder
    /// churn no longer does.
    known_version: Option<i64>,
}

async fn run_agent(
    api: ApiClient,
    mut config: AgentConfig,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    events_tx: mpsc::Sender<AgentEvent>,
) {
    let mut slots: HashMap<String, SaveSlot> = HashMap::new();

    // Channel used by every fs watcher — debounced events all funnel here
    // and we route them by path. mpsc::unbounded would be fine since the
    // debouncer already throttles, but we cap at 256 to be defensive.
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(256);

    // Backup tasks signal "save_id of save just successfully backed up"
    // so the agent loop can clear `has_pending`. Cap matches `cmd_rx`.
    let (done_tx, mut done_rx) = mpsc::channel::<BackupDone>(64);

    // Process watcher: periodic poll. We refresh only the bits we care
    // about (process names + exe paths) to keep CPU near zero when idle.
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let mut poll = tokio::time::interval(Duration::from_secs(config.poll_secs.max(1)));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // DETECCIÓN (fase 3, ADR 0020): store de correlación proceso↔escritura.
    // Cuando un save vigilado se reescribe, registramos qué proceso de juego
    // estaba vivo. Hoy alimenta atribución/aprendizaje sobre saves ya
    // rastreados; el observador sobre los roots amplios de `roots.rs` (para
    // DESCUBRIR carpetas nuevas) es el paso siguiente, más pesado, y queda
    // fuera de este cableado.
    let corr_path = crate::correlation::CorrelationStore::default_path().ok();
    let mut corr_store = corr_path
        .as_deref()
        .map(crate::correlation::CorrelationStore::load)
        .unwrap_or_default();

    // DETECCIÓN (fase 3, ADR 0020): sonda de candidatos no-rastreados. Mapea
    // cada carpeta candidata → su última mtime-máxima observada. Cuando una
    // sube (escritura nueva) y hay un juego vivo, registra la correlación. El
    // baseline `None` se siembra en el primer tick sin registrar nada (así no
    // confundimos un fichero pre-existente reciente con una escritura recién
    // observada).
    let mut probes: HashMap<PathBuf, Option<std::time::SystemTime>> = HashMap::new();

    tracing::info!(
        debounce_secs = config.debounce_secs,
        poll_secs = config.poll_secs,
        max_retries = config.max_retries,
        "agent: started"
    );

    loop {
        tokio::select! {
            // ----- Commands from the host -----
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(AgentCommand::AddSave(save)) => {
                        handle_add(
                            &mut slots, save, &fs_tx, &api, &events_tx, &cmd_tx, &config,
                        );
                    }
                    Some(AgentCommand::RearmWatcher(id)) => {
                        // Auto-restore created files where there were none —
                        // the watcher we built (or skipped) on AddSave needs
                        // to be rebuilt against the now-existing directory.
                        if let Some(slot) = slots.get_mut(&id) {
                            arm_watcher(slot, &fs_tx);
                        }
                    }
                    Some(AgentCommand::AutoRestoreFinished { id, not_on_server, synced_version }) => {
                        // The background restore task signalled completion
                        // (success or failure). Clear the in-flight flag so
                        // the reconciliation sweep can try again once the
                        // cooldown expires — `next_auto_restore_at` was set
                        // when we spawned, so we don't reset it here…
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.restoring = false;
                            // Remember the version we just synced to so the next
                            // sweep can skip the expensive download-to-diff when
                            // nothing newer has landed from another device.
                            if synced_version.is_some() {
                                slot.known_version = synced_version;
                            }
                            // …unless the save simply isn't on the server
                            // (404). Retrying every 60s can't conjure a
                            // snapshot that doesn't exist; park it on a long
                            // backoff so we check ~hourly instead of spamming.
                            if not_on_server {
                                slot.next_auto_restore_at = Some(
                                    TokioInstant::now()
                                        + Duration::from_secs(AUTO_RESTORE_NOT_FOUND_BACKOFF_SECS),
                                );
                            }
                        }
                    }
                    Some(AgentCommand::SetAutoRestore(enabled)) => {
                        let was = config.auto_restore;
                        config.auto_restore = enabled;
                        tracing::info!(
                            auto_restore = enabled,
                            "agent: auto_restore preference updated"
                        );
                        // Flipping from off → on is the user's cue that they
                        // want any already-empty folder pulled back right
                        // now. Don't wait for the next poll tick.
                        if !was && enabled {
                            sweep_for_auto_restore(
                                &mut slots, &api, &events_tx, &cmd_tx, &config,
                            );
                        }
                    }
                    Some(AgentCommand::SetProbeCandidates(dirs)) => {
                        // Reemplaza el set conservando los baselines de los que
                        // siguen; los nuevos arrancan en `None` (se siembran en
                        // el próximo tick). Drop de los que ya no son candidatos
                        // (se rastrearon o dejaron de detectarse).
                        let mut next: HashMap<PathBuf, Option<std::time::SystemTime>> =
                            HashMap::with_capacity(dirs.len());
                        for d in dirs {
                            let baseline = probes.get(&d).copied().flatten();
                            next.insert(d, baseline);
                        }
                        tracing::debug!(count = next.len(), "agent: probe candidates updated");
                        probes = next;
                    }
                    Some(AgentCommand::RemoveSave(id)) => {
                        if let Some(slot) = slots.remove(&id) {
                            if let Some(p) = slot.pending {
                                p.abort();
                            }
                            // watcher dropped here, releasing inotify handle.
                        }
                    }
                    Some(AgentCommand::BackupNow(id)) => {
                        if slots.contains_key(&id) {
                            schedule_backup(
                                &mut slots, &id, BackupReason::Manual,
                                Duration::ZERO, &api, &events_tx, &config, &done_tx, &cmd_tx,
                            );
                        }
                    }
                    Some(AgentCommand::SweepAll { window_secs }) => {
                        sweep_all(
                            &mut slots, window_secs, &api, &events_tx,
                            &config, &done_tx, &cmd_tx,
                        );
                    }
                    Some(AgentCommand::QueryStatus(resp)) => {
                        let snapshot: Vec<AgentSlotStatus> = slots
                            .values()
                            .map(|s| AgentSlotStatus {
                                save_id: s.save.save_id.clone(),
                                display_name: s.save.display_name.clone(),
                                path: s.save.local_path.clone(),
                                watcher_armed: s.watcher.is_some(),
                                process_running: s.is_running,
                                last_fs_event_at: s.last_fs_event_at,
                                next_scheduled_backup_at: s.next_scheduled_backup_at,
                            })
                            .collect();
                        let _ = resp.send(snapshot);
                    }
                    Some(AgentCommand::Shutdown) | None => {
                        tracing::info!("agent: shutting down");
                        break;
                    }
                }
            }

            // ----- Filesystem debounce hits -----
            Some(path) = fs_rx.recv() => {
                if let Some(save_id) = match_save_for_path(&slots, &path) {
                    let now = OffsetDateTime::now_utc();
                    // Per-save preset overrides win over the global config.
                    let debounce_secs = slots
                        .get(&save_id)
                        .and_then(|s| s.save.policy.debounce_secs)
                        .unwrap_or(config.debounce_secs);
                    let min_interval_secs = slots
                        .get(&save_id)
                        .and_then(|s| s.save.policy.min_snapshot_interval_secs)
                        .unwrap_or(config.min_snapshot_interval_secs);
                    let mut delay = Duration::from_secs(debounce_secs);
                    if let Some(slot) = slots.get_mut(&save_id) {
                        slot.has_pending = true;
                        slot.last_fs_event_at = Some(now);
                        // Anti-starvation cap. Each fs event resets the
                        // debounce, so a game writing every second would
                        // never settle and never flush ("se quedó todo en
                        // cola"). Anchor the oldest un-flushed change; once
                        // it has waited MAX_BACKUP_WAIT_SECS, stop resetting
                        // and back up now even though writes keep arriving.
                        let waited_since = *slot.first_pending_event_at.get_or_insert(now);
                        if (now - waited_since).whole_seconds() >= MAX_BACKUP_WAIT_SECS {
                            delay = Duration::ZERO;
                            slot.first_pending_event_at = Some(now);
                        }
                        // Minimum-interval floor (ADR 0018, eje A). Never start
                        // a new backup sooner than `min_snapshot_interval_secs`
                        // after the last successful one — coalesce the burst
                        // into the next allowed slot. The anchor is the fixed
                        // `last_backup_at`, so repeated writes converge on the
                        // same fire time instead of drifting. Wins over the
                        // anti-starvation `delay = ZERO` above: we deliberately
                        // wait, and always upload the final state when we do.
                        if min_interval_secs > 0 {
                            if let Some(last) = slot.last_backup_at {
                                let earliest = last
                                    + Duration::from_secs(min_interval_secs);
                                if now + delay < earliest {
                                    delay = (earliest - now).unsigned_abs();
                                }
                            }
                        }
                    }
                    tracing::info!(
                        save_id = %save_id,
                        path = %path.display(),
                        delay_ms = delay.as_millis() as u64,
                        "agent: fs event observed; scheduling backup"
                    );
                    schedule_backup(
                        &mut slots, &save_id, BackupReason::FilesystemSettled,
                        delay,
                        &api, &events_tx, &config, &done_tx, &cmd_tx,
                    );

                    // DETECCIÓN (fase 3, ADR 0020): la carpeta se reescribió;
                    // muestrea los procesos de juego vivos y registra la
                    // correlación proceso↔escritura. Alimenta atribución y la
                    // señal +0.50 del scoring para descubrimientos futuros.
                    sys.refresh_processes_specifics(
                        ProcessesToUpdate::All,
                        true,
                        ProcessRefreshKind::everything(),
                    );
                    let games = crate::correlation::sample_game_processes(&sys);
                    if !games.is_empty() {
                        let dir = slots
                            .get(&save_id)
                            .map(|s| s.save.local_path.clone())
                            .unwrap_or_else(|| path.clone());
                        corr_store.record(&dir, &games);
                        if let Some(p) = &corr_path {
                            if let Err(e) = corr_store.save(p) {
                                tracing::debug!(error = %e, "agent: failed to persist correlation store");
                            }
                        }
                    }
                }
            }

            // ----- Process poll tick -----
            _ = poll.tick() => {
                process_poll(&mut sys, &mut slots, &events_tx, &api, &config, &done_tx, &cmd_tx);
                // Watcher self-healing: a slot whose folder didn't exist when
                // the game was tracked (freshly installed, save dir created on
                // first save) never armed its watcher, and nothing rearms it
                // short of an auto-restore or an app restart. Every tick,
                // (re)arm any slot that has no watcher but whose folder now
                // exists. Cheap (a stat per tracked save) and silent for the
                // common already-armed case.
                for slot in slots.values_mut() {
                    if slot.watcher.is_none() && slot.save.local_path.is_dir() {
                        tracing::info!(
                            save_id = %slot.save.save_id,
                            path = %slot.save.local_path.display(),
                            "agent: save folder now present; rearming fs watcher"
                        );
                        arm_watcher(slot, &fs_tx);
                    }
                }
                // Reconciliation backstop: every tick, look for tracked
                // saves whose local folder is empty and (a) restore is enabled
                // for that save (global default or per-save preset), (b) we're
                // not already restoring, and (c) the cooldown has elapsed.
                // Catches the cases the event-driven paths miss — uninstall
                // while Hoard was closed, network came back online after a
                // failed attempt, user just turned auto_restore on with several
                // stale slots. The per-slot filter inside resolves the
                // effective preference, so we always call (a backup-only save
                // is filtered out there, not here).
                sweep_for_auto_restore(
                    &mut slots, &api, &events_tx, &cmd_tx, &config,
                );

                // DETECCIÓN (fase 3, ADR 0020): sonda de candidatos. `sys` ya
                // viene refrescado por `process_poll`. Para cada candidato no
                // rastreado, si su carpeta se reescribió desde el último tick
                // y hay un juego vivo, registra la correlación. Esto es lo que
                // rompe el huevo-y-gallina: el siguiente escaneo verá el bonus
                // +0.50 y ascenderá el candidato a `High`.
                if !probes.is_empty() {
                    probe_candidates(&mut probes, &sys, &mut corr_store, corr_path.as_deref());
                }
            }

            // ----- Backup success notifications -----
            Some(done) = done_rx.recv() => {
                if let Some(slot) = slots.get_mut(&done.save_id) {
                    slot.has_pending = false;
                    slot.next_scheduled_backup_at = None;
                    slot.first_pending_event_at = None;
                    // Advance the throttle anchor only on a real upload — a
                    // skip/empty must not push the next change a full
                    // min-interval into the future.
                    if done.committed {
                        slot.last_backup_at = Some(OffsetDateTime::now_utc());
                        // Remember the version we just produced so the sweep
                        // won't re-download our own upload to diff it.
                        if done.version_num.is_some() {
                            slot.known_version = done.version_num;
                        }
                    }
                    if let Some(h) = done.new_set_hash {
                        slot.last_set_hash = Some(h);
                    }
                }
            }
        }
    }
}

/// Register a save with the agent and arm its fs watcher immediately.
///
/// Pre-1.4 this deferred the watcher to `GameStarted`, which silently broke
/// autobackup for saves whose Ludusavi manifest entry had no `processes`
/// and that weren't a Steam install — the process poll never matched, the
/// watcher never armed, no events fired, the Dashboard pill stayed
/// "Inactivo" forever. Arming up front trades one inotify slot per tracked
/// save for end-to-end reliability; `process_poll` still emits
/// `GameStarted`/`GameStopped` for UI signalling but no longer gates the
/// fs subsystem.
///
/// Since 1.4.2: if `config.auto_restore` is on and the local folder is
/// missing or empty, kick off a background restore of the latest server
/// snapshot. Files land on disk, the agent loop receives `RearmWatcher`,
/// and the slot ends up watching the restored folder for the rest of
/// the session.
fn handle_add(
    slots: &mut HashMap<String, SaveSlot>,
    save: WatchedSave,
    fs_tx: &mpsc::Sender<PathBuf>,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
) {
    let save_for_restore = save.clone();
    let save_id = save.save_id.clone();
    let known_version = save.known_version;
    let mut slot = SaveSlot {
        save,
        watcher: None,
        pending: None,
        is_running: false,
        has_pending: false,
        last_fs_event_at: None,
        next_scheduled_backup_at: None,
        first_pending_event_at: None,
        last_backup_at: None,
        restoring: false,
        next_auto_restore_at: None,
        last_set_hash: None,
        known_version,
    };
    arm_watcher(&mut slot, fs_tx);
    slots.insert(save_id.clone(), slot);

    // Since 1.5.4 auto-restore is diff-based and non-destructive: it always
    // runs when `auto_restore` is on, and decides per-file whether to copy.
    // If nothing's missing the task ends with `restored == 0` and no event
    // is emitted, so this is cheap even on a fully-populated slot.
    //
    // Since 1.5.5 (ADR 0014) the same "user is playing" guard from the
    // sweep applies here too: if the folder was just touched, the user is
    // likely mid-session — let the next sweep handle it once mtime
    // stabilises. Since 1.7.x this is unconditional (no longer gated on
    // `processes.is_empty()`): a game whose process name doesn't match the
    // manifest leaves `is_running` false *and* `processes` non-empty, so
    // the old gate skipped this guard and an auto-restore could fire
    // mid-session, resurrecting rotated-out autosaves. The recent-touch
    // check is the reliable "user is playing" signal regardless of process
    // detection.
    if config.auto_restore {
        let recently_touched =
            is_path_recently_touched(&save_for_restore.local_path, RECENT_SAVE_GRACE);
        if recently_touched {
            tracing::debug!(
                save_id = %save_id,
                path = %save_for_restore.local_path.display(),
                "agent: handle_add auto-restore deferred — folder touched recently"
            );
        } else {
            if let Some(slot) = slots.get_mut(&save_id) {
                slot.restoring = true;
                slot.next_auto_restore_at =
                    Some(TokioInstant::now() + Duration::from_secs(AUTO_RESTORE_COOLDOWN_SECS));
            }
            spawn_auto_restore(
                save_for_restore,
                api.clone(),
                events_tx.clone(),
                cmd_tx.clone(),
                config.conflict_root.clone(),
                config.conflict_retention_days,
                // Fresh add: no known baseline yet, so this first pull downloads
                // once to establish it.
                None,
            );
        }
    }
}

/// Minimum interval between successive auto-restore attempts for the
/// same save. Applied to both successful and failed attempts so a server
/// that's flapping ("snapshot available", "snapshot gone", "snapshot
/// available" — possible during a GC race) doesn't get hammered by the
/// reconciliation sweep.
const AUTO_RESTORE_COOLDOWN_SECS: u64 = 60;

/// Backoff applied when an auto-restore fails with a 404: the save is tracked
/// locally but has no record/snapshot on the backend we're talking to (e.g.
/// saves carried over from another account, or a stale `state.json` entry).
/// Retrying on the normal 60s cooldown floods the log with WARNs forever, so
/// we space these out to roughly hourly — still self-heals if the user later
/// uploads the save, without the spam.
const AUTO_RESTORE_NOT_FOUND_BACKOFF_SECS: u64 = 60 * 60;

/// Hard ceiling on how long a continuously-writing save can defer its
/// backup. The notify debounce resets the timer on every write, so a game
/// that autosaves every second would never settle and never flush. Once
/// the oldest un-backed-up change has waited this long, the fs handler
/// forces the backup with a zero delay even though writes keep arriving.
/// Kept comfortably above the default 5 s debounce so normal saves still
/// coalesce; only pathological writers ever hit it.
const MAX_BACKUP_WAIT_SECS: i64 = 30;

/// True if `path` doesn't exist on disk, or exists as a directory that
/// contains no entries. Anything else (file, broken symlink, populated
/// directory) returns `false`. Errors reading the directory are treated
/// conservatively as "not empty" so we never wipe a user's save folder
/// just because we couldn't enumerate it (NFS hiccup, etc).
fn is_path_empty_or_missing(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    if !path.is_dir() {
        return false;
    }
    match std::fs::read_dir(path) {
        Ok(mut it) => it.next().is_none(),
        Err(_) => false,
    }
}

/// Background task: resolve the latest snapshot for `save`, download it
/// into the local path, emit `SaveAutoRestored` on success or
/// `SaveAutoRestoreFailed` otherwise, and ping the agent loop to re-arm
/// the watcher against the now-populated folder.
fn spawn_auto_restore(
    save: WatchedSave,
    api: ApiClient,
    events_tx: mpsc::Sender<AgentEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    conflict_root: Option<PathBuf>,
    conflict_retention_days: u32,
    known_version: Option<i64>,
) {
    tokio::spawn(async move {
        tracing::debug!(
            save_id = %save.save_id,
            game_slug = %save.game_slug,
            path = %save.local_path.display(),
            "agent: auto-restore diff — checking server snapshot against local"
        );
        let retention = Duration::from_secs(u64::from(conflict_retention_days) * 86_400);
        let mut not_on_server = false;
        let mut synced_version: Option<i64> = None;
        match run_auto_restore(&api, &save, conflict_root.as_deref(), retention, known_version).await {
            Ok(Some(outcome)) => {
                // We downloaded and diffed against this version; remember it so
                // the next sweep can short-circuit.
                synced_version = Some(outcome.version_num);
                let touched = outcome.files_restored + outcome.conflicts_backed_up;
                if touched > 0 {
                    tracing::info!(
                        save_id = %save.save_id,
                        version_num = outcome.version_num,
                        restored = outcome.files_restored,
                        backed_up = outcome.conflicts_backed_up,
                        local_wins = outcome.conflicts_local_wins,
                        bytes = outcome.bytes_extracted,
                        "auto-restore diff: applied {} files (incl. {} conflict-backups), {} kept local",
                        touched,
                        outcome.conflicts_backed_up,
                        outcome.conflicts_local_wins
                    );
                    let _ = events_tx
                        .send(AgentEvent::SaveAutoRestored {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            version_num: outcome.version_num,
                            files_extracted: touched,
                            bytes_extracted: outcome.bytes_extracted,
                        })
                        .await;
                    if outcome.conflicts_backed_up > 0 {
                        if let Some(dir) = outcome.conflict_dir.clone() {
                            let _ = events_tx
                                .send(AgentEvent::SaveConflictsBackedUp {
                                    save_id: save.save_id.clone(),
                                    game_slug: save.game_slug.clone(),
                                    count: outcome.conflicts_backed_up,
                                    conflict_dir: dir,
                                })
                                .await;
                        }
                    }
                    // Tell the agent loop to rebuild the fs watcher now that
                    // the directory actually has contents. Safe to send even
                    // if it was already armed — `arm_watcher` overwrites.
                    let _ = cmd_tx
                        .send(AgentCommand::RearmWatcher(save.save_id.clone()))
                        .await;
                } else if outcome.conflicts_local_wins > 0 {
                    tracing::debug!(
                        save_id = %save.save_id,
                        local_wins = outcome.conflicts_local_wins,
                        "auto-restore diff: nothing copied; {} files newer locally",
                        outcome.conflicts_local_wins
                    );
                }
                // else: every file present and identical — silent no-op.
            }
            Ok(None) => {
                tracing::debug!(
                    save_id = %save.save_id,
                    "agent: auto-restore — server has no snapshots yet; nothing to restore"
                );
            }
            Err(e) => {
                // A 404 means the save has no record/snapshot on the backend
                // (carried over from another account, stale state, or the
                // remote was purged). It's not a transient failure — don't
                // raise it to the user as an error and don't keep retrying on
                // the short cooldown; park it on a long backoff (below).
                not_on_server = matches!(e.downcast_ref::<ApiError>(), Some(ApiError::NotFound));
                if not_on_server {
                    tracing::debug!(
                        save_id = %save.save_id,
                        "agent: auto-restore — save not on server (404); backing off"
                    );
                } else {
                    tracing::warn!(
                        save_id = %save.save_id,
                        error = %e,
                        "agent: auto-restore failed"
                    );
                    let _ = events_tx
                        .send(AgentEvent::SaveAutoRestoreFailed {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }
        // Always clear the slot's `restoring` flag, even on failure — the
        // reconciliation sweep is responsible for retrying once the
        // cooldown expires; we just need to mark this attempt as done.
        let _ = cmd_tx
            .send(AgentCommand::AutoRestoreFinished {
                id: save.save_id.clone(),
                not_on_server,
                synced_version,
            })
            .await;
    });
}

/// Reconciliation sweep: every tick, schedule a diff-based auto-restore for
/// any save not already being restored and outside its cooldown window. The
/// restore task itself decides whether anything actually needs copying —
/// since 1.5.4 a populated local folder no longer skips the attempt at
/// this stage; it skips inside `restore_files_into` once we've compared
/// the snapshot against what's on disk.
///
/// Guards apply *before* spawning to avoid stomping on a save the user is
/// actively touching:
/// 1. `slot.is_running` → game is open, skip.
/// 2. `slot.has_pending` → un-flushed local changes queued, skip.
/// 3. `last_fs_event_at` within `RECENT_SAVE_GRACE` → the watcher saw a
///    write recently, skip.
/// 4. Disk mtime within `RECENT_SAVE_GRACE` → fallback for the startup
///    window before the agent has fs history of its own.
///
/// Since 1.7.x the activity guards (2, 3) drive the decision and the mtime
/// check is only a fallback. The earlier version gated solely on
/// `is_running` + dir mtime, both of which miss real-world cases: a game
/// whose process name doesn't match its manifest never sets `is_running`,
/// and autosavers that truncate-and-overwrite the same file in place don't
/// bump the *directory* mtime — so the sweep auto-restored mid-session,
/// re-downloading autosaves the game had already rotated away and failing
/// uploads as the restore mutated the folder under them. The agent's own
/// inotify stream catches both.
///
/// Cheap: per-slot work here is just a `restoring` flag check and a
/// timer compare. The network/disk cost happens inside the spawned task,
/// which dedupes via `restoring` so the next sweep doesn't pile up.
fn sweep_for_auto_restore(
    slots: &mut HashMap<String, SaveSlot>,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
) {
    let now = TokioInstant::now();
    // Collect candidate save_ids first to keep the borrow checker happy
    // (we mutate the slot afterwards, then spawn a task that holds a
    // clone of `WatchedSave`).
    let candidates: Vec<(String, WatchedSave)> = slots
        .iter()
        .filter(|(id, slot)| {
            // Per-save preset can opt out of restore (backup-only) or opt in
            // even when the global default is off.
            if !slot.save.policy.auto_restore.unwrap_or(config.auto_restore) {
                return false;
            }
            if slot.restoring {
                return false;
            }
            if let Some(t) = slot.next_auto_restore_at {
                if now < t {
                    return false;
                }
            }
            if slot.is_running {
                tracing::debug!(
                    save_id = %id,
                    "sweep: skipping — game process is running"
                );
                return false;
            }
            // The agent's own watcher is a far more reliable "user is here"
            // signal than disk mtime: inotify catches in-place file rewrites
            // that DON'T bump the directory's mtime (OpenTTD and other
            // autosavers truncate-and-overwrite the same .sav). If there are
            // un-flushed changes queued, or we observed an fs event within
            // the grace window, the user is mid-session — never auto-restore
            // into a folder they're actively writing, or the restore and the
            // backup fight over the same files (re-adding rotated autosaves,
            // failing uploads mid-mutation).
            if slot.has_pending {
                tracing::debug!(
                    save_id = %id,
                    "sweep: skipping — un-flushed local changes pending"
                );
                return false;
            }
            if let Some(last) = slot.last_fs_event_at {
                if (OffsetDateTime::now_utc() - last).whole_seconds()
                    < RECENT_SAVE_GRACE.as_secs() as i64
                {
                    tracing::debug!(
                        save_id = %id,
                        "sweep: skipping — fs event observed recently"
                    );
                    return false;
                }
            }
            // Disk-mtime fallback: covers the window right after the agent
            // starts, before it has any fs history of its own.
            if is_path_recently_touched(&slot.save.local_path, RECENT_SAVE_GRACE) {
                tracing::debug!(
                    save_id = %id,
                    path = %slot.save.local_path.display(),
                    "sweep: skipping — save folder touched recently"
                );
                return false;
            }
            true
        })
        .map(|(id, slot)| (id.clone(), slot.save.clone()))
        .collect();

    for (id, save) in candidates {
        let known_version = slots.get(&id).and_then(|s| s.known_version);
        if let Some(slot) = slots.get_mut(&id) {
            slot.restoring = true;
            slot.next_auto_restore_at = Some(now + Duration::from_secs(AUTO_RESTORE_COOLDOWN_SECS));
        }
        tracing::debug!(
            save_id = %id,
            "agent: reconciliation sweep — scheduling diff auto-restore"
        );
        spawn_auto_restore(
            save,
            api.clone(),
            events_tx.clone(),
            cmd_tx.clone(),
            config.conflict_root.clone(),
            config.conflict_retention_days,
            known_version,
        );
    }
}

/// Grace window for the "save touched recently" heuristic in sweep guards.
/// Five minutes matches the ADR 0014 acceptance: while playing, the
/// process poll will normally mark the slot `is_running`; this catches the
/// case where the slot has no process match in the catalog.
const RECENT_SAVE_GRACE: Duration = Duration::from_secs(5 * 60);

/// True if `path` exists and has been modified within `grace`. Conservative
/// on errors: an unreadable path returns `false` so we don't deadlock the
/// auto-restore against a slot we can't stat.
fn is_path_recently_touched(path: &Path, grace: Duration) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(mtime) = meta.modified() else {
        return false;
    };
    match std::time::SystemTime::now().duration_since(mtime) {
        Ok(age) => age < grace,
        Err(_) => false,
    }
}

/// Mayor mtime entre la propia carpeta y sus ficheros inmediatos (no
/// recursivo — barato y suficiente: un save que se escribe deja un fichero
/// nuevo/tocado en el primer nivel, p.ej. el `.zip` de Factorio en `saves/`).
/// `None` si la carpeta no se puede leer.
fn dir_max_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    let mut max = std::fs::metadata(dir).ok().and_then(|m| m.modified().ok());
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            if let Ok(m) = entry.metadata() {
                if let Ok(t) = m.modified() {
                    max = Some(match max {
                        Some(cur) if cur >= t => cur,
                        _ => t,
                    });
                }
            }
        }
    }
    max
}

/// Recorre las carpetas candidatas sondeadas, actualiza sus baselines de
/// mtime y devuelve aquellas reescritas desde el último tick. El baseline
/// `None` (primer avistamiento) sólo se siembra, sin reportar — evita
/// atribuir un fichero pre-existente reciente a una escritura no presenciada.
/// Pura (sin I/O de procesos ni persistencia) para poder testearla.
fn probe_detect_writes(
    probes: &mut HashMap<PathBuf, Option<std::time::SystemTime>>,
) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for (dir, baseline) in probes.iter_mut() {
        let Some(current) = dir_max_mtime(dir) else {
            continue;
        };
        let is_write = matches!(*baseline, Some(prev) if current > prev);
        *baseline = Some(current);
        if is_write {
            written.push(dir.clone());
        }
    }
    written
}

/// DETECCIÓN (fase 3, ADR 0020): sondea los candidatos y, para los reescritos
/// desde el último tick, si hay un juego vivo registra la correlación
/// proceso↔escritura y persiste el store. Es lo que rompe el huevo-y-gallina:
/// jugar un juego no rastreado deja por fin el rastro +0.50 que el siguiente
/// escaneo necesita para ascenderlo a `High`.
fn probe_candidates(
    probes: &mut HashMap<PathBuf, Option<std::time::SystemTime>>,
    sys: &System,
    corr_store: &mut crate::correlation::CorrelationStore,
    corr_path: Option<&Path>,
) {
    let written = probe_detect_writes(probes);
    if written.is_empty() {
        return;
    }
    // Sólo muestreamos procesos cuando de verdad hubo una escritura (perezoso).
    let games = crate::correlation::sample_game_processes(sys);
    if games.is_empty() {
        return;
    }
    for dir in &written {
        tracing::info!(
            dir = %dir.display(),
            process = %games[0].name,
            "agent: probe write correlated to live game; recording"
        );
        corr_store.record(dir, &games);
    }
    if let Some(p) = corr_path {
        if let Err(e) = corr_store.save(p) {
            tracing::debug!(error = %e, "agent: failed to persist correlation store (probe)");
        }
    }
}

/// Internal restore primitive returning the outcome summary or `None` if
/// the server has no snapshots for this save (in which case auto-restore
/// is a no-op, not a failure).
struct AutoRestoreOutcome {
    version_num: i64,
    /// Files copied from the remote snapshot into the local folder (those
    /// that were missing locally). Bytes equal between staging and local
    /// don't count.
    files_restored: u64,
    /// Files where the local copy was preserved because its mtime was
    /// newer than the remote (or `conflict_root` was unset — see
    /// `restore_files_into` for the fallback path).
    conflicts_local_wins: u64,
    /// Files where the local copy was moved into the conflict backup dir
    /// before being overwritten by the remote version (ADR 0014).
    conflicts_backed_up: u64,
    /// Where the local versions were parked, if any. `None` when
    /// `conflicts_backed_up == 0`.
    conflict_dir: Option<PathBuf>,
    /// Total bytes copied. Sum of `restored` + `conflicts_resolved_remote`
    /// file sizes.
    bytes_extracted: u64,
}

/// Per-file outcome accounting for diff-based restore. Returned by
/// `restore_files_into` and embedded into `AutoRestoreOutcome`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RestoreStats {
    /// Files copied from `source` into `target` because they were missing
    /// from `target`.
    pub restored: usize,
    /// Files present in both `source` and `target` with identical bytes.
    /// Left as-is.
    pub skipped: usize,
    /// Files where bytes differ and the *remote* won by mtime, so we
    /// overwrote the local copy with the staged remote version.
    pub conflicts_resolved_remote: usize,
    /// Files where bytes differ and the *local* won by mtime, so we left
    /// the local copy alone. Also incremented as a safety fallback when
    /// `conflict_backup_dir` is `None` and the remote would have won.
    pub conflicts_resolved_local: usize,
    /// Files where the local copy was moved into the conflict backup dir
    /// before being replaced by the remote version (subset of
    /// `conflicts_resolved_remote`).
    pub conflicts_backed_up: usize,
    /// Total bytes copied across `restored` + `conflicts_resolved_remote`.
    /// Useful for the `SaveAutoRestored` event payload.
    pub bytes_restored: u64,
}

async fn run_auto_restore(
    api: &ApiClient,
    save: &WatchedSave,
    conflict_root: Option<&Path>,
    retention: Duration,
    known_version: Option<i64>,
) -> Result<Option<AutoRestoreOutcome>> {
    let latest = if api.is_cloud().await {
        api.cloud_sync()
            .await?
            .saves
            .into_iter()
            .find(|e| e.save_id == save.save_id)
            .map(|e| e.latest_version_num)
    } else {
        api.get_save(&save.save_id).await?.latest_version_num
    };
    // Version gate: if we're already synced to the server's latest version,
    // there's nothing newer from another device to pull, so skip the expensive
    // download-to-diff entirely. This is the fix for the bandwidth blowout —
    // the sweep used to re-download the full snapshot every ~50s just to diff
    // it against a folder that hadn't changed, exhausting the 15-min cloud
    // quota (429 storm) and starving real uploads. A genuine cross-device
    // update bumps the server version above `known_version` and still pulls.
    if let (Some(v), Some(known)) = (latest, known_version) {
        if known >= v {
            tracing::debug!(
                save_id = %save.save_id,
                version = v,
                "agent: auto-restore — already synced to latest version; skipping download"
            );
            if let Some(root) = conflict_root {
                if let Err(e) = cleanup_old_conflicts(root, retention).await {
                    tracing::debug!(error = %e, "cleanup_old_conflicts failed (up-to-date path)");
                }
            }
            return Ok(None);
        }
    }
    let Some(version) = latest else {
        // Still sweep TTL before bailing — keeps the conflict dir bounded
        // even for saves whose remote has been purged.
        if let Some(root) = conflict_root {
            if let Err(e) = cleanup_old_conflicts(root, retention).await {
                tracing::debug!(error = %e, "cleanup_old_conflicts failed (no-snapshot path)");
            }
        }
        return Ok(None);
    };
    // Stage the snapshot in a unique temp dir so we never overwrite the
    // user's local files during extraction. The staging dir is empty by
    // construction, so `download_snapshot` extracts into it cleanly even
    // with `force=false`. Cleanup happens in `cleanup_staging` at the end.
    let staging = staging_dir_for(&save.save_id);
    tokio::fs::create_dir_all(&staging)
        .await
        .with_context(|| format!("creating staging dir {}", staging.display()))?;

    let download_result = crate::restore::download_snapshot(
        api,
        &save.save_id,
        version,
        &staging,
        crate::restore::RestoreOptions {
            skip_verify: false,
            force: false,
        },
        |_, _| {},
    )
    .await;

    let outcome = match download_result {
        Ok(o) => o,
        Err(e) => {
            cleanup_staging(&staging).await;
            return Err(e);
        }
    };
    let _ = outcome; // we walk the staging dir directly for the diff

    // Per-attempt timestamped subdir so concurrent restores never collide
    // and the TTL sweep can drop the whole subtree in one shot. We compute
    // it lazily *only if* a conflict_root is configured — `restore_files_into`
    // treats `None` as the safe legacy fallback.
    let conflict_backup_dir: Option<PathBuf> = conflict_root.map(|root| {
        let ts = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown-ts".to_string())
            // Colons aren't legal in Windows paths and look weird everywhere.
            .replace(':', "-");
        root.join(&save.save_id).join(ts)
    });

    let copy_result =
        restore_files_into(&save.local_path, &staging, conflict_backup_dir.as_deref()).await;
    cleanup_staging(&staging).await;

    // Best-effort TTL sweep regardless of the per-file outcome — we want
    // bounded disk usage even when the current restore had no conflicts.
    if let Some(root) = conflict_root {
        if let Err(e) = cleanup_old_conflicts(root, retention).await {
            tracing::debug!(error = %e, "cleanup_old_conflicts failed");
        }
    }

    let stats = copy_result?;
    let dir_used = if stats.conflicts_backed_up > 0 {
        conflict_backup_dir
    } else {
        None
    };

    Ok(Some(AutoRestoreOutcome {
        version_num: version,
        files_restored: stats.restored as u64,
        conflicts_local_wins: stats.conflicts_resolved_local as u64,
        conflicts_backed_up: stats.conflicts_backed_up as u64,
        conflict_dir: dir_used,
        bytes_extracted: stats.bytes_restored,
    }))
}

/// Build a unique staging directory under the system temp dir. We embed
/// the save_id (sanitised to alphanumeric+dash) and a monotonic nanosecond
/// counter so concurrent restores for the same save never collide.
fn staging_dir_for(save_id: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let safe_id: String = save_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    std::env::temp_dir().join(format!(
        "hoard-restore-{safe_id}-{n}-{}",
        std::process::id()
    ))
}

/// Best-effort tempdir cleanup. We log but never propagate the error: a
/// leaked staging dir is annoying but not user-visible, and the OS will
/// reap `/tmp` on reboot anyway.
async fn cleanup_staging(staging: &Path) {
    if let Err(e) = tokio::fs::remove_dir_all(staging).await {
        tracing::debug!(
            staging = %staging.display(),
            error = %e,
            "agent: failed to clean up restore staging dir"
        );
    }
}

/// Walk `conflict_root` two levels deep (`<save_id>/<timestamp>/`) and
/// remove every timestamp dir whose mtime is older than `now - retention`.
/// No-op when the root doesn't exist (typical fresh install). Errors are
/// logged but never propagated — a stuck conflict dir is much better than
/// killing the auto-restore tick.
pub(crate) async fn cleanup_old_conflicts(conflict_root: &Path, retention: Duration) -> Result<()> {
    if !conflict_root.exists() {
        return Ok(());
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(retention)
        .unwrap_or(std::time::UNIX_EPOCH);
    let mut save_entries = tokio::fs::read_dir(conflict_root)
        .await
        .with_context(|| format!("reading conflict root {}", conflict_root.display()))?;
    while let Some(save_entry) = save_entries.next_entry().await? {
        if !save_entry.file_type().await?.is_dir() {
            continue;
        }
        let save_dir = save_entry.path();
        let mut ts_entries = match tokio::fs::read_dir(&save_dir).await {
            Ok(it) => it,
            Err(e) => {
                tracing::debug!(
                    dir = %save_dir.display(),
                    error = %e,
                    "agent: skipping unreadable conflict save dir"
                );
                continue;
            }
        };
        while let Some(ts_entry) = ts_entries.next_entry().await? {
            if !ts_entry.file_type().await?.is_dir() {
                continue;
            }
            let ts_dir = ts_entry.path();
            let mtime = match ts_entry.metadata().await.and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(e) => {
                    tracing::debug!(
                        dir = %ts_dir.display(),
                        error = %e,
                        "agent: couldn't read conflict ts mtime; leaving it alone"
                    );
                    continue;
                }
            };
            if mtime < cutoff {
                match tokio::fs::remove_dir_all(&ts_dir).await {
                    Ok(()) => tracing::info!(
                        dir = %ts_dir.display(),
                        "agent: removed expired conflict backup"
                    ),
                    Err(e) => tracing::warn!(
                        dir = %ts_dir.display(),
                        error = %e,
                        "agent: failed to remove expired conflict backup"
                    ),
                }
            }
        }
    }
    Ok(())
}

/// Copy files from `source` into `target` non-destructively, resolving
/// per-file conflicts via mtime (ADR 0014).
///
/// Walks `source` recursively. For each file:
///
/// - `target/rel` missing → copy; bump `restored`.
/// - `target/rel` exists with identical bytes → skip; bump `skipped`.
/// - `target/rel` exists with different bytes:
///   - `local_mtime > remote_mtime + 1s` → local wins, untouched; bump
///     `conflicts_resolved_local`.
///   - Otherwise (remote newer, or within ±1s tolerance) → remote wins.
///     If `conflict_backup_dir` is `Some(dir)`, move `target/rel` to
///     `dir/rel` (creating parents) and bump `conflicts_backed_up`, then
///     copy `source/rel` over and bump `conflicts_resolved_remote`. If
///     `conflict_backup_dir` is `None`, *do not* overwrite — bump
///     `conflicts_resolved_local` as a safety fallback (legacy 1.5.4
///     behaviour) and log a warn.
///
/// Errors propagate only for I/O failures we can't classify (e.g.
/// permission denied reading a file we just listed).
pub(crate) async fn restore_files_into(
    target: &Path,
    source: &Path,
    conflict_backup_dir: Option<&Path>,
) -> Result<RestoreStats> {
    let mut stats = RestoreStats::default();
    let mut stack: Vec<PathBuf> = vec![source.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .with_context(|| format!("reading staging dir {}", dir.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                // Skip symlinks, devices etc — they shouldn't appear in
                // a hoard snapshot but we'd rather no-op than crash.
                continue;
            }
            let rel = path
                .strip_prefix(source)
                .with_context(|| format!("path {} not under source", path.display()))?;
            let dest = target.join(rel);
            if dest.exists() {
                if files_have_equal_bytes(&path, &dest).await? {
                    stats.skipped += 1;
                    continue;
                }
                // Bytes differ — resolve via mtime. 1s tolerance covers
                // FAT32 and friends; remote ties take the local side so a
                // close call doesn't trash data.
                if local_mtime_wins(&dest, &path).await {
                    tracing::debug!(
                        rel = %rel.display(),
                        "auto-restore diff: local wins on mtime"
                    );
                    stats.conflicts_resolved_local += 1;
                    continue;
                }
                let Some(backup_root) = conflict_backup_dir else {
                    // No backup dir configured (legacy fallback): never
                    // destroy local data even if remote looks newer.
                    tracing::warn!(
                        rel = %rel.display(),
                        "auto-restore diff: remote appears newer but no conflict_backup_dir; keeping local"
                    );
                    stats.conflicts_resolved_local += 1;
                    continue;
                };
                let backup_dest = backup_root.join(rel);
                if let Some(parent) = backup_dest.parent() {
                    tokio::fs::create_dir_all(parent).await.with_context(|| {
                        format!("creating conflict backup parent dir {}", parent.display())
                    })?;
                }
                // `rename` first (cheap, atomic). Fall back to copy+remove
                // when the conflict root is on a different filesystem
                // (typical when state_dir lives on the system disk and the
                // save folder is on a different volume).
                if let Err(e) = tokio::fs::rename(&dest, &backup_dest).await {
                    tracing::debug!(
                        rel = %rel.display(),
                        error = %e,
                        "auto-restore diff: rename across filesystems failed, falling back to copy"
                    );
                    tokio::fs::copy(&dest, &backup_dest)
                        .await
                        .with_context(|| {
                            format!(
                                "copying {} → {} for conflict backup",
                                dest.display(),
                                backup_dest.display()
                            )
                        })?;
                    tokio::fs::remove_file(&dest).await.with_context(|| {
                        format!("removing local {} after conflict backup", dest.display())
                    })?;
                }
                stats.conflicts_backed_up += 1;
                let copied = tokio::fs::copy(&path, &dest)
                    .await
                    .with_context(|| format!("copying {} → {}", path.display(), dest.display()))?;
                stats.conflicts_resolved_remote += 1;
                stats.bytes_restored += copied;
                continue;
            }
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("creating parent dir {} for restore", parent.display())
                })?;
            }
            let copied = tokio::fs::copy(&path, &dest)
                .await
                .with_context(|| format!("copying {} → {}", path.display(), dest.display()))?;
            stats.restored += 1;
            stats.bytes_restored += copied;
        }
    }

    Ok(stats)
}

/// True when the local file's mtime is more than 1s newer than the remote
/// file's. Conservative on errors: if we can't read either mtime, we treat
/// the remote as the winner — the snapshot's authority comes from the
/// server's committed timestamps, which are more reliable than a local
/// filesystem with quirks (FAT32 2s rounding, network share clock skew).
async fn local_mtime_wins(local: &Path, remote: &Path) -> bool {
    let local_mtime = match tokio::fs::metadata(local).await.and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let remote_mtime = match tokio::fs::metadata(remote).await.and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    match local_mtime.duration_since(remote_mtime) {
        Ok(d) => d > Duration::from_secs(1),
        Err(_) => false,
    }
}

/// Cheap bytes-equal: size first (saves the read for the common
/// different-sized case), then a single shot read of each file and a
/// linear compare. Files in tracked saves are small enough that
/// chunk-streaming would only matter for pathological archives — the
/// per-file alloc cost is much smaller than the network/zstd cost we
/// already paid to land them in staging.
async fn files_have_equal_bytes(a: &Path, b: &Path) -> Result<bool> {
    let meta_a = tokio::fs::metadata(a).await?;
    let meta_b = tokio::fs::metadata(b).await?;
    if meta_a.len() != meta_b.len() {
        return Ok(false);
    }
    let bytes_a = tokio::fs::read(a).await?;
    let bytes_b = tokio::fs::read(b).await?;
    Ok(bytes_a == bytes_b)
}

/// Try to attach an fs debouncer to `slot`. Tolerant: a missing folder or
/// an inotify error logs and leaves `slot.watcher == None` so the agent
/// keeps running for the other slots. Re-arming later is fine — we just
/// overwrite the field.
fn arm_watcher(slot: &mut SaveSlot, fs_tx: &mpsc::Sender<PathBuf>) {
    let path = slot.save.local_path.clone();
    if !path.is_dir() {
        tracing::info!(
            save_id = %slot.save.save_id,
            path = %path.display(),
            "agent: save path missing on add; fs watcher not armed"
        );
        slot.watcher = None;
        return;
    }
    match build_watcher(&path, fs_tx.clone()) {
        Ok(w) => {
            tracing::info!(
                save_id = %slot.save.save_id,
                path = %path.display(),
                "agent: fs watcher armed"
            );
            slot.watcher = Some(w);
        }
        Err(e) => {
            tracing::warn!(
                save_id = %slot.save.save_id,
                path = %path.display(),
                error = %e,
                "agent: couldn't arm fs watcher"
            );
            slot.watcher = None;
        }
    }
}

fn build_watcher(
    path: &Path,
    fs_tx: mpsc::Sender<PathBuf>,
) -> Result<Debouncer<notify::RecommendedWatcher>> {
    let watch_root = path.to_path_buf();
    let mut debouncer = new_debouncer(
        // Internal aggregation window for notify-debouncer-mini. We use a
        // small value (2 s) here and apply our larger product debounce by
        // resetting the schedule timer on each event. That way we still see
        // bursts as a single "settled" signal upstream.
        Duration::from_secs(2),
        move |res: DebounceEventResult| {
            if let Ok(events) = res {
                if !events.is_empty() {
                    let _ = fs_tx.try_send(watch_root.clone());
                }
            }
        },
    )?;
    debouncer
        .watcher()
        .watch(path, notify::RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// Find which save a path event belongs to. The fs watcher emits the root
/// it was registered for, so this is a direct lookup by canonical prefix.
fn match_save_for_path(slots: &HashMap<String, SaveSlot>, path: &Path) -> Option<String> {
    for slot in slots.values() {
        if slot.save.local_path == path || path.starts_with(&slot.save.local_path) {
            return Some(slot.save.save_id.clone());
        }
    }
    None
}

/// Cancel any in-flight pending backup, then schedule a new one to run
/// after `delay`. The pending task does the wait *and* the upload, so we
/// can abort the wait cleanly when a new event resets the timer.
#[allow(clippy::too_many_arguments)]
fn schedule_backup(
    slots: &mut HashMap<String, SaveSlot>,
    save_id: &str,
    reason: BackupReason,
    delay: Duration,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) {
    let Some(slot) = slots.get_mut(save_id) else {
        return;
    };
    // Was a backup already scheduled for this slot? If so, this call is
    // just resetting the debounce timer inside an in-progress window — the
    // feed already shows a "queued" row for it. Re-announcing on every fs
    // event is what flooded the activity feed with orphaned "en cola"
    // entries when a game autosaves every second. Only announce on the
    // leading edge; the row resolves when the upload completes (which
    // clears `next_scheduled_backup_at` via `done_rx`).
    let already_scheduled = slot.next_scheduled_backup_at.is_some();
    if let Some(p) = slot.pending.take() {
        p.abort();
    }

    slot.next_scheduled_backup_at = Some(OffsetDateTime::now_utc() + delay);

    tracing::info!(
        save_id = %save_id,
        delay_ms = delay.as_millis() as u64,
        reason = ?reason,
        "agent: backup scheduled"
    );

    // Don't announce zero-delay backups (manual / forced flush) — they'd
    // add noise — nor re-announce a window that's already queued, nor the
    // staggered sweep entries (there's no user-visible trigger and one row
    // per save every hour would flood the feed; the resulting upload still
    // announces normally when it runs).
    if delay > Duration::ZERO
        && !already_scheduled
        && !matches!(reason, BackupReason::SweepStaggered)
    {
        let _ = events_tx.try_send(AgentEvent::BackupScheduled {
            save_id: save_id.to_string(),
            delay_ms: delay.as_millis() as u64,
            reason,
        });
    }

    let api = api.clone();
    let events_tx = events_tx.clone();
    let done_tx = done_tx.clone();
    let cmd_tx = cmd_tx.clone();
    let save = slot.save.clone();
    let prev_set_hash = slot.last_set_hash.clone();
    let max_retries = config.max_retries;
    // Per-save preset can force backup-only (`Some(false)`) or force restore
    // (`Some(true)`) regardless of the global default.
    let auto_restore = slot.save.policy.auto_restore.unwrap_or(config.auto_restore);
    let conflict_root = config.conflict_root.clone();
    let conflict_retention_days = config.conflict_retention_days;

    slot.pending = Some(tokio::spawn(async move {
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        run_backup_with_retry(
            api,
            save,
            prev_set_hash,
            events_tx,
            done_tx,
            cmd_tx,
            max_retries,
            auto_restore,
            conflict_root,
            conflict_retention_days,
        )
        .await;
    }));
}

/// Nominal hash-throughput budget for the staggered sweep: how many bytes of
/// save data each second of the *effective* window covers. Calibrated so
/// ~20 GiB of saves stretches the window to ~2h (≈6 min per GiB), keeping
/// sustained disk reads thin. Below this footprint the configured interval
/// dominates and the window stays at its nominal length.
const SWEEP_BYTES_PER_WINDOW_SEC: f64 = 20.0 * 1024.0 * 1024.0 * 1024.0 / 7200.0;

/// Floor on the gap between consecutive saves in a staggered sweep, so even a
/// pile of tiny saves gets a visible beat between each re-hash instead of
/// firing back-to-back.
const SWEEP_MIN_GAP_SECS: f64 = 15.0;

/// Staggered backup sweep (see `AgentCommand::SweepAll`). Walks each tracked
/// save's folder for its byte footprint (metadata only — no file contents are
/// read here), then schedules a re-hash for each at a size-proportional offset
/// inside an effective window. The window is
/// `max(window_secs, total / SWEEP_BYTES_PER_WINDOW_SEC)`, so a small set
/// finishes within the nominal interval while tens of GB stretch it out. Saves
/// already queued for backup (live fs event, or a still-running previous
/// sweep) are left alone so repeated ticks don't reset the stagger or pile up
/// concurrent hashes.
#[allow(clippy::too_many_arguments)]
fn sweep_all(
    slots: &mut HashMap<String, SaveSlot>,
    window_secs: u64,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) {
    // Snapshot (id, path, already-queued) up front: scheduling borrows `slots`
    // mutably, so we can't hold an iterator over it while calling
    // `schedule_backup` below.
    let entries: Vec<(String, PathBuf, bool)> = slots
        .values()
        .map(|s| {
            (
                s.save.save_id.clone(),
                s.save.local_path.clone(),
                s.next_scheduled_backup_at.is_some(),
            )
        })
        .collect();
    if entries.is_empty() {
        return;
    }

    // Byte footprint per save (metadata walk). Missing/unreadable folders
    // count as zero — they're handled (or skipped-empty) when their turn to
    // back up comes.
    let sized: Vec<(String, bool, u64)> = entries
        .into_iter()
        .map(|(id, path, queued)| (id, queued, dir_size_bytes(&path)))
        .collect();
    let total_bytes: u64 = sized.iter().map(|(_, _, b)| *b).sum();
    let n = sized.len() as f64;

    // Effective window: grows past the nominal interval once the footprint is
    // large enough that spreading it thin needs more time.
    let window = window_secs.max(1) as f64;
    let effective_window = if total_bytes > 0 {
        window.max(total_bytes as f64 / SWEEP_BYTES_PER_WINDOW_SEC)
    } else {
        window
    };

    tracing::info!(
        saves = sized.len(),
        total_mib = (total_bytes / (1024 * 1024)),
        window_secs,
        effective_window_secs = effective_window as u64,
        "agent: starting staggered backup sweep"
    );

    let mut offset = 0.0_f64;
    for (id, already_queued, bytes) in sized {
        // Per-save slice of the window: size-proportional when we have a
        // total, an even split otherwise, floored so tiny saves still space
        // out.
        let slice = if total_bytes > 0 {
            (effective_window * (bytes as f64 / total_bytes as f64)).max(SWEEP_MIN_GAP_SECS)
        } else {
            (effective_window / n).max(SWEEP_MIN_GAP_SECS)
        };
        // Skip saves already on the schedule (live fs change, or a previous
        // sweep that hasn't run yet): don't reset their timer. We still
        // advance `offset` by their slice so the remaining saves keep their
        // size-proportional spacing — and so a long sweep that overruns into
        // the next tick finishes instead of restarting.
        if !already_queued {
            schedule_backup(
                slots,
                &id,
                BackupReason::SweepStaggered,
                Duration::from_secs_f64(offset),
                api,
                events_tx,
                config,
                done_tx,
                cmd_tx,
            );
        }
        offset += slice;
    }
}

/// Sum the byte size of every regular file under `root`, recursively. Reads
/// directory entries + file metadata only — never opens a file — so it's the
/// cheap way to learn a save's footprint for sweep staggering. Unreadable
/// dirs/entries are skipped rather than erroring; a best-effort estimate is
/// all the scheduler needs.
fn dir_size_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            }
            // symlinks ignored, mirroring walk_source.
        }
    }
    total
}

/// Upload + retry. Backoff is `2 ** attempt` seconds, capped at 5 min.
/// `max_retries == 0` means "try once and give up on failure".
///
/// Since 1.4.3 there's a pre-check: if the local folder is missing or empty
/// at upload time, we never push an empty snapshot. The user can wipe a
/// save folder for any number of reasons (uninstall, manual cleanup,
/// crashed mod) and shipping an empty backup would silently overwrite the
/// last good copy on the server with nothing. Instead:
///
/// - `auto_restore = true`  → spawn a restore task to repopulate the
///   folder from the latest server snapshot and emit `SaveAutoRestored`.
/// - `auto_restore = false` → emit `BackupSkippedEmpty` and bail. The UI
///   surfaces a toast pointing the user at the Settings toggle.
#[allow(clippy::too_many_arguments)]
async fn run_backup_with_retry(
    api: ApiClient,
    save: WatchedSave,
    prev_set_hash: Option<String>,
    events_tx: mpsc::Sender<AgentEvent>,
    done_tx: mpsc::Sender<BackupDone>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    max_retries: u32,
    auto_restore: bool,
    conflict_root: Option<PathBuf>,
    conflict_retention_days: u32,
) {
    if is_path_empty_or_missing(&save.local_path) {
        tracing::info!(
            save_id = %save.save_id,
            path = %save.local_path.display(),
            auto_restore,
            "agent: backup skipped — local folder is empty/missing"
        );
        // Always clear has_pending so a future fs event isn't blocked.
        let _ = done_tx.try_send(BackupDone {
            save_id: save.save_id.clone(),
            new_set_hash: None,
            committed: false,
            version_num: None,
        });
        if auto_restore {
            spawn_auto_restore(
                save.clone(),
                api.clone(),
                events_tx.clone(),
                cmd_tx,
                conflict_root,
                conflict_retention_days,
                // Empty/missing folder: we genuinely want the save back, so
                // don't version-gate this pull.
                None,
            );
        } else {
            let _ = events_tx
                .send(AgentEvent::BackupSkippedEmpty {
                    save_id: save.save_id.clone(),
                    game_slug: save.game_slug.clone(),
                })
                .await;
        }
        return;
    }
    let mut attempt = 0u32;
    loop {
        let _ = events_tx
            .send(AgentEvent::BackupStarted {
                save_id: save.save_id.clone(),
            })
            .await;

        let outcome = upload_directory_checked(
            &api,
            &save.save_id,
            &save.game_slug,
            &save.label,
            &save.local_path,
            prev_set_hash.as_deref(),
            // base_version: the auto-path doesn't yet track the last-synced
            // version per save (WatchedSave carries none), so it pushes
            // without a fast-forward base for now. The server still records
            // the DAG parent; conflict-aware auto-sync is the next step.
            None,
            |_, _| {},
        )
        .await;

        match outcome {
            Ok(BackupResult::Skipped) => {
                // The save's cheap set signature is unchanged since the last
                // upload: the watcher fired on a settle that didn't actually
                // write anything. Skip the no-op snapshot, clear has_pending.
                tracing::info!(
                    save_id = %save.save_id,
                    "agent: backup skipped — no content change since last upload"
                );
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: None,
                    committed: false,
                    version_num: None,
                });
                return;
            }
            Ok(BackupResult::Unchanged { signature }) => {
                // The cheap signature drifted (mtime bump) but the bytes are
                // identical to the last upload. No snapshot, but cache the
                // refreshed composite so the next check hits the fast path
                // instead of re-reading every file.
                tracing::info!(
                    save_id = %save.save_id,
                    "agent: backup skipped — bytes unchanged despite mtime drift"
                );
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: Some(signature),
                    committed: false,
                    version_num: None,
                });
                return;
            }
            Ok(BackupResult::Uploaded {
                outcome: o,
                signature,
            }) => {
                let _ = events_tx
                    .send(AgentEvent::BackupSuccess {
                        save_id: save.save_id.clone(),
                        version_num: o.snapshot.version_num,
                        total_bytes: o.total_bytes,
                    })
                    .await;
                // Tell the agent loop to clear has_pending and cache the new
                // signature. If the channel is full or the agent is shutting
                // down we just drop the signal — worst case we re-upload an
                // unchanged snapshot on the next GameStopped, a soft failure.
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: Some(signature),
                    committed: true,
                    version_num: Some(o.snapshot.version_num),
                });
                return;
            }
            Err(e) => {
                let will_retry = attempt < max_retries;
                let _ = events_tx
                    .send(AgentEvent::BackupFailed {
                        save_id: save.save_id.clone(),
                        error: e.to_string(),
                        will_retry,
                    })
                    .await;
                if !will_retry {
                    return;
                }
                let backoff_secs = (1u64 << attempt.min(8)).min(300);
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                attempt += 1;
            }
        }
    }
}

/// One sweep of the process table. Emits transitions + schedules a
/// post-game backup when a watched game stops running.
///
/// Since 1.4 this no longer touches the fs watcher — the watcher is armed
/// in `handle_add` and lives for the slot's lifetime. `process_poll` is
/// pure UI signal (Dashboard pill, "the game just closed → flush" hint).
#[allow(clippy::too_many_arguments)]
fn process_poll(
    sys: &mut System,
    slots: &mut HashMap<String, SaveSlot>,
    events_tx: &mpsc::Sender<AgentEvent>,
    api: &ApiClient,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) {
    // Refresh every process. The `true` flag asks sysinfo to remove
    // entries for processes that have exited since the last refresh,
    // which is exactly what we need to detect "game stopped".
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    // Build a set of "currently running" save_ids. Two matchers cooperate:
    // process-name match (manifest-driven, storefront-agnostic) takes
    // precedence; install-dir match is the legacy v0.2 fallback for
    // saves registered without a manifest.
    let mut running: HashSet<String> = HashSet::new();
    for slot in slots.values() {
        let proc_names: HashSet<String> = slot
            .save
            .processes
            .iter()
            .map(|p| p.to_lowercase())
            .collect();
        let install_dir = slot.save.steam_install_dir.as_ref();

        for proc in sys.processes().values() {
            // Name match — works on every storefront on Windows, and on
            // Proton/Wine where the wineprefix process keeps the .exe name.
            if !proc_names.is_empty() {
                let name = proc.name().to_string_lossy().to_lowercase();
                if proc_names.contains(&name) {
                    running.insert(slot.save.save_id.clone());
                    break;
                }
            }
            // Legacy install-dir fallback. Skipped if name-match is
            // configured to avoid double counting.
            if proc_names.is_empty() {
                if let (Some(exe), Some(dir)) = (proc.exe(), install_dir) {
                    if exe.starts_with(dir) {
                        running.insert(slot.save.save_id.clone());
                        break;
                    }
                }
            }
        }
    }

    // Diff against previous tick to fire transition events.
    // We collect first, then mutate, to keep the borrow-checker happy.
    let transitions: Vec<(String, bool)> = slots
        .keys()
        .map(|id| (id.clone(), running.contains(id)))
        .filter(|(id, now)| slots.get(id).map(|s| s.is_running != *now).unwrap_or(false))
        .collect();

    for (id, now_running) in transitions {
        let (game_slug, local_path, had_pending) = {
            let slot = match slots.get(&id) {
                Some(s) => s,
                None => continue,
            };
            (
                slot.save.game_slug.clone(),
                slot.save.local_path.clone(),
                slot.has_pending,
            )
        };

        if now_running {
            // Decide the pre-launch sync barrier *before* flipping
            // `is_running` — the sweep skips running slots, but the whole
            // point of the barrier is to pull on launch. We still honour the
            // other "user is here" guards so we never clobber an active local
            // session: un-flushed changes, a recent fs event, a recently
            // touched folder, an in-flight restore, or an unexpired cooldown
            // all veto the pull. The restore itself is conflict-aware
            // (local-newer files win, conflicts are backed up), so even when
            // it does fire it can't lose newer local progress.
            let barrier_save: Option<WatchedSave> = {
                slots.get(&id).and_then(|slot| {
                    // Per-save preset can disable (backup-only) or enable the
                    // pull barrier regardless of the global default.
                    if !slot.save.policy.auto_restore.unwrap_or(config.auto_restore) {
                        return None;
                    }
                    if slot.restoring {
                        return None;
                    }
                    if let Some(t) = slot.next_auto_restore_at {
                        if TokioInstant::now() < t {
                            return None;
                        }
                    }
                    if slot.has_pending {
                        return None;
                    }
                    if let Some(last) = slot.last_fs_event_at {
                        if (OffsetDateTime::now_utc() - last).whole_seconds()
                            < RECENT_SAVE_GRACE.as_secs() as i64
                        {
                            return None;
                        }
                    }
                    if is_path_recently_touched(&slot.save.local_path, RECENT_SAVE_GRACE) {
                        return None;
                    }
                    Some(slot.save.clone())
                })
            };

            if let Some(slot) = slots.get_mut(&id) {
                slot.is_running = true;
            }
            tracing::info!(
                save_id = %id,
                game_slug = %game_slug,
                path = %local_path.display(),
                "agent: GameStarted"
            );
            let _ = events_tx.try_send(AgentEvent::GameStarted {
                save_id: id.clone(),
                game_slug,
            });

            // Pre-launch sync barrier (Fase 1): the instant a game launches,
            // pull the latest remote snapshot so a cross-device hand-off feels
            // immediate — play on the tablet, sit down at the PC, launch, and
            // the tablet's progress is already there. Reuses the same
            // conflict-aware restore as the reconciliation sweep.
            if let Some(save) = barrier_save {
                let known_version = slots.get(&id).and_then(|s| s.known_version);
                if let Some(slot) = slots.get_mut(&id) {
                    slot.restoring = true;
                    slot.next_auto_restore_at =
                        Some(TokioInstant::now() + Duration::from_secs(AUTO_RESTORE_COOLDOWN_SECS));
                }
                tracing::info!(
                    save_id = %id,
                    "agent: GameStarted — pre-launch sync barrier, pulling latest snapshot"
                );
                spawn_auto_restore(
                    save,
                    api.clone(),
                    events_tx.clone(),
                    cmd_tx.clone(),
                    config.conflict_root.clone(),
                    config.conflict_retention_days,
                    known_version,
                );
            }
        } else {
            if let Some(slot) = slots.get_mut(&id) {
                slot.is_running = false;
            }
            tracing::info!(
                save_id = %id,
                game_slug = %game_slug,
                had_pending,
                "agent: GameStopped"
            );
            let _ = events_tx.try_send(AgentEvent::GameStopped {
                save_id: id.clone(),
                game_slug,
            });
            // Final flush on GameStopped *only* if something changed since
            // the last successful backup — avoids re-uploading an identical
            // snapshot every time the user quits.
            if had_pending {
                schedule_backup(
                    slots,
                    &id,
                    BackupReason::GameStopped,
                    Duration::from_secs(2),
                    api,
                    events_tx,
                    config,
                    done_tx,
                    cmd_tx,
                );
            } else {
                tracing::debug!(
                    save_id = %id,
                    "agent: GameStopped with no pending changes; skipping backup"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn config_defaults_are_sane() {
        let c = AgentConfig::default();
        assert!(c.debounce_secs >= 5, "too eager");
        assert!(c.debounce_secs <= 120, "too sleepy");
        assert!(c.poll_secs >= 1);
        assert!(c.max_retries >= 1);
    }

    #[test]
    fn probe_seeds_baseline_then_reports_writes() {
        let dir = tempfile::tempdir().unwrap();
        let cand = dir.path().to_path_buf();
        std::fs::write(cand.join("save1.zip"), b"a").unwrap();

        let mut probes: HashMap<PathBuf, Option<std::time::SystemTime>> = HashMap::new();
        probes.insert(cand.clone(), None);

        // Primer tick: sólo siembra el baseline, no reporta nada.
        assert!(probe_detect_writes(&mut probes).is_empty());
        assert!(probes[&cand].is_some());

        // Una escritura posterior (mtime mayor) sí se reporta.
        let later = std::time::SystemTime::now() + Duration::from_secs(120);
        filetime::set_file_mtime(
            cand.join("save1.zip"),
            filetime::FileTime::from_system_time(later),
        )
        .unwrap();
        let written = probe_detect_writes(&mut probes);
        assert_eq!(written, vec![cand.clone()]);

        // Sin nuevos cambios: silencio.
        assert!(probe_detect_writes(&mut probes).is_empty());
    }

    #[test]
    fn match_save_for_path_finds_exact_and_subpath() {
        let save = WatchedSave {
            save_id: "abc".into(),
            game_slug: "stardew-valley".into(),
            display_name: "Stardew Valley".into(),
            label: "main".into(),
            local_path: PathBuf::from("/tmp/saves/stardew"),
            steam_install_dir: None,
            processes: vec![],
            policy: Default::default(),
            known_version: None,
        };
        let mut slots = HashMap::new();
        slots.insert(
            "abc".to_string(),
            SaveSlot {
                save,
                watcher: None,
                pending: None,
                is_running: false,
                has_pending: false,
                last_fs_event_at: None,
                next_scheduled_backup_at: None,
                first_pending_event_at: None,
                last_backup_at: None,
                restoring: false,
                next_auto_restore_at: None,
                last_set_hash: None,
                known_version: None,
            },
        );

        assert_eq!(
            match_save_for_path(&slots, Path::new("/tmp/saves/stardew")),
            Some("abc".into())
        );
        assert_eq!(
            match_save_for_path(&slots, Path::new("/tmp/saves/stardew/farm")),
            Some("abc".into())
        );
        assert_eq!(
            match_save_for_path(&slots, Path::new("/tmp/saves/other")),
            None
        );
    }

    /// Regression for the "watcher only arms on GameStarted" bug.
    /// A save with no `processes` and no `steam_install_dir` should still
    /// trigger a debounced backup when its folder changes — even with no
    /// game process running. Today this fails: `handle_add` doesn't arm
    /// the watcher and `process_poll` never finds a matching process, so
    /// the fs event is never observed.
    #[tokio::test(flavor = "current_thread")]
    async fn fs_event_triggers_backup_without_game_running() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let save_path = tmp.path().to_path_buf();

        let api = ApiClient::new("http://127.0.0.1:1", "fake").expect("fake api client");
        let (events_tx, mut events_rx) = mpsc::channel::<AgentEvent>(64);

        let save = WatchedSave {
            save_id: "watcher-bug-1".into(),
            game_slug: "fake-game".into(),
            display_name: "Fake Game".into(),
            label: "main".into(),
            local_path: save_path.clone(),
            steam_install_dir: None,
            processes: vec![],
            policy: Default::default(),
            known_version: None,
        };

        // Short debounce so the test completes well under the 10s timeout.
        let config = AgentConfig {
            debounce_secs: 1,
            poll_secs: 2,
            max_retries: 0,
            auto_restore: false,
            conflict_root: None,
            conflict_retention_days: 14,
            min_snapshot_interval_secs: 0,
        };

        let (handle, task) = spawn(api, config, vec![save], events_tx);

        // Give the agent a beat to register the save before we touch the
        // folder — otherwise the fs event could land before `AddSave` is
        // processed.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Touch a file inside the watched directory.
        let mut f = std::fs::File::create(save_path.join("save.dat")).expect("create save file");
        f.write_all(b"hello").expect("write save file");
        f.sync_all().expect("sync save file");
        drop(f);

        // Wait for BackupScheduled within 10s. If the bug is present this
        // times out because no watcher is ever armed.
        let scheduled = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(evt) = events_rx.recv().await {
                if let AgentEvent::BackupScheduled { save_id, .. } = evt {
                    return save_id;
                }
            }
            "<channel closed>".to_string()
        })
        .await;

        // Best-effort teardown before asserting so the task doesn't leak.
        let _ = handle.shutdown().await;
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await;

        let save_id = scheduled.expect(
            "timed out waiting for BackupScheduled — the fs watcher never armed for an idle save",
        );
        assert_eq!(save_id, "watcher-bug-1");
    }

    /// Helper for the diff-restore tests: write `contents` to `path`
    /// creating parent dirs as needed.
    fn write_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    /// Source has A, B, C. Target has only A (identical to source). The
    /// diff restore copies B and C and leaves A alone.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_copies_missing_files() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"alpha");
        write_file(&source.join("b.dat"), b"beta");
        write_file(&source.join("nested/c.dat"), b"gamma");
        write_file(&target.join("a.dat"), b"alpha");

        let stats = restore_files_into(target, source, None).await.unwrap();

        assert_eq!(stats.restored, 2, "B and C should be copied");
        assert_eq!(stats.skipped, 1, "A is identical, skipped silently");
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_resolved_local, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(
            stats.bytes_restored,
            (b"beta".len() + b"gamma".len()) as u64
        );

        // Local A untouched.
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"alpha");
        // B and C now present locally with source contents.
        assert_eq!(std::fs::read(target.join("b.dat")).unwrap(), b"beta");
        assert_eq!(
            std::fs::read(target.join("nested/c.dat")).unwrap(),
            b"gamma"
        );
    }

    /// Conflict case: A exists in both source and target but bytes differ.
    /// The local copy wins — bytes on disk stay as the target's version
    /// and the conflict is reported in stats.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_preserves_local_on_conflict() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"remote-version");
        write_file(&target.join("a.dat"), b"LOCAL-WORK");

        let stats = restore_files_into(target, source, None).await.unwrap();

        assert_eq!(stats.restored, 0, "nothing copied — A is a conflict");
        assert_eq!(stats.skipped, 0);
        // No conflict_backup_dir → fallback to "keep local" regardless of
        // mtime, accounted under `conflicts_resolved_local`.
        assert_eq!(stats.conflicts_resolved_local, 1);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(stats.bytes_restored, 0);
        // Local content preserved verbatim.
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"LOCAL-WORK");
    }

    /// Everything identical between source and target: zero restores, zero
    /// conflicts, just `skipped` accounting. The agent uses
    /// `restored == 0 && conflicts == 0` to keep the activity feed quiet.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_silent_when_all_identical() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"alpha");
        write_file(&source.join("sub/b.dat"), b"beta");
        write_file(&target.join("a.dat"), b"alpha");
        write_file(&target.join("sub/b.dat"), b"beta");

        let stats = restore_files_into(target, source, None).await.unwrap();

        assert_eq!(stats.restored, 0);
        assert_eq!(stats.skipped, 2);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_resolved_local, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(stats.bytes_restored, 0);
    }

    /// Empty target dir: every file in source gets copied, no conflicts.
    /// Mirrors the "agent boots, save folder was wiped" scenario.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_full_restore_when_target_empty() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"alpha");
        write_file(&source.join("b.dat"), b"beta-bytes");
        write_file(&source.join("deep/nested/c.dat"), b"gamma!");

        let stats = restore_files_into(target, source, None).await.unwrap();

        assert_eq!(stats.restored, 3);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_resolved_local, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(
            stats.bytes_restored,
            (b"alpha".len() + b"beta-bytes".len() + b"gamma!".len()) as u64
        );
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"alpha");
        assert_eq!(
            std::fs::read(target.join("deep/nested/c.dat")).unwrap(),
            b"gamma!"
        );
    }

    /// Helper: set both file mtimes deterministically so the mtime branch
    /// is exercised without relying on test runtime ordering.
    fn set_mtime(path: &Path, mtime: std::time::SystemTime) {
        let ft = filetime::FileTime::from_system_time(mtime);
        filetime::set_file_mtime(path, ft).expect("set mtime");
    }

    /// Remote newer than local + a conflict_backup_dir → remote wins. The
    /// previous local bytes land in `conflict_backup_dir/<rel>` before
    /// being overwritten by the staged remote version.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_remote_wins_when_remote_newer() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let backup_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();
        let backup = backup_tmp.path();

        write_file(&source.join("a.dat"), b"remote-new");
        write_file(&target.join("a.dat"), b"local-old");
        // local mtime = T-10s, remote mtime = T+10s (clearly newer).
        let now = std::time::SystemTime::now();
        set_mtime(&target.join("a.dat"), now - Duration::from_secs(10));
        set_mtime(&source.join("a.dat"), now + Duration::from_secs(10));

        let stats = restore_files_into(target, source, Some(backup))
            .await
            .unwrap();

        assert_eq!(stats.conflicts_resolved_remote, 1);
        assert_eq!(stats.conflicts_backed_up, 1);
        assert_eq!(stats.conflicts_resolved_local, 0);
        assert_eq!(stats.restored, 0);
        assert_eq!(stats.bytes_restored, b"remote-new".len() as u64);
        // Target now has the remote version.
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"remote-new");
        // The previous local bytes were parked in the backup root.
        assert_eq!(std::fs::read(backup.join("a.dat")).unwrap(), b"local-old");
    }

    /// Local newer than remote (well past the 1s tolerance) → local wins.
    /// The remote file is *not* applied and no conflict backup is taken.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_local_wins_when_local_newer() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let backup_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();
        let backup = backup_tmp.path();

        write_file(&source.join("a.dat"), b"remote-old");
        write_file(&target.join("a.dat"), b"LOCAL-WORK");
        let now = std::time::SystemTime::now();
        set_mtime(&source.join("a.dat"), now - Duration::from_secs(60));
        set_mtime(&target.join("a.dat"), now);

        let stats = restore_files_into(target, source, Some(backup))
            .await
            .unwrap();

        assert_eq!(stats.conflicts_resolved_local, 1);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(stats.bytes_restored, 0);
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"LOCAL-WORK");
        // No backup was created — `backup` is still empty.
        assert!(std::fs::read_dir(backup).unwrap().next().is_none());
    }

    /// Even with the remote winning by mtime, when `conflict_backup_dir`
    /// is `None` the agent must never overwrite local data. This is the
    /// 1.5.4 fallback for hosts where the conflict root couldn't be
    /// resolved (state_dir missing, permission denied, etc).
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_skips_when_no_backup_dir_provided() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"remote-new");
        write_file(&target.join("a.dat"), b"local-old");
        let now = std::time::SystemTime::now();
        set_mtime(&target.join("a.dat"), now - Duration::from_secs(10));
        set_mtime(&source.join("a.dat"), now + Duration::from_secs(10));

        let stats = restore_files_into(target, source, None).await.unwrap();

        assert_eq!(stats.conflicts_resolved_local, 1);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        // Local content was preserved.
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"local-old");
    }

    /// `cleanup_old_conflicts` walks two levels deep and removes only the
    /// timestamp dirs older than the retention window. The save_id parent
    /// is left in place even after its children disappear.
    #[tokio::test(flavor = "current_thread")]
    async fn cleanup_old_conflicts_respects_ttl() {
        let root_tmp = tempfile::tempdir().unwrap();
        let root = root_tmp.path();

        let old_dir = root.join("save-A").join("2026-04-01T00-00-00Z");
        let fresh_dir = root.join("save-A").join("2026-05-20T00-00-00Z");
        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::create_dir_all(&fresh_dir).unwrap();
        std::fs::write(old_dir.join("dummy.txt"), b"x").unwrap();
        std::fs::write(fresh_dir.join("dummy.txt"), b"x").unwrap();

        let now = std::time::SystemTime::now();
        set_mtime(&old_dir, now - Duration::from_secs(30 * 86_400));
        set_mtime(&fresh_dir, now - Duration::from_secs(60));

        cleanup_old_conflicts(root, Duration::from_secs(14 * 86_400))
            .await
            .expect("cleanup ok");

        assert!(
            !old_dir.exists(),
            "old conflict dir should have been pruned"
        );
        assert!(fresh_dir.exists(), "fresh conflict dir must survive");
        assert!(root.join("save-A").exists(), "save_id parent stays");
    }

    /// `cleanup_old_conflicts` on a non-existent root is a no-op, not an
    /// error. Mirrors the fresh-install case where the conflict dir hasn't
    /// been touched yet.
    #[tokio::test(flavor = "current_thread")]
    async fn cleanup_old_conflicts_handles_missing_dir() {
        let parent = tempfile::tempdir().unwrap();
        let missing = parent.path().join("does-not-exist");
        assert!(!missing.exists());
        cleanup_old_conflicts(&missing, Duration::from_secs(14 * 86_400))
            .await
            .expect("missing root must be no-op");
    }
}
