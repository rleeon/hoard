//! Long-running "live agent" that watches tracked saves and backs them up.
//!
//! Three independent loops cooperate inside one Tokio task:
//!
//! 1. Filesystem watcher: `notify-debouncer-mini` aggregates raw inotify
//!    events into a debounced stream. When a save folder settles for
//!    `debounce_secs`, we enqueue a backup.
//! 2. Process watcher: a periodic `sysinfo` poll asks "is any tracked
//!    game's executable running?" and emits `GameStarted` / `GameStopped`
//!    transitions. On stop we also enqueue an immediate backup, since the
//!    user just finished playing.
//! 3. Backup scheduler: drains the queue, runs `upload_directory` per
//!    entry, and applies exponential backoff (`2 ** retry` seconds, capped)
//!    on failure up to `max_retries`.
//!
//! Everything outside the agent talks to it through two channels:
//! - `AgentCommand` (mpsc, in): add or remove watched saves, shut down.
//! - `AgentEvent` (mpsc, out): fire-and-forget notifications the desktop UI
//!   surfaces as Tauri events.
//!
//! The agent never panics on a missing path or a failed upload; those become
//! events the UI can show. Loss-of-network is the common case and we want it
//! to look like "we'll retry in a bit", not a crash.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use hoard_core::kernel;
use hoard_core::kernel::correlation::accept_correlation_signals;
use hoard_core::wire::VersionOrigin;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::{Deserialize, Serialize};
use sysinfo::{
    Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, RefreshKind, System, UpdateKind,
};
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant as TokioInstant;

use crate::api::{ApiClient, ApiError};
use crate::backup::{upload_directory_checked, BackupResult, ServerHead};

/// Configuration for the live agent. Defaults are tuned for v0.3's
/// "instant feel" priority:
///
/// - **5 s debounce**: short enough that auto-backup feels immediate
///   after a save, long enough to coalesce torn writes (Bethesda games,
///   Souls games re-write the save file mid-burst). v0.2's 30 s default
///   was much more conservative; product call to match the user's ask.
/// - **2 s process poll** *while a game is running*: catches "I quit the
///   game" within seconds. When idle the poll backs off to
///   `poll_secs * IDLE_POLL_MULT` (the common case is no game running, so
///   this keeps `/proc` scans, the agent's dominant idle cost, rare). The
///   refresh itself is name+exe only, never the full per-process snapshot.
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
    /// (ADR 0018, axis A: data saving). After a backup succeeds, the agent won't
    /// start another for this save until the interval elapses; intermediate writes
    /// coalesce into the next one (the final state is always uploaded). It kills
    /// the "one version per minute" cadence of games that autosave every few
    /// seconds (OpenTTD). `0` disables the floor (every settle backs up, with no
    /// wait). The desktop derives this from `Prefs::data_saving` via
    /// `min_snapshot_interval_for`: the slider's low band, the default included,
    /// gives `0`; only pushing towards saving produces a floor, up to 600 s.
    pub min_snapshot_interval_secs: u64,
    /// Mirror of `Prefs::global_sync`. Distinct from [`Self::auto_restore`]:
    /// it opts every save into restore (same effect as `auto_restore` on the
    /// eligibility floor) *and* unlocks the low-latency pull paths, which are
    /// poller/SSE `ForceRestore` push and the pre-launch sync barrier on
    /// `GameStarted`. The version-gate inside `run_auto_restore`
    /// (`known >= latest`) still holds, so it never re-downloads a save the
    /// device is already current on. Backup-only presets
    /// (`policy.auto_restore == Some(false)`) still opt out.
    ///
    /// It does **not** bypass the "user is mid-session" guards (`is_running`,
    /// `has_pending`, recent-fs-event, recent-mtime). It used to ("pull the
    /// moment it's outdated, even while playing") and on a single device
    /// that raced the user's own backup: the pull re-applied the last
    /// *uploaded* version over progress the debounced backup hadn't flushed
    /// yet, so intermediate sessions never got versioned (REPO data-loss
    /// incident, 2026-07-05). A mid-session pull is never needed for
    /// correctness: if another device genuinely advanced the save, our next
    /// upload gets a 409 non-fast-forward and the reconcile path merges the
    /// remote head in before retrying. So outdated-while-playing now defers
    /// to the reconciliation sweep, which catches up as soon as the session
    /// settles, and a deferred `ForceRestore` that finds un-flushed local
    /// changes flushes them immediately (the reductor marks that flush
    /// *urgent*, so it skips the data-saving min-interval floor, but never an
    /// error backoff), so live progress becomes a cloud version within seconds
    /// instead of waiting out the debounce window. There is no guarded-path
    /// exception left: the pre-launch barrier died with the inversion of
    /// `run_agent` (ADR 0021 Slice 2b): the tick is the only authority.
    pub global_sync: bool,
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
            global_sync: false,
        }
    }
}

/// The data-saving slider's threshold below which NO floor is imposed between
/// snapshots: the change goes up as soon as the debounce settles, with no "queued,
/// waiting". It covers the factory default (`data_saving = 0.3`) so the user never
/// sees an upload waiting unless they deliberately asked to save data.
const DATA_SAVING_NO_FLOOR_UPTO: f64 = 0.4;

/// Maps the user's `data_saving` knob (0..=1) to a minimum snapshot interval in
/// seconds (ADR 0018, decision 4). The low band (`k <= DATA_SAVING_NO_FLOOR_UPTO`,
/// the default included) returns `0`: no wait, and the upload is immediate after the
/// debounce. Above the threshold the floor grows linearly up to 600 s (`k = 1`,
/// maximum saving, about ten minutes between snapshots). Presets with an explicit
/// floor (`short_session` 30 s, `data_saver` 600 s) still decide per save.
pub fn min_snapshot_interval_for(data_saving: f64) -> u64 {
    let k = data_saving.clamp(0.0, 1.0);
    if k <= DATA_SAVING_NO_FLOOR_UPTO {
        return 0;
    }
    let t = (k - DATA_SAVING_NO_FLOOR_UPTO) / (1.0 - DATA_SAVING_NO_FLOOR_UPTO);
    (600.0 * t).round() as u64
}

/// The *minimal* process-refresh set the agent actually consumes. The process
/// poll reads each process's `name()` (always populated, no flag), its `exe()`
/// for the legacy install-dir fallback, and its `cpu_usage()` to spot a
/// just-launched untracked game (see `process_poll`). Everything else
/// `ProcessRefreshKind::everything()` pulls (memory, disk I/O, environ,
/// cmdline, cwd, root, user) is dead weight re-read from `/proc/<pid>/*` for
/// every process on the box on every tick, and was the bulk of the agent's
/// idle CPU. `OnlyIfNotSet` reads each `exe` path exactly once per PID (it never
/// changes); `with_cpu` adds no per-process file read, since utime and stime come from
/// the same `/proc/<pid>/stat` already parsed for the name, plus a single
/// global `/proc/stat` read per tick, so steady-state ticks stay cheap.
///
/// `Process::status()` (see [`is_defunct`]) needs no flag of its own and adds
/// no cost: `ProcessRefreshKind` has no switch for it because sysinfo always
/// populates it from the state field of that same already-parsed
/// `/proc/<pid>/stat` (macOS/Windows fill it from the process snapshot they
/// already walk). The zombie filter is free.
fn proc_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::new()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cpu()
}

/// Is this process listed by the OS but no longer able to run code? A zombie
/// has already exited and only lingers because its parent hasn't reaped it,
/// which under Proton is routine: the game quits, the wine supervisor leaves
/// the .exe defunct, and the entry can sit there until the prefix tears down
/// (on a Steam Deck, often not before the next reboot).
///
/// It matters because a defunct entry keeps its name and its exe path, so every
/// strong matcher in [`process_poll`] (name, identity token, open handles,
/// install dir) went on matching it and the slot stayed `is_running` for good.
/// That pinned the mid-session veto open and a save pushed from another device
/// never landed. A zombie cannot be writing a save file, so it cannot be
/// evidence that the user is playing.
fn is_defunct(status: ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Zombie | ProcessStatus::Dead)
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
    /// `GameStarted` and `GameStopped` transitions. Rarely populated (only the
    /// curated `builtin_processes_for` list: Minecraft, emulators); the TOML catalog
    /// that fed it was removed in 1.5.0. With the list empty the match does NOT stop
    /// at `steam_install_dir`: the poll also matches by generic identity (the
    /// process's name or folder against the game's slug, list-free and
    /// cross-platform; see `game_identity_tokens` and
    /// `process_identity_candidates`).
    #[serde(default)]
    pub processes: Vec<String>,
    /// [`Self::processes`] are shared with other tracked saves, so seeing the
    /// process does not identify which of them is being played. See
    /// [`crate::state::SaveState::shared_processes`].
    #[serde(default)]
    pub shared_processes: bool,
    /// Resolved per-save sync overrides (from the save's preset). Empty by
    /// default = inherit every global `AgentConfig` setting. The agent reads
    /// `policy.<field>.unwrap_or(config.<field>)` at each decision point. See
    /// [`crate::presets`].
    #[serde(default)]
    pub policy: crate::presets::SavePolicy,
    /// The user's standing yes to writing THIS game's config back on restore.
    /// See [`crate::state::SaveState::allow_device_local`]. `None`/`Some(false)`
    /// = gate shut, as always.
    #[serde(default)]
    pub allow_device_local: Option<bool>,
    /// Cloud version this device last committed or restored, read from
    /// `state.json` (`last_version_num`). Seeds the slot's `known_version`
    /// so the reconciliation sweep's version-gate is armed from the first
    /// tick after a restart: without it every restart re-downloads every
    /// snapshot to diff and drains the bandwidth quota. `None` for a
    /// freshly tracked save (nothing committed yet) is correct: the gate
    /// stays open so an empty/new device still pulls.
    #[serde(default)]
    pub known_version: Option<i64>,
    /// Skip-by-set-hash signature of the last successful upload, read from
    /// `state.json` (`set_hash`). Seeds the slot's `last_set_hash` so the
    /// first backup sweep after a restart can compare against it and skip a
    /// no-op upload instead of re-pushing an identical snapshot. `None` for a
    /// freshly tracked save (nothing committed yet). Without this every app
    /// restart re-uploads every save as a new identical version.
    #[serde(default)]
    pub set_hash: Option<String>,
    /// PLAYTIME-ONLY tracking: this entry is here purely to count hours played
    /// for the recap (hoard-wrapple), never to back up a save. A `track_only`
    /// slot arms no fs watcher and is skipped by every backup/restore/sweep
    /// path; the process poll still matches it (by `processes` / install dir)
    /// so [`crate::playtime`] accrues its time. Used for always-online games
    /// with no local save worth syncing (Fortnite, Rust, Valorant…). Surfaced
    /// in amber in the UI. `default` keeps older `state.json` files loading.
    #[serde(default)]
    pub track_only: bool,
}

/// The event and per-slot status contract lives in the leaf kernel: with the engine
/// in its own process (`hoardd`) these types cross the socket, so a client cannot
/// need the engine's crate to read them (ADR 0021, part A and C.6). They are
/// re-exported here, so `hoard_agent::agent::AgentEvent` is still the right path for
/// the desktop and the CLI.
pub use hoard_core::ipc::events::{AgentEvent, AgentSlotStatus, BackupReason};

/// How a spawned auto-restore attempt ended. Drives how the slot's
/// `next_auto_restore_at` is re-armed and whether the consecutive-failure
/// counter moves, so the three error classes stay visibly distinct:
/// a 404 is permanent-ish, a 401 isn't the save's fault, and everything else
/// is the transient-or-chronic case the escalating backoff exists for.
#[derive(Debug, Clone)]
enum AutoRestoreDisposition {
    /// The attempt finished without error (restored, or nothing to pull).
    /// Resets the failure counter and any stuck state.
    Ok,
    /// 404: the save has no record/snapshot on the backend we're talking to
    /// (carried over from another account, stale state, remote purged). Parks
    /// the slot on the long not-found backoff. Not a "failure" for backoff
    /// purposes: retrying can't conjure a snapshot that doesn't exist, and
    /// this arm already paces itself.
    NotOnServer,
    /// 401: session-wide, not this save's problem. The stored cloud JWT is
    /// expired and the refresh hasn't landed in this client yet. Swallowed and
    /// left on the normal short cooldown so it retries as soon as the token is
    /// back. Deliberately does *not* touch the failure counter: counting a
    /// token blip toward "this save is stuck" would let one expired session
    /// mark every tracked save as broken.
    Unauthorized,
    /// 429: the server's rolling bandwidth limiter deferred this download. Like
    /// [`Self::Unauthorized`] it isn't this save's fault and must *not* touch the
    /// failure counter: counting a throttle toward "stuck" is exactly what made
    /// a busy reconciliation sweep spam "keeps failing to restore (3×)". Carries
    /// the server's `retry_after_secs` so the slot re-arms on the exact window
    /// slide instead of the generic 60s cooldown. Swallowed (no failure toast).
    Throttled { retry_after_secs: u32 },
    /// Any other error (network, sha mismatch, permission denied, timeout).
    /// Carries the formatted error chain for the event. Escalates the
    /// consecutive-failure counter and the backoff.
    Failed(String),
}

/// Commands the host (Tauri command handlers, tests) sends to the agent.
enum AgentCommand {
    // Boxed: `WatchedSave` is much larger than the other variants, so keeping
    // it inline made every `AgentCommand` value as big as a `WatchedSave`.
    AddSave(Box<WatchedSave>),
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
    /// missing on AddSave) or armed against an empty directory. Either
    /// way we re-arm it now so the freshly-restored save is being watched.
    /// Not exposed through `AgentHandle` because only the auto-restore
    /// task ever fires it.
    RearmWatcher(String),
    /// Internal: a spawned auto-restore task finished (success or failure).
    /// Clears `slot.restoring` so the reconciliation sweep can try again
    /// next tick. `outcome` decides how the slot is re-armed; see
    /// [`AutoRestoreDisposition`].
    AutoRestoreFinished {
        id: String,
        disposition: AutoRestoreDisposition,
        /// The cloud version this slot is now synced to (the latest the restore
        /// observed), so the slot can remember it and the reconciliation sweep
        /// skips re-downloading the same version next tick. `None` when the
        /// attempt didn't reach a known version (404, transient failure).
        synced_version: Option<i64>,
        /// Post-merge set signature to adopt as the slot's `last_set_hash`.
        /// Only `Some` when the merged tree equals head exactly (no local
        /// divergence): adopting it makes the fs events the merge triggered
        /// settle as a `Skipped` no-op instead of firing a redundant upload of
        /// content already on the server. `None` leaves `last_set_hash` alone
        /// so a genuinely-diverged tree still uploads.
        post_restore_set_hash: Option<String>,
        /// Whether this attempt actually wrote files into the folder (restored
        /// or conflict-backed-up ≥1 file), as opposed to a no-op "already
        /// synced" pass. Only a real write bumps the folder mtime / echoes fs
        /// events, so only then do we stamp `last_restore_at` to keep the
        /// restore from vetoing the next pull (see `mid_session_reason`).
        wrote_files: bool,
    },
    /// Live-toggle `config.auto_restore` so the user's Settings change
    /// reaches the running agent without a restart. When flipped from
    /// `false → true` the agent also kicks an immediate reconciliation
    /// sweep so any tracked save with an empty local folder gets restored
    /// right away.
    SetAutoRestore(bool),
    /// Live-toggle `config.global_sync` (sync global). Distinct from
    /// `SetAutoRestore`: when flipped `false → true` the agent kicks an
    /// immediate sweep so every outdated-but-idle save catches up right away.
    /// See [`AgentConfig::global_sync`].
    SetGlobalSync(bool),
    /// Global sync, the low-latency path: the `cloud_pull` poller (or the
    /// self-hosted SSE) spotted that a particular save moved forward a version and
    /// asks to pull it now, skipping the sweep's cooldown. It honours the `restoring`
    /// flag (restores never overlap), the backup-only opt-out from a preset, and the
    /// live-session guards (`is_running`, `has_pending`, recent activity): with the
    /// game open the pull is NOT discarded but recorded in `SaveSlot::pull_pending`
    /// and executed when the game closes. The version gate inside `run_auto_restore`
    /// avoids the download when we are already up to date.
    ///
    /// `version_num` is the remote head when the caller already has it (SSE).
    /// Without it this is only an early tick: the reducer still needs `cloud_heads`
    /// populated, which self-hosted now fills via `list_saves`.
    ForceRestore {
        save_id: String,
        version_num: Option<i64>,
    },
    /// Detection (phase 3, ADR 0020): the list of candidate folders that have been
    /// detected but NOT yet tracked, which the desktop's scan wants probed. The agent
    /// watches their mtime on every process tick: if one is rewritten while a game is
    /// alive, it records the process-to-write correlation, the same +0.50 signal only
    /// already-tracked saves used to earn. It breaks the chicken and egg: playing an
    /// untracked game finally leaves a trace, and the next scan promotes it to `High`
    /// and auto-tracks it. It replaces the whole set on every call.
    SetProbeCandidates(Vec<PathBuf>),
    /// Internal: a backup task exhausted its retry budget and failed for real.
    /// Sent by `run_backup_with_retry` instead of just giving up, because
    /// giving up wedged the slot: no `BackupDone` is emitted on this path (the
    /// local changes are still un-versioned, so `has_pending` must stay set to
    /// keep every restore off them) and `has_pending` is itself a
    /// `mid_session_reason` veto, so the save could neither be uploaded nor
    /// pulled until the user happened to write the folder again. The handler
    /// feeds it to the reductor as `OpResult::Failed`, which re-arms the upload
    /// on [`kernel::reconcile::BACKUP_FAILURE_BACKOFF_SECS`], the recovery path
    /// that doesn't depend on a new fs event.
    RetryBackupAfterFailure(String),
    /// Internal: the upload hit a 409 the reconcile couldn't resolve, so the
    /// server says we're behind, yet there is nothing newer to pull. Same
    /// wedge-avoidance contract as [`AgentCommand::RetryBackupAfterFailure`] (no
    /// `BackupDone`, `has_pending` survives), but fed to the reducer as
    /// [`kernel::OpResult::ConflictStalled`] so it escalates on
    /// [`kernel::reconcile::CONFLICT_STALL_BACKOFF_SECS`] and, after
    /// [`kernel::reconcile::CONFLICT_STALL_GIVE_UP_AFTER`] of them, stops
    /// retrying and asks for a human.
    ///
    /// It used to send `RetryBackupAfterFailure`, which re-armed the upload on a
    /// flat ten-minute backoff with no counter: 1,701 events across 5 users, and
    /// one save stuck at ~4.5 attempts/h for 14 days through three app versions.
    ParkBackupConflict {
        id: String,
        /// The 409's chain, for the warning the UI shows if the budget runs out. The
        /// reducer does not carry it (its `ConflictStalled` carries no text) so the
        /// shell keeps it, exactly as with `last_restore_error`.
        error: String,
    },
    /// Internal: the upload hit a 402: the whole account is out of storage.
    /// Same wedge-avoidance contract as `RetryBackupAfterFailure` (no
    /// `BackupDone`, `has_pending` survives), but fed to the reductor as
    /// [`kernel::OpResult::QuotaFull`] so it parks on the long
    /// [`kernel::reconcile::QUOTA_FULL_BACKOFF_SECS`] instead of retrying every
    /// ten minutes against a wall that only a human can move.
    ParkBackupQuotaFull(String),
    /// Internal: the upload hit a **budget** 429: a rolling bandwidth window,
    /// the storage quota, or the server's loop brake. Same wedge-avoidance
    /// contract as the two above (no `BackupDone`, `has_pending` survives), fed
    /// to the reducer as [`kernel::OpResult::Throttled`] so the wait the server
    /// actually asked for is the one we sit out.
    ///
    /// Separate from [`AgentCommand::ParkBackupQuotaFull`] because the wait is
    /// the server's number, not ours: a bandwidth window slides in minutes while
    /// a full account needs an hour, and answering both with the same constant
    /// is how a client ends up hammering one or sleeping through the other.
    ParkBackupThrottled {
        id: String,
        retry_after_secs: u32,
    },
    /// Latest known cloud version per save id, as last seen by the `cloud_pull`
    /// poller's manifest. The poller already fetches the full manifest once per tick,
    /// so it hands the map to the agent and the reconciliation sweep can version-gate
    /// locally instead of each `run_auto_restore` re-fetching the same manifest
    /// (cloud) or hitting `get_save` per candidate (the old N+1). It replaces the
    /// whole map each call. Cloud pollers send the full manifest; self-hosted SSE
    /// uses [`AgentCommand::ForceRestore`] to merge one head instead (it must not
    /// replace the map). The engine also fills this itself via
    /// [`AgentCommand::CloudHeadsObserved`].
    ///
    /// Since ADR 0021 D.12 this is a latency hint rather than the only source: the
    /// engine observes the head itself ([`Self::CloudHeadsObserved`]) when this push
    /// is late, so a dead poller no longer leaves it blind.
    SetCloudVersions {
        versions: HashMap<String, i64>,
        /// `(game_slug, label)` → `save_id`, from the same manifest the versions
        /// came from. See [`CloudHeads::aliases`] for why a version map keyed by
        /// the server's ids alone can't answer for every tracked save.
        aliases: HashMap<(String, String), String>,
    },
    /// Internal (ADR 0021 D.12): the result of the cloud observation the engine
    /// itself triggers from its tick. The query lives in the shell, since the kernel
    /// does no IO, and enters the reducer as part of the `Observation`.
    CloudHeadsObserved {
        /// `Some` = the list arrived (full `save_id` → version map, replaces
        /// the cache). `None` = the attempt produced no heads (network, 401,
        /// unresolved mode): freshness is **not** sealed, so blindness stays
        /// visible instead of looking like a live feed.
        versions: Option<HashMap<String, i64>>,
        /// `(game_slug, label)` → `save_id`, from the same manifest pass. Lets
        /// the cache answer for a save whose local id the cloud has never seen
        /// (see [`CloudHeads::aliases`]).
        aliases: Option<HashMap<(String, String), String>>,
        /// What content each head has (the manifest's digest), for D.8.3's
        /// anti-relaunch check. It travels alongside `versions` and comes from the
        /// same pass over the manifest, so both describe the same instant of the
        /// server.
        digests: Option<HashMap<String, ServerHead>>,
        /// The deployment's context according to the cached `/v1/health` probe:
        /// `Some(true)` cloud, `Some(false)` self-hosted, `None` unresolved (a failed
        /// probe). Only a definite value moves the latch.
        is_cloud: Option<bool>,
    },
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
        self.tx.send(AgentCommand::AddSave(Box::new(save))).await?;
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
    /// "agent diagnostics" panel: it surfaces the same internal state we'd
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

    /// Push a new `global_sync` preference into the running agent. On a
    /// `false → true` flip the agent immediately sweeps every slot, pulling
    /// any outdated save that isn't mid-session (the version-gate keeps it
    /// free when the device is already current). See
    /// [`AgentConfig::global_sync`].
    pub async fn set_global_sync(&self, enabled: bool) -> Result<()> {
        self.tx.send(AgentCommand::SetGlobalSync(enabled)).await?;
        Ok(())
    }

    /// Ask the agent to pull a specific save's latest cloud version right now,
    /// bypassing the sweep cooldown. Used by the `cloud_pull` poller when sync
    /// global is on and it spots a save that advanced server-side, so the
    /// download starts within the poll interval instead of up to a cooldown
    /// later. No-op on the agent side if the save is unknown or already
    /// restoring; deferred to the sweep if the save is mid-session.
    /// See [`AgentCommand::ForceRestore`].
    pub async fn force_restore(&self, save_id: String) -> Result<()> {
        self.force_restore_at(save_id, None).await
    }

    /// Like [`Self::force_restore`], but merge `version_num` into the head
    /// cache first so `cloud_ahead` can fire on self-hosted (no cloud manifest).
    pub async fn force_restore_at(&self, save_id: String, version_num: Option<i64>) -> Result<()> {
        self.tx
            .send(AgentCommand::ForceRestore {
                save_id,
                version_num,
            })
            .await?;
        Ok(())
    }

    /// Hand the agent the latest set of untracked candidate folders to probe
    /// for process↔write correlation (ADR 0020 fase 3). The desktop calls
    /// this after each automatic scan with the detected-but-untracked dirs.
    pub async fn set_probe_candidates(&self, dirs: Vec<PathBuf>) -> Result<()> {
        self.tx.send(AgentCommand::SetProbeCandidates(dirs)).await?;
        Ok(())
    }

    /// Feed the agent the latest cloud version per save id, as observed by the
    /// `cloud_pull` poller's manifest. Lets the reconciliation sweep skip the
    /// per-save metadata fetch it would otherwise make. See
    /// [`AgentCommand::SetCloudVersions`].
    pub async fn set_cloud_versions(
        &self,
        versions: HashMap<String, i64>,
        aliases: HashMap<(String, String), String>,
    ) -> Result<()> {
        self.tx
            .send(AgentCommand::SetCloudVersions { versions, aliases })
            .await?;
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
                let _ = cmd_tx_seed.send(AgentCommand::AddSave(Box::new(s))).await;
            }
        });
    }

    // The agent loop needs its own clone of `cmd_tx` so background tasks
    // it spawns (auto-restore is the only one today) can post commands
    // back to it, e.g. `RearmWatcher` after files land on disk.
    let cmd_tx_loop = cmd_tx.clone();
    let task = tokio::spawn(run_agent(api, config, cmd_rx, cmd_tx_loop, events_tx));
    (AgentHandle { tx: cmd_tx }, task)
}

