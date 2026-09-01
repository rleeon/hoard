//! The daemon's journal: the ring from [`hoard_core::ipc::journal`] plus the live
//! push channel.
//!
//! Both halves of event delivery (ADR 0021 D.14.2) share a single write:
//! [`EventLog::record`] stores **and** pushes. Storing serves the client that was
//! away (it asks for its cursor on connect); pushing serves the one that is
//! connected. What never gets pushed is a collapse (a run of the same idle state),
//! because by definition it says nothing new.
//!
//! ## The Slice 5 boundary
//!
//! All the state sits behind this facade: `record`, `since`, `cursor`,
//! `subscribe`. When the journal moves to the ring table in the daemon's private
//! SQLite (Slice 5, which is also C.5's decision log), the bodies of these four
//! methods change and nothing else does: the IPC server does not know where the
//! rows live.

use std::sync::Mutex;

use hoard_core::ipc::journal::{Appended, Backlog, Journal, JournalEntry, DEFAULT_CAPACITY};
use hoard_core::ipc::AgentEvent;
use time::OffsetDateTime;
use tokio::sync::broadcast;

/// Rows in flight per subscriber before it counts as lagging. A normal client
/// drains in microseconds; the ceiling exists so a stuck one cannot grow without
/// bound in the daemon's memory. Past it, the client is sent a `Resync` and asks
/// again by cursor, so falling behind loses nothing.
const PUSH_BUFFER: usize = 256;

pub struct EventLog {
    journal: Mutex<Journal>,
    tx: broadcast::Sender<JournalEntry>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(PUSH_BUFFER);
        Self {
            journal: Mutex::new(Journal::with_capacity(capacity)),
            tx,
        }
    }

    /// Records an engine event and, when it opened a row, pushes it to the
    /// subscribers.
    pub fn record(&self, at: OffsetDateTime, event: AgentEvent) {
        // The lock covers the append only (no await inside), so a slow subscriber
        // cannot block the engine.
        let appended = {
            let mut journal = self.lock();
            journal.append(at, event)
        };
        match appended {
            Appended::Recorded(entry) => {
                // `send` falla cuando no hay suscriptores: es el caso normal
                // (daemon sin clientes) y no un error.
                let _ = self.tx.send(entry);
            }
            Appended::Collapsed { seq, repeat } => {
                tracing::trace!(seq, repeat, "hoardd: collapsed a repeated rest event");
            }
        }
    }

    pub fn since(&self, cursor: u64) -> Backlog {
        self.lock().since(cursor)
    }

    pub fn cursor(&self) -> u64 {
        self.lock().cursor()
    }

    pub fn dropped(&self) -> u64 {
        self.lock().dropped()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<JournalEntry> {
        self.tx.subscribe()
    }

    /// Recovers the poisoned mutex instead of propagating the panic. A panic inside
    /// the append would leave the journal poisoned and **every** later event would
    /// fall on the floor, which is exactly how D.11's poller went mute
    /// (`.lock().unwrap()` on a poisoned mutex). The rows are append-only: the worst
    /// that can have happened is half a row half written, not a state that poisons
    /// what comes next.
    fn lock(&self) -> std::sync::MutexGuard<'_, Journal> {
        self.journal.lock().unwrap_or_else(|poisoned| {
            tracing::error!("hoardd: the journal mutex was poisoned; recovering");
            poisoned.into_inner()
        })
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(save: &str) -> AgentEvent {
        AgentEvent::GameStarted {
            save_id: save.to_string(),
            game_slug: "factorio".to_string(),
        }
    }

    fn deferred() -> AgentEvent {
        AgentEvent::RestoreDeferred {
            save_id: "s1".to_string(),
            game_slug: "factorio".to_string(),
            reason: "game is running".to_string(),
        }
    }

    #[tokio::test]
    async fn recording_pushes_new_rows_and_swallows_collapses() {
        let log = EventLog::new();
        let mut rx = log.subscribe();
        let now = OffsetDateTime::now_utc();

        log.record(now, started("s1"));
        log.record(now, deferred());
        // Three identical idles: one row, one push.
        log.record(now, deferred());
        log.record(now, deferred());

        let first = rx.try_recv().unwrap();
        assert!(matches!(first.event, AgentEvent::GameStarted { .. }));
        let second = rx.try_recv().unwrap();
        assert!(matches!(second.event, AgentEvent::RestoreDeferred { .. }));
        assert!(rx.try_recv().is_err(), "a collapse must not be pushed");

        // The counter is in the journal, though, for the client that arrives late.
        let backlog = log.since(0);
        assert_eq!(backlog.entries.len(), 2);
        assert_eq!(backlog.entries[1].repeat, 3);
        assert_eq!(log.cursor(), 2);
    }

    #[tokio::test]
    async fn a_late_subscriber_catches_up_by_cursor() {
        let log = EventLog::new();
        let now = OffsetDateTime::now_utc();
        log.record(now, started("a"));
        log.record(now, started("b"));

        // It subscribes afterwards, so the push brings it nothing from the past...
        let mut rx = log.subscribe();
        assert!(rx.try_recv().is_err());
        // ...but the backlog does. That is the mute-bell bug, closed by
        // construction.
        let backlog = log.since(0);
        assert_eq!(backlog.entries.len(), 2);
        assert!(!backlog.gap);

        log.record(now, started("c"));
        let live = rx.try_recv().unwrap();
        assert_eq!(live.seq, 3);
    }
}
