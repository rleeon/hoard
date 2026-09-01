use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::Json,
};
use hoard_core::wire::{MaxVersionsBody, MaxVersionsResponse, Whoami};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::routes::health::ServerState;
use crate::routes::repair_username;

/// Identity + quota snapshot for the authenticated user.
///
/// `storage_used_bytes` and `storage_quota_bytes` are read directly from
/// the `users` table (kept in sync by the snapshot upload/delete paths,
/// see `routes::snapshots`). The desktop app uses these for the quota
/// progress bar; see Phase 5 of the v0.3 build plan.
pub async fn whoami(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Whoami>, StatusCode> {
    let user_id = user.user_id.to_string();
    // The client calls here on sign-in and on startup, which makes it the
    // natural place to register the machine in the census (`routes::devices`),
    // the same role `/v1/me` plays on cloud. Best-effort: a failing census must
    // not take down the user's identity, which is what this route is about.
    if let Err(e) = crate::routes::devices::register(&state.pool, &user_id, &headers).await {
        tracing::warn!(error = %e, "devices: register on whoami failed");
    }
    // Runtime query (not the `query!` macro) so the new max_versions column
    // can be selected without regenerating the offline sqlx cache.
    let row: (i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT storage_used_bytes, storage_quota_bytes, max_versions, max_manual_versions \
         FROM users WHERE id = ?",
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "whoami quota lookup failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(Whoami {
        user_id,
        // The `users` row is persisted state, so it goes through the lenient
        // gate: a legacy username the strict `parse` would reject must not
        // 500 `whoami`, which would lock the account out of the app entirely
        // (ADR 0021 C.3).
        username: repair_username(&user.username),
        is_admin: user.is_admin,
        storage_used_bytes: row.0,
        storage_quota_bytes: row.1,
        max_versions: row.2,
        max_manual_versions: row.3,
        max_snapshot_size_bytes: Some(
            (state.config.storage.max_snapshot_size_mb as i64).saturating_mul(1024 * 1024),
        ),
    }))
}

/// `PUT /v1/me/max-versions`: set, or clear with `null`, the per-user cap
/// on stored versions per save, then immediately trash any snapshot already
/// over it so the effect is visible without waiting for the next backup.
/// With `dry_run: true` it only previews the prune count.
pub async fn set_max_versions(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<ServerState>>,
    Json(body): Json<MaxVersionsBody>,
) -> Result<Json<MaxVersionsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let internal = |e: anyhow::Error, what: &str| {
        tracing::error!(error = %e, "{what} failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "internal error" })),
        )
    };
    if let Some(n) = body.max_versions {
        if !(1..=10_000).contains(&n) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "max_versions must be between 1 and 10000" })),
            ));
        }
    }
    let user_id = user.user_id.to_string();

    if body.dry_run {
        // Clearing the cap never prunes, so the preview is only meaningful
        // for a concrete number.
        let pruned = match body.max_versions {
            Some(n) => crate::routes::snapshots::count_over_version_cap(
                &state.pool,
                &user_id,
                n,
                body.manual,
            )
            .await
            .map_err(|e| internal(e, "version-cap count"))?,
            None => 0,
        };
        return Ok(Json(MaxVersionsResponse {
            max_versions: body.max_versions,
            manual: body.manual,
            pruned,
        }));
    }

    // The column is picked by the flag, not interpolated into the SQL.
    let sql = if body.manual {
        "UPDATE users SET max_manual_versions = ? WHERE id = ?"
    } else {
        "UPDATE users SET max_versions = ? WHERE id = ?"
    };
    sqlx::query(sql)
        .bind(body.max_versions)
        .bind(&user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| internal(e.into(), "max_versions update"))?;

    let pruned = crate::routes::snapshots::prune_over_version_cap(&state.pool, &user_id, None)
        .await
        .map_err(|e| internal(e, "version-cap prune"))?;

    Ok(Json(MaxVersionsResponse {
        max_versions: body.max_versions,
        manual: body.manual,
        pruned,
    }))
}
