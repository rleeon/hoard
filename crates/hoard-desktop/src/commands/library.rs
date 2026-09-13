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
use std::time::{Duration, SystemTime};

use hoard_agent::api::{ApiClient, ApiError};
use hoard_agent::detection::{self, DetectedGame, DetectionReport, DetectionTrace};
use hoard_agent::library;
use hoard_agent::manifest::Os;
use hoard_agent::state::CliState;
use hoard_core::ipc::{Payload, Request};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use super::agent::{attach_save_if_running, detach_save_if_running};
use super::auth::pretty_error;
use super::error::AppError;
use crate::state::AppState;

// Persisted detection snapshot + its disk I/O live in `hoard_agent::library`
// (the CLI reuses the same cache). Re-exported so the rest of this module and
// `AppState` keep referring to them under `commands::library::…`.
pub use hoard_agent::library::{
    load_detection_from_disk, save_detection_to_disk_atomic, CachedDetection,
};

/// The most recent scan, so the Library page re-renders instantly when the user
/// navigates back without another sweep.
///
/// `detection.json` is the truth: the service rewrites it on its own schedule
/// (automatic mode, the daily refresh), often with no window listening, so a read
/// checks the file's stamp and re-reads it when it moved. Nothing is read until
/// something asks, so an app that never opens the Library never holds the report.
#[derive(Default)]
pub struct DetectionCache {
    last: Mutex<Option<CachedDetection>>,
    stamp: Mutex<Option<SystemTime>>,
}

impl DetectionCache {
    pub fn current(&self) -> Option<CachedDetection> {
        let stamp = library::detection_cache_path()
            .ok()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok());
        let mut seen = self.stamp.lock().unwrap();
        if stamp.is_some() && stamp != *seen {
            *self.last.lock().unwrap() = load_detection_from_disk();
            *seen = stamp;
        }
        self.last.lock().unwrap().clone()
    }

    fn set(&self, cached: CachedDetection) {
        *self.last.lock().unwrap() = Some(cached);
    }
}

/// How long a sweep in the service may take. A deep one walks every Wine prefix
/// and mounted drive: minutes on a big disk are normal, an hour is not.
const SCAN_LIMIT: Duration = Duration::from_secs(30 * 60);

/// One folder, whose walk has its own timeout: this is only the backstop.
const FOLDER_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Maximum age before the background scheduler triggers an automatic
/// rescan. 24 hours mirrors the catalog-refresh cadence so the two stay
/// loosely in step.
const STALE_AFTER_SECS: i64 = 60 * 60 * 24;
/// How often the background scheduler wakes to check the cache age.
/// 30 minutes is a good middle ground: long enough to be near-free,
/// short enough that the user doesn't have to wait a full sleep period
/// after the OS clock jumps forward.
const POLL_INTERVAL_SECS: u64 = 60 * 30;

/// Wire shape sent to the frontend per progress tick.
#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub done: usize,
    pub total: usize,
}

// Wire types for tracking live in `hoard_agent::library` so the CLI shares
// them. Re-exported here so the Tauri commands and `automatic.rs` keep the
// `commands::library::…` paths.
pub use hoard_agent::library::{AddGameArgs, AdoptArgs, TrackedSave};

/// Run a full auto-detection sweep (no server round-trips). Emits
/// `library://scan-progress` events (`{ done, total }`) as it churns through the
/// catalog. The completed `DetectionReport` is cached so re-renders are free.
#[tauri::command]
pub async fn scan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetectionReport, String> {
    scan(&app, &state, false).await
}

/// Forced re-scan that ignores the in-memory cache. Functionally identical
/// to `scan_library` today (both always do a fresh sweep), but kept as a
/// distinct entry point so the UI's rescan button maps to an unambiguous
/// intent, and so future caching layers don't accidentally short-circuit the
/// user's manual request.
#[tauri::command]
pub async fn rescan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetectionReport, String> {
    scan_library(app, state).await
}

