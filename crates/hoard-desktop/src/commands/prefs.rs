//! User-preference commands.
//!
//! Prefs are loaded from disk on every read because the file is small
//! (a few hundred bytes) and the user can edit it externally if they really
//! want to. Writes go through a Mutex so that concurrent toggles from the
//! Settings page don't race against each other.
//!
//! `set_autostart` is a thin wrapper that pokes the autostart plugin and
//! mirrors the resulting state into prefs so the UI reflects reality even if
//! the user disabled the launcher entry from outside Hoard.

use hoard_agent::prefs::Prefs;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::commands::automatic;
use crate::commands::error::AppError;
use crate::state::AppState;
use crate::tray::{TrayController, TrayState};

/// Read the prefs file from disk. Cheap; called by the Settings page on mount.
#[tauri::command]
pub fn get_prefs() -> Result<Prefs, String> {
    let (prefs, _) = Prefs::load_default().map_err(|e| e.to_string())?;
    Ok(prefs)
}

/// Persist a new prefs object. We replace wholesale rather than merging
/// individual fields — the form on the frontend always submits the full
/// object so there's nothing to lose, and partial-update semantics tend to
/// surprise users who edit prefs.json by hand.
///
/// Side-effect: if `auto_restore` changed and the live agent is running,
/// push the new value into it via `AgentHandle::set_auto_restore`. The
/// agent applies it to its config and, on a `false → true` flip, kicks
/// an immediate reconciliation sweep so the user doesn't have to restart
/// the app to see the new behaviour. Failures here are non-fatal —
/// prefs.json is already saved, and the change will take effect the
/// next time the agent reads its config (worst case, on next boot).
#[tauri::command]
pub async fn save_prefs(state: State<'_, AppState>, prefs: Prefs) -> Result<Prefs, String> {
    let path = Prefs::default_path().map_err(|e| e.to_string())?;
    let prev = Prefs::load(&path).ok();
    prefs.save(&path).map_err(|e| e.to_string())?;

    let auto_restore_changed = match &prev {
        Some(p) => p.auto_restore != prefs.auto_restore,
        // No prior file (first save ever) → treat current value as "new"
        // so the agent picks it up even if it was already running.
        None => true,
    };
    if auto_restore_changed {
        let handle = state.agent.lock().unwrap().clone();
        if let Some(h) = handle {
            if let Err(e) = h.set_auto_restore(prefs.auto_restore).await {
                tracing::warn!(
                    error = %e,
                    "couldn't push auto_restore preference to live agent"
                );
            }
        }
    }
    Ok(prefs)
}

/// Enable or disable the autostart entry. We toggle via the plugin and only
/// then mirror the new value into prefs — if the OS rejects the change (no
/// permission, sandboxed environment) we surface the error and leave prefs
/// untouched so the UI stays honest.
#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| format!("Couldn't update autostart: {e}"))?;

    // Re-query rather than trusting our own input — the plugin sometimes
    // refuses on Linux distros without a `~/.config/autostart` directory and
    // we want to reflect that.
    let actually_enabled = manager
        .is_enabled()
        .map_err(|e| format!("Couldn't read autostart status: {e}"))?;

    let path = Prefs::default_path().map_err(|e| e.to_string())?;
    let mut prefs = Prefs::load(&path).map_err(|e| e.to_string())?;
    prefs.autostart = actually_enabled;
    prefs.save(&path).map_err(|e| e.to_string())?;

    Ok(actually_enabled)
}

/// Read whether autostart is currently enabled. Used on the Settings page
/// load so we don't trust a stale value in prefs.json.
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Couldn't read autostart status: {e}"))
}

/// Flip the sidebar's "Modo Automático" toggle. Persists the new value to
/// `prefs.json`, cascades `auto_restore = true` on activation (and only on
/// activation — turning the toggle off intentionally leaves `auto_restore`
/// alone so the user can keep auto-restore independently), and starts or
/// stops the background scheduler accordingly.
///
/// The cascade direction was a deliberate choice: activating Modo
/// Automático implies "do everything for me", so silently enabling
/// auto-restore is the obvious follow-through. Deactivating it, on the
/// other hand, is "don't scan periodically" — it shouldn't pull the rug
/// out from under a user who explicitly toggled auto-restore on a week ago
/// and forgot.
///
/// Errors are returned as `AppError` so the frontend `showError` flow
/// handles them uniformly with the rest of the 1.5.3 command surface.
#[tauri::command]
pub async fn set_automatic_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<Prefs, AppError> {
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    let prev_auto_restore = prefs.auto_restore;

    prefs.automatic_mode = enabled;
    let mut auto_restore_changed = false;
    if enabled && !prefs.auto_restore {
        prefs.auto_restore = true;
        auto_restore_changed = true;
    }

    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;

    // Push the new auto_restore into the live agent if it cascaded on. We
    // mirror the side-effect that `save_prefs` performs for the same field
    // so toggling Modo Automático behaves identically to flipping the
    // auto-restore checkbox in Settings.
    if auto_restore_changed && prefs.auto_restore != prev_auto_restore {
        let handle = state.agent.lock().unwrap().clone();
        if let Some(h) = handle {
            if let Err(e) = h.set_auto_restore(prefs.auto_restore).await {
                tracing::warn!(
                    error = %e,
                    "couldn't push auto_restore preference to live agent from set_automatic_mode"
                );
            }
        }
    }

    if enabled {
        automatic::start(&app, prefs.automatic_scan_interval_hours);
    } else {
        automatic::stop(&app);
    }

    Ok(prefs)
}

/// Frontend-driven tray-state setter. The dashboard already aggregates agent
/// events into a single per-save activity map; we let it derive the global
/// status and tell us, rather than re-implementing that logic in Rust.
#[tauri::command]
pub fn set_tray_state(app: AppHandle, state: String) -> Result<(), String> {
    let parsed = match state.as_str() {
        "idle" => TrayState::Idle,
        "running" => TrayState::Running,
        "uploading" => TrayState::Uploading,
        "ok" => TrayState::Ok,
        "error" => TrayState::Error,
        "offline" => TrayState::Offline,
        other => return Err(format!("unknown tray state '{other}'")),
    };
    app.state::<TrayController>().set_state(parsed);
    Ok(())
}
