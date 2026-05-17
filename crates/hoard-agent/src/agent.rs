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

use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::api::ApiClient;
use crate::backup::upload_directory;

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
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            debounce_secs: 5,
            poll_secs: 2,
            max_retries: 5,
            auto_restore: false,
        }
    }
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
    /// `GameStarted` / `GameStopped` transitions. Populated from the
    /// manifest by `autodetect`. Empty list = match by `steam_install_dir`
    /// only.
    #[serde(default)]
    pub processes: Vec<String>,
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
}

/// Why we scheduled a backup. Useful in the UI to explain "the game just
/// closed, so I'm backing it up now" vs "the save folder changed".
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupReason {
    FilesystemSettled,
    GameStopped,
    Manual,
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
    /// Internal: an auto-restore task finished writing files into a slot's
    /// local path. The slot's fs watcher was either never armed (path was
    /// missing on AddSave) or armed against an empty directory — either
    /// way we re-arm it now so the freshly-restored save is being watched.
    /// Not exposed through `AgentHandle` because only the auto-restore
    /// task ever fires it.
    RearmWatcher(String),
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
}

async fn run_agent(
    api: ApiClient,
    config: AgentConfig,
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
    let (done_tx, mut done_rx) = mpsc::channel::<String>(64);

    // Process watcher: periodic poll. We refresh only the bits we care
    // about (process names + exe paths) to keep CPU near zero when idle.
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let mut poll = tokio::time::interval(Duration::from_secs(config.poll_secs.max(1)));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                    if let Some(slot) = slots.get_mut(&save_id) {
                        slot.has_pending = true;
                        slot.last_fs_event_at = Some(OffsetDateTime::now_utc());
                    }
                    tracing::info!(
                        save_id = %save_id,
                        path = %path.display(),
                        "agent: fs event observed; scheduling backup"
                    );
                    schedule_backup(
                        &mut slots, &save_id, BackupReason::FilesystemSettled,
                        Duration::from_secs(config.debounce_secs),
                        &api, &events_tx, &config, &done_tx, &cmd_tx,
                    );
                }
            }

            // ----- Process poll tick -----
            _ = poll.tick() => {
                process_poll(&mut sys, &mut slots, &events_tx, &api, &config, &done_tx, &cmd_tx);
            }

            // ----- Backup success notifications -----
            Some(save_id) = done_rx.recv() => {
                if let Some(slot) = slots.get_mut(&save_id) {
                    slot.has_pending = false;
                    slot.next_scheduled_backup_at = None;
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
    let local_path = save.local_path.clone();
    let mut slot = SaveSlot {
        save,
        watcher: None,
        pending: None,
        is_running: false,
        has_pending: false,
        last_fs_event_at: None,
        next_scheduled_backup_at: None,
    };
    arm_watcher(&mut slot, fs_tx);
    slots.insert(slot.save.save_id.clone(), slot);

    if config.auto_restore && is_path_empty_or_missing(&local_path) {
        spawn_auto_restore(
            save_for_restore,
            api.clone(),
            events_tx.clone(),
            cmd_tx.clone(),
        );
    }
}

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
) {
    tokio::spawn(async move {
        tracing::info!(
            save_id = %save.save_id,
            game_slug = %save.game_slug,
            path = %save.local_path.display(),
            "agent: auto-restore — local path empty/missing, checking server"
        );
        match run_auto_restore(&api, &save).await {
            Ok(Some(outcome)) => {
                tracing::info!(
                    save_id = %save.save_id,
                    version_num = outcome.version_num,
                    files = outcome.files_extracted,
                    bytes = outcome.bytes_extracted,
                    "agent: auto-restore succeeded"
                );
                let _ = events_tx
                    .send(AgentEvent::SaveAutoRestored {
                        save_id: save.save_id.clone(),
                        game_slug: save.game_slug.clone(),
                        version_num: outcome.version_num,
                        files_extracted: outcome.files_extracted,
                        bytes_extracted: outcome.bytes_extracted,
                    })
                    .await;
                // Tell the agent loop to rebuild the fs watcher now that
                // the directory actually has contents.
                let _ = cmd_tx
                    .send(AgentCommand::RearmWatcher(save.save_id.clone()))
                    .await;
            }
            Ok(None) => {
                tracing::info!(
                    save_id = %save.save_id,
                    "agent: auto-restore — server has no snapshots yet; nothing to restore"
                );
            }
            Err(e) => {
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
    });
}

/// Internal restore primitive returning the outcome summary or `None` if
/// the server has no snapshots for this save (in which case auto-restore
/// is a no-op, not a failure).
struct AutoRestoreOutcome {
    version_num: i64,
    files_extracted: u64,
    bytes_extracted: u64,
}

async fn run_auto_restore(
    api: &ApiClient,
    save: &WatchedSave,
) -> Result<Option<AutoRestoreOutcome>> {
    let remote = api.get_save(&save.save_id).await?;
    let Some(version) = remote.latest_version_num else {
        return Ok(None);
    };
    // `force=true` because the directory may exist as an empty stub
    // (Library path with no files yet). `is_path_empty_or_missing` is the
    // gate that decided we're allowed to write here in the first place.
    let opts = crate::restore::RestoreOptions {
        skip_verify: false,
        force: true,
    };
    let outcome = crate::restore::download_snapshot(
        api,
        &save.save_id,
        version,
        &save.local_path,
        opts,
        |_, _| {},
    )
    .await?;
    Ok(Some(AutoRestoreOutcome {
        version_num: version,
        files_extracted: outcome.files_extracted as u64,
        bytes_extracted: outcome.bytes_extracted,
    }))
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
    done_tx: &mpsc::Sender<String>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
) {
    let Some(slot) = slots.get_mut(save_id) else {
        return;
    };
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

    // Don't bother announcing zero-delay manual backups twice — that just
    // adds noise to the activity feed.
    if delay > Duration::ZERO {
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
    let max_retries = config.max_retries;
    let auto_restore = config.auto_restore;

    slot.pending = Some(tokio::spawn(async move {
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        run_backup_with_retry(
            api,
            save,
            events_tx,
            done_tx,
            cmd_tx,
            max_retries,
            auto_restore,
        )
        .await;
    }));
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
async fn run_backup_with_retry(
    api: ApiClient,
    save: WatchedSave,
    events_tx: mpsc::Sender<AgentEvent>,
    done_tx: mpsc::Sender<String>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    max_retries: u32,
    auto_restore: bool,
) {
    if is_path_empty_or_missing(&save.local_path) {
        tracing::info!(
            save_id = %save.save_id,
            path = %save.local_path.display(),
            auto_restore,
            "agent: backup skipped — local folder is empty/missing"
        );
        // Always clear has_pending so a future fs event isn't blocked.
        let _ = done_tx.try_send(save.save_id.clone());
        if auto_restore {
            spawn_auto_restore(save.clone(), api.clone(), events_tx.clone(), cmd_tx);
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

        let outcome = upload_directory(&api, &save.save_id, &save.local_path, |_, _| {}).await;

        match outcome {
            Ok(o) => {
                let _ = events_tx
                    .send(AgentEvent::BackupSuccess {
                        save_id: save.save_id.clone(),
                        version_num: o.snapshot.version_num,
                        total_bytes: o.total_bytes,
                    })
                    .await;
                // Tell the agent loop to clear has_pending. If the channel
                // is full or the agent is shutting down we just drop the
                // signal — worst case we re-upload an unchanged snapshot
                // on the next GameStopped, which is a soft failure.
                let _ = done_tx.try_send(save.save_id.clone());
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
    done_tx: &mpsc::Sender<String>,
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
                save_id: id,
                game_slug,
            });
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
    fn match_save_for_path_finds_exact_and_subpath() {
        let save = WatchedSave {
            save_id: "abc".into(),
            game_slug: "stardew-valley".into(),
            display_name: "Stardew Valley".into(),
            label: "main".into(),
            local_path: PathBuf::from("/tmp/saves/stardew"),
            steam_install_dir: None,
            processes: vec![],
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
        };

        // Short debounce so the test completes well under the 10s timeout.
        let config = AgentConfig {
            debounce_secs: 1,
            poll_secs: 2,
            max_retries: 0,
            auto_restore: false,
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
}
