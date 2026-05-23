//! `/v1/cloud/saves*` — upload + download flows for cloud-stored snapshots.

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::quota;
use crate::cloud::r2;
use crate::cloud::state::CloudState;
use axum::{
    extract::{Path, State},
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
    /// Optional, used for analytics + device-limit check.
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
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
    // 1. Quota check first — cheap, fast, and a 402 short-circuits everything else.
    let info = match quota::check_storage(&state, user.user_id, body.size_bytes).await {
        Ok(i) => i,
        Err(resp) => return Ok(resp),
    };

    // 2. Ensure the saves row exists. UPSERT semantics — first version
    //    of a save creates it; subsequent versions just bump latest_version_num.
    let label = body.label.clone().unwrap_or_else(|| "default".to_string());
    let save_row: (String, i64) = sqlx::query_as(
        r#"
        INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
        VALUES ($1, $2, $3, $4, 0)
        ON CONFLICT (user_id, game_slug, label)
        DO UPDATE SET updated_at = now()
        RETURNING id, latest_version_num
        "#,
    )
    .bind(&body.save_id)
    .bind(user.user_id)
    .bind(&body.game_slug)
    .bind(&label)
    .fetch_one(&state.pool)
    .await?;

    let next_version = save_row.1 + 1;
    let r2_key = r2::key_for_snapshot(user.user_id, &save_row.0, next_version as u64);

    // 3. Insert the (pending) save_versions row. We only know size and key
    //    so far — sha256 is filled in by `commit`. Until then the row is
    //    pending and storage_bytes hasn't been credited (trigger runs on
    //    INSERT but with the *requested* size; if the upload fails or never
    //    commits, the cleanup cron deletes pending rows older than 1h).
    sqlx::query(
        r#"
        INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, notes)
        VALUES ($1, $2, $3, '', $4, $5)
        "#,
    )
    .bind(&save_row.0)
    .bind(next_version)
    .bind(body.size_bytes as i64)
    .bind(&r2_key)
    .bind(body.notes.as_deref())
    .execute(&state.pool)
    .await?;

    // 4. Mint the presigned PUT URL.
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
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM saves WHERE id = $1")
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
    sqlx::query(
        "UPDATE saves SET latest_version_num = $1, updated_at = now() WHERE id = $2",
    )
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
) -> Result<Json<DownloadOut>, CloudError> {
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

    Ok(Json(DownloadOut {
        save_id,
        version_num: version,
        sha256,
        size_bytes: size,
        download: presigned,
    }))
}
