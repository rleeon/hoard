//! `/v1/cloud/saves*` — upload + download flows for cloud-stored snapshots.

use crate::cloud::auth::CloudUser;
use crate::cloud::bandwidth;
use crate::cloud::errors::CloudError;
use crate::cloud::plans::Plan;
use crate::cloud::quota;
use crate::cloud::r2;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    Extension,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Body for `POST /v1/cloud/saves`. The client states *intent*: how big the
/// snapshot will be, which save it belongs to, optional notes. The server
/// validates plan + quota, mints a presigned PUT URL for R2, and returns
/// it. Once the upload finishes, the client calls `commit` with the actual
/// sha256.
#[derive(Debug, Deserialize)]
pub struct UploadInit {
    pub save_id: String,
    pub game_slug: String,
    #[serde(default)]
    pub label: Option<String>,
    pub size_bytes: u64,
    /// Number of files inside the (opaque) tar.zst this version packs. The
    /// server can't introspect the blob, so the client declares it; we store
    /// it purely so the History view can show "N archivos". Older clients omit
    /// it → 0 (unknown), same as pre-existing rows.
    #[serde(default)]
    pub file_count: i64,
    /// Optional, used for analytics + device-limit check.
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// When true, this save is excluded from the multi-device manifest pull
    /// — other devices won't auto-restore it. Manual download via
    /// `/v1/cloud/saves/:id/versions/:n/download` still works. Toggleable
    /// per-save by the client (per the 1.6.1 "modo ahorro" UX).
    #[serde(default)]
    pub backup_only: bool,
    /// The version the client based this upload on (its last-synced version
    /// for this save). When present and it no longer matches the server's
    /// `latest_version_num`, the upload is a non-fast-forward (another device
    /// advanced the save) and is rejected with 409 so the client can pull +
    /// resolve instead of clobbering the other device's line.
    #[serde(default)]
    pub base_version: Option<i64>,
}

/// 409 response for a divergent (non-fast-forward) push. Carries the current
/// head so the client knows which version it must reconcile against.
#[derive(Debug, Serialize)]
struct NonFastForwardResponse {
    error: &'static str,
    code: &'static str,
    head_version: i64,
    base_version: i64,
}

impl IntoResponse for NonFastForwardResponse {
    fn into_response(self) -> Response {
        (StatusCode::CONFLICT, Json(self)).into_response()
    }
}

/// 413 response for a single upload that exceeds the per-save cap. Wire
/// shape mirrors the 402 quota response so the client toast layer can
/// reuse one structured-error path.
#[derive(Debug, Serialize)]
struct SaveTooLargeResponse {
    error: &'static str,
    code: &'static str,
    plan: &'static str,
    limit_bytes: u64,
    actual_bytes: u64,
    upgrade_url: String,
}

