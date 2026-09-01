//! Sans-IO domain kernel (ADR 0021, part C).
//!
//! A deterministic function of `(state, observed world, now, seed)` that does no
//! IO at all. Everything else, the daemon runtime, the DB, HTTP, Axum, Tauri, is
//! an IO shell wrapped around this.
//!
//! Hard rule (guardrail D.2): no `Instant::now()`, no `thread_rng`, no IO in
//! here. The instant and the RNG seed arrive as data on [`World`]; the shell
//! samples them and injects them. Jitter uses `StdRng::seed_from_u64(world.seed)`
//! and nothing else.
//!
//! The engine tick is the source of truth: sample the world, build an
//! [`Observation`], call [`reconcile::reconcile`], run the [`Decision`]s (`Act`
//! goes out to IO, `Hold` gets its reason logged). Events from the watcher or
//! from realtime are hints that pull a tick forward; they never decide anything.

pub mod correlation;
pub mod fileclass;
pub mod insight;
pub mod reconcile;
pub mod restore_merge;
pub mod session;
pub mod slots;

use time::OffsetDateTime;

/// The non-determinism injected into the kernel. `now` governs every deadline
/// (min-interval, cooldown, throttle backoff, the veto grace window); `seed`
/// feeds the one place with any randomness in it, the jitter on throttle
/// backoff.
///
/// ADR 0021 (C.2) is blunt about this: both come in as input, and the kernel
/// never calls `Instant::now()` or `thread_rng`, or simulation and replay stop
/// being deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct World {
    pub now: OffsetDateTime,
    pub seed: u64,
}

/// Which IO operation is in flight for a save. This is anti-relaunch memory:
/// without it every tick would kick off a multi-GB transfer that is already
/// running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Backup,
    Restore,
}

/// Restore failure escalation, keyed by cloud *version* rather than by save: a
/// new version is new content and a fresh reason to try again, so it resets the
/// escalation instead of inheriting the old version's penalty. Sans-IO twin of
/// `agent::AutoRestoreFailures`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestoreFailures {
    pub consecutive: u32,
    /// `None` means unknown: self-hosted, or before the first poll.
    pub version: Option<i64>,
    pub stuck_notified: bool,
}

/// Escalation for a backup that runs into an **unresolvable** conflict: the
/// server says 409 "you are behind" and reconciliation finds nothing to pull.
/// Neither side can give way on its own, so retrying is asking the same question
/// over and over.
///
/// It exists because the shell used to answer that case by re-arming the retry
/// with no counter and no escalation: 1,701 events, 5 users, one save pinned for
/// 14 days at roughly 4.5 attempts an hour that outlived three releases of the
/// app. The comment above it read "surface the conflict rather than risk a loop",
/// and the code right underneath built the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConflictStall {
    pub consecutive: u32,
    /// A different cloud head is new information (maybe now there *is* something
    /// to pull) and resets the whole escalation.
    pub version: Option<i64>,
    /// Budget spent, stop retrying on our own. Cleared by a user action such as
    /// a manual copy, by a backup that succeeds, or by the cloud publishing
    /// another version.
    ///
    /// Does two jobs at once, on purpose: it is the gate `decide_backup` checks
    /// to stop emitting the upload, and it is the edge the shell derives the UI
    /// warning from (same as [`RestoreFailures::stuck_notified`], compared
    /// before and after the reducer). A separate flag for the warning could
    /// drift away from the brake, and then the UI calls a save stuck while it
    /// uploads happily, or says nothing about one that never will.
    pub needs_attention: bool,
}

