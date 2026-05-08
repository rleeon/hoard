//! Hoard desktop app entry point.
//!
//! The actual Tauri builder lives in `lib.rs` so that `cargo tauri build` can
//! reuse it from `main.rs` (binary) and from a future mobile target. For now
//! we're desktop-only.

mod commands;
mod state;
mod tray;

use hoard_agent::config::CliConfig;
use hoard_agent::prefs::Prefs;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::state::AppState;
use crate::tray::{TrayController, TrayState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Set up tracing with two layers: stdout (for `RUST_LOG=...` development
    // and journald/Console capture in production) and a daily-rotating file
    // under the user's cache dir. The file layer is what the in-app Logs
    // viewer reads.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,hoard=debug"));

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true));

    // We deliberately let log-init failures fall through to "stdout-only" —
    // a corrupt cache_dir shouldn't keep the app from starting.
    let _file_guard = match CliConfig::logs_dir() {
        Ok(dir) => match std::fs::create_dir_all(&dir) {
            Ok(()) => {
                let appender = tracing_appender::rolling::daily(&dir, "agent.log");
                let (nb_writer, guard) = tracing_appender::non_blocking(appender);
                let _ = registry
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_target(true)
                            .with_ansi(false)
                            .with_writer(nb_writer),
                    )
                    .try_init();
                Some(guard)
            }
            Err(e) => {
                eprintln!("hoard: couldn't create logs dir ({e}); logging to stdout only");
                let _ = registry.try_init();
                None
            }
        },
        Err(e) => {
            eprintln!("hoard: couldn't resolve logs dir ({e}); logging to stdout only");
            let _ = registry.try_init();
            None
        }
    };
    // Hold the non-blocking guard for the lifetime of the process — when it
    // drops the writer thread is joined and any buffered lines flushed.
    // We park it on a `Box::leak` because Tauri's event loop is the actual
    // main loop and we can't easily thread the guard through it. Leaking is
    // cheap (~one Arc) and equivalent semantically to "live forever".
    if let Some(g) = _file_guard {
        Box::leak(Box::new(g));
    }

    let app = tauri::Builder::default()
        // Single instance: clicking the launcher again brings the existing
        // window to the front instead of spawning a second copy.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        // Persistent KV store for the frontend (wizard step, UI prefs).
        // We don't read it from Rust today; later phases probably will.
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::from_disk())
        .manage(TrayController::default())
        .invoke_handler(tauri::generate_handler![
            commands::misc::greet,
            commands::auth::health_check,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::is_logged_in,
            commands::auth::current_user,
            commands::auth::refresh_quota,
            commands::library::scan_library,
            commands::library::cached_detection,
            commands::library::add_game_to_tracking,
            commands::library::list_tracked_saves,
            commands::library::untrack_save,
            commands::agent::start_agent,
            commands::agent::stop_agent,
            commands::agent::backup_now,
            commands::agent::agent_status,
            commands::prefs::get_prefs,
            commands::prefs::save_prefs,
            commands::prefs::set_autostart,
            commands::prefs::is_autostart_enabled,
            commands::prefs::set_tray_state,
            commands::history::list_save_snapshots,
            commands::history::save_snapshot_detail,
            commands::history::delete_snapshot,
            commands::history::undelete_snapshot,
            commands::history::restore_snapshot,
            commands::history::set_save_paused,
            commands::history::set_save_local_path,
            commands::history::tail_logs,
            commands::history::logs_path,
            commands::catalog::update_catalog,
            commands::catalog::catalog_status,
        ])
        .setup(|app| {
            // Build the tray as soon as we have an AppHandle. Failures here
            // shouldn't kill the app — Linux desktops without an AppIndicator
            // host (some minimal Wayland sessions) will reject our tray and
            // we want to keep running with just the window visible.
            if let Err(e) = tray::install(&app.handle().clone()) {
                tracing::warn!(error = %e, "couldn't install tray icon");
            } else {
                // Apply offline as the initial state — the agent forwarder
                // will recolour to idle as soon as it boots.
                app.state::<TrayController>().set_state(TrayState::Offline);
            }

            // If prefs say "start minimised", hide the main window before it
            // ever paints. Combined with autostart, this gives users a quiet
            // launch — Hoard appears only as a tray icon.
            if let Ok((prefs, _)) = Prefs::load_default() {
                if prefs.start_minimised {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            }

            // Kick off a background Ludusavi-catalog refresh if the cached
            // copy is missing or older than a week. Fire-and-forget — the
            // app keeps running on the embedded catalog while the
            // download happens, and the next launch picks up the fresh
            // override transparently.
            commands::catalog::auto_update_catalog_in_background(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // RunEvent loop — we hijack `WindowEvent::CloseRequested` so the X button
    // hides the window instead of quitting (when the user has opted into
    // close-to-tray, which is the default). Quitting goes through the tray's
    // Quit menu item or `app.exit(0)`.
    app.run(|app_handle, event| {
        if let RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            if label != "main" {
                return;
            }
            // Read prefs lazily — the user may have toggled close-to-tray
            // between launches. If reading fails for any reason, fall back
            // to the safe default of "hide the window" so a backup in flight
            // isn't dropped.
            let close_to_tray = Prefs::load_default()
                .map(|(p, _)| p.close_to_tray)
                .unwrap_or(true);

            if close_to_tray {
                api.prevent_close();
                if let Some(window) = app_handle.get_webview_window(&label) {
                    let _ = window.hide();
                }
                // The frontend pops a one-time toast explaining we're still
                // running. That's gated by `Prefs::seen_tray_hint` so power
                // users don't see it after they've understood the deal.
                let _ = tauri::Emitter::emit(app_handle, "tray://hidden-to-tray", ());
            }
        }
    });
}
