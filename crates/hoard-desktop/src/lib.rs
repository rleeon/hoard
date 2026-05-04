//! Hoard desktop app entry point.
//!
//! The actual Tauri builder lives in `lib.rs` so that `cargo tauri build` can
//! reuse it from `main.rs` (binary) and from a future mobile target. For now
//! we're desktop-only.

mod commands;
mod state;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,hoard=debug")),
        )
        .init();

    tauri::Builder::default()
        // Single instance: clicking the launcher again brings the existing
        // window to the front instead of spawning a second copy.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        // Persistent KV store for the frontend (wizard step, UI prefs).
        // We don't read it from Rust today; later phases probably will.
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::from_disk())
        .invoke_handler(tauri::generate_handler![
            commands::misc::greet,
            commands::auth::health_check,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::is_logged_in,
            commands::auth::current_user,
            commands::library::scan_library,
            commands::library::cached_detection,
            commands::library::add_game_to_tracking,
            commands::library::list_tracked_saves,
            commands::library::untrack_save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
