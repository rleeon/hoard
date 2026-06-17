use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

// ─── Response types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub version_num: i64,
    /// The version this snapshot descends from. `None` = root (first version
    /// of the save). The DAG edge that makes divergence detectable.
    pub parent_version: Option<i64>,
    pub device_name: Option<String>,
    pub notes: Option<String>,
    pub total_size_bytes: i64,
    pub file_count: i64,
    pub is_pinned: bool,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SnapshotDetail {
    #[serde(flatten)]
    pub summary: SnapshotSummary,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub size_bytes: i64,
    pub sha256: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub include_deleted: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn internal() -> (StatusCode, Json<serde_json::Value>) {
    err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

/// Validate that a relative path stays inside its parent directory.
/// Rejects: absolute paths, "..", empty components, drive prefixes.
fn is_safe_relative_path(p: &str) -> bool {
    if p.is_empty() || p.starts_with('/') || p.starts_with('\\') {
        return false;
    }
    let path = std::path::Path::new(p);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return false, // ParentDir, RootDir, Prefix, CurDir all unsafe
        }
    }
    true
}

async fn ownership_check(
    pool: &sqlx::SqlitePool,
    save_id: &str,
    user_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT game_slug, label FROM saves WHERE id=? AND user_id=?",
        save_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| (r.game_slug, r.label)))
}

// ─── POST /v1/saves/:save_id/snapshots ──────────────────────────────────────

