use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;

/// Identity + quota snapshot for the authenticated user.
///
/// `storage_used_bytes` and `storage_quota_bytes` are read directly from
/// the `users` table (kept in sync by the snapshot upload/delete paths,
/// see `routes::snapshots`). The desktop app uses these for the quota
/// progress bar — see Phase 5 of the v0.3 build plan.
#[derive(Serialize)]
pub struct WhoamiResponse {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
    pub storage_used_bytes: i64,
    pub storage_quota_bytes: i64,
}

pub async fn whoami(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<WhoamiResponse>, StatusCode> {
    let user_id = user.user_id.to_string();
    let row = sqlx::query!(
        "SELECT storage_used_bytes, storage_quota_bytes FROM users WHERE id = ?",
        user_id,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "whoami quota lookup failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(WhoamiResponse {
        user_id,
        username: user.username,
        is_admin: user.is_admin,
        storage_used_bytes: row.storage_used_bytes,
        storage_quota_bytes: row.storage_quota_bytes,
    }))
}
