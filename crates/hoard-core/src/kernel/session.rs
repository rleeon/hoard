//! The "user is mid-session" veto. The shell samples `now` and the folder mtime
//! and passes them in as data ([`World`] / [`Observation`]); this decides.

use time::{Duration, OffsetDateTime};

use super::{Action, Decision, Observation, State, World};

/// Grace window for the "save was touched recently" heuristic. Five minutes, as
/// accepted in ADR 0014: while someone is playing, the process poll usually
/// marks the slot `is_running` anyway, so this covers the slot that matches no
/// process in the catalogue.
pub const RECENT_SAVE_GRACE_SECS: i64 = 5 * 60;

/// The same window as a [`Duration`], for the full-precision folder mtime
/// comparison.
const RECENT_SAVE_GRACE: Duration = Duration::seconds(RECENT_SAVE_GRACE_SECS);

/// Is the user mid-session, such that a pull could walk over progress the backup
/// has not captured yet?
///
/// Guards fire in order:
///  1. `is_running`, the game is running right now.
///  2. `save_files_locked`, a save file is held open elsewhere.
///  3. `has_pending`, unversioned local changes.
///  4. a recent `last_fs_event_at`, the watcher saw a write (inotify catches
///     in-place rewrites that never move the directory mtime).
///  5. a recent folder mtime, the fallback for the startup window before the
///     agent has any fs history.
///
/// A recent `last_restore_at` suppresses 4 and 5: our own auto-restore touches
/// the folder and emits fs events, and without that suppression a restore would
/// veto the *next* pull for the whole grace window, throttling cross-device
/// saves to one per window (v101 restored, v102 blocked for five minutes). The
/// live-session guards are never suppressed, because `is_running` and
/// `has_pending` mean real progress is at stake.
pub fn mid_session_decision(state: &State, obs: &Observation, world: &World) -> Decision {
    if let Some(reason) = veto_reason(state, obs, world) {
        Decision::Hold { reason }
    } else {
        Decision::Act(Action::Pull)
    }
}

/// The first guard that fires, or `None` when the slot is quiet. Split out so
/// the wrapper in `agent.rs` can keep returning `Option<&'static str>` without
/// remapping the decision.
pub fn veto_reason(state: &State, obs: &Observation, world: &World) -> Option<&'static str> {
    if state.is_running {
        return Some("game process is running");
    }
    // A save file held open exclusively means "the game is writing", asserted by
    // the filesystem rather than by recognising a process. It sits right behind
    // `is_running` because it means the same thing, and it covers what
    // `is_running` cannot see: the game whose executable matches nothing.
    if obs.save_files_locked {
        return Some("save files are open in another process");
    }
    if state.has_pending {
        return Some("un-flushed local changes pending");
    }
    let now = world.now;
    let touch_is_ours = state
        .last_restore_at
        .is_some_and(|r| (now - r).whole_seconds() < RECENT_SAVE_GRACE_SECS);
    if !touch_is_ours {
        if let Some(last) = state.last_fs_event_at {
            if (now - last).whole_seconds() < RECENT_SAVE_GRACE_SECS {
                return Some("fs event observed recently");
            }
        }
        if folder_touched_recently(obs.folder_mtime, now) {
            return Some("save folder touched recently");
        }
    }
    None
}