pub async fn create(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<SnapshotSummary>), (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    let (game_slug, label) = ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;

    // Quota check setup
    let (quota, used): (i64, i64) = sqlx::query!(
        "SELECT storage_quota_bytes, storage_used_bytes FROM users WHERE id=?",
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .map(|r| (r.storage_quota_bytes, r.storage_used_bytes))
    .map_err(|_| internal())?;

    let max_per_snapshot = (state.config.storage.max_snapshot_size_mb as i64) * 1024 * 1024;
    // Per-file multipart keeps its modest cap (each file is a round-trip and a
    // row). The packed mode (a single `pack` tar field, ADR 0019) lifts it:
    // thousands of tiny files arrive in one stream, so handles and round-trips
    // stop being the bottleneck. `max_files` starts at the per-file cap and is
    // raised the moment a `pack` field is seen.
    const MAX_FILES_PER_SNAPSHOT: usize = 1000;
    const MAX_FILES_PACKED: usize = 50_000;
    let mut max_files = MAX_FILES_PER_SNAPSHOT;

    let upload_id = Uuid::new_v4().to_string();
    let tmp_root = state.config.storage.data_dir.join("tmp").join(&upload_id);
    tokio::fs::create_dir_all(&tmp_root)
        .await
        .map_err(|_| internal())?;

    // Cleanup helper if anything goes wrong
    let cleanup_tmp = || {
        let p = tmp_root.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::remove_dir_all(&p).await;
        });
    };

    let mut device_name: Option<String> = None;
    let mut notes: Option<String> = None;
    // The version the client based this snapshot on (its last-synced version
    // for this save). When present and it no longer matches the server's head,
    // another device advanced the save → non-fast-forward, rejected below.
    let mut base_version: Option<i64> = None;
    let mut files: Vec<(String, i64, String)> = Vec::new(); // (rel_path, size, sha256)
    let mut total_size: i64 = 0;

    while let Some(field_result) = multipart.next_field().await.transpose() {
        let mut field = match field_result {
            Ok(f) => f,
            Err(e) => {
                warn!(error=%e, "multipart error");
                cleanup_tmp();
                return Err(err(StatusCode::BAD_REQUEST, "malformed multipart"));
            }
        };

        let name = field.name().unwrap_or("").to_string();

        if name == "device_name" {
            device_name = field.text().await.ok();
            continue;
        }
        if name == "notes" {
            notes = field.text().await.ok();
            continue;
        }
        if name == "base_version" {
            base_version = field
                .text()
                .await
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok());
            continue;
        }

        // ── Packed mode (ADR 0019) ──────────────────────────────────────
        // A single uncompressed tar carrying many files. We stream-unpack it
        // straight from the request body into `tmp_root`, hashing each entry,
        // and feed the same `files` vec the per-file path uses below — so the
        // commit logic (dedup, blobs, quota) is identical regardless of how
        // the bytes arrived.
        if name == "pack" {
            max_files = MAX_FILES_PACKED;
            let byte_stream = field.map(|r| r.map_err(std::io::Error::other));
            let reader = tokio_util::io::StreamReader::new(byte_stream);
            let mut archive = tokio_tar::Archive::new(reader);
            let mut entries = match archive.entries() {
                Ok(e) => e,
                Err(e) => {
                    warn!(error=%e, "opening pack tar");
                    cleanup_tmp();
                    return Err(err(StatusCode::BAD_REQUEST, "malformed pack archive"));
                }
            };
            while let Some(entry_res) = entries.next().await {
                let mut entry = match entry_res {
                    Ok(e) => e,
                    Err(e) => {
                        warn!(error=%e, "reading pack entry");
                        cleanup_tmp();
                        return Err(err(StatusCode::BAD_REQUEST, "malformed pack archive"));
                    }
                };
                if entry.header().entry_type().is_dir() {
                    continue;
                }
                let rel = match entry.path() {
                    Ok(p) => p.to_string_lossy().replace('\\', "/"),
                    Err(_) => {
                        cleanup_tmp();
                        return Err(err(StatusCode::BAD_REQUEST, "unsafe file path"));
                    }
                };
                if !is_safe_relative_path(&rel) {
                    cleanup_tmp();
                    return Err(err(StatusCode::BAD_REQUEST, "unsafe file path"));
                }
                if files.len() >= max_files {
                    cleanup_tmp();
                    return Err(err(StatusCode::BAD_REQUEST, "too many files in snapshot"));
                }
                let dest = tmp_root.join(&rel);
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|_| {
                        cleanup_tmp();
                        internal()
                    })?;
                }
                let mut out = tokio::fs::File::create(&dest).await.map_err(|_| {
                    cleanup_tmp();
                    internal()
                })?;
                let mut hasher = Sha256::new();
                let mut size: i64 = 0;
                let mut buf = vec![0u8; 256 * 1024];
                loop {
                    let n = match entry.read(&mut buf).await {
                        Ok(n) => n,
                        Err(e) => {
                            warn!(error=%e, "pack entry read");
                            cleanup_tmp();
                            return Err(err(StatusCode::BAD_REQUEST, "stream error"));
                        }
                    };
                    if n == 0 {
                        break;
                    }
                    size += n as i64;
                    total_size += n as i64;
                    if total_size > max_per_snapshot {
                        cleanup_tmp();
                        return Err(err(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "snapshot exceeds size limit",
                        ));
                    }
                    hasher.update(&buf[..n]);
                    if out.write_all(&buf[..n]).await.is_err() {
                        cleanup_tmp();
                        return Err(internal());
                    }
                }
                let _ = out.flush().await;
                let sha = hex::encode(hasher.finalize());
                files.push((rel, size, sha));
            }
            continue;
        }

        if name != "files" && name != "files[]" {
            // Drain unknown field
            let _ = field.bytes().await;
            continue;
        }

        let file_name = match field.file_name() {
            Some(f) => f.to_string(),
            None => {
                cleanup_tmp();
                return Err(err(StatusCode::BAD_REQUEST, "file field missing filename"));
            }
        };

        if !is_safe_relative_path(&file_name) {
            cleanup_tmp();
            return Err(err(StatusCode::BAD_REQUEST, "unsafe file path"));
        }

        if files.len() >= max_files {
            cleanup_tmp();
            return Err(err(StatusCode::BAD_REQUEST, "too many files in snapshot"));
        }

        let dest = tmp_root.join(&file_name);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|_| {
                cleanup_tmp();
                internal()
            })?;
        }

        let mut file = tokio::fs::File::create(&dest).await.map_err(|_| {
            cleanup_tmp();
            internal()
        })?;

        let mut hasher = Sha256::new();
        let mut size: i64 = 0;

        while let Some(chunk_result) = field.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    warn!(error=%e, "chunk read error");
                    cleanup_tmp();
                    return Err(err(StatusCode::BAD_REQUEST, "stream error"));
                }
            };
            size += chunk.len() as i64;
            total_size += chunk.len() as i64;

            if total_size > max_per_snapshot {
                cleanup_tmp();
                return Err(err(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "snapshot exceeds size limit",
                ));
            }
            // Quota is checked at commit time against deduplicated bytes — a
            // re-upload of mostly-identical files (the OpenTTD case) adds
            // almost nothing, so the streaming `total_size` check would reject
            // uploads that actually fit. `max_per_snapshot` still caps disk.

            hasher.update(&chunk);
            if let Err(e) = file.write_all(&chunk).await {
                warn!(error=%e, "file write error");
                cleanup_tmp();
                return Err(internal());
            }
        }

        let _ = file.flush().await;
        let sha = hex::encode(hasher.finalize());
        files.push((file_name, size, sha));
    }

    if files.is_empty() {
        cleanup_tmp();
        return Err(err(StatusCode::BAD_REQUEST, "no files uploaded"));
    }

    // ── Atomic commit: DB transaction + content-addressed blob placement ──
    //
    // Each file's bytes are stored once per user at blobs/<user>/<sha[0:2]>/<sha>
    // (ADR 0018, eje C). Identical files across versions share one blob; the
    // version is just its list of `snapshot_files` rows. Quota counts unique
    // blob bytes, so only the genuinely-new bytes of this upload are charged.
    let data_dir = state.config.storage.data_dir.clone();
    let _ = &game_slug; // path components no longer used for storage layout
    let _ = &label;

    // ── Chunk planning (ADR 0019, Fase 4) ───────────────────────────────────
    // A file above the chunk threshold is split by the content-defined chunker
    // so a monolithic save that rewrites a few KB per version re-stores only
    // the changed chunks. Planning only hashes (no disk writes yet), so a quota
    // rejection below costs nothing to undo. Files at/below the threshold keep
    // the whole-file blob path untouched.
    let mut chunk_plans: std::collections::HashMap<usize, Vec<crate::chunking::ChunkPlan>> =
        std::collections::HashMap::new();
    for (i, (rel, size, _sha)) in files.iter().enumerate() {
        if *size as u64 > crate::chunking::CHUNK_THRESHOLD {
            match crate::chunking::plan_chunks(&tmp_root.join(rel)).await {
                Ok(plan) => {
                    chunk_plans.insert(i, plan);
                }
                Err(e) => {
                    warn!(error=%e, path=%rel, "chunk planning failed");
                    cleanup_tmp();
                    return Err(internal());
                }
            }
        }
    }

    // New bytes = distinct content (whole-file blobs for small files, per-chunk
    // for chunked ones) not already on disk for this user. Both stores are
    // counted so dedup across versions is reflected in quota exactly once.
    let mut seen_blobs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_chunks: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut newly_stored_bytes: i64 = 0;
    for (i, (_, size, sha)) in files.iter().enumerate() {
        if let Some(plan) = chunk_plans.get(&i) {
            for c in plan {
                if seen_chunks.insert(c.sha256.clone())
                    && !crate::chunking::chunk_path(&data_dir, &user_id, &c.sha256).exists()
                {
                    newly_stored_bytes += c.len as i64;
                }
            }
        } else if seen_blobs.insert(sha.clone())
            && !crate::blobs::blob_path(&data_dir, &user_id, sha).exists()
        {
            newly_stored_bytes += size;
        }
    }
    if used + newly_stored_bytes > quota {
        cleanup_tmp();
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "storage quota exceeded"));
    }

    let snapshot_id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await.map_err(|_| {
        cleanup_tmp();
        internal()
    })?;

    let head: i64 = sqlx::query!("SELECT latest_version_num FROM saves WHERE id=?", save_id)
        .fetch_one(&mut *tx)
        .await
        .map(|r| r.latest_version_num)
        .map_err(|_| {
            cleanup_tmp();
            internal()
        })?;

    // Fast-forward check (the DAG's enforcement). A client that declares a
    // base version which is no longer the head has diverged: another device
    // pushed since it last synced. Reject so the client can pull + merge
    // (keep-both) instead of silently overwriting the other line.
    if let Some(base) = base_version {
        if base != head {
            cleanup_tmp();
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "non-fast-forward: another device advanced this save since your base version",
                    "code": "non_fast_forward",
                    "head_version": head,
                    "base_version": base,
                })),
            ));
        }
    }
    let new_version = head + 1;
    // Root version has no parent; every other version points at the head it
    // descended from.
    let parent_version: Option<i64> = (head > 0).then_some(head);

    let file_count = files.len() as i64;
    sqlx::query(
        "INSERT INTO snapshots (id, save_id, version_num, device_name, notes,
                                total_size_bytes, file_count, parent_version)
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(&snapshot_id)
    .bind(&save_id)
    .bind(new_version)
    .bind(&device_name)
    .bind(&notes)
    .bind(total_size)
    .bind(file_count)
    .bind(parent_version)
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        cleanup_tmp();
        internal()
    })?;

    // Blobs we physically placed this request, so we can roll them back if the
    // transaction fails to commit.
    let mut created_blobs: Vec<PathBuf> = Vec::new();
    let rollback_blobs = |blobs: &[PathBuf]| {
        let blobs: Vec<PathBuf> = blobs.to_vec();
        tokio::spawn(async move {
            for p in blobs {
                let _ = tokio::fs::remove_file(&p).await;
            }
        });
    };

    for (i, (rel_path, size, sha)) in files.iter().enumerate() {
        let file_id = Uuid::new_v4().to_string();
        if sqlx::query!(
            "INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256)
             VALUES (?,?,?,?,?)",
            file_id,
            snapshot_id,
            rel_path,
            size,
            sha
        )
        .execute(&mut *tx)
        .await
        .is_err()
        {
            rollback_blobs(&created_blobs);
            cleanup_tmp();
            return Err(internal());
        }

        // ── Chunked file (ADR 0019, Fase 4) ─────────────────────────────
        // A large file has no whole-file blob: its bytes live as
        // content-defined chunks listed in order by snapshot_file_chunks.
        // Each chunk is refcounted and placed on disk exactly like a blob,
        // so dedup, GC and quota treat chunks and blobs uniformly.
        if let Some(plan) = chunk_plans.get(&i) {
            let src = tmp_root.join(rel_path);
            for (ordinal, c) in plan.iter().enumerate() {
                let ord = ordinal as i64;
                let csize = c.len as i64;
                if sqlx::query(
                    "INSERT INTO snapshot_file_chunks (snapshot_file_id, ordinal, chunk_sha256)
                     VALUES (?,?,?)",
                )
                .bind(&file_id)
                .bind(ord)
                .bind(&c.sha256)
                .execute(&mut *tx)
                .await
                .is_err()
                {
                    rollback_blobs(&created_blobs);
                    cleanup_tmp();
                    return Err(internal());
                }
                if sqlx::query(
                    "INSERT INTO chunks (user_id, sha256, size_bytes, refcount)
                     VALUES (?,?,?,1)
                     ON CONFLICT(user_id, sha256) DO UPDATE SET refcount = refcount + 1",
                )
                .bind(&user_id)
                .bind(&c.sha256)
                .bind(csize)
                .execute(&mut *tx)
                .await
                .is_err()
                {
                    rollback_blobs(&created_blobs);
                    cleanup_tmp();
                    return Err(internal());
                }
                let dst = crate::chunking::chunk_path(&data_dir, &user_id, &c.sha256);
                if !dst.exists() {
                    if crate::chunking::place_chunk(&src, c.offset, c.len, &dst)
                        .await
                        .is_err()
                    {
                        warn!(sha = %c.sha256, "chunk placement failed");
                        rollback_blobs(&created_blobs);
                        cleanup_tmp();
                        return Err(internal());
                    }
                    created_blobs.push(dst);
                }
            }
            // tmp source is left for cleanup_tmp; chunks were copied out.
            continue;
        }

        // Reference-count the blob (insert at 1, or bump an existing one).
        if sqlx::query(
            "INSERT INTO blobs (user_id, sha256, size_bytes, refcount)
             VALUES (?,?,?,1)
             ON CONFLICT(user_id, sha256) DO UPDATE SET refcount = refcount + 1",
        )
        .bind(&user_id)
        .bind(sha)
        .bind(size)
        .execute(&mut *tx)
        .await
        .is_err()
        {
            rollback_blobs(&created_blobs);
            cleanup_tmp();
            return Err(internal());
        }

        // Place the bytes on disk exactly once.
        let dst = crate::blobs::blob_path(&data_dir, &user_id, sha);
        if !dst.exists() {
            if let Some(parent) = dst.parent() {
                if tokio::fs::create_dir_all(parent).await.is_err() {
                    rollback_blobs(&created_blobs);
                    cleanup_tmp();
                    return Err(internal());
                }
            }
            let src = tmp_root.join(rel_path);
            // Same filesystem (tmp + blobs share data_dir) → rename; fall back
            // to copy on the off chance of EXDEV.
            let placed = match tokio::fs::rename(&src, &dst).await {
                Ok(_) => true,
                Err(_) => tokio::fs::copy(&src, &dst).await.is_ok(),
            };
            if !placed {
                warn!(sha = %sha, "blob placement failed");
                rollback_blobs(&created_blobs);
                cleanup_tmp();
                return Err(internal());
            }
            created_blobs.push(dst);
        }
    }

    sqlx::query!(
        "UPDATE saves SET latest_version_num=? WHERE id=?",
        new_version,
        save_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal()
    })?;

    let new_used = used + newly_stored_bytes;
    sqlx::query!(
        "UPDATE users SET storage_used_bytes=? WHERE id=?",
        new_used,
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal()
    })?;

    let audit_id = Uuid::new_v4().to_string();
    let metadata = serde_json::json!({
        "save_id": save_id,
        "version_num": new_version,
        "files": file_count,
        "bytes": total_size,
        "new_bytes": newly_stored_bytes,
    })
    .to_string();
    sqlx::query!(
        "INSERT INTO audit_log (id, user_id, event_type, entity_id, metadata)
         VALUES (?,?,'snapshot.created',?,?)",
        audit_id,
        user_id,
        snapshot_id,
        metadata
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal()
    })?;

    if let Err(e) = tx.commit().await {
        warn!(error=%e, "transaction commit failed");
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        return Err(internal());
    }

    // Drop any leftover tmp files (duplicate-sha entries we didn't move out).
    cleanup_tmp();

    info!(
        user = %user.username,
        save_id = %save_id,
        version = new_version,
        files = file_count,
        bytes = total_size,
        "snapshot created"
    );

    // Push the new version to any of this user's other devices listening on
    // `/v1/events`, so they pull within ~1s instead of waiting for the agent's
    // reconciliation sweep. No-op when nobody is connected (incl. cloud, which
    // never has subscribers here).
    state.events.publish(
        user.user_id,
        crate::routes::events::SaveEvent {
            save_id: save_id.clone(),
            version_num: new_version,
        },
    );

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    Ok((
        StatusCode::CREATED,
        Json(SnapshotSummary {
            id: snapshot_id,
            version_num: new_version,
            parent_version,
            device_name,
            notes,
            total_size_bytes: total_size,
            file_count,
            is_pinned: false,
            deleted_at: None,
            created_at: now,
        }),
    ))
}

