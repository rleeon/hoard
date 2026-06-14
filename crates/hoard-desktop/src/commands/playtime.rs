//! Playtime recap data. Reads the agent's local `playtime.json` (per-local-day
//! seconds played, written by the in-process agent's poll loop) and hands it to
//! the UI. Nothing here touches the network — playtime is local-only data.

use hoard_agent::playtime::{PlaytimeStore, PlaytimeSummary};

/// Per-day / per-game playtime totals for the recap heatmap. Empty until the
/// agent has observed a tracked game running for at least one poll interval.
#[tauri::command]
pub fn list_playtime() -> Result<PlaytimeSummary, String> {
    let path = PlaytimeStore::default_path().map_err(|e| e.to_string())?;
    Ok(PlaytimeStore::load(&path).summary())
}
