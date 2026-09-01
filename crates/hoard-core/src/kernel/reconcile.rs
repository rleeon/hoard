//! The pure reconciling reducer (ADR 0021, C.1 and C.2).
//!
//! ```text
//! reconcile(&State, &Observation, World) -> (State, Vec<Decision>)
//! ```
//!
//! Deterministic and sans-IO: every bit of non-determinism comes in through
//! [`World`] (`now`, `seed`). Authority is inverted, so the tick is the source of
//! truth. Each tick the shell samples the world, builds an [`Observation`], calls
//! `reconcile`, and runs the [`Decision`]s. Events from the watcher or realtime
//! are hints that only pull a tick forward (they arrive as `obs.fs_event` and
//! `obs.op_result`); they never decide anything on their own.
//!
//! The session veto composes by reusing [`session::veto_reason`]. `reconcile` is
//! the high-level reconciler; the veto is its sub-decider.
//!
//! ## Invariants (property tests with shrinking, further down)
//! - converged means `Hold` only, and zero `Act`.
//! - no `Act` without a delta in the input to cause it (`now` crossing a deadline
//!   *is* a delta, so the retry after a 429 does not violate this).
//! - never `Act(Backup)` and `Act(Restore)` at once (they must not fight over the
//!   folder), and never `Act(Restore)` mid-session (the R.E.P.O. data loss).
//! - never lose a local newer than the remote (`Restore` implies no
//!   `has_pending`).
//! - at most one storage `Act` per tick.
//! - a deferred pull never stalls the upload that would unblock it (D.8.1).

use rand::{rngs::StdRng, Rng, SeedableRng};
use time::{Duration, OffsetDateTime};

use super::{
    session, Action, ConflictStall, Decision, Observation, Op, OpResult, RestoreFailures, State,
    World,
};

// ---- pacing constants (sans-IO twins of the ones in `agent.rs`)

/// Minimum cooldown between restore attempts, success or failure. Matches
/// `agent::AUTO_RESTORE_COOLDOWN_SECS`.
pub const RESTORE_COOLDOWN_SECS: i64 = 60;

/// Long backoff when a restore 404s, meaning the save is not on the backend.
/// Matches `agent::AUTO_RESTORE_NOT_FOUND_BACKOFF_SECS`.
pub const NOT_FOUND_BACKOFF_SECS: i64 = 60 * 60;

/// Escalation for a restore that keeps failing against the SAME cloud version:
/// 60 s, 5 min, 15 min, 60 min, then 60 min forever. Matches
/// `agent::AUTO_RESTORE_FAILURE_BACKOFF_SECS`.
pub const FAILURE_BACKOFF_SECS: [i64; 4] = [60, 5 * 60, 15 * 60, 60 * 60];

/// Consecutive failures on one version before a save is called stuck.
pub const STUCK_AFTER: u32 = 3;

/// Long backoff after an *upload* burns its internal retry budget. Ten minutes,
/// deliberately far slower than that budget (seconds): what survives the retries
/// is not a lost packet but a real fault, a downed server, no network, an
/// unreadable disk, an expired token, and those resolve on the scale of minutes
/// or hours. Long enough not to hammer a dead backend or paint the feed red,
/// short enough that recovery is unattended. This was `agent::BACKUP_RETRY_BACKOFF`,
/// policy living in the shell (ADR 0021 D.8.2).
pub const BACKUP_FAILURE_BACKOFF_SECS: i64 = 10 * 60;

/// Escalation for an upload hitting an *unresolvable* conflict (409 "you are
/// behind" plus a reconcile with nothing to pull): 10, 20, 40, 80 minutes.
///
/// Exponential rather than flat like [`BACKUP_FAILURE_BACKOFF_SECS`], because it
/// is not the same fault. An ordinary failure heals on its own (the network
/// comes back, the server boots) and the backoff only has to avoid hammering
/// meanwhile. A conflict with no way out does not heal with time: every retry
/// asks the same question and gets the same answer. Retrying is worth it in case
/// the cloud moves, and that happens on the scale of a play session, not of ten
/// minutes.
pub const CONFLICT_STALL_BACKOFF_SECS: [i64; 4] = [10 * 60, 20 * 60, 40 * 60, 80 * 60];

/// Consecutive unresolvable conflicts against the same cloud head after which
/// retrying stops and the save asks for a person.
///
/// Five: the four rungs above, two and a half hours in total, and that is
/// enough. The real case ran for 14 days at about 4.5 attempts an hour and
/// survived three releases of the app with nobody looking at it, because nothing
/// showed it. An infinite silent retry is not fault tolerance, it is a hidden
/// fault.
pub const CONFLICT_STALL_GIVE_UP_AFTER: u32 = 5;

/// The `Hold` reason for an upload that spent its conflict budget. The UI shows
/// it as "this needs you to look at it", so it is a constant rather than a
/// literal, same as [`HOLD_BACKUP_MIN_INTERVAL`].
pub const HOLD_BACKUP_NEEDS_ATTENTION: &str = "backup conflict needs the user";

/// Rest after a 402 (account full). Far longer than an ordinary failure's:
/// freeing space is a human action (archiving games, upgrading), not a network
/// blip that clears itself in ten minutes. With twenty saves in the library, the
/// ordinary failure backoff would be about 120 POSTs an hour that we already
/// know will come back 402.
pub const QUOTA_FULL_BACKOFF_SECS: i64 = 60 * 60;

/// Ceiling on the wait a 429 can ask this client to sit out.
///
/// The cap exists so a malformed or hostile `retry_after` can't park a save
/// until the next restart, not to second-guess our own server. It used to be
/// 300 s, which did second-guess it: `loopguard::QUOTA_WAIT_SECS` answers a full
/// account with 3600, the same hour as [`QUOTA_FULL_BACKOFF_SECS`], picked so
/// both ends behave alike, and the client silently shortened it to five
/// minutes. One account spent four days at ~170 refusals an hour against a
/// brake that had already told it to come back in one.
///
/// Derived from `QUOTA_FULL_BACKOFF_SECS` rather than written as another 3600:
/// the two numbers mean the same thing (how long a wall a human has to move
/// stays a wall) and drifting apart would put the loop straight back.
pub const MAX_THROTTLE_WAIT_SECS: i64 = QUOTA_FULL_BACKOFF_SECS;

/// Fixed cadence of the airbag poll to `/v1/cloud/sync`. The source of truth for
/// the number: `hoard_agent::prefs::CLOUD_POLL_INTERVAL_SECS` re-exports it, so
/// the staleness threshold below derives from the real cadence instead of
/// duplicating a literal that can drift.
pub const CLOUD_POLL_INTERVAL_SECS: i64 = 60;

/// How many poll intervals may be missed before cloud observation is declared
/// blind. Five: one or two failures in a row are a network hiccup or a short
/// suspend and do not deserve noise, while five minutes without contact is no
/// longer a hiccup but a fault (ADR 0021 D.10, where the poller died and went 47
/// minutes without anything saying so).
pub const CLOUD_STALE_AFTER_POLLS: i64 = 5;

// A single missed poll can never declare the observation blind, because networks
// hiccup. Checked at compile time rather than in a test, so it cannot even build
// wrong.
const _: () = assert!(CLOUD_STALE_AFTER_POLLS >= 2);

/// The age past which [`Observation::cloud_version_as_of`] stops being credible
/// and the reducer emits [`CLOUD_STALE_REASON`] instead of `"converged"`.
pub const CLOUD_STALE_AFTER_SECS: i64 = CLOUD_POLL_INTERVAL_SECS * CLOUD_STALE_AFTER_POLLS;

/// The `Hold` reason when the cloud version cache has aged out: we are not
/// converged, we are blind. Same principle as logging vetoes, which is that an
/// invisible failure becomes an observable one.
pub const CLOUD_STALE_REASON: &str = "cloud state stale";

/// The age of [`Observation::cloud_version_as_of`] past which the engine goes and
/// fetches the cloud head itself (ADR 0021 D.12), rather than waiting for the
/// client's poller to push it.
///
/// Above the poller's cadence on purpose: a live poller refreshes the stamp
/// before this is reached, so its feed *suppresses* the query and the cost stays
/// at one manifest per interval rather than two. When the poller dies, which was
/// the D.12 fault where the task vanished without a log, the engine covers the
/// gap on its own and the degradation is "I take until the next tick" rather
/// than "blind forever". It lives here, next to the cadence it derives from, even
/// though the shell is what makes the query (the kernel does no IO).
pub const CLOUD_SELF_OBSERVE_AFTER_SECS: i64 = CLOUD_POLL_INTERVAL_SECS * 3 / 2;

// The engine ALWAYS tries to refresh before declaring itself blind. If this
// relation were inverted, the `Hold{"cloud state stale"}` would be accusing a
// staleness nobody has tried to fix yet. Checked at compile time.
const _: () = assert!(CLOUD_SELF_OBSERVE_AFTER_SECS < CLOUD_STALE_AFTER_SECS);
// And never below the poller's cadence, or a healthy poller and the engine would
// tread on each other's manifest every interval: two GETs where there should be
// one.
const _: () = assert!(CLOUD_SELF_OBSERVE_AFTER_SECS >= CLOUD_POLL_INTERVAL_SECS);

/// Sticky grace window after the process stops being seen, before it is declared
/// stopped. Six seconds, down from the historical 90
/// (`agent::STRONG_STOP_GRACE_FLOOR_SECS`): since the session veto anchors on
/// `is_running`, those 90 seconds were added to EVERY GameStopped, inflating both
/// close-detection latency and cross-device restore latency, because the
/// receiving machine kept vetoing pulls for 90 seconds after the game closed.
pub const RUNNING_STICKY_GRACE_SECS: i64 = 6;

// ---- the reducer