/// The deep, user-triggered detection sweep behind the Library's deep-scan tile. It
/// runs [`detection::detect_all_deep`], which on top of the normal pipeline looks
/// at the expensive places the periodic scan skips: arbitrary Wine prefixes
/// (Heroic/CrossOver/Flatpak/mounted media), Flatpak/Snap/EmuDeck save roots,
/// deeper directory walks and a relaxed precision gate. Slow by design, so
/// it's never on the automatic tick. Emits the same `library://scan-progress`
/// events and persists the merged report like a normal scan.
#[tauri::command]
pub async fn deep_scan_library(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetectionReport, String> {
    scan(&app, &state, true).await
}

/// One sweep, blacklist and discarded folders already applied (see
/// [`library::detect_filtered`]).
///
/// In the service when it runs detection (ADR 0021, Slice 8): this window then
/// never loads the catalogue, and the progress arrives as the service's notes on
/// the same `library://scan-progress` channel. An older service leaves it to this
/// process, as before.
async fn scan(app: &AppHandle, state: &AppState, deep: bool) -> Result<DetectionReport, String> {
    if state.daemon.owns_detection().await {
        let payload = state
            .daemon
            .request_long(Request::Scan { deep }, SCAN_LIMIT)
            .await
            .map_err(pretty_error)?;
        return match payload {
            // The service wrote the cache on disk already; `current` reads it.
            Payload::Detection { report } => {
                serde_json::from_value(report).map_err(|e| e.to_string())
            }
            other => Err(format!("unexpected answer to a scan: {other:?}")),
        };
    }
    let progress = {
        let app = app.clone();
        // Best-effort: a missing window means the user closed it mid-scan,
        // and we just stop reporting. Any other emit error is noise.
        move |done: usize, total: usize| {
            let _ = app.emit("library://scan-progress", ScanProgress { done, total });
        }
    };
    let report = library::detect_filtered(deep, progress)
        .await
        .map_err(pretty_error)?;
    persist_scan(state, report.clone());
    Ok(report)
}

/// "Add from folder": the games inside ONE folder the user chose (see
/// [`library::scan_folder`]). In the service when it runs detection, since naming
/// what is in the folder reads the catalogue.
#[tauri::command]
pub async fn scan_folder(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<DetectedGame>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("{} isn't a folder.", root.display()));
    }
    if state.daemon.owns_detection().await {
        let payload = state
            .daemon
            .request_long(Request::ScanFolder { path }, FOLDER_LIMIT)
            .await
            .map_err(pretty_error)?;
        return match payload {
            Payload::Detected { games } => serde_json::from_value(games).map_err(|e| e.to_string()),
            other => Err(format!("unexpected answer to a folder scan: {other:?}")),
        };
    }
    // The walk is synchronous filesystem I/O: off the async runtime, so a slow or
    // large folder doesn't stall the UI's event loop.
    tokio::task::spawn_blocking(move || library::scan_folder(&root))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Return the previous scan if there is one. Used by the Library page to render
/// quickly on navigation; if `None`, the UI triggers `scan_library`.
#[tauri::command]
pub fn cached_detection(state: State<'_, AppState>) -> Option<DetectionReport> {
    state.detection_cache.current().map(|c| c.report)
}

/// What local detection already knows about one slug, so the "link to this machine"
/// dialog can offer the detected folders as one-click options instead of sending the
/// user hunting through a folder picker.
///
/// A `scanned_at: None` result means nobody ever scanned here, and the UI offers a
/// scan rather than claiming there's nothing.
///
/// `tracked_paths` (the folders this machine already tracks) comes from the caller:
/// the Library already has that list on screen, so opening the dialog does not cost
/// one request to the server.
#[tauri::command]
pub fn detected_paths_for_game(
    game_slug: String,
    tracked_paths: Vec<String>,
    state: State<'_, AppState>,
) -> library::LocalDetection {
    let tracked: Vec<std::path::PathBuf> = tracked_paths
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    let cached = state.detection_cache.current();
    library::local_detection(cached.as_ref(), &game_slug, &tracked)
}

/// Update both the in-memory cache and the on-disk copy. Disk failures are
/// logged at WARN: the user still has a working session, we just lose the
/// cold-start optimisation on next launch.
fn persist_scan(state: &AppState, report: DetectionReport) {
    let cached = CachedDetection {
        report,
        scanned_at: OffsetDateTime::now_utc(),
    };
    state.detection_cache.set(cached.clone());
    if let Err(e) = save_detection_to_disk_atomic(&cached) {
        tracing::warn!(error = %e, "couldn't persist detection cache");
    }
}

