//! Content-addressed upload, self-hosted (`/v1/saves/{id}/cas/*`).
//!
//! # Why it exists
//!
//! Hoard's storage has deduplicated since ADR 0018: every file is stored once per
//! user under its sha256, and a version is nothing but its list of references.
//! What self-hosted did *not* deduplicate was the transfer. The only way in was
//! the multipart at `POST /v1/saves/{id}/snapshots`, which swallows the whole
//! folder on every copy: the server received 3 GB, wrote them into `tmp/`, hashed
//! them, and found it already had 2.99 GB. It threw those away and kept the 10 MB
//! that were new.
//!
//! That costs three things at once: the user's upload bandwidth, the space in
//! `tmp/` for a full copy, and, the one that actually broke, the request body
//! limit. `storage.max_snapshot_size_mb` and any reverse proxy in front see one
//! request the size of the entire save, so a big save came back 413 with no fix
//! short of raising the cap. Hoard Cloud never had that problem because it
//! negotiates content up front; self-hosted was still dragging the old protocol.
//!
//! # The protocol
//!
//! 1. `POST /v1/saves/{id}/cas/init`: the client declares the manifest (path, sha
//!    and size of each file). The server answers with which shas it does not have
//!    and opens a staging area.
//! 2. `PUT /v1/cas/blobs/{upload_id}/{sha}`: one missing blob, one body. The
//!    server writes it into staging, verifying the hash on the way in.
//! 3. `POST /v1/saves/{id}/cas/commit`: the manifest again; the server places the
//!    new blobs, writes the rows and advances the head.
//!
//! # Where it departs from cloud, and why
//!
//! Cloud signs presigned R2 URLs and the client writes straight into the bucket.
//! Not here: ADR 0020 says the self-hosted client never talks to storage, because
//! the backend may be local disk, MinIO, an `rclone serve s3` over OneDrive. The
//! server is always in the middle, so step 2 is a PUT against the server itself.
//!
//! Cloud also reserves the version in the `init` with a pending row
//! (`sha256 = ''`) and confirms it afterwards. Here the `init` writes nothing to
//! the database: it is a query. The manifest travels again in the commit, which is
//! what assigns the version number inside the same transaction that checks the
//! head. An abandoned init leaves no pending row to clean up and no phantom
//! version in the history; all it leaves is bytes in `tmp/`, which
//! `retention.tmp_cleanup_hours` already sweeps.
//!
//! # Chunking is still the server's business
//!
//! A file above `chunking::CHUNK_THRESHOLD` is split into content-defined chunks
//! exactly as in the multipart path (ADR 0019), and for the same reason: a
//! monolithic save that rewrites a few KB per version must not re-store the whole
//! file. The client neither knows nor needs to: it negotiates whole files, and the
//! server decides how it stores them.
//!
//! The other side of that: a chunked file has no row in `blobs`, so asking only
//! that table would call it absent and the client would re-upload it whole every
//! time. [`stored_representation`] also looks at the user's `snapshot_files`, and
//! the commit *copies the chunk list* from the old version instead of asking for
//! the bytes.

use axum::{
    body::Body,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
};
use futures::StreamExt;
use hoard_core::wire::{CasCommit, CasFile, CasInit, CasInitOut, CasMissing, Snapshot};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::snapshots::{
    blob_in_db, chunk_in_db, err, internal, internal_logged, is_safe_relative_path,
    ownership_check, prune_over_version_cap, snapshot_too_large, snapshot_too_large_declared,
};

type ApiError = (StatusCode, Json<serde_json::Value>);

/// Cap on files per version. The same as the multipart's packed mode: there is no
/// handle and no round trip per file here either, so what gets bounded is the size
/// of the transaction, not the cost of the transfer.
const MAX_FILES: usize = 50_000;

/// How the server already holds a given piece of content.
#[derive(Debug, Clone)]
enum Stored {
    /// A whole-file blob, with a row in `blobs`.
    Blob { size_bytes: i64 },
    /// Chunked (ADR 0019). `file_id` is a `snapshot_files` row to copy the
    /// ordered chunk list from.
    Chunks { size_bytes: i64, file_id: String },
}

impl Stored {
    fn size_bytes(&self) -> i64 {
        match self {
            Stored::Blob { size_bytes } | Stored::Chunks { size_bytes, .. } => *size_bytes,
        }
    }
}

