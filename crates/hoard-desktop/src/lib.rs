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
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::commands::automatic::AutomaticScheduler;
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
        .with(tracing_subscriber::fmt::layer().with_target(true))
        // Ship events to the connected server (best-effort, drop-on-full).
        // Inert until a session + log-accepting server are present.
        .with(hoard_agent::logship::start());

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
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            // On Linux/Windows a `hoard://…` deep link opened while the app is
            // already running arrives as a *second launch* — the OS hands the
            // URL to this callback as an argv entry, NOT through the deep-link
            // plugin's `on_open_url` channel (that one only fires on cold
            // start / macOS). Without forwarding it here the OAuth handoff is
            // silently dropped and the app never sees the session. The app is
            // already running so its listener is up: emit, and also buffer it
            // so a not-yet-mounted webview still drains it on mount.
            if let Some(url) = first_hoard_url(argv.iter().cloned()) {
                tracing::info!(url = %url, "deep link via single-instance argv");
                capture_deep_link(app, url, true);
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
        // Deep link — the `hoard://` scheme is owned by this app. The OAuth
        // callback from Hoard Cloud (`hoard://auth/callback?access_token=…`)
        // is forwarded to the frontend via the `deep-link://new-url` event
        // and consumed by the cloud store in /account.
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::from_disk())
        .manage(TrayController::default())
        .manage(AutomaticScheduler::default())
        .manage(commands::cloud_pull::CloudPullScheduler::default())
        .invoke_handler(tauri::generate_handler![
            commands::misc::greet,
            commands::auth::health_check,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::is_logged_in,
            commands::auth::current_user,
            commands::auth::refresh_quota,
            commands::library::scan_library,
            commands::library::rescan_library,
            commands::library::cached_detection,
            commands::library::add_game_to_tracking,
            commands::library::list_tracked_saves,
            commands::library::untrack_save,
            commands::library::delete_save_completely,
            commands::library::rename_save_label,
            commands::library::set_manual_path,
            commands::library::clear_manual_path,
            commands::library::ignore_detected_game,
            commands::library::unignore_detected_game,
            commands::library::list_ignored_slugs,
            commands::library::detection_diagnostics,
            commands::agent::start_agent,
            commands::agent::stop_agent,
            commands::agent::backup_now,
            commands::agent::agent_status,
            commands::prefs::get_prefs,
            commands::prefs::save_prefs,
            commands::prefs::set_autostart,
            commands::prefs::is_autostart_enabled,
            commands::prefs::set_automatic_mode,
            commands::prefs::set_scheduler_interval,
            commands::prefs::set_conflict_retention,
            commands::prefs::set_cloud_poll_interval,
            commands::prefs::set_live_activity_visible,
            commands::prefs::set_data_saving,
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
            commands::updates::check_for_updates,
            commands::updates::apply_desktop_update,
            commands::updates::apply_server_update,
            commands::updates::trigger_server_upgrade,
            commands::cloud::cloud_login_url,
            commands::cloud::cloud_complete_login,
            commands::cloud::cloud_take_pending_deep_link,
            commands::cloud::cloud_current_account,
            commands::cloud::cloud_is_logged_in,
            commands::cloud::cloud_refresh_account,
            commands::cloud::cloud_logout,
            commands::cloud::cloud_export_all,
            commands::cloud::cloud_delete_account,
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

            // Periodic detection refresh: if the cached scan is older than
            // 24h, redo it in the background so the Library page is fresh
            // when the user next opens it. Skipped entirely on a fresh
            // install (no cache) so we don't spam disk on first launch.
            commands::library::spawn_periodic_rescan(app.handle().clone());

            // Re-arm the Modo Automático scheduler if the user had it on
            // before the app last closed. The scheduler state singleton is
            // already managed above; this fire-and-forget task just reads
            // prefs.json and (if the toggle was on) calls `start()`.
            let auto_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::automatic::restart_if_enabled(&auto_handle).await {
                    tracing::warn!(error = %e, "couldn't rehydrate automatic-mode scheduler");
                }
            });

            // Cloud-pull poller — independent cadence from the hourly
            // scheduler above. Boots only when a cloud session exists on
            // disk; otherwise lies dormant until the user signs in (the
            // login command starts it explicitly).
            let cloud_pull_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::cloud_pull::restart_if_enabled(&cloud_pull_handle).await {
                    tracing::warn!(error = %e, "couldn't rehydrate cloud-pull poller");
                }
            });

            // Wire the deep-link receiver. `hoard://auth/callback?...` URLs
            // (clicked from the browser after a Supabase OAuth round-trip)
            // come in here on macOS (Apple events) and on some Linux/Windows
            // runtime deliveries. We forward the raw URL to the frontend and
            // also buffer it, so a webview that hasn't mounted its listener
            // yet still drains it on mount. We bring the window to the front
            // so the user sees the result.
            let dl_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    tracing::info!(url = %url, "deep link opened (on_open_url)");
                    if let Some(window) = dl_handle.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    capture_deep_link(&dl_handle, url.to_string(), true);
                }
            });

            // Cold start: when the OS launches us *fresh* with the callback URL
            // (the common Linux/Windows case — the app wasn't running when the
            // user clicked the link), the URL arrives as a launch argument and
            // neither the single-instance callback (we ARE the first instance)
            // nor a runtime `on_open_url` necessarily fires before the webview
            // mounts. Scan argv ourselves and buffer the URL; the frontend
            // drains it on mount. Don't emit — no listener exists yet.
            if let Some(url) = first_hoard_url(std::env::args()) {
                tracing::info!(url = %url, "deep link via launch argv (cold start)");
                capture_deep_link(app.handle(), url, false);
            }

            // On Linux/Windows the desktop entry handles the scheme, but in
            // `cargo tauri dev` (no installer) we have to register at
            // runtime so the OS knows to dispatch `hoard://…` to us. This
            // is a no-op when the scheme is already registered.
            #[cfg(any(target_os = "linux", windows))]
            {
                if let Err(e) = app.deep_link().register("hoard") {
                    tracing::warn!(error = %e, "couldn't register hoard:// scheme at runtime");
                }
            }
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

/// Pick the first `hoard://` URL out of a list of process arguments. The OS
/// hands the deep link to us as one of the argv entries (Linux/Windows); the
/// position isn't fixed (argv[0] is the binary, and some launchers prepend
/// flags), so we scan rather than index.
fn first_hoard_url<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    args.into_iter().find(|a| a.starts_with("hoard://"))
}

/// Stash a `hoard://` URL where the frontend can find it and, optionally,
/// emit the live `deep-link://new-url` event. We always buffer (so a webview
/// whose listener isn't ready yet still drains it on mount) and emit only when
/// a window already exists to receive it. The buffer is cleared on a
/// successful `cloud_complete_login`.
pub(crate) fn capture_deep_link(app: &tauri::AppHandle, url: String, emit: bool) {
    *app.state::<AppState>().pending_deep_link.lock().unwrap() = Some(url.clone());
    if emit {
        let _ = app.emit("deep-link://new-url", url);
    }
}
