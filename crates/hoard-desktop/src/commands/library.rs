//! Game-library commands: detection, tracking, and listing tracked saves.
//!
//! `scan_library` runs the auto-detection sweep (filesystem heuristic +
//! Steam library scan) against the catalog. Long scans emit
//! `library://scan-progress` events so the UI can show a progress bar.
//!
//! `add_game_to_tracking` records that the user wants Hoard to back up a
//! given game/path pair. It creates a Save row on the server and stores the
//! local path in the agent's on-disk state file.
//!
//! `list_tracked_saves` returns the saves the current user has registered,
//! enriched with the local path from state so the UI doesn't need a second
//! round-trip.

use std::path::PathBuf;
use std::sync::Mutex;

use hoard_agent::api::{ApiClient, ApiError};
use hoard_agent::config::CliConfig;
use hoard_agent::credentials;
use hoard_agent::detection::{self, DetectionReport};
use hoard_agent::manifest::Os;
use hoard_agent::state::{CliState, SaveState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use super::agent::{attach_save_if_running, detach_save_if_running, watched_save_from};
use super::auth::pretty_error;
use crate::state::AppState;

/// Persisted detection snapshot. Lives on disk alongside `state.json` so the
/// Library page paints immediately on cold start without a fresh sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDetection {
    pub report: DetectionReport,
    #[serde(with = "time::serde::rfc3339")]
    pub scanned_at: OffsetDateTime,
}

/// In-memory cache of the most recent scan, so the Library page can
/// re-render instantly when the user navigates back without forcing
/// another sweep. Wrapped in a `Mutex<Option<…>>` and stored on
/// `AppState`. Hydrated at boot from `detection.json` next to `state.json`;
/// `scan_library` re-writes both memory and disk atomically.
#[derive(Default)]
pub struct DetectionCache {
    pub last: Mutex<Option<CachedDetection>>,
}

/// Maximum age before the background scheduler triggers an automatic
/// rescan. 24 hours mirrors the catalog-refresh cadence so the two stay
/// loosely in step.
const STALE_AFTER_SECS: i64 = 60 * 60 * 24;
/// How often the background scheduler wakes to check the cache age.
/// 30 minutes is a good middle ground: long enough to be near-free,
/// short enough that the user doesn't have to wait a full sleep period
/// after the OS clock jumps forward.
const POLL_INTERVAL_SECS: u64 = 60 * 30;

/// Path to the persisted detection cache. Lives in the same dir as
/// `state.json` (see `hoard_agent::config::CliConfig::state_dir`).
pub fn detection_cache_path() -> anyhow::Result<PathBuf> {
    Ok(CliConfig::state_dir()?.join("detection.json"))
}

/// Load the on-disk detection cache. Returns `None` when the file is
/// missing, malformed, or otherwise unreadable — corruption is logged at
/// WARN and we fall back to a fresh boot rather than crashing the app.
pub fn load_detection_from_disk() -> Option<CachedDetection> {
    let path = match detection_cache_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "couldn't resolve detection cache path");
            return None;
        }
    };
    if !path.exists() {
        return None;
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<CachedDetection>(&text) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "detection cache malformed; ignoring");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "couldn't read detection cache");
            None
        }
    }
}

/// Write the cache atomically: serialize → tmp file → `fs::rename`.
/// On Unix that's an atomic operation; on Windows it's atomic for files on
/// the same volume, which `state_dir` always satisfies.
fn save_detection_to_disk_atomic(cached: &CachedDetection) -> anyhow::Result<()> {
    let path = detection_cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cached)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Wire shape sent to the frontend per progress tick.
#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub done: usize,
    pub total: usize,
}

/// Wire shape for one tracked save in the Library list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedSave {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub local_path: String,
    pub last_version_num: Option<i64>,
    pub last_backup_at: Option<String>,
    /// `true` if the user has paused tracking for this save. Paused saves
    /// stay in the list but the agent ignores them — useful when the user
    /// is reorganising files or doesn't want chatty backups during a
    /// modding session.
    #[serde(default)]
    pub paused: bool,
    /// Total bytes this save occupies on the server, summed across every
    /// non-deleted snapshot. The server fills this in on `/v1/saves`; we
    /// surface it so the Library page can show "23.4 MB" next to each
    /// game.
    #[serde(default)]
    pub total_size_bytes: i64,
}