/// Does the server already have this sha's bytes for this user, and in what shape?
///
/// It asks `blobs` first (the normal case) and, failing that, looks for one of the
/// user's `snapshot_files` with that sha and chunks. Trashed snapshots are
/// deliberately included: a deleted snapshot still pins its bytes against the
/// quota until the purge frees them, so its content is available and referencing
/// it is correct.
async fn stored_representation(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    sha: &str,
) -> Result<Option<Stored>, sqlx::Error> {
    if let Some(size) =
        sqlx::query_scalar::<_, i64>("SELECT size_bytes FROM blobs WHERE user_id=? AND sha256=?")
            .bind(user_id)
            .bind(sha)
            .fetch_optional(pool)
            .await?
    {
        return Ok(Some(Stored::Blob { size_bytes: size }));
    }

    let row = sqlx::query(
        "SELECT sf.id AS id, sf.size_bytes AS size_bytes
           FROM snapshot_files sf
           JOIN snapshots s ON s.id = sf.snapshot_id
           JOIN saves sv ON sv.id = s.save_id
          WHERE sv.user_id = ? AND sf.sha256 = ?
            AND EXISTS (SELECT 1 FROM snapshot_file_chunks c WHERE c.snapshot_file_id = sf.id)
          LIMIT 1",
    )
    .bind(user_id)
    .bind(sha)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| Stored::Chunks {
        size_bytes: r.get("size_bytes"),
        file_id: r.get("id"),
    }))
}

/// An upload's staging folder. It sits flush against `tmp/` so
/// `cleanup::purge_tmp`'s age sweep, which only looks at first-level entries,
/// picks up abandoned uploads.
fn staging_dir(data_dir: &std::path::Path, upload_id: &str) -> PathBuf {
    data_dir.join("tmp").join(format!("cas-{upload_id}"))
}

/// A staging area's owner, written when it is opened.
///
/// The `upload_id` is a v4 UUID minted by the server, so guessing it is not
/// realistic, but "not realistic" is not an access control. With the owner on
/// disk, uploading into somebody else's upload is impossible by construction and
/// does not depend on nobody leaking an id.
fn owner_file(dir: &std::path::Path) -> PathBuf {
    dir.join("owner")
}

/// Validates an `upload_id` *before* it goes into a file path. UUIDs only: no
/// `..`, no separators, no surprises.
fn valid_upload_id(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

/// Canonical lowercase-hex sha256. It gets interpolated into storage keys and
/// staging paths, so it is checked first.
fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Manifest to unique shas, with each one's declared size. The same content
/// repeated at several paths is uploaded once.
fn unique_shas(files: &[CasFile]) -> Vec<(String, i64)> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for f in files {
        if seen.insert(f.sha256.as_str()) {
            out.push((f.sha256.as_str().to_string(), f.size_bytes.max(0)));
        }
    }
    out
}

/// Checks that apply to both init and commit: a non-empty manifest, within the
/// file cap, with safe paths.
fn validate_manifest(files: &[CasFile]) -> Result<(), ApiError> {
    if files.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "empty manifest"));
    }
    if files.len() > MAX_FILES {
        return Err(err(StatusCode::BAD_REQUEST, "too many files in snapshot"));
    }
    if let Some(bad) = files
        .iter()
        .find(|f| !is_safe_relative_path(&f.relative_path))
    {
        warn!(path = %bad.relative_path, "cas: unsafe relative path in manifest");
        return Err(err(StatusCode::BAD_REQUEST, "unsafe file path"));
    }
    Ok(())
}

// ---- POST /v1/saves/:save_id/cas/init

