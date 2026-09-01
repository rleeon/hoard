//! Append-only journal with a cursor: the "client that wasn't there" half of
//! event delivery (ADR 0021, D.14.2).
//!
//! Pushing over the socket serves the client connected *now*. It does nothing
//! for the one that starts late, and push-only is exactly the silent-bell bug,
//! a UI with neither a snapshot nor a backlog. So the daemon records what
//! happens and the client asks for "everything after cursor N" before it starts
//! listening live.
//!
//! ## What does not get recorded: repeated rests
//!
//! Recording every decision of every tick amplifies writes absurdly. Measured in
//! this repo on 2026-07-25: 3015 `cloud state stale` entries in 36 minutes,
//! about 84 a minute, over 100k a day on a 2 s tick, and the disk being
//! protected is the Deck's SSD. The rule is to record transitions and actions,
//! not repeated rests, and to collapse runs of the same reason into one row with
//! a counter.
//!
//! That is [`collapse_key`]. Rest and veto events (the engine is waiting on
//! something, and is still waiting on the same thing) collapse onto the tail row
//! with a counter; transitions and actions always get their own row. A collapse
//! is never pushed live, because by definition there is nothing new to say.
//!
//! ## Where Slice 5 takes over
//!
//! No IO here: [`JournalEntry`] is the wire type, [`collapse_key`] is the
//! policy, [`Journal`] is a capped in-memory ring. Slice 5 swaps the *store*
//! only, for a ring table in the daemon's private SQLite, which is also the C.5
//! decision log. The type and the policy stay put. Nothing that needs a disk
//! belongs in this file.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::events::AgentEvent;

/// Sized for the gap it actually has to cover: a reconnecting client takes
/// seconds, not hours. A client asking for further back than the ring still
/// holds is told so ([`Backlog::gap`]) rather than handed a partial history and
/// left to believe it is complete.
pub const DEFAULT_CAPACITY: usize = 1024;

/// `seq` is the cursor: monotonic, gapless, per daemon run. A restarted daemon
/// starts over at 1, and the `epoch` in [`super::Welcome`] is what tells the
/// client its old cursor is worthless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub seq: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// When the *last* occurrence was seen; equal to `at` while `repeat == 1`.
    #[serde(with = "time::serde::rfc3339")]
    pub last_at: OffsetDateTime,
    /// Occurrences collapsed into this row, starting at 1. Only a rest event can
    /// ever go above 1 (see [`collapse_key`]).
    pub repeat: u32,
    pub event: AgentEvent,
}

#[derive(Debug, Clone)]
pub enum Appended {
    /// A new row, and the only thing that gets pushed live.
    Recorded(JournalEntry),
    /// A run of the same rest, added to row `seq`'s counter. Not pushed: the
    /// client already knows the engine is waiting on that.
    Collapsed { seq: u64, repeat: u32 },
}

/// The answer to "give me everything after cursor N".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlog {
    pub entries: Vec<JournalEntry>,
    /// Cursor after applying `entries`: the last row's `seq`, whether or not
    /// anything new came back.
    pub cursor: u64,
    /// The journal no longer holds everything the client asked for, either
    /// because it fell off the ring or because the cursor belongs to another
    /// daemon run. The client must re-seed with [`super::Request::Status`]
    /// instead of assuming continuity. Lying here is how a history goes missing
    /// with nobody noticing.
    #[serde(default)]
    pub gap: bool,
}

#[derive(Debug)]
pub struct Journal {
    entries: VecDeque<JournalEntry>,
    capacity: usize,
    /// Starts at 1 so that `cursor == 0` unambiguously means "I have never seen
    /// anything".
    next_seq: u64,
    dropped: u64,
}