/// Begin tracking a detected game. Creates a Save on the server and writes
/// the local path into the agent's state file. Returns the resulting
/// `TrackedSave` so the UI can append it to the list without a re-fetch. All
/// the business logic (cloud vs self-hosted, dedup, 409 re-link) lives in
/// `hoard_agent::library`; this wrapper just tells the sync service and
/// prettifies errors.
#[tauri::command]
pub async fn add_game_to_tracking(
    app: AppHandle,
    args: AddGameArgs,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&app, &state).await?;
    let outcome = library::add_to_tracking(&client, args)
        .await
        .map_err(pretty_error)?;
    // Attach to the running agent so it starts watching immediately.
    attach_save_if_running(&state, outcome.watched).await;
    Ok(outcome.tracked)
}

/// Adopt (vincular) a cloud save that belongs to another machine: associate a
/// local folder on THIS machine with the existing `save_id` instead of minting
/// a new save. In sync mode the agent auto-restores the latest snapshot on add
/// (the version-gate is left open); in backup-only mode it just watches the
/// folder. Core of cross-device sync. Logic in `hoard_agent::library::adopt`.
#[tauri::command]
pub async fn adopt_save(
    app: AppHandle,
    args: AdoptArgs,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&app, &state).await?;
    let outcome = library::adopt(&client, args).await.map_err(pretty_error)?;
    attach_save_if_running(&state, outcome.watched).await;
    Ok(outcome.tracked)
}

/// List the saves Hoard is tracking for the logged-in user. Server-side data is
/// the source of truth for `latest_version_num`; the local path comes from
/// `CliState`. Logic (dedup self-heal, orphan detection, local sizes) lives in
/// `hoard_agent::library::list_tracked`; this wrapper detaches any duplicate
/// rows the self-heal pruned from the watched set.
#[tauri::command]
pub async fn list_tracked_saves(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<TrackedSave>, String> {
    let client = current_client(&app, &state).await?;
    let (out, detached) = library::list_tracked(&client).await.map_err(pretty_error)?;
    for id in detached {
        detach_save_if_running(&state, id).await;
    }
    Ok(out)
}

/// Name a folder without touching its number. The number is what pairs it with
/// the same folder on the other machines, so the UI never edits the label whole;
/// see `hoard_core::kernel::slots`.
#[tauri::command]
pub async fn set_save_slot_name(
    app: AppHandle,
    save_id: String,
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&app, &state).await?;
    let (tracked, watched) = library::set_slot_name(&client, &save_id, name.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    reseat_renamed(&state, watched).await;
    Ok(tracked)
}

/// Move a folder to another number. A number the cloud already holds comes back
/// as `slot_taken:<n>` so the UI can offer linking to that row instead, which is
/// what actually pairs the two machines.
#[tauri::command]
pub async fn renumber_save_slot(
    app: AppHandle,
    save_id: String,
    slot: u32,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&app, &state).await?;
    let (tracked, watched) = library::renumber(&client, &save_id, slot)
        .await
        .map_err(|e| e.to_string())?;
    reseat_renamed(&state, watched).await;
    Ok(tracked)
}

/// Re-attach to the running agent so its in-memory `WatchedSave` picks up the
/// new label: the watcher uses it as part of the upload key, so a stale one
/// uploads under the old name and forks the row.
async fn reseat_renamed(
    state: &State<'_, AppState>,
    watched: Option<hoard_agent::agent::WatchedSave>,
) {
    if let Some(watched) = watched {
        attach_save_if_running(state, watched).await;
    }
}

/// Rename a save's label whole. Only reachable for the free-form labels that
/// predate slots; anything with a number goes through `set_save_slot_name` or
/// `renumber_save_slot`, which keep the two halves apart. On a 409 (another save
/// in the same game already uses that label) we surface a distinguishable error
/// string so the UI can show the localized "label already exists" message
/// instead of a generic toast.
#[tauri::command]
pub async fn rename_save_label(
    app: AppHandle,
    save_id: String,
    new_label: String,
    state: State<'_, AppState>,
) -> Result<TrackedSave, String> {
    let client = current_client(&app, &state).await?;
    let (tracked, watched) = library::rename_label(&client, &save_id, &new_label)
        .await
        .map_err(|e| {
            // A 409 (another save in the same game already uses that label)
            // gets tagged so the JS side can match it without parsing the
            // server's English error text.
            let is_conflict = e
                .downcast_ref::<ApiError>()
                .map(|api| matches!(api, ApiError::Conflict(_)))
                .unwrap_or(false);
            if is_conflict {
                "conflict:label_collision".to_string()
            } else {
                pretty_error(e)
            }
        })?;

    // Re-attach to the running agent so its in-memory WatchedSave picks up the
    // new label (the watcher uses label as part of the upload key).
    if let Some(watched) = watched {
        attach_save_if_running(&state, watched).await;
    }
    Ok(tracked)
}

