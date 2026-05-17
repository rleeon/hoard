//! Live-agent commands: start/stop the watcher and forward its events.
//!
//! The agent runs in the background as long as the desktop app is open. It
//! watches the user's tracked-save folders, detects when games launch and
//! quit, and uploads snapshots after a debounce.
//!
//! `start_agent` is invoked once per session — usually right after login.
//! It does three things:
//!
//! 1. Hydrate the list of tracked saves from the local state file plus the
//!    server's known-paths data (so we know each game's Steam install dir
//!    for process matching).
//! 2. Spawn the live agent (`hoard_agent::agent::spawn`) with that list.
//! 3. Stand up a forwarder task that re-emits every `AgentEvent` as a
//!    Tauri event named `agent://<event_type>`. The frontend subscribes
//!    with `listen()`.
//!
//! `stop_agent` cleanly shuts the agent down on logout. `backup_now`
//! exposes the manual-trigger button on the dashboard.

use std::path::PathBuf;

use hoard_agent::agent::{self, AgentConfig, AgentEvent, AgentSlotStatus, WatchedSave};
use hoard_agent::manifest::Os;
use hoard_agent::prefs::Prefs;
use hoard_agent::state::CliState;
use hoard_agent::steam;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use super::library::current_client;
use crate::state::AppState;

/// Summary of the running agent that the UI can display in Settings.
#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    pub running: bool,
    pub watched_count: usize,
}