/// Reconciles the durable state with the sampled world and returns the new state
/// plus the decisions to run this tick. Deterministic: the same inputs give the
/// same output, jitter included, via `StdRng::seed_from_u64`.
pub fn reconcile(state: &State, obs: &Observation, world: World) -> (State, Vec<Decision>) {
    let mut next = state.clone();
    let mut decisions: Vec<Decision> = Vec::new();
    let now = world.now;

    // A playtime-only entry has no folder to sync, ever.
    if next.track_only {
        return (next, vec![hold("track-only entry")]);
    }

    // The fs hint (a debounced write landed this tick) marks pending. It is a
    // hint: it pulls the tick forward, it does not decide.
    if obs.fs_event {
        next.has_pending = true;
        next.last_fs_event_at = Some(now);
    }

    // Live session status from process evidence, with stickiness.
    apply_running_stickiness(&mut next, obs, now);

    // The cloud published a different version from the one that kept failing, so
    // that is new information rather than a retry: the failure escalation dies
    // and the brake comes off (D.8.2). The shell used to do this on
    // `SetCloudVersions`, which was policy outside the kernel and invisible to
    // the C.5 replay.
    clear_restore_backoff_on_new_version(&mut next, obs);
    clear_conflict_stall_on_new_version(&mut next, obs);

    // Ingest the result of an in-flight op that just finished. Clears
    // `in_flight` and updates the bookkeeping and backoff. May emit `Throttle`.
    if let Some(result) = obs.op_result {
        ingest_op_result(&mut next, result, obs, now, world.seed, &mut decisions);
    }

    // Anti-relaunch: if there is still an op in flight (no result this tick), do
    // NOT relaunch. Moving GBs takes minutes. Hold, with a reason.
    if next.in_flight.is_some() {
        decisions.push(hold("operation in flight"));
        return (next, decisions);
    }

    // ---- restore decision (cloud to local)
    // We restore when the local folder is empty (uninstalled, or fresh), when the
    // cloud is ahead (another device uploaded a higher version), or when a
    // deferred pull is left over from an earlier tick. `cloud_ahead` may have
    // stopped being provable from the cache, but `pull_pending` remembers the
    // intent: the pull survives the veto and lands when the game closes.
    let ahead = cloud_ahead(&next, obs);
    let want_restore = next.restore_enabled && (obs.local_empty || ahead || next.pull_pending);
    if want_restore {
        // Restore cooldown or backoff still active. The 429 after a throttle
        // lands here, and `now` crossing the deadline is the delta that frees it.
        let cooling = next.next_restore_at.is_some_and(|t| now < t);
        if cooling {
            decisions.push(hold("restore cooldown"));
        } else {
            match session::veto_reason(&next, obs, &world) {
                // Mid-session: never pull into a live folder (the R.E.P.O. data
                // loss). If there is a real update waiting, the pull is DEFERRED
                // rather than lost.
                Some(reason) => {
                    if ahead || next.pull_pending {
                        next.pull_pending = true;
                        // `deferred_notified` de-duplicates ONLY the UI notice,
                        // never the action. Storing the *action* in an edge flag
                        // inside a level-triggered reducer is the one-shot that
                        // stalled the (has_pending, cloud_ahead) pair (D.8.1).
                        if next.deferred_notified {
                            decisions.push(hold(reason));
                        } else {
                            next.deferred_notified = true;
                            decisions.push(Decision::Act(Action::DeferPull));
                        }
                    } else {
                        decisions.push(hold(reason));
                    }
                }
                // Quiet: restore now.
                None => {
                    start_restore(&mut next, now);
                    decisions.push(Decision::Act(Action::Restore));
                    return (next, decisions);
                }
            }
        }
        // The pull does not proceed this tick (cooldown or veto), but the backup
        // still can. Only an upload clears `has_pending`, so returning here left
        // the slot stalled for as long as the cloud was ahead: the veto looks at
        // `has_pending`, and `has_pending` was waiting on a backup that never got
        // emitted. That was the deadlock the `DeferPull` executor used to break by
        // hand in the shell, policy outside the kernel (D.8.1). A mid-session
        // backup is the feature (debounced autobackup while you play), not a bug:
        // the hard invariant is that nothing gets restored, not that nothing gets
        // uploaded. And it is *urgent*: until it lands, the pull stays vetoed.
        let urgent = ahead || next.pull_pending;
        if let Some(d) = decide_backup(&mut next, obs, now, urgent) {
            decisions.push(d);
        }
        return (next, decisions);
    }

    // ---- backup decision (local to cloud)
    // Converged if there is nothing to upload, so nothing to do (the base C.1
    // invariant). Unless we are not converged but *blind*: if the cloud
    // observation aged out, or never arrived while there was a cloud to observe,
    // then `cloud_version` is a lying input and the `cloud_ahead = false` above
    // proves nothing. That gets said with its own reason (ADR 0021 D.10), so the
    // poller's failure stops passing for normality. Only the rest's reason
    // changes; the upload is untouched, because letting a dead poller stop
    // backups would trade an invisible failure for data loss.
    let idle = if cloud_state_stale(obs, now) {
        hold(CLOUD_STALE_REASON)
    } else {
        hold("converged")
    };
    decisions.push(decide_backup(&mut next, obs, now, false).unwrap_or(idle));
    (next, decisions)
}

// ---- pure helpers

fn hold(reason: &'static str) -> Decision {
    Decision::Hold { reason }
}

/// Decides the local-to-cloud upload, split out so it can also be taken when the
/// pull does not proceed (see the D.8.1 deadlock). Returns:
///
/// - `Some(Act(Backup))` with a REAL content delta (a fingerprint different from
///   the synced one) and pacing satisfied, marking the op in flight;
/// - `Some(Hold(...))` when a pacing brake has not expired yet;
/// - `None` when there is nothing to upload, leaving the caller to pick a reason.
///
/// Demanding real divergence is what kills the compression hot loop: a spurious
/// `has_pending` with identical content does not upload, because converged means
/// zero actions.
///
/// `urgent` means this upload is the flush that unblocks a cross-device pull
/// (the cloud is ahead, or a deferred pull is waiting). Only then does it skip
/// the data-saving floor, and never an error backoff.
fn decide_backup(
    next: &mut State,
    obs: &Observation,
    now: OffsetDateTime,
    urgent: bool,
) -> Option<Decision> {
    if !(next.has_pending && local_diverged(next, obs)) {
        return None;
    }
    // The upload spent its budget of unresolvable conflicts: it stops and asks
    // for a person. Ahead of any pacing brake because it is not a pacing brake.
    // There is no deadline to cross that lifts it, only a user action, a
    // successful backup, or a new cloud head.
    if next.backup_conflict.needs_attention {
        return Some(hold(HOLD_BACKUP_NEEDS_ATTENTION));
    }
    // The game is writing the save right now, so uploading would capture a
    // half-written file. This is a pacing brake, not an error: as soon as it lets
    // go of the file, the next tick uploads. Ahead of the backoffs because it is
    // more specific, and gives the real reason instead of "waiting".
    if obs.save_files_locked {
        return Some(hold("save files are open in another process"));
    }
    // Error backoff (an upload 429, or exhausted backup retries): never skipped.
    // Skipping it means hammering a dead backend or burning the quota.
    if next.next_backup_at.is_some_and(|t| now < t) {
        return Some(hold("backup backoff"));
    }
    // The min-interval floor (data saving, ADR 0018 axis A) is pacing, not error.
    // A flush that unblocks a pull may skip it; otherwise local progress stays
    // unversioned, the `has_pending` veto stands, and the cross-device update
    // waits a whole interval (up to 10 minutes on the `data_saver` preset) before
    // it can land.
    if !urgent && backup_floor(next).is_some_and(|t| now < t) {
        // Two different reasons on purpose: one is the pace the user chose, the
        // other is the one we imposed because of how their game behaves. In a log
        // they look the same, right up until somebody has to be told why their
        // save is "slow", and then they mean very different things.
        return Some(hold(if next.min_backup_interval_secs > 0 {
            HOLD_BACKUP_MIN_INTERVAL
        } else {
            HOLD_BACKUP_BURST
        }));
    }
    next.in_flight = Some(Op::Backup);
    Some(Decision::Act(Action::Backup))
}

/// The two hold reasons that mean "there is something to upload and it will go up
/// shortly", as against the ones that mean "it cannot be uploaded" (error
/// backoff, open file). Constants rather than literals because the shell decides
/// off them whether to show the wait in the UI, and a reason renamed here and not
/// there puts the floor back to being invisible, which is exactly why the first
/// attempt had to be reverted.
pub const HOLD_BACKUP_MIN_INTERVAL: &str = "backup min-interval";
pub const HOLD_BACKUP_BURST: &str = "backup autosave burst";

/// Is this reason a wait with a time on it that the UI should be able to show?
pub fn hold_is_paced_backup(reason: &str) -> bool {
    reason == HOLD_BACKUP_MIN_INTERVAL || reason == HOLD_BACKUP_BURST
}

/// Window over which a save's commits are counted to decide whether the game is
/// rewriting its autosave in a loop.
pub const BURST_WINDOW_SECS: i64 = 600;
/// Commits inside that window from which the floor is imposed. Three in ten
/// minutes is already more than any history makes use of.
pub const BURST_THRESHOLD: u32 = 3;
/// The floor imposed then, and the only rung there is: it does not scale with
/// frequency. A game autosaving every six seconds goes from one version every six
/// seconds to one a minute, and stays there.
pub const BURST_FLOOR_SECS: u64 = 60;

/// The floor that actually governs this save.
///
/// An explicit interval always wins: a preset the user can see and chose put it
/// there (`short_session` at 30 s for a game that wipes its folder between
/// rounds, `data_saver` at 600 s), and raising it unasked would betray exactly
/// what was requested. The adaptive one only fills the "no floor at all" gap,
/// which is the default and until now meant literally none: one save reached
/// 2,233 versions in a day and 1,027 uploads in four and a half hours, because
/// the game rewrote `auto.sav` every few seconds and every rewrite was a version
/// in the cloud (aug-2026).
fn effective_min_interval(state: &State) -> u64 {
    if state.min_backup_interval_secs > 0 {
        return state.min_backup_interval_secs;
    }
    if state.burst_backups >= BURST_THRESHOLD {
        BURST_FLOOR_SECS
    } else {
        0
    }
}

/// The min-interval floor, *derived* from `last_backup_at` plus
/// [`effective_min_interval`] rather than stored in `next_backup_at`. Keeping it
/// apart from the backoff is what makes "saver pacing" (skippable by a
/// cross-device flush) distinguishable from "error backoff" (never), and it makes
/// the anchor, `last_backup_at`, which only advances on a real commit, the single
/// memory of the floor: a no-op cannot push it (the R.E.P.O. regression, D.8.2).
///
/// Public because the shell needs the same number to *show* it: a wait nobody
/// can see reads as "Hoard isn't picking up my changes", which is why the first
/// attempt at a fixed floor had to be reverted. The shell asks for the deadline
/// and puts it in `next_scheduled_backup_at`, where the UI's "next copy in Xs"
/// already reads from.
pub fn backup_floor(state: &State) -> Option<OffsetDateTime> {
    let secs = effective_min_interval(state);
    if secs == 0 {
        return None;
    }
    state
        .last_backup_at
        .map(|t| t + Duration::seconds(secs as i64))
}

/// Counts this commit into the burst window, opening a fresh one if the previous
/// has expired. Called ONLY with a real commit, for the same reason as
/// `last_backup_at`: a no-op is not the game being active and cannot push a quiet
/// save onto the adaptive floor.
fn count_burst(state: &mut State, now: OffsetDateTime) {
    let open = state
        .burst_since
        .is_some_and(|t| now - t <= Duration::seconds(BURST_WINDOW_SECS));
    if open {
        state.burst_backups = state.burst_backups.saturating_add(1);
    } else {
        state.burst_since = Some(now);
        state.burst_backups = 1;
    }
}

