use axum::{
    body::Body,
    extract::{Extension, Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
};
use futures::StreamExt;
use hoard_core::ids::Sha256 as Sha256Hex;
use hoard_core::wire::{Snapshot, SnapshotDetail, SnapshotFile};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::repair_ts;

// ─── Response types ─────────────────────────────────────────────────────────
//
// The shapes live in `hoard_core::wire` (ADR 0021 C.6), shared with the client:
// `Snapshot`, `SnapshotDetail` and `SnapshotFile`.

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

pub(crate) fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

pub(crate) fn internal() -> (StatusCode, Json<serde_json::Value>) {
    err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

/// A 500 that says, **in the log**, what actually went wrong.
///
/// The client still gets the same opaque `{"error":"internal server error"}`,
/// it must not learn about our paths or SQL. The operator gets the cause.
///
/// This exists because the whole upload route used to discard its errors: every
/// fallible call was mapped with a closure that ignored its argument, so a real
/// error ("no space left on device", "permission denied", "database is locked")
/// went straight to the floor. A self-hoster then saw a bare 500, their server
/// log showed *nothing*, and pinning it down took four rounds of questions and
/// two log dumps (2026-08-07). An upload can fail a dozen ways; the server knew
/// which one every single time and threw it away.
///
/// `what` names the step, not the error; the error speaks for itself. Keep the
/// names coarse and stable: they are what an operator greps for.
pub(crate) fn internal_logged<E: std::fmt::Display>(
    what: &'static str,
    e: E,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!(error = %e, step = what, "snapshot upload failed");
    internal()
}

/// 413 for a snapshot that overruns this server's per-snapshot cap.
///
/// Structured like Hoard Cloud's `save_too_large` body so a self-hosted user
/// gets a number to act on instead of a shrug. Before this, self-hosted 413s
/// carried only `{"error": …}`, the client parsed zeroes out of it and fell back
/// to "the server refused it as too large (413)", which is indistinguishable
/// from the 413 an nginx in front returns, and sends the user hunting through
/// proxy configs when the answer was `storage.max_snapshot_size_mb`.
///
/// `received_bytes` is deliberately **not** `actual_bytes`: we abort mid-stream,
/// so all we know is how far we got before bailing. Reporting that as the
/// snapshot's size would be a lie that reads as precision. The client words it
/// as a floor.
///
/// Use [`snapshot_too_large_declared`] where the size **is** known.
pub fn snapshot_too_large(
    limit_bytes: i64,
    received_bytes: i64,
) -> (StatusCode, Json<serde_json::Value>) {
    too_large_body(limit_bytes, Some(received_bytes), None, "stream")
}

/// The same 413 for the content-addressed path, where the manifest declares the
/// version's size up front and nothing has been transmitted yet.
///
/// It exists because sending that figure as `received_bytes` made the client
/// tell a self-hoster "3.6 GB sent before it stopped" for an upload that moved
/// zero bytes: the rejection happens at `cas_init`, before a single blob
/// travels (ago-2026). Same number, opposite meaning: one is a floor of what
/// arrived, the other is exactly how big the save is.
pub fn snapshot_too_large_declared(
    limit_bytes: i64,
    actual_bytes: i64,
) -> (StatusCode, Json<serde_json::Value>) {
    too_large_body(limit_bytes, None, Some(actual_bytes), "cas_init")
}

/// `route` goes in the log because the `target` cannot: this helper lives in
/// `routes::snapshots`, so every rejection it emits is stamped with that module
/// even when `routes::cas` is the one refusing. An operator reading their own
/// logs was pointed at the wrong half of the server.
fn too_large_body(
    limit_bytes: i64,
    received_bytes: Option<i64>,
    actual_bytes: Option<i64>,
    route: &'static str,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!(
        limit_bytes,
        received_bytes,
        actual_bytes,
        route,
        "snapshot rejected: over the per-snapshot size limit (storage.max_snapshot_size_mb)"
    );
    let mut body = serde_json::json!({
        "error": "snapshot exceeds size limit",
        "code": "snapshot_too_large",
        "limit_bytes": limit_bytes.max(0),
    });
    if let Some(n) = received_bytes {
        body["received_bytes"] = serde_json::json!(n.max(0));
    }
    if let Some(n) = actual_bytes {
        body["actual_bytes"] = serde_json::json!(n.max(0));
    }
    (StatusCode::PAYLOAD_TOO_LARGE, Json(body))
}

/// Is this whole-file blob already stored for the user? The `blobs` table is
/// the source of truth (a row exists iff the object is stored and refcounted),
/// so dedup and quota consult it instead of a per-key HEAD against the store,
/// which on the S3 backend would be one network round-trip per file.
pub(crate) async fn blob_in_db(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    sha: &str,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT 1 FROM blobs WHERE user_id=? AND sha256=? LIMIT 1")
            .bind(user_id)
            .bind(sha)
            .fetch_optional(pool)
            .await?
            .is_some(),
    )
}

