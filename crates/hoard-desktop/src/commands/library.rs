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
use hoard_agent::credentials;
use hoard_agent::detection::{self, DetectionReport};
use hoard_agent::manifest::Os;
use hoard_agent::state::{CliState, SaveState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use time::OffsetDateTime;

use super::agent::{attach_save_if_running, detach_save_if_running, watched_save_from};
use super::auth::pretty_error;
use crate::state::AppState;

/// In-memory cache of the most recent scan, so the Library page can
/// re-render instantly when the user navigates back without forcing
/// another sweep. Wrapped in a `Mutex<Option<…>>` and stored on
/// `AppState`.
#[derive(Default)]
pub struct DetectionCache {
    pub last: Mutex<Option<DetectionReport>>,
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

    // Cache the latest report for later renders.
    *state.detection_cache.last.lock().unwrap() = Some(report.clone());
    Ok(report)
}

/// Return the previous scan if one is in memory. Used by the Library page
/// to render quickly on navigation; if `None`, the UI triggers `scan_library`.
#[tauri::command]
pub fn cached_detection(state: State<'_, AppState>) -> Option<DetectionReport> {
    state.detection_cache.last.lock().unwrap().clone()
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