// ─── GET /v1/saves/:save_id/snapshots ───────────────────────────────────────

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<SnapshotSummary>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    if ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "save not found"));
    }

    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);

    // Runtime query (not the `query!` macro) so the new parent_version column
    // can be selected without regenerating the offline sqlx cache.
    let sql = if q.include_deleted {
        "SELECT id, version_num, parent_version, device_name, notes, total_size_bytes,
                file_count, is_pinned, deleted_at, created_at
         FROM snapshots WHERE save_id=?
         ORDER BY version_num DESC LIMIT ? OFFSET ?"
    } else {
        "SELECT id, version_num, parent_version, device_name, notes, total_size_bytes,
                file_count, is_pinned, deleted_at, created_at
         FROM snapshots WHERE save_id=? AND deleted_at IS NULL
         ORDER BY version_num DESC LIMIT ? OFFSET ?"
    };
    let rows: Vec<SnapshotSummary> = sqlx::query(sql)
        .bind(&save_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| internal())?
        .iter()
        .map(|r| SnapshotSummary {
            id: r.get("id"),
            version_num: r.get("version_num"),
            parent_version: r.get("parent_version"),
            device_name: r.get("device_name"),
            notes: r.get("notes"),
            total_size_bytes: r.get("total_size_bytes"),
            file_count: r.get("file_count"),
            is_pinned: r.get::<i64, _>("is_pinned") != 0,
            deleted_at: r.get("deleted_at"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(rows))
}

// ─── GET /v1/saves/:save_id/snapshots/:version ──────────────────────────────

pub async fn detail(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Json<SnapshotDetail>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();
    if ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "save not found"));
    }

    let snap = sqlx::query(
        "SELECT id, version_num, parent_version, device_name, notes, total_size_bytes,
                file_count, is_pinned, deleted_at, created_at
         FROM snapshots WHERE save_id=? AND version_num=?",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| internal())?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    let snap_id: String = snap.get("id");
    let files = sqlx::query!(
        "SELECT relative_path, size_bytes, sha256 FROM snapshot_files
         WHERE snapshot_id=? ORDER BY relative_path",
        snap_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| internal())?
    .into_iter()
    .map(|r| FileEntry {
        relative_path: r.relative_path,
        size_bytes: r.size_bytes,
        sha256: r.sha256,
    })
    .collect();

    Ok(Json(SnapshotDetail {
        summary: SnapshotSummary {
            id: snap.get("id"),
            version_num: snap.get("version_num"),
            parent_version: snap.get("parent_version"),
            device_name: snap.get("device_name"),
            notes: snap.get("notes"),
            total_size_bytes: snap.get("total_size_bytes"),
            file_count: snap.get("file_count"),
            is_pinned: snap.get::<i64, _>("is_pinned") != 0,
            deleted_at: snap.get("deleted_at"),
            created_at: snap.get("created_at"),
        },
        files,
    }))
}

