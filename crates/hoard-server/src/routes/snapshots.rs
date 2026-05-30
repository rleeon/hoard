use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

// ─── Response types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub version_num: i64,
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
    const MAX_FILES_PER_SNAPSHOT: usize = 1000;

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

        if files.len() >= MAX_FILES_PER_SNAPSHOT {
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

    // New bytes = distinct shas in this upload not already on disk for this user.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut newly_stored_bytes: i64 = 0;
    for (_, size, sha) in &files {
        if seen.insert(sha.as_str()) && !crate::blobs::blob_path(&data_dir, &user_id, sha).exists() {
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

    let new_version: i64 = sqlx::query!("SELECT latest_version_num FROM saves WHERE id=?", save_id)
        .fetch_one(&mut *tx)
        .await
        .map(|r| r.latest_version_num + 1)
        .map_err(|_| {
            cleanup_tmp();
            internal()
        })?;

    let file_count = files.len() as i64;
    sqlx::query!(
        "INSERT INTO snapshots (id, save_id, version_num, device_name, notes,
                                total_size_bytes, file_count)
         VALUES (?,?,?,?,?,?,?)",
        snapshot_id,
        save_id,
        new_version,
        device_name,
        notes,
        total_size,
        file_count
    )
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

    for (rel_path, size, sha) in &files {
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

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    Ok((
        StatusCode::CREATED,
        Json(SnapshotSummary {
            id: snapshot_id,
            version_num: new_version,
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

    let rows = if q.include_deleted {
        sqlx::query!(
            "SELECT id, version_num, device_name, notes, total_size_bytes, file_count,
                    is_pinned, deleted_at, created_at
             FROM snapshots WHERE save_id=?
             ORDER BY version_num DESC LIMIT ? OFFSET ?",
            save_id,
            limit,
            offset
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| internal())?
        .into_iter()
        .map(|r| SnapshotSummary {
            id: r.id,
            version_num: r.version_num,
            device_name: r.device_name,
            notes: r.notes,
            total_size_bytes: r.total_size_bytes,
            file_count: r.file_count,
            is_pinned: r.is_pinned != 0,
            deleted_at: r.deleted_at,
            created_at: r.created_at,
        })
        .collect()
    } else {
        sqlx::query!(
            "SELECT id, version_num, device_name, notes, total_size_bytes, file_count,
                    is_pinned, deleted_at, created_at
             FROM snapshots WHERE save_id=? AND deleted_at IS NULL
             ORDER BY version_num DESC LIMIT ? OFFSET ?",
            save_id,
            limit,
            offset
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| internal())?
        .into_iter()
        .map(|r| SnapshotSummary {
            id: r.id,
            version_num: r.version_num,
            device_name: r.device_name,
            notes: r.notes,
            total_size_bytes: r.total_size_bytes,
            file_count: r.file_count,
            is_pinned: r.is_pinned != 0,
            deleted_at: r.deleted_at,
            created_at: r.created_at,
        })
        .collect()
    };

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

    let snap = sqlx::query!(
        "SELECT id, version_num, device_name, notes, total_size_bytes, file_count,
                is_pinned, deleted_at, created_at
         FROM snapshots WHERE save_id=? AND version_num=?",
        save_id,
        version
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| internal())?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    let files = sqlx::query!(
        "SELECT relative_path, size_bytes, sha256 FROM snapshot_files
         WHERE snapshot_id=? ORDER BY relative_path",
        snap.id
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
            id: snap.id,
            version_num: snap.version_num,
            device_name: snap.device_name,
            notes: snap.notes,
            total_size_bytes: snap.total_size_bytes,
            file_count: snap.file_count,
            is_pinned: snap.is_pinned != 0,
            deleted_at: snap.deleted_at,
            created_at: snap.created_at,
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
    // referenced blobs (ADR 0018, eje C). No per-version folder exists anymore.
    let file_rows = sqlx::query(
        "SELECT relative_path, sha256 FROM snapshot_files WHERE snapshot_id=? ORDER BY relative_path",
    )
    .bind(&snap_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| internal())?;

    let data_dir = state.config.storage.data_dir.clone();
    let uid = user_id.clone();
    let entries: Vec<(String, PathBuf)> = file_rows
        .into_iter()
        .map(|r| {
            let rel: String = r.get("relative_path");
            let sha: String = r.get("sha256");
            let bp = crate::blobs::blob_path(&data_dir, &uid, &sha);
            (rel, bp)
        })
        .collect();

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

        for (rel, blob) in &entries {
            if let Err(e) = tar.append_path_with_name(blob, rel).await {
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

    let filename = format!("{}-{}-v{}.tar.zst", game_slug, label, version);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/zstd".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
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
