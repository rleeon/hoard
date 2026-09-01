//! The Tauri bridge for adding emulators by hand.
//!
//! The catalogue, path resolution and the two probes (a portable install on
//! another drive, splitting per title) live in [`hoard_agent::emulators`]: they are
//! detection, and detection is shared by both frontends. What is left here are the
//! `#[tauri::command]`s that serve that data to the UI, plus the live process
//! picker.

use hoard_agent::emulators;
use serde::Serialize;

use hoard_agent::proclist::RunningProcess;

/// One catalogue entry, with its paths already resolved for this machine.
#[derive(Debug, Clone, Serialize)]
pub struct EmulatorPreset {
    pub id: &'static str,
    pub display_name: &'static str,
    pub system: &'static str,
    pub processes: Vec<&'static str>,
    /// Native save folders that exist on this machine; the first is the best
    /// default. It can come back empty (a portable emulator, an install off the
    /// beaten path), and then the UI asks the user for the folder.
    pub save_paths: Vec<String>,
    /// True when this emulator's save root can be split into one folder per game.
    /// The UI then offers picking titles instead of adding the whole tree.
    pub splits_per_title: bool,
}

/// The emulator catalogue with its folders resolved against the host. It feeds the
/// "Add emulator" dialog. Cheap (a handful of `stat`s) except when drives have to be
/// probed, hence the `spawn_blocking`.
#[tauri::command]
pub async fn list_emulator_presets() -> Result<Vec<EmulatorPreset>, String> {
    tokio::task::spawn_blocking(|| {
        emulators::CATALOG
            .iter()
            .map(|def| {
                // Installed first, portable second: when somebody has both, the
                // installed copy is the one their emulator opens by default and it
                // should be the default here too.
                let mut save_paths = emulators::resolve_save_paths(def);
                for p in emulators::portable_save_paths(def) {
                    let s = p.to_string_lossy().into_owned();
                    if !save_paths.contains(&s) {
                        save_paths.push(s);
                    }
                }
                EmulatorPreset {
                    id: def.id,
                    display_name: def.display_name,
                    system: def.system,
                    processes: def.processes.to_vec(),
                    save_paths,
                    splits_per_title: def.title_layout.is_some(),
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("Couldn't read the emulator catalogue: {e}"))
}

/// A game found inside a console's save tree.
#[derive(Debug, Clone, Serialize)]
pub struct EmulatorTitle {
    /// The title's id exactly as the folder names it. It is the only thing two
    /// different installs call the same.
    pub title_id: String,
    pub path: String,
}

/// The games inside an emulator's save folder.
///
/// It returns empty when the tree does not have the expected shape, and that is
/// **not an error**: it means the caller should keep offering the root as it is. A
/// layout guess that misses would leave the user with no detection at all, which is
/// worse than the problem this solves.
#[tauri::command]
pub async fn list_emulator_titles(
    emulator_id: String,
    root: String,
) -> Result<Vec<EmulatorTitle>, String> {
    let Some(layout) = emulators::find(&emulator_id).and_then(|d| d.title_layout) else {
        return Ok(Vec::new());
    };
    let found = tokio::task::spawn_blocking(move || {
        emulators::split_per_title(std::path::Path::new(&root), layout)
    })
    .await
    .map_err(|e| format!("Couldn't read the emulator's games: {e}"))?;

    Ok(found
        .into_iter()
        .map(|t| EmulatorTitle {
            title_id: t.title_id,
            path: t.path.to_string_lossy().into_owned(),
        })
        .collect())
}

/// A live snapshot of the game-looking processes, for the picker that saves typing
/// the executable's name. The CPU sample blocks for a moment, so it runs off the
/// async runtime.
#[tauri::command]
pub async fn list_running_processes() -> Result<Vec<RunningProcess>, String> {
    tokio::task::spawn_blocking(hoard_agent::proclist::list_game_like_processes)
        .await
        .map_err(|e| format!("Couldn't sample processes: {e}"))
}