/// Validate a user-picked override path: non-empty, exists, is a directory.
/// Pulled out of the Tauri command so the failure messages can be unit-tested
/// without spinning up Tauri's `State` machinery.
fn validate_override_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path can't be empty.".to_string());
    }
    let buf = PathBuf::from(trimmed);
    if !buf.exists() {
        return Err(format!("{} doesn't exist.", buf.display()));
    }
    if !buf.is_dir() {
        return Err(format!("{} isn't a folder.", buf.display()));
    }
    Ok(buf)
}

/// Record a manual save-folder override for `slug`. The path must already
/// exist and be a directory: we validate up-front so an obvious typo in
/// the picker doesn't get persisted and silently fail the next re-scan.
///
/// After the override lands in `state.json`, we kick a background re-scan
/// so the Library card flips to source=manual without the user clicking
/// "Rescan". Disk failures inside the background scan are logged; the
/// command itself returns as soon as the override is durable.
#[tauri::command]
pub async fn set_manual_path(
    app: AppHandle,
    state: State<'_, AppState>,
    slug: String,
    path: String,
) -> Result<(), String> {
    let path_buf = validate_override_path(&path)?;

    let (mut cli_state, state_path) = CliState::load_default().map_err(|e| e.to_string())?;
    // Pointing one game at ANOTHER's folder cannot undo itself: the override lives
    // in `device.json` and survives everything the user knows how to delete. It is
    // rejected here, naming the game that already claims it.
    let cached = state.detection_cache.current();
    if let Some(owner) = hoard_agent::library::manual_override_conflict(
        &cli_state,
        cached.as_ref().map(|c| &c.report),
        &slug,
        &path_buf,
    ) {
        return Err(format!(
            "That folder is '{owner}'s, not {slug}'s: one folder, one game. Pick the folder this game writes to."
        ));
    }
    cli_state.set_manual_path(&slug, path_buf);
    cli_state.save(&state_path).map_err(|e| e.to_string())?;

    // Refresh the detection cache so subsequent renders see source=manual
    // without forcing the user to click "Rescan". A short-circuit failure
    // here is harmless: the next scheduled rescan will pick up the
    // override either way.
    if let Err(e) = scan(&app, &state, false).await {
        tracing::warn!(error = %e, "post-override detection refresh failed");
    }
    Ok(())
}

/// Drop the manual override for `slug` so the next re-scan goes back to
/// whatever the heuristics produce. Mirrors `set_manual_path` and also
/// refreshes the detection cache in-line so the UI re-paints without an
/// explicit rescan click.
#[tauri::command]
pub async fn clear_manual_path(
    app: AppHandle,
    state: State<'_, AppState>,
    slug: String,
) -> Result<(), String> {
    let (mut cli_state, state_path) = CliState::load_default().map_err(|e| e.to_string())?;
    cli_state.clear_manual_path(&slug);
    cli_state.save(&state_path).map_err(|e| e.to_string())?;

    if let Err(e) = scan(&app, &state, false).await {
        tracing::warn!(error = %e, "post-clear detection refresh failed");
    }
    Ok(())
}

/// Persistently blacklist a game from the Library page: future scans filter
/// the slug out **and** any save tracked under it stops being watched here
/// (server data untouched; see [`hoard_agent::library::ignore_slug`]).
/// Reversible via [`unignore_detected_game`]. Idempotent.
///
/// Returns how many tracked saves it dropped, so the UI can say so.
#[tauri::command]
pub async fn ignore_detected_game(
    slug: String,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let untracked = library::ignore_slug(&slug).map_err(|e| AppError::plain(pretty_error(e)))?;
    for save_id in &untracked {
        detach_save_if_running(&state, save_id.clone()).await;
    }
    Ok(untracked.len())
}

/// Drops a folder from the scan. A thin wrapper over the agent: the logic (and the
/// state) live there, as the CLI/desktop parity rule requires.
#[tauri::command]
pub async fn exclude_scan_path(path: String) -> Result<(), AppError> {
    hoard_agent::library::exclude_path(std::path::Path::new(path.trim()))
        .map_err(|e| AppError::plain(e.to_string()))
}