impl Default for Journal {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl Journal {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity: capacity.max(1),
            next_seq: 1,
            dropped: 0,
        }
    }

    pub fn cursor(&self) -> u64 {
        self.next_seq - 1
    }

    /// Rows that fell off the ring since startup. Diagnostic, and the signal
    /// that the cap is sized wrong.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Records `event`, or bumps the tail row's counter when it is a rest
    /// identical to the one already there.
    pub fn append(&mut self, at: OffsetDateTime, event: AgentEvent) -> Appended {
        if let Some(key) = collapse_key(&event) {
            if let Some(tail) = self.entries.back_mut() {
                if collapse_key(&tail.event).as_deref() == Some(key.as_str()) {
                    tail.repeat = tail.repeat.saturating_add(1);
                    tail.last_at = at;
                    return Appended::Collapsed {
                        seq: tail.seq,
                        repeat: tail.repeat,
                    };
                }
            }
        }
        let entry = JournalEntry {
            seq: self.next_seq,
            at,
            last_at: at,
            repeat: 1,
            event,
        };
        self.next_seq += 1;
        self.entries.push_back(entry.clone());
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
            self.dropped += 1;
        }
        Appended::Recorded(entry)
    }

    /// Everything after `cursor`, flagging [`Backlog::gap`] when the full
    /// stretch cannot be served.
    pub fn since(&self, cursor: u64) -> Backlog {
        let entries: Vec<JournalEntry> = self
            .entries
            .iter()
            .filter(|e| e.seq > cursor)
            .cloned()
            .collect();
        // Two different reasons for a gap: the stretch asked for already fell
        // off the ring, or the cursor comes from the future, which usually means
        // the client kept a previous daemon's cursor. The handshake `epoch` is
        // the real detection; this is the belt to its braces.
        let oldest = self.entries.front().map(|e| e.seq);
        let lost = match oldest {
            Some(first) => cursor + 1 < first,
            None => self.dropped > 0 && cursor < self.cursor(),
        };
        let from_the_future = cursor > self.cursor();
        Backlog {
            entries,
            cursor: self.cursor(),
            gap: lost || from_the_future,
        }
    }
}

