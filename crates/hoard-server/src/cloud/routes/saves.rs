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
    let row: Option<(Uuid, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT s.user_id, sv.r2_key, sv.sha256, sv.size_bytes
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
    let Some((owner, r2_key, sha256, size)) = row else {
        return Err(CloudError::NotFound("version not found"));
    };
    if owner != user.user_id {
        return Err(CloudError::Forbidden("not your save"));
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

/// `DELETE /v1/cloud/saves/:save_id` — wipe a cloud save and every version
/// it holds so the user can reclaim storage. Cloud keeps no per-snapshot
/// history (only the latest committed version is surfaced), so "delete a
/// snapshot" on cloud means "delete the whole save". We drop the R2 blobs
/// first (best-effort) then cascade-delete the DB rows; the storage_bytes
/// trigger credits the freed space back as each `save_versions` row goes.
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

    // Gather every blob key for this save so we can purge R2 before the DB
    // rows (and their keys) disappear.
    let keys: Vec<(String,)> =
        sqlx::query_as("SELECT r2_key FROM save_versions WHERE save_id = $1")
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

    // Cascade: deleting the save removes its save_versions rows (FK ON DELETE
    // CASCADE), and the AFTER DELETE trigger decrements profiles.storage_bytes
    // per row.
    sqlx::query("DELETE FROM saves WHERE id = $1 AND user_id = $2")
        .bind(&save_id)
        .bind(user.user_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
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
