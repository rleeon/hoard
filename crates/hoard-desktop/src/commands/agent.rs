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
use std::sync::{Mutex, OnceLock};

use hoard_agent::agent::{self, AgentConfig, AgentEvent, AgentSlotStatus, WatchedSave};
use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;
use hoard_agent::manifest::Os;
use hoard_agent::prefs::Prefs;
use hoard_agent::presets::{self, SavePolicy};
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

/// Clone of the live agent's `ApiClient`, kept so the cloud token-refresh path
/// can push a fresh Supabase JWT into the long-lived agent without rebuilding
/// it. The agent is spawned once per session with a single client; for a Hoard
/// Cloud session that client's JWT expires after ~1h, and nothing else refreshes
/// it. Before this hook the auto-restore sweep kept firing with the stale token
/// and 401'd every tick ("no se pudo restaurar …"). `ApiClient` shares its token
/// cell across clones, so calling `set_token` here updates the agent's copy too.
static AGENT_CLIENT: OnceLock<Mutex<Option<ApiClient>>> = OnceLock::new();

fn agent_client_slot() -> &'static Mutex<Option<ApiClient>> {
    AGENT_CLIENT.get_or_init(|| Mutex::new(None))
}

/// Remember the running agent's client so token refreshes can reach it.
pub fn register_agent_client(client: ApiClient) {
    *agent_client_slot().lock().unwrap() = Some(client);
}

/// Forget the agent's client (logout / agent stop).
pub fn clear_agent_client() {
    *agent_client_slot().lock().unwrap() = None;
}

/// Push a freshly-rotated bearer token into the running agent's client, if any.
/// Called from the cloud token-refresh path so the agent's long-lived client
/// keeps working across JWT expiry. No-op when no agent is running.
pub fn update_agent_token(token: &str) {
    if let Some(client) = agent_client_slot().lock().unwrap().as_ref() {
        client.set_token(token);
    }
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
    // Keep a handle on the agent's client so the cloud token-refresh path can
    // swap in a fresh JWT as it rotates (see `update_agent_token`). `ApiClient`
    // shares its token cell across clones, so this clone stays in lock-step.
    register_agent_client(client.clone());
    let saves = hydrate_watched_saves(&state).map_err(|e| e.to_string())?;
    let watched_count = saves.len();

    // Pull the user's auto-restore preference + conflict retention so the
    // agent knows whether to pull missing snapshots on attach and how long
    // to keep conflict backups under `<state_dir>/conflicts/`.
    let prefs_loaded = Prefs::load_default().ok();
    let auto_restore = prefs_loaded
        .as_ref()
        .map(|(p, _)| p.auto_restore)
        .unwrap_or(false);
    let conflict_retention_days = prefs_loaded
        .as_ref()
        .map(|(p, _)| p.conflict_retention_days)
        .unwrap_or(14);
    // "Ahorro de datos" knob → minimum interval between snapshots per save
    // (ADR 0018 eje A). Maps `data_saving ∈ [0,1]` to 5s..600s via the
    // shared lerp helper so the cadence floor matches the server-side
    // retention scaling.
    let min_snapshot_interval_secs = prefs_loaded
        .as_ref()
        .map(|(p, _)| agent::min_snapshot_interval_for(p.data_saving))
        .unwrap_or_else(|| agent::min_snapshot_interval_for(0.3));
    // state_dir resolution can fail on locked-down hosts (no $HOME etc).
    // When it does, fall back to None — the agent then keeps the legacy
    // "never destroy local" behaviour for conflicts.
    let conflict_root = CliConfig::state_dir().ok().map(|d| d.join("conflicts"));
    let config = AgentConfig {
        auto_restore,
        conflict_root,
        conflict_retention_days,
        min_snapshot_interval_secs,
        ..AgentConfig::default()
    };

    let (events_tx, mut events_rx) = mpsc::channel::<AgentEvent>(256);
    // Capture per-save metadata before moving `saves` into the agent so we
    // can fire `agent://watcher-armed` for each slot right after spawn.
    // The frontend uses these events to flip the LiveStatus indicator to
    // "watching" without having to round-trip `agent_status`.
    let armed: Vec<WatcherArmed> = saves
        .iter()
        .map(|s| WatcherArmed {
            save_id: s.save_id.clone(),
            game_slug: s.game_slug.clone(),
        })
        .collect();
    let (handle, _task) = agent::spawn(client, config, saves, events_tx);

    // Fan out the synthetic watcher-armed events. Done after `spawn` so the
    // frontend's "armed" count never exceeds what the agent actually
    // tracks. Best-effort — emit failures fall through.
    for entry in &armed {
        let _ = app.emit("agent://watcher-armed", entry);
    }

    // Forwarder task — translate AgentEvent into Tauri events the UI can
    // subscribe to. `agent://*` is our private event namespace; the
    // dashboard listens with a single `listen("agent://...")` per type.
    //
    // A handful of events are also re-emitted under second, UX-friendlier
    // topics so the new LiveStatus + ActivityFeed surface can subscribe to
    // semantic names ("upload", "throttled") without forcing every legacy
    // listener to rename. The original `agent://backup-*` topics stay live.
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
                AgentEvent::SaveConflictsBackedUp { .. } => "agent://save-conflicts-backed-up",
            };
            let _ = app_for_emit.emit(topic, &ev);

            // Aliases for the new UX surface. Same payload, friendlier
            // channel name. `throttled` only fires for the filesystem-
            // settled flavour of BackupScheduled — the "game stopped"
            // and "manual" reasons aren't throttling, they're triggers.
            match &ev {
                AgentEvent::BackupStarted { .. } => {
                    let _ = app_for_emit.emit("agent://upload-started", &ev);
                }
                AgentEvent::BackupSuccess { .. } => {
                    let _ = app_for_emit.emit("agent://upload-completed", &ev);
                }
                AgentEvent::BackupScheduled {
                    reason: hoard_agent::agent::BackupReason::FilesystemSettled,
                    ..
                } => {
                    let _ = app_for_emit.emit("agent://throttled", &ev);
                }
                _ => {}
            }
        }
    });

    *state.agent.lock().unwrap() = Some(handle);
    Ok(AgentStatus {
        running: true,
        watched_count,
    })
}

