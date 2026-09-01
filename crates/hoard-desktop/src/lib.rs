//! Hoard desktop app entry point.
//!
//! The actual Tauri builder lives in `lib.rs` so that `cargo tauri build` can
//! reuse it from `main.rs` (binary) and from a future mobile target. For now
//! we're desktop-only.

mod commands;
mod daemon;
mod screen_telemetry;
mod state;
mod tray;

use hoard_agent::config::CliConfig;
use hoard_agent::prefs::Prefs;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Log timestamp timer that renders in the user's local timezone and labels
/// the offset, so a reader never has to guess what clock a line is on. When
/// the machine *is* on UTC (offset zero, or the offset couldn't be resolved
/// and we fell back) the suffix is the literal `UTC`; otherwise it's the
/// numeric offset, e.g. `+02:00`.
#[derive(Clone, Copy)]
struct LocalTimer {
    offset: time::UtcOffset,
}

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        const FMT: &[time::format_description::BorrowedFormatItem<'_>] = time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
        );
        let now = time::OffsetDateTime::now_utc().to_offset(self.offset);
        let body = now.format(&FMT).map_err(|_| std::fmt::Error)?;
        let secs = self.offset.whole_seconds();
        if secs == 0 {
            write!(w, "{body} UTC")
        } else {
            let sign = if secs < 0 { '-' } else { '+' };
            let h = secs.abs() / 3600;
            let m = (secs.abs() % 3600) / 60;
            write!(w, "{body} {sign}{h:02}:{m:02}")
        }
    }
}

use crate::commands::automatic::AutomaticScheduler;
use crate::state::AppState;
use crate::tray::{TrayController, TrayState};

/// Route panics through `tracing` so they reach the log file, not just stderr.
///
/// A bundled GUI app has nowhere to print: the default hook writes the panic to
/// stderr, which on a double-clicked desktop app is `/dev/null`. So a task that
/// died of a panic left **no trace whatsoever**: the ADR 0021 D.12 poller was
/// exactly that, a `tokio::spawn` that stopped existing between two log lines,
/// and reading the log the failure was indistinguishable from a healthy loop
/// that simply had nothing to say. A background task that dies in silence is a
/// bug in its own right; this makes any future one land in the file the in-app
/// Logs viewer reads.
///
/// Chains to the previous hook so `cargo tauri dev` still gets the usual stderr
/// dump (and any backtrace `RUST_BACKTRACE` asks for).
fn install_panic_logger() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("<unnamed>").to_string();
        tracing::error!(
            location = %location,
            thread = %thread,
            message = %panic_message(info.payload()),
            "PANIC: a task or thread died"
        );
        previous(info);
    }));
}

