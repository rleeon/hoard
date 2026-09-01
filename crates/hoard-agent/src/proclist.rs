//! Live snapshot of game-like processes for the manual emulator-add picker.
//!
//! When the user adds an emulator save by hand they must tell Hoard which
//! executable means "I'm playing" (the emulator's process name). Typing it is
//! error-prone (`pcsx2-qt.exe` against `pcsx2.exe`) so the UI offers a picker
//! populated from this snapshot: open the emulator, hit "detect", and the
//! active process (highest CPU) floats to the top.
//!
//! This is a one-shot, on-demand sample (the desktop calls it from a Tauri
//! command), *not* part of the agent's steady-state poll. It reuses
//! [`crate::correlation::is_game_like`] and the same hard filters as
//! [`crate::correlation::sample_game_processes`] (drop threads, require an exe
//! on disk) so the list matches what the engine would actually treat as a game.

use serde::Serialize;
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::correlation::is_game_like;

/// One candidate process for the picker, deduplicated by executable name.
#[derive(Debug, Clone, Serialize)]
pub struct RunningProcess {
    /// Executable name exactly as the OS reports it (e.g. `"pcsx2-qt.exe"`).
    /// This is what gets stored in `WatchedSave.processes` and matched
    /// case-insensitively by the agent's process poll, so it must round-trip
    /// verbatim.
    pub name: String,
    /// Peak CPU usage across live instances (100.0 = one fully-used core).
    /// Sorting by it puts the emulator the user just launched at the top.
    pub cpu: f32,
}

/// Sample currently-running, game-like processes, deduplicated by name and
/// sorted by CPU (descending), then name. Synchronous and briefly blocking
/// (~one CPU sampling interval): call it from `spawn_blocking` so the async
/// runtime keeps turning.
pub fn list_game_like_processes() -> Vec<RunningProcess> {
    // `with_cpu` needs two refreshes spaced by the minimum interval to compute
    // a non-zero delta; `with_exe` lets `is_game_like` check the binary path.
    let kind = ProcessRefreshKind::new()
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_cpu();
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);

    let mut by_name: HashMap<String, f32> = HashMap::new();
    for proc in sys.processes().values() {
        // Same hard filters as `sample_game_processes`: a game is a process,
        // not a thread, and always has a binary on disk (kernel threads don't).
        if proc.thread_kind().is_some() {
            continue;
        }
        let Some(exe) = proc.exe().map(|p| p.to_path_buf()) else {
            continue;
        };
        let name = proc.name().to_string_lossy().to_string();
        if !is_game_like(&name, Some(&exe)) {
            continue;
        }
        let slot = by_name.entry(name).or_insert(0.0);
        *slot = slot.max(proc.cpu_usage());
    }

    let mut out: Vec<RunningProcess> = by_name
        .into_iter()
        .map(|(name, cpu)| RunningProcess { name, cpu })
        .collect();
    out.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}