/// Signal from a finished backup task back to the agent loop.
struct BackupDone {
    save_id: String,
    /// `Some` when a new snapshot was uploaded, carrying the fresh set
    /// signature to cache on the slot. `None` when the backup was skipped
    /// (unchanged) or the folder was empty, so the slot keeps its previous
    /// signature.
    new_set_hash: Option<String>,
    /// `true` only when a real snapshot reached the server. The min-interval
    /// throttle anchors on `last_backup_at`, which must advance **only** on a
    /// genuine upload. A skip (unchanged bytes) or an empty/missing folder is
    /// not a backup: if it bumped the anchor, the next real change would be
    /// throttled a full `min_snapshot_interval_secs` out, and with
    /// auto-restore re-emptying the folder each cycle, the anchor would keep
    /// advancing on phantom "backups" and a short play session would never
    /// flush its progress before the game closed (R.E.P.O. regression).
    committed: bool,
    /// Version number of the snapshot just uploaded (`Some` only when `committed`,
    /// or when the content was already up there; see [`Self::landed`]). The agent
    /// advances the slot's `known_version` to this so the reconciliation sweep won't
    /// re-download a version this device itself just produced. `None` on skip or
    /// empty.
    version_num: Option<i64>,
    /// Nothing was uploaded because it was already uploaded (ADR 0021 D.8.3): the
    /// local content is that of the version the server publishes as its head. It
    /// travels to the `Observation` as `upload_landed`, where the reducer uses it to
    /// tell this no-op from the 409 settled onto the head: that one wrote into the
    /// folder (and stamps `last_restore_at`), this one touched nothing.
    landed: bool,
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
    /// The session in progress started on a weak signal alone (folder-to-process
    /// correlation) and no strong signal has corroborated it since. If it also ends
    /// without a single write to the folder, it was a phantom session: the
    /// correlated process was not the game, and the observation takes a strike
    /// ([`CorrelationStore::strike_phantom`]) so a poisoned attribution (an hourly
    /// task, a resident) discards itself instead of vetoing mid-session sync
    /// forever.
    weak_session: bool,
    /// Last poll at which this slot's process was seen running. Powers the
    /// stop-debounce (`RUNNING_STICKY_SECS`): a correlation match is CPU-gated,
    /// so a Paradox game idling in a menu or on a loading screen can dip below
    /// the threshold for a tick and drop out of the running set. Without a
    /// grace window that flaps GameStarted/Stopped (and its final-flush backup)
    /// every few seconds. We keep the slot "running" until this is older than
    /// the grace. `None` until first seen running.
    last_running_seen: Option<TokioInstant>,
    /// Has the save folder changed since the last successful backup?
    /// Drives the v0.3 "final-flush-only-if-pending" rule on `GameStopped`
    /// so there is no point re-uploading an unchanged save just because the user
    /// quit. Set on every fs event; cleared on backup success.
    has_pending: bool,
    /// Most recent debounced fs event observed for this slot. Surfaced via
    /// `AgentSlotStatus` so the diagnostics panel can prove the watcher
    /// is actually seeing writes.
    last_fs_event_at: Option<OffsetDateTime>,
    /// When our own auto-restore last *wrote files* into this slot's folder
    /// (UTC). A restore bumps the folder mtime and echoes fs events, which would
    /// otherwise trip the `mid_session_reason` "folder touched recently" /
    /// "fs event observed recently" vetoes and throttle the NEXT cross-device
    /// pull for a whole `RECENT_SAVE_GRACE`, so back-to-back saves from another
    /// device landed at most one per window on the receiver. This lets the veto
    /// tell our own restore writes apart from the user's. Only set when files
    /// were actually applied (not on a no-op "already synced" pass).
    last_restore_at: Option<OffsetDateTime>,
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
    /// until the first success this session. Owned by the kernel now (mapped
    /// to [`kernel::State::last_backup_at`]); the reductor advances it on a
    /// committed backup.
    last_backup_at: Option<OffsetDateTime>,
    /// The window and count of recent commits: the adaptive floor's memory, which
    /// groups a game whose autosave is rewritten every few seconds. They map to
    /// [`kernel::State::burst_since`] and `burst_backups`, and like all the pacing
    /// they live only in memory: when the engine starts a save begins with no burst
    /// and its first copy goes out at once.
    burst_since: Option<OffsetDateTime>,
    burst_backups: u32,
    /// The IO operation in flight for this slot (anti-relaunch, ADR 0021 C.1): while
    /// it is `Some`, the reducer holds ("operation in flight") rather than relaunching
    /// a multi-GB upload or download. It replaces the old `restoring: bool` and now
    /// tells a backup from a restore. The reducer sets it when emitting `Act(Backup)`
    /// or `Act(Restore)`; the shell clears it when it ingests the matching
    /// [`kernel::OpResult`]. It maps 1:1 to [`kernel::State::in_flight`].
    in_flight: Option<kernel::Op>,
    /// The earliest instant for the next backup because of an error backoff (an
    /// upload 429, or exhausted upload retries). `None` means no brake. The
    /// min-interval floor does not live here: the reducer derives it from
    /// `last_backup_at` (see `kernel::reconcile`). An `OffsetDateTime` rather than a
    /// `TokioInstant` because the kernel is sans-IO and compares against `world.now`;
    /// the conversion lives here in the shell (ADR 0021 D.7). It maps to
    /// [`kernel::State::next_backup_at`].
    next_backup_at: Option<OffsetDateTime>,
    /// The earliest instant for the next restore (cooldown, failure backoff or
    /// download throttle backoff). It used to be an `Option<TokioInstant>`; now an
    /// `OffsetDateTime` to match the kernel.
    /// Mapea a [`kernel::State::next_restore_at`].
    next_restore_at: Option<OffsetDateTime>,
    /// The restore failure escalation per cloud version (404, 401 and 429 do not
    /// count). The reducer escalates and resets it on ingesting a result; the shell
    /// reads it to emit the stuck and recovered events. It used to be
    /// `AutoRestoreFailures`, with methods in the shell; now it is the kernel's pure
    /// [`kernel::RestoreFailures`] and the logic lives in the reducer.
    restore_failures: kernel::RestoreFailures,
    /// The escalation for the 409 reconciliation cannot resolve (the server says
    /// "you are behind" and there is nothing to pull). The reducer escalates it,
    /// resets it and decides when the budget runs out; the shell reads
    /// `needs_attention`'s edge to warn the user. It maps to
    /// [`kernel::State::backup_conflict`].
    backup_conflict: kernel::ConflictStall,
    /// The skip-by-set-hash signature of this session's last successful upload
    /// (ADR 0019). Compared against the freshly walked signature before each backup;
    /// an unchanged signature means the watcher fired on a no-op settle, so the
    /// upload is skipped. In-memory only: cross-restart persistence is the CLI's and
    /// desktop's job via `state.json`. It is still the composite
    /// `"<cheap>:<content>"` that `run_backup_with_retry` consumes; its cheap half
    /// feeds the kernel's fingerprint (`synced_fingerprint`).
    last_set_hash: Option<String>,
    /// The `u64` fingerprint of the local content already synced, for the reducer's
    /// "converged means zero actions" invariant (it kills the R2 compression hot
    /// loop). It is the hash of [`Self::last_set_hash`]'s cheap half: the same
    /// function as the fingerprint sampled in the L1 observation, so identical
    /// content gives the same `u64` and the reducer holds. It maps to
    /// [`kernel::State::synced_fingerprint`].
    synced_fingerprint: Option<u64>,
    /// The folder's own mtime (its inode) as seen on the last tick: the gate on L1
    /// sampling. We only re-hash (`walk_source` plus `compute_set_signature`) when
    /// this mtime changed, when the watcher set [`Self::needs_l1`], or on a sweep or
    /// manual copy, never on every tick (ADR 0021 C.1, tiered L0/L1 observation).
    last_l0_mtime: Option<OffsetDateTime>,
    /// Forces the L1 fingerprint to be computed on the next tick even when the L0
    /// mtime has not changed: set by the fs watcher (an in-place rewrite in a
    /// subdirectory does not move the folder's own mtime), by the hourly sweep
    /// (`SweepAll`) and by a manual backup (`BackupNow`). Cleared after sampling.
    needs_l1: bool,
    /// The user asked for this copy by hand (`BackupNow`) and it has not gone out
    /// yet. The backup's launch consumes it to label the version deliberate, which
    /// is what protects it from a burst of automatic copies pushing it out of the
    /// history. It is cleared on launch rather than on completion: if the upload
    /// fails and the reducer retries it, the retry is still the copy the user asked
    /// for.
    manual_requested: bool,
    /// The result of an IO op that just finished, queued for the next `reconcile` to
    /// ingest (clearing `in_flight`, updating the bookkeeping and backoff). In the
    /// inverted model an op finishing is an *input* to the reducer rather than an
    /// event that mutates state on its own. It maps to
    /// [`kernel::Observation::op_result`].
    pending_op_result: Option<kernel::OpResult>,
    /// The answer from the content-addressed anti-relaunch check, queued alongside
    /// the `pending_op_result` of an upload that uploaded nothing because the content
    /// was already on the server (ADR 0021 D.8.3). It maps to
    /// [`kernel::Observation::upload_landed`].
    pending_upload_landed: Option<bool>,
    /// The error chain of the last failed restore, queued alongside a
    /// `pending_op_result` of `Failed`. The reducer does not carry it (its
    /// `OpResult::Failed` carries no string) so the shell keeps it for the
    /// [`AgentEvent::SaveAutoRestoreStuck`] event it emits on crossing the
    /// threshold.
    last_restore_error: Option<String>,
    /// The last unresolvable 409's chain, queued alongside a `pending_op_result` of
    /// `ConflictStalled`. As with [`Self::last_restore_error`]: the reducer carries
    /// no text, and the `BackupNeedsAttention` event has to say why.
    last_conflict_error: Option<String>,
    /// Cloud version this slot is known to be synced to, advanced on a genuine
    /// upload commit and after a successful auto-restore. The reconciliation
    /// sweep passes it to `run_auto_restore`, which skips the download-to-diff
    /// when the server's latest version isn't newer than this. `None` until the
    /// first commit/restore this session (the first sweep then downloads once to
    /// establish the baseline). This is what stops the every-tick re-download
    /// that used to burn the cloud bandwidth quota: a real cross-device update
    /// (another device committed a higher version) still pulls; our own folder
    /// churn no longer does.
    known_version: Option<i64>,
    /// A cross-device update is waiting to land in this slot, but a pull was vetoed
    /// by [`mid_session_reason`]. Set instead of dropping the `ForceRestore`
    /// outright: "the sweep re-runs every tick, so it lands as soon as the session
    /// settles" assumed the session ends. On a Steam Deck it often doesn't: suspend
    /// and resume keeps the game alive across days, and Proton regularly leaves the
    /// process behind after the user quits, so the veto held forever and a save made
    /// on another device only showed up after a Steam restart. Consumed by the
    /// reducer on the first tick where the slot is quiet (game closed, nothing
    /// pending). It maps to [`kernel::State::pull_pending`].
    pull_pending: bool,
    /// Has [`AgentEvent::RestoreDeferred`] already gone out for the update
    /// currently waiting? The reductor re-evaluates the veto every tick, so
    /// without this the feed would take one "waiting" line per save per tick.
    /// Cleared when the game starts (a new session earns a new notice) and when
    /// the deferred pull finally fires. Mapea a
    /// [`kernel::State::deferred_notified`].
    deferred_notified: bool,
}

/// The kernel's deterministic RNG seed for this save (ADR 0021 C.2): the throttle
/// backoff's jitter has to be reproducible, so it derives from the `save_id` rather
/// than from `thread_rng`. An injectable replica of the shell's original
/// `hash(id) % 6`.
fn seed_for(save_id: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(save_id, &mut h);
    std::hash::Hasher::finish(&h)
}

/// A process-stable `u64` hash of a set signature. It is only ever compared within
/// one run (the sampled fingerprint against the synced one), so `DefaultHasher` is
/// enough; it needs no cross-restart stability.
fn fingerprint_of(sig: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(sig, &mut h);
    std::hash::Hasher::finish(&h)
}

/// The kernel's fingerprint from the composite `"<cheap>:<content>"` the backup
/// persists: it takes the cheap half (the signature over paths, sizes and mtimes),
/// which is exactly what [`observe_local_fingerprint`] samples. So identical content
/// gives the same `u64`.
fn fingerprint_from_set_hash(composite: &str) -> u64 {
    let cheap = composite.split_once(':').map_or(composite, |(c, _)| c);
    fingerprint_of(cheap)
}

/// L1 sampling (ADR 0021 C.1): it walks the folder and computes the cheap signature
/// (`compute_set_signature`, without reading bytes, the same one the backup's skip
/// uses) hashed to a `u64`. Only called when L0 moved or a hint focused the save.
/// `None` when the folder could not be walked (the reducer then falls back to
/// `has_pending`).
fn observe_local_fingerprint(path: &Path, game_slug: &str) -> Option<u64> {
    // The same shields the backup uses, or the two signatures diverge forever and
    // the reducer sees a pending change that never resolves.
    let shields = crate::savefilter::shields_for_slug(game_slug);
    let files = crate::backup::walk_source(path, &shields).ok()?;
    Some(fingerprint_of(&crate::backup::compute_set_signature(
        &files,
    )))
}

/// Builds the slot's durable [`kernel::State`] to hand to the reducer (ADR 0021
/// D.7: the `SaveSlot` to `kernel::State` conversion lives in the shell).
///
/// `is_running` and `last_running_seen` are fed from the already-debounced value
/// `process_poll` maintains (its 6 s sticky, `STRONG_STOP_GRACE_FLOOR_SECS`), so the
/// kernel's stickiness is a passthrough and the grace is not doubled. The reducer
/// owns the rest of the sync fields.
fn state_from_slot(slot: &SaveSlot, config: &AgentConfig, now: OffsetDateTime) -> kernel::State {
    kernel::State {
        track_only: slot.save.track_only,
        restore_enabled: slot
            .save
            .policy
            .auto_restore
            .unwrap_or(config.auto_restore || config.global_sync),
        min_backup_interval_secs: slot
            .save
            .policy
            .min_snapshot_interval_secs
            .unwrap_or(config.min_snapshot_interval_secs),
        is_running: slot.is_running,
        // Passthrough: `process_alive` equals `is_running` in the observation, so
        // the kernel's grace never extends beyond the shell's sticky.
        last_running_seen: if slot.is_running { Some(now) } else { None },
        has_pending: slot.has_pending,
        last_fs_event_at: slot.last_fs_event_at,
        last_restore_at: slot.last_restore_at,
        known_version: slot.known_version,
        synced_fingerprint: slot.synced_fingerprint,
        last_backup_at: slot.last_backup_at,
        burst_since: slot.burst_since,
        burst_backups: slot.burst_backups,
        in_flight: slot.in_flight,
        next_backup_at: slot.next_backup_at,
        next_restore_at: slot.next_restore_at,
        pull_pending: slot.pull_pending,
        deferred_notified: slot.deferred_notified,
        restore_failures: slot.restore_failures,
        backup_conflict: slot.backup_conflict,
    }
}

/// Pours the state the reducer returned back into the slot. It does NOT touch
/// `is_running` or `last_running_seen`: those belong to `process_poll` (detection,
/// the GameStarted and GameStopped events, playtime); the reducer only reads them.
fn apply_state_to_slot(slot: &mut SaveSlot, next: kernel::State) {
    slot.has_pending = next.has_pending;
    slot.last_fs_event_at = next.last_fs_event_at;
    slot.last_restore_at = next.last_restore_at;
    slot.known_version = next.known_version;
    slot.synced_fingerprint = next.synced_fingerprint;
    slot.last_backup_at = next.last_backup_at;
    slot.burst_since = next.burst_since;
    slot.burst_backups = next.burst_backups;
    slot.in_flight = next.in_flight;
    slot.next_backup_at = next.next_backup_at;
    slot.next_restore_at = next.next_restore_at;
    slot.pull_pending = next.pull_pending;
    slot.deferred_notified = next.deferred_notified;
    slot.restore_failures = next.restore_failures;
    slot.backup_conflict = next.backup_conflict;
}

/// The cloud-head cache, with the stamp of when it arrived. The pair travels
/// together on purpose: a head with no date cannot be told from a frozen head, and
/// that was exactly ADR 0021 D.10's fault. The poller died, `versions` stayed pinned
/// at v120, and the reducer decided correctly on a lying input, labelling it
/// `converged`.
///
/// Since D.12 it is filled two ways, and that is the structural fix: the engine
/// queries the manifest itself every
/// [`kernel::reconcile::CLOUD_SELF_OBSERVE_AFTER_SECS`]
/// ([`Self::due_for_self_observation`]) and the client's push (`SetCloudVersions`,
/// from the desktop's poller or the CLI's `cloud_live`) remains a latency hint. A
/// live feed refreshes the stamp before the deadline expires, so it suppresses the
/// engine's own query and the cost stays one manifest per interval; a dead poller
/// now only costs latency, not permanent blindness.
#[derive(Debug, Clone)]
struct CloudHeads {
    /// The latest cloud version per `save_id`, exactly as the manifest brought it.
    versions: HashMap<String, i64>,
    /// `(game_slug, label)` → the `save_id` the cloud knows that row by.
    ///
    /// The bridge between the two keys. The cloud identifies a save by name and
    /// this machine by a uuid it made up; `cas_init` takes the uuid and resolves
    /// it by name, so a device whose local id drifted (a re-detected folder, a
    /// rebuilt `state.json`) keeps uploading fine while being unable to find
    /// itself in anything the server hands back. Without this index
    /// `versions.get(local_id)` is `None` forever: the row goes blind to the
    /// cloud (it sees no new versions and can't clear a conflict) and does it
    /// silently, because "absent from the manifest" and "converged" read the
    /// same. That is what left a save fourteen days out of sync in aug-2026.
    aliases: HashMap<(String, String), String>,
    /// What content that head has, per `save_id`: the digest of the manifest the
    /// server publishes (ADR 0021 D.8.3). Only the engine's own observation fills it,
    /// since the client's push brings versions rather than digests, so it can lag
    /// behind [`Self::versions`]; that is why [`Self::head_for`] demands the version
    /// match before believing it.
    digests: HashMap<String, ServerHead>,
    /// Cuándo aterrizó ese feed, venga de donde venga. `None` = todavía ninguno.
    as_of: Option<OffsetDateTime>,
    /// The engine's last attempt to observe the cloud itself, successful or not. It
    /// paces the retries: without this stamp a downed server, or a 401 session, would
    /// have every tick, every two seconds, firing the query again.
    last_attempt_at: Option<OffsetDateTime>,
    /// Is there a cloud to observe? `Some(true)` cloud, `Some(false)` self-hosted,
    /// `None` unresolved (the `/v1/health` probe has not answered yet). Latched: once
    /// resolved it does not take it back, because the agent's `ApiClient` does not
    /// change server within its life.
    is_cloud: Option<bool>,
    /// When this engine started expecting cloud heads. It is the anchor of the
    /// startup allowance the kernel uses to report "I have never heard anything from
    /// the cloud" (ADR 0021 D.11).
    expecting_since: OffsetDateTime,
}

impl CloudHeads {
    fn new(now: OffsetDateTime) -> Self {
        Self {
            versions: HashMap::new(),
            aliases: HashMap::new(),
            digests: HashMap::new(),
            as_of: None,
            last_attempt_at: None,
            is_cloud: None,
            expecting_since: now,
        }
    }

    /// Installs a new feed and stamps its timestamp. `digests` only comes from the
    /// engine's own observation; a client push passes `None` and leaves whatever was
    /// there (which [`Self::head_for`] will discard if the version no longer
    /// matches).
    fn feed(
        &mut self,
        versions: HashMap<String, i64>,
        aliases: Option<HashMap<(String, String), String>>,
        digests: Option<HashMap<String, ServerHead>>,
        now: OffsetDateTime,
    ) {
        self.versions = versions;
        // Same deal as `digests`: a push that carries no names leaves the ones
        // already here. A stale alias can only point at a row that is no longer
        // in `versions`, and `cloud_id_for` throws it out when it is.
        if let Some(aliases) = aliases {
            self.aliases = aliases;
        }
        if let Some(digests) = digests {
            self.digests = digests;
        }
        self.as_of = Some(now);
    }

    /// The id the cloud knows this save by: ours if it recognises it, otherwise
    /// the row carrying its `(game, label)`, the same two steps the server
    /// itself takes in `resolve_save_row`, in the same order.
    ///
    /// An alias only counts while the row it names is still in this pass's feed;
    /// otherwise it describes a row that has been deleted.
    fn cloud_id_for<'a>(&'a self, save: &'a WatchedSave) -> &'a str {
        if self.versions.contains_key(&save.save_id) {
            return &save.save_id;
        }
        let label = if save.label.is_empty() {
            "default"
        } else {
            save.label.as_str()
        };
        match self
            .aliases
            .get(&(save.game_slug.clone(), label.to_string()))
        {
            Some(id) if self.versions.contains_key(id) => id.as_str(),
            _ => &save.save_id,
        }
    }

    /// A save's head, resolving first which id the cloud knows it by.
    fn version_for(&self, save: &WatchedSave) -> Option<i64> {
        self.versions.get(self.cloud_id_for(save)).copied()
    }

    /// A save's head together with its content, for D.8.3's anti-relaunch check.
    ///
    /// It is only returned when the digest we hold is the one for the version that is
    /// the head right now: a digest paired with an old version would describe content
    /// that is no longer the server's, and believing it would skip an upload that is
    /// genuinely needed. Pairing them rather than keeping two loose maps is what
    /// makes that failure impossible.
    fn head_for(&self, save: &WatchedSave) -> Option<&ServerHead> {
        let id = self.cloud_id_for(save);
        let head = self.digests.get(id)?;
        (self.versions.get(id) == Some(&head.version_num)).then_some(head)
    }

    /// The anchor for the startup allowance that goes into the
    /// [`kernel::Observation`]: there is only something to wait for when we know
    /// there is a cloud. With no resolved context nothing is claimed, since declaring
    /// blindness without knowing whether the cloud exists would be inventing a
    /// fault.
    fn expected_since(&self) -> Option<OffsetDateTime> {
        (self.is_cloud == Some(true)).then_some(self.expecting_since)
    }

    /// Merge one save's head without replacing the rest of the map.
    ///
    /// SSE delivers one `(save_id, version)` per commit. [`Self::feed`]
    /// replaces the whole map, so a one-row push would wipe every other save.
    /// Does **not** bump `as_of`: a live SSE stream must not suppress the
    /// periodic `list_saves` / cloud-sync pass that keeps the rest of the
    /// library honest.
    fn merge_version(&mut self, save_id: String, version: i64) {
        match self.versions.get(&save_id).copied() {
            Some(known) if known >= version => {}
            _ => {
                self.versions.insert(save_id, version);
            }
        }
    }

    /// Is it time for the engine to fetch the head itself? Two independent brakes:
    ///
    /// - Freshness: with a recent feed (the poller's or our own) there is nothing to
    ///   fetch. That is what stops a healthy client duplicating the GET.
    /// - Pacing: at most one attempt per interval whatever happens, so a downed
    ///   backend gets one attempt every minute and a half rather than one per tick.
    ///
    /// Self-hosted has heads too (`GET /v1/saves`). Skipping that left `cloud_ahead`
    /// stuck on `None` so auto-restore never ran unless the folder was empty.
    fn due_for_self_observation(&self, now: OffsetDateTime) -> bool {
        let stale_enough = |t: OffsetDateTime| {
            (now - t).whole_seconds() >= kernel::reconcile::CLOUD_SELF_OBSERVE_AFTER_SECS
        };
        self.as_of.is_none_or(stale_enough) && self.last_attempt_at.is_none_or(stale_enough)
    }
}

/// The engine observes the cloud (ADR 0021 D.12). One single call to the manifest,
/// rather than a GET per save per tick, whose result comes back into the loop as
/// [`AgentCommand::CloudHeadsObserved`] and from there into the
/// [`kernel::Observation`].
///
/// It runs in its own task so it does not block the agent's `select!`: the
/// `/v1/health` probe and the manifest each have a minute of timeout, and the loop
/// has to keep serving fs events, processes and commands meanwhile.
///
/// Why the engine and not the client: the UI knew about v181 because *it* asked the
/// server; the agent did not, because it depended on a foreign and fragile process
/// pushing it. With that process dead, the engine stayed blind forever with no
/// self-recovery. Observing for itself, a dead poller degrades to "I take until the
/// next interval", C.1's level-triggered property applied to the transport as well.
///
/// Best-effort from end to end: any failure is reported as `versions: None` (the
/// freshness stamp is not set and the blindness stays visible) and retried at the
/// next deadline.
/// How often the engine ships this machine's playtime. Playtime is a daily
/// aggregate, not an event stream, so the only thing a short interval buys is
/// a fresher recap for someone who opens Wrapple right now; 30 min keeps a
/// long session from being invisible without costing more than a couple of
/// requests an hour.
const PLAYTIME_SHIP_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Ship the local playtime breakdown to the account's server.
///
/// **Why the engine and not the window.** Until now the only caller of the
/// playtime push was the Wrapple screen itself, so the hours reached the server
/// only when a user opened the recap, which meant the recap showed real hours
/// exclusively to people who had already opened it before. Everyone else saw
/// zero and concluded it was broken. The store has always accrued correctly in
/// `process_poll`; it was the delivery that depended on someone looking. Same
/// lesson as D.12: an engine that needs a client alive to do its job is an
/// engine that goes quiet the moment the window closes.
///
/// Gated on `prefs.wrapple_telemetry`, read fresh so flipping the switch takes
/// effect within one interval. Off means nothing is sent: the daemon does not
/// report the fact that it is off, which would be exactly the telemetry the
/// user just declined.
///
/// Best-effort: any failure is a debug line and a retry next interval.
async fn ship_playtime(api: ApiClient, path: Option<PathBuf>) {
    // Unreadable prefs mean the default (on), same as everywhere else the
    // engine reads them: a missing file must not silently revoke consent the
    // user never withdrew.
    let allowed = crate::prefs::Prefs::load_default()
        .map(|(p, _)| p.wrapple_telemetry)
        .unwrap_or(true);
    if !allowed {
        return;
    }
    let Some(path) = path else { return };
    let store = crate::playtime::PlaytimeStore::load(&path);
    let rows = store.upload_rows();
    if rows.is_empty() {
        return;
    }
    let dev = crate::logship::device_identity();
    let body = crate::cloud_account::PlaytimeUploadBody {
        device_fp: dev.fingerprint,
        authoritative: store.is_authoritative(),
        rows,
    };
    match api.push_playtime(&body).await {
        Ok(()) => tracing::debug!("agent: playtime shipped"),
        Err(e) => tracing::debug!(error = %e, "agent: playtime ship failed; retrying next round"),
    }
}

fn heads_from_selfhosted_saves(
    saves: Vec<crate::api::Save>,
) -> (HashMap<String, i64>, HashMap<(String, String), String>) {
    let mut versions = HashMap::with_capacity(saves.len());
    let mut aliases = HashMap::with_capacity(saves.len());
    for e in saves {
        let Some(v) = e.latest_version_num else {
            continue;
        };
        let id = e.id.to_string();
        versions.insert(id.clone(), v);
        aliases.insert(
            (
                e.game_slug.to_string(),
                if e.label.is_empty() {
                    "default".to_string()
                } else {
                    e.label
                },
            ),
            id,
        );
    }
    (versions, aliases)
}

async fn observe_cloud_heads(api: ApiClient, cmd_tx: mpsc::Sender<AgentCommand>) {
    // The probe is cached after the first success; a failure is NOT cached, so the
    // next attempt asks again.
    let cloud = api.is_cloud().await;
    // Read AFTER the probe so "self-hosted" can be told from "it could not be
    // resolved": `is_cloud()` collapses both into `false`.
    let is_cloud = api.probed_is_cloud();
    let observed = if cloud {
        match api.cloud_sync().await {
            Ok(manifest) => {
                // Two maps from the same pass: the version (the head) and what
                // content it has (its manifest's digest, D.8.3). They come out
                // together on purpose, since a digest from a different pass would
                // describe content that is no longer that version's.
                let mut versions: HashMap<String, i64> = HashMap::new();
                let mut aliases: HashMap<(String, String), String> = HashMap::new();
                let mut digests: HashMap<String, ServerHead> = HashMap::new();
                for e in manifest.saves {
                    versions.insert(e.save_id.clone(), e.latest_version_num);
                    // The name→id index. Built from the same pass so it can
                    // never describe a row this feed doesn't carry. The cloud
                    // holds one row per (user, game, label), so the key is
                    // unique by construction; an empty label is the server's
                    // "default" (see `resolve_save_row`).
                    aliases.insert(
                        (
                            e.game_slug.clone(),
                            if e.label.is_empty() {
                                "default".to_string()
                            } else {
                                e.label.clone()
                            },
                        ),
                        e.save_id.clone(),
                    );
                    digests.insert(
                        e.save_id,
                        ServerHead {
                            version_num: e.latest_version_num,
                            digest: e.latest_sha256,
                        },
                    );
                }
                tracing::debug!(count = versions.len(), "agent: observed cloud heads");
                Some((versions, aliases, digests))
            }
            Err(e) => {
                // A warn rather than a debug: since D.12 this is the main route for
                // observing the cloud, and it going quiet is exactly the fault that
                // cost two dogfooding sessions to find.
                tracing::warn!(error = %e, "agent: couldn't observe the cloud head");
                None
            }
        }
    } else if is_cloud == Some(false) {
        match api.list_saves(None).await {
            Ok(saves) => {
                let (versions, aliases) = heads_from_selfhosted_saves(saves);
                tracing::debug!(count = versions.len(), "agent: observed self-hosted heads");
                Some((versions, aliases, HashMap::new()))
            }
            Err(e) => {
                tracing::warn!(error = %e, "agent: couldn't observe self-hosted heads");
                None
            }
        }
    } else {
        tracing::debug!("agent: cloud head observation skipped, server mode unresolved");
        None
    };
    let (versions, aliases, digests) = match observed {
        Some((versions, aliases, digests)) => (Some(versions), Some(aliases), Some(digests)),
        None => (None, None, None),
    };
    let _ = cmd_tx
        .send(AgentCommand::CloudHeadsObserved {
            versions,
            aliases,
            digests,
            is_cloud,
        })
        .await;
}

/// Samples the world for one slot and builds the tick's [`kernel::Observation`]
/// (ADR 0021 C.1). L0 (the folder's own mtime plus emptiness) is cheap every tick;
/// L1 (the fingerprint) is only computed when L0 moved, the watcher set `needs_l1`,
/// or a sweep or manual copy forced it, never re-hashing everything every tick.
fn observe_slot(slot: &mut SaveSlot, cloud: &CloudHeads) -> kernel::Observation {
    let folder_mtime = folder_own_mtime(&slot.save.local_path);
    let local_empty = is_path_empty_or_missing(&slot.save.local_path);
    let l0_changed = folder_mtime != slot.last_l0_mtime;
    slot.last_l0_mtime = folder_mtime;
    let compute_l1 = !slot.save.track_only && !local_empty && (l0_changed || slot.needs_l1);
    slot.needs_l1 = false;
    let local_fingerprint = if compute_l1 {
        observe_local_fingerprint(&slot.save.local_path, &slot.save.game_slug)
    } else {
        None
    };
    kernel::Observation {
        folder_mtime,
        folder_size: None,
        local_empty,
        local_fingerprint,
        // The process state belongs to `process_poll` (already with its 6 s sticky);
        // here it is a passthrough for the kernel's stickiness.
        process_alive: slot.is_running,
        // The lock probe: "the game is writing the save NOW", said by the filesystem
        // rather than by matching a process. It is only paid for when it can really
        // change a decision: if the slot is already running, the veto fires earlier
        // and probing would be an `open()` per file every two seconds for nothing.
        // On POSIX it is always `false` (there is no mandatory locking); see
        // `crate::locks`.
        save_files_locked: !slot.is_running
            && !slot.save.track_only
            && crate::locks::any_file_locked(&slot.save.local_path),
        cloud_version: cloud.version_for(&slot.save),
        // The stamp belongs to the *feed*, not to the save: the manifest arrives
        // whole, so a save missing from it has `cloud_version: None` with an equally
        // fresh stamp. That is what lets the kernel tell "converged" from "blind"
        // (ADR 0021 D.10).
        cloud_version_as_of: cloud.as_of,
        // And this is what lets it tell "there is no cloud to look at" from "there
        // is a cloud and I have known nothing about it since boot" (D.11).
        cloud_feed_expected_since: cloud.expected_since(),
        // The fs watcher marks `has_pending` and `last_fs_event_at` on the slot
        // directly (see the `fs_rx` branch), so there is no need to re-mark through
        // `fs_event` here; the reducer reads them from the state.
        fs_event: false,
        op_result: slot.pending_op_result.take(),
        // The content-addressed anti-relaunch check (ADR 0021 D.8.3): it is answered
        // by the upload's executor, since it is IO and the kernel does none, by
        // comparing the local content against the server's head digest. `Some(true)`
        // means the upload a restart left half done did land, so there is nothing to
        // re-upload and the reducer only has to record the version. `None` means it
        // was not checked (nobody uploaded anything this tick).
        upload_landed: slot.pending_upload_landed.take(),
    }
}