/// How the last IO operation ended, reported by the shell as part of the next
/// tick's [`Observation`]. With the authority inverted, finishing an op is an
/// *input* to the reducer rather than an event that mutates state behind its
/// back: the shell says "the restore ended like this" and the reducer clears
/// `in_flight` and updates the bookkeeping. Maps 1:1 onto the dispositions the
/// engine already told apart (`AutoRestoreDisposition` plus `BackupDone`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpResult {
    /// Finished without error. `version` and `fingerprint` are the head we are
    /// now synced to.
    ///
    /// `wrote` is the commit-vs-no-op discriminant (ADR 0021 D.8.2): on a backup
    /// `true` means a new snapshot reached the server, on a restore it means
    /// files were applied. `false` is the no-op pass, skipped by signature, empty
    /// folder, archived save, too large, or already in sync, and the reducer
    /// treats it differently. A no-op does **not** anchor the min-interval
    /// (passing it off as a commit is the R.E.P.O. regression: the next real
    /// upload gets pushed out by a whole interval and a short session never
    /// flushes its progress) and does not stamp `last_restore_at`.
    ///
    /// A no-op backup *with* a `version` is the special case of the 409
    /// non-fast-forward that settled onto the remote head: nothing was committed,
    /// but the merge wrote into the folder the way a restore does, so the reducer
    /// does stamp `last_restore_at` there.
    Ok {
        version: Option<i64>,
        fingerprint: Option<u64>,
        wrote: bool,
    },
    /// 404, the save is not on the backend. Not a failure (retrying will not
    /// conjure a snapshot that is not there); parked on the long backoff.
    NotFound,
    /// 401, expired session, not the save's fault. Neither escalates nor resets
    /// the counter; short cooldown so it retries as soon as the token refreshes.
    Unauthorized,
    /// 429, bandwidth limit. Like 401 it leaves the failure counter alone
    /// (counting a throttle as "stuck" was exactly the notification spam bug).
    /// Symmetric between backup and restore.
    Throttled { retry_after_secs: u32 },
    /// 402, the **account** is out of room. Unlike a 429 there is no window that
    /// waits it out on its own: until the user frees space or upgrades, every
    /// upload hits the same wall. Parks the upload on a long rest
    /// ([`reconcile::QUOTA_FULL_BACKOFF_SECS`]) while keeping `has_pending`,
    /// since the bytes still live only on disk and clearing it would let a
    /// restore walk over them. Leaves the failure counter alone too: a full
    /// account is not a broken save.
    QuotaFull,
    /// 409 with **no way out**: the server says we are behind and reconciliation
    /// finds nothing to pull. Different from [`Self::Failed`] because the cure is
    /// different. An ordinary failure heals with time (the network comes back,
    /// the server boots) and deserves a flat backoff; this one heals with no
    /// amount of time, so it escalates through [`ConflictStall`] and past
    /// [`reconcile::CONFLICT_STALL_GIVE_UP_AFTER`] it stops retrying and asks for
    /// a human. Like [`Self::Failed`] on an upload it keeps `has_pending`: the
    /// changes are still unversioned.
    ConflictStalled,
    /// Anything else (network, sha, permissions, timeout) once the executor has
    /// burned its internal retries. What it does depends on the op in flight. On
    /// a **download** it escalates the per-cloud-version failure counter and the
    /// restore backoff. On an **upload** it re-arms the attempt on the long
    /// backoff ([`reconcile::BACKUP_FAILURE_BACKOFF_SECS`]) and keeps
    /// `has_pending`, because the changes never made it into a version and
    /// dropping them would let a restore overwrite them.
    Failed,
}

