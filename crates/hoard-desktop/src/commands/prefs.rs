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

use hoard_agent::prefs::{Prefs, SyncMode};
use hoard_core::ipc::Request;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::commands::agent::push_pref;
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
/// individual fields: the form on the frontend always submits the full
/// object so there's nothing to lose, and partial-update semantics tend to
/// surprise users who edit prefs.json by hand.
///
/// Side-effect: if `auto_restore` changed, push the new value into the sync
/// service's engine (`Request::SetAutoRestore`). The engine applies it to its
/// config and, on a `false → true` flip, kicks an immediate reconciliation
/// sweep so the user doesn't have to restart anything to see the new
/// behaviour. Failures here are non-fatal: prefs.json is already saved, and
/// the service reads the same file the next time it starts its engine.
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
        push_pref(
            &state,
            Request::SetAutoRestore {
                enabled: prefs.auto_restore,
            },
        )
        .await;
    }

    let global_sync_changed = match &prev {
        Some(p) => p.global_sync != prefs.global_sync,
        None => true,
    };
    if global_sync_changed {
        push_pref(
            &state,
            Request::SetGlobalSync {
                enabled: prefs.global_sync,
            },
        )
        .await;
    }
    Ok(prefs)
}

/// Flip the sidebar's "Sync" toggle (sync global). Distinct from
/// `set_automatic_mode`: it doesn't start any scheduler and doesn't cascade
/// `auto_restore`. It just persists `global_sync` and pushes it into the
/// service's engine so the change takes effect without a restart. On a
/// `false → true` flip the engine sweeps immediately, pulling any outdated
/// save even mid-session.
#[tauri::command]
pub async fn set_global_sync(state: State<'_, AppState>, enabled: bool) -> Result<Prefs, AppError> {
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.global_sync = enabled;
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;

    push_pref(&state, Request::SetGlobalSync { enabled }).await;

    Ok(prefs)
}

/// Set the single user-facing operating mode (`backup_only` / `full_sync`).
/// This is the onboarding + Settings radio: it maps the chosen [`SyncMode`]
/// onto the internal `global_sync` / `auto_restore` flags, persists prefs, and
/// hot-reconfigures the service's engine so the change takes effect without a
/// restart. Per-save presets still override as exceptions.
///
/// We push *both* flags when either changed, mirroring what `save_prefs` does
/// field by field: on a flip into `FullSync` the engine sweeps immediately and
/// pulls any outdated save.
#[tauri::command]
pub async fn set_sync_mode(state: State<'_, AppState>, mode: SyncMode) -> Result<Prefs, AppError> {
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    let prev_auto_restore = prefs.auto_restore;
    let prev_global_sync = prefs.global_sync;

    prefs.set_sync_mode(mode);
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;

    if prefs.global_sync != prev_global_sync {
        push_pref(
            &state,
            Request::SetGlobalSync {
                enabled: prefs.global_sync,
            },
        )
        .await;
    }
    if prefs.auto_restore != prev_auto_restore {
        push_pref(
            &state,
            Request::SetAutoRestore {
                enabled: prefs.auto_restore,
            },
        )
        .await;
    }

    Ok(prefs)
}

/// Enable or disable the autostart entry. We toggle via the plugin and only
/// then mirror the new value into prefs: if the OS rejects the change (no
/// permission, sandboxed environment) we surface the error and leave prefs
/// untouched so the UI stays honest.
#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    // On Linux the autostart plugin writes `~/.config/autostart/<app>.desktop`
    // but does *not* create the directory itself, and on a fresh XDG profile that
    // folder often doesn't exist yet, so `enable()` fails and autostart never
    // takes. Create it up front so enabling is reliable.
    #[cfg(target_os = "linux")]
    if enabled {
        ensure_autostart_dir();
    }

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| format!("Couldn't update autostart: {e}"))?;

    // Re-query rather than trusting our own input: the plugin sometimes
    // refuses on Linux distros without a `~/.config/autostart` directory and
    // we want to reflect that.
    let actually_enabled = manager
        .is_enabled()
        .map_err(|e| format!("Couldn't read autostart status: {e}"))?;

    let path = Prefs::default_path().map_err(|e| e.to_string())?;
    let mut prefs = Prefs::load(&path).map_err(|e| e.to_string())?;
    prefs.autostart = actually_enabled;
    prefs.save(&path).map_err(|e| e.to_string())?;

    // The sync service follows the same switch as the app: they are two processes,
    // so "start at login" has two entries to register, and turning it off has to
    // remove both, or the user turns autostart off and the sync keeps starting on its
    // own.
    //
    // Awaited, unlike the app-start reaffirmation: the user is standing in front
    // of the switch they just flipped, and a service half that failed has to be
    // known by the time this returns or the page has nothing to show. The reason
    // is typed and cached, so the page reads it back with
    // `service_autostart_state`.
    apply_service_autostart(actually_enabled).await;

    Ok(actually_enabled)
}