/// The reconciliation step (ADR 0021 C.1, Slice 2b): the inverted authority. For
/// each slot: sample the world, build the [`kernel::Observation`], call the pure
/// reducer [`kernel::reconcile`], pour the state back, run the
/// [`kernel::Decision`]s (`Act` goes to IO, `Hold` gets its reason logged). Zero
/// policy here: every sync decision (backup, restore, defer, veto, cooldown,
/// backoff, throttle, min-interval) is taken by the reducer. The tick is the source
/// of truth; fs, realtime and op events only left hints on the slot that bring this
/// step forward.
#[allow(clippy::too_many_arguments)]
fn reconcile_all(
    slots: &mut HashMap<String, SaveSlot>,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    cloud: &CloudHeads,
) {
    let now = OffsetDateTime::now_utc();
    let ids: Vec<String> = slots.keys().cloned().collect();
    for id in ids {
        // The slot may have vanished between iterations (not here, but for hygiene).
        let Some(slot) = slots.get_mut(&id) else {
            continue;
        };
        // A pre-reducer snapshot to derive the observability events (stuck and
        // recovered) from `restore_failures`' delta.
        let was_stuck = slot.restore_failures.stuck_notified;
        let err_for_stuck = slot.last_restore_error.take();
        let was_blocked = slot.backup_conflict.needs_attention;
        let err_for_conflict = slot.last_conflict_error.take();

        let world = kernel::World {
            now,
            seed: seed_for(&id),
        };
        let obs = observe_slot(slot, cloud);
        let state = state_from_slot(slot, config, now);
        let (next, decisions) = kernel::reconcile::reconcile(&state, &obs, world);
        // Read before the state is moved into the slot: it is when the copy can
        // next go out, and the shell owes the user that number (see the
        // `Hold` arm below).
        let floor = kernel::reconcile::backup_floor(&next);
        // Read while `slot` is still borrowed: `execute_action` takes the whole
        // map, so the head has to be resolved and cloned before that.
        let server_head = cloud.head_for(&slot.save).cloned();
        apply_state_to_slot(slot, next);

        // The stuck and recovered events come purely from the failure escalation's
        // delta. The reducer decides (it escalates on ingesting a `Failed`, resets on
        // an `Ok` or on a new cloud version); the shell only translates the edge into
        // UI events (ADR 0021 C.5: the veto or failure is first-class and visible).
        // With no gate on the result type, so the reset on a new version, which no
        // longer arrives as an op, also announces the recovery.
        let now_stuck = slot.restore_failures.stuck_notified;
        if !was_stuck && now_stuck {
            let _ = events_tx.try_send(AgentEvent::SaveAutoRestoreStuck {
                save_id: id.clone(),
                game_slug: slot.save.game_slug.clone(),
                failures: slot.restore_failures.consecutive,
                error: err_for_stuck.unwrap_or_default(),
            });
        }
        if was_stuck && !now_stuck {
            tracing::info!(
                save_id = %id,
                "agent: auto-restore escalation cleared, save recovered"
            );
            let _ = events_tx.try_send(AgentEvent::SaveAutoRestoreRecovered {
                save_id: id.clone(),
                game_slug: slot.save.game_slug.clone(),
            });
        }

        // And the same for an upload stuck on a conflict with no way out: the reducer
        // decides when the budget runs out and when it is released (a successful copy,
        // a new cloud head, or the user asking by hand); the shell only translates the
        // edge into events. The warning coming off the SAME flag that brakes the
        // upload is what stops the UI calling a save stuck while it uploads fine, or
        // staying quiet about one that does not.
        let now_blocked = slot.backup_conflict.needs_attention;
        if !was_blocked && now_blocked {
            tracing::warn!(
                save_id = %id,
                game_slug = %slot.save.game_slug,
                conflicts = slot.backup_conflict.consecutive,
                "agent: giving up on this upload, the conflict needs the user"
            );
            let _ = events_tx.try_send(AgentEvent::BackupNeedsAttention {
                save_id: id.clone(),
                game_slug: slot.save.game_slug.clone(),
                label: slot.save.label.clone(),
                conflicts: slot.backup_conflict.consecutive,
                error: err_for_conflict.unwrap_or_default(),
            });
        }
        if was_blocked && !now_blocked {
            tracing::info!(
                save_id = %id,
                "agent: upload conflict cleared, this save can sync again"
            );
            let _ = events_tx.try_send(AgentEvent::BackupAttentionCleared {
                save_id: id.clone(),
                game_slug: slot.save.game_slug.clone(),
            });
        }

        for decision in decisions {
            match decision {
                kernel::Decision::Act(action) => execute_action(
                    slots,
                    &id,
                    action,
                    api,
                    events_tx,
                    cmd_tx,
                    config,
                    done_tx,
                    server_head.clone(),
                ),
                kernel::Decision::Hold { reason } => {
                    tracing::debug!(save_id = %id, reason, "agent: reconcile hold");
                    if kernel::reconcile::hold_is_paced_backup(reason) {
                        announce_backup_wait(slots, &id, floor, now, events_tx);
                    }
                }
            }
        }
    }
}

/// Show a deferred backup instead of just logging it.
///
/// The reducer can hold an upload for a full minute (the adaptive floor under
/// a game that rewrites its autosave in a loop) and until now that hold was a
/// `debug!` line and nothing else. The first attempt at a floor was a fixed one
/// for everybody and had to be reverted for exactly this: it was invisible, and
/// what reached support was "Hoard isn't picking up my changes". A conditional
/// floor that nobody can see fails the same way, only to fewer people.
///
/// `next_scheduled_backup_at` is where the answer belongs: the overlay's "next
/// copy in Xs" and the Settings diagnostics both read it already, and the
/// debounce timer writes the same field. It is cleared when the upload actually
/// starts, so a stale deadline can't outlive the wait.
///
/// The `BackupScheduled` event that goes with it is announced on the rising
/// edge only. The reducer holds on **every** tick while the floor stands (every
/// two seconds), and re-announcing each time is what used to flood the feed
/// with orphan "queued" rows for a game that autosaves every second.
fn announce_backup_wait(
    slots: &mut HashMap<String, SaveSlot>,
    id: &str,
    floor: Option<OffsetDateTime>,
    now: OffsetDateTime,
    events_tx: &mpsc::Sender<AgentEvent>,
) {
    let Some(floor) = floor else { return };
    let Some(slot) = slots.get_mut(id) else {
        return;
    };
    // Only a save with something to upload is waiting for anything. Holding a
    // save with nothing pending is the ordinary quiet state, not a queue.
    if !slot.has_pending {
        return;
    }
    let remaining = floor - now;
    if !remaining.is_positive() {
        return;
    }
    let already_announced = slot.next_scheduled_backup_at.is_some();
    slot.next_scheduled_backup_at = Some(floor);
    if already_announced {
        return;
    }
    let _ = events_tx.try_send(AgentEvent::BackupScheduled {
        save_id: id.to_string(),
        delay_ms: (remaining.whole_milliseconds().max(0)) as u64,
        // The same reason the debounce announces with, so the desktop's
        // "queued, waiting" surface needs no new variant on the wire (ADR 0021
        // C.6): what tells the two apart there is the delay being longer than a
        // debounce, which is precisely what this is.
        reason: BackupReason::FilesystemSettled,
    });
}

/// Runs a [`kernel::Action`] the reducer asked for: the decision-to-IO translation.
/// `Pull` and `Restore` share one executor (`spawn_auto_restore` into
/// `run_auto_restore`) so they do not become two diverging paths for retry,
/// throttle and integrity, which is exactly what the 429 was (ADR 0021 D.7).
#[allow(clippy::too_many_arguments)]
fn execute_action(
    slots: &mut HashMap<String, SaveSlot>,
    id: &str,
    action: kernel::Action,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    head: Option<ServerHead>,
) {
    match action {
        kernel::Action::Backup => {
            execute_backup(slots, id, api, events_tx, cmd_tx, config, done_tx, head);
        }
        // Pull and Restore: different kernel intents, one executor.
        kernel::Action::Restore | kernel::Action::Pull => {
            execute_restore(slots, id, api, events_tx, cmd_tx, config);
        }
        // A UI notice, nothing more. The flush that used to live here (uploading what
        // was pending so the cloud stopped being ahead) was policy in the shell: the
        // reducer held the pull and returned before the backup branch, so the shell
        // unstuck the (has_pending, cloud_ahead) pair by hand. Now the reducer itself
        // emits `Backup` on the same tick it defers the pull (ADR 0021 D.8.1), so all
        // that is left here is the notification.
        kernel::Action::DeferPull => {
            let Some(game_slug) = slots.get(id).map(|s| s.save.game_slug.clone()) else {
                return;
            };
            tracing::info!(
                save_id = %id,
                "agent: cross-device update deferred mid-session; pulls when the folder settles"
            );
            let _ = events_tx.try_send(AgentEvent::RestoreDeferred {
                save_id: id.to_string(),
                game_slug,
                reason: "mid-session".to_string(),
            });
        }
        // The deadline already lives in `next_backup_at` and `next_restore_at`; the
        // shell does not retry until it is crossed. This only records it.
        kernel::Action::Throttle { until } => {
            tracing::info!(save_id = %id, ?until, "agent: throttled, backing off until deadline");
        }
    }
}

/// Launches the backup (local to cloud upload) the reducer asked for. `in_flight`
/// was already marked `Some(Backup)` by the reducer, so anti-relaunch protects it
/// from firing again. On finishing, `run_backup_with_retry` reports through
/// `done_tx` or `RetryBackupAfterFailure` and the shell turns it into an `OpResult`
/// for the next tick.
#[allow(clippy::too_many_arguments)]
fn execute_backup(
    slots: &mut HashMap<String, SaveSlot>,
    id: &str,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
    done_tx: &mpsc::Sender<BackupDone>,
    // What content the server publishes as this save's head, when we know it: it is
    // what lets us spot that the upload a daemon restart left half done had already
    // landed (ADR 0021 D.8.3).
    head: Option<ServerHead>,
) {
    let Some(slot) = slots.get_mut(id) else {
        return;
    };
    // `in_flight` was already set to `Some(Backup)` by the reducer (the only route
    // that emits `Act(Backup)`), so anti-relaunch covers this upload without the
    // shell touching sync state.
    //
    // The previous upload, if there was one, has already finished: the reducer does
    // not re-emit Backup with `in_flight` set. Cancel any pending fs debounce timer,
    // since its job (the nudge) no longer applies and the upload starts now.
    if let Some(p) = slot.pending.take() {
        p.abort();
    }
    slot.next_scheduled_backup_at = None;
    let save = slot.save.clone();
    let prev_set_hash = slot.last_set_hash.clone();
    let base_version = slot.known_version;
    let max_retries = config.max_retries;
    let auto_restore = slot
        .save
        .policy
        .auto_restore
        .unwrap_or(config.auto_restore || config.global_sync);
    let conflict_root = config.conflict_root.clone();
    let conflict_retention_days = config.conflict_retention_days;
    let origin = if std::mem::take(&mut slot.manual_requested) {
        VersionOrigin::Manual
    } else {
        VersionOrigin::Automatic
    };
    let api = api.clone();
    let events_tx = events_tx.clone();
    let done_tx = done_tx.clone();
    let cmd_tx = cmd_tx.clone();
    tracing::info!(save_id = %id, "agent: reconcile → backup");
    tokio::spawn(async move {
        run_backup_with_retry(
            api,
            save,
            prev_set_hash,
            base_version,
            head,
            origin,
            events_tx,
            done_tx,
            cmd_tx,
            max_retries,
            auto_restore,
            conflict_root,
            conflict_retention_days,
        )
        .await;
    });
}

/// Launches the restore (cloud to local, conflict-aware) the reducer asked for.
/// `in_flight` was already set to `Some(Restore)` and `next_restore_at` armed the
/// cooldown, both by the reducer; the result comes back as `AutoRestoreFinished`
/// and then an `OpResult`.
fn execute_restore(
    slots: &HashMap<String, SaveSlot>,
    id: &str,
    api: &ApiClient,
    events_tx: &mpsc::Sender<AgentEvent>,
    cmd_tx: &mpsc::Sender<AgentCommand>,
    config: &AgentConfig,
) {
    let (save, known_version) = match slots.get(id) {
        Some(s) => (s.save.clone(), s.known_version),
        None => return,
    };
    tracing::info!(save_id = %id, "agent: reconcile → restore");
    spawn_auto_restore(
        save,
        api.clone(),
        events_tx.clone(),
        cmd_tx.clone(),
        config.conflict_root.clone(),
        config.conflict_retention_days,
        known_version,
        // Authoritative: the reducer already decided there is something to pull
        // (empty, or the cloud ahead), so look for the real head rather than trusting
        // a cache that may be a tick stale. The internal version gate makes it free
        // when we are already up to date.
        None,
        None,
    );
}

