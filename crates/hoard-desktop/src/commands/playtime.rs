//! Playtime recap data + playtime-only game tracking.
//!
//! Two concerns live here:
//!
//! * **Recap data** ([`list_playtime`]): reads the agent's local
//!   `playtime.json` (per-local-day seconds played, written by the in-process
//!   agent's poll loop) and hands it to the UI. Nothing here touches the
//!   network — playtime is local-only data.
//! * **Playtime-only games** (the rest): always-online titles with no save
//!   worth syncing (Fortnite, Rust, VALORANT…). We auto-enrol them — matching
//!   the curated [`hoard_agent::playtime_catalog`] against the installed-game
//!   scan (Steam + Epic + GOG + MS Store) — as `track_only` agent slots so
//!   their hours accrue for the recap without ever backing anything up. The
//!   user can drop any of them; the opt-out persists in `CliState`.

use std::collections::HashSet;
use std::path::PathBuf;

use hoard_agent::agent::WatchedSave;
use hoard_agent::manifest::Os;
use hoard_agent::playtime::{PlaytimeStore, PlaytimeSummary};
use hoard_agent::playtime_catalog;
use hoard_agent::state::CliState;
use hoard_agent::{launchers, steam};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

/// Per-day / per-game playtime totals for the recap heatmap. Empty until the
/// agent has observed a tracked game running for at least one poll interval.
#[tauri::command]
pub fn list_playtime() -> Result<PlaytimeSummary, String> {
    let path = PlaytimeStore::default_path().map_err(|e| e.to_string())?;
    Ok(PlaytimeStore::load(&path).summary())
}

/// One playtime-only game surfaced to the UI (the amber "Jugados, sin copia"
/// list). `excluded` is the user's opt-out: when true the game is in the
/// catalog/installed set but we are *not* counting it.
#[derive(Debug, Clone, Serialize)]
pub struct PlaytimeGameInfo {
    pub slug: String,
    pub display_name: String,
    pub excluded: bool,
}

/// Synthetic `save_id` for a playtime-only slot. Prefixed so it never collides
/// with a real save's UUID and is obvious in logs.
fn playtime_save_id(slug: &str) -> String {
    format!("playtime:{slug}")
}

/// Installed games (any launcher) that match the curated playtime catalog,
/// keyed by catalog slug, paired with the install dir we found them in (used
/// as the process poll's dir-prefix fallback when a process-name match misses).
/// First match per slug wins. Epic/GOG/MS readers no-op off Windows; Steam
/// works everywhere, so on Linux this still finds e.g. Rust.
fn installed_catalog_games(os: Os) -> Vec<(&'static str, Option<PathBuf>)> {
    let mut sources: Vec<(String, PathBuf)> = Vec::new();
    for app in steam::list_installed_steam_games(os).unwrap_or_default() {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_epic_games(os) {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_gog_games(os) {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_msstore_games(os) {
        sources.push((app.name, app.install_dir));
    }
    let mut out: Vec<(&'static str, Option<PathBuf>)> = Vec::new();
    let mut seen: HashSet<&'static str> = HashSet::new();
    for (name, dir) in sources {
        if let Some(g) = playtime_catalog::game_for_store_name(&name) {
            if seen.insert(g.slug) {
                out.push((g.slug, Some(dir)));
            }
        }
    }
    out
}

/// Build the `track_only` WatchedSaves to seed into the agent: every installed
/// catalog game that isn't already tracked as a real save and that the user
/// hasn't excluded. Shared by [`crate::commands::agent::hydrate_watched_saves`]
/// and the live attach path.
pub fn derive_playtime_saves(cli_state: &CliState, tracked_slugs: &HashSet<String>) -> Vec<WatchedSave> {
    installed_catalog_games(Os::current())
        .into_iter()
        .filter_map(|(slug, dir)| {
            if tracked_slugs.contains(slug) || cli_state.is_playtime_excluded(slug) {
                return None;
            }
            Some(playtime_watched_save(slug, dir))
        })
        .collect()
}

/// Construct the `track_only` slot for one catalog slug.
fn playtime_watched_save(slug: &str, install_dir: Option<PathBuf>) -> WatchedSave {
    let game = playtime_catalog::by_slug(slug);
    let display_name = game.map(|g| g.display_name.to_string()).unwrap_or_else(|| slug.to_string());
    let processes = game
        .map(|g| g.processes.iter().map(|p| p.to_string()).collect())
        .unwrap_or_default();
    WatchedSave {
        save_id: playtime_save_id(slug),
        game_slug: slug.to_string(),
        display_name,
        label: "playtime".to_string(),
        // Playtime-only: no folder on disk. The agent's track_only guards keep
        // this sentinel out of every watch/backup/restore path.
        local_path: PathBuf::new(),
        steam_install_dir: install_dir,
        processes,
        policy: Default::default(),
        known_version: None,
        set_hash: None,
        track_only: true,
    }
}

/// Slugs already tracked as real saves on this machine (their playtime is
/// counted via the save slot, so we never enrol a playtime-only duplicate).
fn tracked_slugs(cli_state: &CliState) -> HashSet<String> {
    cli_state.saves.values().map(|s| s.game_slug.clone()).collect()
}

/// The amber "Jugados, sin copia" list: installed catalog games not tracked as
/// saves (flagged with the user's opt-out state), plus any excluded-but-now-
/// uninstalled game so the user can still re-enable it.
#[tauri::command]
pub fn list_playtime_games() -> Result<Vec<PlaytimeGameInfo>, String> {
    let (cli_state, _) = CliState::load_default().map_err(|e| e.to_string())?;
    let tracked = tracked_slugs(&cli_state);
    let mut out: Vec<PlaytimeGameInfo> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (slug, _) in installed_catalog_games(Os::current()) {
        if tracked.contains(slug) {
            continue;
        }
        let Some(g) = playtime_catalog::by_slug(slug) else {
            continue;
        };
        out.push(PlaytimeGameInfo {
            slug: slug.to_string(),
            display_name: g.display_name.to_string(),
            excluded: cli_state.is_playtime_excluded(slug),
        });
        seen.insert(slug.to_string());
    }

    // Excluded games no longer installed still get a row so they're re-enablable.
    for slug in &cli_state.playtime_excluded {
        if seen.contains(slug) || tracked.contains(slug) {
            continue;
        }
        if let Some(g) = playtime_catalog::by_slug(slug) {
            out.push(PlaytimeGameInfo {
                slug: slug.clone(),
                display_name: g.display_name.to_string(),
                excluded: true,
            });
        }
    }

    out.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(out)
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

/// Re-allow `slug` for playtime tracking: clear the opt-out and, if it's
/// installed, attach its slot to the running agent so it counts without a
/// restart. Idempotent.
#[tauri::command]
pub async fn include_playtime_game(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let slug = slug.trim().to_string();
    if slug.is_empty() {
        return Err("Slug is empty.".into());
    }
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.include_playtime(&slug);
    cli_state.save(&path).map_err(|e| e.to_string())?;

    // Attach live if the game is installed and not already a tracked save.
    if !tracked_slugs(&cli_state).contains(&slug) {
        if let Some((s, dir)) = installed_catalog_games(Os::current())
            .into_iter()
            .find(|(s, _)| *s == slug)
        {
            super::agent::attach_save_if_running(&state, playtime_watched_save(s, dir)).await;
        }
    }
    Ok(())
}
