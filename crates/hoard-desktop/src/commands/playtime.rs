//! Playtime recap data + playtime-only game tracking.
//!
//! Two concerns live here:
//!
//! * **Recap data** ([`list_playtime`]): reads the agent's local
//!   `playtime.json` (per-local-day seconds played, written by the in-process
//!   agent's poll loop) and hands it to the UI. Nothing here touches the
//!   network: playtime is local-only data.
//! * **Playtime-only games** (the rest): always-online titles with no save
//!   worth syncing (Fortnite, Rust, VALORANT...). We auto-enrol them, matching
//!   the curated [`hoard_agent::playtime_catalog`] against the installed-game
//!   scan (Steam, Epic, GOG, MS Store), as `track_only` agent slots so
//!   their hours accrue for the recap without ever backing anything up. The
//!   user can drop any of them; the opt-out persists in `CliState`.

use std::time::Duration;

use hoard_agent::library::{self, PlaytimeGameInfo};
use hoard_agent::playtime::{PlaytimeStore, PlaytimeSummary};
use hoard_agent::state::CliState;
use hoard_core::ipc::{Payload, Request};
use tauri::State;

use crate::state::AppState;

/// The service answers within one sweep of the disks, 20 s at most with every
/// place walked at once. This only catches a service that stopped answering.
const PLAYTIME_GAMES_LIMIT: Duration = Duration::from_secs(2 * 60);

/// Per-day / per-game playtime totals for the recap heatmap. Empty until the
/// agent has observed a tracked game running for at least one poll interval.
#[tauri::command]
pub fn list_playtime() -> Result<PlaytimeSummary, String> {
    // Idempotent: it adopts the legacy playtime into the active context when that
    // has not happened yet (the recap opening before the agent starts, say).
    let _ = PlaytimeStore::migrate_legacy_into_current_context();
    let path = PlaytimeStore::default_path().map_err(|e| e.to_string())?;
    Ok(PlaytimeStore::load(&path).summary())
}

/// Synthetic `save_id` for a playtime-only slot. Prefixed so it never collides
/// with a real save's UUID and is obvious in logs.
fn playtime_save_id(slug: &str) -> String {
    format!("playtime:{slug}")
}

/// The amber "Jugados, sin copia" list.
///
/// Async, and asked of the service: knowing what is installed sweeps the disks
/// for Steam libraries, and as a sync command that ran on the window's thread.
/// With a NAS mapped as `Z:` it held the window long enough for Windows to grey
/// it out as not responding (issue #67), while the service swept the same
/// share for its own scan. A service from before the request, or one that
/// can't answer, gets the list worked out here, off the window's thread.
#[tauri::command]
pub async fn list_playtime_games(
    state: State<'_, AppState>,
) -> Result<Vec<PlaytimeGameInfo>, String> {
    if state.daemon.owns_detection().await {
        match state
            .daemon
            .request_long(Request::PlaytimeGames, PLAYTIME_GAMES_LIMIT)
            .await
        {
            Ok(Payload::PlaytimeGames { games }) => {
                return serde_json::from_value(games).map_err(|e| e.to_string());
            }
            Ok(other) => tracing::warn!(?other, "unexpected answer to the playtime games"),
            Err(e) => tracing::info!(
                error = %format!("{e:#}"),
                "the service can't list the playtime games; working them out here"
            ),
        }
    }
    tokio::task::spawn_blocking(library::playtime_games)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Drop `slug` from playtime tracking: persist the opt-out and detach its live
/// slot so its hours stop accruing immediately. Idempotent.
#[tauri::command]
pub async fn exclude_playtime_game(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let slug = slug.trim().to_string();
    if slug.is_empty() {
        return Err("Slug is empty.".into());
    }
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.exclude_playtime(slug.clone());
    cli_state.save(&path).map_err(|e| e.to_string())?;
    super::agent::detach_save_if_running(&state, playtime_save_id(&slug)).await;
    Ok(())
}

/// Re-allow `slug` for playtime tracking: clear the opt-out and let the service
/// know, so an installed game counts without a restart. Idempotent.
#[tauri::command]
pub async fn include_playtime_game(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let slug = slug.trim().to_string();
    if slug.is_empty() {
        return Err("Slug is empty.".into());
    }
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.include_playtime(&slug);
    cli_state.save(&path).map_err(|e| e.to_string())?;
    // The service seeds the hours-only slots from `state.json` itself
    // (`library::derive_playtime_saves`): whether the game is installed is its
    // question, and asking it here swept the disks a second time.
    state.daemon.notify_reload().await;
    Ok(())
}