async fn run_agent(
    api: ApiClient,
    mut config: AgentConfig,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    events_tx: mpsc::Sender<AgentEvent>,
) {
    let mut slots: HashMap<String, SaveSlot> = HashMap::new();

    // Latest cloud version per save id. Since ADR 0021 D.12 the engine keeps
    // this fresh **itself** (`observe_cloud_heads`: cloud `/v1/cloud/sync` or
    // self-hosted `/v1/saves`); the client-side pollers' `SetCloudVersions`
    // / SSE `ForceRestore` push is a latency hint on top. Lets the
    // reconciliation sweep version-gate locally instead of having each
    // `run_auto_restore` re-fetch the manifest.
    let mut cloud_heads = CloudHeads::new(OffsetDateTime::now_utc());
    // The cloud observation in flight, if any. A `JoinHandle` rather than a boolean
    // on purpose: `is_finished()` is also true when the task dies on a panic or gets
    // cancelled, so the slot cannot stay "occupied" forever, which is exactly how the
    // desktop's poller went quiet (D.11) and how its whole task disappeared (D.12).
    let mut cloud_probe: Option<JoinHandle<()>> = None;

    // The playtime upload in flight, with its deadline. The same `JoinHandle` rather
    // than a boolean as `cloud_probe`, and for the same reason: a task that dies on a
    // panic must not leave the slot occupied forever. The first upload goes out on the
    // first tick (we do not wait half an hour for a freshly started machine to report
    // what it already had on disk).
    let mut playtime_ship: Option<JoinHandle<()>> = None;
    let mut playtime_ship_due = tokio::time::Instant::now();

    // Channel used by every fs watcher: debounced events all funnel here
    // and we route them by path. mpsc::unbounded would be fine since the
    // debouncer already throttles, but we cap at 256 to be defensive.
    let (fs_tx, mut fs_rx) = mpsc::channel::<PathBuf>(256);

    // Backup tasks signal completion (committed / no-op / conflict-settled) so
    // the agent loop can feed it into the reductor as an `OpResult`. Cap matches
    // `cmd_rx`.
    let (done_tx, mut done_rx) = mpsc::channel::<BackupDone>(64);

    // The out-of-band reconciliation nudge (ADR 0021 C.1: events are hints that only
    // bring a tick forward). The fs debounce timer fires it when a write settles; the
    // loop then runs `reconcile_all` without waiting for the next `poll.tick()`. The
    // nudges coalesce (the queue is drained before reconciling) so a burst of
    // autosaves does not fire a burst of reconciliations.
    let (nudge_tx, mut nudge_rx) = mpsc::channel::<()>(64);

    // Process watcher: periodic poll. We refresh only the bits we care
    // about (process names + exe paths) to keep CPU near zero when idle.
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_processes(proc_refresh_kind()));
    let active_poll = Duration::from_secs(config.poll_secs.max(1));
    let idle_poll = active_poll.saturating_mul(IDLE_POLL_MULT);
    let mut poll = tokio::time::interval(active_poll);
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Start fast so a game already open at launch is caught on the first tick;
    // `process_poll`'s return value drives the fast↔idle cadence thereafter.
    let mut polling_fast = true;

    // Detection (phase 3, ADR 0020): the process-to-write correlation store. When a
    // watched save is rewritten, we record which game process was alive. Today it
    // feeds attribution and learning over already-tracked saves; the observer over
    // `roots.rs`' broad roots, for DISCOVERING new folders, is the next and heavier
    // step and stays out of this wiring.
    let corr_path = crate::correlation::CorrelationStore::default_path().ok();
    let mut corr_store = corr_path
        .as_deref()
        .map(crate::correlation::CorrelationStore::load)
        .unwrap_or_default();

    // Playtime: real hours per local day. It is fed on every poll tick with the saves
    // whose game process is still alive (see `process_poll`). It adopts the legacy
    // global file into the active context once before loading, so the main account
    // keeps its history and the rest start empty (the store resolves per sync
    // context).
    if let Err(e) = crate::playtime::PlaytimeStore::migrate_legacy_into_current_context() {
        tracing::debug!(error = %e, "agent: legacy playtime migration skipped");
    }
    let playtime_path = crate::playtime::PlaytimeStore::default_path().ok();
    let mut playtime = playtime_path
        .as_deref()
        .map(crate::playtime::PlaytimeStore::load)
        .unwrap_or_default();

    // Detection (phase 3, ADR 0020): the untracked-candidate probe. It maps each
    // candidate folder to the last maximum mtime observed. When one goes up (a new
    // write) and a game is alive, it records the correlation. The `None` baseline is
    // seeded on the first tick without recording anything, so a pre-existing recent
    // file is not confused with a freshly observed write.
    let mut probes: HashMap<PathBuf, Option<std::time::SystemTime>> = HashMap::new();

    // PIDs we've already flagged as heavy untracked games this session (see
    // `AgentEvent::HeavyProcessDetected`). Keeps the immediate-scan trigger to
    // one event per process; `process_poll` prunes exited PIDs each tick so a
    // relaunch re-triggers.
    let mut reported_heavy: HashSet<Pid> = HashSet::new();
    // The cross-tick state of correlation detection (the WEAK signal), which is now
    // a PID transition rather than presence plus CPU: `prev_pids` is the previous
    // tick's snapshot of live PIDs (to know which were BORN this tick) and
    // `corr_running` maps `save_id` to the `(pid, start_time)` of the process keeping
    // that slot "running" today. A resident (a chat client since boot) is never new,
    // so it never fires "it started"; the slot stops when its PID dies.
    let mut prev_pids: HashSet<Pid> = HashSet::new();
    let mut corr_running: HashMap<String, (Pid, u64)> = HashMap::new();

    // Playtime "only what you play": an index from Steam folder to slug for the
    // installed library. The poll attributes hours to any Steam game that runs,
    // tracked or not. It is rebuilt on a TTL (see `playtime_index`); empty until the
    // first `refresh_if_stale`.
    let mut steam_index = crate::playtime_index::SteamPlaytimeIndex::new();

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
                        // handle_add registers the slot, arms the watcher and, when
                        // the folder already holds content diverging from what is
                        // synced, seeds `has_pending` for the initial baseline. The
                        // decision (restore into empty, upload the baseline, veto for
                        // the session) is taken by the reducer in the reconcile
                        // below; the recency veto covers the "folder touched
                        // recently" case that used to be deferred by hand.
                        handle_add(&mut slots, *save, &fs_tx);
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::RearmWatcher(id)) => {
                        // Auto-restore created files where there were none, so
                        // the watcher we built (or skipped) on AddSave needs
                        // to be rebuilt against the now-existing directory.
                        if let Some(slot) = slots.get_mut(&id) {
                            arm_watcher(slot, &fs_tx);
                        }
                    }
                    Some(AgentCommand::AutoRestoreFinished { id, disposition, synced_version, post_restore_set_hash, wrote_files }) => {
                        // The restore op finished: in the inverted model its result
                        // is an *input* to the reducer (ADR 0021 C.1). We translate
                        // the disposition into the kernel's `OpResult` and queue it;
                        // the next `reconcile_all` clears `in_flight`, applies the
                        // bookkeeping and backoff (cooldown, 404, 401, 429, the
                        // per-version failure escalation) and, through
                        // `restore_failures`' delta, emits the stuck and recovered
                        // events. `wrote_files` travels in `OpResult::Ok.wrote` to
                        // stamp `last_restore_at` (so the restore's own touch does
                        // not veto the next pull); the synced fingerprint comes from
                        // the post-merge signature only when the tree ended up equal
                        // to the head, with no local divergence.
                        let fingerprint =
                            post_restore_set_hash.as_deref().map(fingerprint_from_set_hash);
                        let op_result = match disposition {
                            AutoRestoreDisposition::Ok => kernel::OpResult::Ok {
                                version: synced_version,
                                fingerprint,
                                wrote: wrote_files,
                            },
                            AutoRestoreDisposition::NotOnServer => kernel::OpResult::NotFound,
                            AutoRestoreDisposition::Unauthorized => kernel::OpResult::Unauthorized,
                            AutoRestoreDisposition::Throttled { retry_after_secs } => {
                                kernel::OpResult::Throttled { retry_after_secs }
                            }
                            AutoRestoreDisposition::Failed(err) => {
                                if let Some(slot) = slots.get_mut(&id) {
                                    slot.last_restore_error = Some(err);
                                }
                                kernel::OpResult::Failed
                            }
                        };
                        if let Some(slot) = slots.get_mut(&id) {
                            // Adopting the post-merge signature also refreshes the
                            // backup's skip: the merge's own writes do not bounce
                            // back as a redundant re-upload of the head.
                            if let Some(h) = post_restore_set_hash {
                                slot.last_set_hash = Some(h);
                            }
                            slot.pending_op_result = Some(op_result);
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::SetAutoRestore(enabled)) => {
                        let was = config.auto_restore;
                        config.auto_restore = enabled;
                        tracing::info!(
                            auto_restore = enabled,
                            "agent: auto_restore preference updated"
                        );
                        // Off to on means "bring me up to date now". Reconcile without
                        // waiting for the next tick: the reducer restores any empty or
                        // outdated folder, with the version gate and the session
                        // vetoes intact.
                        if !was && enabled {
                            reconcile_all(
                                &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                                &cloud_heads,
                            );
                        }
                    }
                    Some(AgentCommand::SetGlobalSync(enabled)) => {
                        let was = config.global_sync;
                        config.global_sync = enabled;
                        tracing::info!(
                            global_sync = enabled,
                            "agent: global_sync preference updated"
                        );
                        if !was && enabled {
                            reconcile_all(
                                &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                                &cloud_heads,
                            );
                        }
                    }
                    Some(AgentCommand::ForceRestore {
                        save_id,
                        version_num,
                    }) => {
                        // Cloud poller sends `SetCloudVersions` first, then this
                        // as a tick nudge. Self-hosted SSE has no manifest push:
                        // it carries `version_num` on the event so we merge that
                        // one head, otherwise `cloud_ahead` stays false.
                        if let Some(v) = version_num {
                            cloud_heads.merge_version(save_id, v);
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::SetCloudVersions { versions: map, aliases }) => {
                        tracing::debug!(
                            count = map.len(),
                            "agent: cloud version cache updated from poller"
                        );
                        // Only the head cache is refreshed: releasing a parked save's
                        // backoff when the cloud publishes a version different from
                        // the one that was failing is policy, and it has lived in the
                        // reducer since Slice 2c (ADR 0021 D.8.2). The
                        // `reconcile_all` below applies it on seeing the new
                        // `cloud_version`, and the "recovered" event comes out of
                        // `restore_failures`' delta like any other.
                        //
                        // The feed's timestamp is stamped here: it is what lets the
                        // reducer say "blind" instead of "converged" if the poller
                        // goes quiet again (ADR 0021 D.10).
                        cloud_heads.feed(map, Some(aliases), None, OffsetDateTime::now_utc());
                        // A push can only come from a cloud client (the desktop's
                        // poller or the CLI's `cloud_live`, both behind a cloud
                        // session), so it counts as context evidence when the probe
                        // has not managed to run yet, which is the boot-with-no-
                        // network case where otherwise we would never know there is a
                        // cloud to miss. But it does NOT overwrite an already-resolved
                        // probe: with the agent pointing at a self-hosted server and a
                        // live cloud session on disk, the poller feeds heads that are
                        // not this engine's, and believing it would make us report
                        // blindness about a cloud this agent does not watch.
                        cloud_heads.is_cloud.get_or_insert(true);
                        // The cloud may have moved ahead: reconcile to push the
                        // updates that have just been unblocked.
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::CloudHeadsObserved { versions, aliases, digests, is_cloud }) => {
                        // The engine looked at the cloud itself (ADR 0021 D.12). The
                        // context latch only moves on a definite answer: a failed
                        // probe cannot demote a cloud deployment to "there is no cloud
                        // to look at here".
                        if let Some(c) = is_cloud {
                            cloud_heads.is_cloud = Some(c);
                        }
                        // With no heads (`None`) the freshness stamp is NOT touched:
                        // recording that the attempt happened (`last_attempt_at`,
                        // stamped when it was launched) only paces the retries;
                        // staleness keeps counting from the last real data, which is
                        // what makes the blindness observable.
                        if let Some(map) = versions {
                            tracing::debug!(
                                count = map.len(),
                                "agent: cloud version cache updated from the engine's own observation"
                            );
                            cloud_heads.feed(map, aliases, digests, OffsetDateTime::now_utc());
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::SetProbeCandidates(dirs)) => {
                        // It replaces the set while keeping the baselines of the ones
                        // that remain; new ones start at `None` (seeded on the next
                        // tick). The ones that are no longer candidates are dropped
                        // (they got tracked, or stopped being detected).
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
                        // A manual backup: mark pending when the content diverges from
                        // what is synced (as with the backup's skip-by-set-hash,
                        // identical content produces no snapshot) and let the reducer
                        // decide. `needs_l1` forces a fresh fingerprint.
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.needs_l1 = true;
                            slot.manual_requested = true;
                            // Pressing "back up now" IS the intervention the save was
                            // asking for: it releases the conflict escalation and its
                            // brake, or the button would do nothing and the user would
                            // have no way to answer the warning.
                            if slot.backup_conflict != kernel::ConflictStall::default() {
                                tracing::info!(
                                    save_id = %id,
                                    "agent: manual backup, clearing the conflict escalation"
                                );
                                slot.backup_conflict = kernel::ConflictStall::default();
                                slot.next_backup_at = None;
                            }
                            mark_pending_if_diverged(slot);
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::RetryBackupAfterFailure(id)) => {
                        // The upload burned its internal retry budget. Like any op
                        // ending, it is an *input* to the reducer:
                        // `OpResult::Failed` over a backup `in_flight` clears the op,
                        // arms `next_backup_at` on the long backoff and KEEPS
                        // `has_pending` (the changes never reached a version, and
                        // losing them would let a restore walk over them). That pacing
                        // was shell policy until Slice 2c; now it lives in the kernel
                        // (ADR 0021 D.8.2).
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.next_scheduled_backup_at = None;
                            slot.pending_op_result = Some(kernel::OpResult::Failed);
                            tracing::info!(
                                save_id = %id,
                                backoff_secs = kernel::reconcile::BACKUP_FAILURE_BACKOFF_SECS,
                                "agent: backup retries exhausted, re-arming on the long backoff"
                            );
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::ParkBackupConflict { id, error }) => {
                        // A conflict reconciliation cannot resolve. Like the ones
                        // above it is an input to the reducer, with its own
                        // disposition: it escalates the slot's counter and, once the
                        // budget runs out, stops retrying and marks the save as
                        // needing the user. The UI warning comes off `needs_attention`'s
                        // edge in `reconcile_all`, not from here.
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.next_scheduled_backup_at = None;
                            slot.pending_op_result = Some(kernel::OpResult::ConflictStalled);
                            slot.last_conflict_error = Some(error);
                            tracing::info!(
                                save_id = %id,
                                conflicts = slot.backup_conflict.consecutive + 1,
                                give_up_after = kernel::reconcile::CONFLICT_STALL_GIVE_UP_AFTER,
                                "agent: upload conflict has no resolution, escalating the backoff"
                            );
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::ParkBackupQuotaFull(id)) => {
                        // A full account (402). Like the case above it is an input to
                        // the reducer, but with its own disposition: it parks the
                        // upload for an hour, keeps `has_pending` and does not count
                        // as the save failing, because it is not.
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.next_scheduled_backup_at = None;
                            slot.pending_op_result = Some(kernel::OpResult::QuotaFull);
                            tracing::info!(
                                save_id = %id,
                                backoff_secs = kernel::reconcile::QUOTA_FULL_BACKOFF_SECS,
                                "agent: cloud storage full, parking the upload until space is freed"
                            );
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::ParkBackupThrottled { id, retry_after_secs }) => {
                        // Budget 429. Like the two branches above this is a
                        // reducer input, but the deadline comes off the wire:
                        // `OpResult::Throttled` carries the server's own
                        // `retry_after` into `reconcile::throttle_until`.
                        if let Some(slot) = slots.get_mut(&id) {
                            slot.next_scheduled_backup_at = None;
                            slot.pending_op_result =
                                Some(kernel::OpResult::Throttled { retry_after_secs });
                            tracing::info!(
                                save_id = %id,
                                retry_after_secs,
                                "agent: server asked for a wait, parking the upload until it's up"
                            );
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
                        );
                    }
                    Some(AgentCommand::SweepAll { window_secs }) => {
                        // `window_secs` was the width of the old `sweep_all`'s
                        // size-based staggering; today it is informational (the
                        // staggering was simplified, see below). It is recorded and
                        // not used for pacing.
                        tracing::debug!(window_secs, "agent: hourly sweep, re-checking fingerprints");
                        // The hourly sweep (Automatic Mode): it re-hashes every save to
                        // catch changes the fs watcher missed. In the inverted model
                        // that means recomputing the L1 fingerprint and, when it
                        // diverges from what is synced, marking `has_pending` so the
                        // reducer uploads. The old `sweep_all`'s size-based staggering
                        // (to smooth IO) is simplified: today we walk every folder at
                        // once. `has_pending` is only set on a REAL divergence, so the
                        // veto stays honest.
                        for slot in slots.values_mut() {
                            if slot.save.track_only {
                                continue;
                            }
                            slot.needs_l1 = true;
                            mark_pending_if_diverged(slot);
                        }
                        reconcile_all(
                            &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx,
                            &cloud_heads,
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
                    let mut delay = Duration::from_secs(debounce_secs);
                    // Was a debounce window already open? It is only announced on the
                    // rising edge: re-announcing on every fs event is what flooded the
                    // feed with orphaned "queued" rows when a game autosaves every
                    // second.
                    let already_scheduled = slots
                        .get(&save_id)
                        .is_some_and(|s| s.next_scheduled_backup_at.is_some());
                    if let Some(slot) = slots.get_mut(&save_id) {
                        // Watcher hints (ADR 0021 C.1): they mark pending and focus the
                        // save for L1 sampling, since an in-place rewrite in a
                        // subdirectory does not move the folder's own mtime and L0
                        // would not see it. The reducer decides; this only brings a
                        // tick forward. The min-interval floor is no longer computed
                        // here: it lives in the reducer (`next_backup_at`).
                        slot.has_pending = true;
                        slot.last_fs_event_at = Some(now);
                        slot.needs_l1 = true;
                        // The anti-starvation cap. Every fs event restarts the
                        // debounce, so a game writing every second would never settle
                        // ("it all stayed queued"). This anchors the oldest change
                        // without flushing; past MAX_BACKUP_WAIT_SECS it stops
                        // restarting and nudges now, even while writes keep arriving.
                        let waited_since = *slot.first_pending_event_at.get_or_insert(now);
                        if (now - waited_since).whole_seconds() >= MAX_BACKUP_WAIT_SECS {
                            delay = Duration::ZERO;
                            slot.first_pending_event_at = Some(now);
                        }
                        slot.next_scheduled_backup_at = Some(now + delay);
                        // (Re)arms the debounce timer: on settling it fires a nudge
                        // that runs `reconcile_all` without waiting for the poll tick.
                        // Cancelling the previous one restarts the debounce, as
                        // before. The reducer may still defer the upload
                        // (min-interval).
                        if let Some(p) = slot.pending.take() {
                            p.abort();
                        }
                        let nudge = nudge_tx.clone();
                        slot.pending = Some(tokio::spawn(async move {
                            if delay > Duration::ZERO {
                                tokio::time::sleep(delay).await;
                            }
                            let _ = nudge.send(()).await;
                        }));
                    }
                    tracing::info!(
                        save_id = %save_id,
                        path = %path.display(),
                        delay_ms = delay.as_millis() as u64,
                        "agent: fs event observed; nudging reconcile after debounce"
                    );
                    // The UI's "next backup in Xs" pill. It used to be emitted by
                    // `schedule_backup`; now the data lives here, in the debounce
                    // timer. The same rules as before: nothing at zero delay and
                    // nothing when the window was already open. (The reducer may still
                    // defer the upload on min-interval; the announcement belongs to
                    // the debounce, as it always did.)
                    if delay > Duration::ZERO && !already_scheduled {
                        let _ = events_tx.try_send(AgentEvent::BackupScheduled {
                            save_id: save_id.clone(),
                            delay_ms: delay.as_millis() as u64,
                            reason: BackupReason::FilesystemSettled,
                        });
                    }

                    // Detection (phase 3, ADR 0020): the folder was rewritten, so
                    // sample the live game processes and record the process-to-write
                    // correlation. It feeds attribution and scoring's +0.50 signal for
                    // future discoveries.
                    sys.refresh_processes_specifics(
                        ProcessesToUpdate::All,
                        true,
                        proc_refresh_kind(),
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
                // Observing the world includes observing the CLOUD (ADR 0021 D.12),
                // not just the disk and the process table. The deadline is set by
                // `due_for_self_observation`: a healthy client pushes the heads before
                // it expires and this never fires, so the steady-state cost is zero
                // and the worst case one manifest per interval. The query goes to its
                // own task (the loop cannot block for a minute on a GET) and comes
                // back as `CloudHeadsObserved`.
                let cloud_now = OffsetDateTime::now_utc();
                let probe_free = cloud_probe.as_ref().is_none_or(JoinHandle::is_finished);
                if probe_free && cloud_heads.due_for_self_observation(cloud_now) {
                    cloud_heads.last_attempt_at = Some(cloud_now);
                    cloud_probe = Some(tokio::spawn(observe_cloud_heads(
                        api.clone(),
                        cmd_tx.clone(),
                    )));
                }

                // Playtime: observing the world also includes telling the server what
                // this machine has played. The consent gate is read inside the task,
                // fresh, so turning the switch off takes effect within the same
                // interval.
                let ship_free = playtime_ship.as_ref().is_none_or(JoinHandle::is_finished);
                if ship_free && tokio::time::Instant::now() >= playtime_ship_due {
                    playtime_ship_due = tokio::time::Instant::now() + PLAYTIME_SHIP_INTERVAL;
                    playtime_ship = Some(tokio::spawn(ship_playtime(
                        api.clone(),
                        playtime_path.clone(),
                    )));
                }

                // Refresh the Steam index when its TTL expired (cheap in the steady
                // state) before the poll attributes hours by folder.
                steam_index.refresh_if_stale();
                let any_running = process_poll(
                    &mut sys, &mut slots, &events_tx, &config,
                    &mut playtime, playtime_path.as_deref(), &mut reported_heavy,
                    &mut corr_store, corr_path.as_deref(), &steam_index, &mut prev_pids,
                    &mut corr_running,
                );
                // Watcher self-healing: a slot whose folder didn't exist when
                // the game was tracked (freshly installed, save dir created on
                // first save) never armed its watcher, and nothing rearms it
                // short of an auto-restore or an app restart. Every tick,
                // (re)arm any slot that has no watcher but whose folder now
                // exists. Cheap (a stat per tracked save) and silent for the
                // common already-armed case.
                for slot in slots.values_mut() {
                    if slot.save.track_only {
                        continue;
                    }
                    if slot.watcher.is_none() && slot.save.local_path.is_dir() {
                        tracing::info!(
                            save_id = %slot.save.save_id,
                            path = %slot.save.local_path.display(),
                            "agent: save folder now present; rearming fs watcher"
                        );
                        arm_watcher(slot, &fs_tx);
                    }
                }
                // The reconciliation (ADR 0021 C.1): the tick is the source of truth.
                // `process_poll` already sampled the world (processes into
                // `is_running`, events, playtime); now each slot is reconciled against
                // the reducer. It replaces both the old `sweep_for_auto_restore` and
                // the flush and pull in `process_poll`'s transitions: the reducer
                // emits a restore for empty or outdated folders, the final flush
                // backup when the game closes, and the deferred pull's landing, all
                // level-triggered, with no policy in the loop.
                reconcile_all(
                    &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx, &cloud_heads,
                );

                // DETECCIÓN (fase 3, ADR 0020): sonda de candidatos. `sys` ya
                // viene refrescado por `process_poll`. Para cada candidato no
                // tracked, when its folder was rewritten since the last tick
                // y hay un juego vivo, registra la correlación. Esto es lo que
                // rompe el huevo-y-gallina: el siguiente escaneo verá el bonus
                // +0.50 y ascenderá el candidato a `High`.
                if !probes.is_empty() {
                    probe_candidates(&mut probes, &sys, &mut corr_store, corr_path.as_deref());
                }

                // Adapt the poll cadence to whether anything is running. Only
                // rebuild the interval on an actual transition so steady state
                // never churns the timer.
                if any_running != polling_fast {
                    polling_fast = any_running;
                    let period = if any_running { active_poll } else { idle_poll };
                    poll = tokio::time::interval_at(TokioInstant::now() + period, period);
                    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }
            }

            // ----- Backup completions -----
            Some(done) = done_rx.recv() => {
                // The upload finished. In the inverted model its result is an
                // *input* to the reducer: here it is only translated (the executor's
                // conversion, ADR 0021 D.7). `committed` travels as
                // `OpResult::Ok { wrote }`, the commit-versus-no-op discriminant the
                // reducer uses to anchor, or NOT anchor, the min-interval: a skip,
                // an unchanged, an empty, an archived, or the 409 settled onto the
                // head, is not a backup and must not move the anchor (the R.E.P.O.
                // regression). That distinction lived here as shell bookkeeping; now
                // it is in the kernel (D.8.2), where C.5's replay reproduces it.
                if let Some(slot) = slots.get_mut(&done.save_id) {
                    slot.next_scheduled_backup_at = None;
                    slot.first_pending_event_at = None;
                    if let Some(h) = &done.new_set_hash {
                        slot.last_set_hash = Some(h.clone());
                    }
                    slot.pending_op_result = Some(kernel::OpResult::Ok {
                        version: done.version_num,
                        fingerprint: done.new_set_hash.as_deref().map(fingerprint_from_set_hash),
                        wrote: done.committed,
                    });
                    // The content-addressed check's answer (D.8.3) travels in the
                    // same observation as the op's result: it is what lets the
                    // reducer tell this no-op (nothing uploaded and nothing written
                    // to the folder) from the 409 settled onto the head, which did
                    // write and therefore stamps `last_restore_at`.
                    if done.landed {
                        slot.pending_upload_landed = Some(true);
                    }
                }
                reconcile_all(
                    &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx, &cloud_heads,
                );
            }

            // ----- Nudge de reconciliación (debounce fs asentado) -----
            Some(()) = nudge_rx.recv() => {
                // They coalesce: a burst of autosaves across several slots leaves
                // several nudges; we drain them and reconcile once.
                while nudge_rx.try_recv().is_ok() {}
                reconcile_all(
                    &mut slots, &api, &events_tx, &cmd_tx, &config, &done_tx, &cloud_heads,
                );
            }
        }
    }
}

/// Marks `has_pending` when the local content diverges from what is already synced
/// (an L1 fingerprint different from `synced_fingerprint`), which is the condition
/// for the reducer to take a backup. Used where there was no fs event but there may
/// be something to upload: an add with content, the hourly sweep (`SweepAll`) and a
/// manual backup (`BackupNow`). It only marks on REAL divergence, so the veto, which
/// reads `has_pending`, stays honest (marking it spuriously would veto pulls
/// forever). An empty folder or a track-only slot is not marked (there is nothing to
/// upload; an empty one is resolved by the reducer through the restore branch).
fn mark_pending_if_diverged(slot: &mut SaveSlot) {
    if slot.save.track_only || is_path_empty_or_missing(&slot.save.local_path) {
        return;
    }
    let fp = observe_local_fingerprint(&slot.save.local_path, &slot.save.game_slug);
    if fp.is_some() && fp != slot.synced_fingerprint {
        slot.has_pending = true;
        slot.needs_l1 = true;
    }
}

/// Register a save with the agent and arm its fs watcher immediately.
///
/// Pre-1.4 this deferred the watcher to `GameStarted`, which silently broke
/// autobackup for saves whose Ludusavi manifest entry had no `processes`
/// and that weren't a Steam install: the process poll never matched, the
/// watcher never armed, no events fired, the Dashboard pill stayed
/// "Inactivo" forever. Arming up front trades one inotify slot per tracked
/// save for end-to-end reliability; `process_poll` still emits
/// `GameStarted`/`GameStopped` for UI signalling but no longer gates the
/// fs subsystem.
///
/// Slice 2b (ADR 0021): it no longer launches the on-add restore or the initial
/// backup by hand. It only registers the slot, seeds the synced fingerprint from the
/// persisted set-hash, and marks `has_pending` when there is diverging content. The
/// reducer, in the `reconcile_all` that follows the `AddSave`, decides: it restores
/// an empty or outdated folder, uploads the baseline of new content, and the recency
/// veto defers when the user is mid-session (replacing the old
/// `is_path_recently_touched` check).
fn handle_add(
    slots: &mut HashMap<String, SaveSlot>,
    save: WatchedSave,
    fs_tx: &mpsc::Sender<PathBuf>,
) {
    let save_id = save.save_id.clone();
    let known_version = save.known_version;
    let last_set_hash = save.set_hash.clone();
    // Seed the synced fingerprint from the persisted set-hash (state.json) so
    // "converged means zero actions" holds from the first tick: without it a save
    // already uploaded would re-upload its baseline on start.
    let synced_fingerprint = last_set_hash.as_deref().map(fingerprint_from_set_hash);
    let mut slot = SaveSlot {
        save,
        watcher: None,
        pending: None,
        burst_since: None,
        burst_backups: 0,
        manual_requested: false,
        is_running: false,
        weak_session: false,
        last_running_seen: None,
        has_pending: false,
        last_fs_event_at: None,
        last_restore_at: None,
        next_scheduled_backup_at: None,
        first_pending_event_at: None,
        last_backup_at: None,
        in_flight: None,
        next_backup_at: None,
        next_restore_at: None,
        restore_failures: kernel::RestoreFailures::default(),
        backup_conflict: kernel::ConflictStall::default(),
        last_set_hash,
        synced_fingerprint,
        last_l0_mtime: None,
        needs_l1: false,
        pending_op_result: None,
        pending_upload_landed: None,
        last_restore_error: None,
        last_conflict_error: None,
        known_version,
        pull_pending: false,
        deferred_notified: false,
    };
    // Playtime-only entries exist purely to be matched by the process poll
    // so their hours accrue for the recap. They own no save folder, so we
    // never arm a watcher or run any restore/backup logic for them.
    if slot.save.track_only {
        slots.insert(save_id, slot);
        return;
    }
    arm_watcher(&mut slot, fs_tx);
    // Content already on disk that diverges from what is synced (a fresh add with no
    // set-hash, the emulator case, or offline changes): seed `has_pending` so the
    // reducer takes the baseline. Empty plus restore enabled means the reducer
    // restores.
    mark_pending_if_diverged(&mut slot);
    slots.insert(save_id, slot);
}

/// Idle process-poll slowdown factor. When no tracked game is running the agent
/// polls the process table every `poll_secs * IDLE_POLL_MULT` instead of every
/// `poll_secs`. Scanning every process on the box is the agent's dominant idle
/// cost, and while idle there's nothing to detect "stopping": only launches,
/// whose detection just gains up to one idle interval of latency (absorbed by
/// the conflict-aware pre-launch barrier). The first running game snaps the
/// cadence back to `poll_secs`.
const IDLE_POLL_MULT: u32 = 4;

/// CPU floor (sysinfo `cpu_usage()`, where 100.0 = one fully-used core) above
/// which a *game-like, untracked* process is treated as a just-launched game
/// worth an immediate detection scan (`AgentEvent::HeavyProcessDetected`). Set
/// low enough to catch lightweight indie titles, high enough that idle helper
/// processes that slip past `correlation::is_game_like` don't keep firing. A
/// false positive only costs one cheap metadata scan (debounced desktop-side),
/// so we bias toward catching games.
const HEAVY_PROCESS_CPU_PCT: f32 = 25.0;

/// The CPU floor (sysinfo's `cpu_usage()`, where 100.0 is one core maxed out) for a
/// CORRELATION match to count as "the game is running". Process names declared by
/// the manifest count with or without CPU (a paused game is still "running"), but
/// correlation's folder-to-process attribution is noisy: a background utility (RTSS,
/// ctfmon, taskhostw, RadeonSoftware) that touches a save folder ends up correlated
/// and, idling at about 0%, would fire a false "it started" and a false auto-restore
/// barrier. Demanding real CPU separates an off-catalogue game actively running from
/// a helper at rest. Below `HEAVY_PROCESS_CPU_PCT` so a moderately active game still
/// counts.
const CORRELATION_MIN_CPU_PCT: f32 = 5.0;

/// How much a write to the folder is worth as proof that "of all the saves sharing
/// this process, THIS one is being played".
///
/// Ten titles from one emulated console declare the same executable, so the process
/// name does not choose between them and we have to look at which one receives the
/// saves. The window is generous on purpose: a game that saves every twenty minutes
/// would still count between autosaves, and the price of overshooting is bounded,
/// since the other title would have had to save within the same window to slip in.
const SHARED_PROCESS_ACTIVITY: time::Duration = time::Duration::minutes(30);

/// Grace window (in *poll ticks*) before a slot that dropped out of the running
/// set is declared stopped. A correlation match is CPU-gated
/// (`CORRELATION_MIN_CPU_PCT`), so a game idling in a menu or grinding a loading
/// screen can dip under the floor for a tick and momentarily look stopped;
/// without this it flaps GameStarted/Stopped (and a final-flush backup) every
/// few seconds. We keep the slot "running" for this many consecutive
/// not-seen polls (converted to seconds via `poll_secs`, floored at
/// [`STRONG_STOP_GRACE_FLOOR_SECS`]) before firing GameStopped. A genuine quit
/// still resolves within the grace.
const RUNNING_STICKY_POLLS: u64 = 3;

/// Floor for the strong-signal stop grace (see the `sticky` computation below). It
/// only has to swallow a rare one-tick process-table refresh race, so a handful of
/// seconds is plenty. It was 90 s, badly over-provisioned: because the
/// [`mid_session_reason`] veto keys on `is_running`, that 90 s got tacked onto
/// *every* GameStopped, inflating both close-detection latency ("two minutes to
/// notice I quit") and cross-device restore latency (the receiver keeps vetoing
/// pulls for exactly this long after the game quits). 6 s is about
/// RUNNING_STICKY_POLLS ticks at the default 2 s poll, still comfortably above any
/// real refresh hiccup.
const STRONG_STOP_GRACE_FLOOR_SECS: u64 = 6;

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
#[allow(clippy::too_many_arguments)]
fn spawn_auto_restore(
    save: WatchedSave,
    api: ApiClient,
    events_tx: mpsc::Sender<AgentEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    conflict_root: Option<PathBuf>,
    conflict_retention_days: u32,
    known_version: Option<i64>,
    // Latest cloud version the `cloud_pull` poller last reported for this save,
    // when known. Lets `run_auto_restore` version-gate without its own metadata
    // fetch. `None` falls back to the per-save network call (self-hosted,
    // headless CLI, fresh add, or the authoritative force/barrier paths).
    cached_latest: Option<i64>,
    // One manifest fetch shared by every restore of the same sweep: when
    // `cached_latest` is `None` on a cold start (the poller hasn't filled the
    // cache yet) the first task pulls `/v1/cloud/sync` once and the rest
    // reuse it, instead of N tasks fetching the identical manifest (the
    // startup burst that tripped the server's poll guard).
    shared_manifest: Option<Arc<tokio::sync::OnceCell<crate::api::CloudManifest>>>,
) {
    tokio::spawn(async move {
        tracing::debug!(
            save_id = %save.save_id,
            game_slug = %save.game_slug,
            path = %save.local_path.display(),
            "agent: auto-restore diff, checking server snapshot against local"
        );
        let retention = Duration::from_secs(u64::from(conflict_retention_days) * 86_400);
        let mut disposition = AutoRestoreDisposition::Ok;
        let mut synced_version: Option<i64> = None;
        // Adopted as the slot's `last_set_hash` only when the merge left the
        // tree equal to head (no divergence), so the writes the merge made
        // don't bounce back as a redundant upload. Stays `None` on a diverged
        // tree so the genuinely-new local content still uploads.
        let mut post_restore_set_hash: Option<String> = None;
        // True once we've actually written pulled files into the folder, used
        // to stamp `last_restore_at` so our own writes don't veto the next pull.
        let mut wrote_files = false;
        match run_auto_restore(
            &api,
            &save,
            conflict_root.as_deref(),
            retention,
            known_version,
            cached_latest,
            shared_manifest,
        )
        .await
        {
            Ok(AutoRestorePull::Merged(outcome)) => {
                // We downloaded and diffed against this version; remember it so
                // the next sweep can short-circuit.
                synced_version = Some(outcome.version_num);
                // When the merged tree equals head, adopt its signature so the
                // restore's own writes don't trigger a redundant re-upload.
                if !outcome.local_diverged {
                    post_restore_set_hash = outcome.disk_set_hash.clone();
                }
                let touched = outcome.files_restored + outcome.conflicts_backed_up;
                wrote_files = touched > 0;
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
                    // if it was already armed; `arm_watcher` overwrites.
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
                // else: every file present and identical, so a silent no-op.
            }
            // Nothing to pull, either way. `run_auto_restore` already logged which
            // case it was, so don't second-guess it here; the old unconditional "no
            // snapshots yet" line contradicted the up-to-date path. Neither arm
            // touches `synced_version`: the gate only fires when this device is
            // already at or past head, so writing the server's number back could walk
            // our own cursor backwards.
            Ok(AutoRestorePull::AlreadyAtHead { .. }) => {}
            Ok(AutoRestorePull::NothingRemote) => {}
            Err(e) => {
                // A 404 means the save has no record/snapshot on the backend
                // (carried over from another account, stale state, or the
                // remote was purged). It's not a transient failure, so don't
                // raise it to the user as an error and don't keep retrying on
                // the short cooldown; park it on a long backoff (below).
                let api_err = e.downcast_ref::<ApiError>();
                let not_on_server = matches!(api_err, Some(ApiError::NotFound));
                // A 401 is session-wide, not per-save: at launch the stored cloud JWT
                // can be expired and the desktop's refresh path hasn't pushed a fresh
                // token into this client yet, so the startup reconciliation sweep
                // would emit one `SaveAutoRestoreFailed` per tracked save, a burst of
                // "could not restore" popups. Swallow it (the global cloud status
                // already reflects the session problem) and let the normal short
                // cooldown retry once the token is refreshed.
                let unauthorized = matches!(api_err, Some(ApiError::Unauthorized));
                // A 429 is the rolling bandwidth limiter, not a per-save failure:
                // during a reconciliation sweep every tracked save races for the
                // same window, so one over-quota moment 429s a dozen restores at
                // once. Treated as a failure it burned the escalation budget and
                // fired "keeps failing to restore (3×)" for saves that were never
                // broken. Honour the server's retry_after and don't count it.
                let throttled = match api_err {
                    Some(ApiError::RateLimited {
                        retry_after_seconds,
                        ..
                    }) => Some(*retry_after_seconds),
                    _ => None,
                };
                if not_on_server {
                    disposition = AutoRestoreDisposition::NotOnServer;
                    tracing::debug!(
                        save_id = %save.save_id,
                        "agent: auto-restore: save not on server (404); backing off"
                    );
                } else if let Some(retry_after_secs) = throttled {
                    disposition = AutoRestoreDisposition::Throttled { retry_after_secs };
                    tracing::debug!(
                        save_id = %save.save_id,
                        retry_after_secs,
                        "agent: auto-restore throttled (429); waiting the server window"
                    );
                } else if unauthorized {
                    disposition = AutoRestoreDisposition::Unauthorized;
                    tracing::debug!(
                        save_id = %save.save_id,
                        "agent: auto-restore deferred, session unauthorized (token refresh pending)"
                    );
                } else {
                    let chain = format!("{e:#}");
                    tracing::warn!(
                        save_id = %save.save_id,
                        error = %chain,
                        "agent: auto-restore failed"
                    );
                    let _ = events_tx
                        .send(AgentEvent::SaveAutoRestoreFailed {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            error: chain.clone(),
                        })
                        .await;
                    disposition = AutoRestoreDisposition::Failed(chain);
                }
            }
        }
        // Always clear the slot's `restoring` flag, even on failure: the
        // reconciliation sweep is responsible for retrying once the
        // cooldown (or, on repeated failures, the escalating backoff the
        // handler arms from `outcome`) expires; we just need to mark this
        // attempt as done.
        let _ = cmd_tx
            .send(AgentCommand::AutoRestoreFinished {
                id: save.save_id.clone(),
                disposition,
                synced_version,
                post_restore_set_hash,
                wrote_files,
            })
            .await;
    });
}

/// The save folder's own mtime (its inode, not recursive), as an
/// `OffsetDateTime`, or `None` if it can't be stat'd. This is the sampled
/// [`kernel::Observation::folder_mtime`] the sans-IO session veto consumes:
/// same source as [`is_path_recently_touched`], but the recency comparison
/// now lives in the kernel against an injected `now`.
fn folder_own_mtime(path: &Path) -> Option<OffsetDateTime> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(OffsetDateTime::from)
}

/// The greatest mtime between the folder itself and its immediate files (not
/// recursive: cheap and enough, since a save being written leaves a new or touched
/// file at the first level, such as Factorio's `.zip` in `saves/`). `None` when the
/// folder cannot be read.
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

/// Walks the probed candidate folders, updates their mtime baselines and returns the
/// ones rewritten since the last tick. The `None` baseline (a first sighting) is only
/// seeded, never reported, which avoids attributing a pre-existing recent file to a
/// write nobody witnessed. Pure (no process IO and no persistence) so it can be
/// tested.
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

/// Detection (phase 3, ADR 0020): probes the candidates and, for the ones rewritten
/// since the last tick, records the process-to-write correlation when a game is alive
/// and persists the store. It is what breaks the chicken and egg: playing an
/// untracked game finally leaves the +0.50 trace the next scan needs to promote it to
/// `High`.
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
    // Processes are only sampled when there really was a write (lazily).
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
    /// newer than the remote (or `conflict_root` was unset; see
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
    /// True when the merged local tree is strictly ahead of the head we pulled:
    /// some local file was newer (kept on mtime) or local-only. The conflict
    /// reconcile path uses this to decide whether the follow-up upload carries
    /// real data (`true` → push it, fast-forwarding from the new head) or would
    /// just mint a redundant copy of head (`false` → settle without uploading).
    local_diverged: bool,
    /// Cheap set signature (`"<paths+sizes+mtimes>:"`, content half empty) of
    /// the local folder *after* the merge, in the exact format
    /// `upload_directory_checked` compares against. When the tree matches head
    /// (`!local_diverged`) the caller stores this as the slot's
    /// `last_set_hash`, so the fs events the merge itself triggered settle as a
    /// no-op `Skipped` instead of firing a redundant upload. `None` if the
    /// post-merge walk failed (best-effort; we just skip the optimisation).
    disk_set_hash: Option<String>,
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
    /// Files present locally (`target`) but absent from the remote snapshot
    /// (`source`): local-only content the merge left untouched. Together with
    /// `conflicts_resolved_local` this tells the caller whether the merged tree
    /// genuinely diverges from the head (a follow-up upload carries real data)
    /// or matches it exactly (re-uploading would only mint a redundant no-op
    /// version). Counted with the same recursive walk as the restore itself.
    pub target_only: usize,
}