/// Releases the restore failure escalation when the cloud publishes a version
/// different from the one it was failing against (D.8.2). The backoff was about
/// *that* version; a new one is new content and a fresh reason to try now, not to
/// inherit the penalty. It only acts on a live escalation, so it does not tread
/// on the ordinary post-restore cooldown.
fn clear_restore_backoff_on_new_version(next: &mut State, obs: &Observation) {
    let active = next.restore_failures.consecutive > 0 || next.restore_failures.stuck_notified;
    if active && next.restore_failures.version != obs.cloud_version {
        next.restore_failures = RestoreFailures::default();
        next.next_restore_at = None;
    }
}

/// Starts a restore: marks the op in flight and arms the cooldown. A pending
/// deferred pull counts as consumed, since we are executing it.
fn start_restore(next: &mut State, now: OffsetDateTime) {
    next.in_flight = Some(Op::Restore);
    next.next_restore_at = Some(now + Duration::seconds(RESTORE_COOLDOWN_SECS));
    next.pull_pending = false;
    next.deferred_notified = false;
}

/// Does the poller's cache say the save moved past what this device holds? A
/// cached version with no `known_version` counts as ahead, because we never
/// synced this save. With no cache entry we do not know, and never claim it.
/// Twin of `agent::cloud_ahead`.
fn cloud_ahead(state: &State, obs: &Observation) -> bool {
    match obs.cloud_version {
        Some(latest) => state.known_version.is_none_or(|known| latest > known),
        None => false,
    }
}

/// Has the cloud observation stopped being credible? `true` when the latest thing
/// we know about it is older than [`CLOUD_STALE_AFTER_SECS`].
///
/// Two ways to be blind, one countdown:
///
/// - A stale feed. There were heads and they stopped arriving, so it ages from
///   the stamp ([`Observation::cloud_version_as_of`]).
/// - No feed ever. The worst blindness, and the one that slipped through as
///   `converged` until D.11 was finished off. It ages from
///   [`Observation::cloud_feed_expected_since`], the moment the engine started
///   expecting heads. Without that anchor (self-hosted, a CLI daemon, an
///   unresolved context) there is no cloud to observe and nothing gets reported:
///   the distinction is cloud context versus self-hosted, not `None` versus
///   `Some`.
///
/// A `now` earlier than the anchor (a clock jumping backwards) is not staleness
/// either, since the subtraction comes out negative.
fn cloud_state_stale(obs: &Observation, now: OffsetDateTime) -> bool {
    let anchor = match obs.cloud_version_as_of {
        Some(as_of) => Some(as_of),
        None => obs.cloud_feed_expected_since,
    };
    anchor.is_some_and(|t| (now - t).whole_seconds() > CLOUD_STALE_AFTER_SECS)
}

/// Does the local content differ from what is already synced? With an L1
/// fingerprint computed it compares; without one (nothing was hashed this tick)
/// it trusts `has_pending`, since the fs hint said something changed. The
/// `Some(fp) == synced` case is what makes converged mean zero actions even when
/// `has_pending` was set by a spurious settle.
fn local_diverged(state: &State, obs: &Observation) -> bool {
    match obs.local_fingerprint {
        Some(fp) => state.synced_fingerprint != Some(fp),
        None => true,
    }
}

/// Derives `is_running` (durable status) from process evidence with a sticky
/// grace window. A correlation match is CPU-gated and can drop below the
/// threshold for one tick, and without grace that flaps GameStarted and
/// GameStopped. Keeps the slot running until `last_running_seen` is older than
/// [`RUNNING_STICKY_GRACE_SECS`].
fn apply_running_stickiness(next: &mut State, obs: &Observation, now: OffsetDateTime) {
    if obs.process_alive {
        next.is_running = true;
        next.last_running_seen = Some(now);
    } else if next.is_running {
        let expired = next
            .last_running_seen
            .is_none_or(|seen| (now - seen).whole_seconds() >= RUNNING_STICKY_GRACE_SECS);
        if expired {
            next.is_running = false;
        }
    }
}

/// Ingests a finished op's result: clears `in_flight` and applies the
/// disposition. Maps 1:1 onto `agent`'s `AutoRestoreDisposition` plus
/// `BackupDone`. The 429 (`Throttled`) is symmetric between backup and restore:
/// it brakes the right op and leaves the failure counter alone. `Failed` also
/// distinguishes the op, since a failed upload re-arms on its own long backoff
/// rather than escalating the restore ladder.
fn ingest_op_result(
    next: &mut State,
    result: OpResult,
    obs: &Observation,
    now: OffsetDateTime,
    seed: u64,
    decisions: &mut Vec<Decision>,
) {
    let op = next.in_flight.take();
    match result {
        OpResult::Ok {
            version,
            fingerprint,
            wrote,
        } => {
            // A restore can come back `Ok` without moving anything: the snapshot
            // is downloaded, diffed against the folder, and the diff decides
            // there is nothing to write. If the folder is still empty on top of
            // that, the trigger that brought it (`local_empty`, which
            // deliberately bypasses the version gate) is still true next tick and
            // we download the same snapshot again. Forever, and at the full price
            // of the download: one client ate 3,752 downloads and 10.6 GB between
            // 2026-07-27 and 08-03 without writing a byte to disk.
            //
            // The failure escalation is the only thing that can brake that, so a
            // "success" that makes no progress must not clear it. `!wrote` plus an
            // empty folder is the only unambiguous combination: if something was
            // written there was progress, even if the observation arrives late.
            let restore_stalled = matches!(op, Some(Op::Restore)) && !wrote && obs.local_empty;
            if !restore_stalled {
                next.restore_failures = RestoreFailures::default();
            }
            if version.is_some() {
                next.known_version = version;
            }
            if fingerprint.is_some() {
                next.synced_fingerprint = fingerprint;
            }
            match op {
                Some(Op::Backup) => {
                    // The content reached a version, or already was in one, so
                    // either way the changes stop being unversioned.
                    next.has_pending = false;
                    // And commit or no-op, the upload is no longer stuck: the
                    // unresolvable 409 resolved. The whole escalation is released,
                    // which is also what turns the warning off in the UI.
                    next.backup_conflict = ConflictStall::default();
                    if wrote {
                        // A real commit moves the min-interval anchor (ADR 0018).
                        // The floor derives from it ([`backup_floor`]); there is
                        // no need, and no benefit, to writing it into
                        // `next_backup_at`, which is the error backoff lane.
                        next.last_backup_at = Some(now);
                        count_burst(next, now);
                    } else {
                        // A no-op (skipped by signature, empty, archived, too
                        // large, the 409 settled onto the head, or an upload that
                        // had already landed) is not a backup, so it does not move
                        // the min-interval anchor. Moving it would push the next
                        // real upload out by a whole interval and a short session
                        // would never flush its progress (the R.E.P.O. regression,
                        // D.8.2).
                        //
                        // A no-op WITH a version is normally the 409
                        // non-fast-forward settled onto the head: the merge wrote
                        // into the folder just as a restore does, so
                        // `last_restore_at` gets stamped and that touch of ours
                        // does not veto the next pull.
                        //
                        // Unless the content-addressed check says the content was
                        // already up there (D.8.3). Then not a single byte was
                        // written to the folder, and stamping a touch that never
                        // happened would falsify the veto's grace window: the
                        // kernel would believe itself the author of somebody
                        // else's write and let through a pull that should wait.
                        if version.is_some() && obs.upload_landed != Some(true) {
                            next.last_restore_at = Some(now);
                        }
                    }
                }
                Some(Op::Restore) => {
                    // Only a real write touches the folder and should stamp
                    // `last_restore_at`, which avoids self-vetoing the next pull.
                    if wrote {
                        next.last_restore_at = Some(now);
                    }
                    next.pull_pending = false;
                    next.deferred_notified = false;
                    // A download that made no progress escalates up the same
                    // ladder as a failure (60 s, 5 min, 15 min, 60 min). It is not
                    // an error, since the server answered, but repeating it every
                    // tick is not syncing either, and the empty folder will ask
                    // for it again anyway. A new cloud version resets the
                    // escalation through `clear_restore_backoff_on_new_version`,
                    // so a legitimate later pull is not punished.
                    if restore_stalled {
                        let delay = record_failure(&mut next.restore_failures, obs.cloud_version);
                        next.next_restore_at = Some(now + Duration::seconds(delay));
                    }
                }
                None => {}
            }
        }
        // 404: park on the long backoff (a restore concept).
        OpResult::NotFound => {
            next.next_restore_at = Some(now + Duration::seconds(NOT_FOUND_BACKOFF_SECS));
        }
        // 401: not the save's fault. Short cooldown, counter untouched.
        OpResult::Unauthorized => {
            next.next_restore_at = Some(now + Duration::seconds(RESTORE_COOLDOWN_SECS));
        }
        // 429: symmetric backoff by op; failure counter untouched.
        OpResult::Throttled { retry_after_secs } => {
            let until = throttle_until(now, retry_after_secs, seed);
            match op {
                Some(Op::Backup) => next.next_backup_at = Some(until),
                _ => next.next_restore_at = Some(until),
            }
            decisions.push(Decision::Act(Action::Throttle { until }));
        }
        // 402: the account is full. It brakes uploads only, since a download
        // consumes no quota and a pending restore should carry on, and it leaves
        // `has_pending` alone so the slot stays vetoed from pulls while the
        // changes are only on disk.
        OpResult::QuotaFull => {
            if matches!(op, Some(Op::Backup)) {
                next.next_backup_at = Some(now + Duration::seconds(QUOTA_FULL_BACKOFF_SECS));
            }
        }
        // 409 with no way out: the server says we are behind and there is nothing
        // to pull. It escalates up its own ladder and, once spent, stops
        // retrying: `needs_attention` is what `decide_backup` reads to stop
        // emitting the upload. Like an upload's `Failed`, it keeps `has_pending`,
        // because the local changes are still unversioned.
        OpResult::ConflictStalled => {
            if let Some(delay) = record_conflict(&mut next.backup_conflict, obs.cloud_version) {
                next.next_backup_at = Some(now + Duration::seconds(delay));
            }
        }
        // Any other error, depending on the op:
        // - upload: it spent its internal retry budget, so it re-arms on the long
        //   backoff and KEEPS `has_pending` (the changes never reached a version,
        //   and dropping them would let a restore walk over them). The shell used
        //   to do this in `RetryBackupAfterFailure` (D.8.2).
        // - download (or no op in flight, as before): escalate the per-cloud-
        //   version failure counter and the restore backoff.
        OpResult::Failed => match op {
            Some(Op::Backup) => {
                next.next_backup_at = Some(now + Duration::seconds(BACKUP_FAILURE_BACKOFF_SECS));
            }
            _ => {
                let delay = record_failure(&mut next.restore_failures, obs.cloud_version);
                next.next_restore_at = Some(now + Duration::seconds(delay));
            }
        },
    }
}