/// Writes this install into the manifest and leaves the terminal within reach.
///
/// The two halves of "installing the app installs the whole of Hoard", and both have
/// to run **here** and not in the installer: somebody who downloads the `.deb` from
/// the web never goes through `hoard install`, so if the app did not do this, that
/// machine would be left with no manifest (and its first `upgrade` would not know
/// what it updates) and with the terminal inside the bundle, present but
/// unwritable.
///
/// In the background and best-effort: neither half may delay or bring down the
/// window's start.
pub(crate) fn register_installation() {
    tauri::async_runtime::spawn(async move {
        match hoard_agent::install::Manifest::reconcile() {
            Ok(m) => tracing::info!(
                components = m
                    .components
                    .iter()
                    .map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                delivery = m.delivery.map(|d| d.as_str()).unwrap_or("-"),
                "install manifest reconciled"
            ),
            Err(err) => {
                tracing::warn!(error = %format!("{err:#}"), "couldn't reconcile the install manifest")
            }
        }
        use hoard_agent::install::CliReach;
        match hoard_agent::install::ensure_cli_reachable() {
            Ok(CliReach::AddedToPath(dir)) => tracing::info!(
                dir = %dir.display(),
                "added the bundled `hoard` command to the user PATH (needs a new terminal)"
            ),
            Ok(CliReach::Linked(path)) => {
                tracing::info!(path = %path.display(), "linked the bundled `hoard` command")
            }
            Ok(CliReach::AlreadyReachable) | Ok(CliReach::NotBundled) => {}
            // The expected AppImage case: its copy would disappear when the app
            // closed, so there the core installer is what puts the terminal in
            // place.
            Ok(CliReach::Unreachable(why)) => {
                tracing::info!(reason = %why, "the bundled `hoard` command stays out of PATH")
            }
            Err(err) => {
                tracing::warn!(error = %format!("{err:#}"), "couldn't make `hoard` reachable")
            }
        }
    });
}

/// How the sync service's login start actually went, for the window to show.
///
/// The interesting field is `unsupported`: a machine where login start can't be
/// declared at all (an AppImage that can't leave a stable copy of the daemon, a
/// box without systemd). That used to end in a `tracing::warn!` inside the
/// service, so the Settings switch read "on" while the sync only ever ran with
/// the window open, and the user had nothing to look at. It is `None` when
/// login start is registered, and when it's off because the user turned it off.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ServiceAutostart {
    /// Whether login start is meant to be on at all (mirrors `prefs.autostart`).
    pub enabled: bool,
    /// Which service manager took it (`"systemd --user"`, `"Task Scheduler"`,
    /// `"Startup entry (HKCU Run)"`). The one that *actually* took it: on
    /// Windows the task and the Run entry are two different answers.
    pub manager: Option<String>,
    /// Unit / label / task name, for a user who wants to ask the OS directly.
    pub unit: Option<String>,
    /// Typed reason there is no login start here, if there isn't one:
    /// `"no_stable_path"` / `"no_service_manager"`. The sentence comes from
    /// i18n keyed on this; the raw text is in `detail`.
    pub unsupported: Option<String>,
    /// Raw failure text, for the detail line and for a bug report. `None` when
    /// nothing failed.
    pub detail: Option<String>,
}

/// Last outcome of registering the service for login start.
///
/// Cached rather than probed on demand, because probing honestly would mean
/// doing the work: whether an AppImage can leave a stable copy of the daemon is
/// only answered by trying. This is what really happened on the last attempt: at
/// app start, and on every flip of the switch.
fn service_autostart_slot() -> &'static std::sync::Mutex<ServiceAutostart> {
    static SLOT: std::sync::OnceLock<std::sync::Mutex<ServiceAutostart>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(ServiceAutostart::default()))
}