async fn run_auto_restore(
    api: &ApiClient,
    save: &WatchedSave,
    conflict_root: Option<&Path>,
    retention: Duration,
    known_version: Option<i64>,
    // A head somebody else already learned, to skip the fetch. It carries a
    // version but no id, so it can only be trusted for a save whose local id
    // *is* the cloud's, and every caller today passes `None` and takes the
    // resolving path below. Don't start passing it without carrying the id
    // alongside: a version read off one row and downloaded from another 404s.
    cached_latest: Option<i64>,
    shared_manifest: Option<Arc<tokio::sync::OnceCell<crate::api::CloudManifest>>>,
) -> Result<AutoRestorePull> {
    // Prefer the version the cloud_pull poller already learned this tick: it
    // fetched the whole manifest once, so reusing it spares us a per-save
    // `cloud_sync`/`get_save` round-trip (the old sweep N+1). When the poller
    // cache is cold (cold start), the sweep's `shared_manifest` cell fills
    // that role: one fetch for the whole batch of restores. Only the
    // authoritative force-restore / pre-launch barrier paths (which pass
    // neither) and self-hosted / headless one-offs hit the network per save.
    // Which cloud row is this save, and how far ahead is it? The two answers
    // come together on purpose: the id we must do the IO with is not always the
    // id we track locally (see `CloudManifest::entry_for`), and a version read
    // off one row while downloading from another is how a restore 404s.
    let (cloud_id, latest) = match cached_latest {
        Some(v) => (save.save_id.clone(), Some(v)),
        None => {
            if api.is_cloud().await {
                let manifest = match &shared_manifest {
                    Some(cell) => cell
                        .get_or_try_init(|| async { api.cloud_sync().await })
                        .await?
                        .clone(),
                    None => api.cloud_sync().await?,
                };
                match manifest.entry_for(&save.save_id, &save.game_slug, &save.label) {
                    Some(e) => {
                        if e.save_id != save.save_id {
                            // Loud, because for two weeks this was silent: the
                            // lookup missed, the sweep read "the server has
                            // nothing", and a save that was fourteen versions
                            // behind looked converged.
                            tracing::warn!(
                                local_save_id = %save.save_id,
                                cloud_save_id = %e.save_id,
                                game_slug = %save.game_slug,
                                label = %save.label,
                                "agent: local save id isn't the cloud's; matched by (game, label) instead"
                            );
                        }
                        (e.save_id.clone(), Some(e.latest_version_num))
                    }
                    None => (save.save_id.clone(), None),
                }
            } else {
                // Self-hosted addresses rows by the id in the URL and never
                // relabels them, so there is nothing to resolve.
                (
                    save.save_id.clone(),
                    api.get_save(&save.save_id).await?.latest_version_num,
                )
            }
        }
    };
    // Version gate: if we're already synced to the server's latest version,
    // there's nothing newer from another device to pull, so skip the expensive
    // download-to-diff entirely. This is the fix for the bandwidth blowout:
    // the sweep used to re-download the full snapshot every ~50s just to diff
    // it against a folder that hadn't changed, exhausting the 15-min cloud
    // quota (429 storm) and starving real uploads. A genuine cross-device
    // update bumps the server version above `known_version` and still pulls.
    // A locally empty/missing folder is the one case where "already on the
    // latest version" is a lie worth pulling for: the user wiped the save
    // (manual cleanup, uninstall, deleted folder) and the cloud copy is the
    // only one left. Restoring it is exactly what they want, so don't let the
    // version gate short-circuit an empty folder, so fall through to the download
    // even when `known >= v`.
    if let (Some(v), Some(known)) = (latest, known_version) {
        if known >= v && !is_path_empty_or_missing(&save.local_path) {
            tracing::debug!(
                save_id = %save.save_id,
                version = v,
                "agent: auto-restore: already synced to latest version; skipping download"
            );
            if let Some(root) = conflict_root {
                if let Err(e) = cleanup_old_conflicts(root, retention).await {
                    tracing::debug!(error = %e, "cleanup_old_conflicts failed (up-to-date path)");
                }
            }
            return Ok(AutoRestorePull::AlreadyAtHead { version_num: v });
        }
    }
    let Some(version) = latest else {
        tracing::debug!(
            save_id = %save.save_id,
            "agent: auto-restore: the server has no snapshots yet; nothing to restore"
        );
        // Still sweep TTL before bailing, which keeps the conflict dir bounded
        // even for saves whose remote has been purged.
        if let Some(root) = conflict_root {
            if let Err(e) = cleanup_old_conflicts(root, retention).await {
                tracing::debug!(error = %e, "cleanup_old_conflicts failed (no-snapshot path)");
            }
        }
        return Ok(AutoRestorePull::NothingRemote);
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
        // The cloud's id for this row, which is what the manifest and blob
        // endpoints answer to. Passing the local one 404s whenever the two
        // drifted apart.
        &cloud_id,
        version,
        &staging,
        crate::restore::RestoreOptions {
            skip_verify: false,
            force: false,
            // Dedup against the *live* folder, not `staging`: staging is empty
            // by construction, so indexing it would find nothing to reuse.
            // Files already on disk are copied into staging instead of pulled
            // from R2, and the merge below treats them exactly like downloaded
            // ones (ADR 0021 D.13).
            reuse_from: Some(save.local_path.clone()),
            // Auto-restore: gate shut unless the user opened it for this game.
            // Writing the config of the PC that uploaded the snapshot over this
            // one with nobody watching is exactly the crash to avoid, so the
            // default stays no; but in some games the config and the save are
            // the same file, and there keeping it shut restores half a save.
            gate: hoard_core::kernel::fileclass::RestoreGate {
                shields: crate::savefilter::shields_for_slug(&save.game_slug),
                allow_device_local: save.allow_device_local.unwrap_or(false),
            },
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
    // it lazily *only if* a conflict_root is configured; `restore_files_into`
    // treats `None` as the safe legacy fallback.
    let conflict_backup_dir: Option<PathBuf> = conflict_root.map(|root| {
        let ts = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown-ts".to_string())
            // Colons aren't legal in Windows paths and look weird everywhere.
            .replace(':', "-");
        root.join(&save.save_id).join(ts)
    });

    let copy_result = restore_files_into(
        &save.local_path,
        &staging,
        conflict_backup_dir.as_deref(),
        &crate::savefilter::shields_for_slug(&save.game_slug),
    )
    .await;
    cleanup_staging(&staging).await;

    // Best-effort TTL sweep regardless of the per-file outcome, because we want
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

    // Did anything local survive the merge that head doesn't have? A newer
    // local file (kept on mtime) or a local-only file means the merged tree is
    // ahead of head; otherwise the tree now equals head exactly.
    let local_diverged = stats.conflicts_resolved_local > 0 || stats.target_only > 0;
    // Cheap (no byte reads) signature of the merged folder, in the composite
    // `"<cheap>:"` shape `upload_directory_checked` splits on, and the empty
    // content half is fine because the fast-path skip only compares the cheap
    // half. Best-effort: a walk error just drops the redundant-upload
    // optimisation, never blocks the restore.
    let disk_set_hash = crate::backup::walk_source(
        &save.local_path,
        &crate::savefilter::shields_for_slug(&save.game_slug),
    )
    .ok()
    .map(|files| format!("{}:", crate::backup::compute_set_signature(&files)));

    Ok(AutoRestorePull::Merged(AutoRestoreOutcome {
        version_num: version,
        files_restored: stats.restored as u64,
        conflicts_local_wins: stats.conflicts_resolved_local as u64,
        conflicts_backed_up: stats.conflicts_backed_up as u64,
        conflict_dir: dir_used,
        bytes_extracted: stats.bytes_restored,
        local_diverged,
        disk_set_hash,
    }))
}

/// What a reconcile pull found. Three answers and not `Option`, because the two
/// empty ones are not the same fact and the 409 handler has to tell them apart:
/// "the local folder already holds the server's head" is a safe place to
/// fast-forward from, while "the server has no row I can see" means we know
/// nothing about the remote content and must not push over it.
enum AutoRestorePull {
    /// Downloaded and merged into the live folder.
    Merged(AutoRestoreOutcome),
    /// The version gate held: this device is already at the server's head, so
    /// there was nothing newer to pull.
    AlreadyAtHead { version_num: i64 },
    /// The server has no snapshot for this save: purged, or a row we can't resolve.
    NothingRemote,
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
/// logged but never propagated: a stuck conflict dir is much better than
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
///     `conflict_backup_dir` is `None`, *do not* overwrite: bump
///     `conflicts_resolved_local` as a safety fallback (legacy 1.5.4
///     behaviour) and log a warn.
///
/// Errors propagate only for I/O failures we can't classify (e.g.
/// permission denied reading a file we just listed).
pub(crate) async fn restore_files_into(
    target: &Path,
    source: &Path,
    conflict_backup_dir: Option<&Path>,
    shields: &[String],
) -> Result<RestoreStats> {
    let mut stats = RestoreStats::default();
    let mut stack: Vec<PathBuf> = vec![source.to_path_buf()];
    // Relative paths seen in the remote snapshot. Used after the merge to spot
    // local-only files (in `target`, not in `source`) → `stats.target_only`.
    let mut source_rels: HashSet<PathBuf> = HashSet::new();

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
                // Skip symlinks, devices and the like: they shouldn't appear in a
                // hoard snapshot but we'd rather no-op than crash.
                continue;
            }
            let rel = path
                .strip_prefix(source)
                .with_context(|| format!("path {} not under source", path.display()))?;
            source_rels.insert(rel.to_path_buf());
            let dest = target.join(rel);
            if dest.exists() {
                if files_have_equal_bytes(&path, &dest).await? {
                    stats.skipped += 1;
                    continue;
                }
                // Bytes differ. The resolution policy is the kernel's; this shell
                // samples the mtime winner and executes the chosen branch. A one-second
                // tolerance covers FAT32 and friends; remote ties take the local side
                // so a close call doesn't trash data.
                let local_wins = local_mtime_wins(&dest, &path).await;
                let backup_root = match kernel::restore_merge::resolve_conflict(
                    local_wins,
                    conflict_backup_dir.is_some(),
                ) {
                    kernel::restore_merge::ConflictResolution::KeepLocal => {
                        if local_wins {
                            tracing::debug!(
                                rel = %rel.display(),
                                "auto-restore diff: local wins on mtime"
                            );
                        } else {
                            // Remote looked newer but there's no
                            // conflict_backup_dir (legacy fallback): never
                            // destroy local data.
                            tracing::warn!(
                                rel = %rel.display(),
                                "auto-restore diff: remote appears newer but no conflict_backup_dir; keeping local"
                            );
                        }
                        stats.conflicts_resolved_local += 1;
                        continue;
                    }
                    kernel::restore_merge::ConflictResolution::BackupThenTakeRemote => {
                        conflict_backup_dir
                            .expect("BackupThenTakeRemote is only chosen when a backup dir exists")
                    }
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
                preserve_staging_mtime(&path, &dest).await;
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
            preserve_staging_mtime(&path, &dest).await;
            stats.restored += 1;
            stats.bytes_restored += copied;
        }
    }

    // Second pass over `target`: count files the snapshot didn't carry. These
    // are local-only and survive the merge, so the merged tree is strictly
    // ahead of head and a follow-up upload is real, not redundant. We don't
    // filter transient lock files here: a stray lock counting as divergence
    // only costs one extra upload (the safe direction), never a skipped one.
    let mut tstack: Vec<PathBuf> = vec![target.to_path_buf()];
    while let Some(dir) = tstack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            // The folder can be empty or vanish mid-walk; treat as no extras.
            Err(_) => continue,
        };
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                tstack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(target) else {
                continue;
            };
            // A file the backup NEVER uploads is not divergence either.
            // `disk_set_hash` is computed with `walk_source`, which already leaves the
            // litter out; counting it here puts the two halves out of step and marks
            // `local_diverged` on every auto-restore of any game with a `Player.log`
            // or a `.DS_Store`, meaning a full extra walk and content hash, forever.
            //
            // Config does still count, and deliberately: it exists only locally until
            // it is uploaded, so discarding it here would adopt a signature saying "we
            // are in sync" and the next backup would skip the file down the fast path
            // without ever having uploaded it.
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !kernel::fileclass::classify(&rel_str, shields).is_backed_up() {
                continue;
            }
            if !source_rels.contains(rel) {
                stats.target_only += 1;
            }
        }
    }

    Ok(stats)
}

/// Re-stamp `dest` with `src`'s mtime after a copy. `fs::copy` writes the
/// destination with mtime=now, but the staging tree carries the snapshot's
/// original mtimes (restore.rs re-applies them on extraction) and they must
/// survive into the live folder: a game that picks "continue" by
/// most-recent file would otherwise see every restored save as brand-new
/// and load the wrong one, and the follow-up merged-tree upload would
/// record the inflated mtimes server-side, poisoning future merges on
/// other devices. Best-effort: a failure only degrades ordering, never data.
async fn preserve_staging_mtime(src: &Path, dest: &Path) {
    let mtime = match tokio::fs::metadata(src).await.and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                src = %src.display(),
                error = %e,
                "restore: couldn't read staging mtime; destination keeps mtime=now"
            );
            return;
        }
    };
    if let Err(e) = filetime::set_file_mtime(dest, filetime::FileTime::from_system_time(mtime)) {
        tracing::warn!(
            dest = %dest.display(),
            error = %e,
            "restore: couldn't re-apply snapshot mtime; destination keeps mtime=now"
        );
    }
}

/// True when the local file's mtime is more than one second newer than the remote
/// file's. Conservative on errors: when either mtime cannot be read the remote wins,
/// because a snapshot's authority comes from the server's committed timestamps, which
/// are more reliable than a local filesystem with quirks (FAT32's two-second
/// rounding, clock skew on a network share).
async fn local_mtime_wins(local: &Path, remote: &Path) -> bool {
    // Sans-IO boundary: this shell samples both mtimes; the kernel decides.
    // An unreadable file → `None` → the kernel hands the tie to the remote,
    // exactly as the old early-return `false` did.
    let local_mtime = tokio::fs::metadata(local)
        .await
        .and_then(|m| m.modified())
        .ok();
    let remote_mtime = tokio::fs::metadata(remote)
        .await
        .and_then(|m| m.modified())
        .ok();
    kernel::restore_merge::local_wins_on_mtime(local_mtime, remote_mtime)
}