/// Records a restore failure against the observed cloud version
/// (`obs.cloud_version`, the head we were trying to fetch, same as the original
/// engine's `latest_versions.get(id)`) and returns the backoff to apply. Sans-IO
/// twin of `AutoRestoreFailures::record_failure`: a different version resets the
/// escalation, which is the other half of
/// [`clear_restore_backoff_on_new_version`]. The original tuple's second value,
/// "emit stuck", is decided by the shell reading `stuck_notified`.
fn record_failure(f: &mut RestoreFailures, latest: Option<i64>) -> i64 {
    if f.version != latest {
        f.version = latest;
        f.consecutive = 0;
        f.stuck_notified = false;
    }
    f.consecutive = f.consecutive.saturating_add(1);
    if f.consecutive >= STUCK_AFTER {
        f.stuck_notified = true;
    }
    backoff_secs(f.consecutive)
}

/// Records an unresolvable conflict against the observed cloud head and returns
/// the backoff to apply, or `None` when the budget is spent and retrying has to
/// stop.
///
/// A head different from the counted one resets the escalation, for the same
/// reason as in [`record_failure`]: the cloud moved, so it is no longer the same
/// question and maybe now there *is* something to pull.
fn record_conflict(c: &mut ConflictStall, latest: Option<i64>) -> Option<i64> {
    if c.version != latest {
        *c = ConflictStall {
            version: latest,
            ..ConflictStall::default()
        };
    }
    c.consecutive = c.consecutive.saturating_add(1);
    if c.consecutive >= CONFLICT_STALL_GIVE_UP_AFTER {
        c.needs_attention = true;
        return None;
    }
    let idx = (c.consecutive as usize - 1).min(CONFLICT_STALL_BACKOFF_SECS.len() - 1);
    Some(CONFLICT_STALL_BACKOFF_SECS[idx])
}

/// Releases the conflict escalation when the cloud publishes a head different
/// from the one it stalled against. Twin of
/// [`clear_restore_backoff_on_new_version`], and needed separately: a save that
/// already gave up never ingests another `ConflictStalled`, because it stopped
/// retrying, so without this nothing could unstick it except the user, not even
/// the other device publishing the missing version.
fn clear_conflict_stall_on_new_version(next: &mut State, obs: &Observation) {
    let active = next.backup_conflict.consecutive > 0 || next.backup_conflict.needs_attention;
    if active && next.backup_conflict.version != obs.cloud_version {
        next.backup_conflict = ConflictStall::default();
        next.next_backup_at = None;
    }
}

/// Backoff for a given number of consecutive failures (1-based). Saturates on
/// the last rung. Same as `agent::auto_restore_backoff`.
fn backoff_secs(failures: u32) -> i64 {
    let idx = (failures.max(1) as usize - 1).min(FAILURE_BACKOFF_SECS.len() - 1);
    FAILURE_BACKOFF_SECS[idx]
}