/// Best-effort text of a panic payload (`panic!("literal")` and
/// `panic!("{fmt}")` cover everything we throw in practice).
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMABUF renderer breaks compositing on a number of setups
    // (NVIDIA, nested/remote X, some Mesa versions): transparent webviews go
    // black and reshaped regions (the Pro overlay's web panels) come back torn.
    // Forcing the non-DMABUF path keeps GL compositing but avoids that buffer
    // sharing. Linux-only and overridable: only set when the user hasn't.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Set up tracing with two layers: stdout (for `RUST_LOG=...` development
    // and journald/Console capture in production) and a daily-rotating file
    // under the user's cache dir. The file layer is what the in-app Logs
    // viewer reads.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,hoard=debug"));

    // Resolve the local UTC offset *before* any threads spawn, because `time`'s
    // `current_local_offset` refuses to read the environment once the process
    // is multi-threaded (it's a soundness guard on POSIX). If it fails we fall
    // back to UTC, which the timer then labels as such.
    let timer = LocalTimer {
        offset: time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
    };
    // Hand the same pre-thread offset to the playtime tracker so its per-day
    // buckets land on the user's local calendar, matching the recap's binning.
    hoard_agent::playtime::set_local_offset(timer.offset);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_timer(timer),
        )
        // Ship events to the connected server (best-effort, drop-on-full).
        // Inert until a session + log-accepting server are present.
        .with(hoard_agent::logship::start());

    // We deliberately let log-init failures fall through to "stdout-only":
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
                            .with_timer(timer)
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
    // Hold the non-blocking guard for the lifetime of the process: when it
    // drops the writer thread is joined and any buffered lines flushed.
    // We park it on a `Box::leak` because Tauri's event loop is the actual
    // main loop and we can't easily thread the guard through it. Leaking is
    // cheap (~one Arc) and equivalent semantically to "live forever".
    if let Some(g) = _file_guard {
        Box::leak(Box::new(g));
    }

    install_panic_logger();

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
            // already running arrives as a *second launch*, since the OS hands the
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
        // Deep link: the `hoard://` scheme is owned by this app. The OAuth
        // callback from Hoard Cloud (`hoard://auth/callback?access_token=...`)
        // is forwarded to the frontend via the `deep-link://new-url` event
        // and consumed by the cloud store in /account.
        .plugin(tauri_plugin_deep_link::init())
        // Global hotkey support. No accelerator is bound here; the Pro overlay
        // UI registers Ctrl+O at runtime when present. Community build: unused.
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::from_disk())
        .manage(TrayController::default())
        .manage(AutomaticScheduler::default())
        .manage(commands::cloud_pull::CloudPullScheduler::default())
        .manage(commands::cloud_realtime::RealtimeScheduler::default())
        // Gates of the devices + bell feeds. Missing here until ADR 0021 D.12,
        // and `app.state::<T>()` panics on unmanaged state: every `kick_*` call
        // took its caller's task down with it, including the cloud-pull timer
        // loop, which is why the poller died after exactly one tick.
        .manage(commands::cloud_feed::CloudFeed::default())
        .manage(commands::selfhosted_events::SelfHostedEventsScheduler::default())
        .manage(commands::screen::ScreenProc::default())
        .manage(screen_telemetry::ScreenTelemetry::default())
        // La ventana nace oculta (`"visible": false`) y este flag decide si
        // llega a mostrarse: en un arranque silencioso, no.
        .manage(commands::window::StartHidden::default())
        .invoke_handler(tauri::generate_handler![
            commands::misc::greet,
            commands::window::ui_ready,
            // HUD sobre el juego (la app normal, no Hoard-Screen).
            commands::overlay::overlay_toggle,
            commands::overlay::overlay_set_visible,
            commands::overlay::overlay_is_visible,
            commands::misc::open_external,
            commands::covers::cover_bytes,
            commands::covers::steam_app_id_for_slug,
            commands::covers::has_custom_cover,
            commands::covers::set_custom_cover,
            commands::covers::remove_custom_cover,
            commands::wrapple::wrapple_read_image,
            commands::wrapple::wrapple_set_avatar,
            commands::wrapple::wrapple_avatar_bytes,
            commands::wrapple::wrapple_clear_avatar,
            commands::wrapple::wrapple_save_card,
            commands::auth::health_check,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::is_logged_in,
            commands::auth::current_user,
            commands::auth::refresh_quota,
            commands::library::scan_library,
            commands::library::rescan_library,
            commands::library::deep_scan_library,
            commands::library::scan_folder,
            commands::library::cached_detection,
            commands::library::add_game_to_tracking,
            commands::library::adopt_save,
            commands::library::list_tracked_saves,
            commands::library::untrack_save,
            commands::library::delete_save_completely,
            commands::library::rename_save_label,
            commands::library::set_save_slot_name,
            commands::library::renumber_save_slot,
            commands::library::set_manual_path,
            commands::library::clear_manual_path,
            commands::emulators::list_emulator_presets,
            commands::emulators::list_emulator_titles,
            commands::emulators::list_running_processes,
            commands::library::ignore_detected_game,
            commands::library::unignore_detected_game,
            commands::library::exclude_scan_path,
            commands::library::unexclude_scan_path,
            commands::library::list_excluded_scan_paths,
            commands::library::list_ignored_slugs,
            commands::library::detection_diagnostics,
            commands::library::detected_paths_for_game,
            commands::agent::start_agent,
            commands::agent::stop_agent,
            commands::agent::attach_agent_events,
            commands::agent::agent_snapshot,
            commands::agent::detach_agent_events,
            commands::agent::backup_now,
            commands::agent::sweep_backups,
            commands::agent::agent_status,
            commands::prefs::get_prefs,
            commands::prefs::save_prefs,
            commands::prefs::set_autostart,
            commands::prefs::is_autostart_enabled,
            commands::prefs::service_autostart_state,
            commands::prefs::set_automatic_mode,
            commands::prefs::set_global_sync,
            commands::prefs::set_sync_mode,
            commands::prefs::set_scan_interval,
            commands::prefs::set_backup_interval,
            commands::prefs::set_conflict_retention,
            commands::prefs::set_live_activity_visible,
            commands::prefs::set_data_saving,
            commands::prefs::set_tray_state,
            commands::playtime::list_playtime,
            commands::playtime::list_playtime_games,
            commands::playtime::exclude_playtime_game,
            commands::playtime::include_playtime_game,
            commands::history::list_save_snapshots,
            commands::history::preview_restore,
            commands::history::save_snapshot_detail,
            commands::history::delete_snapshot,
            commands::history::undelete_snapshot,
            commands::history::get_max_versions,
            commands::history::preview_max_versions,
            commands::history::set_max_versions,
            commands::history::restore_snapshot,
            commands::history::set_save_paused,
            commands::history::set_save_local_path,
            commands::history::list_save_presets,
            commands::history::set_save_preset,
            commands::history::set_save_allow_config,
            commands::history::tail_logs,
            commands::history::logs_path,
            commands::catalog::update_catalog,
            commands::catalog::catalog_status,
            commands::updates::update_status,
            commands::updates::restart_app,
            commands::updates::apply_staged_update,
            commands::updates::snooze_update,
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
            commands::cloud::cloud_export_status,
            commands::cloud::cloud_storage_games,
            commands::cloud::cloud_archive_save,
            commands::cloud::cloud_reactivate_save,
            commands::cloud::cloud_delete_account,
            commands::cloud::cloud_accept_terms,
            commands::cloud::cloud_terms_status,
            commands::cloud::cloud_reactivate_account,
            commands::cloud::cloud_entitlements,
            commands::cloud::cloud_activate_feature,
            commands::cloud::cloud_sync_playtime,
            commands::cloud_feed::notifications_backlog,
            commands::cloud_feed::devices_refresh,
            commands::devices::devices_list,
            commands::cloud_feed::notification_dismiss,
            commands::screen::screen_open,
            commands::screen::screen_send,
            commands::screen::screen_close,
            commands::screen::screen_is_open,
            commands::screen::screen_list_windows,
            commands::screen::screen_list_monitors,
            commands::screen::screen_note,
        ])
        .setup(|app| {
            // Build the tray as soon as we have an AppHandle. Failures here
            // shouldn't kill the app: Linux desktops without an AppIndicator
            // host (some minimal Wayland sessions) will reject our tray and
            // we want to keep running with just the window visible.
            //
            // A missing libayatana-appindicator3/libappindicator3 (no distro
            // package, or a Flatpak runtime that doesn't ship it) doesn't
            // surface as an `Err` here: `libappindicator-sys` panics straight
            // out of its dlopen check instead, which skips right past the
            // `Result` handling below. `catch_unwind` is what actually
            // delivers on the comment above in that case.
            let tray_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tray::install(&app.handle().clone())
            }));
            match tray_result {
                Ok(Ok(())) => {
                    // Apply offline as the initial state; the agent forwarder
                    // will recolour to idle as soon as it boots.
                    app.state::<TrayController>().set_state(TrayState::Offline);
                }
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "couldn't install tray icon");
                }
                Err(_) => {
                    tracing::warn!(
                        "tray icon backend panicked (missing libayatana-appindicator3 / \
                         libappindicator3?), continuing without a tray icon"
                    );
                }
            }

            // First-run bootstrap + silent-boot handling. `prefs.json` missing
            // means a fresh install: apply the shipped defaults to the OS
            // (autostart on) since the pref alone is just a mirror: the
            // autostart plugin owns the real entry, and the Settings page
            // re-probes it, so without registering here the toggle would snap
            // back to off.
            let first_run = Prefs::default_path().map(|p| !p.exists()).unwrap_or(false);
            if let Ok((mut prefs, path)) = Prefs::load_default() {
                // Re-assert the OS autostart entry on *every* launch when the
                // user wants it on, not just first run. The OS entry drifts
                // out of sync behind our back, and since the Settings page
                // re-probes `is_enabled()` on mount, any drift snaps the toggle
                // back to off. Concretely:
                //   - Windows: `is_enabled()` returns false when the Task
                //     Manager / Settings "Startup apps" override disabled the
                //     entry (StartupApproved\Run), or when an MSI update rewrote
                //     the install path and dropped the HKCU\...\Run value. This
                //     is the "toggle keeps turning itself off, only on Windows"
                //     report; Linux only checks file existence so it doesn't
                //     flip, but its `.desktop` Exec can still go stale.
                //   - Both: after an update the recorded binary path can point
                //     at a version that no longer exists, so autostart silently
                //     launches nothing at login even while the toggle reads on.
                // `enable()` is idempotent: it rewrites the entry with the
                // current binary path and (on Windows) resets the StartupApproved
                // override to enabled. Disabling in-app clears `autostart`, so
                // we never fight a user who deliberately turned it off.
                if prefs.autostart {
                    use tauri_plugin_autostart::ManagerExt;
                    #[cfg(target_os = "linux")]
                    commands::prefs::ensure_autostart_dir();
                    match app.autolaunch().enable() {
                        Ok(()) => tracing::info!("autostart entry re-asserted at startup"),
                        Err(e) => {
                            // Best-effort: some minimal Linux sessions have no
                            // autostart dir we can write. Only demote the pref on
                            // a fresh install, where the failure means autostart
                            // truly never took; for an existing install a
                            // transient failure shouldn't silently wipe intent.
                            tracing::warn!(error = %e, "couldn't re-assert autostart at startup");
                            if first_run {
                                prefs.autostart = false;
                            }
                        }
                    }
                    // Persist so the mirror matches the OS truth from the start.
                    let _ = prefs.save(&path);
                }

                // And the other half of "start at login": the sync service (ADR
                // 0021). The app and the service are two processes, so registering
                // only the app would mean the sync did not run until somebody opened
                // the window, which is exactly what this design fixes. It is
                // reaffirmed on every start for the same reasons as the app's entry
                // (an update moves the binary), it is idempotent and cheap (it
                // rewrites nothing when the unit already matches), and it does **not**
                // touch a service that is already running.
                commands::prefs::sync_service_autostart(prefs.autostart);

                // And the third leg: recording which components this machine has,
                // and making `hoard` typeable in a terminal. It goes here because
                // whoever installs the app from the web never goes through `hoard
                // install`.
                commands::prefs::register_installation();

                // Silent start: the autostart entry launches Hoard with `--silent`
                // (see the plugin's init above), so at login the app stays in the
                // tray, while a manual double-click always shows the UI, even with
                // `start_minimised` set. Without this condition a fresh install would
                // start invisible.
                //
                // The window no longer has to be hidden here: it is born hidden and
                // only shown when the frontend calls `ui_ready`. What gets marked is
                // the opposite, that on this start it must not be shown at all.
                let silent = std::env::args().any(|a| a == "--silent");
                commands::window::mark_start_hidden(
                    &app.handle().clone(),
                    prefs.start_minimised && silent,
                );
            }

            // Y la red de seguridad de esa ventana oculta: si el frontend nunca
            // llega a pintar, la mostramos igualmente pasado el plazo. Una UI
            // rota se reporta; una app que "no abre", no.
            commands::window::spawn_fallback_show(app.handle().clone());

            // Kick off a background Ludusavi-catalog refresh if the cached
            // copy is missing or older than a week. Fire-and-forget: the
            // app keeps running on the embedded catalog while the
            // download happens, and the next launch picks up the fresh
            // override transparently.
            commands::catalog::auto_update_catalog_in_background(app.handle().clone());

            // Periodic detection refresh: if the cached scan is older than
            // 24h, redo it in the background so the Library page is fresh
            // when the user next opens it. Skipped entirely on a fresh
            // install (no cache) so we don't spam disk on first launch.
            commands::library::spawn_periodic_rescan(app.handle().clone());

            // Re-arm the automatic-mode scheduler if the user had it on
            // before the app last closed. The scheduler state singleton is
            // already managed above; this fire-and-forget task just reads
            // prefs.json and (if the toggle was on) calls `start()`.
            let auto_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::automatic::restart_if_enabled(&auto_handle).await {
                    tracing::warn!(error = %e, "couldn't rehydrate automatic-mode scheduler");
                }
            });

            // The cloud-pull poller, on an independent cadence from the hourly
            // scheduler above. Boots only when a cloud session exists on
            // disk; otherwise lies dormant until the user signs in (the
            // login command starts it explicitly).
            let cloud_pull_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = commands::cloud_pull::restart_if_enabled(&cloud_pull_handle).await {
                    tracing::warn!(error = %e, "couldn't rehydrate cloud-pull poller");
                }
                // Realtime push rides alongside the poller: it accelerates
                // "something changed" from up to one poll interval down to
                // about 1 s. Best-effort: the poll above is the fallback.
                commands::cloud_realtime::restart_if_enabled(&cloud_pull_handle);
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
            // (the common Linux/Windows case, where the app wasn't running when the
            // user clicked the link), the URL arrives as a launch argument and
            // neither the single-instance callback (we ARE the first instance)
            // nor a runtime `on_open_url` necessarily fires before the webview
            // mounts. Scan argv ourselves and buffer the URL; the frontend
            // drains it on mount. Don't emit: no listener exists yet.
            if let Some(url) = first_hoard_url(std::env::args()) {
                tracing::info!(url = %url, "deep link via launch argv (cold start)");
                capture_deep_link(app.handle(), url, false);
            }

            // On Linux/Windows the desktop entry handles the scheme, but in
            // `cargo tauri dev` (no installer) we have to register at
            // runtime so the OS knows to dispatch `hoard://…` to us. This
            // is a no-op when the scheme is already registered.
            //
            // Skipped entirely under Flatpak: since flatpak registers through
            // the .desktop file.
            #[cfg(any(target_os = "linux", windows))]
            {
                if !hoard_agent::install::running_under_flatpak() {
                    if let Err(e) = app.deep_link().register("hoard") {
                        tracing::warn!(error = %e, "couldn't register hoard:// scheme at runtime");
                    }
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // The RunEvent loop: we hijack `WindowEvent::CloseRequested` so the X button
    // hides the window instead of quitting (when the user has opted into
    // close-to-tray, which is the default). Quitting goes through the tray's
    // Quit menu item or `app.exit(0)`.
    app.run(|app_handle, event| {
        // Quitting has nothing left to wait for. There used to be a block here until
        // no token rotation was halfway through (GoTrue rotates server-side before we
        // persist, and dying in that gap orphaned the new pair, losing the session on
        // the next start). What rotates is the service, and it outlives us closing.
        if let RunEvent::ExitRequested { .. } = event {
            return;
        }
        if let RunEvent::WindowEvent {
            label,
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            if label != "main" {
                return;
            }
            // Read prefs lazily: the user may have toggled close-to-tray
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