#[derive(Debug, Clone, Serialize)]
struct WatcherArmed {
    save_id: String,
    game_slug: String,
}

/// Tear the agent down. Used on logout and on app exit (best-effort).
#[tauri::command]
pub async fn stop_agent(state: State<'_, AppState>) -> Result<(), String> {
    let handle = state.agent.lock().unwrap().take();
    clear_agent_client();
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

/// Kick a staggered backup sweep across every tracked save. Backs the Modo
/// Automático backup tick (driven from Rust by `automatic::run_backup_sweep`):
/// instead of the old "loop `backup_now`
/// over every save" burst, the agent spreads each save's re-hash across an
/// effective window (grown when there are tens of GB of saves) so sustained
/// disk use stays low. The nominal window is the persisted
/// `automatic_backup_interval_secs`. No-op (not an error) when the agent isn't
/// running — the next login boots it and the next tick sweeps.
#[tauri::command]
pub async fn sweep_backups(state: State<'_, AppState>) -> Result<(), String> {
    let handle = state.agent.lock().unwrap().clone();
    let Some(h) = handle else {
        return Ok(());
    };
    let window_secs = Prefs::load_default()
        .map(|(p, _)| p.automatic_backup_interval_secs)
        .unwrap_or(3600);
    h.sweep_all(window_secs).await.map_err(|e| e.to_string())?;
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

        let policy = resolve_policy(&save_state.game_slug, save_state.preset.as_deref());
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
            processes: resolve_processes(&save_state.game_slug),
            policy,
            known_version: save_state.last_version_num,
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

/// Resolve a save's effective sync policy: the user-pinned preset wins; with
/// none pinned, fall back to our built-in catalog for known-quirky games
/// (R.E.P.O. → short-session). An unknown name yields the empty (inherit-all)
/// policy.
pub(crate) fn resolve_policy(game_slug: &str, stored_preset: Option<&str>) -> SavePolicy {
    let name = stored_preset.or_else(|| presets::builtin_preset_for(game_slug));
    SavePolicy::from_preset(name)
}

/// Path-aware helper used by the library command when adding a save and the
/// agent is running. Builds a `WatchedSave` from minimal inputs, resolving the
/// save's sync policy from `preset` (or the built-in catalog).
pub(crate) fn watched_save_from(
    save_id: String,
    game_slug: String,
    display_name: String,
    label: String,
    local_path: PathBuf,
    preset: Option<&str>,
) -> WatchedSave {
    let steam_apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();
    let steam_install_dir = steam_apps
        .iter()
        .find(|a| name_matches(&a.name, &game_slug))
        .map(|a| a.install_dir.clone());
    let policy = resolve_policy(&game_slug, preset);
    let processes = resolve_processes(&game_slug);
    WatchedSave {
        save_id,
        game_slug,
        display_name,
        label,
        local_path,
        steam_install_dir,
        processes,
        policy,
        // Freshly tracked or just-added save: nothing committed from here yet,
        // so leave the gate open. Once the first backup lands, the slot's
        // `known_version` advances via `BackupDone`.
        known_version: None,
    }
}

/// Process names that mark a game as "running" for slugs the storefront can't
/// supply (TLauncher Minecraft, native Factorio). Pulled from the built-in
/// catalog so "is the game open" works without a Steam install dir.
pub(crate) fn resolve_processes(game_slug: &str) -> Vec<String> {
    presets::builtin_processes_for(game_slug)
        .iter()
        .map(|s| s.to_string())
        .collect()
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
