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
use hoard_agent::detection::{self, DetectionReport, DetectionTrace};
use hoard_agent::manifest::Os;
use hoard_agent::state::{CliState, SaveState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use super::agent::{attach_save_if_running, detach_save_if_running, watched_save_from};
use super::auth::pretty_error;
use super::error::AppError;
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
    /// `true` when the save exists server-side but this machine has no
    /// matching `CliState` row — i.e. the user reinstalled, switched PCs,
    /// or cleared local state without deleting the server data. The UI
    /// shows these with a discreet "Sin estado local" badge and disables
    /// the untrack (local-only) button: there's nothing local to clean,
    /// so the only sensible action is the full `delete_save_completely`.
    #[serde(default)]
    pub orphan: bool,
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

    // Load CliState so detect_all sees the user's manual_paths overrides.
    // Failures here mean state.json is unreadable; we surface that instead
    // of silently scanning without overrides, otherwise the user sees their
    // manual picks vanish from the Library card on every re-scan.
    let (cli_state, _) = CliState::load_default().map_err(|e| e.to_string())?;

    let mut report = detection::detect_all(os, &cli_state, progress)
        .await
        .map_err(pretty_error)?;

    // Drop user-blacklisted slugs *after* detection finishes so the walker
    // still benefits from their install dirs when cross-referencing other
    // games on the same volume. The filter is purely a UI-edge concern.
    report.games.retain(|g| !cli_state.is_ignored(&g.slug));

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

    // Cloud has no server-side `create_save`: the row is materialized on the
    // first upload via UPSERT on (user_id, game_slug, label). The client just
    // mints a local save_id, records the path, and starts watching.
    if client.is_cloud().await {
        let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
        // Dedup by (game_slug, label): unlike the self-hosted path — where a
        // second create hits a 409 and re-links to the one server row — the
        // cloud path used to mint a fresh uuid every call, so adding the same
        // game twice (re-track, re-run onboarding, a detection re-add) left
        // two local rows for one partida and the Library listed it twice.
        // Reuse the existing id, just refreshing the local path.
        let save_id = cli_state
            .saves
            .iter()
            .find(|(_, st)| st.game_slug == args.game_slug && st.label == label)
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        cli_state.saves.insert(
            save_id.clone(),
            SaveState {
                local_path: local_path.clone(),
                game_slug: args.game_slug.clone(),
                label: label.clone(),
                last_backup_at: None,
                last_version_num: None,
                paused: false,
                set_hash: None,
            },
        );
        cli_state.save(&path).map_err(|e| e.to_string())?;

        let watched = watched_save_from(
            save_id.clone(),
            args.game_slug.clone(),
            args.game_slug.clone(),
            label.clone(),
            local_path.clone(),
        );
        attach_save_if_running(&state, watched).await;

        return Ok(TrackedSave {
            save_id,
            game_slug: args.game_slug,
            label,
            local_path: local_path.to_string_lossy().into_owned(),
            last_version_num: None,
            last_backup_at: None,
            paused: false,
            total_size_bytes: 0,
            orphan: false,
        });
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
            set_hash: None,
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
        orphan: false,
    })
}

