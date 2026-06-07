//! History, restore, manual-override and log-viewer commands.
//!
//! These power the per-save History page (timeline of snapshots, restore,
//! soft-delete, undelete) plus the small "manual control" surface area:
//! pause/resume, edit local path, and read the agent log file from the
//! Settings → Advanced screen.
//!
//! Restore is the only command here that does meaningful Rust work: the
//! others are thin wrappers around the API client. Restore optionally runs
//! a synchronous "backup the current state first" step so the user can roll
//! back if they restored the wrong version.

use std::path::PathBuf;

use hoard_agent::backup;
use hoard_agent::config::CliConfig;
use hoard_agent::restore::{self, RestoreOptions};
use hoard_agent::state::CliState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use time::format_description::well_known::Rfc3339;

use super::agent::{attach_save_if_running, detach_save_if_running, watched_save_from};
use super::auth::pretty_error;
use super::library::current_client;
use crate::state::AppState;

/// Wire shape for one snapshot in the History timeline.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotWire {
    pub version_num: i64,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub is_pinned: bool,
    pub created_at: String,
    pub deleted_at: Option<String>,
}

/// Wire shape for the per-snapshot detail view.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotDetailWire {
    #[serde(flatten)]
    pub snapshot: SnapshotWire,
    pub files: Vec<SnapshotFileWire>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotFileWire {
    pub relative_path: String,
    pub size_bytes: i64,
    pub sha256: String,
}

fn fmt_time(t: time::OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_else(|_| t.to_string())
}

fn snapshot_to_wire(s: hoard_agent::api::Snapshot) -> SnapshotWire {
    SnapshotWire {
        version_num: s.version_num,
        file_count: s.file_count,
        total_size_bytes: s.total_size_bytes,
        is_pinned: s.is_pinned,
        created_at: fmt_time(s.created_at),
        deleted_at: s.deleted_at.map(fmt_time),
    }
}

/// List snapshots for a save. `include_deleted` brings rows in the trash too
/// (recoverable until the server's retention window passes).
#[tauri::command]
pub async fn list_save_snapshots(
    save_id: String,
    include_deleted: bool,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotWire>, String> {
    let client = current_client(&state)?;
    // Cloud now exposes the full version history via a dedicated endpoint
    // (the sync manifest still only carries the latest). One row per version.
    if client.is_cloud().await {
        let snaps = client
            .cloud_list_versions(&save_id, include_deleted)
            .await
            .map_err(pretty_error)?;
        return Ok(snaps.into_iter().map(snapshot_to_wire).collect());
    }
    let snaps = client
        .list_snapshots(&save_id, include_deleted)
        .await
        .map_err(pretty_error)?;
    Ok(snaps.into_iter().map(snapshot_to_wire).collect())
}

/// Detail view: snapshot metadata + per-file list, used by the expandable
/// row on the History page.
#[tauri::command]
pub async fn save_snapshot_detail(
    save_id: String,
    version: i64,
    state: State<'_, AppState>,
) -> Result<SnapshotDetailWire, String> {
    let client = current_client(&state)?;
    // Cloud stores a whole-archive blob with no per-file index, so the detail
    // view returns the matching version's metadata with an empty file list.
    if client.is_cloud().await {
        let snap = client
            .cloud_list_versions(&save_id, true)
            .await
            .map_err(pretty_error)?
            .into_iter()
            .find(|s| s.version_num == version)
            .ok_or_else(|| "snapshot not found".to_string())?;
        return Ok(SnapshotDetailWire {
            snapshot: snapshot_to_wire(snap),
            files: Vec::new(),
        });
    }
    let detail = client
        .snapshot_detail(&save_id, version)
        .await
        .map_err(pretty_error)?;
    Ok(SnapshotDetailWire {
        snapshot: snapshot_to_wire(detail.snapshot),
        files: detail
            .files
            .into_iter()
            .map(|f| SnapshotFileWire {
                relative_path: f.relative_path,
                size_bytes: f.size_bytes,
                sha256: f.sha256,
            })
            .collect(),
    })
}

/// Move a snapshot to the server-side trash. Recoverable for the configured
/// retention window. The snapshot disappears from the active list but
/// `include_deleted=true` queries can still see it.
#[tauri::command]
pub async fn delete_snapshot(
    save_id: String,
    version: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = current_client(&state)?;
    // Cloud now deletes a single version (blob + row) and repoints the latest
    // pointer; the whole save is removed only when no versions remain. The
    // server handles that fallback, so this stays per-version.
    if client.is_cloud().await {
        return client
            .cloud_delete_version(&save_id, version)
            .await
            .map_err(pretty_error);
    }
    client
        .snapshot_delete(&save_id, version)
        .await
        .map_err(pretty_error)
}

/// Pull a snapshot back out of the trash. Inverse of `delete_snapshot`.
#[tauri::command]
pub async fn undelete_snapshot(
    save_id: String,
    version: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = current_client(&state)?;
    if client.is_cloud().await {
        return Err("Restoring snapshots from trash isn't supported on Hoard Cloud.".to_string());
    }
    client
        .snapshot_restore(&save_id, version)
        .await
        .map_err(pretty_error)
}

/// Phase the restore flow currently sits in. Lets the UI swap progress
/// labels ("Backing up current state" vs "Downloading version 7…") without
/// the frontend having to track which call it issued first.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePhase {
    PreBackup,
    Downloading,
    Done,
}