/// Throttle backoff deadline: the server's wait (clamped to 1..=300, plus 2) plus
/// per-save jitter. The jitter uses `StdRng::seed_from_u64(seed)` and never
/// `thread_rng` (ADR C.2: simulation and replay have to be deterministic). In the
/// inverted engine the shell derives `seed` from the `save_id`, reproducing the
/// original `hash(id) % 6` in an injectable way.
fn throttle_until(now: OffsetDateTime, retry_after_secs: u32, seed: u64) -> OffsetDateTime {
    let wait = (u64::from(retry_after_secs)).clamp(1, MAX_THROTTLE_WAIT_SECS as u64) + 2;
    let mut rng = StdRng::seed_from_u64(seed);
    let jitter: u64 = rng.gen_range(0..6);
    now + Duration::seconds((wait + jitter) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const BASE: OffsetDateTime = OffsetDateTime::UNIX_EPOCH;

    fn at(off: i64) -> OffsetDateTime {
        BASE + Duration::seconds(off)
    }

    fn world(now_off: i64) -> World {
        World {
            now: at(now_off),
            seed: 0,
        }
    }

    /// A real slot (not track-only) with restore enabled and nothing in flight.
    fn base_state() -> State {
        State {
            restore_enabled: true,
            ..Default::default()
        }
    }

    /// A quiescent observation: no one-shot signals (fs, op), a populated
    /// folder, the cloud not ahead, the process dead. The starting point for
    /// "converged".
    fn quiet_obs() -> Observation {
        Observation {
            folder_mtime: Some(at(-10_000)), // muy vieja: el fallback de disco no salta
            ..Default::default()
        }
    }

    fn acts(ds: &[Decision]) -> Vec<&Action> {
        ds.iter().filter_map(Decision::action).collect()
    }

    fn storage_act_count(ds: &[Decision]) -> usize {
        ds.iter()
            .filter(|d| matches!(d.action(), Some(Action::Backup) | Some(Action::Restore)))
            .count()
    }

    // ---- D.4 corpus (fixed deterministic scenarios)

    /// The compression hot loop (1.29M R2 ops): converged means zero actions.
    /// The bug emitted actions (compress, upload) with no input delta at all.
    /// Here, with the local fingerprint EQUAL to the synced one, not even a
    /// spurious `has_pending` fires a backup: only `Hold { "converged" }`.
    #[test]
    fn d4_converged_emits_zero_actions() {
        let state = State {
            has_pending: true, // settle espurio del watcher
            synced_fingerprint: Some(0xABCD),
            known_version: Some(7),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(0xABCD), // contenido idéntico a lo ya subido
            cloud_version: Some(7),          // nube no adelantada
            ..quiet_obs()
        };
        let (_next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            acts(&ds).is_empty(),
            "convergido debe emitir cero Act, salió: {ds:?}"
        );
        assert_eq!(ds, vec![hold("converged")]);
    }

    /// The hot loop in its dynamic form: two identical ticks in a row do not emit
    /// a second action. A backup starts once; the second tick sees it in flight
    /// and holds.
    #[test]
    fn d4_no_action_without_a_delta() {
        let state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2), // difiere → hay delta real la 1ª vez
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs, world(0));
        assert_eq!(acts(&d1), vec![&Action::Backup], "el delta real sube");
        // Same world, same now: with no new delta there is no second action.
        let (_s2, d2) = reconcile(&s1, &obs, world(0));
        assert!(
            acts(&d2).is_empty(),
            "sin nuevo delta no debe re-actuar, salió: {d2:?}"
        );
        assert_eq!(d2, vec![hold("operation in flight")]);
    }

    /// A 429 on restore, plus backup/restore symmetry. The throttle brakes the
    /// right op and does NOT touch the failure counter, and `now` crossing the
    /// deadline frees it. Throttles used to be handled on backup only.
    #[test]
    fn d4_throttle_is_symmetric_and_does_not_count_as_failure() {
        // Restore throttled.
        let state = State {
            in_flight: Some(Op::Restore),
            known_version: Some(3),
            ..base_state()
        };
        let obs = Observation {
            local_empty: true, // querríamos restaurar
            cloud_version: Some(5),
            op_result: Some(OpResult::Throttled {
                retry_after_secs: 30,
            }),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            matches!(
                ds.iter().find_map(Decision::action),
                Some(Action::Throttle { .. })
            ),
            "un 429 de restore emite Throttle: {ds:?}"
        );
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "un throttle NO cuenta como fallo"
        );
        let until = next
            .next_restore_at
            .expect("restore frenado hasta un deadline");
        assert!(until > at(0), "el backoff mira al futuro");

        // Before the deadline: cooldown, no restore.
        let obs_after = Observation {
            local_empty: true,
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let mid = (until - at(0)).whole_seconds() / 2;
        let (_n, ds_mid) = reconcile(&next, &obs_after, world(mid));
        assert_eq!(ds_mid, vec![hold("restore cooldown")]);

        // Past the deadline (a legitimate delta): the restore proceeds.
        let past = (until - at(0)).whole_seconds() + 1;
        let (_n2, ds_past) = reconcile(&next, &obs_after, world(past));
        assert_eq!(
            acts(&ds_past),
            vec![&Action::Restore],
            "tras el backoff, restaura"
        );

        // Symmetry: the SAME throttle on a backup brakes `next_backup_at`, not
        // the restore.
        let bstate = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let bobs = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: 30,
            }),
            ..quiet_obs()
        };
        let (bn, bds) = reconcile(&bstate, &bobs, world(0));
        assert!(
            bn.next_backup_at.is_some(),
            "el throttle de backup frena el backup"
        );
        assert!(bn.next_restore_at.is_none(), "sin tocar el lado restore");
        assert!(
            matches!(
                bds.iter().find_map(Decision::action),
                Some(Action::Throttle { .. })
            ),
            "backup también emite Throttle: {bds:?}"
        );
    }

    /// The hour the server's brake asks for has to survive the cap.
    ///
    /// `loopguard::QUOTA_WAIT_SECS` answers a full account with 3600,
    /// deliberately the same as [`QUOTA_FULL_BACKOFF_SECS`], and the 300 s cap
    /// that used to live here silently shortened it to five minutes: twelve
    /// retries an hour, per save, against a wall only a person can move.
    #[test]
    fn a_server_asking_for_an_hour_gets_an_hour() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: QUOTA_FULL_BACKOFF_SECS as u32,
            }),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        let waited = (next.next_backup_at.expect("parked") - at(0)).whole_seconds();
        assert!(
            waited >= QUOTA_FULL_BACKOFF_SECS,
            "the cap must not shorten what the server asked for: {waited}s"
        );

        // And the cap still exists: an absurd `retry_after` can't park the save
        // until the next restart.
        let obs_bogus = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: u32::MAX,
            }),
            ..quiet_obs()
        };
        let (bogus, _) = reconcile(&state, &obs_bogus, world(0));
        let capped = (bogus.next_backup_at.expect("parked") - at(0)).whole_seconds();
        assert!(
            capped <= MAX_THROTTLE_WAIT_SECS + 8,
            "a junk retry_after gets capped: {capped}s"
        );
    }

    /// A full account (402) parks the upload for an hour, keeps `has_pending`
    /// (the bytes are still only on disk) and does not count as the save
    /// failing. And it leaves the restore side alone: downloading costs no quota.
    #[test]
    fn quota_full_parks_the_upload_without_blaming_the_save() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            op_result: Some(OpResult::QuotaFull),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        let until = next.next_backup_at.expect("la subida queda aparcada");
        assert_eq!(
            (until - at(0)).whole_seconds(),
            QUOTA_FULL_BACKOFF_SECS,
            "el park del 402 es el largo, no el de un fallo cualquiera"
        );
        assert!(next.has_pending, "los cambios locales siguen sin versión");
        assert!(next.next_restore_at.is_none(), "sin tocar el lado restore");
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "una cuenta llena NO es un save roto"
        );

        // Before the deadline there is no retry; past it, the upload goes again.
        let obs_quiet = quiet_obs();
        let (_m, ds_mid) = reconcile(&next, &obs_quiet, world(QUOTA_FULL_BACKOFF_SECS / 2));
        assert!(
            !acts(&ds_mid).contains(&&Action::Backup),
            "dentro del park no se reintenta: {ds_mid:?}"
        );
        let (_p, ds_past) = reconcile(&next, &obs_quiet, world(QUOTA_FULL_BACKOFF_SECS + 1));
        assert!(
            acts(&ds_past).contains(&&Action::Backup),
            "pasada la hora vuelve a intentarlo: {ds_past:?}"
        );
    }

    /// The deferred pull that never landed. Mid-session with the cloud ahead, it
    /// gets DEFERRED (one notification only) and survives the veto; when the game
    /// closes with nothing pending, the pull LANDS.
    #[test]
    fn d4_deferred_pull_survives_veto_and_lands_on_close() {
        // Mid-session: process alive, cloud ahead.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            known_version: Some(4),
            ..base_state()
        };
        let obs_playing = Observation {
            process_alive: true,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs_playing, world(0));
        assert_eq!(
            acts(&d1),
            vec![&Action::DeferPull],
            "1ª vez: difiere y notifica"
        );
        assert!(s1.pull_pending && s1.deferred_notified);

        // Still playing: no re-notification, holds with the veto's reason.
        let (s2, d2) = reconcile(&s1, &obs_playing, world(1));
        assert!(acts(&d2).is_empty(), "no re-notifica cada tick");
        assert_eq!(d2, vec![hold("game process is running")]);
        assert!(s2.pull_pending, "el pull diferido sobrevive");

        // Game closed over 6 s ago (sticky expired) and nothing pending: it lands.
        let obs_closed = Observation {
            process_alive: false,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (s3, d3) = reconcile(&s2, &obs_closed, world(10));
        assert_eq!(
            acts(&d3),
            vec![&Action::Restore],
            "al cerrar, el pull aterriza"
        );
        assert!(!s3.pull_pending && !s3.deferred_notified, "consumido");
    }

    /// The sticky window as a veto-latency invariant. The process dies; inside
    /// the 6 s window the session veto still holds (anti-flap grace), but just
    /// past the window it lifts, and not at 90 s.
    #[test]
    fn d4_veto_latency_is_six_seconds_not_ninety() {
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            known_version: Some(1),
            ..base_state()
        };
        let obs = Observation {
            process_alive: false, // el juego se cerró
            local_empty: true,    // hay algo que restaurar
            cloud_version: Some(2),
            ..quiet_obs()
        };
        // At 5 s: inside the grace, still "running", so it defers and holds.
        let (_n5, d5) = reconcile(&state, &obs, world(5));
        assert!(
            !acts(&d5).contains(&&Action::Restore),
            "dentro de la gracia el veto aún retiene: {d5:?}"
        );
        // At 7 s: past the 6 s grace, the veto lifts and it restores.
        let (n7, d7) = reconcile(&state, &obs, world(7));
        assert!(!n7.is_running, "pasada la gracia, deja de correr");
        assert_eq!(
            acts(&d7),
            vec![&Action::Restore],
            "el veto se levanta a los 6 s, no a los 90"
        );
    }

    /// Regression: a restore that comes back `Ok` without writing and leaves the
    /// folder empty is NOT progress. `local_empty` bypasses the version gate on
    /// purpose, so without braking here the next tick asks for the same snapshot
    /// and the (download, don't write) pair repeats forever at the full price of
    /// the download: 3,752 downloads and 10.6 GB in production between 2026-07-27
    /// and 08-03.
    #[test]
    fn restore_ok_sin_escribir_y_carpeta_vacia_no_reintenta_de_inmediato() {
        let state = State {
            known_version: Some(2),
            in_flight: Some(Op::Restore),
            ..base_state()
        };
        // The op comes back OK, without writing, and the folder is still empty.
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(2),
                fingerprint: None,
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (n1, d1) = reconcile(&state, &obs, world(0));
        assert!(
            !acts(&d1).contains(&&Action::Restore),
            "no puede relanzar el restore en el mismo tick: {d1:?}"
        );
        assert_eq!(
            n1.restore_failures.consecutive, 1,
            "el 'éxito' que no progresa cuenta como intento, no limpia la escalada"
        );
        assert!(n1.next_restore_at.is_some(), "queda un backoff armado");

        // Next tick with the folder still empty: held by the cooldown, not
        // another download.
        let obs2 = Observation {
            local_empty: true,
            cloud_version: Some(2),
            ..quiet_obs()
        };
        let (_n2, d2) = reconcile(&n1, &obs2, world(1));
        assert!(
            !acts(&d2).contains(&&Action::Restore),
            "sigue frenado mientras dura el backoff: {d2:?}"
        );

        // And a restore that DOES write clears the escalation: the brake is only
        // for the one that makes no progress.
        let obs_ok = Observation {
            local_empty: false,
            cloud_version: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(2),
                fingerprint: None,
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (n3, _d3) = reconcile(&state, &obs_ok, world(0));
        assert_eq!(
            n3.restore_failures.consecutive, 0,
            "un restore con escritura real sí es progreso"
        );
    }

    /// Never `Act(Restore)` with unversioned local changes (never lose newer
    /// local): `has_pending` is a veto reason, and with the cloud ahead it defers
    /// rather than walking over local progress. And since D.8.1, the same tick
    /// releases the upload: deferring the pull cannot leave local progress
    /// unversioned, because only a backup clears `has_pending`.
    #[test]
    fn d4_never_restore_over_unflushed_local() {
        let state = State {
            has_pending: true,
            known_version: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_empty: false,
            cloud_version: Some(9),
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &obs, world(0));
        assert!(
            !acts(&ds).contains(&&Action::Restore),
            "no restaurar sobre local sin versionar: {ds:?}"
        );
        assert_eq!(
            acts(&ds),
            vec![&Action::DeferPull, &Action::Backup],
            "se difiere el pull y se vuelca lo local"
        );
    }

    // ---- D.8 corpus (the policy the kernel was missing)

    /// The `has_pending` plus `cloud_ahead` deadlock. Two cloud advances in the
    /// SAME session, with no game close in between, must not stall the slot.
    ///
    /// The bug: the reducer held the pull (correctly) and returned before the
    /// backup branch, so `has_pending`, which only an upload clears, stayed set
    /// forever; and since `has_pending` is itself a veto reason, nothing went up
    /// and nothing came down. The `DeferPull` *executor* in the shell (`agent.rs`)
    /// unstuck it, policy outside the kernel and invisible to the C.5 replay.
    #[test]
    fn d8_two_cloud_advances_in_one_session_do_not_wedge() {
        // Live session: the game is running and there is unversioned progress.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            has_pending: true,
            known_version: Some(4),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        // First cloud advance (v6 > v4) while playing.
        let obs1 = Observation {
            process_alive: true,
            cloud_version: Some(6),
            local_fingerprint: Some(2), // contenido local divergente
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs1, world(0));
        assert!(
            acts(&d1).contains(&&Action::DeferPull),
            "1ª vez: difiere y notifica: {d1:?}"
        );
        assert!(
            acts(&d1).contains(&&Action::Backup),
            "y suelta el backup que destraba `has_pending`: {d1:?}"
        );
        assert!(s1.pull_pending, "el pull diferido sobrevive");
        assert_eq!(s1.in_flight, Some(Op::Backup));

        // The upload hits a 409 and settles onto the remote head (v7): no commit,
        // but `known_version` advances and `has_pending` clears.
        let obs_done = Observation {
            process_alive: true,
            cloud_version: Some(6),
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(7),
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (s2, _d2) = reconcile(&s1, &obs_done, world(1));
        assert!(!s2.has_pending, "la subida destrabó los cambios locales");
        assert_eq!(s2.known_version, Some(7));
        assert!(
            s2.pull_pending,
            "el pull sigue pendiente: el juego no ha cerrado"
        );

        // The user keeps playing and saves again; the cloud advances AGAIN (v8)
        // with no game close in between. The slot must not stall.
        let s3 = State {
            has_pending: true,
            ..s2
        };
        let obs2 = Observation {
            process_alive: true,
            cloud_version: Some(8),
            local_fingerprint: Some(3),
            ..quiet_obs()
        };
        let (s4, d4) = reconcile(&s3, &obs2, world(2));
        assert!(
            acts(&d4).contains(&&Action::Backup),
            "el 2º adelanto tampoco encalla la subida: {d4:?}"
        );
        assert!(
            !acts(&d4).contains(&&Action::DeferPull),
            "pero NO re-notifica: `deferred_notified` de-duplica sólo el aviso: {d4:?}"
        );
        assert!(
            !acts(&d4).contains(&&Action::Restore),
            "y jamás restaura mid-session: {d4:?}"
        );
        assert!(s4.pull_pending, "la intención de pull sigue viva");
    }

    /// The other half: between two advances, with the pull already deferred and
    /// the cloud no longer ahead, the mid-session autobackup keeps working. The
    /// `pull_pending` branch used to return before the backup too, killing the
    /// upload for the rest of the session.
    #[test]
    fn d8_deferred_pull_does_not_starve_mid_session_backups() {
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            has_pending: true,
            pull_pending: true,
            deferred_notified: true,
            known_version: Some(7),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            process_alive: true,
            cloud_version: Some(7), // la nube ya no va por delante
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            acts(&ds).contains(&&Action::Backup),
            "un pull pendiente no debe matar el autobackup de la sesión: {ds:?}"
        );
        assert!(next.pull_pending, "y el pull sigue esperando al cierre");
    }

    /// The *backup* failure backoff, now inside the kernel. The shell used to
    /// restore it (`RetryBackupAfterFailure`): clear `in_flight`, arm the long
    /// backoff, keep `has_pending`. An upload failure does not escalate the
    /// restore ladder.
    #[test]
    fn d8_backup_failure_backs_off_inside_the_kernel() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Failed),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.in_flight, None, "la op terminó");
        assert!(
            next.has_pending,
            "los cambios nunca llegaron a una versión: siguen pendientes"
        );
        assert_eq!(
            next.next_backup_at,
            Some(at(BACKUP_FAILURE_BACKOFF_SECS)),
            "re-armado en el backoff largo"
        );
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "un fallo de subida no escala la escalada del restore"
        );
        assert!(next.next_restore_at.is_none(), "ni frena el lado restore");
        assert_eq!(ds.last(), Some(&hold("backup backoff")));
        assert!(
            !acts(&ds).contains(&&Action::Backup),
            "no se relanza dentro del backoff: {ds:?}"
        );

        // Past the backoff (`now` crossing a deadline IS a delta): it retries.
        let obs_after = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (_n, ds_after) = reconcile(&next, &obs_after, world(BACKUP_FAILURE_BACKOFF_SECS + 1));
        assert_eq!(acts(&ds_after), vec![&Action::Backup]);
    }

    /// The 409-with-no-way-out bug: the shell answered "you are behind, but there
    /// is nothing to pull" by restoring the retry every ten minutes, with no
    /// counter and no escalation. 1,701 events, 5 users, and one save pinned for
    /// 14 days at about 4.5 attempts an hour that outlived three releases.
    ///
    /// Now it escalates 10, 20, 40, 80 minutes and stops at the fifth:
    /// `needs_attention` and not one more `Act(Backup)` on its own.
    #[test]
    fn an_unresolvable_conflict_escalates_and_then_stops() {
        let mut state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let mut clock = 0i64;
        for (attempt, backoff) in CONFLICT_STALL_BACKOFF_SECS.iter().enumerate() {
            // The upload goes out, hits the conflict, and comes back with it.
            state.in_flight = Some(Op::Backup);
            let obs = Observation {
                local_fingerprint: Some(2),
                op_result: Some(OpResult::ConflictStalled),
                ..quiet_obs()
            };
            let (next, ds) = reconcile(&state, &obs, world(clock));
            assert_eq!(
                next.backup_conflict.consecutive,
                attempt as u32 + 1,
                "el contador tiene que ir subiendo"
            );
            assert!(
                !next.backup_conflict.needs_attention,
                "aún queda presupuesto en el intento {}",
                attempt + 1
            );
            assert_eq!(
                next.next_backup_at,
                Some(at(clock + backoff)),
                "cada choque espera más que el anterior"
            );
            assert!(next.has_pending, "los cambios siguen sin versionar");
            assert!(
                !acts(&ds).contains(&&Action::Backup),
                "no se relanza dentro del backoff: {ds:?}"
            );
            // And inside the backoff it stays quiet, however much time passes.
            let quiet = Observation {
                local_fingerprint: Some(2),
                ..quiet_obs()
            };
            let (_m, mid) = reconcile(&next, &quiet, world(clock + backoff / 2));
            assert!(
                !acts(&mid).contains(&&Action::Backup),
                "el backoff manda hasta que vence: {mid:?}"
            );
            // Expired, so it retries: `now` crossing the deadline IS a delta.
            clock += backoff + 1;
            let (after, retried) = reconcile(&next, &quiet, world(clock));
            assert!(
                acts(&retried).contains(&&Action::Backup),
                "vencido el backoff hay que volver a intentarlo: {retried:?}"
            );
            state = after;
        }

        // Fifth collision: the budget is spent.
        state.in_flight = Some(Op::Backup);
        let obs = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::ConflictStalled),
            ..quiet_obs()
        };
        let (given_up, ds) = reconcile(&state, &obs, world(clock));
        assert_eq!(
            given_up.backup_conflict.consecutive,
            CONFLICT_STALL_GIVE_UP_AFTER
        );
        assert!(
            given_up.backup_conflict.needs_attention,
            "al quinto, el save pide una persona"
        );
        assert_eq!(ds.last(), Some(&hold(HOLD_BACKUP_NEEDS_ATTENTION)));

        // And it never retries on its own again: not in a minute, not in a week.
        let quiet = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        for later in [clock + 60, clock + 86_400, clock + 7 * 86_400] {
            let (_n, ds) = reconcile(&given_up, &quiet, world(later));
            assert!(
                !acts(&ds).contains(&&Action::Backup),
                "un save rendido no puede volver a reintentar solo (t={later}): {ds:?}"
            );
            assert_eq!(ds.last(), Some(&hold(HOLD_BACKUP_NEEDS_ATTENTION)));
        }
    }

    /// The number that exposed it, put to the test: fourteen days on the clock
    /// with the conflict always answering the same. It used to be about 1,500
    /// attempts (one every ten minutes, forever); now it is five and done.
    ///
    /// Simulated tick by tick rather than reasoned about from the backoff,
    /// because the loop lived precisely in the seam between "the reducer arms the
    /// deadline" and "the next tick crosses it".
    #[test]
    fn fourteen_days_of_the_same_conflict_cost_five_attempts() {
        let mut state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let quiet = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let conflicted = Observation {
            op_result: Some(OpResult::ConflictStalled),
            ..quiet.clone()
        };

        let mut attempts = 0;
        let mut in_flight = false;
        // One tick a minute for 14 days.
        for minute in 0..(14 * 24 * 60) {
            let now = minute * 60;
            let obs = if in_flight { &conflicted } else { &quiet };
            let (next, ds) = reconcile(&state, obs, world(now));
            in_flight = false;
            if acts(&ds).contains(&&Action::Backup) {
                attempts += 1;
                // The upload goes out and comes back with the same old conflict.
                in_flight = true;
            }
            state = next;
        }

        assert_eq!(
            attempts, CONFLICT_STALL_GIVE_UP_AFTER,
            "el 409 sin salida sólo puede costar el presupuesto, no catorce días de intentos"
        );
        assert!(
            state.backup_conflict.needs_attention,
            "y acaba pidiendo una persona"
        );
        assert!(
            state.has_pending,
            "sin perder los cambios locales por el camino"
        );
    }

    /// Giving up cannot be a life sentence: if the cloud publishes another head,
    /// the question is no longer the same (maybe now there *is* something to
    /// pull) and the save tries again on its own. Without this nothing could
    /// unstick it except the user, not even the other device uploading what was
    /// missing.
    #[test]
    fn a_new_cloud_head_un_stalls_a_save_that_gave_up() {
        let state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            known_version: Some(4),
            backup_conflict: ConflictStall {
                consecutive: CONFLICT_STALL_GIVE_UP_AFTER,
                version: Some(4),
                needs_attention: true,
            },
            next_backup_at: Some(at(9_000)),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(
            next.backup_conflict,
            ConflictStall::default(),
            "la escalada muere con la cabeza contra la que se contaba"
        );
        // The cloud is ahead, so this tick is a download; what matters is that
        // the upload's brake came off.
        assert!(next.next_backup_at.is_none(), "y su freno con ella");
        assert!(!ds.is_empty());
    }

    /// A copy that succeeds releases the whole escalation, the no-op included,
    /// since that also means the conflict resolved.
    #[test]
    fn a_backup_that_lands_clears_the_conflict_escalation() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            backup_conflict: ConflictStall {
                consecutive: 3,
                version: Some(4),
                needs_attention: false,
            },
            ..base_state()
        };
        let obs = Observation {
            cloud_version: Some(4),
            op_result: Some(OpResult::Ok {
                version: Some(5),
                fingerprint: Some(2),
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.backup_conflict, ConflictStall::default());
    }

    /// Commit versus no-op in `OpResult::Ok`. THE R.E.P.O. regression: a no-op
    /// pass is not a backup and must not move the min-interval anchor, or the next
    /// real upload gets pushed out by a whole interval. With the folder being
    /// emptied by a restore, the anchor advanced on phantom backups and a short
    /// session never flushed its progress.
    #[test]
    fn a_calm_save_never_waits() {
        // What broke in June and must not break again: with no preset and no
        // burst, a copy goes out as soon as the debounce settles. A floor nobody
        // can see or change reads as "it isn't noticing my changes".
        let state = State {
            min_backup_interval_secs: 0,
            burst_since: Some(at(0)),
            burst_backups: BURST_THRESHOLD - 1,
            last_backup_at: Some(at(0)),
            ..State::default()
        };
        assert_eq!(backup_floor(&state), None);
    }

    /// A game rewriting its autosave every few seconds: on the third commit
    /// inside the window the floor kicks in, and stays at 60 s however many more
    /// it makes. One game managed 1,027 uploads in four and a half hours
    /// without this.
    #[test]
    fn an_autosave_burst_gets_one_minute_and_no_more() {
        let mut state = State {
            min_backup_interval_secs: 0,
            ..State::default()
        };
        for i in 0..3 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(state.burst_backups, 3);
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);

        // Ten more commits do not raise it by a second: one rung only.
        for i in 3..13 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);
    }

    /// Past the window with no activity the count opens fresh, so the save goes
    /// back to uploading immediately: the floor lasts as long as the burst does.
    #[test]
    fn the_burst_forgets_itself_once_the_game_calms_down() {
        let mut state = State {
            min_backup_interval_secs: 0,
            ..State::default()
        };
        for i in 0..5 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);

        count_burst(&mut state, at(BURST_WINDOW_SECS + 60));
        assert_eq!(state.burst_backups, 1);
        assert_eq!(effective_min_interval(&state), 0);
    }

    /// The preset the user chose beats the adaptive floor in both directions:
    /// `short_session` keeps its 30 s even mid-burst (it is a game that wipes its
    /// folder between rounds, and losing one round is losing the run), and
    /// `data_saver` keeps its 600 s.
    #[test]
    fn an_explicit_preset_wins_over_the_adaptive_floor() {
        let mut short = State {
            min_backup_interval_secs: 30,
            ..State::default()
        };
        for i in 0..10 {
            count_burst(&mut short, at(i * 6));
        }
        assert_eq!(effective_min_interval(&short), 30);

        let saver = State {
            min_backup_interval_secs: 600,
            burst_backups: 0,
            ..State::default()
        };
        assert_eq!(effective_min_interval(&saver), 600);
    }

    #[test]
    fn d8_no_op_backup_does_not_anchor_the_min_interval() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            min_backup_interval_secs: 600,
            synced_fingerprint: Some(1),
            ..base_state()
        };

        // A pure no-op (skipped by signature, empty, archived): no version.
        let obs_noop = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: None,
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (noop, _) = reconcile(&state, &obs_noop, world(0));
        assert!(
            noop.last_backup_at.is_none(),
            "un no-op no ancla el min-interval (R.E.P.O.)"
        );
        assert!(noop.next_backup_at.is_none(), "ni arma el suelo");
        assert!(!noop.has_pending, "pero sí destraba los cambios");
        assert_eq!(noop.synced_fingerprint, Some(2), "y adopta la firma");
        assert!(
            noop.last_restore_at.is_none(),
            "un no-op sin versión no tocó la carpeta"
        );

        // A real commit anchors the floor.
        let obs_commit = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (committed, _) = reconcile(&state, &obs_commit, world(0));
        assert_eq!(committed.last_backup_at, Some(at(0)));
        assert_eq!(committed.known_version, Some(9));

        // And that anchor is what really brakes the next upload: a new write at
        // 100 s is held, and past the floor (600 s) it goes up. With the no-op's
        // anchor (never set) there would be no brake at all, which is exactly
        // right, because nothing was uploaded.
        let obs_more = Observation {
            fs_event: true,
            local_fingerprint: Some(7),
            ..quiet_obs()
        };
        let (_n, held) = reconcile(&committed, &obs_more, world(100));
        assert_eq!(held, vec![hold("backup min-interval")]);
        let (_n, freed) = reconcile(&committed, &obs_more, world(601));
        assert_eq!(acts(&freed), vec![&Action::Backup]);
        let (_n, no_floor) = reconcile(&noop, &obs_more, world(100));
        assert_eq!(
            acts(&no_floor),
            vec![&Action::Backup],
            "un no-op no dejó ancla, así que no frena nada"
        );

        // A no-op WITH a version is the 409 settled onto the head: the merge wrote
        // into the folder like a restore, so it stamps `last_restore_at` (so that
        // touch of ours does not veto the next pull) but still does not anchor the
        // floor.
        let obs_settled = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (settled, _) = reconcile(&state, &obs_settled, world(0));
        assert_eq!(settled.known_version, Some(9));
        assert_eq!(settled.last_restore_at, Some(at(0)));
        assert!(
            settled.last_backup_at.is_none(),
            "asentarse a la cabeza no es un commit propio"
        );
    }

    /// Anti-relaunch against the server's truth.
    ///
    /// The real case: the daemon restarts (routine since Slice 4) with an upload
    /// in flight that did commit. The in-memory `in_flight` is gone, so the engine
    /// launches the upload again; the content-addressed check discovers that
    /// content is already the head and uploads nothing. What the reducer has to do
    /// with that answer:
    ///
    /// - adopt the version and the signature (converge),
    /// - do NOT anchor the min-interval (there was no commit of ours: R.E.P.O.),
    /// - and do NOT stamp `last_restore_at`, because unlike the 409 settled onto
    ///   the head, not a byte was written to the folder here.
    #[test]
    fn d8_3_an_upload_that_already_landed_converges_without_faking_a_local_touch() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            min_backup_interval_secs: 600,
            known_version: Some(8),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(9),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: false,
            }),
            upload_landed: Some(true),
            ..quiet_obs()
        };

        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.in_flight, None, "la op terminó");
        assert!(!next.has_pending, "el contenido está en una versión");
        assert_eq!(
            next.known_version,
            Some(9),
            "adopta la versión que ya lo tenía"
        );
        assert_eq!(next.synced_fingerprint, Some(2));
        assert!(
            next.last_backup_at.is_none(),
            "no se subió nada: anclar el suelo aquí es la regresión R.E.P.O."
        );
        assert!(
            next.last_restore_at.is_none(),
            "y no se escribió en la carpeta: sellar un toque que no existió \
             falsearía la ventana de gracia del veto"
        );
        assert!(
            !acts(&ds).iter().any(|a| matches!(a, Action::Backup)),
            "y sobre todo: no se relanza la subida ({ds:?})"
        );

        // Next tick, with no op in flight and the same content: converged means
        // zero actions. That is the half that matters. If the reducer had not
        // adopted the version and signature, it would emit `Backup` again here and
        // we would have the very loop D.8.3 exists to kill.
        let quiet = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(9),
            ..quiet_obs()
        };
        let (_after, ds_after) = reconcile(&next, &quiet, world(1));
        assert_eq!(ds_after, vec![hold("converged")]);
    }

    /// The flush that unblocks a cross-device pull skips the *data saving* floor
    /// (as the executor used to, going straight to the backup) but NOT an error
    /// backoff. Without this, on the `data_saver` preset (600 s) another device's
    /// update would wait the whole interval, because the pull stays vetoed until
    /// `has_pending` clears.
    #[test]
    fn d8_cross_device_flush_skips_the_savings_floor_but_not_a_backoff() {
        // A commit 100 s ago with a 600 s floor, and the user has saved again.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(100)),
            has_pending: true,
            min_backup_interval_secs: 600,
            last_backup_at: Some(at(0)),
            known_version: Some(4),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let quiet_cloud = Observation {
            process_alive: true,
            cloud_version: Some(4), // nube al día
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &quiet_cloud, world(100));
        assert_eq!(
            ds,
            vec![hold("backup min-interval")],
            "sin urgencia el suelo de ahorro manda"
        );

        // The cloud advances: the flush is no longer pacing, it unblocks the pull.
        let ahead = Observation {
            cloud_version: Some(6),
            ..quiet_cloud.clone()
        };
        let (_n, ds_urgent) = reconcile(&state, &ahead, world(100));
        assert!(
            acts(&ds_urgent).contains(&&Action::Backup),
            "el flush cross-device no espera al suelo de ahorro: {ds_urgent:?}"
        );

        // But an error backoff does brake it: that is not pacing.
        let backing_off = State {
            next_backup_at: Some(at(700)),
            ..state
        };
        let (_n, ds_backoff) = reconcile(&backing_off, &ahead, world(100));
        assert!(
            !acts(&ds_backoff).contains(&&Action::Backup),
            "un backoff de error no se salta ni por urgencia: {ds_backoff:?}"
        );
        assert_eq!(ds_backoff.last(), Some(&hold("backup backoff")));
    }

    /// A new cloud version clears the restore backoff. The backoff was about the
    /// version that was failing; the server publishing another is new information,
    /// not a retry. The shell used to do this on `SetCloudVersions`.
    #[test]
    fn d8_new_cloud_version_clears_the_restore_backoff() {
        // Three failures against v5: stuck and parked for an hour.
        let state = State {
            known_version: Some(3),
            restore_failures: RestoreFailures {
                consecutive: 3,
                version: Some(5),
                stuck_notified: true,
            },
            next_restore_at: Some(at(3600)),
            ..base_state()
        };

        // Same version: the escalation holds and so does the brake.
        let obs_same = Observation {
            local_empty: true,
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let (same, ds_same) = reconcile(&state, &obs_same, world(0));
        assert!(
            same.restore_failures.stuck_notified,
            "sin novedad, sigue stuck"
        );
        assert_eq!(ds_same, vec![hold("restore cooldown")]);

        // The server publishes v6: the escalation dies and the pull goes now.
        let obs_new = Observation {
            local_empty: true,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (fresh, ds_new) = reconcile(&state, &obs_new, world(0));
        assert_eq!(
            fresh.restore_failures,
            RestoreFailures::default(),
            "versión nueva ⇒ escalada reseteada (el shell lo lee para 'recovered')"
        );
        assert_eq!(
            acts(&ds_new),
            vec![&Action::Restore],
            "y el reintento no espera al backoff viejo: {ds_new:?}"
        );
    }

    /// The restore failure escalation anchors on the observed CLOUD version, the
    /// head we were trying to fetch, not on the local one. That is what makes the
    /// reset-on-new-version coherent.
    #[test]
    fn d8_restore_failures_anchor_on_the_observed_cloud_version() {
        let state = State {
            in_flight: Some(Op::Restore),
            known_version: Some(3),
            ..base_state()
        };
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(9),
            op_result: Some(OpResult::Failed),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.restore_failures.version, Some(9));
        assert_eq!(next.restore_failures.consecutive, 1);
        assert_eq!(
            next.next_restore_at,
            Some(at(FAILURE_BACKOFF_SECS[0])),
            "primer escalón del backoff"
        );
    }

    // ---- D.10 corpus (the cloud poller goes quiet)

    /// Converged versus blind. With the cloud version cache aged out, the rest
    /// stops being labelled `converged` and says why it knows nothing: the
    /// invisible failure (a dead poller) made observable. With no feed stamp
    /// (self-hosted, CLI, no poller) no staleness is reported at all.
    #[test]
    fn d10_stale_cloud_cache_is_not_convergence() {
        let state = State {
            known_version: Some(120),
            synced_fingerprint: Some(0xABCD),
            ..base_state()
        };
        let fed_at = |off: i64| Observation {
            local_fingerprint: Some(0xABCD),
            cloud_version: Some(120),
            cloud_version_as_of: Some(at(off)),
            ..quiet_obs()
        };

        // A feed that just arrived: genuinely converged.
        let (_n, fresh) = reconcile(&state, &fed_at(0), world(1));
        assert_eq!(fresh, vec![hold("converged")]);

        // Right on the threshold, it still gets the benefit of the doubt: the
        // comparison is strictly greater, so the tick landing exactly on the
        // deadline does not accuse yet.
        let (_n, edge) = reconcile(&state, &fed_at(0), world(CLOUD_STALE_AFTER_SECS));
        assert_eq!(
            edge,
            vec![hold("converged")],
            "el umbral no muerde antes de tiempo"
        );

        // One second later: blind, with its own reason.
        let (_n, stale) = reconcile(&state, &fed_at(0), world(CLOUD_STALE_AFTER_SECS + 1));
        assert_eq!(
            stale,
            vec![hold(CLOUD_STALE_REASON)],
            "una caché de nube envejecida no es convergencia"
        );
        assert!(
            acts(&stale).is_empty(),
            "pero sigue sin inventarse acciones"
        );

        // With no cloud to observe (self-hosted, CLI daemon) there is nothing to
        // declare stale, however far `now` is from the epoch.
        let no_feed = Observation {
            local_fingerprint: Some(0xABCD),
            ..quiet_obs()
        };
        let (_n, headless) = reconcile(&state, &no_feed, world(100_000));
        assert_eq!(
            headless,
            vec![hold("converged")],
            "sin contexto de nube no se reporta obsolescencia"
        );
    }

    /// "I have never heard anything from the cloud" is the worst blindness of
    /// all, and until this it was the only one slipping through as `converged`:
    /// the old `is_some_and` over the feed stamp let the `None` past. With a cloud
    /// context, the countdown runs from when the engine started expecting heads.
    #[test]
    fn d11_never_heard_from_the_cloud_is_stale_too() {
        let state = State {
            known_version: Some(120),
            synced_fingerprint: Some(0xABCD),
            ..base_state()
        };
        // Cloud context, zero feeds: the engine started at `at(0)`.
        let blind = Observation {
            local_fingerprint: Some(0xABCD),
            cloud_feed_expected_since: Some(at(0)),
            ..quiet_obs()
        };

        // Inside the startup allowance: silence, still normal.
        let (_n, booting) = reconcile(&state, &blind, world(CLOUD_STALE_AFTER_SECS));
        assert_eq!(
            booting,
            vec![hold("converged")],
            "el margen de arranque no acusa antes de tiempo"
        );

        // Past the allowance with not one head: blind, with the same reason as a
        // stale feed (to the UI and to the replay it is the same fault).
        let (_n, blind_ds) = reconcile(&state, &blind, world(CLOUD_STALE_AFTER_SECS + 1));
        assert_eq!(blind_ds, vec![hold(CLOUD_STALE_REASON)]);
        assert!(
            acts(&blind_ds).is_empty(),
            "y sigue sin inventarse acciones"
        );

        // A real feed beats the startup anchor: the fresh stamp rejuvenates the
        // observation even with the engine up for hours.
        let fed = Observation {
            cloud_version_as_of: Some(at(10_000)),
            ..blind.clone()
        };
        let (_n, ds_fed) = reconcile(&state, &fed, world(10_001));
        assert_eq!(ds_fed, vec![hold("converged")]);

        // And with no cloud context (self-hosted), the same silence never
        // accuses: the distinction is the context, not `None` versus `Some`.
        let selfhosted = Observation {
            cloud_feed_expected_since: None,
            ..blind
        };
        let (_n, ds_self) = reconcile(&state, &selfhosted, world(100_000));
        assert_eq!(ds_self, vec![hold("converged")]);
    }

    /// The "the engine refreshes before declaring itself blind" relation does NOT
    /// live here: it is a `const _: () = assert!(...)` next to
    /// [`CLOUD_SELF_OBSERVE_AFTER_SECS`], which is strictly stronger than a test,
    /// since the crate does not compile if anyone inverts the numbers. This test
    /// only pins that the new threshold still derives from the real poll cadence
    /// (like the staleness one, see the D.10 test) and not from a loose literal
    /// that can drift.
    #[test]
    fn d12_self_observation_threshold_derives_from_the_poll_cadence() {
        assert_eq!(
            CLOUD_SELF_OBSERVE_AFTER_SECS,
            CLOUD_POLL_INTERVAL_SECS * 3 / 2
        );
    }

    /// The threshold derives from the poll cadence, not from a loose number: if
    /// the interval changes tomorrow, the threshold follows. (The "more than one
    /// missed poll" floor is checked at compile time, next to the constant.)
    #[test]
    fn d10_stale_threshold_derives_from_the_poll_cadence() {
        assert_eq!(
            CLOUD_STALE_AFTER_SECS,
            CLOUD_POLL_INTERVAL_SECS * CLOUD_STALE_AFTER_POLLS
        );
    }

    /// Staleness ONLY changes the rest's reason. A dead poller cannot brake the
    /// upload: that would trade an invisible failure for data loss, leaving local
    /// progress unversioned. Nor does it brake a restore we already know is due.
    #[test]
    fn d10_stale_cloud_cache_does_not_stop_syncing() {
        let ancient = Some(at(-10 * CLOUD_STALE_AFTER_SECS));

        // There is divergent local progress: it uploads regardless.
        let pending = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            known_version: Some(120),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(120),
            cloud_version_as_of: ancient,
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&pending, &obs, world(0));
        assert_eq!(
            acts(&ds),
            vec![&Action::Backup],
            "una caché ciega no puede dejar el progreso local sin versionar: {ds:?}"
        );

        // And an empty folder still fires the restore off the old cache: what we
        // know still counts, we just know there may be more.
        let empty = Observation {
            local_empty: true,
            cloud_version: Some(121),
            cloud_version_as_of: ancient,
            ..quiet_obs()
        };
        let (_n, ds_empty) = reconcile(&base_state(), &empty, world(0));
        assert_eq!(acts(&ds_empty), vec![&Action::Restore]);
    }

    /// track-only: never syncs anything.
    #[test]
    fn track_only_never_acts() {
        let state = State {
            track_only: true,
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(99),
            fs_event: true,
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &obs, world(0));
        assert!(acts(&ds).is_empty());
        assert_eq!(ds, vec![hold("track-only entry")]);
    }

    // ---- invariants (proptest with shrinking)

    prop_compose! {
        fn arb_failures()(
            consecutive in 0u32..6,
            version in prop::option::of(0i64..20),
            stuck in any::<bool>(),
        ) -> RestoreFailures {
            RestoreFailures { consecutive, version, stuck_notified: stuck }
        }
    }

    prop_compose! {
        /// An arbitrary conflict escalation. It goes into the generator because
        /// its `needs_attention` brakes the upload: the invariants have to hold
        /// for a slot that gave up as well as a healthy one.
        fn arb_conflicts()(
            consecutive in 0u32..8,
            version in prop::option::of(0i64..20),
            needs_attention in any::<bool>(),
        ) -> ConflictStall {
            ConflictStall { consecutive, version, needs_attention }
        }
    }

    prop_compose! {
        /// Arbitrary state with times anchored to `BASE` (bounded offsets).
        fn arb_state()(
            track_only in any::<bool>(),
            restore_enabled in any::<bool>(),
            is_running in any::<bool>(),
            running_seen in prop::option::of(-100i64..100),
            has_pending in any::<bool>(),
            fs_at in prop::option::of(-100i64..100),
            restore_at in prop::option::of(-100i64..100),
            known_version in prop::option::of(0i64..20),
            synced_fp in prop::option::of(0u64..8),
            backup_at in prop::option::of(-100i64..100),
            in_flight in prop::option::of(prop_oneof![Just(Op::Backup), Just(Op::Restore)]),
            next_backup in prop::option::of(-100i64..200),
            next_restore in prop::option::of(-100i64..200),
            pull_pending in any::<bool>(),
            deferred_notified in any::<bool>(),
            min_interval in 0u64..120,
            // The burst goes into the generator rather than being a default: the
            // adaptive floor changes when an upload may go, so the invariants
            // (idempotence, convergence) have to survive it too.
            burst_since in prop::option::of(-1200i64..0),
            burst_backups in 0u32..8,
            failures in arb_failures(),
            conflicts in arb_conflicts(),
        ) -> State {
            State {
                track_only,
                restore_enabled,
                is_running,
                last_running_seen: running_seen.map(at),
                has_pending,
                last_fs_event_at: fs_at.map(at),
                last_restore_at: restore_at.map(at),
                known_version,
                synced_fingerprint: synced_fp,
                last_backup_at: backup_at.map(at),
                in_flight,
                next_backup_at: next_backup.map(at),
                next_restore_at: next_restore.map(at),
                pull_pending,
                deferred_notified,
                min_backup_interval_secs: min_interval,
                burst_since: burst_since.map(at),
                burst_backups,
                restore_failures: failures,
                backup_conflict: conflicts,
            }
        }
    }

    prop_compose! {
        /// An arbitrary observation. `quiescent` forces the one-shot signals (fs,
        /// op, upload) to `false`/`None`: the stable world for the idempotence
        /// invariant.
        fn arb_obs(quiescent: bool)(
            mtime in prop::option::of(-100i64..100),
            size in prop::option::of(0u64..1_000),
            local_empty in any::<bool>(),
            local_fp in prop::option::of(0u64..8),
            process_alive in any::<bool>(),
            cloud_version in prop::option::of(0i64..20),
            // Covers the fresh feed, the stale one and the deployment with no
            // poller, so the invariants hold with a blind cloud cache too.
            cloud_as_of in prop::option::of(-2 * CLOUD_STALE_AFTER_SECS..100),
            // Same for the context: with a cloud to observe (inside and outside
            // the startup allowance) and without one.
            cloud_expected in prop::option::of(-2 * CLOUD_STALE_AFTER_SECS..100),
            fs_event in any::<bool>(),
            retry in 0u32..600,
            has_op in any::<bool>(),
            op_kind in 0u8..5,
            ok_ver in prop::option::of(0i64..20),
            ok_fp in prop::option::of(0u64..8),
            ok_wrote in any::<bool>(),
        ) -> Observation {
            let op_result = if quiescent || !has_op {
                None
            } else {
                Some(match op_kind {
                    0 => OpResult::Ok { version: ok_ver, fingerprint: ok_fp, wrote: ok_wrote },
                    1 => OpResult::NotFound,
                    2 => OpResult::Unauthorized,
                    3 => OpResult::Throttled { retry_after_secs: retry },
                    _ => OpResult::Failed,
                })
            };
            Observation {
                folder_mtime: mtime.map(at),
                folder_size: size,
                local_empty,
                local_fingerprint: local_fp,
                process_alive,
                // The proptest does not model the lock probe: it is a shell-side
                // pacing brake (only Windows can assert it) and leaving it always
                // false keeps the state space to what this test covers.
                save_files_locked: false,
                cloud_version,
                cloud_version_as_of: cloud_as_of.map(at),
                cloud_feed_expected_since: cloud_expected.map(at),
                fs_event: if quiescent { false } else { fs_event },
                op_result,
                upload_landed: None,
            }
        }
    }

    fn arb_world() -> impl Strategy<Value = World> {
        (-100i64..300, any::<u64>()).prop_map(|(now_off, seed)| World {
            now: at(now_off),
            seed,
        })
    }

    proptest! {
        /// At most one storage action (Backup or Restore) per tick.
        #[test]
        fn inv_storage_acts_bounded(state in arb_state(), obs in arb_obs(false), w in arb_world()) {
            let (_n, ds) = reconcile(&state, &obs, w);
            prop_assert!(storage_act_count(&ds) <= 1, "más de una acción de storage: {ds:?}");
        }

        /// Backup and Restore never in the same tick; they must not fight.
        #[test]
        fn inv_backup_restore_mutually_exclusive(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            let a = acts(&ds);
            prop_assert!(
                !(a.contains(&&Action::Backup) && a.contains(&&Action::Restore)),
                "backup y restore juntos: {ds:?}"
            );
        }

        /// Never `Act(Restore)` mid-session or over unversioned local content
        /// (the R.E.P.O. data loss plus never-lose-newer-local). If it restores,
        /// the resulting state is neither running nor pending.
        #[test]
        fn inv_restore_never_mid_session(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (next, ds) = reconcile(&state, &obs, w);
            if acts(&ds).contains(&&Action::Restore) {
                prop_assert!(!next.is_running, "restore con juego corriendo: {ds:?}");
                prop_assert!(!next.has_pending, "restore sobre local sin versionar: {ds:?}");
            }
        }

        /// The base and dynamic invariant (C.1/C.2): under quiescent input the
        /// reducer is idempotent. Re-applying it to its own output at the same
        /// `now` emits no `Act` at all. This kills the hot loop: no action without
        /// a new delta. (A tick's deltas, fs and op, are excluded for being
        /// exactly that.)
        #[test]
        fn inv_idempotent_under_quiescence(
            state in arb_state(), obs in arb_obs(true), w in arb_world()
        ) {
            let (s1, _d1) = reconcile(&state, &obs, w);
            let (_s2, d2) = reconcile(&s1, &obs, w);
            prop_assert!(
                acts(&d2).is_empty(),
                "acción sin delta al reconciliar sobre la propia salida: {d2:?}"
            );
        }

        /// D.8.1: with unversioned local changes, divergent content, nothing in
        /// flight and pacing satisfied, the tick ALWAYS emits the upload, which is
        /// the only way to clear `has_pending`. No restore branch (cooldown, veto,
        /// deferred pull) may swallow it: that was the deadlock the shell unstuck
        /// by hand.
        ///
        /// The only exception is a save that gave up (`needs_attention`), where
        /// not emitting the upload is the decision rather than an oversight. That
        /// is the difference between the two ways of being stopped: stalled with
        /// nothing saying so (the bug), and stopped, said out loud, with three
        /// ways out (a manual copy, a successful copy, a new cloud head).
        #[test]
        fn inv_pending_local_changes_always_get_a_backup(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            let eligible = !state.track_only
                && state.in_flight.is_none()
                && obs.op_result.is_none()
                && !state.backup_conflict.needs_attention
                && (state.has_pending || obs.fs_event)
                && local_diverged(&state, &obs)
                && state.next_backup_at.is_none_or(|t| w.now >= t)
                && backup_floor(&state).is_none_or(|t| w.now >= t);
            if eligible {
                prop_assert!(
                    acts(&ds).contains(&&Action::Backup),
                    "cambios pendientes sin subida: el slot queda encallado: {ds:?}"
                );
            }
        }

        /// Never `Act(Backup)` with a restore in flight; do not upload while
        /// downloading. Anti-relaunch holds every op in flight.
        #[test]
        fn inv_no_backup_while_restoring(
            state in arb_state(), obs in arb_obs(true), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            if state.in_flight == Some(Op::Restore) && obs.op_result.is_none() {
                prop_assert!(
                    !acts(&ds).contains(&&Action::Backup),
                    "backup mientras un restore está en vuelo: {ds:?}"
                );
            }
        }
    }
}