/// List the saves Hoard is tracking for the logged-in user. Server-side
/// data is the source of truth for `latest_version_num`; the local path
/// comes from `CliState`.
#[tauri::command]
pub async fn list_tracked_saves(state: State<'_, AppState>) -> Result<Vec<TrackedSave>, String> {
    let client = current_client(&state)?;

    // Cloud has no per-user save listing that includes never-uploaded saves:
    // the manifest only carries saves with at least one committed version.
    // CliState is the tracked-set source of truth here; the manifest just
    // enriches latest_version_num / size for saves that have been uploaded.
    if client.is_cloud().await {
        let manifest = client.cloud_sync().await.map_err(pretty_error)?;
        let (cli_state, _) = CliState::load_default().map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(cli_state.saves.len());
        for (id, st) in &cli_state.saves {
            let entry = manifest.saves.iter().find(|e| &e.save_id == id);
            out.push(TrackedSave {
                save_id: id.clone(),
                game_slug: st.game_slug.clone(),
                label: st.label.clone(),
                local_path: st.local_path.to_string_lossy().into_owned(),
                last_version_num: entry.map(|e| e.latest_version_num),
                last_backup_at: None,
                paused: st.paused,
                total_size_bytes: entry.map(|e| e.latest_size_bytes).unwrap_or(0),
                orphan: false,
            });
        }
        return Ok(out);
    }

    let saves = client.list_saves(None).await.map_err(pretty_error)?;
    let (cli_state, _) = CliState::load_default().map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(saves.len());
    for s in saves {
        // Saves with a matching CliState row are the happy path: this
        // machine is actively tracking them, so we surface the local path
        // and pause flag straight from state.
        //
        // Saves *without* a CliState row used to be skipped — that hid
        // server-side rows after a reinstall, a PC switch, or a manual
        // CliState wipe, leaving the user no way to clean them up from the
        // UI (only `hoard-admin` worked). Since 1.5.1 we emit them with
        // `orphan: true` so the Library strip can show them with a "Sin
        // estado local" badge and offer the destructive
        // `delete_save_completely` as the only sensible action.
        match cli_state.saves.get(&s.id) {
            Some(st) => {
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
                    orphan: false,
                });
            }
            None => {
                out.push(TrackedSave {
                    save_id: s.id,
                    game_slug: s.game_slug,
                    label: s.label,
                    local_path: String::new(),
                    last_version_num: s.latest_version_num,
                    last_backup_at: format_optional_time(Some(s.updated_at)),
                    paused: false,
                    total_size_bytes: s.total_size_bytes.unwrap_or(0),
                    orphan: true,
                });
            }
        }
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
        orphan: false,
    })
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
/// exist and be a directory — we validate up-front so an obvious typo in
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
    cli_state.set_manual_path(&slug, path_buf);
    cli_state.save(&state_path).map_err(|e| e.to_string())?;

    // Refresh the detection cache so subsequent renders see source=manual
    // without forcing the user to click "Rescan". A short-circuit failure
    // here is harmless — the next scheduled rescan will pick up the
    // override either way.
    let app_for_progress = app.clone();
    let progress = move |done: usize, total: usize| {
        let _ = app_for_progress.emit("library://scan-progress", ScanProgress { done, total });
    };
    match detection::detect_all(Os::current(), &cli_state, progress).await {
        Ok(mut report) => {
            report.games.retain(|g| !cli_state.is_ignored(&g.slug));
            persist_scan(&state, report);
        }
        Err(e) => tracing::warn!(error = %e, "post-override detection refresh failed"),
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

    let app_for_progress = app.clone();
    let progress = move |done: usize, total: usize| {
        let _ = app_for_progress.emit("library://scan-progress", ScanProgress { done, total });
    };
    match detection::detect_all(Os::current(), &cli_state, progress).await {
        Ok(mut report) => {
            report.games.retain(|g| !cli_state.is_ignored(&g.slug));
            persist_scan(&state, report);
        }
        Err(e) => tracing::warn!(error = %e, "post-clear detection refresh failed"),
    }
    Ok(())
}

/// Persistently blacklist a detected game from the Library page. Subsequent
/// scans run the full detection pipeline but the matching slug is filtered
/// out before the report reaches the UI. Reversible via
/// [`unignore_detected_game`]. Idempotent.
#[tauri::command]
pub async fn ignore_detected_game(slug: String) -> Result<(), AppError> {
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        return Err(AppError::plain("Slug is empty."));
    }
    let (mut cli_state, path) =
        CliState::load_default().map_err(|e| AppError::plain(e.to_string()))?;
    cli_state.add_ignored_slug(trimmed.to_string());
    cli_state
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;
    Ok(())
}