/// Run a full auto-detection sweep against the **bundled** catalog (no
/// server round-trips). Emits `library://scan-progress` events
/// (`{ done, total }`) as it churns through the catalog. The completed
/// `DetectionReport` is also stored on the app state so re-renders are free.
#[tauri::command]
pub async fn scan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetectionReport, String> {
    let os = Os::current();

    let app_for_progress = app.clone();
    let progress = move |done: usize, total: usize| {
        // Best-effort: a missing window means the user closed it mid-scan,
        // and we just stop reporting. Any other emit error is noise.
        let _ = app_for_progress.emit("library://scan-progress", ScanProgress { done, total });
    };

    let report = detection::detect_all(os, progress)
        .await
        .map_err(pretty_error)?;

    persist_scan(&state, report.clone());
    Ok(report)
}

/// Forced re-scan that ignores the in-memory cache. Functionally identical
/// to `scan_library` today (both always do a fresh sweep), but kept as a
/// distinct entry point so the UI's "Re-escanear" button maps to an
/// unambiguous intent — and so future caching layers don't accidentally
/// short-circuit the user's manual request.
#[tauri::command]
pub async fn rescan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetectionReport, String> {
    scan_library(app, state).await
}

/// Return the previous scan if one is in memory. Used by the Library page
/// to render quickly on navigation; if `None`, the UI triggers `scan_library`.
#[tauri::command]
pub fn cached_detection(state: State<'_, AppState>) -> Option<DetectionReport> {
    state
        .detection_cache
        .last
        .lock()
        .unwrap()
        .as_ref()
        .map(|c| c.report.clone())
}

/// Update both the in-memory cache and the on-disk copy. Disk failures are
/// logged at WARN — the user still has a working session, we just lose the
/// cold-start optimisation on next launch.
fn persist_scan(state: &State<'_, AppState>, report: DetectionReport) {
    let cached = CachedDetection {
        report,
        scanned_at: OffsetDateTime::now_utc(),
    };
    *state.detection_cache.last.lock().unwrap() = Some(cached.clone());
    if let Err(e) = save_detection_to_disk_atomic(&cached) {
        tracing::warn!(error = %e, "couldn't persist detection cache");
    }
}

/// Args for `add_game_to_tracking`. We keep them in a struct so adding more
/// optional fields later (label override, paths preference) doesn't reshape
/// the Tauri command signature.
#[derive(Debug, Clone, Deserialize)]
pub struct AddGameArgs {
    pub game_slug: String,
    pub label: Option<String>,
    pub local_path: String,
    /// Optional catalog metadata. We pass this to the server so it can
    /// self-heal its games table when the desktop's Ludusavi catalog is
    /// fresher than the server's seed (e.g. self-hosted server still on
    /// v1.0.0 while the desktop has auto-refreshed). Older servers ignore
    /// the extra fields silently.
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub steam_app_id: Option<i64>,
}

