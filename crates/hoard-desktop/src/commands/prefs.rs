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
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

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
#[tauri::command]
pub fn save_prefs(prefs: Prefs) -> Result<Prefs, String> {
    let path = Prefs::default_path().map_err(|e| e.to_string())?;
    prefs.save(&path).map_err(|e| e.to_string())?;
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