fn record_service_autostart(state: ServiceAutostart) {
    let mut slot = service_autostart_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = state;
}

/// What the Settings page reads to explain a login start that isn't happening.
#[tauri::command]
pub fn service_autostart_state() -> ServiceAutostart {
    service_autostart_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Register (or remove) the sync service's login start and record how it went.
///
/// Returns the outcome instead of only logging it. The service manager
/// subprocesses (`systemctl`, `launchctl`, `schtasks`) are the slow part, but
/// none of these declare-or-remove calls starts or stops a running service, so
/// they're a couple of subprocesses and a file write, quick enough for the
/// Settings toggle to wait on, which is the only way its failure can ever be
/// said out loud.
pub(crate) async fn apply_service_autostart(enabled: bool) -> ServiceAutostart {
    let outcome = if enabled {
        hoardd::autostart::ensure_installed().await.map(|i| {
            tracing::info!(
                manager = i.manager,
                unit = i.id,
                "sync service set to start at login"
            );
            ServiceAutostart {
                enabled,
                manager: Some(i.manager.to_string()),
                unit: Some(i.id.to_string()),
                unsupported: None,
                detail: None,
            }
        })
    } else {
        hoardd::autostart::uninstall().await.map(|removed| {
            if removed {
                tracing::info!("sync service removed from login start");
            }
            ServiceAutostart {
                enabled,
                ..Default::default()
            }
        })
    };
    let state = match outcome {
        Ok(state) => state,
        Err(err) => {
            let detail = format!("{err:#}");
            let unsupported = hoardd::autostart::unsupported_reason(&err);
            tracing::warn!(
                error = %detail,
                enabled,
                unsupported = unsupported.map(|u| u.as_str()).unwrap_or("-"),
                "the sync service won't start at login"
            );
            ServiceAutostart {
                enabled,
                manager: None,
                unit: None,
                unsupported: unsupported.map(|u| u.as_str().to_string()),
                detail: Some(detail),
            }
        }
    };
    record_service_autostart(state.clone());
    state
}

/// The app-start half: reaffirm what prefs say without holding up the window.
pub(crate) fn sync_service_autostart(enabled: bool) {
    tauri::async_runtime::spawn(async move {
        apply_service_autostart(enabled).await;
    });
}

/// Best-effort creation of the XDG autostart directory
/// (`$XDG_CONFIG_HOME/autostart`, defaulting to `~/.config/autostart`). The
/// autostart plugin drops a `.desktop` file in here but won't `mkdir -p` the
/// parent, so on a clean profile enabling autostart silently fails. We
/// swallow errors: if we can't create it the subsequent `enable()` will
/// surface a real error to the caller.
#[cfg(target_os = "linux")]
pub(crate) fn ensure_autostart_dir() {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")));
    if let Some(base) = base {
        let dir = base.join("autostart");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!(error = %e, path = %dir.display(), "couldn't create autostart dir");
        }
    }
}

/// Read whether autostart is currently enabled. Used on the Settings page
/// load so we don't trust a stale value in prefs.json.
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Couldn't read autostart status: {e}"))
}

/// Flips the sidebar's automatic-mode toggle. Persists the new value to
/// `prefs.json`, cascades `auto_restore = true` on activation (and only on
/// activation: turning the toggle off intentionally leaves `auto_restore`
/// alone so the user can keep auto-restore independently), and starts or
/// stops the background scheduler accordingly.
///
/// The cascade direction was a deliberate choice: turning automatic mode on
/// implies "do everything for me", so silently enabling auto-restore is the
/// obvious follow-through. Turning it off, on the other hand, is "don't scan
/// periodically", and it shouldn't pull the rug out from under a user who
/// explicitly toggled auto-restore on a week ago and forgot.
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

    // Push the new auto_restore into the service's engine if it cascaded on. We
    // mirror the side-effect that `save_prefs` performs for the same field
    // so toggling automatic mode behaves identically to flipping the
    // auto-restore checkbox in Settings.
    if auto_restore_changed && prefs.auto_restore != prev_auto_restore {
        push_pref(
            &state,
            Request::SetAutoRestore {
                enabled: prefs.auto_restore,
            },
        )
        .await;
    }

    if enabled {
        automatic::start(
            &app,
            prefs.automatic_scan_interval_secs,
            prefs.automatic_backup_interval_secs,
        );
    } else {
        automatic::stop(&app);
    }

    Ok(prefs)
}