/// Begin tracking a detected game. Creates a Save on the server and writes
/// the local path into the agent's state file. Returns the resulting
/// `TrackedSave` so the UI can append it to the list without a re-fetch.
#[tauri::command]
pub async fn add_game_to_tracking(
    args: AddGameArgs,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&state)?;
    let label = args.label.unwrap_or_else(|| "main".to_string());

    let local_path = PathBuf::from(&args.local_path);
    if local_path.as_os_str().is_empty() {
        return Err("Save folder path can't be empty.".to_string());
    }
    // Auto-create when the folder is missing — useful when the user wants
    // to start tracking a save before launching the game (e.g. restoring
    // from another machine first).
    if !local_path.exists() {
        std::fs::create_dir_all(&local_path)
            .map_err(|e| format!("Couldn't create {}: {e}", local_path.display()))?;
    } else if !local_path.is_dir() {
        return Err(format!("{} isn't a folder.", local_path.display()));
    }

    let save = match client
        .create_save_with_meta(
            &args.game_slug,
            &label,
            args.display_name.as_deref(),
            args.steam_app_id,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            // A 409 means there's already a server-side save for this
            // (game_slug, label) pair owned by this user. That happens when
            // the user destracked the save locally — which intentionally
            // leaves the server row in place to preserve snapshots — and
            // then re-tracks the same game. Recover by finding the existing
            // save and re-linking it into local state, so destrack+retrack
            // restores the user's snapshot history instead of dead-ending
            // with an opaque error.
            let is_conflict = e
                .downcast_ref::<ApiError>()
                .map(|api| matches!(api, ApiError::Conflict(_)))
                .unwrap_or(false);
            if !is_conflict {
                return Err(pretty_error(e));
            }
            let existing = client
                .list_saves(Some(&args.game_slug))
                .await
                .map_err(pretty_error)?;
            existing
                .into_iter()
                .find(|s| s.game_slug == args.game_slug && s.label == label)
                .ok_or_else(|| "Couldn't re-link the existing save on the server.".to_string())?
        }
    };

    // Persist the local-path mapping so backup/restore know where to look.
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.saves.insert(
        save.id.clone(),
        SaveState {
            local_path: local_path.clone(),
            game_slug: save.game_slug.clone(),
            label: save.label.clone(),
            last_backup_at: None,
            last_version_num: None,
            paused: false,
        },
    );
    cli_state.save(&path).map_err(|e| e.to_string())?;

    // Attach to the running agent so it starts watching immediately.
    let watched = watched_save_from(
        save.id.clone(),
        save.game_slug.clone(),
        args.game_slug.clone(),
        save.label.clone(),
        local_path.clone(),
    );
    attach_save_if_running(&state, watched).await;

    // Surface whatever the server already knows about this save — for a
    // brand-new one these are zero/None; for a re-linked one (destrack +
    // retrack) they restore the snapshot history into the Library card.
    let last_version_num = save.latest_version_num;
    let total_size_bytes = save.total_size_bytes.unwrap_or(0);
    Ok(TrackedSave {
        save_id: save.id,
        game_slug: save.game_slug,
        label: save.label,
        local_path: local_path.to_string_lossy().into_owned(),
        last_version_num,
        last_backup_at: None,
        paused: false,
        total_size_bytes,
    })
}

/// List the saves Hoard is tracking for the logged-in user. Server-side
/// data is the source of truth for `latest_version_num`; the local path
/// comes from `CliState`.
#[tauri::command]
pub async fn list_tracked_saves(state: State<'_, AppState>) -> Result<Vec<TrackedSave>, String> {
    let client = current_client(&state)?;
    let saves = client.list_saves(None).await.map_err(pretty_error)?;
    let (cli_state, _) = CliState::load_default().map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(saves.len());
    for s in saves {
        // Only surface saves that this machine is actively tracking. Without
        // this filter, destracking a save (which only removes the local
        // CliState row, on purpose, to preserve snapshots on the server) was
        // bouncing back on the next app launch as a ghost "tracked" card —
        // and worse, suppressed the amber "no save folder" alert because the
        // detection card thought the game was already being watched.
        let Some(st) = cli_state.saves.get(&s.id) else {
            continue;
        };
        let local = st.local_path.to_string_lossy().into_owned();
        let paused = st.paused;
        out.push(TrackedSave {
            save_id: s.id,
            game_slug: s.game_slug,
            label: s.label,
            local_path: local,
            last_version_num: s.latest_version_num,
            last_backup_at: format_optional_time(Some(s.updated_at)),
            paused,
            total_size_bytes: s.total_size_bytes.unwrap_or(0),
        });
    }
    Ok(out)
}

/// Rename a save's label both on the server and in local state. On a 409
/// (another save in the same game already uses that label) we surface a
/// distinguishable error string so the UI can show the localized "label
/// already exists" message instead of a generic toast.
#[tauri::command]
pub async fn rename_save_label(
    save_id: String,
    new_label: String,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let trimmed = new_label.trim();
    if trimmed.is_empty() {
        return Err("Label can't be empty.".to_string());
    }
    let client = current_client(&state)?;
    let updated = client
        .rename_save_label(&save_id, trimmed)
        .await
        .map_err(|e| {
            let is_conflict = e
                .downcast_ref::<ApiError>()
                .map(|api| matches!(api, ApiError::Conflict(_)))
                .unwrap_or(false);
            if is_conflict {
                // Tagged so the JS side can match without parsing the
                // server's English error text.
                "conflict:label_collision".to_string()
            } else {
                pretty_error(e)
            }
        })?;

    // Update local state with the new label so subsequent backups land in
    // the right directory and the UI doesn't have to refetch.
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    let local_path_string = if let Some(entry) = cli_state.saves.get_mut(&save_id) {
        entry.label = updated.label.clone();
        entry.local_path.to_string_lossy().into_owned()
    } else {
        String::new()
    };
    cli_state.save(&path).map_err(|e| e.to_string())?;

    // Re-attach to the running agent so its in-memory WatchedSave picks up
    // the new label (the watcher uses label as part of the upload key).
    let local_path = PathBuf::from(&local_path_string);
    if !local_path_string.is_empty() {
        let watched = watched_save_from(
            updated.id.clone(),
            updated.game_slug.clone(),
            updated.game_slug.clone(),
            updated.label.clone(),
            local_path.clone(),
        );
        attach_save_if_running(&state, watched).await;
    }

    Ok(TrackedSave {
        save_id: updated.id,
        game_slug: updated.game_slug,
        label: updated.label,
        local_path: local_path_string,
        last_version_num: updated.latest_version_num,
        last_backup_at: None,
        paused: false,
        total_size_bytes: updated.total_size_bytes.unwrap_or(0),
    })
}