/// The kernel's own durable memory (the "spec/status" of ADR C.1): what the
/// reconciler remembers and cannot rebuild by looking at the world as it is now.
/// Distinct from the sampled world, which is [`Observation`].
///
/// Holds the save's resolved policy, live session status (durable, sticky across
/// ticks), the deferred-pull journal, the operation in flight, and the pacing
/// deadlines. Every deadline is compared against `world.now` to stay sans-IO.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct State {
    // ---- resolved policy
    /// Playtime-only entry: there is no folder to sync.
    pub track_only: bool,
    pub restore_enabled: bool,
    /// Floor between committing backups (ADR 0018, axis A). `0` means no floor.
    /// Measured from [`Self::last_backup_at`], which only advances on a real
    /// commit, so a no-op pass cannot push the floor out.
    pub min_backup_interval_secs: u64,

    // ---- live session status, durable across ticks
    /// The game process is running. Sticky: stays `true` through the grace
    /// window after it stops being visible, see
    /// [`reconcile::RUNNING_STICKY_GRACE_SECS`].
    pub is_running: bool,
    /// Last time the process was seen alive; the anchor for that stickiness.
    pub last_running_seen: Option<OffsetDateTime>,
    pub has_pending: bool,
    pub last_fs_event_at: Option<OffsetDateTime>,
    /// Last time *this* device restored into the folder. Its own touch, which
    /// must not veto the next pull.
    pub last_restore_at: Option<OffsetDateTime>,

    // ---- sync bookkeeping
    /// Cloud version this device is synced to. `None` until the first commit or
    /// restore.
    pub known_version: Option<i64>,
    /// Fingerprint of the local content already synced, uploaded or downloaded.
    /// Matching it against the observed fingerprint is what makes "converged
    /// means zero actions" true, and that is what killed the compression hot
    /// loop.
    pub synced_fingerprint: Option<u64>,
    /// Last backup that actually committed, and the anchor of the min-interval.
    /// Only an `OpResult::Ok { wrote: true }` moves it; letting a no-op move it
    /// would push the next real upload out by a whole interval (the R.E.P.O.
    /// regression).
    pub last_backup_at: Option<OffsetDateTime>,
    /// Start of the window this save's commits are counted in, and how many have
    /// landed inside it. This is the memory behind the adaptive floor: with no
    /// preset pinning an interval, a quiet save uploads as soon as the debounce
    /// settles, and one whose game rewrites its autosave every few seconds gets
    /// batched.
    ///
    /// It exists because a fixed floor for everyone was tried and had to come
    /// out: it was invisible and read as "it isn't noticing my changes". This one
    /// only shows up once the save itself proves it is needed.
    pub burst_since: Option<OffsetDateTime>,
    pub burst_backups: u32,

    // ---- operation in flight (anti-relaunch)
    /// A backup or restore is running; the tick must not launch it again.
    /// Cleared when the matching [`OpResult`] is ingested.
    pub in_flight: Option<Op>,

    // ---- pacing deadlines, compared against world.now
    /// No backup starts before this instant, because of an **error** backoff: a
    /// 429 on upload, or exhausted upload retries. `None` means no brake. The
    /// min-interval floor deliberately does not live here, it derives from
    /// [`Self::last_backup_at`] plus [`Self::min_backup_interval_secs`], so that
    /// saver pacing (which a cross-device flush may skip) stays distinguishable
    /// from an error backoff (which it never may).
    pub next_backup_at: Option<OffsetDateTime>,
    /// No restore starts before this instant: cooldown, failure backoff or
    /// download throttle backoff. `None` means no brake.
    pub next_restore_at: Option<OffsetDateTime>,

    // ---- deferred pull journal
    /// A cross-device update is waiting but a pull was vetoed mid-session. It
    /// survives the veto and lands when the game closes (the Deck bug).
    pub pull_pending: bool,
    /// The user has already been told about this waiting pull. De-duplicates
    /// **only the UI notification**, one per update rather than one per tick, and
    /// never the action itself: storing an action in an edge flag inside a
    /// level-triggered reducer is precisely the D.8.1 deadlock.
    pub deferred_notified: bool,

    // ---- failure escalation
    pub restore_failures: RestoreFailures,
    /// Counter and escalation for the 409 reconciliation cannot resolve. The
    /// brake that stops that case retrying forever.
    pub backup_conflict: ConflictStall,
}