impl IntoResponse for SaveTooLargeResponse {
    fn into_response(self) -> Response {
        (StatusCode::PAYLOAD_TOO_LARGE, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct UploadInitOut {
    pub version_num: i64,
    pub r2_key: String,
    pub upload: r2::PresignedUrl,
    pub quota: quota::QuotaInfo,
}

pub async fn init_upload(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<UploadInit>,
) -> Result<Response, CloudError> {
    // 1. Per-save size cap — cheapest check, doesn't even touch the DB.
    //    Resolve the plan from the cached load() in `check_storage` below
    //    would mean two queries; do a tiny direct lookup so we can 413
    //    before incurring the storage SUM.
    let plan = match plan_for_user(&state, user.user_id).await? {
        Some(p) => p,
        None => return Err(CloudError::NotFound("no profile")),
    };
    let limits = plan.limits();
    if body.size_bytes > limits.max_save_size_bytes {
        let upgrade_url = state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(|| "https://hoard.services/upgrade".to_string());
        return Ok(SaveTooLargeResponse {
            error: "save exceeds per-save size limit",
            code: "save_too_large",
            plan: plan.as_str(),
            limit_bytes: limits.max_save_size_bytes,
            actual_bytes: body.size_bytes,
            upgrade_url,
        }
        .into_response());
    }

    // 2. Bandwidth window — pre-upload check so we 429 *before* presigning.
    //    The PUT itself goes direct to R2 so we can't intercept the bytes;
    //    we credit the window in `commit_upload` once R2 head confirms the
    //    object landed.
    if let Err(resp) = bandwidth::check(&state, user.user_id, body.size_bytes, &limits).await {
        return Ok(resp);
    }

    // 3. Storage quota.
    let info = match quota::check_storage(&state, user.user_id, body.size_bytes).await {
        Ok(i) => i,
        Err(resp) => return Ok(resp),
    };

    // 4. Ensure the saves row exists. UPSERT semantics — first version
    //    of a save creates it; subsequent versions just bump latest_version_num.
    //    `backup_only` is captured per-save: a row toggled to backup_only=true
    //    on a later upload stays out of the manifest until explicitly
    //    re-enabled by the client.
    let label = body.label.clone().unwrap_or_else(|| "default".to_string());
    let save_row: (String, i64) = sqlx::query_as(
        r#"
        INSERT INTO saves (id, user_id, game_slug, label, latest_version_num, backup_only)
        VALUES ($1, $2, $3, $4, 0, $5)
        ON CONFLICT (user_id, game_slug, label)
        DO UPDATE SET updated_at = now(), backup_only = EXCLUDED.backup_only
        RETURNING id, latest_version_num
        "#,
    )
    .bind(&body.save_id)
    .bind(user.user_id)
    .bind(&body.game_slug)
    .bind(&label)
    .bind(body.backup_only)
    .fetch_one(&state.pool)
    .await?;

    let head = save_row.1;

    // Fast-forward check (the DAG's enforcement). A base that no longer matches
    // the head means another device pushed since the client last synced.
    if let Some(base) = body.base_version {
        if base != head {
            return Ok(NonFastForwardResponse {
                error:
                    "non-fast-forward: another device advanced this save since your base version",
                code: "non_fast_forward",
                head_version: head,
                base_version: base,
            }
            .into_response());
        }
    }

    let next_version = head + 1;
    // Root version has no parent; otherwise it descends from the current head.
    let parent_version: Option<i64> = (head > 0).then_some(head);
    let r2_key = r2::key_for_snapshot(user.user_id, &save_row.0, next_version as u64);

    // 5. Insert the (pending) save_versions row. We only know size and key
    //    so far — sha256 is filled in by `commit`. Until then the row is
    //    pending and storage_bytes hasn't been credited (trigger runs on
    //    INSERT but with the *requested* size; if the upload fails or never
    //    commits, the cleanup cron deletes pending rows older than 1h).
    sqlx::query(
        r#"
        INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, notes, parent_version, file_count)
        VALUES ($1, $2, $3, '', $4, $5, $6, $7)
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(body.size_bytes as i64)
    .bind(&r2_key)
    .bind(body.notes.as_deref())
    .bind(parent_version)
    .bind(body.file_count.max(0))
    .execute(&state.pool)
    .await?;

    // 6. Mint the presigned PUT URL.
    let upload = state
        .r2
        .presign_put(&r2_key, None)
        .await
        .map_err(CloudError::Internal)?;

    Ok(Json(UploadInitOut {
        version_num: next_version,
        r2_key,
        upload,
        quota: info,
    })
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct UploadCommit {
    pub sha256: String,
    /// Actual size as observed by the client post-upload. We verify against
    /// R2 head to make sure the user didn't lie.
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadCommitOut {
    pub save_id: String,
    pub version_num: i64,
    pub committed: bool,
}

pub async fn commit_upload(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
    Json(body): Json<UploadCommit>,
) -> Result<Json<UploadCommitOut>, CloudError> {
    // Owner check first — never trust a save_id from the request without
    // verifying the JWT subject owns it.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    // Look up the pending row.
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT r2_key, size_bytes FROM save_versions
            WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((r2_key, expected_size)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };

    // Confirm the object actually landed in R2 with the right size.
    let head_size = state
        .r2
        .head(&r2_key)
        .await
        .map_err(CloudError::Internal)?
        .ok_or(CloudError::BadRequest(
            "R2 object missing — upload not complete".into(),
        ))?;
    if head_size as u64 != body.size_bytes {
        return Err(CloudError::BadRequest(format!(
            "size mismatch: client says {} but R2 has {}",
            body.size_bytes, head_size
        )));
    }
    if expected_size != head_size {
        // Adjust pending size to actual, so quota accounting tracks reality.
        // Storage trigger keeps profiles.storage_bytes coherent on UPDATE.
    }

    // Atomically finalize: stamp sha256, bump the parent save's latest_version_num.
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE save_versions SET sha256 = $1, size_bytes = $2
            WHERE save_id = $3 AND version_num = $4",
    )
    .bind(&body.sha256)
    .bind(head_size)
    .bind(&save_id)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2")
        .bind(version)
        .bind(&save_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'upload', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(head_size)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Credit the bandwidth window with the observed size. Done after the
    // commit so a failed upload doesn't eat into the user's quota. We log
    // and swallow errors here — the upload itself was successful; a stale
    // bandwidth counter is recoverable, a 500 returned to the client is
    // not.
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, head_size as u64).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on upload");
    }

    Ok(Json(UploadCommitOut {
        save_id,
        version_num: version,
        committed: true,
    }))
}

// ===========================================================================
// Content-addressed (per-file SHA dedup) upload + download.
//
// The client declares a manifest of (relative_path, sha256, size). The server
// answers which whole-file blobs it doesn't already have; the client PUTs only
// those to R2 (one object per blob, keyed by SHA), then commits. Bytes shared
// with a previous version — even most of a 600 MB save the game rewrote in
// place — are never re-uploaded.
// ===========================================================================

/// One file in a content-addressed manifest.
#[derive(Debug, Deserialize)]
pub struct CasFileEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    /// Source file mtime (unix seconds), preserved on restore. Optional.
    #[serde(default)]
    pub modified_at: Option<i64>,
}

/// Body for `POST /v1/cloud/saves/cas`. Same intent as [`UploadInit`] but
/// carries the per-file manifest instead of a single archive size.
#[derive(Debug, Deserialize)]
pub struct CasInit {
    pub save_id: String,
    pub game_slug: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub backup_only: bool,
    #[serde(default)]
    pub base_version: Option<i64>,
    pub files: Vec<CasFileEntry>,
}

/// A blob the server doesn't have yet — the client must PUT it.
#[derive(Debug, Serialize)]
pub struct CasMissingBlob {
    pub sha256: String,
    pub size_bytes: i64,
    pub r2_key: String,
    pub upload: r2::PresignedUrl,
}

#[derive(Debug, Serialize)]
pub struct CasInitOut {
    pub version_num: i64,
    /// Blobs the client must upload. The full file set is whatever it declared
    /// in `files`; everything not listed here is already stored server-side.
    pub missing: Vec<CasMissingBlob>,
    pub quota: quota::QuotaInfo,
}

pub async fn cas_init(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<CasInit>,
) -> Result<Response, CloudError> {
    if body.files.is_empty() {
        return Err(CloudError::BadRequest("empty manifest".into()));
    }

    let plan = match plan_for_user(&state, user.user_id).await? {
        Some(p) => p,
        None => return Err(CloudError::NotFound("no profile")),
    };
    let limits = plan.limits();

    // Logical save size = sum of all file sizes. Drives the per-save cap.
    let logical_size: i64 = body.files.iter().map(|f| f.size_bytes.max(0)).sum();
    if logical_size as u64 > limits.max_save_size_bytes {
        let upgrade_url = state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(|| "https://hoard.services/upgrade".to_string());
        return Ok(SaveTooLargeResponse {
            error: "save exceeds per-save size limit",
            code: "save_too_large",
            plan: plan.as_str(),
            limit_bytes: limits.max_save_size_bytes,
            actual_bytes: logical_size as u64,
            upgrade_url,
        }
        .into_response());
    }

    // Dedup the manifest down to unique (sha -> size). Then find which of those
    // the user already has stored, so we only charge bandwidth + storage for —
    // and only presign — the genuinely new bytes.
    let mut unique: BTreeMap<String, i64> = BTreeMap::new();
    for f in &body.files {
        unique.entry(f.sha256.clone()).or_insert(f.size_bytes.max(0));
    }
    let all_shas: Vec<String> = unique.keys().cloned().collect();
    let existing: Vec<(String,)> = sqlx::query_as(
        "SELECT sha256 FROM cloud_blobs WHERE user_id = $1 AND sha256 = ANY($2)",
    )
    .bind(user.user_id)
    .bind(&all_shas)
    .fetch_all(&state.pool)
    .await?;
    let existing: std::collections::HashSet<String> =
        existing.into_iter().map(|(s,)| s).collect();

    let missing_shas: Vec<(&String, &i64)> = unique
        .iter()
        .filter(|(sha, _)| !existing.contains(*sha))
        .collect();
    let new_bytes: u64 = missing_shas.iter().map(|(_, sz)| **sz as u64).sum();

    // Bandwidth + storage are charged only on the bytes we'll actually move
    // and persist. A save rewritten in place with 10 MB of real change costs
    // 10 MB here, not 600 MB.
    if let Err(resp) = bandwidth::check(&state, user.user_id, new_bytes, &limits).await {
        return Ok(resp);
    }
    let info = match quota::check_storage(&state, user.user_id, new_bytes).await {
        Ok(i) => i,
        Err(resp) => return Ok(resp),
    };

    // Ensure the saves row exists (same UPSERT semantics as the archive path).
    let label = body.label.clone().unwrap_or_else(|| "default".to_string());
    let save_row: (String, i64) = sqlx::query_as(
        r#"
        INSERT INTO saves (id, user_id, game_slug, label, latest_version_num, backup_only)
        VALUES ($1, $2, $3, $4, 0, $5)
        ON CONFLICT (user_id, game_slug, label)
        DO UPDATE SET updated_at = now(), backup_only = EXCLUDED.backup_only
        RETURNING id, latest_version_num
        "#,
    )
    .bind(&body.save_id)
    .bind(user.user_id)
    .bind(&body.game_slug)
    .bind(&label)
    .bind(body.backup_only)
    .fetch_one(&state.pool)
    .await?;
    let head = save_row.1;

    if let Some(base) = body.base_version {
        if base != head {
            return Ok(NonFastForwardResponse {
                error:
                    "non-fast-forward: another device advanced this save since your base version",
                code: "non_fast_forward",
                head_version: head,
                base_version: base,
            }
            .into_response());
        }
    }

    let next_version = head + 1;
    let parent_version: Option<i64> = (head > 0).then_some(head);

    // Replace any stale pending row at this version (an earlier init that never
    // committed). Its manifest rows cascade away with it.
    sqlx::query(
        "DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2 AND sha256 = ''",
    )
    .bind(&save_row.0)
    .bind(next_version)
    .execute(&state.pool)
    .await?;

    // Pending content-addressed version. sha256 = '' until commit; r2_key is
    // unused (blobs carry their own keys). The storage trigger skips
    // content_addressed rows, so this charges nothing — blobs do, at commit.
    sqlx::query(
        r#"
        INSERT INTO save_versions
            (save_id, version_num, size_bytes, sha256, r2_key, notes, parent_version, file_count, content_addressed)
        VALUES ($1, $2, $3, '', '', $4, $5, $6, TRUE)
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(logical_size)
    .bind(body.notes.as_deref())
    .bind(parent_version)
    .bind(body.files.len() as i64)
    .execute(&state.pool)
    .await?;

    // Record the manifest in one round-trip via UNNEST.
    let paths: Vec<String> = body.files.iter().map(|f| f.relative_path.clone()).collect();
    let shas: Vec<String> = body.files.iter().map(|f| f.sha256.clone()).collect();
    let sizes: Vec<i64> = body.files.iter().map(|f| f.size_bytes.max(0)).collect();
    let mtimes: Vec<Option<i64>> = body.files.iter().map(|f| f.modified_at).collect();
    sqlx::query(
        r#"
        INSERT INTO save_version_files (save_id, version_num, relative_path, sha256, size_bytes, modified_at)
        SELECT $1, $2, p, s, z, m
          FROM UNNEST($3::text[], $4::text[], $5::bigint[], $6::bigint[]) AS t(p, s, z, m)
        ON CONFLICT (save_id, version_num, relative_path) DO NOTHING
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(&paths)
    .bind(&shas)
    .bind(&sizes)
    .bind(&mtimes)
    .execute(&state.pool)
    .await?;

    // Mint a presigned PUT for each missing blob.
    let mut missing = Vec::with_capacity(missing_shas.len());
    for (sha, size) in missing_shas {
        let key = r2::key_for_blob(user.user_id, sha);
        let upload = state
            .r2
            .presign_put(&key, None)
            .await
            .map_err(CloudError::Internal)?;
        missing.push(CasMissingBlob {
            sha256: sha.clone(),
            size_bytes: *size,
            r2_key: key,
            upload,
        });
    }

    Ok(Json(CasInitOut {
        version_num: next_version,
        missing,
        quota: info,
    })
    .into_response())
}

/// `POST /v1/cloud/saves/:save_id/versions/:version/cas/commit` — finalize a
/// content-addressed upload. Verifies each new blob landed in R2, bumps blob
/// refcounts (storage is charged on a blob's first reference), stamps the
/// version's manifest digest and advances the save head. Idempotent and
/// race-safe: the commit is claimed with a guarded `sha256 = ''` update.
pub async fn cas_commit(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Json<UploadCommitOut>, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    // The version must exist and be a content-addressed row.
    let vrow: Option<(String, bool)> = sqlx::query_as(
        "SELECT sha256, content_addressed FROM save_versions
            WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((cur_sha, content_addressed)) = vrow else {
        return Err(CloudError::NotFound("version not found"));
    };
    if !content_addressed {
        return Err(CloudError::BadRequest(
            "version is not content-addressed".into(),
        ));
    }
    if !cur_sha.is_empty() {
        // Already committed — idempotent success.
        return Ok(Json(UploadCommitOut {
            save_id,
            version_num: version,
            committed: true,
        }));
    }

    // Full manifest, ordered, for the digest; distinct shas for refcounting.
    let manifest: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT relative_path, sha256, size_bytes FROM save_version_files
            WHERE save_id = $1 AND version_num = $2
         ORDER BY relative_path",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_all(&state.pool)
    .await?;
    if manifest.is_empty() {
        return Err(CloudError::BadRequest("version has no manifest".into()));
    }

    // Manifest digest: a stable content identity for the whole version, used
    // wherever the archive's whole-blob sha256 was (list/sync/integrity).
    let mut hasher = Sha256::new();
    for (path, sha, size) in &manifest {
        hasher.update(path.as_bytes());
        hasher.update([0u8]);
        hasher.update(sha.as_bytes());
        hasher.update([0u8]);
        hasher.update(size.to_le_bytes());
        hasher.update([0u8]);
    }
    let digest = hex::encode(hasher.finalize());

    // Distinct (sha -> size).
    let mut unique: BTreeMap<String, i64> = BTreeMap::new();
    for (_, sha, size) in &manifest {
        unique.entry(sha.clone()).or_insert(*size);
    }
    let all_shas: Vec<String> = unique.keys().cloned().collect();
    let existing: Vec<(String,)> = sqlx::query_as(
        "SELECT sha256 FROM cloud_blobs WHERE user_id = $1 AND sha256 = ANY($2)",
    )
    .bind(user.user_id)
    .bind(&all_shas)
    .fetch_all(&state.pool)
    .await?;
    let existing: std::collections::HashSet<String> =
        existing.into_iter().map(|(s,)| s).collect();

    // Verify every new blob actually landed in R2 before we reference it.
    let mut new_bytes: u64 = 0;
    for (sha, size) in &unique {
        if existing.contains(sha) {
            continue;
        }
        let key = r2::key_for_blob(user.user_id, sha);
        let head = state.r2.head(&key).await.map_err(CloudError::Internal)?;
        if head.is_none() {
            return Err(CloudError::BadRequest(format!(
                "blob {sha} was not uploaded"
            )));
        }
        new_bytes += *size as u64;
    }

    // Claim + finalize atomically. The guarded update fences concurrent commits
    // and makes a double-commit a no-op (0 rows → someone else won the race).
    let mut tx = state.pool.begin().await?;
    let claimed = sqlx::query(
        "UPDATE save_versions SET sha256 = $1
            WHERE save_id = $2 AND version_num = $3
              AND content_addressed = TRUE AND sha256 = ''",
    )
    .bind(&digest)
    .bind(&save_id)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    if claimed.rows_affected() == 0 {
        // Lost the race — another commit finalized it. Treat as success.
        tx.rollback().await.ok();
        return Ok(Json(UploadCommitOut {
            save_id,
            version_num: version,
            committed: true,
        }));
    }

    // Bump refcounts: +1 per distinct blob this version references. New blobs
    // are inserted at refcount 1 (their first reference charges storage via the
    // cloud_blobs trigger); existing ones just increment.
    for (sha, size) in &unique {
        let key = r2::key_for_blob(user.user_id, sha);
        sqlx::query(
            r#"
            INSERT INTO cloud_blobs (user_id, sha256, size_bytes, r2_key, refcount)
            VALUES ($1, $2, $3, $4, 1)
            ON CONFLICT (user_id, sha256)
            DO UPDATE SET refcount = cloud_blobs.refcount + 1
            "#,
        )
        .bind(user.user_id)
        .bind(sha)
        .bind(*size)
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2")
        .bind(version)
        .bind(&save_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'upload', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(new_bytes as i64)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Credit bandwidth with the bytes actually transferred (the new blobs).
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, new_bytes).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on cas commit");
    }

    Ok(Json(UploadCommitOut {
        save_id,
        version_num: version,
        committed: true,
    }))
}

/// One file in a version manifest response. `download` is populated only when
/// the request asked to presign (the restore path); the History detail view
/// omits it and pays no bandwidth.
#[derive(Debug, Serialize)]
pub struct ManifestFile {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<r2::PresignedUrl>,
}

#[derive(Debug, Serialize)]
pub struct VersionManifestOut {
    /// False for legacy archive versions — the client then falls back to the
    /// whole-archive `download` endpoint.
    pub content_addressed: bool,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestQuery {
    /// When true, mint a presigned GET per (unique) blob and charge bandwidth.
    /// When false (default) just list the files — cheap, no bandwidth.
    #[serde(default)]
    pub presign: bool,
}

/// `GET /v1/cloud/saves/:save_id/versions/:version/manifest` — the per-file
/// manifest of a content-addressed version. With `?presign=true` it also mints
/// a download URL per blob (restore); without it, just the listing (History).
pub async fn version_manifest(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
    axum::extract::Query(q): axum::extract::Query<ManifestQuery>,
) -> Result<Response, CloudError> {
    let vrow: Option<(Uuid, bool)> = sqlx::query_as(
        r#"
        SELECT s.user_id, sv.content_addressed
          FROM save_versions sv
          JOIN saves s ON s.id = sv.save_id
         WHERE sv.save_id = $1 AND sv.version_num = $2
        "#,
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((owner, content_addressed)) = vrow else {
        return Err(CloudError::NotFound("version not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("not your save"));
    }
    if !content_addressed {
        return Ok(Json(VersionManifestOut {
            content_addressed: false,
            files: Vec::new(),
        })
        .into_response());
    }

    let rows: Vec<(String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT relative_path, sha256, size_bytes, modified_at FROM save_version_files
            WHERE save_id = $1 AND version_num = $2
         ORDER BY relative_path",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_all(&state.pool)
    .await?;

    if !q.presign {
        let files = rows
            .into_iter()
            .map(|(relative_path, sha256, size_bytes, modified_at)| ManifestFile {
                relative_path,
                sha256,
                size_bytes,
                modified_at,
                download: None,
            })
            .collect();
        return Ok(Json(VersionManifestOut {
            content_addressed: true,
            files,
        })
        .into_response());
    }

    // Presign path: charge bandwidth for the unique bytes downloaded, then mint
    // one GET per distinct blob and map it back onto every file that uses it.
    let mut unique_size: BTreeMap<String, i64> = BTreeMap::new();
    for (_, sha, size, _) in &rows {
        unique_size.entry(sha.clone()).or_insert(*size);
    }
    let download_bytes: u64 = unique_size.values().map(|s| *s as u64).sum();

    let plan = plan_for_user(&state, user.user_id)
        .await?
        .unwrap_or(Plan::Free);
    let limits = plan.limits();
    if let Err(resp) = bandwidth::check(&state, user.user_id, download_bytes, &limits).await {
        return Ok(resp);
    }

    let mut url_for: BTreeMap<String, r2::PresignedUrl> = BTreeMap::new();
    for sha in unique_size.keys() {
        let key = r2::key_for_blob(user.user_id, sha);
        let presigned = state
            .r2
            .presign_get(&key, None)
            .await
            .map_err(CloudError::Internal)?;
        url_for.insert(sha.clone(), presigned);
    }

    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'download', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(download_bytes as i64)
    .execute(&state.pool)
    .await?;
    if let Err(e) = bandwidth::record(&state.pool, user.user_id, download_bytes).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on cas manifest");
    }

    let files = rows
        .into_iter()
        .map(|(relative_path, sha256, size_bytes, modified_at)| ManifestFile {
            download: url_for.get(&sha256).cloned(),
            relative_path,
            sha256,
            size_bytes,
            modified_at,
        })
        .collect();
    Ok(Json(VersionManifestOut {
        content_addressed: true,
        files,
    })
    .into_response())
}

#[derive(Debug, Serialize)]
pub struct DownloadOut {
    pub save_id: String,
    pub version_num: i64,
    pub sha256: String,
    pub size_bytes: i64,
    pub download: r2::PresignedUrl,
}

pub async fn download(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, CloudError> {
    let row: Option<(Uuid, String, String, i64, bool)> = sqlx::query_as(
        r#"
        SELECT s.user_id, sv.r2_key, sv.sha256, sv.size_bytes, sv.content_addressed
          FROM save_versions sv
          JOIN saves s ON s.id = sv.save_id
         WHERE sv.save_id = $1 AND sv.version_num = $2
           AND sv.deleted_at IS NULL
        "#,
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((owner, r2_key, sha256, size, content_addressed)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("not your save"));
    }
    if content_addressed {
        // Content-addressed versions have no single archive blob. The client
        // must pull them through the per-file manifest endpoint instead.
        return Err(CloudError::BadRequest(
            "version is content-addressed — use the manifest endpoint".into(),
        ));
    }

    // Bandwidth gate. Downloads count against the same window as uploads
    // (one quota, one counter). We credit optimistically below since the
    // presigned URL almost always gets used; if the client never fetches
    // it the overcount falls off in 15 min.
    let plan = plan_for_user(&state, user.user_id)
        .await?
        .unwrap_or(Plan::Free);
    let limits = plan.limits();
    if let Err(resp) = bandwidth::check(&state, user.user_id, size.max(0) as u64, &limits).await {
        return Ok(resp);
    }

    let presigned = state
        .r2
        .presign_get(&r2_key, None)
        .await
        .map_err(CloudError::Internal)?;

    sqlx::query(
        "INSERT INTO sync_log (user_id, save_id, version_num, kind, bytes)
             VALUES ($1, $2, $3, 'download', $4)",
    )
    .bind(user.user_id)
    .bind(&save_id)
    .bind(version)
    .bind(size)
    .execute(&state.pool)
    .await?;

    if let Err(e) = bandwidth::record(&state.pool, user.user_id, size.max(0) as u64).await {
        tracing::warn!(error = %e, user_id = %user.user_id, "bandwidth: record failed on download");
    }

    Ok(Json(DownloadOut {
        save_id,
        version_num: version,
        sha256,
        size_bytes: size,
        download: presigned,
    })
    .into_response())
}

/// One row in the cloud version history. Mirrors the self-hosted
/// `SnapshotSummary` wire shape (and the agent's `Snapshot`) so the desktop's
/// History view renders cloud and self-hosted snapshots through one type.
#[derive(Debug, Serialize)]
pub struct VersionEntry {
    /// String id for parity with self-hosted (which keys on a UUID). The
    /// version number is unique within a save and all the client needs.
    pub id: String,
    pub save_id: String,
    pub version_num: i64,
    pub parent_version: Option<i64>,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub is_pinned: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct ListVersionsQuery {
    #[serde(default)]
    pub include_deleted: bool,
}

/// `GET /v1/cloud/saves/:save_id/versions` — full committed version history
/// for a save. The sync manifest only carries the latest version; this
/// surfaces every committed one so History can list and restore any of them.
/// The R2 blobs and `save_versions` rows already persist per version — this
/// just exposes them. Pending (uncommitted, `sha256 = ''`) rows are excluded.
pub async fn list_versions(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ListVersionsQuery>,
) -> Result<Json<Vec<VersionEntry>>, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    type Row = (
        i64,
        i64,
        Option<i64>,
        i64,
        bool,
        time::OffsetDateTime,
        Option<time::OffsetDateTime>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT version_num, size_bytes, parent_version, file_count, is_pinned, created_at, deleted_at
          FROM save_versions
         WHERE save_id = $1
           AND sha256 <> ''
           AND ($2 OR deleted_at IS NULL)
      ORDER BY version_num DESC
        "#,
    )
    .bind(&save_id)
    .bind(q.include_deleted)
    .fetch_all(&state.pool)
    .await?;

    let out = rows
        .into_iter()
        .map(
            |(version_num, size, parent, file_count, is_pinned, created, deleted)| VersionEntry {
                id: version_num.to_string(),
                save_id: save_id.clone(),
                version_num,
                parent_version: parent,
                file_count,
                total_size_bytes: size,
                is_pinned,
                created_at: created,
                deleted_at: deleted,
            },
        )
        .collect();
    Ok(Json(out))
}

/// `DELETE /v1/cloud/saves/:save_id/versions/:version` — drop a single
/// committed version. We delete the R2 blob (best-effort) then the row (the
/// storage_bytes trigger credits the freed space). If the deleted version was
/// the save's head, `latest_version_num` is repointed at the highest
/// remaining committed version; if none remain the whole save row goes so it
/// stops showing up empty in the manifest.
pub async fn delete_version(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path((save_id, version)): Path<(String, i64)>,
) -> Result<Response, CloudError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT r2_key, content_addressed FROM save_versions WHERE save_id = $1 AND version_num = $2",
    )
    .bind(&save_id)
    .bind(version)
    .fetch_optional(&state.pool)
    .await?;
    let Some((r2_key, content_addressed)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };

    if content_addressed {
        // Gather this version's distinct blobs before the manifest cascades
        // away, then drop the version row (cascades save_version_files) and
        // release one reference per blob.
        let shas: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT sha256 FROM save_version_files WHERE save_id = $1 AND version_num = $2",
        )
        .bind(&save_id)
        .bind(version)
        .fetch_all(&state.pool)
        .await?;

        sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
            .bind(&save_id)
            .bind(version)
            .execute(&state.pool)
            .await?;

        release_blobs(&state, user.user_id, shas.iter().map(|(s,)| (s.as_str(), 1))).await;
    } else {
        if let Err(e) = state.r2.delete_object(&r2_key).await {
            tracing::warn!(error = %e, r2_key = %r2_key, "cloud delete version: R2 object delete failed");
        }
        sqlx::query("DELETE FROM save_versions WHERE save_id = $1 AND version_num = $2")
            .bind(&save_id)
            .bind(version)
            .execute(&state.pool)
            .await?;
    }

    // Repoint head / drop the save if it's now empty.
    let new_head: Option<(i64,)> = sqlx::query_as(
        "SELECT MAX(version_num) FROM save_versions WHERE save_id = $1 AND sha256 <> ''",
    )
    .bind(&save_id)
    .fetch_optional(&state.pool)
    .await?;
    match new_head.and_then(|(v,)| (v > 0).then_some(v)) {
        Some(head) => {
            sqlx::query("UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2")
                .bind(head)
                .bind(&save_id)
                .execute(&state.pool)
                .await?;
        }
        None => {
            sqlx::query("DELETE FROM saves WHERE id = $1 AND user_id = $2")
                .bind(&save_id)
                .bind(user.user_id)
                .execute(&state.pool)
                .await?;
        }
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `DELETE /v1/cloud/saves/:save_id` — wipe a cloud save and every version
/// it holds so the user can reclaim storage. We drop the R2 blobs first
/// (best-effort) then cascade-delete the DB rows; the storage_bytes trigger
/// credits the freed space back as each `save_versions` row goes.
pub async fn delete_save(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Path(save_id): Path<String>,
) -> Result<Response, CloudError> {
    // Owner check — never trust a save_id from the request.
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
        .bind(&save_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(owner) = owner else {
        return Err(CloudError::NotFound("save not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("save belongs to a different user"));
    }

    // Legacy archive versions: one opaque R2 object each. Purge before the
    // rows (and their keys) cascade away.
    let keys: Vec<(String,)> = sqlx::query_as(
        "SELECT r2_key FROM save_versions
            WHERE save_id = $1 AND content_addressed = FALSE AND r2_key <> ''",
    )
    .bind(&save_id)
    .fetch_all(&state.pool)
    .await?;
    for (key,) in &keys {
        if let Err(e) = state.r2.delete_object(key).await {
            // A leaked blob is recoverable by a later sweep; a 500 here would
            // strand the row pointing at a key we already tried to drop.
            tracing::warn!(error = %e, r2_key = %key, "cloud delete: R2 object delete failed");
        }
    }

    // Content-addressed versions: release one reference per distinct version of
    // this save that pointed at each blob. Read the counts before the manifest
    // cascades away with the save.
    let blob_refs: Vec<(String, i64)> = sqlx::query_as(
        "SELECT sha256, COUNT(DISTINCT version_num) FROM save_version_files
            WHERE save_id = $1 GROUP BY sha256",
    )
    .bind(&save_id)
    .fetch_all(&state.pool)
    .await?;

    // Cascade: deleting the save removes its save_versions + save_version_files
    // rows (FK ON DELETE CASCADE). The save_versions storage trigger skips
    // content-addressed rows; blob storage is credited as refcounts hit 0 below.
    sqlx::query("DELETE FROM saves WHERE id = $1 AND user_id = $2")
        .bind(&save_id)
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;

    release_blobs(
        &state,
        user.user_id,
        blob_refs.iter().map(|(s, n)| (s.as_str(), *n)),
    )
    .await;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Release `n` references from each blob, deleting the R2 object + row when a
/// blob's refcount reaches zero (the cloud_blobs trigger credits the freed
/// storage on the 0-transition). Best-effort: a failure here only leaks a blob,
/// recoverable by a later sweep, so errors are logged not propagated.
async fn release_blobs<'a, I>(state: &CloudState, user_id: Uuid, blobs: I)
where
    I: IntoIterator<Item = (&'a str, i64)>,
{
    for (sha, dec) in blobs {
        let row: Result<Option<(i64, String)>, _> = sqlx::query_as(
            "UPDATE cloud_blobs SET refcount = GREATEST(0, refcount - $3)
                WHERE user_id = $1 AND sha256 = $2
             RETURNING refcount, r2_key",
        )
        .bind(user_id)
        .bind(sha)
        .bind(dec)
        .fetch_optional(&state.pool)
        .await;
        match row {
            Ok(Some((refcount, key))) if refcount <= 0 => {
                if let Err(e) = state.r2.delete_object(&key).await {
                    tracing::warn!(error = %e, r2_key = %key, "cloud blob GC: R2 delete failed");
                }
                if let Err(e) = sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1 AND sha256 = $2")
                    .bind(user_id)
                    .bind(sha)
                    .execute(&state.pool)
                    .await
                {
                    tracing::warn!(error = %e, sha = %sha, "cloud blob GC: row delete failed");
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, sha = %sha, "cloud blob GC: refcount decrement failed");
            }
        }
    }
}

/// Tiny one-query helper to fetch a user's plan tag without going through
/// the full quota::load pipeline. Used by paths that need the plan but
/// don't need (yet) the storage figures.
async fn plan_for_user(state: &CloudState, user_id: Uuid) -> Result<Option<Plan>, CloudError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT plan FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(row.map(|r| Plan::from_str(&r.0).unwrap_or(Plan::Free)))
}
