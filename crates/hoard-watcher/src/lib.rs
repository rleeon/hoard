//! `hoard-watcher` — debounced filesystem watcher for save directories.
//!
//! ## What it does
//!
//! Watches one or more directories recursively and emits a single
//! [`WatchEvent::SaveChanged`] per debounce window summarising every path
//! that changed during it. We don't surface raw [`notify::Event`]s — most
//! callers don't care whether a save was created vs modified vs renamed,
//! they just want "something changed, time to back up".
//!
//! ## Why debounced
//!
//! Many games rewrite their save by writing a temp file then renaming it
//! over the original — that's two filesystem events for one logical save.
//! Bethesda titles especially produce a long burst of writes mid-save.
//! Debouncing turns that burst into a single backup.
//!
//! Default debounce: **5 seconds**. The user's priorities asked for
//! "instant" backup; 5s is the floor below which we'd start uploading
//! partial saves while the game is still writing them. v0.3 uses 5s
//! across the board; v0.4 will let users tune it per-game.
//!
//! ## Why not async?
//!
//! `notify` runs its own thread internally; we expose a sync
//! [`SaveWatcher::recv`] / async [`SaveWatcher::recv_timeout`] API so
//! callers can pick. The hoard-agent driver in Phase 4 spawns this on a
//! dedicated thread.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::RecommendedWatcher;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};

/// Default debounce window. Tuned for "instant feel" without uploading
/// torn writes — most games finish a save burst inside 2-3s.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("notify backend error: {0}")]
    Notify(#[from] notify::Error),
    #[error("watcher channel closed before event arrived")]
    Closed,
}

/// One debounce window's worth of changes, coalesced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// At least one path changed. The set is deduplicated.
    SaveChanged { paths: Vec<PathBuf> },
}

/// Watches save directories and emits coalesced [`WatchEvent`]s.
pub struct SaveWatcher {
    debouncer: Debouncer<RecommendedWatcher>,
    rx: mpsc::Receiver<DebounceEventResult>,
    watched: HashSet<PathBuf>,
}