pub async fn init(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Json(body): Json<CasInit>,
) -> Result<Json<CasInitOut>, ApiError> {
    let user_id = user.user_id.to_string();
    ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged("ownership lookup", e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;

    validate_manifest(&body.files)?;

    // The per-version cap is measured against the save's *logical* size, not
    // against what will be transferred. It is the same promise the multipart makes
    // and the one the operator believes they are configuring: "do not store
    // versions bigger than X". That the bytes now travel in pieces does not change
    // what the version occupies.
    let logical: i64 = body.files.iter().map(|f| f.size_bytes.max(0)).sum();
    let max_per_snapshot = (state.config.storage.max_snapshot_size_mb as i64) * 1024 * 1024;
    if logical > max_per_snapshot {
        // Here the real size is known, because the manifest declares it, unlike
        // the multipart, which aborts mid-transfer and can only say how far it
        // got. It goes out as `actual_bytes` for exactly that reason: not a byte
        // has left here yet, so reporting it as "received" made the client say
        // "3.6 GB sent before stopping".
        return Err(snapshot_too_large_declared(max_per_snapshot, logical));
    }

    // Reject the non-fast-forward *before* a byte moves. In the multipart this
    // check arrives after the whole save has been uploaded; here it is the first
    // thing, which is half the reason for having an `init` at all.
    let head: i64 = sqlx::query_scalar("SELECT latest_version_num FROM saves WHERE id=?")
        .bind(&save_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| internal_logged("reading the save's latest version", e))?;
    if let Some(base) = body.base_version {
        // A base that does not match the head is rejected so a push cannot bury
        // a version it never saw. But a manifest that brings that version
        // *entirely* cannot bury it: its content is still there, file by file, in
        // the one about to be written. It is the same judgement the agent makes
        // when it reconciles and finds its folder already contains the head, made
        // here with the manifest already in the request, for clients that cannot
        // do it themselves. Reading the head out of a 409 body is from aug-2026,
        // and before that a rejection left them knowing they had diverged but not
        // from what.
        if base != head && !manifest_covers_head(&state.pool, &save_id, head, &body.files).await? {
            return Err(non_fast_forward(&save_id, head, base));
        }
        if base != head {
            tracing::warn!(
                %save_id,
                head_version = head,
                base_version = base,
                "cas init: la base diverge pero el manifiesto trae la cabeza entera — se deja pasar"
            );
        }
    }

    // What is missing. The sizes recorded are the ones the client declares; they
    // are only used for the progress bar and for the quota warning below. The real
    // charge is made by the commit against whatever actually landed.
    let mut missing = Vec::new();
    let mut missing_bytes: i64 = 0;
    for (sha, size) in unique_shas(&body.files) {
        if !valid_sha256(&sha) {
            return Err(err(StatusCode::BAD_REQUEST, "invalid sha256 in manifest"));
        }
        if stored_representation(&state.pool, &user_id, &sha)
            .await
            .map_err(|e| internal_logged("blob dedup lookup", e))?
            .is_none()
        {
            missing_bytes += size;
            missing.push(CasMissing {
                sha256: body
                    .files
                    .iter()
                    .find(|f| f.sha256.as_str() == sha)
                    .map(|f| f.sha256.clone())
                    .expect("sha came from this manifest"),
                size_bytes: size,
            });
        }
    }

    // Early quota warning, using the declared sizes. It is not the gate (that is
    // in the commit, against the real bytes) but it stops somebody uploading 8 GB
    // only to have them refused at the end.
    let (quota, used): (i64, i64) =
        sqlx::query_as("SELECT storage_quota_bytes, storage_used_bytes FROM users WHERE id=?")
            .bind(&user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| internal_logged("quota lookup", e))?;
    if used + missing_bytes > quota {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "storage quota exceeded"));
    }

    let upload_id = Uuid::new_v4().to_string();
    let dir = staging_dir(&state.config.storage.data_dir, &upload_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| internal_logged("creating the upload tmp dir", e))?;
    tokio::fs::write(owner_file(&dir), &user_id)
        .await
        .map_err(|e| internal_logged("creating the upload tmp dir", e))?;

    info!(
        user = %user.username,
        save_id = %save_id,
        files = body.files.len(),
        missing = missing.len(),
        logical_bytes = logical,
        missing_bytes,
        "cas init"
    );

    Ok(Json(CasInitOut {
        upload_id,
        version_num: head + 1,
        missing,
        missing_bytes,
    }))
}

/// Does this push bring everything the head has?
///
/// A *strict* superset: it has to bring the whole head *and* something of its own.
/// Dropping a file the head has is precisely the burial the 409 exists to prevent,
/// and it still gets one. Bringing the head and nothing else is a client with no
/// new content to write: the agent settles onto the head rather than re-uploading
/// it, and minting an identical version here would only fatten the history of a
/// machine whose only loss was its place in the queue. A head with no files (a
/// half-built version, or one from before content-addressing) concedes nothing
/// either: there is nothing to compare against.
async fn manifest_covers_head(
    pool: &sqlx::SqlitePool,
    save_id: &str,
    head: i64,
    files: &[CasFile],
) -> Result<bool, ApiError> {
    if head <= 0 {
        return Ok(false);
    }
    let head_files: Vec<(String, String)> = sqlx::query_as(
        "SELECT sf.relative_path, sf.sha256
           FROM snapshot_files sf
           JOIN snapshots s ON s.id = sf.snapshot_id
          WHERE s.save_id = ? AND s.version_num = ?",
    )
    .bind(save_id)
    .bind(head)
    .fetch_all(pool)
    .await
    .map_err(|e| internal_logged("reading the head's manifest", e))?;
    if head_files.is_empty() {
        return Ok(false);
    }
    let incoming: std::collections::HashSet<(&str, &str)> = files
        .iter()
        .map(|f| (f.relative_path.as_str(), f.sha256.as_str()))
        .collect();
    let covers_all = head_files
        .iter()
        .all(|(path, sha)| incoming.contains(&(path.as_str(), sha.as_str())));
    Ok(covers_all && incoming.len() > head_files.len())
}

/// The divergence 409. It carries `save_id` even though here it is always the id
/// the client asked for (self-hosted never relabels rows, and the route already
/// names one) so the body has a single shape across both deployments. On Cloud
/// that field is the canonical row the push was rejected against, which may not be
/// the one the client thought it was writing to, and the client parses one
/// structure instead of branching on which server answered.
fn non_fast_forward(save_id: &str, head: i64, base: i64) -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": "non-fast-forward: another device advanced this save since your base version",
            "code": "non_fast_forward",
            "head_version": head,
            "base_version": base,
            "save_id": save_id,
        })),
    )
}