/// Persist a new detection-scan interval (in seconds) for automatic mode.
/// Caller is the Settings slider; range is 60..=3600 (1 min to 1 h), and the
/// scan is the cheap, metadata-only half so it's allowed to run often. If the
/// toggle is on we restart the schedulers so the new cadence applies
/// immediately (and, thanks to `automatic::start`'s tick-on-start, a scan
/// fires right after saving).
#[tauri::command]
pub async fn set_scan_interval(app: AppHandle, secs: u64) -> Result<Prefs, AppError> {
    if !(60..=3600).contains(&secs) {
        return Err(AppError::plain(format!(
            "scan interval out of range: {secs}s (expected 60..=3600)"
        )));
    }
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.automatic_scan_interval_secs = secs;
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;

    if prefs.automatic_mode {
        automatic::start(
            &app,
            prefs.automatic_scan_interval_secs,
            prefs.automatic_backup_interval_secs,
        );
    }

    Ok(prefs)
}

/// Persist a new backup-sweep interval (in seconds) for automatic mode.
/// Caller is the Settings slider; range is 300..=86400 (5 min to 24 h), and the
/// sweep re-hashes file bytes so it's the expensive half and runs rarely. The
/// agent staggers the per-save work across an effective window that grows with
/// the total save footprint, so this is the *nominal* cadence, not a hard
/// ceiling on a large set. Restarts the schedulers if the toggle is on.
#[tauri::command]
pub async fn set_backup_interval(app: AppHandle, secs: u64) -> Result<Prefs, AppError> {
    if !(300..=86400).contains(&secs) {
        return Err(AppError::plain(format!(
            "backup interval out of range: {secs}s (expected 300..=86400)"
        )));
    }
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.automatic_backup_interval_secs = secs;
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;

    if prefs.automatic_mode {
        automatic::start(
            &app,
            prefs.automatic_scan_interval_secs,
            prefs.automatic_backup_interval_secs,
        );
    }

    Ok(prefs)
}

/// Persist whether the floating ActivityFeed panel renders. Pure state
/// flip, with no side effects beyond writing prefs.json. The frontend reads the
/// new value through the standard prefs store subscription.
#[tauri::command]
pub async fn set_live_activity_visible(visible: bool) -> Result<Prefs, AppError> {
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.live_activity_visible = visible;
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;
    Ok(prefs)
}

/// Persist a new retention window (in days) for per-save conflict backups.
/// Caller is the Settings slider; range is 1..=30. The agent reads this
/// value on its next auto-restore sweep, so no live restart is needed.
#[tauri::command]
pub async fn set_conflict_retention(_app: AppHandle, days: u32) -> Result<Prefs, AppError> {
    if !(1..=30).contains(&days) {
        return Err(AppError::plain(format!(
            "conflict retention out of range: {days} (expected 1..=30)"
        )));
    }
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.conflict_retention_days = days;
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;
    Ok(prefs)
}

/// Persist the "ahorro de datos" knob `k ∈ [0,1]` (ADR 0018). Caller is the
/// Settings slider. Clamped defensively to `[0,1]`. The value scales both the
/// client-side `min_snapshot_interval` and the server-side `RetentionPolicy`;
/// the new interval takes effect the next time the agent boots (logout/login
/// or app restart), so no live restart is wired here.
#[tauri::command]
pub async fn set_data_saving(saving: f64) -> Result<Prefs, AppError> {
    if !saving.is_finite() {
        return Err(AppError::plain(format!(
            "data_saving must be a finite number, got {saving}"
        )));
    }
    let path = Prefs::default_path().map_err(|e| AppError::plain(e.to_string()))?;
    let mut prefs = Prefs::load(&path).map_err(|e| AppError::plain(e.to_string()))?;
    prefs.data_saving = saving.clamp(0.0, 1.0);
    prefs
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;
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