/// Stop tracking a save. Removes the local-state row but leaves server data
/// intact (delete from the History view if you want that gone too).
#[tauri::command]
pub async fn untrack_save(save_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.saves.remove(&save_id);
    cli_state.save(&path).map_err(|e| e.to_string())?;
    detach_save_if_running(&state, save_id).await;
    Ok(())
}

// ---- helpers ----------------------------------------------------------

/// Build an `ApiClient` from the credentials on disk. We don't reuse a
/// long-lived client on `AppState` because the user can log out at any time
/// and we want fresh creds per command — the cost is negligible (`reqwest`
/// connections are pooled internally).
pub(crate) fn current_client(state: &State<'_, AppState>) -> Result<ApiClient, String> {
    // The cached UserInfo gives us the URL; the keychain holds the token.
    // If either is missing we surface a single uniform error so the frontend
    // can route the user back to the login screen.
    let user = state
        .user
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Not logged in. Sign in again to continue.".to_string())?;
    let creds = credentials::load()
        .map_err(|e| format!("Couldn't load credentials: {e}"))?
        .ok_or_else(|| "Saved credentials are missing. Sign in again.".to_string())?;
    ApiClient::new(user.server_url, creds.token).map_err(|e| e.to_string())
}

fn format_optional_time(t: Option<OffsetDateTime>) -> Option<String> {
    use time::format_description::well_known::Rfc3339;
    t.and_then(|x| x.format(&Rfc3339).ok())
}

/// Spawn a long-lived background task that re-scans the catalog whenever the
/// persisted cache turns 24 hours old. Wakes every 30 minutes; cheap on a
/// schedule clock that's "behind" because we just check timestamps. Errors
/// are logged and swallowed — a transient detection failure must not crash
/// the app loop.
pub fn spawn_periodic_rescan(app: AppHandle) {
    tokio::spawn(async move {
        use std::time::Duration;
        loop {
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
            let needs_rescan = {
                let state = app.state::<AppState>();
                let guard = state.detection_cache.last.lock().unwrap();
                match guard.as_ref() {
                    Some(c) => {
                        // Use unix timestamps so we don't depend on time
                        // crate's Duration arithmetic — it's just a subtraction.
                        let age_secs = OffsetDateTime::now_utc().unix_timestamp()
                            - c.scanned_at.unix_timestamp();
                        age_secs >= STALE_AFTER_SECS
                    }
                    // Nothing cached yet — leave it for the user's first
                    // explicit scan rather than spinning up detection
                    // silently on a fresh install.
                    None => false,
                }
            };
            if !needs_rescan {
                continue;
            }
            tracing::info!("detection cache older than 24h, refreshing in background");
            let os = Os::current();
            // No progress emit on the background path — the UI isn't
            // listening, and repainting a progress bar while the user is
            // on another page would be noise.
            let report = match detection::detect_all(os, |_, _| {}).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "background detection refresh failed");
                    continue;
                }
            };
            let cached = CachedDetection {
                report,
                scanned_at: OffsetDateTime::now_utc(),
            };
            {
                let state = app.state::<AppState>();
                *state.detection_cache.last.lock().unwrap() = Some(cached.clone());
            }
            if let Err(e) = save_detection_to_disk_atomic(&cached) {
                tracing::warn!(error = %e, "couldn't persist refreshed detection cache");
            }
        }
    });
}