/// `Some` for rest events, where the engine is still waiting on the same thing,
/// `None` for transitions and actions, which always earn a row.
///
/// The key is the event's own JSON, so only *identical* repetitions collapse: a
/// different veto reason, error or game opens a new row. Serialising instead of
/// listing fields by hand is deliberate, since adding a field to a variant then
/// cannot forget to update the key.
///
/// What collapses, and why:
///
/// - `RestoreDeferred`, the mid-session veto. The sweep re-evaluates it every
///   tick and re-emits it while the game lives; this is the 3015-in-36-minutes
///   case in miniature.
/// - `SaveAutoRestoreFailed`. The same error over and over is the sweep
///   retrying, not N separate incidents. This is what flooded the feed in July.
/// - `SaveAutoRestoreStuck`, already one-shot per (save, version); collapsing
///   makes it idempotent if the shell re-emits.
/// - `BackupNeedsAttention`, the same: one-shot per edge, and a restarted engine
///   rebuilds the slot state from scratch and can cross the edge again.
/// - `BackupThrottled`, waiting on the server's bandwidth window. A rest with a
///   reason; only a different `retry_after_secs` opens a row.
/// - `BackupQuotaFull`. Every save discovers a full account on its own and the
///   park re-emits hourly, so these are N reports of one fact rather than N
///   incidents. It collapses on the figures, so the row refreshes once the user
///   frees something and it still is not enough.
/// - `BackupFilesUnreadable`. The same unreadable file comes back on every copy
///   for as long as the cause lasts, and a stalled on-demand file provider can
///   last weeks. One warning, not one per copy. It collapses on content, so a
///   different file or a different error opens a row.
/// - `HeavyProcessDetected`. Seeing the same heavy process again is not a new
///   discovery.
///
/// What does not collapse even when it repeats: `BackupScheduled`. It looks like
/// a rest, but every emission is new information, because the debounce *reset*
/// and the countdown in the UI starts over. Collapsing it would silence that
/// refresh, since collapses are never pushed. Transitions and real actions do
/// not collapse either; they are the history.
pub fn collapse_key(event: &AgentEvent) -> Option<String> {
    // `BackupQuotaFull` is the only one that collapses *across* saves: the fact
    // belongs to the account, not to the save, so twenty games hitting the same
    // wall are one row rather than twenty. Hence the hand-built key; serialising
    // the whole event would drag in `save_id` and split them apart again.
    if let AgentEvent::BackupQuotaFull {
        plan,
        used_bytes,
        limit_bytes,
        ..
    } = event
    {
        return Some(format!("quota_full:{plan}:{used_bytes}:{limit_bytes}"));
    }
    let restful = matches!(
        event,
        AgentEvent::RestoreDeferred { .. }
            | AgentEvent::SaveAutoRestoreFailed { .. }
            | AgentEvent::SaveAutoRestoreStuck { .. }
            | AgentEvent::BackupThrottled { .. }
            | AgentEvent::BackupFilesUnreadable { .. }
            | AgentEvent::BackupNeedsAttention { .. }
            | AgentEvent::HeavyProcessDetected { .. }
    );
    if !restful {
        return None;
    }
    serde_json::to_string(event).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_753_000_000 + secs).unwrap()
    }

    fn deferred(save: &str, reason: &str) -> AgentEvent {
        AgentEvent::RestoreDeferred {
            save_id: save.to_string(),
            game_slug: "factorio".to_string(),
            reason: reason.to_string(),
        }
    }

    fn started(save: &str) -> AgentEvent {
        AgentEvent::GameStarted {
            save_id: save.to_string(),
            game_slug: "factorio".to_string(),
        }
    }

    #[test]
    fn cursor_starts_empty_and_advances_by_one() {
        let mut j = Journal::default();
        assert_eq!(j.cursor(), 0);
        j.append(ts(0), started("a"));
        assert_eq!(j.cursor(), 1);
        j.append(ts(1), started("b"));
        assert_eq!(j.cursor(), 2);
    }

    /// The measured case: a run of identical rests is one row with a counter.
    /// 3015 holds in 36 minutes cannot be 3015 writes.
    #[test]
    fn a_run_of_the_same_rest_collapses_into_one_row() {
        let mut j = Journal::default();
        let first = j.append(ts(0), deferred("s1", "game is running"));
        assert!(matches!(first, Appended::Recorded(_)));
        for i in 1..3015 {
            match j.append(ts(i), deferred("s1", "game is running")) {
                Appended::Collapsed { seq, repeat } => {
                    assert_eq!(seq, 1);
                    assert_eq!(repeat as i64, i + 1);
                }
                Appended::Recorded(_) => panic!("identical rest must not open a new row"),
            }
        }
        assert_eq!(j.len(), 1);
        assert_eq!(j.cursor(), 1);
        let row = &j.since(0).entries[0];
        assert_eq!(row.repeat, 3015);
        assert_eq!(row.at, ts(0));
        assert_eq!(row.last_at, ts(3014));
    }

    /// A different reason is new information, so it gets its own row.
    #[test]
    fn a_different_reason_opens_a_new_row() {
        let mut j = Journal::default();
        j.append(ts(0), deferred("s1", "game is running"));
        j.append(ts(1), deferred("s1", "local changes pending"));
        assert_eq!(j.len(), 2);
    }

    /// Transitions and actions never collapse. They are the history the late
    /// client came for.
    #[test]
    fn transitions_never_collapse() {
        let mut j = Journal::default();
        for i in 0..3 {
            assert!(matches!(
                j.append(ts(i), started("s1")),
                Appended::Recorded(_)
            ));
        }
        assert_eq!(j.len(), 3);
        assert!(collapse_key(&started("s1")).is_none());
        assert!(collapse_key(&AgentEvent::BackupScheduled {
            save_id: "s1".into(),
            delay_ms: 5000,
            reason: crate::ipc::events::BackupReason::FilesystemSettled,
        })
        .is_none());
    }

    /// Collapsing only ever looks at the tail, never at a buried row, because
    /// reaching back would reorder the history.
    #[test]
    fn collapsing_only_looks_at_the_tail() {
        let mut j = Journal::default();
        j.append(ts(0), deferred("s1", "game is running"));
        j.append(ts(1), started("s1"));
        j.append(ts(2), deferred("s1", "game is running"));
        assert_eq!(j.len(), 3);
        assert_eq!(j.since(0).entries[2].repeat, 1);
    }

    #[test]
    fn since_returns_only_newer_rows() {
        let mut j = Journal::default();
        j.append(ts(0), started("a"));
        j.append(ts(1), started("b"));
        j.append(ts(2), started("c"));
        let back = j.since(1);
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.entries[0].seq, 2);
        assert_eq!(back.cursor, 3);
        assert!(!back.gap);
        // Up to date: nothing new, and no gap.
        let none = j.since(3);
        assert!(none.entries.is_empty());
        assert!(!none.gap);
    }

    /// What the ring throws away is reported as a gap, not papered over.
    #[test]
    fn dropping_old_rows_reports_a_gap() {
        let mut j = Journal::with_capacity(2);
        j.append(ts(0), started("a"));
        j.append(ts(1), started("b"));
        j.append(ts(2), started("c"));
        assert_eq!(j.len(), 2);
        assert_eq!(j.dropped(), 1);
        let from_scratch = j.since(0);
        assert!(from_scratch.gap);
        assert_eq!(from_scratch.entries.len(), 2);
        // A client that already had row 1 lost nothing.
        assert!(!j.since(1).gap);
    }

    /// A cursor from the future, usually a previous daemon's, is a gap, so the
    /// client re-seeds instead of waiting for events that already happened.
    #[test]
    fn a_cursor_from_the_future_is_a_gap() {
        let mut j = Journal::default();
        j.append(ts(0), started("a"));
        assert!(j.since(99).gap);
    }
}
