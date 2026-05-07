//! Process watcher — primary detection path.
//!
//! We poll the OS process table at a configurable interval (default 2s)
//! and compare against each manifest's `processes` list. When a match
//! starts, we emit [`ProcessEvent::GameStarted`]; when its last instance
//! disappears, [`ProcessEvent::GameStopped`].
//!
//! Why polling, not eventfd/ETW? sysinfo is portable across Win/Linux/Mac
//! and 2-second granularity is plenty: nobody opens and closes a game in
//! under 2 seconds, and the cost (one process snapshot) is negligible.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// One frame's worth of state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    /// First instance of a known game's process appeared.
    GameStarted { manifest_id: String, pid: u32 },
    /// Last instance of a known game's process disappeared.
    GameStopped { manifest_id: String, pid: u32 },
}

/// Watches the process table for games matching a fixed set of manifests.
///
/// Driven by the caller — call [`Self::poll`] periodically from a tokio
/// task. The watcher itself does no I/O scheduling.
pub struct ProcessWatcher {
    /// Lowercased process exe-name → manifest id. We lowercase because
    /// Windows is case-insensitive and `Game.exe` may appear as `game.exe`.
    name_to_id: HashMap<String, String>,
    /// pid → manifest id of currently-running known games.
    running: HashMap<u32, String>,
    sys: System,
}

impl ProcessWatcher {
    /// Build a watcher that recognises any process named in any manifest's
    /// `processes` list. Names are matched case-insensitively against the
    /// process file name (no path).
    pub fn new<'a, I>(manifests: I) -> Self
    where
        I: IntoIterator<Item = &'a hoard_manifest::GameManifest>,
    {
        let mut name_to_id = HashMap::new();
        for m in manifests {
            for p in &m.processes {
                name_to_id.insert(p.to_lowercase(), m.id.clone());
            }
        }
        Self {
            name_to_id,
            running: HashMap::new(),
            sys: System::new(),
        }
    }

    /// Recommended poll interval. Tuned so a "click play, see Hoard
    /// notify" round-trip feels instant (≤ 2s) without burning CPU.
    pub fn recommended_interval() -> Duration {
        Duration::from_secs(2)
    }

    /// Refresh the process table and return events since last call.
    ///
    /// First call returns `GameStarted` for every already-running known
    /// game; that's intentional (lets the agent reconcile state after a
    /// crash-restart).
    pub fn poll(&mut self) -> Vec<ProcessEvent> {
        // Refresh just process names — we don't need cmdlines or env.
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new(),
        );

        let mut seen: HashMap<u32, String> = HashMap::new();
        for (pid, proc_) in self.sys.processes() {
            // sysinfo gives us the exe file name (no path). On Windows
            // that's `Game.exe`; on Linux it's the comm name (which
            // matches the manifest's process entry for Steam/Proton runs).
            let name = proc_.name().to_string_lossy().to_lowercase();
            if let Some(id) = self.name_to_id.get(&name) {
                seen.insert(pid.as_u32(), id.clone());
            }
        }

        let mut events = Vec::new();

        // New pids → GameStarted. We dedupe by manifest id: if a game
        // launches two processes with the same exe name (rare, but
        // possible for launcher+game pairs), we only emit one event per
        // manifest until they all disappear.
        let prev_ids: HashSet<&String> = self.running.values().collect();
        for (pid, id) in &seen {
            if !self.running.contains_key(pid) && !prev_ids.contains(id) {
                events.push(ProcessEvent::GameStarted {
                    manifest_id: id.clone(),
                    pid: *pid,
                });
            }
        }

        // Disappeared pids → if no other pid for that manifest remains,
        // emit GameStopped.
        let still_running_ids: HashSet<&String> = seen.values().collect();
        for (pid, id) in &self.running {
            if !seen.contains_key(pid) && !still_running_ids.contains(id) {
                events.push(ProcessEvent::GameStopped {
                    manifest_id: id.clone(),
                    pid: *pid,
                });
            }
        }

        self.running = seen;
        events
    }

    /// pids currently associated with a known game. Useful for the UI
    /// "is anything playing right now" indicator.
    pub fn running_games(&self) -> impl Iterator<Item = (&u32, &String)> {
        self.running.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_manifest::{GameManifest, ManifestSource};

    fn fake_manifest(id: &str, exe: &str) -> GameManifest {
        GameManifest {
            id: id.to_string(),
            title: id.to_string(),
            steam_appid: None,
            gog_id: None,
            epic_id: None,
            xbox_pfn: None,
            processes: vec![exe.to_string()],
            windows_paths: vec![],
            linux_paths: vec![],
            source: ManifestSource::HandCurated,
            notes: None,
        }
    }

    #[test]
    fn lookup_table_is_lowercase() {
        let m = fake_manifest("foo", "Foo.EXE");
        let w = ProcessWatcher::new(std::iter::once(&m));
        assert!(w.name_to_id.contains_key("foo.exe"));
        assert!(!w.name_to_id.contains_key("Foo.EXE"));
    }

    #[test]
    fn empty_watcher_emits_nothing() {
        let m = fake_manifest("ghost", "definitely-not-a-real-process-9b9b9b.exe");
        let mut w = ProcessWatcher::new(std::iter::once(&m));
        let events = w.poll();
        assert!(events.is_empty(), "no real process matches a fake exe");
    }

    #[test]
    fn detects_running_self_as_game() {
        // Use the test binary itself as the "game" — this proves the
        // process scan actually finds running processes.
        //
        // We can't use `current_exe()` here because Linux truncates the
        // process comm to 15 bytes, which mangles cargo's hashed test
        // binary names. Ask sysinfo what name *it* sees for our pid and
        // match against that — that's the same string `poll` will see.
        let mut sys = sysinfo::System::new();
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::new(),
        );
        let our_pid = sysinfo::Pid::from_u32(std::process::id());
        let Some(our_name) = sys
            .process(our_pid)
            .map(|p| p.name().to_string_lossy().into_owned())
        else {
            // No /proc visible? Skip rather than fail — we don't want this
            // test red on container environments without /proc.
            eprintln!("self-process not visible to sysinfo; skipping");
            return;
        };

        let m = fake_manifest("self-test", &our_name);
        let mut w = ProcessWatcher::new(std::iter::once(&m));
        let events = w.poll();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProcessEvent::GameStarted { manifest_id, .. } if manifest_id == "self-test")),
            "expected GameStarted for our own process named {our_name:?}; got {events:?}"
        );

        // Second poll with the process still running → no new event.
        let events2 = w.poll();
        assert!(
            events2.is_empty(),
            "no churn while the same pid is still running; got {events2:?}"
        );
    }
}