/// A cheap bytes-equal: size first (which saves the read in the common
/// different-sized case), then one shot read of each file and a linear compare. Files
/// in tracked saves are small enough that chunk-streaming would only matter for
/// pathological archives, and the per-file allocation cost is much smaller than the
/// network and zstd cost we already paid to land them in staging.
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
/// keeps running for the other slots. Re-arming later is fine, since we just
/// overwrite the field.
fn arm_watcher(slot: &mut SaveSlot, fs_tx: &mpsc::Sender<PathBuf>) {
    let path = slot.save.local_path.clone();
    if !path.is_dir() && !path.is_file() {
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
    // A single-file save: the PARENT DIRECTORY is watched and filtered by name.
    // Watching the inode directly is no use with games that save the safe way, writing
    // a temporary, deleting the original and renaming the temporary over it, because
    // the file being watched stops existing and the watch dies with it. The parent
    // survives that dance. `watch_root` is still the save's path, which is what the
    // loop uses to match the event to its slot.
    let watch_root = path.to_path_buf();
    let single_file = path.is_file();
    let (watch_target, want_name) = if single_file {
        (
            path.parent().unwrap_or(path).to_path_buf(),
            path.file_name().map(|s| s.to_os_string()),
        )
    } else {
        (path.to_path_buf(), None)
    };
    let mut debouncer = new_debouncer(
        // Internal aggregation window for notify-debouncer-mini. We use a
        // small value (2 s) here and apply our larger product debounce by
        // resetting the schedule timer on each event. That way we still see
        // bursts as a single "settled" signal upstream.
        Duration::from_secs(2),
        move |res: DebounceEventResult| {
            if let Ok(events) = res {
                // With a single file we watch its folder, so the neighbours' events
                // have to be discarded: otherwise any other save in the same folder
                // would wake this one.
                let relevant = match &want_name {
                    Some(name) => events.iter().any(|e| e.path.file_name() == Some(name)),
                    None => !events.is_empty(),
                };
                if relevant {
                    let _ = fs_tx.try_send(watch_root.clone());
                }
            }
        },
    )?;
    let mode = if single_file {
        notify::RecursiveMode::NonRecursive
    } else {
        notify::RecursiveMode::Recursive
    };
    debouncer.watcher().watch(&watch_target, mode)?;
    Ok(debouncer)
}

/// Find which save a path event belongs to. The fs watcher emits the root
/// it was registered for, so this is a direct lookup by canonical prefix.
fn match_save_for_path(slots: &HashMap<String, SaveSlot>, path: &Path) -> Option<String> {
    for slot in slots.values() {
        // Playtime-only slots own no folder; their sentinel path is empty,
        // which `starts_with` would treat as a prefix of *every* path.
        if slot.save.track_only {
            continue;
        }
        if slot.save.local_path == path || path.starts_with(&slot.save.local_path) {
            return Some(slot.save.save_id.clone());
        }
    }
    None
}

/// Sums the byte size of every regular file under `root`, recursively. It reads
/// directory entries and file metadata only, never opening a file, so it is the cheap
/// way to learn a save's footprint for sweep staggering. Unreadable directories and
/// entries are skipped rather than erroring; a best-effort estimate is all the
/// scheduler needs.
pub fn dir_size_bytes(root: &Path) -> u64 {
    // A single-file save occupies whatever that file occupies.
    if root.is_file() {
        return std::fs::metadata(root).map(|m| m.len()).unwrap_or(0);
    }
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
    // The version this device believes is the server head. Sent as the upload's
    // fast-forward base so the server rejects (409 non-fast-forward) when another
    // device advanced the save since we last synced; see the `ApiError::Conflict`
    // arm below, which reconciles and retries instead of burying their version.
    // `None` only for a save never synced from this device (no head yet) and the
    // empty/missing-folder restore path, which never uploads.
    mut base_version: Option<i64>,
    // The server's head (version plus its content's digest) for D.8.3's anti-relaunch
    // check: if what we were about to upload is already that head, the previous upload
    // landed and uploading again would only create a duplicate version.
    head: Option<ServerHead>,
    // What kind of copy this is. It only changes the label stored with the version;
    // the upload path is the same.
    origin: VersionOrigin,
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
            "agent: backup skipped, local folder is empty or missing"
        );
        // A no-op: it clears has_pending (through the shell's bookkeeping) so a
        // future fs event is not blocked. The restore is NO LONGER launched from
        // here; with the folder empty the reducer will emit `Restore` on the next
        // tick (the `local_empty` branch), with no duplicated execution paths. The
        // "backup skipped: empty folder" toast only when restore is disabled
        // (otherwise the reducer fills it).
        let _ = done_tx.try_send(BackupDone {
            save_id: save.save_id.clone(),
            new_set_hash: None,
            committed: false,
            version_num: None,
            landed: false,
        });
        if !auto_restore {
            let _ = events_tx
                .send(AgentEvent::BackupSkippedEmpty {
                    save_id: save.save_id.clone(),
                    game_slug: save.game_slug.clone(),
                    likely_wrong_path: save.known_version.is_none(),
                })
                .await;
        }
        return;
    }
    let mut attempt = 0u32;
    // Bandwidth-limit (429) waits are tracked separately from real-failure
    // retries: a throttle isn't a failure, so it shouldn't eat the small
    // exponential-backoff budget meant for flaky network. We honour the
    // server's `retry_after_secs` and cap how many times we'll sit out a
    // window so a user genuinely parked over quota eventually surfaces.
    let mut throttle_waits = 0u32;
    const MAX_THROTTLE_WAITS: u32 = 5;
    // Fast-forward conflicts (409) are reconciled-then-retried, not backed off.
    // Cap the reconcile loop so a head that keeps advancing under us (a very
    // chatty sibling device) can't spin forever: after this many we surface
    // the conflict as a failure and let the next scheduled backup try fresh.
    let mut conflict_reconciles = 0u32;
    const MAX_CONFLICT_RECONCILES: u32 = 3;
    loop {
        let outcome = upload_directory_checked(
            &api,
            &save.save_id,
            &save.game_slug,
            &save.label,
            &save.local_path,
            prev_set_hash.as_deref(),
            // Fast-forward base: the version this device last synced. The server
            // rejects with 409 (non-fast-forward) if the head moved past it,
            // which the `ApiError::Conflict` arm below catches to reconcile +
            // retry instead of clobbering the newer remote version.
            base_version,
            head.as_ref(),
            origin,
            |_, _| {},
            // Emit "uploading…" only once the signature checks have decided a
            // real upload is happening; a Skipped or Unchanged settle stays
            // quiet in the feed (BUG 2). Only on the first attempt: retries
            // re-firing it filled the feed with "Subiendo… / falló" pairs.
            || {
                if attempt == 0 {
                    let _ = events_tx.try_send(AgentEvent::BackupStarted {
                        save_id: save.save_id.clone(),
                        game_slug: save.game_slug.clone(),
                        label: save.label.clone(),
                    });
                }
            },
        )
        .await;

        match outcome {
            Ok(BackupResult::Skipped) => {
                // The save's cheap set signature is unchanged since the last
                // upload: the watcher fired on a settle that didn't actually
                // write anything. Skip the no-op snapshot, clear has_pending.
                tracing::info!(
                    save_id = %save.save_id,
                    "agent: backup skipped, no content change since last upload"
                );
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: None,
                    committed: false,
                    version_num: None,
                    landed: false,
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
                    "agent: backup skipped, bytes unchanged despite mtime drift"
                );
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: Some(signature),
                    committed: false,
                    version_num: None,
                    landed: false,
                });
                return;
            }
            // The content was already on the server (ADR 0021 D.8.3): the upload a
            // daemon restart left half done had landed after all. Nothing is
            // uploaded; only the version that already holds it is adopted.
            //
            // A `BackupSuccess` is emitted, with `already_landed`, rather than an
            // event of its own, because to the user the fact IS "it is saved in
            // version N", and because it is what makes the service persist
            // `last_version_num` and `set_hash` into `state.json`. Without that row,
            // the next start would see the cloud ahead and download its own content.
            Ok(BackupResult::AlreadyLanded {
                version_num,
                signature,
            }) => {
                tracing::info!(
                    save_id = %save.save_id,
                    version_num,
                    "agent: nothing to upload, this content is already the server's head"
                );
                let _ = events_tx
                    .send(AgentEvent::BackupSuccess {
                        save_id: save.save_id.clone(),
                        version_num,
                        // Zero bytes because zero bytes travelled: the size the UI
                        // shows is the upload's, and there was none here.
                        total_bytes: 0,
                        set_hash: Some(signature.clone()),
                        already_landed: true,
                        deliberate: origin.is_deliberate(),
                    })
                    .await;
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: Some(signature),
                    // NOT a commit: nothing reached the server on this pass, and
                    // moving the min-interval anchor with a no-op is the R.E.P.O.
                    // regression (D.8.2). The version is adopted, though.
                    committed: false,
                    version_num: Some(version_num),
                    landed: true,
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
                        set_hash: Some(signature.clone()),
                        already_landed: false,
                        deliberate: origin.is_deliberate(),
                    })
                    .await;
                // Partial upload: the save was over the plan's per-save cap so
                // only the newest files went up. Fire a second event *after*
                // success so the UI's amber "plan too small" state wins over the
                // green "ok": the backup worked, but the user must know Free
                // isn't enough for this save.
                if let Some(t) = &o.trimmed {
                    let _ = events_tx
                        .send(AgentEvent::BackupTrimmed {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            label: save.label.clone(),
                            kept_files: t.kept_files as u64,
                            omitted_files: t.omitted_files as u64,
                            omitted_bytes: t.omitted_bytes,
                            plan: t.plan.clone(),
                            limit_bytes: t.limit_bytes,
                        })
                        .await;
                }
                // Partial for another reason: there were files whose bytes would not
                // be read and the upload carried on without them. It is counted like
                // the plan trim, after the success so amber beats green, because the
                // deal is the same: the copy is useful, but what is not inside it
                // cannot be hidden from the user.
                if let Some(first) = o.unreadable.first() {
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        count = o.unreadable.len(),
                        kept_files = o.file_count,
                        path = %first.relative_path,
                        error = %first.error,
                        "agent: snapshot uploaded without files it couldn't read"
                    );
                    let _ = events_tx
                        .send(AgentEvent::BackupFilesUnreadable {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            label: save.label.clone(),
                            count: o.unreadable.len() as u64,
                            kept_files: o.file_count as u64,
                            sample_path: first.relative_path.clone(),
                            sample_error: first.error.clone(),
                            uploaded: true,
                        })
                        .await;
                }
                // Tell the agent loop to clear has_pending and cache the new
                // signature. If the channel is full or the agent is shutting down we
                // just drop the signal; worst case we re-upload an unchanged snapshot
                // on the next GameStopped, which is a soft failure.
                let _ = done_tx.try_send(BackupDone {
                    save_id: save.save_id.clone(),
                    new_set_hash: Some(signature),
                    committed: true,
                    version_num: Some(o.snapshot.version_num),
                    landed: false,
                });
                return;
            }
            Err(e) => {
                // Fast-forward conflict (409 non_fast_forward): another device
                // advanced this save past our `base_version`. Re-pushing our
                // stale content with a backoff is exactly how a behind device
                // used to bury a sibling's version. Instead reconcile: pull the
                // remote head with the conflict-aware merge (local-newer files
                // survive, remote-newer overwrite with a backup) and then retry the
                // upload fast-forwarding from the new head. So on a newer remote
                // head, restore wins; only genuinely-newer-or-additional local
                // content goes up afterwards (a purely-stale device matches head
                // and settles). Bounded by MAX_CONFLICT_RECONCILES.
                // Only a *non-fast-forward* 409 means "you're behind, reconcile
                // first", which is the single 409 the upload path emits today
                // (`init_upload`/`cas_init`), and the server tags it in the body
                // (`code: "non_fast_forward"`). A tagged body arrives typed and
                // carries the head we have to reconcile against; the message
                // fallback is for a server too old to send the code, which
                // leaves us knowing only *that* we diverged.
                let nff = e.chain().find_map(|c| {
                    match c.downcast_ref::<crate::api::ApiError>() {
                        Some(crate::api::ApiError::NonFastForward(d)) => Some(Some(d)),
                        // Untagged: still a divergence, just a mute one.
                        Some(crate::api::ApiError::Conflict(m))
                            if m.contains("non-fast-forward") =>
                        {
                            Some(None)
                        }
                        _ => None,
                    }
                });
                if let Some(detail) = nff {
                    if conflict_reconciles >= MAX_CONFLICT_RECONCILES {
                        let chain = format!("{e:#}");
                        tracing::warn!(
                            save_id = %save.save_id,
                            game_slug = %save.game_slug,
                            conflict_reconciles,
                            error = %chain,
                            "agent: backup conflict, remote head kept moving; giving up after reconcile retries"
                        );
                        let _ = events_tx
                            .send(AgentEvent::BackupFailed {
                                save_id: save.save_id.clone(),
                                game_slug: save.game_slug.clone(),
                                error: chain,
                                will_retry: false,
                            })
                            .await;
                        return;
                    }
                    conflict_reconciles += 1;
                    // The head the server named, when it named one. It is the
                    // whole point of reading the 409's body: without it the only
                    // way to learn where the line went is to ask a second
                    // endpoint, and when that second answer disagreed (a row we
                    // couldn't resolve, a head that raced) the conflict got
                    // parked with the number sitting unread in the rejection.
                    let server_head = detail.and_then(|d| d.head());
                    tracing::info!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        base_version = ?base_version,
                        server_head = ?server_head,
                        cloud_save_id = ?detail.and_then(|d| d.canonical_id_for(&save.save_id)),
                        conflict_reconciles,
                        "agent: backup rejected (non-fast-forward), reconciling remote head before retry"
                    );
                    let retention =
                        Duration::from_secs(u64::from(conflict_retention_days) * 86_400);
                    // Pass our stale `base_version` as known_version (so the
                    // version-gate won't trip, since remote is strictly ahead) and
                    // `None` cached_latest / shared_manifest so we fetch the
                    // authoritative head rather than trust a cache that may
                    // itself be stale.
                    match run_auto_restore(
                        &api,
                        &save,
                        conflict_root.as_deref(),
                        retention,
                        base_version,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(AutoRestorePull::Merged(outcome)) => {
                            let touched = outcome.files_restored + outcome.conflicts_backed_up;
                            if touched > 0 {
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
                                // The merge wrote into the live folder, so re-arm
                                // the watcher so the slot tracks the new state.
                                let _ = cmd_tx
                                    .send(AgentCommand::RearmWatcher(save.save_id.clone()))
                                    .await;
                            }
                            if !outcome.local_diverged {
                                // The merged tree equals the head we just pulled:
                                // re-uploading would only mint head+1 with identical
                                // bytes (and fan a no-op realtime push out to every
                                // other device). Settle instead. In the inverted model
                                // we signal ONE termination: a non-committed
                                // `BackupDone` that CARRIES the settled version
                                // (`version_num`) and the post-merge signature. The
                                // shell (the `done_rx` branch) treats it like the
                                // 409-settle: it advances `known_version`, adopts the
                                // fingerprint, stamps `last_restore_at` (the merge
                                // wrote like a restore) and clears has_pending,
                                // without crossing a restore `OpResult` with a backup
                                // `in_flight`.
                                tracing::info!(
                                    save_id = %save.save_id,
                                    game_slug = %save.game_slug,
                                    version_num = outcome.version_num,
                                    "agent: backup conflict reconciled to head with no local divergence, settled without re-upload"
                                );
                                let _ = done_tx
                                    .send(BackupDone {
                                        save_id: save.save_id.clone(),
                                        new_set_hash: outcome.disk_set_hash.clone(),
                                        committed: false,
                                        version_num: Some(outcome.version_num),
                                        landed: false,
                                    })
                                    .await;
                                return;
                            }
                            // Local content survived the merge that head lacks, so
                            // fast-forward from the head we just reconciled to and
                            // retry so genuinely new local data goes up.
                            // `known_version` will advance in the commit's final
                            // `BackupDone`; no intermediate `AutoRestoreFinished` is
                            // needed (the gate arms on completion). `last_set_hash` is
                            // left stale so the retry sees the divergence and
                            // uploads.
                            base_version = Some(outcome.version_num);
                            continue;
                        }
                        // The reconcile pulled nothing because this folder
                        // already holds the server's head. That is not a dead
                        // end: it is the one case where fast-forwarding is
                        // provably safe: our tree contains everything the head
                        // has, so pushing head+1 buries nobody, and the version
                        // we descend from stays in history either way.
                        //
                        // It needs the server's own number to do it. Asking the
                        // manifest a second time is what used to fail here: on a
                        // save whose local id isn't the cloud's, the lookup
                        // missed and this arm read it as "nothing to pull" and
                        // parked a conflict that could never clear itself.
                        // Both answers have to name the same version. They come
                        // from two reads of the server a moment apart, and a
                        // manifest that lags (or leads) the rejection describes a
                        // head this folder was never checked against, so rebasing
                        // onto that is how you push over content you never saw.
                        Ok(AutoRestorePull::AlreadyAtHead { version_num })
                            if server_head == Some(version_num) =>
                        {
                            tracing::info!(
                                save_id = %save.save_id,
                                game_slug = %save.game_slug,
                                head = version_num,
                                "agent: backup conflict: the local tree already holds head; fast-forwarding the base and retrying"
                            );
                            base_version = Some(version_num);
                            continue;
                        }
                        // The server named head 0: the save it is holding for
                        // us has no versions at all. Nobody advanced past us;
                        // the history our cursor descends from is gone (the row
                        // was deleted while this folder kept its number, e.g. a
                        // game un-archived and dropped). Descending from 0 buries
                        // nothing, because there is nothing there, and it is the
                        // only base the server will accept from here: our own
                        // number can never come back down on its own, so without
                        // this the save retries into its conflict budget and
                        // parks forever. Servers old enough not to send a head
                        // fall through to the arm below.
                        Ok(AutoRestorePull::AlreadyAtHead { .. })
                        | Ok(AutoRestorePull::NothingRemote)
                            if server_head == Some(0) =>
                        {
                            tracing::warn!(
                                save_id = %save.save_id,
                                game_slug = %save.game_slug,
                                base_version = ?base_version,
                                "agent: backup conflict: the server has no history for this save; restarting from version 1"
                            );
                            base_version = Some(0);
                            continue;
                        }
                        Ok(AutoRestorePull::AlreadyAtHead { .. })
                        | Ok(AutoRestorePull::NothingRemote) => {
                            // 409 said we're behind, yet the reconcile found
                            // nothing newer to pull and the server didn't name a
                            // head we can rebase onto (an old server, a purged
                            // remote, a row we can't resolve). We can't pick a
                            // safe new base, so surface the conflict rather than
                            // risk pushing over content we never saw.
                            let chain = format!("{e:#}");
                            tracing::warn!(
                                save_id = %save.save_id,
                                error = %chain,
                                "agent: backup conflict but reconcile found nothing to pull; surfacing"
                            );
                            let _ = events_tx
                                .send(AgentEvent::BackupFailed {
                                    save_id: save.save_id.clone(),
                                    game_slug: save.game_slug.clone(),
                                    error: chain.clone(),
                                    will_retry: false,
                                })
                                .await;
                            // Hand the retry back to the loop: it clears `in_flight`
                            // and restores `next_backup_at` (keeping has_pending,
                            // since the local changes never reached a version).
                            // Without this the op would stay "in flight" forever.
                            //
                            // Down its own lane rather than an ordinary failure's:
                            // this is not a fault time cures, so the reducer escalates
                            // it and eventually stops. The `RetryBackupAfterFailure`
                            // that used to be here restored the attempt every ten
                            // minutes with no counter at all, right under a comment
                            // that said it was avoiding the loop.
                            let _ = cmd_tx
                                .send(AgentCommand::ParkBackupConflict {
                                    id: save.save_id.clone(),
                                    error: chain,
                                })
                                .await;
                            return;
                        }
                        Err(re) => {
                            let chain = format!("{re:#}");
                            tracing::warn!(
                                save_id = %save.save_id,
                                error = %chain,
                                "agent: backup conflict; reconcile failed, surfacing"
                            );
                            let _ = events_tx
                                .send(AgentEvent::BackupFailed {
                                    save_id: save.save_id.clone(),
                                    game_slug: save.game_slug.clone(),
                                    error: chain,
                                    will_retry: false,
                                })
                                .await;
                            let _ = cmd_tx
                                .send(AgentCommand::RetryBackupAfterFailure(save.save_id.clone()))
                                .await;
                            return;
                        }
                    }
                }
                // An impossible root (a whole profile, a complete Proton prefix): not
                // a transient failure, and retrying does not fix it, so it settles
                // without marking red or re-arming the backoff. It is shouted with the
                // path and the reason in front, which is the only thing that lets the
                // user understand why their game is not uploading: the structural
                // guard already existed but only ran on adding, and a row poisoned
                // before that never went through it again. Reported aug-2026 (a Steam
                // Deck uploading the `pfx`).
                if let Some(unsafe_src) = e
                    .chain()
                    .find_map(|c| c.downcast_ref::<crate::backup::UnsafeSource>())
                {
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        path = %unsafe_src.path.display(),
                        reason = %unsafe_src.reason,
                        "agent: refusing to back up this save, the tracked folder can't be a \
                         game's save folder; re-point it at the folder inside"
                    );
                    crate::telemetry::rejected_root(
                        &save.game_slug,
                        &unsafe_src.path,
                        &unsafe_src.reason,
                    );
                    let _ = done_tx.try_send(BackupDone {
                        save_id: save.save_id.clone(),
                        new_set_hash: None,
                        committed: false,
                        version_num: None,
                        landed: false,
                    });
                    return;
                }
                // Empty source (no regular files to upload): not a failure.
                // Pushing an empty snapshot would clobber the last good server
                // copy, so we skip exactly like the up-front empty-folder guard.
                // Reached when the folder holds only subdirs / no files (e.g. an
                // empty `Repo/saves`). Clear has_pending so a later write isn't
                // blocked, and settle without a red "falló".
                if e.chain().any(|c| c.is::<crate::backup::EmptySource>()) {
                    // It has never uploaded anything AND it is empty: almost always a
                    // wrongly detected path (the native folder tracked while the game
                    // runs under Proton, the container instead of its `remote/`, a
                    // phase-4 guess). It is said loudly, with the path in front, rather
                    // than left in an INFO nobody reads. If it had uploaded before,
                    // becoming empty is a legitimate state change and the user is not
                    // bothered.
                    let likely_wrong_path = save.known_version.is_none();
                    if likely_wrong_path {
                        tracing::warn!(
                            save_id = %save.save_id,
                            game_slug = %save.game_slug,
                            path = %save.local_path.display(),
                            "agent: nothing to back up and this save has never had a snapshot: \
                             the tracked folder is probably not where the game saves"
                        );
                        // The warning above is for the human opening their machine's
                        // log; this one is the one that can be counted.
                        crate::telemetry::no_snapshots(&save.game_slug, &save.local_path);
                    } else {
                        tracing::info!(
                            save_id = %save.save_id,
                            game_slug = %save.game_slug,
                            "agent: backup skipped, the source has no files to upload"
                        );
                    }
                    let _ = done_tx.try_send(BackupDone {
                        save_id: save.save_id.clone(),
                        new_set_hash: None,
                        committed: false,
                        version_num: None,
                        landed: false,
                    });
                    let _ = events_tx
                        .send(AgentEvent::BackupSkippedEmpty {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            likely_wrong_path,
                        })
                        .await;
                    return;
                }
                // Not one file in the folder would be read, so there is no snapshot to
                // upload: an empty version would delete the last good copy in the
                // cloud. `BackupDone` is NOT sent (the local changes are still
                // unversioned and clearing `has_pending` would let a restore walk over
                // them) and it re-arms on the long backoff, which is exactly what is
                // needed: the known trigger, a stalled on-demand file provider, heals
                // itself as soon as the provider starts, and then the next pass
                // uploads. What no longer happens is it doing so in silence: the event
                // leaves a persistent warning on the game's card. It is copied out of
                // the chain before awaiting anything: an `anyhow::Chain` is not `Send`
                // and this future goes to `tokio::spawn`.
                let unreadable_src = e
                    .chain()
                    .find_map(|c| c.downcast_ref::<crate::backup::UnreadableSource>())
                    .map(|src| (src.path.clone(), src.count, src.first.clone()));
                if let Some((path, count, first)) = unreadable_src {
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        path = %path.display(),
                        count,
                        error = %first,
                        "agent: nothing backed up, not one file in the save folder could be read"
                    );
                    let _ = events_tx
                        .send(AgentEvent::BackupFilesUnreadable {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            label: save.label.clone(),
                            count: count as u64,
                            kept_files: 0,
                            sample_path: path.display().to_string(),
                            sample_error: first,
                            uploaded: false,
                        })
                        .await;
                    let _ = cmd_tx
                        .send(AgentCommand::RetryBackupAfterFailure(save.save_id.clone()))
                        .await;
                    return;
                }
                // Archived game (403 `save_archived`): the user parked this save
                // in the server-side "caja negra". Re-uploading would revive its
                // frozen blobs and undo the quota it freed, so never retry:
                // settle quietly (clear has_pending, no red "falló"). The local
                // save stays put; the desktop learns the archived state from
                // `/v1/cloud/storage/games` and surfaces it there.
                let is_archived = e.chain().any(|c| {
                    matches!(
                        c.downcast_ref::<crate::api::ApiError>(),
                        Some(crate::api::ApiError::Archived)
                    )
                });
                if is_archived {
                    tracing::info!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        "agent: backup skipped, the game is archived on the server"
                    );
                    let _ = done_tx.try_send(BackupDone {
                        save_id: save.save_id.clone(),
                        new_set_hash: None,
                        committed: false,
                        version_num: None,
                        landed: false,
                    });
                    return;
                }
                // A 404 on upload: the server does not know this `save_id`. Retrying
                // will not resurrect it (`library::reconcile_with_server` repairs it
                // when the engine starts, repointing the row at the id the server has
                // now) so it settles like the other terminals. Without this cut-off, a
                // rebuilt database leaves the engine retrying every 600 s forever:
                // 1,353 failed uploads in three days in the aug-2026 case.
                let gone = e.chain().any(|c| {
                    matches!(
                        c.downcast_ref::<crate::api::ApiError>(),
                        Some(crate::api::ApiError::NotFound)
                    )
                });
                if gone {
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        "agent: backup abandoned, the server doesn't know this save; it'll be re-linked on the next engine start"
                    );
                    let _ = done_tx.try_send(BackupDone {
                        save_id: save.save_id.clone(),
                        new_set_hash: None,
                        committed: false,
                        version_num: None,
                        landed: false,
                    });
                    return;
                }
                // Per-save size cap (413 `save_too_large`): the upload can never
                // succeed as-is, so retrying just burns the budget and spams the feed.
                // Emit a dedicated, actionable event and settle (clearing has_pending)
                // so it does not re-fire until the folder actually changes: no red
                // failure, no retry loop.
                let too_large = e
                    .chain()
                    .find_map(|c| c.downcast_ref::<crate::api::ApiError>())
                    .and_then(|api_err| match api_err {
                        crate::api::ApiError::TooLarge(d) => Some(d.clone()),
                        _ => None,
                    });
                if let Some(detail) = too_large {
                    // A 413 can come from three places and each is fixed somewhere
                    // else: the plan's cap on Cloud, a self-hosted server's
                    // `max_snapshot_size_mb`, or a proxy in front that is not even
                    // Hoard. `kind()` decides and `human()` words it; claiming "plan
                    // cap" with nothing behind it is what sent a self-hoster looking
                    // in the wrong place (aug-2026).
                    let kind = detail.kind();
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        kind = ?kind,
                        plan = %detail.plan,
                        limit_bytes = detail.limit_bytes,
                        actual_bytes = detail.actual_bytes,
                        received_bytes = detail.received_bytes,
                        detail = %detail.human(),
                        "agent: backup rejected, the upload was refused as too large"
                    );
                    let _ = done_tx.try_send(BackupDone {
                        save_id: save.save_id.clone(),
                        new_set_hash: None,
                        committed: false,
                        version_num: None,
                        landed: false,
                    });
                    let _ = events_tx
                        .send(AgentEvent::BackupTooLarge {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            label: save.label.clone(),
                            kind,
                            plan: detail.plan.clone(),
                            limit_bytes: detail.limit_bytes,
                            actual_bytes: detail.actual_bytes,
                            received_bytes: detail.received_bytes,
                        })
                        .await;
                    return;
                }
                // Account out of storage (402 `quota_exceeded`): nothing about
                // this save is wrong and nothing will change by retrying: the
                // next attempt, and every other save's, hits the same wall until
                // a human frees space or upgrades. Park it for an hour and say
                // so once, account-wide. Deliberately **no** `BackupDone`: the
                // bytes are still only on disk, so `has_pending` must survive
                // (same contract as the exhausted-retries path below).
                let quota = e
                    .chain()
                    .find_map(|c| c.downcast_ref::<crate::api::ApiError>())
                    .and_then(|api_err| match api_err {
                        crate::api::ApiError::QuotaExceeded(d) => Some(d.clone()),
                        _ => None,
                    });
                if let Some(detail) = quota {
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        plan = %detail.plan,
                        used_bytes = detail.used_bytes,
                        limit_bytes = detail.limit_bytes,
                        over_bytes = detail.over_bytes(),
                        "agent: backup parked, the cloud account is out of storage"
                    );
                    let _ = events_tx
                        .send(AgentEvent::BackupQuotaFull {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            label: save.label.clone(),
                            plan: detail.plan.clone(),
                            used_bytes: detail.used_bytes,
                            limit_bytes: detail.limit_bytes,
                        })
                        .await;
                    let _ = cmd_tx
                        .send(AgentCommand::ParkBackupQuotaFull(save.save_id.clone()))
                        .await;
                    return;
                }
                // Bandwidth throttle (429): wait the server's exact
                // window-slide time and retry without consuming the
                // network-flake budget. Kept out of the failure feed path,
                // we emit an amber "en espera" entry instead.
                //
                // Only the blob PUTs get retried in place (`backup::put_blob_paced`);
                // a 429 that reaches here came from a single request, the init or
                // the commit, so waiting and re-running the whole backup is right
                // for either kind. The kind still travels because the log line has
                // to say which one it was: "bandwidth limit" on what was really the
                // server asking this machine to slow down is what sent a whole
                // investigation looking at plan quotas.
                let throttle = e
                    .chain()
                    .find_map(|c| c.downcast_ref::<crate::api::ApiError>())
                    .and_then(|api_err| match api_err {
                        crate::api::ApiError::RateLimited {
                            kind,
                            retry_after_seconds,
                            body,
                        } => Some((*kind, *retry_after_seconds, body.clone())),
                        _ => None,
                    });
                if let Some((kind, retry_after, body)) = throttle {
                    // A budget 429 is the server saying this operation does not
                    // fit right now: the bandwidth window, the storage quota,
                    // or the loop brake. Sitting on it inside the backup task is
                    // the wrong shape twice over: it holds the task open for
                    // whatever the server asked (an hour, for a full account),
                    // and the cap that stopped it doing so silently shortened
                    // the wait to five minutes, which is how one account
                    // collected ~170 refusals an hour for four days against a
                    // brake that had already told it to come back in one.
                    //
                    // So we don't wait here at all: give up on the attempt and
                    // hand the server's own deadline to the reducer, which
                    // parks the slot and keeps `has_pending` (the bytes are
                    // still only on disk). Same contract as the 402 path.
                    if kind == crate::api::RateLimitKind::Budget {
                        // A full account gets the plain 402 for its first few
                        // refusals and this paced 429 afterwards, and the two
                        // mean the same thing to whoever has to act on it. Say
                        // the same thing, then: the quota event is what puts
                        // "free up space / go Pro" in front of them, and
                        // answering the brake with a wordless wait meant the
                        // one moment worth explaining went quiet exactly when
                        // it started repeating.
                        let quota = crate::api::paced_quota_detail(&body);
                        tracing::info!(
                            save_id = %save.save_id,
                            game_slug = %save.game_slug,
                            retry_after,
                            full = quota.is_some(),
                            "agent: backup parked, the server asked for a wait before trying again"
                        );
                        let event = match &quota {
                            Some(detail) => AgentEvent::BackupQuotaFull {
                                save_id: save.save_id.clone(),
                                game_slug: save.game_slug.clone(),
                                label: save.label.clone(),
                                plan: detail.plan.clone(),
                                used_bytes: detail.used_bytes,
                                limit_bytes: detail.limit_bytes,
                            },
                            None => AgentEvent::BackupThrottled {
                                save_id: save.save_id.clone(),
                                game_slug: save.game_slug.clone(),
                                label: save.label.clone(),
                                retry_after_secs: retry_after,
                            },
                        };
                        let _ = events_tx.send(event).await;
                        let _ = cmd_tx
                            .send(AgentCommand::ParkBackupThrottled {
                                id: save.save_id.clone(),
                                retry_after_secs: retry_after,
                            })
                            .await;
                        return;
                    }
                    if throttle_waits < MAX_THROTTLE_WAITS {
                        // Pacing only, now: *this request* arrived too fast and
                        // the wait is milliseconds to seconds. The cap stays
                        // because that number comes from a per-IP limiter or a
                        // proxy in front rather than from one of our handlers;
                        // +2s jitter avoids a thundering herd of saves all
                        // retrying on the same tick.
                        let wait = (u64::from(retry_after)).clamp(1, 300) + 2;
                        tracing::info!(
                            save_id = %save.save_id,
                            game_slug = %save.game_slug,
                            throttle_waits,
                            retry_after,
                            wait,
                            %kind,
                            "agent: backup throttled (429), waiting to retry"
                        );
                        let _ = events_tx
                            .send(AgentEvent::BackupThrottled {
                                save_id: save.save_id.clone(),
                                game_slug: save.game_slug.clone(),
                                label: save.label.clone(),
                                retry_after_secs: retry_after,
                            })
                            .await;
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        throttle_waits += 1;
                        continue;
                    }
                    // Exhausted our patience for the window, so fall through and
                    // surface it as a normal failure below.
                }
                // The storage endpoint never answered. Not a flake: the
                // connection didn't open, and it won't open on the next attempt
                // either: the retry budget just spends six connect timeouts
                // (~21 s each on Windows) before parking, then re-arms and
                // spends them again. One user's ISP stopped routing to the two
                // anycast addresses R2's S3 endpoint resolves to, and every
                // round burned four minutes to learn the same thing.
                //
                // So park on the first one, with the same no-`BackupDone`
                // contract as the exhausted-retries path below: the bytes are
                // still only on disk, `has_pending` has to survive, and the long
                // backoff is the recovery that doesn't need a new fs event.
                let unreachable =
                    e.chain()
                        .find_map(|c| match c.downcast_ref::<crate::api::ApiError>() {
                            Some(crate::api::ApiError::StorageUnreachable { host, .. }) => {
                                Some(host.clone())
                            }
                            _ => None,
                        });
                if let Some(host) = unreachable {
                    let chain = format!("{e:#}");
                    tracing::warn!(
                        save_id = %save.save_id,
                        game_slug = %save.game_slug,
                        %host,
                        error = %chain,
                        "agent: backup parked, the storage endpoint can't be reached from this machine"
                    );
                    let _ = events_tx
                        .send(AgentEvent::BackupFailed {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            error: chain,
                            will_retry: true,
                        })
                        .await;
                    let _ = cmd_tx
                        .send(AgentCommand::RetryBackupAfterFailure(save.save_id.clone()))
                        .await;
                    return;
                }
                let will_retry = attempt < max_retries;
                // `{:#}` renders the whole anyhow context chain; `.to_string()`
                // alone collapses it to the outermost label ("cloud cas init"),
                // which is what made this failure undiagnosable from the feed.
                let chain = format!("{e:#}");
                let backoff_secs = (1u64 << attempt.min(8)).min(300);
                tracing::warn!(
                    save_id = %save.save_id,
                    game_slug = %save.game_slug,
                    attempt,
                    max_retries,
                    will_retry,
                    backoff_secs = if will_retry { backoff_secs } else { 0 },
                    error = %chain,
                    "agent: backup attempt failed"
                );
                // A feed-visible failure only when the retries are exhausted;
                // intermediate attempts stay in the log, or one flaky burst paints the
                // feed with a dozen failure rows.
                if !will_retry {
                    let _ = events_tx
                        .send(AgentEvent::BackupFailed {
                            save_id: save.save_id.clone(),
                            game_slug: save.game_slug.clone(),
                            error: chain,
                            will_retry,
                        })
                        .await;
                    // Out of retries, but the slot must not be left wedged. We
                    // deliberately send no `BackupDone`: the local changes never
                    // made it to a version, so `has_pending` has to stay set or
                    // a later restore would overwrite them. That also means the
                    // slot is now vetoed from every pull *and* has nothing left
                    // that would re-fire the upload: until this returned, only
                    // a fresh fs event could break the deadlock, so a save whose
                    // game was already closed just sat there. Hand the retry
                    // back to the agent loop instead.
                    let _ = cmd_tx
                        .send(AgentCommand::RetryBackupAfterFailure(save.save_id.clone()))
                        .await;
                    return;
                }
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                attempt += 1;
            }
        }
    }
}

// `accept_correlation_signals` (the phantom-hours filter) lives in the leaf kernel,
// at `hoard_core::kernel::correlation`, and is imported above. It was already a pure
// function, so the kernel is its natural home.

/// The minimum length of an identity token for it to count in the generic match.
/// Below that (`gta`, `ori`, `ff`) it is too short and collides with any old folder
/// or process name.
use hoard_core::ids::MIN_IDENTITY_TOKEN_LEN;

/// The canonical identity token (see [`hoard_core::ids::canon_token`]). It lives in
/// the leaf kernel because `GameSlug::repair` uses it to detect degenerate slugs and
/// the two checks have to be the same one.
use hoard_core::ids::canon_token;

/// Tokens VETOED in the generic identity match: components of the user profile and
/// of install plumbing. A degenerate slug equal to one of these turns arbitrary
/// processes into a strong "you are playing" signal. The real case from jul-2026: one
/// save ended up tracked with its slug set to the Windows account name, and since the
/// username is a path component of EVERY exe under `C:\Users\<user>\...`, any app
/// in the profile fired GameStarted (and the "one game at a time" guard switched the
/// real games off as a side effect). The static list covers the common plumbing; the
/// real home's components, the username included, are added dynamically.
pub(crate) fn is_generic_identity_token(tok: &str) -> bool {
    static HOME_TOKENS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let home = HOME_TOKENS.get_or_init(|| {
        directories::UserDirs::new()
            .map(|u| {
                u.home_dir()
                    .components()
                    .filter_map(|c| match c {
                        std::path::Component::Normal(s) => s.to_str().map(canon_token),
                        _ => None,
                    })
                    .filter(|t| t.len() >= MIN_IDENTITY_TOKEN_LEN)
                    .collect()
            })
            .unwrap_or_default()
    });
    hoard_core::ids::GENERIC_IDENTITY_TOKENS.contains(&tok) || home.iter().any(|h| h == tok)
}

/// A tracked save's identity tokens, derived from data we already have (the slug
/// plus the display name), with NO curated list. They are the keys each live process
/// is compared against. Generic and profile tokens are vetoed
/// ([`is_generic_identity_token`]): a game named that badly loses the token match
/// (it still has the other signals) rather than matching everything.
fn game_identity_tokens(slug: &str, display: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::with_capacity(2);
    for raw in [slug, display] {
        let t = canon_token(raw);
        if t.len() >= MIN_IDENTITY_TOKEN_LEN && !is_generic_identity_token(&t) && !v.contains(&t) {
            v.push(t);
        }
    }
    v
}

/// A live process's identity candidates, list-free and cross-platform: the
/// executable's basename (`.../Stellaris/stellaris` gives `stellaris`), the process
/// name, and each component of the executable's PATH, because the install folder
/// almost always carries the game's name (`steamapps/common/The Witcher 3 Wild
/// Hunt/...`, `GOG Games/...`, macOS's `.app`). With that, a game whose exe is
/// abbreviated (`witcher3.exe`) matches through its folder anyway. The comparison is
/// exact equality of canonical tokens, so a generic component (`common`, `bin`,
/// `x64`) does not collide with a real slug.
fn process_identity_candidates(name: &str, exe: Option<&Path>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let push = |s: &str, v: &mut Vec<String>| {
        let t = canon_token(s);
        if t.len() >= MIN_IDENTITY_TOKEN_LEN && !is_generic_identity_token(&t) && !v.contains(&t) {
            v.push(t);
        }
    };
    push(name, &mut v);
    if let Some(exe) = exe {
        if let Some(base) = exe.file_stem().and_then(|s| s.to_str()) {
            push(base, &mut v);
        }
        for comp in exe.components() {
            if let std::path::Component::Normal(c) = comp {
                if let Some(s) = c.to_str() {
                    push(s, &mut v);
                }
            }
        }
    }
    v
}

/// The paths (fds plus cwd) the `pid` process has open that fall INSIDE one of
/// `folders`. A launch signal agnostic of the installer and of the exe's name: if a
/// game process has a file from the save folder open (or its cwd is there), that
/// process is that save's game, with no catalogue, no Steam and no waiting for it to
/// write (having it open is enough, when listing saves in the load menu or mapping
/// the save into memory). It returns the matched `save_id`s.
///
/// Linux and SteamOS only today (through `/proc/<pid>/fd` and `/proc/<pid>/cwd`,
/// which need no privileges for our own processes). Windows and macOS return empty
/// for now; their equivalent (enumerating handles, `proc_pidfdinfo`) is still to
/// come, and there detection leans on name, folder and correlation.
#[cfg(target_os = "linux")]
fn open_paths_matching(pid: Pid, folders: &[(&str, &Path)]) -> Vec<String> {
    let mut hits: Vec<String> = Vec::new();
    if folders.is_empty() {
        return hits;
    }
    let check = |p: &Path, hits: &mut Vec<String>| {
        for (id, folder) in folders {
            if p.starts_with(folder) && !hits.iter().any(|h| h == id) {
                hits.push((*id).to_string());
            }
        }
    };
    let base = std::path::PathBuf::from(format!("/proc/{pid}"));
    if let Ok(cwd) = std::fs::read_link(base.join("cwd")) {
        check(&cwd, &mut hits);
    }
    if let Ok(entries) = std::fs::read_dir(base.join("fd")) {
        for entry in entries.flatten() {
            if hits.len() == folders.len() {
                break; // ya casaron todos; no sigas leyendo fds
            }
            if let Ok(target) = std::fs::read_link(entry.path()) {
                check(&target, &mut hits);
            }
        }
    }
    hits
}

#[cfg(not(target_os = "linux"))]
fn open_paths_matching(_pid: Pid, _folders: &[(&str, &Path)]) -> Vec<String> {
    Vec::new()
}