// ─── GET /v1/saves/:save_id/snapshots/:version/download ─────────────────────

pub async fn download(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();
    let (game_slug, label) = ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;

    let snap_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM snapshots WHERE save_id=? AND version_num=? AND deleted_at IS NULL",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| internal())?;
    let snap_id = snap_id.ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    // A version is just its list of files; reconstruct the tarball from the
    // referenced blobs and/or chunks (ADR 0018 eje C + ADR 0019 Fase 4). No
    // per-version folder exists anymore. A file is reassembled from its ordered
    // chunks when it has snapshot_file_chunks rows, otherwise from its single
    // whole-file blob — transparent to the client, which gets the same tar.zst.
    let file_rows = sqlx::query(
        "SELECT id, relative_path, size_bytes, sha256 FROM snapshot_files
         WHERE snapshot_id=? ORDER BY relative_path",
    )
    .bind(&snap_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| internal())?;

    let data_dir = state.config.storage.data_dir.clone();
    let uid = user_id.clone();

    // How to source one entry's bytes when building the tar.
    enum DlSource {
        Blob(PathBuf),
        Chunks { paths: Vec<PathBuf>, size: u64 },
    }

    let mut entries: Vec<(String, DlSource)> = Vec::with_capacity(file_rows.len());
    for r in &file_rows {
        let file_id: String = r.get("id");
        let rel: String = r.get("relative_path");
        let size: i64 = r.get("size_bytes");
        let sha: String = r.get("sha256");

        let chunk_rows = sqlx::query(
            "SELECT chunk_sha256 FROM snapshot_file_chunks
             WHERE snapshot_file_id=? ORDER BY ordinal",
        )
        .bind(&file_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| internal())?;

        if chunk_rows.is_empty() {
            entries.push((
                rel,
                DlSource::Blob(crate::blobs::blob_path(&data_dir, &uid, &sha)),
            ));
        } else {
            let paths = chunk_rows
                .iter()
                .map(|c| {
                    let csha: String = c.get("chunk_sha256");
                    crate::chunking::chunk_path(&data_dir, &uid, &csha)
                })
                .collect();
            entries.push((
                rel,
                DlSource::Chunks {
                    paths,
                    size: size as u64,
                },
            ));
        }
    }

    // Build a tar.zst stream in a background task and pipe it to the response body.
    let (tx_bytes, rx_bytes) =
        tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(8);

    tokio::spawn(async move {
        use async_compression::tokio::write::ZstdEncoder;

        struct ChannelWriter {
            tx: tokio::sync::mpsc::Sender<Result<bytes::Bytes, std::io::Error>>,
        }
        impl tokio::io::AsyncWrite for ChannelWriter {
            fn poll_write(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                buf: &[u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                let bytes = bytes::Bytes::copy_from_slice(buf);
                // try_send is fine; if channel is full we briefly back-pressure via Ok(0)?
                // Use blocking_send is not possible in async; use try_send + return Pending if full.
                match self.tx.try_send(Ok(bytes)) {
                    Ok(_) => std::task::Poll::Ready(Ok(buf.len())),
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        // Wake immediately and request retry
                        _cx.waker().wake_by_ref();
                        std::task::Poll::Pending
                    }
                    Err(_) => std::task::Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "receiver dropped",
                    ))),
                }
            }
            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn poll_shutdown(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Ok(()))
            }
        }

        let writer = ChannelWriter {
            tx: tx_bytes.clone(),
        };
        let zstd = ZstdEncoder::new(writer);
        let mut tar = tokio_tar::Builder::new(zstd);

        for (rel, source) in &entries {
            let res = match source {
                DlSource::Blob(blob) => tar.append_path_with_name(blob, rel).await,
                DlSource::Chunks { paths, size } => {
                    // Concatenate the chunk files into one tar entry. Each chunk
                    // is ≤ MAX_CHUNK (a few MiB), so streaming them one at a time
                    // never buffers the whole file in RAM.
                    let stream = futures::stream::iter(paths.clone())
                        .then(|p| async move { tokio::fs::read(&p).await.map(bytes::Bytes::from) })
                        .boxed();
                    let reader = tokio_util::io::StreamReader::new(stream);
                    let mut header = tokio_tar::Header::new_gnu();
                    header.set_size(*size);
                    header.set_mode(0o644);
                    header.set_mtime(0);
                    header.set_entry_type(tokio_tar::EntryType::Regular);
                    tar.append_data(&mut header, rel, reader).await
                }
            };
            if let Err(e) = res {
                warn!(error=%e, path=%rel, "tar build error");
                let _ = tx_bytes.send(Err(e)).await;
                return;
            }
        }

        // Finish tar (writes EOF blocks), then finish zstd
        let zstd_w = match tar.into_inner().await {
            Ok(w) => w,
            Err(e) => {
                let _ = tx_bytes.send(Err(e)).await;
                return;
            }
        };
        let mut zstd_w = zstd_w;
        if let Err(e) = tokio::io::AsyncWriteExt::shutdown(&mut zstd_w).await {
            let _ = tx_bytes.send(Err(e)).await;
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx_bytes);
    let body = Body::from_stream(stream);

    // `game_slug`/`label` are user-set and not constrained to ASCII, so feeding
    // them straight into a header value would panic on control chars / non-ASCII
    // (a client could brick its own download by labeling a save with a newline).
    // Sanitize to a header-safe filename and fall back rather than `unwrap`.
    let filename = sanitize_filename(&format!("{}-{}-v{}.tar.zst", game_slug, label, version));
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zstd"));
    let disposition = format!("attachment; filename=\"{}\"", filename);
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );

    Ok((StatusCode::OK, headers, body).into_response())
}