/// Boot the live agent and start forwarding events. No-op if it's already
/// running (returns the existing status).
#[tauri::command]
pub async fn start_agent(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AgentStatus, String> {
    if state.agent.lock().unwrap().is_some() {
        return Ok(AgentStatus {
            running: true,
            watched_count: 0, // unknown without round-tripping the agent
        });
    }

    let client = current_client(&state)?;
    let saves = hydrate_watched_saves(&state).map_err(|e| e.to_string())?;
    let watched_count = saves.len();

    // Pull the user's auto-restore preference so the agent knows whether it
    // should pull the latest server snapshot when a tracked save's local
    // path is missing or empty on attach.
    let auto_restore = Prefs::load_default()
        .map(|(p, _)| p.auto_restore)
        .unwrap_or(false);
    let config = AgentConfig {
        auto_restore,
        ..AgentConfig::default()
    };

    let (events_tx, mut events_rx) = mpsc::channel::<AgentEvent>(256);
    let (handle, _task) = agent::spawn(client, config, saves, events_tx);

    // Forwarder task — translate AgentEvent into Tauri events the UI can
    // subscribe to. `agent://*` is our private event namespace; the
    // dashboard listens with a single `listen("agent://...")` per type.
    let app_for_emit = app.clone();
    tokio::spawn(async move {
        while let Some(ev) = events_rx.recv().await {
            // The event variant name doubles as the Tauri channel suffix.
            // Serializing the whole enum gives the frontend a tagged
            // payload that's easy to discriminate on.
            let topic = match &ev {
                AgentEvent::GameStarted { .. } => "agent://game-started",
                AgentEvent::GameStopped { .. } => "agent://game-stopped",
                AgentEvent::BackupScheduled { .. } => "agent://backup-scheduled",
                AgentEvent::BackupStarted { .. } => "agent://backup-started",
                AgentEvent::BackupSuccess { .. } => "agent://backup-success",
                AgentEvent::BackupFailed { .. } => "agent://backup-failed",
                AgentEvent::SaveAutoRestored { .. } => "agent://save-auto-restored",
                AgentEvent::SaveAutoRestoreFailed { .. } => "agent://save-auto-restore-failed",
                AgentEvent::BackupSkippedEmpty { .. } => "agent://backup-skipped-empty",
            };
            let _ = app_for_emit.emit(topic, &ev);
        }
    });

    *state.agent.lock().unwrap() = Some(handle);
    Ok(AgentStatus {
        running: true,
        watched_count,
    })
}

/// Tear the agent down. Used on logout and on app exit (best-effort).
#[tauri::command]
pub async fn stop_agent(state: State<'_, AppState>) -> Result<(), String> {
    let handle = state.agent.lock().unwrap().take();
    if let Some(h) = handle {
        h.shutdown().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Force a backup right now for the given save. Bypasses the debounce.
#[tauri::command]
pub async fn backup_now(save_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let handle = state
        .agent
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Agent isn't running. Sign in first.".to_string())?;
    handle
        .backup_now(save_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Diagnostic snapshot of every slot the agent is currently tracking.
/// Powers the hidden "agent diagnostics" panel in Settings — the only
/// non-trace surface that reveals whether each slot's fs watcher actually
/// armed and what it's seen. Returns an empty vec when the agent is not
/// running (no error: the UI shows "agent stopped").
#[tauri::command]
pub async fn agent_status(state: State<'_, AppState>) -> Result<Vec<AgentSlotStatus>, String> {
    let handle = state.agent.lock().unwrap().clone();
    let Some(h) = handle else {
        return Ok(Vec::new());
    };
    h.status().await.map_err(|e| e.to_string())
}

// ---- helpers ----------------------------------------------------------

/// Build the initial watch list from local state. The server-side save row
/// gives us the slug; the local state file gives us the path on this
/// machine. We try to enrich each entry with its Steam install dir so the
/// process watcher has something to match against.
fn hydrate_watched_saves(_state: &State<'_, AppState>) -> anyhow::Result<Vec<WatchedSave>> {
    let (cli_state, _) = CliState::load_default()?;
    if cli_state.saves.is_empty() {
        return Ok(Vec::new());
    }

    // Cache Steam apps once — if the user has 100 tracked saves we don't
    // want to scan `appmanifest_*.acf` 100 times. `list_installed_steam_games`
    // is cheap (sub-second on a normal install) so a single call here is
    // negligible startup cost.
    let steam_apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();

    let mut out = Vec::with_capacity(cli_state.saves.len());
    for (save_id, save_state) in cli_state.saves {
        if save_state.paused {
            // The user has explicitly told us to leave this save alone.
            // Skipping it here keeps the agent unaware of it entirely —
            // no process matching, no FS watch, no backups.
            continue;
        }
        let steam_install_dir = steam_apps
            .iter()
            .find(|a| name_matches(&a.name, &save_state.game_slug))
            .map(|a| a.install_dir.clone());

        // v0.3 process-name match: pull the manifest's `processes` list
        // if the slug matches a curated game; otherwise fall back to
        // legacy install-dir prefix matching only.
        let processes = hoard_agent::hoard_manifest_processes(&save_state.game_slug);

        out.push(WatchedSave {
            save_id,
            game_slug: save_state.game_slug.clone(),
            // We don't store the display name in CliState today; reuse the
            // slug as a stand-in. The UI re-fetches display name from the
            // server cache anyway.
            display_name: save_state.game_slug.clone(),
            label: save_state.label,
            local_path: save_state.local_path,
            steam_install_dir,
            processes,
        });
    }
    Ok(out)
}

/// Loose match between a Steam app name and a Hoard slug. Steam stores
/// "Stardew Valley", we store "stardew-valley"; comparing
/// kebab-cased-lowercase against lowercase-with-spaces-removed catches
/// most cases.
fn name_matches(steam_name: &str, slug: &str) -> bool {
    let a: String = steam_name
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect();
    let b: String = slug.chars().filter(|c| c.is_alphanumeric()).collect();
    !a.is_empty() && a == b
}

/// Glue used by `add_game_to_tracking` so newly tracked saves auto-attach
/// to the running agent without forcing a full restart.
pub(crate) async fn attach_save_if_running(state: &State<'_, AppState>, save: WatchedSave) {
    let handle = state.agent.lock().unwrap().clone();
    if let Some(h) = handle {
        if let Err(e) = h.add_save(save).await {
            tracing::warn!(error = %e, "couldn't attach new save to live agent");
        }
    }
}

/// Glue used by `untrack_save` so removing a save also stops watching it.
pub(crate) async fn detach_save_if_running(state: &State<'_, AppState>, save_id: String) {
    let handle = state.agent.lock().unwrap().clone();
    if let Some(h) = handle {
        if let Err(e) = h.remove_save(save_id).await {
            tracing::warn!(error = %e, "couldn't detach save from live agent");
        }
    }
}

/// Path-aware helper used by the library command when adding a save and the
/// agent is running. Builds a `WatchedSave` from minimal inputs.
pub(crate) fn watched_save_from(
    save_id: String,
    game_slug: String,
    display_name: String,
    label: String,
    local_path: PathBuf,
) -> WatchedSave {
    let steam_apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();
    let steam_install_dir = steam_apps
        .iter()
        .find(|a| name_matches(&a.name, &game_slug))
        .map(|a| a.install_dir.clone());
    let processes = hoard_agent::hoard_manifest_processes(&game_slug);
    WatchedSave {
        save_id,
        game_slug,
        display_name,
        label,
        local_path,
        steam_install_dir,
        processes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_match_kebab_vs_titlecase() {
        assert!(name_matches("Stardew Valley", "stardew-valley"));
        assert!(name_matches("Hollow Knight", "hollow-knight"));
        assert!(name_matches(
            "Subnautica: Below Zero",
            "subnautica-below-zero"
        ));
        assert!(!name_matches("Stardew Valley", "stardew-vallei"));
        assert!(!name_matches("", ""));
    }
}
