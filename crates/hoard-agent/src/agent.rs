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
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::api::ApiClient;
use crate::backup::upload_directory;

/// Configuration for the live agent. Defaults are tuned for "responsive
/// enough to feel alive without thrashing the disk":
///
/// - 30 s debounce: long enough to coalesce a save sequence (game writes
///   several files, then settles), short enough to feel fresh.
/// - 5 s process poll: catches "I quit the game" within seconds without
///   hammering `/proc`.
/// - 5 retries with exponential backoff covers "wifi blipped" without
///   pestering the user forever.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub debounce_secs: u64,
    pub poll_secs: u64,
    pub max_retries: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            debounce_secs: 30,
            poll_secs: 5,
            max_retries: 5,
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
    /// Optional install directory (e.g. Steam's `steamapps/common/<game>`)
    /// used for process matching. If `None`, only fs-debounce drives the
    /// scheduler for this save.
    pub steam_install_dir: Option<PathBuf>,
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

/// Commands the host (Tauri command handlers, tests) sends to the agent.
#[derive(Debug)]
enum AgentCommand {
    AddSave(WatchedSave),
    RemoveSave(String),
    BackupNow(String),
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

    let task = tokio::spawn(run_agent(api, config, cmd_rx, events_tx));
    (AgentHandle { tx: cmd_tx }, task)
}

/// Internal per-save bookkeeping.
struct SaveSlot {
    save: WatchedSave,
    /// Active fs debouncer. Holding the value keeps the watcher alive; we
    /// drop it on `RemoveSave`. Needs `Send` because we own it inside the
    /// agent's tokio task.
    _watcher: Option<Debouncer<notify::RecommendedWatcher>>,
    /// Tokio task that fires the debounced backup. Cancelled and recreated
    /// on every fs event so the timer effectively resets.
    pending: Option<tokio::task::JoinHandle<()>>,
    /// Currently-running guess from the last process poll. Drives
    /// GameStarted/Stopped transitions.
    is_running: bool,
}

async fn run_agent(
    api: ApiClient,
    config: AgentConfig,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    events_tx: mpsc::Sender<AgentEvent>,
) {
    let mut slots: HashMap<String, SaveSlot> = HashMap::new();

    // Channel used by every fs watcher — debounced events all funnel here
    // and we route them by path. mpsc::unbounded would be fine since the
    // debouncer already throttles, but we cap at 256 to be defensive.
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(256);

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
                        handle_add(&mut slots, save, &fs_tx);
                    }
                    Some(AgentCommand::RemoveSave(id)) => {
                        if let Some(slot) = slots.remove(&id) {
                            if let Some(p) = slot.pending {
                                p.abort();
                            }
                            // _watcher dropped here, releasing inotify handle.
                        }
                    }
                    Some(AgentCommand::BackupNow(id)) => {
                        if slots.contains_key(&id) {
                            schedule_backup(
                                &mut slots, &id, BackupReason::Manual,
                                Duration::ZERO, &api, &events_tx, &config,
                            );
                        }
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
                    schedule_backup(
                        &mut slots, &save_id, BackupReason::FilesystemSettled,
                        Duration::from_secs(config.debounce_secs),
                        &api, &events_tx, &config,
                    );
                }
            }

            // ----- Process poll tick -----
            _ = poll.tick() => {
                process_poll(&mut sys, &mut slots, &events_tx, &api, &config);
            }
        }
    }
}

/// Build a fs watcher for `save` and insert into the slot map.
fn handle_add(
    slots: &mut HashMap<String, SaveSlot>,
    save: WatchedSave,
    fs_tx: &mpsc::Sender<PathBuf>,
) {
    let watcher = if save.local_path.is_dir() {
        match build_watcher(&save.local_path, fs_tx.clone()) {
            Ok(w) => Some(w),
            Err(e) => {
                tracing::warn!(
                    save_id = %save.save_id,
                    error = %e,
                    "agent: couldn't start fs watcher"
                );
                None
            }
        }
    } else {
        tracing::info!(
            save_id = %save.save_id,
            path = %save.local_path.display(),
            "agent: save path doesn't exist yet — process events will still fire"
        );
        None
    };

    slots.insert(
        save.save_id.clone(),
        SaveSlot {
            save,
            _watcher: watcher,
            pending: None,
            is_running: false,
        },
    );
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
fn schedule_backup(
    slots: &mut HashMap<String, SaveSlot>,
    save_id: &str,
    reason: BackupReason,
    delay: Duration,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    config: &AgentConfig,
) {
    let Some(slot) = slots.get_mut(save_id) else {
        return;
    };
    if let Some(p) = slot.pending.take() {
        p.abort();
    }

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
    let save = slot.save.clone();
    let max_retries = config.max_retries;

    slot.pending = Some(tokio::spawn(async move {
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        run_backup_with_retry(api, save, events_tx, max_retries).await;
    }));
}

/// Upload + retry. Backoff is `2 ** attempt` seconds, capped at 5 min.
/// `max_retries == 0` means "try once and give up on failure".
async fn run_backup_with_retry(
    api: ApiClient,
    save: WatchedSave,
    events_tx: mpsc::Sender<AgentEvent>,
    max_retries: u32,
) {
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
fn process_poll(
    sys: &mut System,
    slots: &mut HashMap<String, SaveSlot>,
    events_tx: &mpsc::Sender<AgentEvent>,
    api: &ApiClient,
    config: &AgentConfig,
) {
    // Refresh every process. The `true` flag asks sysinfo to remove
    // entries for processes that have exited since the last refresh,
    // which is exactly what we need to detect "game stopped".
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    // Build a set of "currently running" save_ids by checking each
    // watched save's install dir against every process exe path. This is
    // O(saves × processes) which is fine — both numbers are small.
    let mut running: HashSet<String> = HashSet::new();
    for slot in slots.values() {
        let Some(install_dir) = slot.save.steam_install_dir.as_ref() else {
            continue;
        };
        for proc in sys.processes().values() {
            if let Some(exe) = proc.exe() {
                if exe.starts_with(install_dir) {
                    running.insert(slot.save.save_id.clone());
                    break;
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
        let game_slug = slots
            .get(&id)
            .map(|s| s.save.game_slug.clone())
            .unwrap_or_default();
        if let Some(slot) = slots.get_mut(&id) {
            slot.is_running = now_running;
        }
        if now_running {
            let _ = events_tx.try_send(AgentEvent::GameStarted {
                save_id: id,
                game_slug,
            });
        } else {
            let _ = events_tx.try_send(AgentEvent::GameStopped {
                save_id: id.clone(),
                game_slug,
            });
            // Game just closed — settle quickly and back up.
            schedule_backup(
                slots,
                &id,
                BackupReason::GameStopped,
                Duration::from_secs(5),
                api,
                events_tx,
                config,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };
        let mut slots = HashMap::new();
        slots.insert(
            "abc".to_string(),
            SaveSlot {
                save,
                _watcher: None,
                pending: None,
                is_running: false,
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
}
