//! `/v1/feedback`: Hoard-help, the report form inside the app.
//!
//! Open to anyone. A self-hoster has no account here, and a Cloud user whose
//! session is broken is exactly who needs to write, so a bearer token is used
//! when it verifies and ignored when it does not. What keeps an open upload
//! route from becoming free file hosting is the throttle in [`create`], the
//! caps in `hoard_core::wire`, and files that are never served to anybody but
//! the admin.
//!
//! Files go through this process rather than to a presigned R2 URL. Some ISPs
//! do not route the R2 endpoint at all (the August "Stellaris" timeouts), and a
//! report form is the last place to lose people to that. The cost is bounded:
//! each upload holds one 8 MiB part buffer, and [`UPLOADS`] caps how many run
//! at once on a 256 MB machine.

use crate::cloud::{
    auth::CloudUser, errors::CloudError, feedback, routes::admin::call_as, state::CloudState,
};
use axum::{
    body::Body,
    extract::{ConnectInfo, Extension, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::TryStreamExt;
use hoard_core::wire::{
    FeedbackCreate, FeedbackCreated, FEEDBACK_MAX_FILES, FEEDBACK_MAX_FILE_BYTES,
    FEEDBACK_MAX_MESSAGE_CHARS, FEEDBACK_MAX_REPORT_BYTES, FEEDBACK_TOKEN_HEADER,
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;
use tokio_util::io::StreamReader;
use uuid::Uuid;

const PER_IP_HOUR: i64 = 5;
const PER_IP_DAY: i64 = 20;
/// Everybody together. Far above anything real, and the ceiling on what a
/// flood can cost: 3 GiB a day kept 90 days is about 270 GB of R2.
const ALL_HOUR: i64 = 60;
const BYTES_DAY: i64 = 3 * 1024 * 1024 * 1024;

static UPLOADS: Semaphore = Semaphore::const_new(2);

fn throttled(retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after.to_string())],
        Json(serde_json::json!({
            "error": "too many reports, try again later",
            "code": "feedback_throttled",
        })),
    )
        .into_response()
}

/// Leftmost `X-Forwarded-For`, the same trust the cloud rate limiter already
/// gives it, and the peer when there is none.
fn origin(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse::<std::net::IpAddr>().ok());
    match forwarded.or(peer.map(|p| p.ip())) {
        Some(ip) => crate::clientip::throttle_bucket(ip),
        None => "unknown".into(),
    }
}

/// The signed-in user, if the request carries a token that verifies.
async fn caller(state: &CloudState, headers: &HeaderMap) -> Option<(Uuid, Option<String>)> {
    let token = headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?
        .trim();
    let claims = state.jwks.verify(token).await.ok()?;
    Some((Uuid::parse_str(&claims.sub).ok()?, claims.email))
}

fn clip(s: Option<String>, max: usize) -> Option<String> {
    let s = s?.trim().to_string();
    (!s.is_empty()).then(|| s.chars().take(max).collect())
}

/// `POST /v1/feedback`
pub async fn create(
    State(state): State<CloudState>,
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(body): Json<FeedbackCreate>,
) -> Result<Response, CloudError> {
    let message = body.message.trim();
    if message.is_empty() {
        return Err(CloudError::BadRequest("message is empty".into()));
    }
    if message.chars().count() > FEEDBACK_MAX_MESSAGE_CHARS {
        return Err(CloudError::BadRequest("message is too long".into()));
    }
    if body.files.len() > FEEDBACK_MAX_FILES {
        return Err(CloudError::BadRequest(format!(
            "at most {FEEDBACK_MAX_FILES} files"
        )));
    }
    if body.files.iter().any(|f| f.size > FEEDBACK_MAX_FILE_BYTES) {
        return Err(CloudError::BadRequest(
            "a file is over the size limit".into(),
        ));
    }
    let total: u64 = body.files.iter().map(|f| f.size).sum();
    if total > FEEDBACK_MAX_REPORT_BYTES {
        return Err(CloudError::BadRequest(
            "the files are over the size limit".into(),
        ));
    }

    let ip_hash = feedback::sha256_hex(&origin(&headers, peer.map(|c| c.0)));
    let recent = feedback::recent(&state.pool, &ip_hash).await?;
    if recent.ip_hour >= PER_IP_HOUR || recent.all_hour >= ALL_HOUR {
        return Ok(throttled(3600));
    }
    if recent.ip_day >= PER_IP_DAY || recent.bytes_day + total as i64 > BYTES_DAY {
        return Ok(throttled(6 * 3600));
    }

    let user = caller(&state, &headers).await;
    let mode = body
        .mode
        .filter(|m| matches!(m.as_str(), "cloud" | "selfhosted" | "none"));
    // Two v4 uuids, 244 random bits, as the upload credential. Only its hash
    // is stored.
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());

    let mut tx = state.pool.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO feedback_reports
            (user_id, account_email, kind, message, contact, app_version, os, arch,
             mode, ip_hash, upload_token)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id",
    )
    .bind(user.as_ref().map(|u| u.0))
    .bind(user.and_then(|u| u.1))
    .bind(body.kind.as_str())
    .bind(message)
    .bind(clip(body.contact, 200))
    .bind(clip(body.app_version, 64))
    .bind(clip(body.os, 64))
    .bind(clip(body.arch, 64))
    .bind(mode)
    .bind(&ip_hash)
    .bind(feedback::sha256_hex(&token))
    .fetch_one(&mut *tx)
    .await?;
    for (i, f) in body.files.iter().enumerate() {
        let idx = i as i16;
        sqlx::query(
            "INSERT INTO feedback_files (report_id, idx, name, size_bytes, r2_key)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(idx)
        .bind(f.name.chars().take(200).collect::<String>())
        .bind(f.size as i64)
        .bind(feedback::object_key(id, idx, &f.name))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Json(FeedbackCreated {
        id: id.to_string(),
        upload_token: token,
    })
    .into_response())
}