/// Of all the saves declaring the same executable, is THIS the one being played?
///
/// The process name cannot answer: ten titles from an emulated console share it.
/// What does distinguish them is which one receives the saves, so the proof is a
/// recent write to its own folder. With no writes nothing is claimed, which is
/// right: losing a session's start only costs a few minutes of counted hours, while
/// taking it as true for all of them would invent a whole session in the other nine
/// saves.
fn shared_process_is_corroborated(
    last_fs_event_at: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> bool {
    last_fs_event_at.is_some_and(|t| now - t <= SHARED_PROCESS_ACTIVITY)
}

/// One sweep of the process table. It emits transitions and schedules a post-game
/// backup when a watched game stops running.
///
/// Since 1.4 this no longer touches the fs watcher: the watcher is armed in
/// `handle_add` and lives for the slot's lifetime. `process_poll` is pure UI signal
/// (the Dashboard pill, the "the game just closed, so flush" hint).
///
/// It returns whether any tracked game is currently running, so the caller can
/// throttle the poll cadence (fast while a game is up, slow when idle).
#[allow(clippy::too_many_arguments)]
fn process_poll(
    sys: &mut System,
    slots: &mut HashMap<String, SaveSlot>,
    events_tx: &mpsc::Sender<AgentEvent>,
    config: &AgentConfig,
    playtime: &mut crate::playtime::PlaytimeStore,
    playtime_path: Option<&std::path::Path>,
    reported_heavy: &mut HashSet<Pid>,
    // Mutable: stop transitions pass phantom-session strikes to the correlation
    // observations (and discard them on reaching the cap).
    corr_store: &mut crate::correlation::CorrelationStore,
    corr_path: Option<&std::path::Path>,
    steam_index: &crate::playtime_index::SteamPlaytimeIndex,
    prev_pids: &mut HashSet<Pid>,
    corr_running: &mut HashMap<String, (Pid, u64)>,
) -> bool {
    // Slice 2b (ADR 0021 C.1): `process_poll` is the world's sampler. Process
    // detection, `is_running` (with its 6 s sticky), the GameStarted and GameStopped
    // events, playtime, the heavy, correlation and probe passes. It NO LONGER takes
    // sync decisions (barrier, final flush, deferred pull): the reducer emits those in
    // the `reconcile_all` that follows this poll. That is why it stopped receiving
    // `api`, `done_tx`, `cmd_tx` and `latest_versions`.
    // Refresh every process. The `true` flag asks sysinfo to remove entries for
    // processes that have exited since the last refresh, which is exactly what we need
    // to detect "game stopped".
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, proc_refresh_kind());

    // Build a set of "currently running" save_ids. Two matchers cooperate:
    // process-name match (manifest-driven, storefront-agnostic) takes
    // precedence; install-dir match is the legacy v0.2 fallback for
    // saves registered without a manifest.
    //
    // Single pass over the process table: invert the slots into a name→ids
    // index up front so the scan is O(procs + slots) instead of O(procs ×
    // slots): the old nested loop re-scanned every process for every slot and
    // rebuilt a HashSet per slot per tick, which got worse now that playtime-
    // only games add up to ~16 extra slots.
    let mut name_index: HashMap<String, Vec<&str>> = HashMap::new();
    // Saves whose process names are shared with other saves (an emulated console
    // split into one folder per game). They go into a separate index because the name
    // does NOT identify them: ten titles from the same machine list the same
    // executable, and putting them in `name_index` would mark all ten as "playing" the
    // moment the emulator starts. Only candidates are collected here; they get
    // corroborated below against EACH folder's activity.
    let mut shared_name_index: HashMap<String, Vec<&str>> = HashMap::new();
    // Shared-process saves with recent writes in their folder. It is the only evidence
    // of "this one of the ten is the one being played" that works on all three
    // systems: open handles can only be read on Linux (`/proc`). The cost: an emulated
    // title's hours start counting on its first save rather than when the emulator
    // starts. Losing that start is preferred to inventing hours in the other nine
    // saves.
    let mut shared_fs_active: HashSet<&str> = HashSet::new();
    // The generic identity index (slug or name to save_ids), list-free and
    // cross-platform. It is the route that fixes games with no configured processes
    // (Stellaris, Victoria): they used to match only through cold correlation or
    // `steam_install_dir`, so the first session fired neither "it started" nor
    // auto-restore even when the save was detected. Now they match through their own
    // name or folder without depending on Steam or on a curated list.
    let mut token_index: HashMap<String, Vec<&str>> = HashMap::new();
    // The tracked save folders, `(save_id, local_path)`. Used by OPEN-HANDLE
    // detection: a game process with a file open inside one of these folders IS that
    // save's game, agnostic of the installer and of the exe's name (it resolves
    // code-named exes with no catalogue and no Steam). `track_only` ones are skipped
    // (they have no real save).
    let mut save_folders: Vec<(&str, &Path)> = Vec::new();
    let mut dir_slots: Vec<(&str, &Path)> = Vec::new();
    // Candidate correlation signals `(proc_name_lower, save_id, game_slug)`, collected
    // separately so the ambiguous ones can be vetoed BEFORE they count hours (see
    // `accept_correlation_signals`).
    let mut corr_candidates: Vec<(String, &str, &str)> = Vec::new();
    for slot in slots.values() {
        // Generic identity: it counts for EVERY slot (with or without configured
        // processes, `track_only` included). It is additive over a HashSet, so
        // overlapping with `name_index` is harmless.
        //
        // Except the shared-process ones: their identity is exactly what does NOT tell
        // one save from another, and letting them in here would reopen "the emulator
        // starts and all ten titles run" through this door.
        for tok in if slot.save.shared_processes {
            Vec::new()
        } else {
            game_identity_tokens(&slot.save.game_slug, &slot.save.display_name)
        } {
            token_index
                .entry(tok)
                .or_default()
                .push(slot.save.save_id.as_str());
        }
        if !slot.save.track_only {
            save_folders.push((slot.save.save_id.as_str(), slot.save.local_path.as_path()));
        }
        if slot.save.processes.is_empty() {
            // The correlation-learned launch signal (ADR 0020, storefront- and
            // game-agnostic): if Hoard has already seen some GAME process writing into
            // this save's folder, that process is the "you are playing" signal.
            // Without it, a game off the list (say one under Proton whose exe does not
            // fall under `steam_install_dir`) never enters `running` and accrues no
            // hours. BUT folder-to-process attribution is noisy: if something in the
            // background rewrites ANOTHER game's save folder while this one runs, that
            // folder ends up correlated with this process. For detection that is
            // harmless (it can be reviewed); for PLAYTIME it would accrue phantom
            // hours. So only candidates are collected here and filtered below.
            if let Some(obs) = corr_store.signal_for(&slot.save.local_path) {
                // Re-validate the observation against the CURRENT rules and demand an
                // exe on disk: it guards against junk entries recorded by earlier
                // versions with looser filters (a kernel worker with no exe, living
                // 24/7, would accrue hours forever). As soon as the correlation is
                // re-recorded during a real session it is corrected and trusted.
                if obs.exe.is_some()
                    && crate::correlation::is_game_like(&obs.process_name, obs.exe.as_deref())
                {
                    corr_candidates.push((
                        obs.process_name.to_lowercase(),
                        slot.save.save_id.as_str(),
                        slot.save.game_slug.as_str(),
                    ));
                }
            }
            // Legacy fallback only when no process names are configured.
            if let Some(dir) = slot.save.steam_install_dir.as_deref() {
                dir_slots.push((slot.save.save_id.as_str(), dir));
            }
            continue;
        }
        let index = if slot.save.shared_processes {
            if shared_process_is_corroborated(slot.last_fs_event_at, OffsetDateTime::now_utc()) {
                shared_fs_active.insert(slot.save.save_id.as_str());
            }
            &mut shared_name_index
        } else {
            &mut name_index
        };
        for p in &slot.save.processes {
            index
                .entry(p.to_lowercase())
                .or_default()
                .push(slot.save.save_id.as_str());
        }
    }

    // Correlation signals are NOT mixed with the manifest's process names: they go
    // into a separate index because a correlation match only counts as "playing" when
    // the process has real CPU on this tick (see `CORRELATION_MIN_CPU_PCT`). Only the
    // ones surviving the phantom-hours filter are injected (the configured process
    // names belong to games with a manifest).
    let configured: HashSet<String> = name_index.keys().cloned().collect();
    let mut corr_index: HashMap<String, Vec<&str>> = HashMap::new();
    for (pname, save_id) in accept_correlation_signals(&corr_candidates, &configured) {
        corr_index.entry(pname).or_default().push(save_id);
    }

    // The three indexes above all answer "which save is this process?" by
    // process NAME, and a save only appears in them if something ever wrote its
    // `processes` list. Plenty never did: they were tracked by folder and the
    // list is empty. That's fine for "is this game running", since the identity and
    // correlation paths cover it, but not for the heavy-process warning, which
    // reads an empty list as "not tracked". Stellaris has been tracked for
    // months with `processes: []`, so every launch asked for a detection scan
    // for a game that was already in. Hence a second check by identity: the
    // slug of the title the manifest gives this executable, against the slugs
    // already tracked.
    let tracked_slugs: HashSet<&str> = slots
        .values()
        .map(|s| s.save.game_slug.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    // The WEAK signal is a PID transition, not presence plus CPU. `first_tick` (there
    // was no previous snapshot) marks the agent's start: on it we do NOT fire "it
    // started" from correlation, adopting whatever was running as pre-existing, so a
    // resident alive since boot is not confused with a launch. `cur_pids` becomes the
    // next tick's snapshot.
    let first_tick = prev_pids.is_empty();
    let mut cur_pids: HashSet<Pid> = HashSet::with_capacity(sys.processes().len());

    // STRONG signals: the process carries the game's name or identity, runs from its
    // install folder, or has one of its save's files open. All of them demand that the
    // game's real executable EXIST right now and that the process still be ALIVE
    // (`is_defunct`): the exe existing was not enough, since a Proton game that dies
    // badly leaves a zombie with the same name and exe, and that kept the game
    // "running" indefinitely.
    let mut running: HashSet<String> = HashSet::new();
    // WEAK signals (folder-to-process correlation): they do not demand the game's real
    // exe, only that "some game process" touched its folder at some point. A
    // misattributed background process can keep them alive indefinitely (one case ran
    // 35 minutes without closing). They are resolved separately so the "one game at a
    // time" guard below can be applied to them.
    let mut weak_running: HashSet<String> = HashSet::new();
    // Playtime "only what you play": slugs of Steam games that are running but tracked
    // by no slot (neither a real save nor the catalogue). They count for the Wrapped
    // anyway; see `steam_index` and the attribution below.
    let mut steam_running: HashSet<String> = HashSet::new();
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_lowercase();
        // A defunct process keeps its name, exe and `start_time`, so it would match
        // the STRONG signals just like a live one and keep the slot "running" forever
        // (see `is_defunct`). It cannot be writing a save, so it stays out of all four.
        // The WEAK one does not want it either: although it only STARTS on a PID being
        // born, its "same PID still alive" arm accepted the zombie tick after tick (a
        // Proton game that dies badly leaves the zombie with the same `rpid` and
        // `rst`), and the slot never left "running" until a reboot, since a zombie
        // cannot be killed and a force quit generates no transition. See the PoP 2008
        // incident, jul-2026.
        let defunct = is_defunct(proc.status());
        // Name match: works on every storefront on Windows, and on
        // Proton/Wine where the wineprefix process keeps the .exe name.
        if !defunct && !name_index.is_empty() {
            if let Some(ids) = name_index.get(&name) {
                running.extend(ids.iter().map(|id| id.to_string()));
            }
        }
        // A shared process: the name alone does not choose between the saves listing
        // it, so only the one also receiving writes counts. Open handles, where they
        // can be read, resolve the same thing a little earlier and come in through
        // their own branch below.
        if !defunct && !shared_name_index.is_empty() {
            if let Some(ids) = shared_name_index.get(&name) {
                running.extend(
                    ids.iter()
                        .filter(|id| shared_fs_active.contains(*id))
                        .map(|id| id.to_string()),
                );
            }
        }
        // The generic identity match (list-free): the process carries the game's name
        // or runs from its install folder. With no CPU gate, since exact equality with
        // the game's slug or name is a strong signal on its own, and that way a paused
        // game (a Paradox menu at 0% CPU) still counts as running. `is_game_like`
        // discards system processes and launchers.
        if !defunct
            && !token_index.is_empty()
            && crate::correlation::is_game_like(&name, proc.exe())
        {
            for cand in process_identity_candidates(&name, proc.exe()) {
                if let Some(ids) = token_index.get(&cand) {
                    running.extend(ids.iter().map(|id| id.to_string()));
                }
            }
        }
        // The OPEN-HANDLE match (agnostic of installer and exe name): if a game process
        // has a file from the save folder open, it is that save's game. It resolves the
        // code-named and abbreviated exes that neither the name nor the folder gives
        // away, with no catalogue and no Steam. Only for processes that look like
        // games, to bound the cost of reading `/proc/<pid>/fd`.
        if !defunct
            && !save_folders.is_empty()
            && crate::correlation::is_game_like(&name, proc.exe())
        {
            for id in open_paths_matching(*pid, &save_folders) {
                running.insert(id);
            }
        }
        cur_pids.insert(*pid);
        // The correlation match (the WEAK signal) by PID TRANSITION: the slot runs
        // while the PID that started it lives, and it only starts when a PID matching
        // its name is BORN this tick (it was not there on the previous one). With no
        // CPU gate: a resident correlated by mistake never "appears", so no CPU spike
        // can fire it. It goes to `weak_running`; the "one game at a time" guard still
        // discards it when another game runs on a strong signal (see below).
        if !defunct && !corr_index.is_empty() {
            if let Some(ids) = corr_index.get(&name) {
                let st = proc.start_time();
                for id in ids {
                    match corr_running.get(*id) {
                        // It is the PID that already kept this slot alive and still
                        // is.
                        Some((rpid, rst)) if *rpid == *pid && *rst == st => {
                            weak_running.insert(id.to_string());
                        }
                        // A different PID (or a stopped slot): it only counts when it
                        // has just been born. On the first tick after the agent starts
                        // nothing is "new"; a game already open is detected by a strong
                        // signal anyway, and correlation picks it up on relaunch.
                        _ => {
                            if !first_tick && !prev_pids.contains(pid) {
                                weak_running.insert(id.to_string());
                                corr_running.insert(id.to_string(), (*pid, st));
                            }
                        }
                    }
                }
            }
        }
        // Legacy install-dir fallback for slots without process names.
        if !defunct && !dir_slots.is_empty() {
            if let Some(exe) = proc.exe() {
                for (id, dir) in &dir_slots {
                    if exe.starts_with(dir) {
                        running.insert(id.to_string());
                    }
                }
            }
        }

        // Playtime "only what you play" (the recap, Steam): it counts hours for Steam
        // games even when they are not tracked. It is ONLY for the Wrapped, not for
        // detecting launches; "running" detection is installer-agnostic (name, folder,
        // open handles and correlation) and never touches Steam. We demand real CPU,
        // not being a thread, and looking like a game, so background tools under
        // `steamapps/common` are not added up.
        if !steam_index.is_empty()
            && proc.thread_kind().is_none()
            && proc.cpu_usage() >= CORRELATION_MIN_CPU_PCT
        {
            if let Some(exe) = proc.exe() {
                if crate::correlation::is_game_like(&name, Some(exe)) {
                    if let Some(slug) = steam_index.slug_for_exe(exe) {
                        steam_running.insert(slug.to_string());
                    }
                }
            }
        }

        // Immediate-scan trigger: a process burning real CPU that looks like a
        // game but matches no tracked save's process name is probably a
        // just-launched, not-yet-tracked game. Flag it once (deduped by PID) so
        // the desktop fires a detection scan now instead of waiting out the
        // 10-min timer. Cheap: `cpu_usage` and `name` come from the same
        // `/proc/<pid>/stat` already parsed above. Tracked games are skipped
        // via `name_index` (their launch is already handled by the barrier).
        if proc.cpu_usage() >= HEAVY_PROCESS_CPU_PCT
            && !name_index.contains_key(&name)
            && !shared_name_index.contains_key(&name)
            && !corr_index.contains_key(&name)
            && !reported_heavy.contains(pid)
            && crate::correlation::is_game_like(&name, proc.exe())
        {
            // The notice shows the TITLE when the manifest recognises the executable
            // (18k games declare it in `launch:`) and only falls back to the raw
            // process name otherwise. "Possible game detected: Hollow Knight" rather
            // than "hollow_knight.x86_64". The real name is still on the log line next
            // to it for diagnostics.
            let raw = proc.name().to_string_lossy().into_owned();
            let title = hoard_manifest::ludusavi::title_for_exe(&raw)
                .or_else(|| {
                    proc.exe()
                        .and_then(|e| e.file_name())
                        .and_then(|e| e.to_str())
                        .and_then(hoard_manifest::ludusavi::title_for_exe)
                })
                .map(str::to_string);
            // Last check before bothering anyone: if the title the manifest
            // puts on this executable is already one of the tracked slugs, the
            // game is in and all it's missing is a process list. Scanning for
            // it finds nothing new, and the desktop repeats it on every launch,
            // forever.
            if let Some(slug) = title.as_deref().map(hoard_core::ids::slugify) {
                if tracked_slugs.contains(slug.as_str()) {
                    tracing::debug!(
                        process = %name,
                        %slug,
                        "agent: heavy process belongs to a save we already track; no scan needed"
                    );
                    reported_heavy.insert(*pid);
                    continue;
                }
            }
            tracing::info!(
                process = %name,
                title = %title.as_deref().unwrap_or("-"),
                cpu = proc.cpu_usage(),
                "agent: heavy untracked game-like process; requesting detection scan"
            );
            let _ = events_tx.try_send(AgentEvent::HeavyProcessDetected {
                name: title.unwrap_or(raw),
            });
            reported_heavy.insert(*pid);
        }
    }
    // Forget PIDs that have exited so a relaunch of the same game re-triggers.
    reported_heavy.retain(|pid| sys.processes().contains_key(pid));
    // Release the weak attribution of slots whose PID no longer lives, and store the
    // PID snapshot so the next tick knows which were born.
    corr_running.retain(|_, (pid, _)| cur_pids.contains(pid));
    *prev_pids = cur_pids;

    // Stop-debounce for STRONG signals only: a name or handle match can drop for one
    // tick on a process-refresh race. The WEAK signals are already exact (a PID
    // transition: their "stopped" is the PID's death) so they do NOT enter the sticky.
    // Without them here the 90 s cycle and the "35 minutes without closing" disappear.
    // It refreshes the stamp of live strong slots and re-adds the ones that fell inside
    // the grace window.
    let now_inst = TokioInstant::now();
    for id in running.iter() {
        if let Some(slot) = slots.get_mut(id) {
            slot.last_running_seen = Some(now_inst);
            // A strong signal corroborates the session: it is no longer weak-only.
            slot.weak_session = false;
        }
    }
    // A snapshot of the ids with a STRONG signal this tick, BEFORE weak and sticky are
    // mixed in: the transitions below use it to know whether a start was
    // correlation-only (a phantom-session candidate).
    let strong_now: HashSet<String> = running.iter().cloned().collect();
    let sticky = Duration::from_secs(
        config
            .poll_secs
            .saturating_mul(RUNNING_STICKY_POLLS)
            .max(STRONG_STOP_GRACE_FLOOR_SECS),
    );
    let readd: Vec<String> = slots
        .iter()
        .filter(|(id, slot)| {
            slot.is_running
                && !strong_now.contains(id.as_str())
                && slot
                    .last_running_seen
                    .is_some_and(|seen| now_inst.duration_since(seen) < sticky)
        })
        .map(|(id, _)| id.clone())
        .collect();

    // The "one game at a time" guard: the strong signals (`running`) demand the game's
    // real exe to exist, so their slugs are games genuinely running NOW. Almost nobody
    // plays two at once, and a correlation stuck to a background process can keep an
    // already-closed game "started" forever. So when any game runs on a strong signal,
    // we discard the WEAK signals (correlation) and the ones re-added by sticky for
    // OTHER games: starting another game switches the phantom off and keeps it off
    // while you play. With no strong game at all, correlation and sticky still count
    // (games that ONLY match that way are still detected).
    let strong_slugs: HashSet<String> = running
        .iter()
        .filter_map(|id| slots.get(id).map(|s| s.save.game_slug.clone()))
        .collect();
    let survives_guard = |id: &str| -> bool {
        strong_slugs.is_empty()
            || slots
                .get(id)
                .is_some_and(|s| strong_slugs.contains(&s.save.game_slug))
    };
    for id in weak_running.into_iter().chain(readd) {
        if survives_guard(&id) {
            running.insert(id);
        } else {
            tracing::debug!(
                save_id = %id,
                "agent: weak or sticky signal discarded, another game is running on a strong signal"
            );
        }
    }

    // Playtime: attribute this tick's interval to the live games. The cap is four
    // times the poll (30 s minimum) so a suspend and resume is not counted as play.
    let mut running_games: Vec<(String, String)> = running
        .iter()
        .filter_map(|id| {
            slots
                .get(id)
                .map(|s| (id.clone(), s.save.game_slug.clone()))
        })
        .collect();
    // Add the played-but-untracked Steam games no slot already counts (which avoids
    // double counting by slug). The synthetic `save_id` is stable across ticks (it is
    // the anchor key in `PlaytimeStore::accrue`) and its prefix makes it obvious in
    // logs.
    if !steam_running.is_empty() {
        let counted: HashSet<String> = running_games.iter().map(|(_, s)| s.clone()).collect();
        for slug in &steam_running {
            if !counted.contains(slug) {
                running_games.push((format!("playtime:steam:{slug}"), slug.clone()));
            }
        }
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let max_step = config.poll_secs.saturating_mul(4).max(30);
    playtime.accrue(&running_games, now_ms, max_step);

    // Diff against previous tick to fire transition events.
    // We collect first, then mutate, to keep the borrow-checker happy.
    let transitions: Vec<(String, bool)> = slots
        .keys()
        .map(|id| (id.clone(), running.contains(id)))
        .filter(|(id, now)| slots.get(id).map(|s| s.is_running != *now).unwrap_or(false))
        .collect();
    // Persist eagerly when a game just stopped (fresh recap on quit); otherwise
    // throttle to avoid writing the JSON on every poll.
    let any_stop = transitions.iter().any(|(_, now)| !*now);

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
            // ¿Arranque solo-débil? Ninguna señal fuerte lo corrobora este
            // tick: candidato a sesión fantasma (ver `SaveSlot::weak_session`).
            let weak_start = !strong_now.contains(id.as_str());
            if let Some(slot) = slots.get_mut(&id) {
                slot.is_running = true;
                slot.weak_session = weak_start;
                // A new session earns a new "update waiting" notice if a pull
                // gets deferred again. `pull_pending` itself survives: an
                // update that arrived while the game was closed but couldn't
                // land (a restore in flight, un-flushed changes) is still owed.
                slot.deferred_notified = false;
            }
            // The correlated process's name goes into the log: without it a phantom
            // GameStarted is undiagnosable (the MOUSE case, jul-2026: days of hourly
            // starts with no way of knowing which process caused them).
            let corr_process = if weak_start {
                corr_store.attributed_name(&local_path)
            } else {
                None
            };
            tracing::info!(
                save_id = %id,
                game_slug = %game_slug,
                path = %local_path.display(),
                signal = if weak_start { "correlation" } else { "strong" },
                corr_process = %corr_process.as_deref().unwrap_or("-"),
                "agent: GameStarted"
            );
            let _ = events_tx.try_send(AgentEvent::GameStarted {
                save_id: id.clone(),
                game_slug,
            });
            // The old "pre-launch sync barrier" (an edge-triggered pull at the moment
            // of launch) disappears with the inverted authority (ADR 0021 C.1): the
            // model is level-triggered, so the reducer already restored any
            // cross-device delta on a quiet tick BEFORE launching, and the
            // `reconcile_all` that follows this poll defers (with a flush) the pull if
            // the cloud moved ahead right at launch. Some latency is lost in the narrow
            // "bump less than a tick before launching" window (it lands on close); see
            // the slice's summary.
        } else {
            let was_weak_session = slots.get(&id).map(|s| s.weak_session).unwrap_or(false);
            if let Some(slot) = slots.get_mut(&id) {
                slot.is_running = false;
                slot.weak_session = false;
            }
            // A phantom session: it started on correlation alone and died without ONE
            // write to the folder. A real game writes while being played (and every
            // write re-records the observation and absolves it), so this only
            // accumulates over poisoned attributions, such as the hourly task that had
            // one save "mid-session" for days. On the second strike the observation
            // falls and the weak signal dies with it.
            if was_weak_session && !had_pending {
                match corr_store.strike_phantom(&local_path) {
                    Some(true) => {
                        tracing::warn!(
                            save_id = %id,
                            game_slug = %game_slug,
                            "agent: correlation observation discarded, repeated \
                             phantom sessions with no writes"
                        );
                        if let Some(p) = corr_path {
                            if let Err(e) = corr_store.save(p) {
                                tracing::debug!(error = %e, "agent: failed to persist correlation store");
                            }
                        }
                    }
                    Some(false) => {
                        tracing::info!(
                            save_id = %id,
                            game_slug = %game_slug,
                            "agent: phantom session (correlation with no writes), striking the observation"
                        );
                    }
                    None => {}
                }
            } else if had_pending {
                // The session wrote: the attribution is legitimate, so clear the
                // strikes.
                corr_store.absolve(&local_path);
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
            // The final flush when the game closes and the deferred pull's landing are
            // NO LONGER launched here: they are sync decisions the reducer emits in the
            // `reconcile_all` that follows this poll. Clearing `is_running` above lifts
            // the session veto (past the sticky grace) and the reducer sees
            // `has_pending` plus a quiet folder, so a backup (the final flush), and
            // `pull_pending` or cloud-ahead plus quiet, so a restore (the deferred pull
            // lands). `process_poll` only samples the world (ADR 0021 C.1).
            // `had_pending` now only feeds the phantom-session striking above.
        }
    }

    // PLAYTIME: vuelca a disco (inmediato al parar un juego, throttled si no).
    if any_stop {
        playtime.flush(playtime_path, now_ms);
    } else {
        playtime.flush_if_due(playtime_path, now_ms);
    }

    // An untracked Steam game that is running also keeps the fast cadence: otherwise
    // the idle interval could exceed `accrue`'s cap and undercount its hours.
    !running.is_empty() || !steam_running.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn canon_token_unifies_slug_name_and_exe() {
        // All three shapes of the same game collapse to one token.
        assert_eq!(canon_token("victoria-3"), "victoria3");
        assert_eq!(canon_token("Victoria 3"), "victoria3");
        assert_eq!(canon_token("victoria3.exe"), "victoria3exe");
        assert_eq!(canon_token("stellaris"), "stellaris");
    }

    #[test]
    fn game_tokens_drop_short_and_dedup() {
        // A slug and a display name that collapse to the same token give one.
        assert_eq!(
            game_identity_tokens("stellaris", "Stellaris"),
            ["stellaris"]
        );
        // A token that is too short is discarded (it collides with any folder).
        assert!(game_identity_tokens("gta", "GTA").is_empty());
    }

    #[test]
    fn process_matches_game_by_exe_basename() {
        // The Stellaris and Victoria case: the exe carries the game's name.
        let cands = process_identity_candidates(
            "victoria3",
            Some(Path::new(
                "/home/u/.steam/steamapps/common/Victoria 3/binaries/victoria3",
            )),
        );
        assert!(cands.contains(&"victoria3".to_string()));
    }

    #[test]
    fn process_matches_game_by_install_folder() {
        // An abbreviated exe (`witcher3`) but the FOLDER carries the full name: the
        // slug matches through the path component rather than the basename.
        let cands = process_identity_candidates(
            "witcher3",
            Some(Path::new(
                "/games/GOG Games/The Witcher 3 Wild Hunt/bin/x64/witcher3.exe",
            )),
        );
        assert!(cands.contains(&canon_token("the-witcher-3-wild-hunt")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_handle_detects_process_holding_a_save_file() {
        // A file open inside the save folder gives its owning process away, without
        // depending on the exe's name.
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("Save Games");
        std::fs::create_dir_all(&save).unwrap();
        let f = std::fs::File::create(save.join("autosave.sav")).unwrap();
        let pid = Pid::from_u32(std::process::id());
        let folders: Vec<(&str, &Path)> = vec![("save-eu5", save.as_path())];
        let hits = open_paths_matching(pid, &folders);
        assert!(hits.contains(&"save-eu5".to_string()));
        drop(f);
        // With the file closed, it no longer matches.
        let hits2 = open_paths_matching(pid, &folders);
        assert!(!hits2.contains(&"save-eu5".to_string()));
    }

    #[test]
    fn generic_identity_ignores_unrelated_process() {
        // An unrelated process does not produce the game's token.
        let cands =
            process_identity_candidates("firefox", Some(Path::new("/usr/lib/firefox/firefox")));
        assert!(!cands.contains(&"stellaris".to_string()));
    }

    #[test]
    fn generic_and_profile_tokens_are_vetoed() {
        // The real jul-2026 case: a save ended up tracked with its slug set to the
        // Windows username. The username is a path component of EVERY exe in the
        // profile, so any app fired "you are playing". Plumbing tokens cannot be
        // identity, for either a game or a process.
        for t in [
            "users",
            "appdata",
            "roaming",
            "locallow",
            "savedgames",
            "games",
        ] {
            assert!(is_generic_identity_token(t), "{t} debería vetarse");
        }
        assert!(!is_generic_identity_token("eldenring"));
        assert!(!is_generic_identity_token("mousepiforhire"));
        // On the game's side: a degenerate slug produces no tokens...
        assert!(game_identity_tokens("games", "Saved Games").is_empty());
        // ...and on the process's side, the profile's components do not come out as
        // candidates (the exe and its install folder do). A path with the native
        // separator: the components are only extracted that way.
        let cands = process_identity_candidates(
            "game.exe",
            Some(Path::new("/Users/bob/AppData/Roaming/GSE Saves/game.exe")),
        );
        assert!(!cands
            .iter()
            .any(|c| c == "users" || c == "appdata" || c == "roaming"));
        assert!(cands.contains(&"gsesaves".to_string()));
        // An ordinary game keeps its identity through its install folder.
        let cands = process_identity_candidates(
            "witcher3",
            Some(Path::new(
                "/games/GOG Games/The Witcher 3 Wild Hunt/bin/x64/witcher3.exe",
            )),
        );
        assert!(cands.contains(&canon_token("the-witcher-3-wild-hunt")));
    }

    /// Same poisoning, handheld flavour (report ago-2026, Linux handheld): a
    /// save minted from an emulator front-end's `~/Emulation/storage` tree got
    /// the slug `storage`, and on an image-based distro every containerised
    /// process runs out of `…/containers/storage/overlay/<hash>/merged/…`, so
    /// that slug matched a path component of half the process table as a STRONG
    /// signal. The game was "running" forever and nothing could close it.
    #[test]
    fn handheld_plumbing_tokens_are_vetoed() {
        for t in ["storage", "emulation", "roms", "containers", "overlay"] {
            assert!(is_generic_identity_token(t), "{t} should be vetoed");
        }
        // Game side: no tokens at all, so no process can "run" it.
        assert!(game_identity_tokens("storage", "storage").is_empty());
        // Process side: a binary inside a container contributes neither
        // "storage" nor the rest of the overlay plumbing as an identity.
        let cands = process_identity_candidates(
            "gnome-shell",
            Some(Path::new(
                "/var/lib/containers/storage/overlay/2f9a1b/merged/usr/bin/gnome-shell",
            )),
        );
        assert!(
            !cands
                .iter()
                .any(|c| c == "storage" || c == "containers" || c == "overlay" || c == "merged"),
            "container plumbing can't be an identity: {cands:?}"
        );
        // And a real game under the same root keeps its own.
        let cands = process_identity_candidates(
            "hollow_knight",
            Some(Path::new(
                "/home/deck/Emulation/roms/Hollow Knight/hollow_knight.x86_64",
            )),
        );
        assert!(cands.contains(&canon_token("hollow-knight")));
    }

    #[test]
    fn config_defaults_are_sane() {
        let c = AgentConfig::default();
        assert!(c.debounce_secs >= 5, "too eager");
        assert!(c.debounce_secs <= 120, "too sleepy");
        assert!(c.poll_secs >= 1);
        assert!(c.max_retries >= 1);
    }

    /// ADR 0021 D.12: the engine observes the cloud itself, but a healthy client saves
    /// it the trip. While the feed is recent, no query of its own fires. That is what
    /// keeps the cost at ONE manifest per interval rather than two.
    #[test]
    fn self_observation_is_suppressed_by_a_live_feed_and_paced_when_blind() {
        let t0 = OffsetDateTime::UNIX_EPOCH;
        let secs = |n: i64| t0 + Duration::from_secs(n as u64);
        let due = kernel::reconcile::CLOUD_SELF_OBSERVE_AFTER_SECS;

        // A cold start: with no feed and no attempts, it observes now (the engine does
        // not wait to be spoon-fed).
        let mut heads = CloudHeads::new(t0);
        assert!(heads.due_for_self_observation(t0));

        // With a freshly arrived feed (a live poller) there is nothing to fetch...
        heads.feed(HashMap::new(), None, None, secs(10));
        assert!(!heads.due_for_self_observation(secs(10 + due - 1)));
        // ...until that feed gets old: the poller went quiet and the engine covers the
        // gap on its own.
        assert!(heads.due_for_self_observation(secs(10 + due)));

        // An attempt of its own that brought no heads (network down, a 401) paces it:
        // one retry per deadline, not one per two-second tick.
        heads.last_attempt_at = Some(secs(100));
        assert!(!heads.due_for_self_observation(secs(101)));
        assert!(heads.due_for_self_observation(secs(100 + due)));

        // Self-hosted still observes (`GET /v1/saves`). A live feed still
        // suppresses the extra GET until it goes stale.
        heads.is_cloud = Some(false);
        heads.last_attempt_at = Some(secs(100));
        heads.as_of = Some(secs(100));
        assert!(!heads.due_for_self_observation(secs(101)));
        assert!(heads.due_for_self_observation(secs(100 + due)));
    }

    #[test]
    fn merge_version_does_not_wipe_or_regress() {
        let t0 = OffsetDateTime::UNIX_EPOCH;
        let mut heads = CloudHeads::new(t0);
        heads.feed(
            HashMap::from([("a".into(), 2), ("b".into(), 1)]),
            None,
            None,
            t0,
        );
        let as_of = heads.as_of;
        heads.merge_version("b".into(), 4);
        heads.merge_version("b".into(), 3); // older must not regress
        heads.merge_version("c".into(), 1);
        assert_eq!(heads.versions.get("a"), Some(&2));
        assert_eq!(heads.versions.get("b"), Some(&4));
        assert_eq!(heads.versions.get("c"), Some(&1));
        assert_eq!(heads.as_of, as_of, "SSE merge must not suppress list_saves");
    }

    /// ADR 0021 D.11: the startup allowance's anchor only exists when there is a cloud
    /// to observe. With no resolved context nothing is claimed: declaring blindness
    /// without knowing whether there is a cloud would be inventing a fault.
    #[test]
    fn expected_since_is_anchored_only_in_cloud_context() {
        let t0 = OffsetDateTime::UNIX_EPOCH;
        let mut heads = CloudHeads::new(t0);
        assert_eq!(heads.expected_since(), None, "contexto sin resolver");

        heads.is_cloud = Some(false);
        assert_eq!(heads.expected_since(), None, "self-hosted: no hay feed");

        heads.is_cloud = Some(true);
        assert_eq!(
            heads.expected_since(),
            Some(t0),
            "cloud: la cuenta atrás corre desde que el motor arrancó"
        );
    }

    #[test]
    fn a_shared_process_needs_a_write_in_this_saves_own_folder() {
        let now = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(10);

        // The emulator starts and nobody has saved yet: no title counts as playing.
        // This is the case that avoids inventing hours in the nine saves nobody is
        // touching.
        assert!(!shared_process_is_corroborated(None, now));

        // The title that has just saved does.
        assert!(shared_process_is_corroborated(
            Some(now - time::Duration::minutes(1)),
            now
        ));

        // Y sigue contando entre autoguardados espaciados.
        assert!(shared_process_is_corroborated(
            Some(now - time::Duration::minutes(25)),
            now
        ));

        // But a save touched this morning does not slip into the current session just
        // because the emulator is open.
        assert!(!shared_process_is_corroborated(
            Some(now - time::Duration::hours(3)),
            now
        ));
    }

    #[test]
    fn probe_seeds_baseline_then_reports_writes() {
        let dir = tempfile::tempdir().unwrap();
        let cand = dir.path().to_path_buf();
        std::fs::write(cand.join("save1.zip"), b"a").unwrap();

        let mut probes: HashMap<PathBuf, Option<std::time::SystemTime>> = HashMap::new();
        probes.insert(cand.clone(), None);

        // The first tick only seeds the baseline and reports nothing.
        assert!(probe_detect_writes(&mut probes).is_empty());
        assert!(probes[&cand].is_some());

        // A later write (a greater mtime) is reported.
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

    // The `accept_correlation_signals` tests (the D.4 corpus phantom-hours regression
    // included) moved with the function into the leaf kernel:
    // `hoard_core::kernel::correlation::tests`.

    /// A slot in the state a freshly added save has: nothing pending, nothing
    /// scheduled, no burst.
    fn test_slot(save: WatchedSave) -> SaveSlot {
        SaveSlot {
            save,
            watcher: None,
            pending: None,
            burst_since: None,
            burst_backups: 0,
            is_running: false,
            weak_session: false,
            last_running_seen: None,
            has_pending: false,
            last_fs_event_at: None,
            last_restore_at: None,
            next_scheduled_backup_at: None,
            first_pending_event_at: None,
            last_backup_at: None,
            in_flight: None,
            next_backup_at: None,
            next_restore_at: None,
            restore_failures: kernel::RestoreFailures::default(),
            backup_conflict: kernel::ConflictStall::default(),
            last_set_hash: None,
            synced_fingerprint: None,
            last_l0_mtime: None,
            needs_l1: false,
            manual_requested: false,
            pending_op_result: None,
            pending_upload_landed: None,
            last_restore_error: None,
            last_conflict_error: None,
            known_version: None,
            pull_pending: false,
            deferred_notified: false,
        }
    }

    fn tracked(save_id: &str, game_slug: &str, label: &str) -> WatchedSave {
        WatchedSave {
            save_id: save_id.into(),
            game_slug: game_slug.into(),
            display_name: game_slug.into(),
            label: label.into(),
            local_path: PathBuf::from("/tmp/saves/x"),
            steam_install_dir: None,
            processes: vec![],
            shared_processes: false,
            policy: Default::default(),
            allow_device_local: None,
            known_version: None,
            set_hash: None,
            track_only: false,
        }
    }

    fn heads_with(rows: &[(&str, &str, &str, i64)]) -> CloudHeads {
        let mut versions = HashMap::new();
        let mut aliases = HashMap::new();
        for (id, slug, label, v) in rows {
            versions.insert((*id).to_string(), *v);
            aliases.insert(
                ((*slug).to_string(), (*label).to_string()),
                (*id).to_string(),
            );
        }
        let mut heads = CloudHeads::new(OffsetDateTime::UNIX_EPOCH);
        heads.feed(
            versions,
            Some(aliases),
            None,
            OffsetDateTime::UNIX_EPOCH + Duration::from_secs(1),
        );
        heads
    }

    /// A save whose local id the cloud has never seen must still read its own
    /// head. This is the whole aug-2026 failure in one assertion: the lookup
    /// missed, `cloud_version` came back `None`, and a row fourteen versions
    /// behind was reported as converged, while every upload it tried was
    /// rejected as non-fast-forward by the row it couldn't see.
    #[test]
    fn a_drifted_local_id_still_reads_its_cloud_head() {
        let heads = heads_with(&[("cloud-side", "factorio", "main", 284)]);
        let save = tracked("local-only", "factorio", "main");
        assert_eq!(heads.cloud_id_for(&save), "cloud-side");
        assert_eq!(heads.version_for(&save), Some(284));
    }

    /// The id still wins when the cloud knows it, alias or no alias.
    #[test]
    fn a_known_local_id_is_used_as_is() {
        let heads = heads_with(&[
            ("mine", "factorio", "main", 7),
            ("theirs", "factorio", "2 · slot", 99),
        ]);
        let save = tracked("mine", "factorio", "main");
        assert_eq!(heads.cloud_id_for(&save), "mine");
        assert_eq!(heads.version_for(&save), Some(7));
    }

    /// An alias left over from an older feed points at a row this pass doesn't
    /// carry (deleted server-side, or a manifest that came back partial).
    /// Believing it would hand the reducer another save's version.
    #[test]
    fn an_alias_for_a_row_thats_gone_is_ignored() {
        let mut heads = heads_with(&[("cloud-side", "factorio", "main", 284)]);
        // A later feed without that row, and without names: the aliases stay.
        heads.feed(
            HashMap::new(),
            None,
            None,
            OffsetDateTime::UNIX_EPOCH + Duration::from_secs(2),
        );
        let save = tracked("local-only", "factorio", "main");
        assert_eq!(heads.cloud_id_for(&save), "local-only");
        assert_eq!(heads.version_for(&save), None);
    }

    /// Genuinely absent from the cloud stays absent. The fallback resolves an
    /// id; it never invents a head, because "blind" and "converged" have to
    /// stay distinguishable (ADR 0021 D.10).
    #[test]
    fn a_save_the_cloud_doesnt_have_reads_as_absent() {
        let heads = heads_with(&[("cloud-side", "stellaris", "main", 4)]);
        let save = tracked("local-only", "factorio", "main");
        assert_eq!(heads.version_for(&save), None);
    }

    /// The digest is only trusted when it belongs to the version that is head
    /// *now*, and that check has to happen after the id is resolved: a digest
    /// looked up under the local id would simply never be found.
    #[test]
    fn the_head_digest_follows_the_resolved_id() {
        let mut heads = heads_with(&[("cloud-side", "factorio", "main", 284)]);
        heads.digests.insert(
            "cloud-side".into(),
            ServerHead {
                version_num: 284,
                digest: "abc".into(),
            },
        );
        let save = tracked("local-only", "factorio", "main");
        assert_eq!(
            heads.head_for(&save).map(|h| h.digest.as_str()),
            Some("abc")
        );

        // A digest from an older version describes content that is no longer
        // head, so it is refused rather than used to skip a real upload.
        heads.digests.insert(
            "cloud-side".into(),
            ServerHead {
                version_num: 283,
                digest: "stale".into(),
            },
        );
        assert!(heads.head_for(&save).is_none());
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
            shared_processes: false,
            policy: Default::default(),
            allow_device_local: None,
            known_version: None,
            set_hash: None,
            track_only: false,
        };
        let mut slots = HashMap::new();
        slots.insert("abc".to_string(), test_slot(save));

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

    /// The adaptive floor has to be *visible*. A fixed one was reverted for
    /// being invisible ("Hoard isn't picking up my changes"); a conditional one
    /// nobody can see fails the same way to fewer people. The wait lands in
    /// `next_scheduled_backup_at`, which the overlay's "next copy in Xs" reads,
    /// and is announced once, not on every one of the thirty ticks the floor
    /// spans.
    #[tokio::test(flavor = "current_thread")]
    async fn a_deferred_backup_shows_when_it_will_go_out() {
        let save = WatchedSave {
            save_id: "burst-1".into(),
            game_slug: "fake-game".into(),
            display_name: "Fake Game".into(),
            label: "main".into(),
            local_path: PathBuf::from("/tmp/saves/fake"),
            steam_install_dir: None,
            processes: vec![],
            shared_processes: false,
            policy: Default::default(),
            allow_device_local: None,
            known_version: None,
            set_hash: None,
            track_only: false,
        };
        let mut slots = HashMap::new();
        slots.insert("burst-1".to_string(), test_slot(save));
        let (events_tx, mut events_rx) = mpsc::channel::<AgentEvent>(8);

        let now = OffsetDateTime::now_utc();
        let floor = now + time::Duration::seconds(60);

        // Nothing pending is the ordinary quiet state, not a queue.
        announce_backup_wait(&mut slots, "burst-1", Some(floor), now, &events_tx);
        assert!(slots["burst-1"].next_scheduled_backup_at.is_none());
        assert!(events_rx.try_recv().is_err());

        slots.get_mut("burst-1").unwrap().has_pending = true;
        announce_backup_wait(&mut slots, "burst-1", Some(floor), now, &events_tx);
        assert_eq!(slots["burst-1"].next_scheduled_backup_at, Some(floor));
        match events_rx.try_recv() {
            Ok(AgentEvent::BackupScheduled { delay_ms, .. }) => {
                assert!(
                    (59_000..=60_000).contains(&delay_ms),
                    "announced {delay_ms}ms, not the minute it is actually waiting"
                );
            }
            other => panic!("expected a scheduled backup, got {other:?}"),
        }

        // The reducer holds again on every tick; the feed must not.
        for i in 1..30 {
            let t = now + time::Duration::seconds(i * 2);
            announce_backup_wait(&mut slots, "burst-1", Some(floor), t, &events_tx);
        }
        assert!(
            events_rx.try_recv().is_err(),
            "re-announced the same wait, which is what flooded the feed with orphan rows"
        );
    }

    /// A Proton game that dies badly leaves its .exe defunct, keeping the name
    /// and exe path every strong matcher keys on. Nothing about a zombie says
    /// "the user is playing" (it can't write a save file) so it must never
    /// hold a slot `is_running`, which is what pinned the mid-session veto open
    /// and stranded cross-device updates on the Deck.
    #[test]
    fn defunct_processes_are_not_evidence_of_a_live_session() {
        assert!(is_defunct(ProcessStatus::Zombie), "exited, not yet reaped");
        assert!(is_defunct(ProcessStatus::Dead));

        assert!(!is_defunct(ProcessStatus::Run));
        assert!(!is_defunct(ProcessStatus::Sleep));
        // A Paradox game sitting in a menu burns no CPU and reads as Idle, which is
        // very much a live session.
        assert!(!is_defunct(ProcessStatus::Idle));
        // SIGSTOP'd (or suspended): it can resume and write at any moment.
        assert!(!is_defunct(ProcessStatus::Stop));
    }

    /// Pins the assumption `is_defunct` rests on: our minimal
    /// `proc_refresh_kind` really does populate `status()`, and a genuine
    /// unreaped child really does read as defunct through it. If sysinfo ever
    /// puts `status` behind a refresh flag, the zombie filter would silently
    /// go back to matching leftovers: this fails instead.
    #[cfg(target_os = "linux")]
    #[test]
    fn sysinfo_reports_an_unreaped_child_as_defunct() {
        // Exits immediately; we're its parent and don't reap until the end, so
        // it lingers in the process table exactly like Proton's leftovers.
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn a short-lived child");
        let pid = Pid::from_u32(child.id());
        let mut sys =
            System::new_with_specifics(RefreshKind::new().with_processes(proc_refresh_kind()));

        let mut saw_defunct = false;
        for _ in 0..50 {
            sys.refresh_processes_specifics(ProcessesToUpdate::All, true, proc_refresh_kind());
            if let Some(p) = sys.process(pid) {
                if is_defunct(p.status()) {
                    saw_defunct = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        let _ = child.wait();
        assert!(
            saw_defunct,
            "an unreaped exited child must read as defunct through the agent's refresh kind"
        );
    }

    /// An end-to-end integration of the inverted path (ADR 0021 Slice 2b): a save
    /// with no `processes` and no `steam_install_dir` (nothing matches) has to fire a
    /// backup when its folder is rewritten, with no game running. In the inverted
    /// model that is: the watcher armed in `handle_add`, an fs event marking
    /// `has_pending` and `needs_l1` and arming the debounce timer, a nudge,
    /// `reconcile_all`, and the reducer emitting `Backup` (has_pending plus diverging
    /// content), so `run_backup_with_retry` starts and emits `BackupStarted`.
    /// real empezando prueba el mismo invariante extremo-a-extremo, mejor.)
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
            shared_processes: false,
            policy: Default::default(),
            allow_device_local: None,
            known_version: None,
            set_hash: None,
            track_only: false,
        };

        // Short debounce so the test completes well under the 10s timeout.
        let config = AgentConfig {
            debounce_secs: 1,
            poll_secs: 2,
            max_retries: 0,
            auto_restore: false,
            global_sync: false,
            conflict_root: None,
            conflict_retention_days: 14,
            min_snapshot_interval_secs: 0,
        };

        let (handle, task) = spawn(api, config, vec![save], events_tx);

        // Give the agent a beat to register the save before we touch the
        // folder, or the fs event could land before `AddSave` is
        // processed.
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Touch a file inside the watched directory.
        let mut f = std::fs::File::create(save_path.join("save.dat")).expect("create save file");
        f.write_all(b"hello").expect("write save file");
        f.sync_all().expect("sync save file");
        drop(f);

        // Espera `BackupStarted` en 10s. Si el watcher no armara (el bug) o el
        // reductor no emitiera el backup, esto expira.
        let started = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(evt) = events_rx.recv().await {
                if let AgentEvent::BackupStarted { save_id, .. } = evt {
                    return save_id;
                }
            }
            "<channel closed>".to_string()
        })
        .await;

        // Best-effort teardown before asserting so the task doesn't leak.
        let _ = handle.shutdown().await;
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await;

        let save_id = started.expect(
            "timed out waiting for BackupStarted: the fs event never reached the reducer as a backup",
        );
        assert_eq!(save_id, "watcher-bug-1");
    }

    /// A backup that burns its whole retry budget used to leave the slot in a
    /// corner it could never climb out of: no `BackupDone` (correctly, since the
    /// changes never reached a version, so `has_pending` must stay set to keep
    /// restores off them), but also nothing that would ever try the upload
    /// again. `has_pending` is itself a mid-session veto, so the save could
    /// neither be pushed nor pulled until the user happened to write the folder
    /// again. The task must hand a retry back to the agent loop.
    #[tokio::test(flavor = "current_thread")]
    async fn exhausted_backup_retries_hand_back_a_retry_and_keep_changes_pending() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(&tmp.path().join("save.dat"), b"unversioned progress");

        // Port 1 refuses instantly: a real failure, not a throttle or a 413.
        let api = ApiClient::new("http://127.0.0.1:1", "fake").expect("fake api client");
        let (events_tx, mut events_rx) = mpsc::channel::<AgentEvent>(64);
        let (done_tx, mut done_rx) = mpsc::channel::<BackupDone>(8);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<AgentCommand>(8);

        let save = WatchedSave {
            save_id: "wedged-1".into(),
            game_slug: "fake-game".into(),
            display_name: "Fake Game".into(),
            label: "main".into(),
            local_path: tmp.path().to_path_buf(),
            steam_install_dir: None,
            processes: vec![],
            shared_processes: false,
            policy: Default::default(),
            allow_device_local: None,
            known_version: None,
            set_hash: None,
            track_only: false,
        };

        // `max_retries: 0` → the first failure is already the last.
        run_backup_with_retry(
            api,
            save,
            None,
            None,
            None,
            VersionOrigin::Automatic,
            events_tx,
            done_tx,
            cmd_tx,
            0,
            false,
            None,
            14,
        )
        .await;

        let mut failed = false;
        while let Ok(ev) = events_rx.try_recv() {
            if let AgentEvent::BackupFailed { will_retry, .. } = ev {
                assert!(!will_retry, "the budget is spent");
                failed = true;
            }
        }
        assert!(failed, "a real failure must reach the feed");

        assert!(
            done_rx.try_recv().is_err(),
            "no BackupDone: clearing has_pending would let a restore overwrite \
             changes that were never versioned"
        );
        assert!(
            matches!(
                cmd_rx.try_recv(),
                Ok(AgentCommand::RetryBackupAfterFailure(id)) if id == "wedged-1"
            ),
            "the slot must get a retry path that doesn't depend on a new fs event"
        );
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

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

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

    /// La basura local no es divergencia. `disk_set_hash` no la cuenta (sale de
    /// `walk_source`), so counting it here would mark `local_diverged` on every
    /// auto-restore of any game with a log, and the engine would repeat a walk
    /// y un hash de contenido enteros cada vez.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_ignores_junk_when_counting_local_only() {
        let dir = tempfile::tempdir().unwrap();
        let source = &dir.path().join("staging");
        let target = &dir.path().join("live");
        std::fs::create_dir_all(source).unwrap();
        std::fs::create_dir_all(target).unwrap();
        std::fs::write(source.join("slot1.sav"), b"partida").unwrap();
        std::fs::write(target.join("slot1.sav"), b"partida").unwrap();
        // Local only, and of the kind the backup never uploads.
        std::fs::write(target.join("Player.log"), b"log").unwrap();
        std::fs::write(target.join(".DS_Store"), b"junk").unwrap();

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();
        assert_eq!(stats.target_only, 0, "la basura no es divergencia");

        // But config does count: it exists only locally until it is uploaded.
        std::fs::write(target.join("graphics.ini"), b"res=1080").unwrap();
        let stats = restore_files_into(target, source, None, &[]).await.unwrap();
        assert_eq!(stats.target_only, 1, "la config sí debe contar");
    }

    /// Local-only files: a file present in the target but absent from the
    /// remote snapshot is left untouched and counted under `target_only`.
    /// This is the divergence signal the conflict/auto-restore path keys on
    /// to decide it must re-upload rather than settle: getting it wrong
    /// would silently drop local data, so pin the count explicitly.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_counts_local_only_files() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        // Remote snapshot has a.dat; target has the same a.dat plus two files
        // the snapshot knows nothing about (one nested).
        write_file(&source.join("a.dat"), b"alpha");
        write_file(&target.join("a.dat"), b"alpha");
        write_file(&target.join("local-only.sav"), b"unsynced");
        write_file(&target.join("nested/also-local.sav"), b"more");

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

        assert_eq!(stats.restored, 0, "a.dat is identical, nothing copied");
        assert_eq!(stats.skipped, 1, "a.dat skipped");
        assert_eq!(
            stats.target_only, 2,
            "two files exist locally but not in the snapshot"
        );
        assert_eq!(stats.conflicts_resolved_local, 0);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        // Local-only files are never deleted by a restore.
        assert_eq!(
            std::fs::read(target.join("local-only.sav")).unwrap(),
            b"unsynced"
        );
        assert_eq!(
            std::fs::read(target.join("nested/also-local.sav")).unwrap(),
            b"more"
        );
    }

    /// Mirror image: when the target is a strict subset of the snapshot
    /// (everything local also exists remotely), `target_only` is zero, and the
    /// signal that a purely-behind device can settle without re-uploading.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_no_local_only_when_target_is_subset() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"alpha");
        write_file(&source.join("sub/b.dat"), b"beta");
        // Target only has a.dat (subset); b.dat will be copied in.
        write_file(&target.join("a.dat"), b"alpha");

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

        assert_eq!(stats.restored, 1, "b.dat copied");
        assert_eq!(stats.skipped, 1, "a.dat identical");
        assert_eq!(stats.target_only, 0, "no file exists only locally");
    }

    /// Conflict case: A exists in both source and target but bytes differ.
    /// The local copy wins: bytes on disk stay as the target's version
    /// and the conflict is reported in stats.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_preserves_local_on_conflict() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();

        write_file(&source.join("a.dat"), b"remote-version");
        write_file(&target.join("a.dat"), b"LOCAL-WORK");

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

        assert_eq!(stats.restored, 0, "nothing copied: A is a conflict");
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

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

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

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

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

        let stats = restore_files_into(target, source, Some(backup), &[])
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

        let stats = restore_files_into(target, source, Some(backup), &[])
            .await
            .unwrap();

        assert_eq!(stats.conflicts_resolved_local, 1);
        assert_eq!(stats.conflicts_resolved_remote, 0);
        assert_eq!(stats.conflicts_backed_up, 0);
        assert_eq!(stats.bytes_restored, 0);
        assert_eq!(std::fs::read(target.join("a.dat")).unwrap(), b"LOCAL-WORK");
        // No backup was created; `backup` is still empty.
        assert!(std::fs::read_dir(backup).unwrap().next().is_none());
    }

    /// Files written by the merge must keep the snapshot's mtime, not the
    /// time of the restore. `fs::copy` alone stamps mtime=now, which made
    /// every restored save look brand-new: games that pick "continue" by
    /// most-recent file loaded the wrong save, and the follow-up upload
    /// pushed the inflated mtimes to the server.
    #[tokio::test(flavor = "current_thread")]
    async fn restore_files_into_preserves_snapshot_mtimes() {
        let source_tmp = tempfile::tempdir().unwrap();
        let target_tmp = tempfile::tempdir().unwrap();
        let backup_tmp = tempfile::tempdir().unwrap();
        let source = source_tmp.path();
        let target = target_tmp.path();
        let backup = backup_tmp.path();

        let old = std::time::SystemTime::now() - Duration::from_secs(30 * 24 * 3600);
        // Plain restore: file missing locally.
        write_file(&source.join("fresh.dat"), b"from-cloud");
        set_mtime(&source.join("fresh.dat"), old);
        // Conflict the remote wins: overwrite path.
        write_file(&source.join("clash.dat"), b"remote-new");
        write_file(&target.join("clash.dat"), b"local-old");
        set_mtime(&source.join("clash.dat"), old + Duration::from_secs(20));
        set_mtime(&target.join("clash.dat"), old);

        let stats = restore_files_into(target, source, Some(backup), &[])
            .await
            .unwrap();
        assert_eq!(stats.restored, 1);
        assert_eq!(stats.conflicts_resolved_remote, 1);

        let mtime_of = |p: PathBuf| std::fs::metadata(p).unwrap().modified().unwrap();
        let close = |a: std::time::SystemTime, b: std::time::SystemTime| {
            let d = a.duration_since(b).unwrap_or_else(|e| e.duration());
            d < Duration::from_secs(1)
        };
        assert!(
            close(mtime_of(target.join("fresh.dat")), old),
            "restored file must carry the snapshot mtime, not now()"
        );
        assert!(
            close(
                mtime_of(target.join("clash.dat")),
                old + Duration::from_secs(20)
            ),
            "conflict-overwritten file must carry the snapshot mtime, not now()"
        );
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

        let stats = restore_files_into(target, source, None, &[]).await.unwrap();

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