/// Deshace [`exclude_scan_path`].
#[tauri::command]
pub async fn unexclude_scan_path(path: String) -> Result<(), AppError> {
    hoard_agent::library::unexclude_path(std::path::Path::new(path.trim()))
        .map_err(|e| AppError::plain(e.to_string()))
}

/// Las carpetas descartadas, para la lista de Ajustes.
#[tauri::command]
pub async fn list_excluded_scan_paths() -> Result<Vec<String>, AppError> {
    hoard_agent::library::list_excluded_paths()
        .map(|v| v.iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .map_err(|e| AppError::plain(e.to_string()))
}

/// Drop the blacklist entry for `slug` so the next scan re-surfaces it in
/// the Library. Mirror of [`ignore_detected_game`]. Idempotent.
#[tauri::command]
pub async fn unignore_detected_game(slug: String) -> Result<(), AppError> {
    library::unignore_slug(&slug).map_err(|e| AppError::plain(pretty_error(e)))
}

/// Return every slug the user has blacklisted. Sorted alphabetically so the
/// Settings page renders a stable order across refreshes.
#[tauri::command]
pub async fn list_ignored_slugs() -> Result<Vec<String>, AppError> {
    let (cli_state, _) = CliState::load_default().map_err(|e| AppError::plain(e.to_string()))?;
    let mut slugs: Vec<String> = cli_state.ignored_slugs.iter().cloned().collect();
    slugs.sort();
    Ok(slugs)
}

/// Replay the detection pipeline for a single slug and return a trace
/// explaining what every step kept and dropped. Read-only: it does not write
/// to the detection cache or `state.json`. Backs the hidden
/// `/diagnostics` route unlocked by the 5-click sidebar gesture.
#[tauri::command]
pub async fn detection_diagnostics(
    slug: String,
    state: State<'_, AppState>,
) -> Result<DetectionTrace, String> {
    let slug = slug.trim().to_string();
    if slug.is_empty() {
        return Err("Slug is empty.".into());
    }
    if state.daemon.owns_detection().await {
        let payload = state
            .daemon
            .request_long(Request::DiagnoseDetection { slug }, SCAN_LIMIT)
            .await
            .map_err(pretty_error)?;
        return match payload {
            Payload::DetectionTrace { trace } => {
                serde_json::from_value(trace).map_err(|e| e.to_string())
            }
            other => Err(format!("unexpected answer to a diagnosis: {other:?}")),
        };
    }
    let (cli_state, _path) = CliState::load_default().map_err(|e| e.to_string())?;
    Ok(detection::diagnose(&slug, Os::current(), &cli_state).await)
}

/// Stop tracking a save. Removes the local-state row but leaves server data
/// intact (delete from the History view if you want that gone too).
#[tauri::command]
pub async fn untrack_save(save_id: String, state: State<'_, AppState>) -> Result<(), String> {
    library::untrack(&save_id).map_err(|e| e.to_string())?;
    detach_save_if_running(&state, save_id).await;
    Ok(())
}

/// Hard-delete a save: drop the row + every snapshot on the server **and**
/// purge any local CliState that referenced it. Sibling of `untrack_save`,
/// but destructive on purpose: it's what the user clicks when a save was
/// tracked against a wrong path and the only way out is to start over
/// (otherwise `add_game_to_tracking` swallows the next 409 and re-links to
/// the bad row). The matching `manual_paths` override is also cleared so a
/// re-add doesn't immediately bounce back to the same wrong folder.
#[tauri::command]
pub async fn delete_save_completely(
    app: AppHandle,
    save_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = current_client(&app, &state).await?;
    library::delete_completely(&client, &save_id)
        .await
        .map_err(pretty_error)?;
    detach_save_if_running(&state, save_id).await;
    Ok(())
}

// ---- helpers ----------------------------------------------------------

/// Build an `ApiClient` from the credentials on disk. We don't reuse a
/// long-lived client on `AppState` because the user can log out at any time
/// and we want fresh creds per command; the cost is negligible (`reqwest`
/// connections are pooled internally).
pub(crate) async fn current_client(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<ApiClient, String> {
    // Prefer the self-hosted session: the cached UserInfo gives us the URL, and the
    // service lends the token (D.20: the keyring item is its own, and reading it from
    // here is what asked for the password on macOS).
    let self_hosted = state.user.lock().unwrap().clone();
    if let Some(user) = self_hosted {
        let creds = crate::commands::auth::server_session(app)
            .await
            .map_err(|e| format!("{e:#}"))?
            .ok_or_else(|| "Saved credentials are missing. Sign in again.".to_string())?;
        return ApiClient::new(user.server_url, creds.token).map_err(|e| e.to_string());
    }
    // Fall back to a Hoard Cloud session (Gmail login). The cloud API exposes
    // the same `/v1/...` surface and accepts the Supabase JWT as a bearer
    // token, so the agent and every library/history command can talk to it
    // exactly like a self-hosted server. Without this branch a cloud-only user
    // hit "Not logged in" on every monitor/backup action even though the
    // sidebar showed the cloud as connected.
    //
    // The service lends the JWT (D.20): this process does not read the keyring, so it
    // cannot hold on to a stale token or trigger an authorisation dialog on macOS.
    // And it always comes fresh, which is what the note below asked for.
    if let Some(cloud) = crate::commands::cloud::active_creds_via(&state.daemon)
        .await
        .map_err(|e| format!("Couldn't get cloud credentials: {e}"))?
    {
        return ApiClient::new(cloud.server_url, cloud.access_token).map_err(|e| e.to_string());
    }
    Err("Not logged in. Sign in again to continue.".to_string())
}

/// Recompute and install the active sync context from the current session,
/// mirroring [`current_client`]'s self-hosted-wins-else-cloud selection. Call
/// after any login/logout (and once at boot) so `CliState` reads and writes the
/// per-context `saves` file that belongs to the session actually in use.
/// Returns the installed context id, or `None` when fully signed out.
pub(crate) fn sync_active_context(state: &AppState) -> Option<String> {
    let ctx = if let Some(user) = state.user.lock().unwrap().clone() {
        Some(hoard_agent::state::selfhosted_context(&user.server_url))
    } else if let Ok(Some(user_id)) = crate::commands::cloud::active_user_id() {
        Some(hoard_agent::state::cloud_context(&user_id))
    } else {
        None
    };
    hoard_agent::state::set_active_context(ctx.clone());
    ctx
}

/// Spawn a long-lived background task that re-scans the catalog whenever the
/// persisted cache turns 24 hours old, for an older service: the current one keeps
/// the cache fresh itself, window or not. Wakes every 30 minutes; cheap on a
/// schedule clock that's "behind" because we just check timestamps. Errors
/// are logged and swallowed: a transient detection failure must not crash
/// the app loop.
pub fn spawn_periodic_rescan(app: AppHandle) {
    // `tauri::async_runtime::spawn`, not `tokio::spawn`: this is called from
    // `setup()`, which runs before Tauri enters its event loop, so there is
    // no ambient Tokio runtime yet. Using `tokio::spawn` here panics with
    // "there is no reactor running" the instant the app starts, which is how
    // 1.4.0 shipped, which is why the binary refused to launch on every
    // platform after the upgrade until the user reopened it from a terminal
    // and saw the stack trace.
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            let state = app.state::<AppState>();
            if state.daemon.owns_detection().await {
                continue;
            }
            let needs_rescan = match state.detection_cache.current() {
                Some(c) => {
                    OffsetDateTime::now_utc().unix_timestamp() - c.scanned_at.unix_timestamp()
                        >= STALE_AFTER_SECS
                }
                // Nothing cached yet: leave it for the user's first
                // explicit scan rather than spinning up detection
                // silently on a fresh install.
                None => false,
            };
            if !needs_rescan {
                continue;
            }
            tracing::info!("detection cache older than 24h, refreshing in background");
            // No progress emit on the background path: the UI isn't
            // listening, and repainting a progress bar while the user is
            // on another page would be noise.
            match library::detect_filtered(false, |_, _| {}).await {
                Ok(report) => persist_scan(&state, report),
                Err(e) => tracing::warn!(error = %e, "background detection refresh failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_override_path_rejects_empty() {
        let err = validate_override_path("   ").unwrap_err();
        assert!(err.contains("empty"), "expected legible error, got {err:?}");
    }

    #[test]
    fn validate_override_path_rejects_missing() {
        let err = validate_override_path("/definitely/not/a/real/path/zzz").unwrap_err();
        assert!(
            err.contains("doesn't exist"),
            "expected legible error, got {err:?}"
        );
    }

    #[test]
    fn validate_override_path_rejects_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let err = validate_override_path(tmp.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.contains("isn't a folder"),
            "expected legible error, got {err:?}"
        );
    }

    #[test]
    fn validate_override_path_accepts_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let buf = validate_override_path(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(buf, tmp.path());
    }
}