/// An mtime counts as recent only if it is not in the future (`mtime <= now`,
/// matching `SystemTime::duration_since` returning `Ok`) and its age fits inside
/// the grace window at full precision.
fn folder_touched_recently(folder_mtime: Option<OffsetDateTime>, now: OffsetDateTime) -> bool {
    match folder_mtime {
        Some(mtime) => {
            let age = now - mtime;
            !age.is_negative() && age < RECENT_SAVE_GRACE
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> State {
        State::default()
    }

    /// An aged folder, so the disk fallback does not fire on its own and the
    /// other guards can be tested in isolation.
    fn aged_folder(now: OffsetDateTime) -> Observation {
        Observation {
            folder_mtime: Some(now - Duration::hours(1)),
            ..Default::default()
        }
    }

    const NOW: OffsetDateTime = OffsetDateTime::UNIX_EPOCH;

    fn world(now: OffsetDateTime) -> World {
        World { now, seed: 0 }
    }

    /// The guard shared by the sweep, the forced restore and the launch barrier,
    /// after the 2026-07-05 data-loss regression: any sign of a live session
    /// vetoes a pull, and a genuinely quiet slot does not.
    #[test]
    fn flags_live_session_signals() {
        let w = world(NOW);
        let obs = aged_folder(NOW);

        let mut state = quiet();
        assert_eq!(
            mid_session_decision(&state, &obs, &w),
            Decision::Act(Action::Pull),
            "quiet slot must be pullable"
        );

        state.is_running = true;
        assert_eq!(
            mid_session_decision(&state, &obs, &w),
            Decision::Hold {
                reason: "game process is running"
            },
        );
        state.is_running = false;

        state.has_pending = true;
        assert!(matches!(
            mid_session_decision(&state, &obs, &w),
            Decision::Hold { .. }
        ));
        state.has_pending = false;

        state.last_fs_event_at = Some(NOW);
        assert!(matches!(
            mid_session_decision(&state, &obs, &w),
            Decision::Hold {
                reason: "fs event observed recently"
            }
        ));
        state.last_fs_event_at = Some(NOW - Duration::hours(1));
        assert_eq!(
            mid_session_decision(&state, &obs, &w),
            Decision::Act(Action::Pull),
            "an hour-old fs event is outside the grace window"
        );
    }

    #[test]
    fn ignores_own_restore_touch() {
        let w = world(NOW);
        let fresh = Observation {
            folder_mtime: Some(NOW),
            ..Default::default()
        };
        let mut state = quiet();
        assert_eq!(
            veto_reason(&state, &fresh, &w),
            Some("save folder touched recently"),
            "a just-touched folder vetoes by default"
        );
        state.last_restore_at = Some(NOW);
        assert_eq!(
            veto_reason(&state, &fresh, &w),
            None,
            "our own recent restore must not veto the next pull"
        );
        // A real pending change still wins; it is checked before the gate.
        state.has_pending = true;
        assert_eq!(
            veto_reason(&state, &fresh, &w),
            Some("un-flushed local changes pending"),
        );
        state.has_pending = false;
        state.last_restore_at = Some(NOW - Duration::hours(1));
        assert_eq!(
            veto_reason(&state, &fresh, &w),
            Some("save folder touched recently"),
            "a restore older than the grace window stops covering the touch"
        );
    }

    #[test]
    fn falls_back_to_disk_mtime() {
        let w = world(NOW);
        let fresh = Observation {
            folder_mtime: Some(NOW),
            ..Default::default()
        };
        assert!(matches!(
            mid_session_decision(&quiet(), &fresh, &w),
            Decision::Hold {
                reason: "save folder touched recently"
            }
        ));
    }

    // ---- D.4 corpus

    /// The restore that vetoed itself for five minutes: a restore stamps
    /// `last_restore_at` and leaves the folder mtime fresh, and without the
    /// "our own touch" guard the next pull vetoed itself for the whole window.
    /// Inside the window the touch must not veto; one second past it the stamp
    /// expires and the disk fallback vetoes again.
    #[test]
    fn d4_restore_does_not_self_veto_within_grace() {
        let restore_at = OffsetDateTime::UNIX_EPOCH;
        let state = State {
            last_restore_at: Some(restore_at),
            ..quiet()
        };
        let obs = Observation {
            folder_mtime: Some(restore_at),
            ..Default::default()
        };

        let w = world(restore_at + Duration::seconds(30));
        assert_eq!(
            mid_session_decision(&state, &obs, &w),
            Decision::Act(Action::Pull),
            "a restore must not self-veto the next pull inside the window",
        );

        // Past the deadline the restore stamp expires. `now` crossing a deadline
        // is itself a delta worth re-evaluating (ADR C.2), and with the stamp
        // stale a genuinely fresh write vetoes again, as it should: the guard
        // covers the window, it does not paper over the folder forever.
        let later = restore_at + Duration::seconds(RECENT_SAVE_GRACE_SECS + 1);
        let w_later = world(later);
        let obs_fresh = Observation {
            folder_mtime: Some(later),
            ..Default::default()
        };
        assert_eq!(
            mid_session_decision(&state, &obs_fresh, &w_later),
            Decision::Hold {
                reason: "save folder touched recently"
            },
            "past the window a real touch vetoes again",
        );
    }

    /// Live-session guards beat the recency ones, and each reports its own
    /// reason so the veto shows up in the log with a cause (ADR C.5).
    #[test]
    fn d4_hold_carries_the_right_reason() {
        let w = world(NOW);
        let fresh = Observation {
            folder_mtime: Some(NOW),
            ..Default::default()
        };

        let running = State {
            is_running: true,
            ..quiet()
        };
        assert_eq!(
            mid_session_decision(&running, &fresh, &w),
            Decision::Hold {
                reason: "game process is running"
            },
        );

        let pending = State {
            has_pending: true,
            ..quiet()
        };
        assert_eq!(
            mid_session_decision(&pending, &fresh, &w),
            Decision::Hold {
                reason: "un-flushed local changes pending"
            },
        );
    }

    /// Clock skew on a network share can put the mtime in the future, and that
    /// is not evidence of a recent write.
    #[test]
    fn d4_future_folder_mtime_does_not_veto() {
        let w = world(NOW);
        let future = Observation {
            folder_mtime: Some(NOW + Duration::minutes(1)),
            ..Default::default()
        };
        assert_eq!(
            mid_session_decision(&quiet(), &future, &w),
            Decision::Act(Action::Pull),
            "a future mtime is not evidence of a recent write",
        );
    }

    /// A locked save file vetoes on its own, with no other guard firing. That is
    /// the case the others miss: the game whose executable matches nothing,
    /// saving right now.
    #[test]
    fn a_locked_save_file_vetoes_the_pull_on_its_own() {
        let w = world(NOW);
        let obs = Observation {
            save_files_locked: true,
            ..aged_folder(NOW)
        };
        assert_eq!(
            mid_session_decision(&quiet(), &obs, &w),
            Decision::Hold {
                reason: "save files are open in another process"
            },
        );
        assert_eq!(
            mid_session_decision(&quiet(), &aged_folder(NOW), &w),
            Decision::Act(Action::Pull),
        );
    }
}