/// The world as sampled this tick (ADR C.1): what the shell read off the disk,
/// the OS and the server, handed to the kernel as plain data. Observation is
/// **tiered**:
///
/// - **L0**, cheap, every tick: `folder_mtime`, `folder_size`, `local_empty`.
/// - **L1**, only when there is a signal (L0 moved, or a hint pointed at this
///   save): `local_fingerprint`, the hash of the local set. Never re-hash
///   everything every tick.
/// - **Process evidence**: is the game's process alive this tick?
/// - **Server head**: `cloud_version`, which the engine shell polls itself on an
///   interval. The client-side poller is a latency hint, not the only source
///   (ADR 0021 D.12).
/// - **One-shot signals**: `fs_event`, `op_result`, `upload_landed`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observation {
    // ---- L0
    /// The save folder's **own** mtime (its inode, `metadata(path)`, not
    /// recursive), or `None` if it could not be read.
    pub folder_mtime: Option<OffsetDateTime>,
    pub folder_size: Option<u64>,
    /// The local folder is empty or missing, which triggers restore-into-empty.
    pub local_empty: bool,

    // ---- L1, only on a signal
    /// Hash of the local content, computed only when L0 moved or a hint pointed
    /// at this save. `None` means nothing was hashed this tick.
    pub local_fingerprint: Option<u64>,

    // ---- process evidence
    pub process_alive: bool,
    /// Some file in the save is **held open exclusively by another process**:
    /// the game is writing right now.
    ///
    /// This is independent of the process table, and that is what makes it worth
    /// having: it does not depend on recognising the executable, so it covers
    /// the game whose name matches nothing and whose correlation does not exist
    /// yet. Copying the folder at that moment captures a half-written save;
    /// restoring over it is worse.
    ///
    /// Only Windows can assert it today (`ERROR_SHARING_VIOLATION`). On Linux and
    /// macOS a read `open()` never fails just because another process is
    /// writing, so this arrives `false` and the usual guards decide. See
    /// `hoard_agent::locks`.
    pub save_files_locked: bool,

    // ---- server head
    /// Latest cloud version known for this save. `None` means unknown:
    /// self-hosted without a poller, or before the first poll.
    pub cloud_version: Option<i64>,
    /// **Since when** [`Self::cloud_version`] has been the truth: the instant of
    /// the last cloud poller feed. Without this stamp the kernel cannot tell
    /// "converged" from "blind", since both look like `Hold{"converged"}`, and a
    /// dead poller passes for normality. That is ADR 0021 D.10: the poller went
    /// quiet and 47 minutes of silence looked healthy. Once it ages past
    /// [`reconcile::CLOUD_STALE_AFTER_SECS`] the reducer says so out loud.
    ///
    /// `None` means **no feed has arrived yet**, and on its own it means nothing.
    /// What decides whether that is normality or blindness is
    /// [`Self::cloud_feed_expected_since`]. It stamps the *feed*, not the save:
    /// the poller brings the whole manifest, so a save missing from the manifest
    /// has `cloud_version: None` with the feed stamp just as fresh.
    pub cloud_version_as_of: Option<OffsetDateTime>,
    /// Since when this deployment **expects** cloud heads at all: the instant the
    /// engine started watching. `None` means there is no cloud to watch
    /// (self-hosted, a headless CLI daemon, or a context not resolved yet), and
    /// then there is no feed to go stale.
    ///
    /// It exists because of the loose end in ADR 0021 D.11: with only
    /// [`Self::cloud_version_as_of`], the worst blindness of all, "I have never
    /// heard anything from the cloud", was indistinguishable from a deployment
    /// that legitimately has no feed, so a `None` slipped through as `converged`.
    /// The right distinction is not `None` versus `Some`, it is cloud context
    /// versus self-hosted: with a cloud context and no stamp, staleness is
    /// measured from here (the startup allowance), and past
    /// [`reconcile::CLOUD_STALE_AFTER_SECS`] it gets called out like any stale
    /// feed.
    pub cloud_feed_expected_since: Option<OffsetDateTime>,

    // ---- one-shot signals
    /// A debounced write landed in the folder this tick. A hint that pulls the
    /// tick forward and marks `has_pending`.
    pub fs_event: bool,
    pub op_result: Option<OpResult>,
    /// Result of the content-addressed anti-relaunch check. `Some(true)` means
    /// the content of the upload in flight already landed on the server (it
    /// exists in `blobs`/`chunks`), so there is nothing to re-upload. `None`
    /// means it was not checked.
    pub upload_landed: Option<bool>,
}

/// Something the kernel asks the IO shell to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Pull permission from the session-veto sub-decider
    /// ([`session::mid_session_decision`]): "the slot is quiet, a pull *may*
    /// proceed". It is that sub-decider's output, not a high-level command; the
    /// high-level reconciler uses [`Action::Restore`].
    Pull,
    Backup,
    Restore,
    /// Record that a cross-device pull is waiting while we are mid-session; it
    /// runs when the game closes. The shell also tells the user once that an
    /// update is queued.
    DeferPull,
    /// Throttle backoff after a 429: the shell does not retry the op until
    /// `until`. Symmetric between backup and restore; the deadline also lives in
    /// [`State::next_backup_at`] / [`State::next_restore_at`].
    Throttle {
        until: OffsetDateTime,
    },
}

/// The kernel's first-class decision (ADR C.5): either act, or hold with an
/// explicit reason. The veto stops living inside "did nothing" and becomes a
/// `Hold` with a reason you can assert on and log.
///
/// The reason is a `&'static str` on purpose. The one dynamic thing a `Hold`
/// might want, the "until {t}" of a throttle, lives in
/// [`State::next_restore_at`] / [`State::next_backup_at`], its natural home in
/// the durable pacing memory, and in [`Action::Throttle`]. That satisfies D.6
/// ("a `HoldReason` type only *if* some reason needs dynamic data") without
/// breaking the Slice 1 session tests, which match against these strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Act(Action),
    Hold { reason: &'static str },
}

impl Decision {
    pub fn is_act(&self) -> bool {
        matches!(self, Decision::Act(_))
    }

    pub fn action(&self) -> Option<&Action> {
        match self {
            Decision::Act(a) => Some(a),
            Decision::Hold { .. } => None,
        }
    }
}