// ---- PUT /v1/cas/blobs/:upload_id/:sha256

/// How much body we swallow as a courtesy before answering an error, while the
/// client is still writing.
///
/// This exists because answering and closing without reading is not free: hyper
/// closes the socket with data unconsumed, TCP sends RST, and **Windows throws
/// away the response already sitting in its receive buffer**. The client never
/// gets to see the 404 or the 413, only an `error writing a body to connection`
/// (os error 10053/10054) that says nothing. Half of issue #17.
///
/// Capped, because the body can be gigabytes and swallowing all of it just to
/// Capped, because the body can be gigabytes and swallowing all of it just to be
/// able to say "no" is exactly the work the error was trying to avoid. Past the
/// cap we stop and the client gets the same reset as before: no worse than it was.
const MAX_DRAIN_BYTES: u64 = 8 * 1024 * 1024;

/// Empty whatever is left of the body (up to [`MAX_DRAIN_BYTES`]) and return the
/// error unchanged, so the response leaves through a socket the client can
/// still read. See [`MAX_DRAIN_BYTES`].
async fn drain_then(
    stream: &mut axum::body::BodyDataStream,
    error: ApiError,
    already_read: u64,
) -> ApiError {
    let mut drained = already_read;
    while drained < MAX_DRAIN_BYTES {
        match stream.next().await {
            Some(Ok(chunk)) => drained += chunk.len() as u64,
            // A body that dies on its own is no longer in the way: nothing to
            // drain.
            Some(Err(_)) | None => break,
        }
    }
    error
}