/// Drop the blacklist entry for `slug` so the next scan re-surfaces it in
/// the Library. Mirror of [`ignore_detected_game`]. Idempotent.
#[tauri::command]
pub async fn unignore_detected_game(slug: String) -> Result<(), AppError> {
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        return Err(AppError::plain("Slug is empty."));
    }
    let (mut cli_state, path) =
        CliState::load_default().map_err(|e| AppError::plain(e.to_string()))?;
    cli_state.remove_ignored_slug(trimmed);
    cli_state
        .save(&path)
        .map_err(|e| AppError::plain(e.to_string()))?;
    Ok(())
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
/// explaining what every step kept / dropped. Read-only — does not write
/// to the detection cache or `state.json`. Backs the hidden
/// `/diagnostics` route unlocked by the 5-click sidebar gesture.
#[tauri::command]
pub async fn detection_diagnostics(slug: String) -> Result<DetectionTrace, String> {
    if slug.trim().is_empty() {
        return Err("Slug is empty.".into());
    }
    let (cli_state, _path) = CliState::load_default().map_err(|e| e.to_string())?;
    Ok(detection::diagnose(slug.trim(), Os::current(), &cli_state).await)
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

/// Hard-delete a save: drop the row + every snapshot on the server **and**
/// purge any local CliState that referenced it. Sibling of `untrack_save`,
/// but destructive on purpose — it's what the user clicks when a save was
/// tracked against a wrong path and the only way out is to start over
/// (otherwise `add_game_to_tracking` swallows the next 409 and re-links to
/// the bad row). The matching `manual_paths` override is also cleared so a
/// re-add doesn't immediately bounce back to the same wrong folder.
#[tauri::command]
pub async fn delete_save_completely(
    save_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = current_client(&state)?;
    client.delete_save(&save_id).await.map_err(pretty_error)?;

    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    let slug = cli_state.saves.get(&save_id).map(|s| s.game_slug.clone());
    cli_state.saves.remove(&save_id);
    if let Some(slug) = slug {
        cli_state.clear_manual_path(&slug);
    }
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
    // Prefer the self-hosted session: the cached UserInfo gives us the URL and
    // the keychain holds the bearer token.
    let self_hosted = state.user.lock().unwrap().clone();
    if let Some(user) = self_hosted {
        let creds = credentials::load()
            .map_err(|e| format!("Couldn't load credentials: {e}"))?
            .ok_or_else(|| "Saved credentials are missing. Sign in again.".to_string())?;
        return ApiClient::new(user.server_url, creds.token).map_err(|e| e.to_string());
    }
    // Fall back to a Hoard Cloud session (Gmail login). The cloud API exposes
    // the same `/v1/...` surface and accepts the Supabase JWT as a bearer
    // token, so the agent and every library/history command can talk to it
    // exactly like a self-hosted server. Without this branch a cloud-only user
    // hit "Not logged in" on every monitor/backup action even though the
    // sidebar showed "Nube conectada".
    //
    // Note: the JWT is short-lived (~1h). The cloud-pull poller refreshes the
    // on-disk token on a 401, so per-command clients built here stay fresh;
    // the long-lived agent client created at `start_agent` is the one case that
    // can outlive a token and is handled separately.
    if let Some(cloud) = crate::commands::cloud::load_active_creds()
        .map_err(|e| format!("Couldn't load cloud credentials: {e}"))?
    {
        return ApiClient::new(cloud.server_url, cloud.access_token).map_err(|e| e.to_string());
    }
    Err("Not logged in. Sign in again to continue.".to_string())
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
    // `tauri::async_runtime::spawn`, not `tokio::spawn`: this is called from
    // `setup()`, which runs before Tauri enters its event loop, so there is
    // no ambient Tokio runtime yet. Using `tokio::spawn` here panics with
    // "there is no reactor running" the instant the app starts — that's how
    // 1.4.0 shipped, which is why the binary refused to launch on every
    // platform after the upgrade until the user reopened it from a terminal
    // and saw the stack trace.
    tauri::async_runtime::spawn(async move {
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
            // Reload state on each background pass so manual_paths overrides
            // that landed since the previous scan are honoured. Cheap (one
            // small JSON file read) and skipping it would let manual picks
            // silently disappear from the auto-refreshed report.
            let cli_state = match CliState::load_default() {
                Ok((s, _)) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "couldn't load state for background rescan");
                    continue;
                }
            };
            // No progress emit on the background path — the UI isn't
            // listening, and repainting a progress bar while the user is
            // on another page would be noise.
            let mut report = match detection::detect_all(os, &cli_state, |_, _| {}).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(error = %e, "background detection refresh failed");
                    continue;
                }
            };
            // Honour the user's blacklist on the background path too —
            // otherwise the cache flips back to including ignored slugs
            // every time the 24h scheduler fires.
            report.games.retain(|g| !cli_state.is_ignored(&g.slug));
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