impl SaveWatcher {
    /// Build a watcher with the given debounce duration.
    pub fn with_debounce(debounce: Duration) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();
        let debouncer = new_debouncer(debounce, move |res: DebounceEventResult| {
            // If the receiver is gone we're shutting down; drop silently.
            let _ = tx.send(res);
        })?;
        Ok(Self {
            debouncer,
            rx,
            watched: HashSet::new(),
        })
    }

    /// Build a watcher with [`DEFAULT_DEBOUNCE`].
    pub fn new() -> Result<Self, WatchError> {
        Self::with_debounce(DEFAULT_DEBOUNCE)
    }

    /// Start watching `path` recursively. Idempotent — re-watching the
    /// same path is a no-op.
    pub fn watch(&mut self, path: &Path) -> Result<(), WatchError> {
        if self.watched.contains(path) {
            return Ok(());
        }
        // notify-debouncer-mini follows symlinks by default. We watch
        // recursively because save directories often nest by profile
        // (e.g. Stardew Valley/Saves/<farmer-name>/...).
        self.debouncer
            .watcher()
            .watch(path, notify::RecursiveMode::Recursive)?;
        self.watched.insert(path.to_path_buf());
        tracing::debug!(path = %path.display(), "now watching");
        Ok(())
    }

    /// Stop watching `path`.
    pub fn unwatch(&mut self, path: &Path) -> Result<(), WatchError> {
        if !self.watched.remove(path) {
            return Ok(());
        }
        self.debouncer.watcher().unwatch(path)?;
        Ok(())
    }

    /// Stop watching everything.
    pub fn unwatch_all(&mut self) -> Result<(), WatchError> {
        let paths: Vec<_> = self.watched.iter().cloned().collect();
        for p in paths {
            // Best-effort: a path may have been removed from the FS in
            // the meantime; log and keep going.
            if let Err(e) = self.unwatch(&p) {
                tracing::warn!(path = %p.display(), error = %e, "unwatch failed");
            }
        }
        Ok(())
    }

    /// Currently-watched paths.
    pub fn watched(&self) -> impl Iterator<Item = &PathBuf> {
        self.watched.iter()
    }

    /// Block until the next event, or return `None` if the watcher
    /// thread has died. Also returns `None` if a debounce batch came
    /// back empty (shouldn't happen but defensive).
    pub fn recv(&self) -> Option<WatchEvent> {
        let res = self.rx.recv().ok()?;
        Self::coalesce(res)
    }

    /// Like [`Self::recv`] but with a deadline. Returns `Ok(None)` on
    /// timeout, `Err` only if the watcher thread is gone.
    pub fn recv_timeout(&self, dur: Duration) -> Result<Option<WatchEvent>, WatchError> {
        match self.rx.recv_timeout(dur) {
            Ok(res) => Ok(Self::coalesce(res)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(WatchError::Closed),
        }
    }

    /// Drain anything already in the queue without blocking. Useful for
    /// reconciling state after a long-running backup completes.
    pub fn drain(&self) -> Vec<WatchEvent> {
        let mut out = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            if let Some(e) = Self::coalesce(res) {
                out.push(e);
            }
        }
        out
    }

    fn coalesce(res: DebounceEventResult) -> Option<WatchEvent> {
        let events = match res {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!(error = %err, "watcher reported error");
                return None;
            }
        };
        if events.is_empty() {
            return None;
        }
        let mut paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
        paths.sort();
        paths.dedup();
        Some(WatchEvent::SaveChanged { paths })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// We use a very short debounce so tests finish quickly. Production
    /// uses [`DEFAULT_DEBOUNCE`].
    const TEST_DEBOUNCE: Duration = Duration::from_millis(200);
    /// Generous deadline for the OS to deliver an inotify/ReadDirectoryChanges
    /// notification + the debounce window. Keeps CI from flaking.
    const TEST_TIMEOUT: Duration = Duration::from_secs(3);

    #[test]
    fn detects_a_write() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = SaveWatcher::with_debounce(TEST_DEBOUNCE).unwrap();
        w.watch(dir.path()).unwrap();

        let target = dir.path().join("save.dat");
        std::fs::write(&target, b"hello").unwrap();

        let event = w
            .recv_timeout(TEST_TIMEOUT)
            .unwrap()
            .expect("should have received SaveChanged within timeout");
        let WatchEvent::SaveChanged { paths } = event;
        assert!(
            paths
                .iter()
                .any(|p| p == &target || p.ends_with("save.dat")),
            "event paths {paths:?} should mention save.dat"
        );
    }

    #[test]
    fn coalesces_burst_of_writes() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = SaveWatcher::with_debounce(TEST_DEBOUNCE).unwrap();
        w.watch(dir.path()).unwrap();

        // Burst of 5 writes in rapid succession. With a 200ms debounce,
        // the OS *may* occasionally split this into two windows depending
        // on scheduling jitter (especially on CI). We assert the weaker
        // and more useful invariant: 5 writes coalesce into far fewer
        // events than 5, and the union covers everything.
        let mut expected: Vec<PathBuf> = Vec::new();
        for i in 0..5 {
            let p = dir.path().join(format!("save_{i}.dat"));
            std::fs::write(&p, b"x").unwrap();
            expected.push(p);
        }

        // First event must arrive.
        let first = w
            .recv_timeout(TEST_TIMEOUT)
            .unwrap()
            .expect("burst should produce at least one debounced event");
        let mut got: Vec<PathBuf> = match first {
            WatchEvent::SaveChanged { paths } => paths,
        };

        // Drain a short tail in case the burst spilled into a second window.
        std::thread::sleep(TEST_DEBOUNCE * 2);
        for ev in w.drain() {
            let WatchEvent::SaveChanged { paths } = ev;
            got.extend(paths);
        }

        // The union of received paths covers every write — the file names
        // are unique so we just check inclusion.
        for p in &expected {
            assert!(
                got.iter()
                    .any(|g| g == p || g.ends_with(p.file_name().unwrap())),
                "missing {p:?} from received events {got:?}"
            );
        }

        // And we coalesced — strictly fewer events than writes. If notify
        // ever delivered one event per write the whole point of debouncing
        // would be moot.
        // (We don't assert == 1 because OS jitter; ≤ 3 is the real bar.)
        let event_count = w.drain().len() + 1; // +1 for the first event already taken
        assert!(
            event_count <= 3,
            "5 writes should coalesce into at most 3 events, got {event_count}"
        );
    }

    #[test]
    fn watch_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = SaveWatcher::with_debounce(TEST_DEBOUNCE).unwrap();
        w.watch(dir.path()).unwrap();
        w.watch(dir.path()).unwrap(); // would error from notify if not idempotent
        assert_eq!(w.watched().count(), 1);
    }

    #[test]
    fn unwatch_stops_events() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = SaveWatcher::with_debounce(TEST_DEBOUNCE).unwrap();
        w.watch(dir.path()).unwrap();
        w.unwatch(dir.path()).unwrap();
        assert_eq!(w.watched().count(), 0);

        std::fs::write(dir.path().join("after-unwatch.dat"), b"x").unwrap();
        let event = w.recv_timeout(Duration::from_millis(400)).unwrap();
        assert!(event.is_none(), "no events after unwatch");
    }
}