/// One missing blob. The body is the raw bytes; they get written into staging,
/// hashed on the way, and if the sha does not match the one the URL promises the
/// file is deleted and the request rejected.
///
/// Verifying here rather than in the commit is not politeness: the client hashes
/// the file and *then* reads it again to send it, and between the two reads the
/// game may have rotated the save. If nobody checks, the server ends up storing
/// new bytes under the old bytes' sha, a blob whose content is not what its name
/// promises, which on restore hands back a different save with nothing
/// complaining. That is the silent corruption of aug-2026; the client already
/// defends itself by hashing what leaves the socket, and this is the other half.
pub async fn upload_blob(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path((upload_id, sha)): Path<(String, String)>,
    body: Body,
) -> Result<StatusCode, ApiError> {
    let user_id = user.user_id.to_string();
    // The body is taken as a stream before the first validation: every error
    // from here on has to empty it before answering, or the client never gets
    // to read the response. See `drain_then`.
    let mut stream = body.into_data_stream();

    if !valid_upload_id(&upload_id) {
        let e = err(StatusCode::BAD_REQUEST, "invalid upload id");
        return Err(drain_then(&mut stream, e, 0).await);
    }
    if !valid_sha256(&sha) {
        let e = err(StatusCode::BAD_REQUEST, "invalid sha256");
        return Err(drain_then(&mut stream, e, 0).await);
    }

    let dir = staging_dir(&state.config.storage.data_dir, &upload_id);
    let owner = match tokio::fs::read_to_string(owner_file(&dir)).await {
        Ok(o) => o,
        Err(_) => {
            let e = err(StatusCode::NOT_FOUND, "upload not found or expired");
            return Err(drain_then(&mut stream, e, 0).await);
        }
    };
    if owner != user_id {
        // The same body as "it does not exist": somebody who is not the owner
        // must not be able to tell a foreign id from an invented one.
        let e = err(StatusCode::NOT_FOUND, "upload not found or expired");
        return Err(drain_then(&mut stream, e, 0).await);
    }

    let dest = dir.join(&sha);
    let max_per_blob = (state.config.storage.max_snapshot_size_mb as i64) * 1024 * 1024;

    let mut file = match tokio::fs::File::create(&dest).await {
        Ok(f) => f,
        Err(e) => {
            let e = internal_logged("creating the uploaded file", e);
            return Err(drain_then(&mut stream, e, 0).await);
        }
    };
    let mut hasher = Sha256::new();
    let mut size: i64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "cas blob stream error");
                let _ = tokio::fs::remove_file(&dest).await;
                return Err(err(StatusCode::BAD_REQUEST, "stream error"));
            }
        };
        size += chunk.len() as i64;
        if size > max_per_blob {
            let _ = tokio::fs::remove_file(&dest).await;
            // What we already read counts against the drain cap: a blob over
            // the server's limit is precisely the one not worth swallowing
            // whole just to reject it politely.
            let e = snapshot_too_large(max_per_blob, size);
            return Err(drain_then(&mut stream, e, size.max(0) as u64).await);
        }
        hasher.update(&chunk);
        if let Err(e) = file.write_all(&chunk).await {
            warn!(error = %e, "cas blob write error");
            let _ = tokio::fs::remove_file(&dest).await;
            let e = internal_logged("writing the uploaded blob", e);
            return Err(drain_then(&mut stream, e, size.max(0) as u64).await);
        }
    }
    if let Err(e) = file.flush().await {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(internal_logged("writing the uploaded blob", e));
    }
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if actual != sha {
        warn!(
            declared = %sha,
            actual = %actual,
            bytes = size,
            "cas: uploaded blob does not hash to the sha it was announced under — rejected"
        );
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(err(
            StatusCode::BAD_REQUEST,
            "uploaded bytes do not match the declared sha256",
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---- POST /v1/saves/:save_id/cas/commit

/// An object placed into the store during this request, so it can be undone if
/// the transaction never commits.
struct Placed {
    key: String,
    sha: String,
    chunk: bool,
}

pub async fn commit(
    State(state): State<Arc<ServerState>>,
    Extension(user): Extension<AuthUser>,
    Path(save_id): Path<String>,
    Json(body): Json<CasCommit>,
) -> Result<(StatusCode, Json<Snapshot>), ApiError> {
    let user_id = user.user_id.to_string();
    ownership_check(&state.pool, &save_id, &user_id)
        .await
        .map_err(|e| internal_logged("ownership lookup", e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "save not found"))?;
    validate_manifest(&body.files)?;
    if !valid_upload_id(&body.upload_id) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid upload id"));
    }

    let dir = staging_dir(&state.config.storage.data_dir, &body.upload_id);
    let owner = tokio::fs::read_to_string(owner_file(&dir))
        .await
        .map_err(|_| err(StatusCode::NOT_FOUND, "upload not found or expired"))?;
    if owner != user_id {
        return Err(err(StatusCode::NOT_FOUND, "upload not found or expired"));
    }
    let cleanup_staging = || {
        let p = dir.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::remove_dir_all(&p).await;
        });
    };

    // ---- resolve each sha: was it just uploaded, or did we already have it?
    //
    // `staged` are the ones to store now; `reused` the ones already there that
    // only need referencing. Anything else is a manifest that does not match what
    // was uploaded, and gets rejected before the store is touched.
    let mut staged: HashMap<String, (PathBuf, i64)> = HashMap::new();
    let mut reused: HashMap<String, Stored> = HashMap::new();
    for (sha, _declared) in unique_shas(&body.files) {
        if !valid_sha256(&sha) {
            cleanup_staging();
            return Err(err(StatusCode::BAD_REQUEST, "invalid sha256 in manifest"));
        }
        let path = dir.join(&sha);
        match tokio::fs::metadata(&path).await {
            Ok(meta) => {
                // The size that counts is the file's on disk, never the one the
                // client declared: otherwise declaring 1 byte would be enough to
                // slip past the quota and upload a gigabyte.
                staged.insert(sha, (path, meta.len() as i64));
            }
            Err(_) => {
                let Some(stored) = stored_representation(&state.pool, &user_id, &sha)
                    .await
                    .map_err(|e| {
                        cleanup_staging();
                        internal_logged("blob dedup lookup", e)
                    })?
                else {
                    cleanup_staging();
                    warn!(sha = %sha, save_id = %save_id, "cas commit: manifest references a blob that was never uploaded");
                    return Err(err(
                        StatusCode::BAD_REQUEST,
                        "manifest references a blob that was not uploaded",
                    ));
                };
                reused.insert(sha, stored);
            }
        }
    }

    // ---- chunk whatever deserves it (ADR 0019)
    // Planning only (hashing), no writing: if the quota refuses further down there
    // is nothing to undo.
    let mut chunk_plans: HashMap<String, Vec<crate::chunking::ChunkPlan>> = HashMap::new();
    for (sha, (path, size)) in &staged {
        if *size as u64 > crate::chunking::CHUNK_THRESHOLD {
            match crate::chunking::plan_chunks(path).await {
                Ok(plan) => {
                    chunk_plans.insert(sha.clone(), plan);
                }
                Err(e) => {
                    cleanup_staging();
                    return Err(internal_logged("chunk planning", e));
                }
            }
        }
    }

    // ---- genuinely new bytes
    // A chunked file only costs the chunks the user did not already have, so two
    // versions of a monolithic save that changes little cost little, even though
    // the whole file travelled.
    let mut new_bytes: i64 = 0;
    let mut new_chunks: HashSet<String> = HashSet::new();
    for (sha, (_, size)) in &staged {
        if let Some(plan) = chunk_plans.get(sha) {
            for c in plan {
                if new_chunks.contains(&c.sha256) {
                    continue;
                }
                if !chunk_in_db(&state.pool, &user_id, &c.sha256)
                    .await
                    .map_err(|e| {
                        cleanup_staging();
                        internal_logged("chunk dedup lookup", e)
                    })?
                {
                    new_chunks.insert(c.sha256.clone());
                    new_bytes += c.len as i64;
                }
            }
        } else {
            new_bytes += size;
        }
    }

    let (quota, used): (i64, i64) =
        sqlx::query_as("SELECT storage_quota_bytes, storage_used_bytes FROM users WHERE id=?")
            .bind(&user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| {
                cleanup_staging();
                internal_logged("quota lookup", e)
            })?;
    if used + new_bytes > quota {
        cleanup_staging();
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "storage quota exceeded"));
    }

    // The version's size is set by the server, summing what it knows about each
    // piece of content (the staged file, or the row that already existed), not by
    // the client's manifest.
    let mut size_by_sha: HashMap<&str, i64> = HashMap::new();
    for (sha, (_, size)) in &staged {
        size_by_sha.insert(sha.as_str(), *size);
    }
    for (sha, stored) in &reused {
        size_by_sha.insert(sha.as_str(), stored.size_bytes());
    }
    let total_size: i64 = body
        .files
        .iter()
        .map(|f| size_by_sha.get(f.sha256.as_str()).copied().unwrap_or(0))
        .sum();
    let max_per_snapshot = (state.config.storage.max_snapshot_size_mb as i64) * 1024 * 1024;
    if total_size > max_per_snapshot {
        cleanup_staging();
        return Err(snapshot_too_large(max_per_snapshot, total_size));
    }

    // ---- placement: bytes first, database after
    // As in the multipart, and for the same reason: a `put_from_file` against S3
    // is a network upload, and doing it with the transaction open leaves the whole
    // server waiting on SQLite's write lock.
    let store = state.store.clone();
    let mut placed: Vec<Placed> = Vec::new();
    let mut placed_chunks: HashSet<String> = HashSet::new();
    let rollback = {
        let store = store.clone();
        let pool = state.pool.clone();
        let user_id = user_id.clone();
        move |done: &[Placed]| {
            let keys: Vec<(String, String, bool)> = done
                .iter()
                .map(|p| (p.key.clone(), p.sha.clone(), p.chunk))
                .collect();
            let (store, pool, user_id) = (store.clone(), pool.clone(), user_id.clone());
            tokio::spawn(async move {
                for (key, sha, is_chunk) in keys {
                    // Only what nothing references gets deleted: between the
                    // placement and this rollback another request may have
                    // committed a version pointing at the same key. On a database
                    // error we assume it is referenced: an orphan costs space, an
                    // over-eager delete costs data.
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

    for (sha, (path, _size)) in &staged {
        if let Some(plan) = chunk_plans.get(sha) {
            for c in plan.iter() {
                if !new_chunks.contains(&c.sha256) || !placed_chunks.insert(c.sha256.clone()) {
                    continue;
                }
                let key = crate::store::chunk_key(&user_id, &c.sha256);
                let stage = dir.join("_stage").join(&c.sha256);
                if let Some(parent) = stage.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if crate::chunking::place_chunk(path, c.offset, c.len, &stage)
                    .await
                    .is_err()
                    || store.put_from_file(&key, &stage).await.is_err()
                {
                    warn!(sha = %c.sha256, "cas: chunk placement failed");
                    rollback(&placed);
                    cleanup_staging();
                    return Err(internal());
                }
                placed.push(Placed {
                    key,
                    sha: c.sha256.clone(),
                    chunk: true,
                });
            }
            continue;
        }
        let key = crate::store::blob_key(&user_id, sha);
        if store.put_from_file(&key, path).await.is_err() {
            warn!(sha = %sha, "cas: blob placement failed");
            rollback(&placed);
            cleanup_staging();
            return Err(internal());
        }
        placed.push(Placed {
            key,
            sha: sha.clone(),
            chunk: false,
        });
    }

    // ---- transaction: rows only
    let snapshot_id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await.map_err(|e| {
        rollback(&placed);
        cleanup_staging();
        internal_logged("opening the commit transaction", e)
    })?;

    let head: i64 = sqlx::query_scalar("SELECT latest_version_num FROM saves WHERE id=?")
        .bind(&save_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            rollback(&placed);
            cleanup_staging();
            internal_logged("reading the save's latest version", e)
        })?;
    // The init already looked, but minutes can pass between init and commit and
    // another machine may have pushed. This is the check that counts.
    if let Some(base) = body.base_version {
        if base != head {
            rollback(&placed);
            cleanup_staging();
            return Err(non_fast_forward(&save_id, head, base));
        }
    }
    let new_version = head + 1;
    let parent_version: Option<i64> = (head > 0).then_some(head);
    let file_count = body.files.len() as i64;

    let fail = |e: sqlx::Error, step: &'static str| internal_logged(step, e);

    sqlx::query(
        "INSERT INTO snapshots (id, save_id, version_num, device_name, notes,
                                total_size_bytes, file_count, parent_version)
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(&snapshot_id)
    .bind(&save_id)
    .bind(new_version)
    .bind(&body.device_name)
    .bind(&body.notes)
    .bind(total_size)
    .bind(file_count)
    .bind(parent_version)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        rollback(&placed);
        cleanup_staging();
        fail(e, "recording the snapshot")
    })?;

    for f in &body.files {
        let file_id = Uuid::new_v4().to_string();
        let sha = f.sha256.as_str();
        let size = size_by_sha.get(sha).copied().unwrap_or(0);
        sqlx::query(
            "INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256, modified_at)
             VALUES (?,?,?,?,?,?)",
        )
        .bind(&file_id)
        .bind(&snapshot_id)
        .bind(&f.relative_path)
        .bind(size)
        .bind(sha)
        .bind(f.modified_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            rollback(&placed);
            cleanup_staging();
            fail(e, "recording a snapshot file")
        })?;

        // Chunks: the ones just planned, or the ones copied from the version that
        // already had this content. Either way it is referenced chunk by chunk,
        // since a file can repeat the same chunk and every appearance counts.
        let chunks: Vec<(String, i64)> = if let Some(plan) = chunk_plans.get(sha) {
            plan.iter()
                .map(|c| (c.sha256.clone(), c.len as i64))
                .collect()
        } else if let Some(Stored::Chunks { file_id: src, .. }) = reused.get(sha) {
            sqlx::query(
                "SELECT c.chunk_sha256 AS sha, COALESCE(k.size_bytes, 0) AS size
                   FROM snapshot_file_chunks c
                   LEFT JOIN chunks k ON k.user_id = ? AND k.sha256 = c.chunk_sha256
                  WHERE c.snapshot_file_id = ?
                  ORDER BY c.ordinal",
            )
            .bind(&user_id)
            .bind(src)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| {
                rollback(&placed);
                cleanup_staging();
                fail(e, "copying a chunk list")
            })?
            .into_iter()
            .map(|r| (r.get::<String, _>("sha"), r.get::<i64, _>("size")))
            .collect()
        } else {
            Vec::new()
        };

        if chunks.is_empty() {
            sqlx::query(
                "INSERT INTO blobs (user_id, sha256, size_bytes, refcount)
                 VALUES (?,?,?,1)
                 ON CONFLICT(user_id, sha256) DO UPDATE SET refcount = refcount + 1",
            )
            .bind(&user_id)
            .bind(sha)
            .bind(size)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                rollback(&placed);
                cleanup_staging();
                fail(e, "reference-counting a blob")
            })?;
            continue;
        }

        for (ordinal, (csha, csize)) in chunks.iter().enumerate() {
            sqlx::query(
                "INSERT INTO snapshot_file_chunks (snapshot_file_id, ordinal, chunk_sha256)
                 VALUES (?,?,?)",
            )
            .bind(&file_id)
            .bind(ordinal as i64)
            .bind(csha)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                rollback(&placed);
                cleanup_staging();
                fail(e, "recording a file's chunks")
            })?;
            sqlx::query(
                "INSERT INTO chunks (user_id, sha256, size_bytes, refcount)
                 VALUES (?,?,?,1)
                 ON CONFLICT(user_id, sha256) DO UPDATE SET refcount = refcount + 1",
            )
            .bind(&user_id)
            .bind(csha)
            .bind(csize)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                rollback(&placed);
                cleanup_staging();
                fail(e, "reference-counting a chunk")
            })?;
        }
    }

    sqlx::query("UPDATE saves SET latest_version_num=? WHERE id=?")
        .bind(new_version)
        .bind(&save_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            rollback(&placed);
            cleanup_staging();
            fail(e, "advancing the save head")
        })?;

    let new_used = used + new_bytes;
    sqlx::query("UPDATE users SET storage_used_bytes=? WHERE id=?")
        .bind(new_used)
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            rollback(&placed);
            cleanup_staging();
            fail(e, "updating storage accounting")
        })?;

    let metadata = serde_json::json!({
        "save_id": save_id,
        "version_num": new_version,
        "files": file_count,
        "bytes": total_size,
        "new_bytes": new_bytes,
        "transport": "cas",
    })
    .to_string();
    sqlx::query(
        "INSERT INTO audit_log (id, user_id, event_type, entity_id, metadata)
         VALUES (?,?,'snapshot.created',?,?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user_id)
    .bind(&snapshot_id)
    .bind(&metadata)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        rollback(&placed);
        cleanup_staging();
        fail(e, "the commit transaction")
    })?;

    if let Err(e) = tx.commit().await {
        warn!(error = %e, "cas: transaction commit failed");
        rollback(&placed);
        cleanup_staging();
        return Err(internal());
    }
    cleanup_staging();

    info!(
        user = %user.username,
        save_id = %save_id,
        version = new_version,
        files = file_count,
        bytes = total_size,
        new_bytes,
        uploaded_blobs = staged.len(),
        reused_blobs = reused.len(),
        "cas commit"
    );

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

    state.events.publish(
        user.user_id,
        crate::routes::events::SaveEvent {
            save_id: save_id.clone(),
            version_num: new_version,
        },
    );

    // What the history row will say. After the commit and never fatal: the
    // version is stored, and a row that fails to get a label is cosmetic.
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
            device_name: body.device_name,
            notes: body.notes,
            total_size_bytes: total_size,
            file_count,
            is_pinned: false,
            deleted_at: None,
            created_at: time::OffsetDateTime::now_utc(),
            insight,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::ids::Sha256 as Sha256Hex;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;

    fn sha(prefix: &str) -> String {
        let mut s = prefix.to_string();
        while s.len() < 64 {
            s.push('0');
        }
        s
    }

    fn file(path: &str, s: &str, size: i64) -> CasFile {
        CasFile {
            relative_path: path.into(),
            sha256: Sha256Hex::parse(s).unwrap(),
            size_bytes: size,
            modified_at: None,
        }
    }

    async fn mem_pool() -> SqlitePool {
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

    #[test]
    fn upload_ids_and_shas_are_gated_before_they_reach_a_path() {
        // Whatever gets interpolated into a path is validated first. Without
        // this, an `upload_id` with `..` writes outside `tmp/`.
        assert!(valid_upload_id(&Uuid::new_v4().to_string()));
        assert!(!valid_upload_id("../../etc"));
        assert!(!valid_upload_id(""));

        assert!(valid_sha256(&sha("ab")));
        assert!(!valid_sha256("../x"));
        assert!(!valid_sha256(&sha("AB")), "sólo hexadecimal en minúsculas");
        assert!(!valid_sha256("abc"), "longitud exacta");
    }

    #[test]
    fn a_repeated_content_is_negotiated_once() {
        let a = sha("aa");
        let b = sha("bb");
        let files = vec![
            file("save", &a, 10),
            file("save.bak", &a, 10),
            file("other", &b, 20),
        ];
        let u = unique_shas(&files);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0], (a, 10));
        assert_eq!(u[1], (b, 20));
    }

    #[test]
    fn manifests_that_could_write_outside_the_snapshot_are_refused() {
        let s = sha("aa");
        assert!(validate_manifest(&[]).is_err(), "manifiesto vacío");
        assert!(validate_manifest(&[file("../escape", &s, 1)]).is_err());
        assert!(validate_manifest(&[file("/abs", &s, 1)]).is_err());
        assert!(validate_manifest(&[file("saves/a.sav", &s, 1)]).is_ok());
    }

    /// A blob with a row in `blobs` is recognised, and so is a chunked one even
    /// with no `blobs` row. That is the case which, if forgotten, would make a
    /// monolithic save upload whole on every copy.
    #[tokio::test]
    async fn stored_content_is_recognised_as_blob_or_as_chunks() {
        let pool = mem_pool().await;
        let whole = sha("aa");
        let chunked = sha("bb");
        let absent = sha("cc");
        let chunk1 = sha("c1");

        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ('u1','user','x')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO games (slug, display_name) VALUES ('g','G')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO saves (id, user_id, game_slug, label, latest_version_num) VALUES ('sv','u1','g','default',1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshots (id, save_id, version_num, total_size_bytes, file_count) VALUES ('s1','sv',1,300,2)")
            .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO blobs (user_id, sha256, size_bytes, refcount) VALUES ('u1',?,100,1)",
        )
        .bind(&whole)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f1','s1','a',100,?)")
            .bind(&whole).execute(&pool).await.unwrap();

        sqlx::query("INSERT INTO snapshot_files (id, snapshot_id, relative_path, size_bytes, sha256) VALUES ('f2','s1','big',200,?)")
            .bind(&chunked).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO snapshot_file_chunks (snapshot_file_id, ordinal, chunk_sha256) VALUES ('f2',0,?)")
            .bind(&chunk1).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO chunks (user_id, sha256, size_bytes, refcount) VALUES ('u1',?,200,1)",
        )
        .bind(&chunk1)
        .execute(&pool)
        .await
        .unwrap();

        let got = stored_representation(&pool, "u1", &whole).await.unwrap();
        assert!(matches!(got, Some(Stored::Blob { size_bytes: 100 })));

        let got = stored_representation(&pool, "u1", &chunked).await.unwrap();
        match got {
            Some(Stored::Chunks {
                size_bytes,
                file_id,
            }) => {
                assert_eq!(size_bytes, 200);
                assert_eq!(file_id, "f2");
            }
            other => panic!("se esperaba troceado, salió {other:?}"),
        }

        assert!(stored_representation(&pool, "u1", &absent)
            .await
            .unwrap()
            .is_none());
        // Dedup does not cross accounts: another user does not see this content.
        assert!(stored_representation(&pool, "u2", &whole)
            .await
            .unwrap()
            .is_none());
    }
}