// ─── DELETE /v1/saves/:save_id/snapshots/:version (soft delete) ────────────

pub async fn soft_delete(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();
    if ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "save not found"));
    }

    let snap_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM snapshots
         WHERE save_id=? AND version_num=? AND deleted_at IS NULL",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| internal())?;
    let snap_id = snap_id.ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    // Soft-delete is purely logical now (ADR 0018, eje C): mark deleted_at and
    // audit. The blobs stay on disk with their refcount intact — a trashed
    // snapshot still pins its bytes (and quota) until the trash purge actually
    // decrements the refcounts and GCs blobs that reach 0. No folder to move,
    // no quota change here.
    let mut tx = state.pool.begin().await.map_err(|_| internal())?;

    sqlx::query!(
        "UPDATE snapshots SET deleted_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?",
        snap_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal())?;

    let audit_id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO audit_log (id, user_id, event_type, entity_id)
         VALUES (?,?,'snapshot.deleted',?)",
        audit_id,
        user_id,
        snap_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal())?;

    tx.commit().await.map_err(|_| internal())?;

    Ok(StatusCode::NO_CONTENT)
}

// ─── POST /v1/saves/:save_id/snapshots/:version/restore ─────────────────────

pub async fn restore(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();
    if ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|_| internal())?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "save not found"));
    }

    // A purged snapshot has had its row deleted entirely (its blobs GC'd), so a
    // missing row is the "has been purged / never existed" case. A still-present
    // but soft-deleted row is recoverable: its blobs were never removed, so
    // restore is purely clearing deleted_at (ADR 0018, eje C).
    let snap = sqlx::query(
        "SELECT id, deleted_at FROM snapshots
         WHERE save_id=? AND version_num=?",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| internal())?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    let snap_id: String = snap.get("id");
    let deleted_at: Option<String> = snap.get("deleted_at");
    if deleted_at.is_none() {
        return Err(err(StatusCode::CONFLICT, "snapshot is not deleted"));
    }

    let mut tx = state.pool.begin().await.map_err(|_| internal())?;
    sqlx::query!("UPDATE snapshots SET deleted_at=NULL WHERE id=?", snap_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| internal())?;
    let audit_id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO audit_log (id, user_id, event_type, entity_id)
         VALUES (?,?,'snapshot.restored',?)",
        audit_id,
        user_id,
        snap_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| internal())?;
    tx.commit().await.map_err(|_| internal())?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reduce an arbitrary download filename to a header-safe form. HTTP header