/// Chunk-store analogue of [`blob_in_db`] (ADR 0019 chunk table).
pub(crate) async fn chunk_in_db(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    sha: &str,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT 1 FROM chunks WHERE user_id=? AND sha256=? LIMIT 1")
            .bind(user_id)
            .bind(sha)
            .fetch_optional(pool)
            .await?
            .is_some(),
    )
}

/// Validate that a relative path stays inside its parent directory.
/// Rejects: absolute paths, "..", empty components, drive prefixes.
pub(crate) fn is_safe_relative_path(p: &str) -> bool {
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

pub(crate) async fn ownership_check(
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
) -> Result<(StatusCode, Json<Snapshot>), (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    let (game_slug, label) = ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged("ownership lookup", e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;

    // Quota check setup
    let (quota, used): (i64, i64) = sqlx::query!(
        "SELECT storage_quota_bytes, storage_used_bytes FROM users WHERE id=?",
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .map(|r| (r.storage_quota_bytes, r.storage_used_bytes))
    .map_err(|e| internal_logged("quota lookup", e))?;

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
        .map_err(|e| internal_logged("creating the upload tmp dir", e))?;

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
        // and feed the same `files` vec the per-file path uses below, so the
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
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        cleanup_tmp();
                        internal_logged("creating a file's parent dir", e)
                    })?;
                }
                let mut out = tokio::fs::File::create(&dest).await.map_err(|e| {
                    cleanup_tmp();
                    internal_logged("creating the uploaded file", e)
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
                        return Err(snapshot_too_large(max_per_snapshot, total_size));
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
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                cleanup_tmp();
                internal_logged("creating a file's parent dir", e)
            })?;
        }

        let mut file = tokio::fs::File::create(&dest).await.map_err(|e| {
            cleanup_tmp();
            internal_logged("creating the uploaded file", e)
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
                return Err(snapshot_too_large(max_per_snapshot, total_size));
            }
            // Quota is checked at commit time against deduplicated bytes: a
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
    // for chunked ones) not already stored for this user. `new_blobs`/`new_chunks`
    // collect exactly the shas needing a physical upload, so the placement pass
    // below stores each once and skips anything already present. Both stores are
    // counted so dedup across versions is reflected in quota exactly once.
    let mut new_blobs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut new_chunks: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_blobs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_chunks: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut newly_stored_bytes: i64 = 0;
    for (i, (_, size, sha)) in files.iter().enumerate() {
        if let Some(plan) = chunk_plans.get(&i) {
            for c in plan {
                if seen_chunks.insert(c.sha256.clone())
                    && !chunk_in_db(&state.pool, &user_id, &c.sha256)
                        .await
                        .map_err(|e| {
                            cleanup_tmp();
                            internal_logged("chunk dedup lookup", e)
                        })?
                {
                    new_chunks.insert(c.sha256.clone());
                    newly_stored_bytes += c.len as i64;
                }
            }
        } else if seen_blobs.insert(sha.clone())
            && !blob_in_db(&state.pool, &user_id, sha).await.map_err(|e| {
                cleanup_tmp();
                internal_logged("blob dedup lookup", e)
            })?
        {
            new_blobs.insert(sha.clone());
            newly_stored_bytes += size;
        }
    }
    if used + newly_stored_bytes > quota {
        cleanup_tmp();
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "storage quota exceeded"));
    }

    // ── Placement pass: bytes first, database second ────────────────────────
    //
    // Every physical write happens here, *before* the transaction opens. It
    // used to be interleaved with the inserts, which meant the SQLite write
    // lock was held for the whole upload to the storage backend, and a rename on
    // the local backend (microseconds), but a full network PUT per object on
    // S3, and minutes when that S3 endpoint is an rclone bridge to a consumer
    // drive. Every other writer on the server would sit behind it and start
    // failing on the 5 s `busy_timeout`. Doing it out here keeps the
    // transaction to what it should be: a few inserts.
    //
    // Safe to reorder because these objects are content-addressed: a key holds
    // one immutable byte sequence, so writing it before the DB knows about it
    // is at worst an orphan (which the rollback below removes), never a wrong
    // value. `new_blobs` / `new_chunks` already say exactly which shas are
    // missing for this user, so nothing already stored is rewritten.
    let store = state.store.clone();
    // What we physically placed this request, so it can be rolled back if the
    // transaction never commits.
    struct Placed {
        key: String,
        sha: String,
        chunk: bool,
    }
    let mut created_blobs: Vec<Placed> = Vec::new();
    // Within-request dedup: a sha appearing in several files is placed once.
    let mut placed_blobs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut placed_chunks: std::collections::HashSet<String> = std::collections::HashSet::new();
    let rollback_blobs = {
        let store = store.clone();
        let pool = state.pool.clone();
        let user_id = user_id.clone();
        move |placed: &[Placed]| {
            let keys: Vec<(String, String, bool)> = placed
                .iter()
                .map(|p| (p.key.clone(), p.sha.clone(), p.chunk))
                .collect();
            let (store, pool, user_id) = (store.clone(), pool.clone(), user_id.clone());
            tokio::spawn(async move {
                for (key, sha, is_chunk) in keys {
                    // Only drop bytes nothing references. Between our placement
                    // and this rollback another request may have committed a
                    // snapshot pointing at the same content-addressed key,
                    // deleting it then would break *their* save. On a DB error
                    // assume referenced: an orphan costs space, a wrong delete
                    // costs data.
                    let referenced = if is_chunk {
                        chunk_in_db(&pool, &user_id, &sha).await.unwrap_or(true)
                    } else {
                        blob_in_db(&pool, &user_id, &sha).await.unwrap_or(true)
                    };
                    if !referenced {
                        let _ = store.delete(&key).await;
                    }
                }
            });
        }
    };

    for (i, (rel_path, _size, sha)) in files.iter().enumerate() {
        if let Some(plan) = chunk_plans.get(&i) {
            let src = tmp_root.join(rel_path);
            for c in plan.iter() {
                if !new_chunks.contains(&c.sha256) || !placed_chunks.insert(c.sha256.clone()) {
                    continue;
                }
                // We can't rename a byte range, so extract the chunk to a
                // staging file under tmp/ first, then hand it to the backend
                // (same-filesystem rename for local, upload for S3).
                let key = crate::store::chunk_key(&user_id, &c.sha256);
                let stage = tmp_root.join("_stage").join(&c.sha256);
                if crate::chunking::place_chunk(&src, c.offset, c.len, &stage)
                    .await
                    .is_err()
                    || store.put_from_file(&key, &stage).await.is_err()
                {
                    warn!(sha = %c.sha256, "chunk placement failed");
                    rollback_blobs(&created_blobs);
                    cleanup_tmp();
                    return Err(internal());
                }
                created_blobs.push(Placed {
                    key,
                    sha: c.sha256.clone(),
                    chunk: true,
                });
            }
            // tmp source is left for cleanup_tmp; chunks were copied out.
            continue;
        }

        if !new_blobs.contains(sha) || !placed_blobs.insert(sha.clone()) {
            continue;
        }
        let key = crate::store::blob_key(&user_id, sha);
        let src = tmp_root.join(rel_path);
        if store.put_from_file(&key, &src).await.is_err() {
            warn!(sha = %sha, "blob placement failed");
            rollback_blobs(&created_blobs);
            cleanup_tmp();
            return Err(internal());
        }
        created_blobs.push(Placed {
            key,
            sha: sha.clone(),
            chunk: false,
        });
    }

    let snapshot_id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await.map_err(|e| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal_logged("opening the commit transaction", e)
    })?;

    let head: i64 = sqlx::query!("SELECT latest_version_num FROM saves WHERE id=?", save_id)
        .fetch_one(&mut *tx)
        .await
        .map(|r| r.latest_version_num)
        .map_err(|e| {
            rollback_blobs(&created_blobs);
            cleanup_tmp();
            internal_logged("reading the save's latest version", e)
        })?;

    // Fast-forward check (the DAG's enforcement). A client that declares a
    // base version which is no longer the head has diverged: another device
    // pushed since it last synced. Reject so the client can pull + merge
    // (keep-both) instead of silently overwriting the other line.
    if let Some(base) = base_version {
        if base != head {
            rollback_blobs(&created_blobs);
            cleanup_tmp();
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "non-fast-forward: another device advanced this save since your base version",
                    "code": "non_fast_forward",
                    "head_version": head,
                    "base_version": base,
                    // Always the route's id: self-hosted never relabels rows.
                    // Sent anyway so the body has one shape across both
                    // deployments (see `cas::non_fast_forward`).
                    "save_id": save_id,
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
    .map_err(|e| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal_logged("recording the snapshot", e)
    })?;

    // From here the transaction only writes rows; the bytes are already in the
    // store (see the placement pass above), so nothing below can block on the
    // network while holding SQLite's single write lock.
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
        // Each chunk is refcounted exactly like a blob, so dedup, GC and quota
        // treat chunks and blobs uniformly. The bytes went to the store in the
        // placement pass; this only records them.
        if let Some(plan) = chunk_plans.get(&i) {
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
    }

    sqlx::query!(
        "UPDATE saves SET latest_version_num=? WHERE id=?",
        new_version,
        save_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal_logged("reading the save's latest version", e)
    })?;

    let new_used = used + newly_stored_bytes;
    sqlx::query!(
        "UPDATE users SET storage_used_bytes=? WHERE id=?",
        new_used,
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal_logged("updating storage accounting", e)
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
    .map_err(|e| {
        rollback_blobs(&created_blobs);
        cleanup_tmp();
        internal_logged("the commit transaction", e)
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

    // Enforce the user's own "max versions per save" cap: trash the oldest
    // non-pinned snapshots beyond it. Off the response path: a failed prune
    // must not fail an upload that already committed.
    {
        let pool = state.pool.clone();
        let uid = user_id.clone();
        let sid = save_id.clone();
        tokio::spawn(async move {
            if let Err(e) = prune_over_version_cap(&pool, &uid, Some(&sid)).await {
                warn!(error = %e, save_id = %sid, "version-cap prune after commit failed");
            }
        });
    }

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

    // What the history row will say. After the commit and never fatal.
    let insight = match crate::insight::record_selfhosted(&state.pool, &save_id, new_version).await
    {
        Ok(i) => i,
        Err(e) => {
            warn!(error = %e, save_id = %save_id, version = new_version, "insight: not recorded");
            None
        }
    };

    Ok((
        StatusCode::CREATED,
        Json(Snapshot {
            id: snapshot_id,
            save_id: None,
            version_num: new_version,
            parent_version,
            device_name,
            notes,
            total_size_bytes: total_size,
            file_count,
            is_pinned: false,
            deleted_at: None,
            created_at: time::OffsetDateTime::now_utc(),
            insight,
        }),
    ))
}

// ─── GET /v1/saves/:save_id/snapshots ───────────────────────────────────────

pub async fn list(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Snapshot>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = user.user_id.to_string();

    if ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged("ownership lookup", e))?
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
                file_count, is_pinned, deleted_at, created_at, insight
         FROM snapshots WHERE save_id=?
         ORDER BY version_num DESC LIMIT ? OFFSET ?"
    } else {
        "SELECT id, version_num, parent_version, device_name, notes, total_size_bytes,
                file_count, is_pinned, deleted_at, created_at, insight
         FROM snapshots WHERE save_id=? AND deleted_at IS NULL
         ORDER BY version_num DESC LIMIT ? OFFSET ?"
    };
    let rows: Vec<Snapshot> = sqlx::query(sql)
        .bind(&save_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| internal_logged("listing snapshot rows", e))?
        .iter()
        .map(|r| Snapshot {
            id: r.get("id"),
            save_id: None,
            version_num: r.get("version_num"),
            parent_version: r.get("parent_version"),
            device_name: r.get("device_name"),
            notes: r.get("notes"),
            total_size_bytes: r.get("total_size_bytes"),
            file_count: r.get("file_count"),
            is_pinned: r.get::<i64, _>("is_pinned") != 0,
            deleted_at: r
                .get::<Option<String>, _>("deleted_at")
                .as_deref()
                .map(repair_ts),
            created_at: repair_ts(&r.get::<String, _>("created_at")),
            insight: crate::insight::parse_stored(r.get::<Option<String>, _>("insight").as_deref()),
        })
        .collect();

    // Same reason as on cloud: versions from before this come out unlabelled,
    // and this is exactly where that shows. They are computed off the response
    // path, capped, and the next load already brings them.
    let pending: Vec<i64> = rows
        .iter()
        .filter(|s| crate::insight::needs_refresh(s.insight.as_ref()))
        .map(|s| s.version_num)
        .take(crate::insight::BACKFILL_PER_LISTING)
        .collect();
    if !pending.is_empty() {
        let pool = state.pool.clone();
        let sid = save_id.clone();
        tokio::spawn(async move {
            for version in pending {
                if let Err(e) = crate::insight::record_selfhosted(&pool, &sid, version).await {
                    tracing::debug!(error = %e, save_id = %sid, version, "insight: backfill failed");
                }
            }
        });
    }

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
        .map_err(|e| internal_logged("ownership lookup", e))?
        .is_none()
    {
        return Err(err(StatusCode::NOT_FOUND, "save not found"));
    }

    let snap = sqlx::query(
        "SELECT id, version_num, parent_version, device_name, notes, total_size_bytes,
                file_count, is_pinned, deleted_at, created_at, insight
         FROM snapshots WHERE save_id=? AND version_num=?",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| internal_logged("reading a snapshot row", e))?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    let snap_id: String = snap.get("id");
    let files = sqlx::query!(
        "SELECT relative_path, size_bytes, sha256 FROM snapshot_files
         WHERE snapshot_id=? ORDER BY relative_path",
        snap_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| internal_logged("listing snapshot rows", e))?
    .into_iter()
    .map(|r| {
        // The sha is computed by the server itself on upload, so an invalid one
        // means a hand-edited DB. It is neither repaired nor skipped here: the
        // file list is what the client uses to restore and verify, and
        // servirla incompleta escribiría un save a medias. Se falla ruidosamente.
        let sha = Sha256Hex::parse(&r.sha256).map_err(|e| {
            tracing::error!(error = %e, path = %r.relative_path,
                "snapshot_files: sha256 corrupto en la DB");
            internal()
        })?;
        Ok(SnapshotFile {
            relative_path: r.relative_path,
            size_bytes: r.size_bytes,
            sha256: Some(sha),
        })
    })
    .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(SnapshotDetail {
        snapshot: Snapshot {
            id: snap.get("id"),
            save_id: None,
            version_num: snap.get("version_num"),
            parent_version: snap.get("parent_version"),
            device_name: snap.get("device_name"),
            notes: snap.get("notes"),
            total_size_bytes: snap.get("total_size_bytes"),
            file_count: snap.get("file_count"),
            is_pinned: snap.get::<i64, _>("is_pinned") != 0,
            deleted_at: snap
                .get::<Option<String>, _>("deleted_at")
                .as_deref()
                .map(repair_ts),
            created_at: repair_ts(&snap.get::<String, _>("created_at")),
            insight: crate::insight::parse_stored(
                snap.get::<Option<String>, _>("insight").as_deref(),
            ),
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
        .map_err(|e| internal_logged("ownership lookup", e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;

    let snap_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM snapshots WHERE save_id=? AND version_num=? AND deleted_at IS NULL",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| internal_logged("reading a snapshot row", e))?;
    let snap_id = snap_id.ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    // A version is just its list of files; reconstruct the tarball from the
    // referenced blobs and/or chunks (ADR 0018 eje C + ADR 0019 Fase 4). No
    // per-version folder exists anymore. A file is reassembled from its ordered
    // chunks when it has snapshot_file_chunks rows, otherwise from its single
    // whole-file blob, transparent to the client, which gets the same tar.zst.
    let file_rows = sqlx::query(
        "SELECT id, relative_path, size_bytes, sha256 FROM snapshot_files
         WHERE snapshot_id=? ORDER BY relative_path",
    )
    .bind(&snap_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| internal_logged("listing snapshot rows", e))?;

    let uid = user_id.clone();

    // How to source one entry's bytes when building the tar, as storage-backend
    // keys (resolved to readable local paths inside the tar-builder task).
    enum DlSource {
        Blob(String),
        Chunks { keys: Vec<String>, size: u64 },
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
        .map_err(|e| internal_logged("listing snapshot rows", e))?;

        if chunk_rows.is_empty() {
            entries.push((rel, DlSource::Blob(crate::store::blob_key(&uid, &sha))));
        } else {
            let keys = chunk_rows
                .iter()
                .map(|c| {
                    let csha: String = c.get("chunk_sha256");
                    crate::store::chunk_key(&uid, &csha)
                })
                .collect();
            entries.push((
                rel,
                DlSource::Chunks {
                    keys,
                    size: size as u64,
                },
            ));
        }
    }

    // A remote backend streams each needed blob/chunk into this per-download
    // spool dir under tmp/ (bounded memory, never the whole save in RAM); the
    // local backend returns the real blob path and spools nothing. Cleaned up
    // when the tar is done either way.
    let store = state.store.clone();
    let spool_dir = state
        .config
        .storage
        .data_dir
        .join("tmp")
        .join(format!("dl-{}", Uuid::new_v4()));

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

        // Spool files created for a remote backend, for the entry being written
        // right now, and they are dropped as soon as that entry is in the tar (see
        // the end of the loop), so peak scratch space is one file, not the whole
        // snapshot. Local refs have cleanup=false and never land here.
        let mut spooled: Vec<PathBuf> = Vec::new();

        // Resolve a storage key to a readable local path (spooling if remote),
        // recording temporaries for cleanup. On error, emit it and unwind.
        macro_rules! resolve {
            ($key:expr) => {
                match store.local_ref($key, &spool_dir).await {
                    Ok(r) => {
                        if r.cleanup {
                            spooled.push(r.path.clone());
                        }
                        r.path
                    }
                    Err(e) => {
                        warn!(error=%e, key=%$key, "blob fetch error");
                        let io = std::io::Error::other(e.to_string());
                        let _ = tx_bytes.send(Err(io)).await;
                        for p in &spooled {
                            let _ = tokio::fs::remove_file(p).await;
                        }
                        let _ = tokio::fs::remove_dir_all(&spool_dir).await;
                        return;
                    }
                }
            };
        }

        for (rel, source) in &entries {
            let res = match source {
                DlSource::Blob(key) => {
                    let path = resolve!(key);
                    tar.append_path_with_name(&path, rel).await
                }
                DlSource::Chunks { keys, size } => {
                    // Concatenate the chunk files into one tar entry. Each chunk
                    // is ≤ MAX_CHUNK (a few MiB), so streaming them one at a time
                    // never buffers the whole file in RAM.
                    //
                    // Each chunk is also fetched *as the tar consumes it* and its
                    // spool file dropped right after, instead of spooling the
                    // file's chunks up front: on a remote backend a 10 GB save
                    // then needs one chunk of scratch space, not 10 GB. On the
                    // local backend `local_ref` hands back the real chunk path
                    // and nothing is copied or deleted.
                    let stream = futures::stream::iter(keys.clone())
                        .then({
                            let store = store.clone();
                            let spool_dir = spool_dir.clone();
                            move |k| {
                                let (store, spool_dir) = (store.clone(), spool_dir.clone());
                                async move {
                                    let r = store
                                        .local_ref(&k, &spool_dir)
                                        .await
                                        .map_err(std::io::Error::other)?;
                                    let bytes = tokio::fs::read(&r.path).await;
                                    if r.cleanup {
                                        let _ = tokio::fs::remove_file(&r.path).await;
                                    }
                                    bytes.map(bytes::Bytes::from)
                                }
                            }
                        })
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
                for p in &spooled {
                    let _ = tokio::fs::remove_file(p).await;
                }
                let _ = tokio::fs::remove_dir_all(&spool_dir).await;
                return;
            }

            // The entry is fully written, so its spooled bytes are dead weight.
            // Freeing them here is what keeps a remote backend from demanding
            // free local disk the size of the entire snapshot to restore it.
            for p in spooled.drain(..) {
                let _ = tokio::fs::remove_file(&p).await;
            }
        }

        // Finish tar (writes EOF blocks), then finish zstd
        let zstd_w = match tar.into_inner().await {
            Ok(w) => w,
            Err(e) => {
                let _ = tx_bytes.send(Err(e)).await;
                for p in &spooled {
                    let _ = tokio::fs::remove_file(p).await;
                }
                let _ = tokio::fs::remove_dir_all(&spool_dir).await;
                return;
            }
        };
        let mut zstd_w = zstd_w;
        if let Err(e) = tokio::io::AsyncWriteExt::shutdown(&mut zstd_w).await {
            let _ = tx_bytes.send(Err(e)).await;
        }

        // Drop spool files now that the tar has consumed them.
        for p in &spooled {
            let _ = tokio::fs::remove_file(p).await;
        }
        let _ = tokio::fs::remove_dir_all(&spool_dir).await;
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx_bytes);
    let body = Body::from_stream(stream);

    // `game_slug`/`label` are user-set and not constrained to ASCII, so feeding
    // them straight into a header value would panic on control chars / non-ASCII
    // (a client could brick its own download by labeling a save with a newline).
    // Sanitize to a header-safe filename and fall back rather than `unwrap`.
    let filename = sanitize_filename(&format!("{}-{}-v{}.tar.zst", game_slug, label, version));
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zstd"),
    );
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
        .map_err(|e| internal_logged("ownership lookup", e))?
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
    .map_err(|e| internal_logged("reading a snapshot row", e))?;
    let snap_id = snap_id.ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    // Soft-delete is purely logical now (ADR 0018, eje C): mark deleted_at and
    // audit. The blobs stay on disk with their refcount intact: a trashed
    // snapshot still pins its bytes (and quota) until the trash purge actually
    // decrements the refcounts and GCs blobs that reach 0. No folder to move,
    // no quota change here.
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| internal_logged("writing to the database", e))?;

    sqlx::query!(
        "UPDATE snapshots SET deleted_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?",
        snap_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| internal_logged("writing to the database", e))?;

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
    .map_err(|e| internal_logged("committing the transaction", e))?;

    tx.commit()
        .await
        .map_err(|e| internal_logged("committing the transaction", e))?;

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
        .map_err(|e| internal_logged("ownership lookup", e))?
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
    .map_err(|e| internal_logged("reading a snapshot row", e))?
    .ok_or_else(|| err(StatusCode::NOT_FOUND, "snapshot not found"))?;

    let snap_id: String = snap.get("id");
    let deleted_at: Option<String> = snap.get("deleted_at");
    if deleted_at.is_none() {
        return Err(err(StatusCode::CONFLICT, "snapshot is not deleted"));
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| internal_logged("writing to the database", e))?;
    sqlx::query!("UPDATE snapshots SET deleted_at=NULL WHERE id=?", snap_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| internal_logged("writing to the database", e))?;
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
    .map_err(|e| internal_logged("committing the transaction", e))?;
    tx.commit()
        .await
        .map_err(|e| internal_logged("committing the transaction", e))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Count how many snapshots a hypothetical cap of `cap` would trash for this
/// user, without touching anything. Same predicate as
/// [`prune_over_version_cap`]: live, not pinned, with `cap`-or-more newer
/// live siblings in the same save. Powers the "are you sure?" preview the
/// panel shows before lowering the cap.
pub(crate) async fn count_over_version_cap(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    cap: i64,
    manual: bool,
) -> anyhow::Result<u64> {
    let cap = cap.max(1);
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM snapshots s
          JOIN saves sv ON sv.id = s.save_id
         WHERE sv.user_id = ?1 AND s.deleted_at IS NULL AND s.is_pinned = 0
           AND (COALESCE(s.notes,'') IN ('manual','pre-restore')) = ?3
           AND (SELECT COUNT(*) FROM snapshots w
                 WHERE w.save_id = s.save_id AND w.deleted_at IS NULL
                   AND (COALESCE(w.notes,'') IN ('manual','pre-restore')) = ?3
                   AND w.version_num > s.version_num) >= ?2",
    )
    .bind(user_id)
    .bind(cap)
    .bind(manual)
    .fetch_one(pool)
    .await?;
    Ok(n.max(0) as u64)
}

/// Enforce the user's "max versions per save" cap (`users.max_versions`,
/// NULL = unlimited): soft-delete the oldest non-pinned live snapshots so at
/// most `cap` live ones remain per save. Same trash semantics as a manual
/// delete, recoverable until `purge_trash` GCs the blobs. `only_save`
/// narrows the pass to one save (the post-commit hook); `None` sweeps every
/// save of the user (after lowering the cap). Runtime queries (not the
/// `query!` macro) so the new column doesn't require regenerating the
/// offline sqlx cache. Returns how many snapshots were trashed.
pub(crate) async fn prune_over_version_cap(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    only_save: Option<&str>,
) -> anyhow::Result<u64> {
    let caps = sqlx::query("SELECT max_versions, max_manual_versions FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    let Some(row) = caps else { return Ok(0) };
    let auto_cap: Option<i64> = row.get("max_versions");
    let manual_cap: Option<i64> = row.get("max_manual_versions");
    if auto_cap.is_none() && manual_cap.is_none() {
        return Ok(0);
    }
    // An unset cap means no limit. It translates to an unreachable number so the
    // query stays a single one rather than branching.
    let auto_cap = auto_cap.map_or(i64::MAX, |c| c.max(1));
    let manual_cap = manual_cap.map_or(i64::MAX, |c| c.max(1));

    let save_ids: Vec<String> = match only_save {
        Some(id) => vec![id.to_string()],
        None => sqlx::query("SELECT id FROM saves WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(pool)
            .await?
            .iter()
            .map(|r| r.get::<String, _>("id"))
            .collect(),
    };

    let mut pruned = 0u64;
    for save_id in save_ids {
        // Victims: live, not pinned, and not among the newest `cap` live
        // snapshots *of its own class* (the most recent one always falls inside
        // that window, so it never ends up in the trash this way).
        //
        // Two separate windows, one per class: sharing a cap would let a session
        // of autosaves take out the copy the user made by hand, which is exactly
        // the one they wanted to keep.
        let victims: Vec<String> = sqlx::query(
            "SELECT id FROM snapshots
             WHERE save_id = ?1 AND deleted_at IS NULL AND is_pinned = 0
               AND version_num NOT IN (
                   SELECT version_num FROM snapshots
                   WHERE save_id = ?1 AND deleted_at IS NULL
                     AND COALESCE(notes,'') NOT IN ('manual','pre-restore')
                   ORDER BY version_num DESC LIMIT ?2
               )
               AND version_num NOT IN (
                   SELECT version_num FROM snapshots
                   WHERE save_id = ?1 AND deleted_at IS NULL
                     AND COALESCE(notes,'') IN ('manual','pre-restore')
                   ORDER BY version_num DESC LIMIT ?3
               )",
        )
        .bind(&save_id)
        .bind(auto_cap)
        .bind(manual_cap)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.get::<String, _>("id"))
        .collect();

        for snap_id in victims {
            let mut tx = pool.begin().await?;
            sqlx::query(
                "UPDATE snapshots SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ? AND deleted_at IS NULL",
            )
            .bind(&snap_id)
            .execute(&mut *tx)
            .await?;
            let audit_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO audit_log (id, user_id, event_type, entity_id)
                 VALUES (?,?,'snapshot.pruned',?)",
            )
            .bind(&audit_id)
            .bind(user_id)
            .bind(&snap_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            pruned += 1;
        }
    }

    if pruned > 0 {
        info!(user = %user_id, pruned, "version cap: trashed snapshots over max_versions");
    }
    Ok(pruned)
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
        assert_eq!(
            sanitize_filename("ok-name_v1.tar.zst"),
            "ok-name_v1.tar.zst"
        );
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

#[cfg(test)]
mod version_cap_tests {
    use super::*;

    async fn mem_pool() -> sqlx::SqlitePool {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .pragma("foreign_keys", "ON");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    /// `notes = NULL` is an automatic copy; `manual` and `pre-restore` are the
    /// two the user asked for. Same vocabulary the commit path writes.
    async fn seed(pool: &sqlx::SqlitePool, notes: &[Option<&str>]) -> String {
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ('u1','ana','x')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO saves (id, user_id, game_slug, latest_version_num) \
             VALUES ('s1','u1','g',?)",
        )
        .bind(notes.len() as i64)
        .execute(pool)
        .await
        .unwrap();
        for (i, note) in notes.iter().enumerate() {
            sqlx::query("INSERT INTO snapshots (id, save_id, version_num, notes) VALUES (?,?,?,?)")
                .bind(format!("snap-{i}"))
                .bind("s1")
                .bind(i as i64 + 1)
                .bind(*note)
                .execute(pool)
                .await
                .unwrap();
        }
        "u1".to_string()
    }

    async fn live_notes(pool: &sqlx::SqlitePool) -> Vec<(i64, Option<String>)> {
        sqlx::query_as(
            "SELECT version_num, notes FROM snapshots \
             WHERE deleted_at IS NULL ORDER BY version_num",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    /// The question "back up now always cuts a version" raises: twenty presses
    /// in a row must not eat the history the user actually wants. They cannot,
    /// because the two classes are counted against separate caps: a manual
    /// copy can only ever displace another manual one.
    #[tokio::test]
    async fn twenty_manual_copies_never_evict_automatic_history() {
        let pool = mem_pool().await;
        let mut notes: Vec<Option<&str>> = vec![None; 10];
        notes.extend(std::iter::repeat_n(Some("manual"), 20));
        let user = seed(&pool, &notes).await;

        sqlx::query("UPDATE users SET max_versions = 10, max_manual_versions = 5 WHERE id = ?")
            .bind(&user)
            .execute(&pool)
            .await
            .unwrap();
        let pruned = prune_over_version_cap(&pool, &user, None).await.unwrap();

        let live = live_notes(&pool).await;
        let autos = live.iter().filter(|(_, n)| n.is_none()).count();
        let manuals = live.iter().filter(|(_, n)| n.is_some()).count();
        assert_eq!(autos, 10, "an automatic copy was evicted by the button");
        assert_eq!(manuals, 5, "the manual budget was not applied");
        assert_eq!(pruned, 15);

        // And the newest manual, the press that just happened and the save's
        // head, is still there.
        assert!(live.iter().any(|(v, _)| *v == 30));
    }

    /// With no cap set (the default: both columns NULL) nothing is ever
    /// evicted, however many times the button is pressed.
    #[tokio::test]
    async fn without_a_cap_the_button_evicts_nothing() {
        let pool = mem_pool().await;
        let mut notes: Vec<Option<&str>> = vec![None; 5];
        notes.extend(std::iter::repeat_n(Some("manual"), 20));
        let user = seed(&pool, &notes).await;

        assert_eq!(prune_over_version_cap(&pool, &user, None).await.unwrap(), 0);
        assert_eq!(live_notes(&pool).await.len(), 25);
    }

    /// The pre-restore safety copy shares the manual budget, so a run of
    /// button presses can push it out. That is the trade the split was written
    /// with (the alternative is a third cap) but it is the one thing in here
    /// worth knowing: with `max_manual_versions` set low, the copy that lets
    /// you undo a restore is not guaranteed to outlive twenty presses.
    #[tokio::test]
    async fn a_pre_restore_copy_competes_with_the_button() {
        let pool = mem_pool().await;
        let mut notes: Vec<Option<&str>> = vec![Some("pre-restore")];
        notes.extend(std::iter::repeat_n(Some("manual"), 5));
        let user = seed(&pool, &notes).await;

        sqlx::query("UPDATE users SET max_manual_versions = 3 WHERE id = ?")
            .bind(&user)
            .execute(&pool)
            .await
            .unwrap();
        prune_over_version_cap(&pool, &user, None).await.unwrap();

        let live = live_notes(&pool).await;
        assert_eq!(live.len(), 3);
        assert!(
            !live
                .iter()
                .any(|(_, n)| n.as_deref() == Some("pre-restore")),
            "documented behaviour changed: the safety copy now survives"
        );
    }
}