/// Progress payload emitted on `restore://progress`. `total` is 0 when the
/// server didn't send Content-Length — the UI should fall back to a
/// "still working" spinner in that case.
#[derive(Debug, Clone, Serialize)]
pub struct RestoreProgress {
    pub save_id: String,
    pub version: i64,
    pub phase: RestorePhase,
    pub downloaded: u64,
    pub total: u64,
}

/// Summary returned when restore finishes successfully.
#[derive(Debug, Clone, Serialize)]
pub struct RestoreOutcome {
    pub files_extracted: usize,
    pub bytes_extracted: u64,
    pub destination: String,
    pub safety_version: Option<i64>,
}

/// Restore an old snapshot into the local save folder.
///
/// If `backup_first` is true, we synchronously upload the current state as a
/// fresh snapshot *before* overwriting anything — that's the user's escape
/// hatch if they restored the wrong version. The returned `safety_version`
/// is what they'd restore to undo this.
///
/// `destination_override` lets the caller restore into a folder we don't yet
/// have in `CliState` (e.g. the user pulled this save from another machine
/// and never tracked it locally). When provided, the path is created if
/// missing and persisted to `CliState` so subsequent restores can skip the
/// dialog.
#[tauri::command]
pub async fn restore_snapshot(
    app: AppHandle,
    save_id: String,
    version: i64,
    backup_first: bool,
    destination_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<RestoreOutcome, String> {
    let client = current_client(&state)?;

    // Resolve the destination. Priority: explicit override > CliState lookup.
    // If neither yields a path we surface a structured error the frontend
    // catches and turns into a folder-picker dialog.
    let (mut cli_state, cli_state_path) = CliState::load_default().map_err(|e| e.to_string())?;

    let local_path: PathBuf = if let Some(raw) = destination_override.as_deref() {
        let p = PathBuf::from(raw.trim());
        if p.as_os_str().is_empty() {
            return Err("Destination path can't be empty.".to_string());
        }
        // Auto-create the folder if it's missing — the user explicitly
        // picked it as the restore target, so creating an empty dir is
        // less surprising than failing.
        if !p.exists() {
            std::fs::create_dir_all(&p)
                .map_err(|e| format!("Couldn't create {}: {e}", p.display()))?;
        } else if !p.is_dir() {
            return Err(format!("{} isn't a folder.", p.display()));
        }
        // Persist the chosen path so future restores/backups know where
        // this save lives. We need the server-side metadata to fill in
        // `game_slug` / `label` if the entry is brand-new on this machine.
        let entry = cli_state.saves.entry(save_id.clone()).or_insert_with(|| {
            // Defaults that get patched below from the server response.
            hoard_agent::state::SaveState {
                local_path: p.clone(),
                game_slug: String::new(),
                label: String::new(),
                last_backup_at: None,
                last_version_num: None,
                paused: false,
                set_hash: None,
            }
        });
        entry.local_path = p.clone();
        if entry.game_slug.is_empty() || entry.label.is_empty() {
            if client.is_cloud().await {
                if let Ok(manifest) = client.cloud_sync().await {
                    if let Some(e) = manifest.saves.into_iter().find(|e| e.save_id == save_id) {
                        if entry.game_slug.is_empty() {
                            entry.game_slug = e.game_slug;
                        }
                        if entry.label.is_empty() {
                            entry.label = e.label;
                        }
                    }
                }
            } else if let Ok(server_save) = client.get_save(&save_id).await {
                if entry.game_slug.is_empty() {
                    entry.game_slug = server_save.game_slug;
                }
                if entry.label.is_empty() {
                    entry.label = server_save.label;
                }
            }
        }
        cli_state.save(&cli_state_path).map_err(|e| e.to_string())?;
        p
    } else {
        cli_state
            .saves
            .get(&save_id)
            .map(|s| s.local_path.clone())
            .ok_or_else(|| "NEEDS_DESTINATION".to_string())?
    };

    // Slug/label for the cloud upload init (ignored by self-hosted). Read
    // from CliState, which now holds the resolved entry from either branch.
    let (game_slug, label) = cli_state
        .saves
        .get(&save_id)
        .map(|s| (s.game_slug.clone(), s.label.clone()))
        .unwrap_or_default();

    // 1) Optional pre-restore backup. Done synchronously so the user can be
    //    sure the safety net exists before we start overwriting files.
    let mut safety_version = None;
    if backup_first && local_path.exists() {
        // Walk the directory; if it's empty, skip the pre-backup — there's
        // nothing to preserve and `upload_directory` would fail with a
        // "no files found" error which would confuse the user.
        let any_files = std::fs::read_dir(&local_path)
            .map(|mut it| it.any(|e| e.is_ok()))
            .unwrap_or(false);
        if any_files {
            let app_for_progress = app.clone();
            let save_id_for_progress = save_id.clone();
            emit_phase(&app, &save_id, version, RestorePhase::PreBackup, 0, 0);
            let outcome = backup::upload_directory(
                &client,
                &save_id,
                &game_slug,
                &label,
                &local_path,
                // Pre-restore safety backup is an explicit user action; don't
                // gate it on fast-forward.
                None,
                move |uploaded, total| {
                    let _ = app_for_progress.emit(
                        "restore://progress",
                        RestoreProgress {
                            save_id: save_id_for_progress.clone(),
                            version,
                            phase: RestorePhase::PreBackup,
                            downloaded: uploaded,
                            total,
                        },
                    );
                },
            )
            .await
            .map_err(pretty_error)?;
            safety_version = Some(outcome.snapshot.version_num);
        }
    }

    // 2) Download + verify + extract. We pass `force = true` because the
    //    user has explicitly confirmed they want to overwrite; refusing on
    //    "destination not empty" here would defeat the whole point.
    let app_for_dl = app.clone();
    let save_id_for_dl = save_id.clone();
    emit_phase(&app, &save_id, version, RestorePhase::Downloading, 0, 0);
    let outcome = restore::download_snapshot(
        &client,
        &save_id,
        version,
        &local_path,
        RestoreOptions {
            skip_verify: false,
            force: true,
        },
        move |downloaded, total| {
            let _ = app_for_dl.emit(
                "restore://progress",
                RestoreProgress {
                    save_id: save_id_for_dl.clone(),
                    version,
                    phase: RestorePhase::Downloading,
                    downloaded,
                    total,
                },
            );
        },
    )
    .await
    .map_err(pretty_error)?;

    emit_phase(&app, &save_id, version, RestorePhase::Done, 0, 0);

    Ok(RestoreOutcome {
        files_extracted: outcome.files_extracted,
        bytes_extracted: outcome.bytes_extracted,
        destination: outcome.destination.to_string_lossy().into_owned(),
        safety_version,
    })
}

fn emit_phase(
    app: &AppHandle,
    save_id: &str,
    version: i64,
    phase: RestorePhase,
    downloaded: u64,
    total: u64,
) {
    let _ = app.emit(
        "restore://progress",
        RestoreProgress {
            save_id: save_id.to_string(),
            version,
            phase,
            downloaded,
            total,
        },
    );
}

/// Pause or resume tracking for a save. Paused saves stay in the list but
/// the agent won't watch them — useful when the user is reorganising files
/// or modding and doesn't want chatty backups while they work.
#[tauri::command]
pub async fn set_save_paused(
    save_id: String,
    paused: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    let entry = cli_state
        .saves
        .get_mut(&save_id)
        .ok_or_else(|| "That save isn't tracked on this machine — nothing to pause.".to_string())?;
    entry.paused = paused;
    let snapshot = entry.clone();
    cli_state.save(&path).map_err(|e| e.to_string())?;

    if paused {
        // Detach from the running agent so it stops watching immediately.
        // No-op if the agent isn't running.
        detach_save_if_running(&state, save_id).await;
    } else {
        // Re-attach using the freshly-read entry so the agent picks up the
        // current local_path in case it changed since startup.
        let watched = watched_save_from(
            save_id,
            snapshot.game_slug.clone(),
            snapshot.game_slug,
            snapshot.label,
            snapshot.local_path,
        );
        attach_save_if_running(&state, watched).await;
    }
    Ok(())
}

/// Update the local-disk path for a tracked save. Used when the user moves
/// the game folder (re-installed on a different drive, switched from Steam
/// to GOG, etc.). The folder is created if missing — that's the friendly
/// behaviour for "I haven't installed the game yet but I want my saves
/// restored here". The agent gets detached + reattached so the FS watcher
/// rebinds to the new location.
#[tauri::command]
pub async fn set_save_local_path(
    save_id: String,
    new_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(new_path.trim());
    if path_buf.as_os_str().is_empty() {
        return Err("Path can't be empty.".to_string());
    }
    if !path_buf.exists() {
        std::fs::create_dir_all(&path_buf)
            .map_err(|e| format!("Couldn't create {}: {e}", path_buf.display()))?;
    } else if !path_buf.is_dir() {
        return Err(format!("{} isn't a folder.", path_buf.display()));
    }

    let (mut cli_state, path) = CliState::load_default().map_err(|e| e.to_string())?;
    let entry = cli_state
        .saves
        .get_mut(&save_id)
        .ok_or_else(|| "That save isn't tracked on this machine.".to_string())?;
    entry.local_path = path_buf.clone();
    let snapshot = entry.clone();
    cli_state.save(&path).map_err(|e| e.to_string())?;

    // Reseat the live agent's watch by detaching + reattaching. Cheaper than
    // a full restart and means the new path is in effect within a tick.
    detach_save_if_running(&state, save_id.clone()).await;
    if !snapshot.paused {
        let watched = watched_save_from(
            save_id,
            snapshot.game_slug.clone(),
            snapshot.game_slug,
            snapshot.label,
            snapshot.local_path,
        );
        attach_save_if_running(&state, watched).await;
    }
    Ok(())
}

/// One log line as exposed to the Logs viewer. Splitting timestamp out lets
/// the UI render a faded prefix without having to re-parse the line.
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Read the tail of the most recent rolling log file. `max_lines` is capped
/// server-side so a malicious frontend can't ask us to slurp 200 MB.
#[tauri::command]
pub fn tail_logs(max_lines: Option<usize>) -> Result<Vec<LogLine>, String> {
    let cap = max_lines.unwrap_or(500).min(5000);
    let dir = CliConfig::logs_dir().map_err(|e| e.to_string())?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    // `tracing-appender::rolling::daily` produces files named
    // `agent.log.YYYY-MM-DD`. Lex-sorting by name puts the freshest one
    // last; we reverse-sort and take the head so we read the newest
    // available file (which is also the most useful for "what just
    // happened").
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| format!("reading {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("agent.log"))
        .map(|e| e.path())
        .collect();
    entries.sort();
    let Some(latest) = entries.into_iter().next_back() else {
        return Ok(Vec::new());
    };

    let text = std::fs::read_to_string(&latest)
        .map_err(|e| format!("reading {}: {e}", latest.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(cap);
    Ok(lines[start..]
        .iter()
        .map(|raw| parse_log_line(raw))
        .collect())
}

/// Best-effort parse of a tracing pretty-format line into (timestamp, level,
/// message). The format is roughly `2024-12-31T22:14:33.123456Z  INFO module:
/// message`. We don't fight too hard for malformed lines — the user just
/// wants to read them, not query them.
fn parse_log_line(raw: &str) -> LogLine {
    // Try to split on the first two whitespace runs after the timestamp.
    // Fall back to dumping the whole thing in `message` if that fails.
    let mut parts = raw.splitn(3, ' ');
    let ts = parts.next().unwrap_or("").to_string();
    let mut rest = parts.next().unwrap_or("").trim();
    // Some log formats put extra spaces between TS and level — eat them.
    while rest.is_empty() {
        match parts.next() {
            Some(s) => rest = s.trim(),
            None => break,
        }
    }
    let (level, message) = match rest {
        "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" => {
            (rest.to_string(), parts.next().unwrap_or("").to_string())
        }
        _ => ("".to_string(), {
            let mut m = rest.to_string();
            if let Some(tail) = parts.next() {
                m.push(' ');
                m.push_str(tail);
            }
            m
        }),
    };
    LogLine {
        timestamp: ts,
        level,
        message,
    }
}

/// Where the log file lives, so the Settings page can show "logs are at
/// /home/.../agent.log" — useful for users who want to attach them to
/// a bug report.
#[tauri::command]
pub fn logs_path() -> Result<String, String> {
    let dir = CliConfig::logs_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("agent.log").to_string_lossy().into_owned())
}