/// values can't carry control chars or a literal `"`, and `game_slug`/`label`
/// are user-controlled, so we map anything outside a conservative printable-
/// ASCII set to `_`. Keeps the download named sensibly without ever producing a
/// value that `HeaderValue::from_str` would reject.
fn sanitize_filename(name: &str) -> String {
    // Trim the *original* first: a name made only of whitespace/control chars
    // must collapse to the fallback. If we mapped first, each disallowed char
    // would become `_` and `trim()` would no longer see it as empty.
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "download.tar.zst".to_string();
    }
    let cleaned: String = trimmed
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | ' ' => c,
            _ => '_',
        })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "download.tar.zst".to_string()
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod filename_tests {
    use super::*;

    #[test]
    fn strips_control_and_non_ascii() {
        assert_eq!(
            sanitize_filename("elden\nring-v3.tar.zst"),
            "elden_ring-v3.tar.zst"
        );
        assert_eq!(sanitize_filename("zelda—save"), "zelda_save");
        assert_eq!(sanitize_filename("ok-name_v1.tar.zst"), "ok-name_v1.tar.zst");
        // A quote would break the quoted filename; it must be neutralized.
        assert_eq!(sanitize_filename("a\"b"), "a_b");
    }

    #[test]
    fn empty_after_sanitizing_falls_back() {
        assert_eq!(sanitize_filename("\n\r\t"), "download.tar.zst");
    }

    #[test]
    fn sanitized_is_always_a_valid_header_value() {
        let nasty = "💀\n\r\"\0game-\u{202e}evil";
        let f = sanitize_filename(nasty);
        let hv = format!("attachment; filename=\"{f}\"");
        assert!(HeaderValue::from_str(&hv).is_ok());
    }
}