/// The report the token opens, while it is still taking files.
async fn pending(state: &CloudState, id: Uuid, headers: &HeaderMap) -> Result<(), CloudError> {
    let token = headers
        .get(FEEDBACK_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(CloudError::Unauthorized("missing upload token"))?;
    let found: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM feedback_reports
          WHERE id = $1 AND upload_token = $2 AND completed_at IS NULL
            AND created_at > now() - interval '1 day'",
    )
    .bind(id)
    .bind(feedback::sha256_hex(token))
    .fetch_optional(&state.pool)
    .await?;
    found
        .map(|_| ())
        .ok_or(CloudError::NotFound("no open report with that token"))
}

/// `PUT /v1/feedback/:id/files/:idx`, the raw file as the body. Sending the same
/// index again replaces it, which is what a retry after a dropped connection
/// needs.
pub async fn upload(
    State(state): State<CloudState>,
    Path((id, idx)): Path<(Uuid, i16)>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, CloudError> {
    pending(&state, id, &headers).await?;
    let (size, key): (i64, String) = sqlx::query_as(
        "SELECT size_bytes, r2_key FROM feedback_files WHERE report_id = $1 AND idx = $2",
    )
    .bind(id)
    .bind(idx)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(CloudError::NotFound("no such file in this report"))?;

    let Ok(permit) = tokio::time::timeout(Duration::from_secs(120), UPLOADS.acquire()).await else {
        return Ok(throttled(30));
    };
    let _permit = permit.map_err(|e| CloudError::Internal(e.into()))?;

    // One byte past the declared size is enough to know it lied.
    let stream = body.into_data_stream().map_err(std::io::Error::other);
    let reader = StreamReader::new(stream).take(size as u64 + 1);
    let written = state
        .r2
        .put_from_reader(&key, reader)
        .await
        .map_err(CloudError::Internal)?;
    if written != size {
        let _ = state.r2.delete_object(&key).await;
        return Err(CloudError::BadRequest(format!(
            "expected {size} bytes, got {written}"
        )));
    }

    sqlx::query("UPDATE feedback_files SET uploaded_at = now() WHERE report_id = $1 AND idx = $2")
        .bind(id)
        .bind(idx)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /v1/feedback/:id/complete`
pub async fn complete(
    State(state): State<CloudState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, CloudError> {
    pending(&state, id, &headers).await?;
    let missing: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM feedback_files WHERE report_id = $1 AND uploaded_at IS NULL",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if missing > 0 {
        return Err(CloudError::Conflict("some files have not been uploaded"));
    }
    sqlx::query(
        "UPDATE feedback_reports SET completed_at = now(), upload_token = NULL WHERE id = $1",
    )
    .bind(id)
    .execute(&state.pool)
    .await?;

    let st = state.clone();
    tokio::spawn(async move { feedback::notify(&st, id).await });
    Ok(Json(serde_json::json!({ "id": id })))
}

#[derive(serde::Deserialize)]
pub struct FileRequest {
    report_id: Uuid,
    idx: i16,
}

/// `POST /v1/admin/feedback/file`: a ten-minute link to one attachment, for the
/// admin panel. The gate is the panel's own: `admin_feedback()` refuses anybody
/// but the admin, so a call that it answers is a call from the admin.
pub async fn admin_file(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(req): Json<FileRequest>,
) -> Result<Json<serde_json::Value>, CloudError> {
    if let Err(e) = call_as(&state.pool, user.user_id, "admin_feedback").await {
        return Err(
            match e.as_database_error().and_then(|d| d.code()).as_deref() {
                Some("42501") => CloudError::Forbidden("not authorized"),
                _ => e.into(),
            },
        );
    }
    let key: String = sqlx::query_scalar(
        "SELECT r2_key FROM feedback_files
          WHERE report_id = $1 AND idx = $2 AND uploaded_at IS NOT NULL",
    )
    .bind(req.report_id)
    .bind(req.idx)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(CloudError::NotFound("no such file"))?;
    let link = state
        .r2
        .presign_get(&key, Some(Duration::from_secs(600)))
        .await
        .map_err(CloudError::Internal)?;
    Ok(Json(serde_json::json!({ "url": link.url })))
}
