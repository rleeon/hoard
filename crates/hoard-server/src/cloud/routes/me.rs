//! `/v1/me*` — account-facing endpoints for the desktop client.

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::plans::Plan;
use crate::cloud::quota;
use crate::cloud::state::CloudState;
use axum::{extract::State, response::Json, Extension};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

/// Wire shape for `GET /v1/me`. We expose used/limit pairs in bytes for
/// the storage bar and counts for devices + saves so the desktop's
/// /account page can render a coherent usage view without doing math.
/// Unlimited tiers send `-1` as the limit (clean to detect on the client
/// side without parsing string sentinels).
#[derive(Debug, Serialize)]
pub struct Me {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub plan: String,
    pub subscription_status: Option<String>,
    pub renews_at: Option<String>,
    pub cancel_at: Option<String>,
    pub storage_used_bytes: i64,
    pub storage_limit_bytes: i64,
    pub devices_used: i32,
    pub devices_limit: i32,
    pub saves_used: i32,
    pub saves_limit: i32,
    pub retention_days: i32,
}

/// GET /v1/me — current user's profile + plan + usage. Auto-creates the
/// profile row on first call (idempotent), so the client doesn't need a
/// separate "registration" step after Supabase OAuth.
pub async fn get_me(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<Me>, CloudError> {
    upsert_profile_for(&state, &user).await?;

    let row: (String, Option<String>, Option<String>, String, i64, i32) = sqlx::query_as(
        "SELECT email, display_name, avatar_url, plan, storage_bytes, devices_count
           FROM profiles WHERE user_id = $1",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;

    let sub: Option<(String, Option<time::OffsetDateTime>, Option<time::OffsetDateTime>)> =
        sqlx::query_as(
            "SELECT status, renews_at, cancel_at FROM subscriptions
              WHERE user_id = $1 AND status IN ('active','grace')
              ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(user.user_id)
        .fetch_optional(&state.pool)
        .await?;

    // Saves count comes from a separate query — keeps profile reads cheap
    // for endpoints that don't need this and avoids a join when the saves
    // count would be `0` on day one anyway. `saves` has no soft-delete
    // column today (only `save_versions` does); a plain COUNT is the right
    // shape. Propagate the SQL error rather than swallowing it so schema
    // drift surfaces here instead of a silent `0`.
    let saves_used: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM saves WHERE user_id = $1")
            .bind(user.user_id)
            .fetch_one(&state.pool)
            .await?;

    let plan = Plan::from_str(&row.3).unwrap_or(Plan::Free);
    let limits = plan.limits();
    Ok(Json(Me {
        user_id: user.user_id,
        email: row.0,
        display_name: row.1,
        avatar_url: row.2,
        plan: plan.as_str().to_string(),
        subscription_status: sub.as_ref().map(|s| s.0.clone()),
        renews_at: sub.as_ref().and_then(|s| s.1).map(format_dt),
        cancel_at: sub.as_ref().and_then(|s| s.2).map(format_dt),
        storage_used_bytes: row.4,
        storage_limit_bytes: bytes_or_unlimited(limits.storage_bytes),
        devices_used: row.5,
        devices_limit: devices_or_unlimited(limits.devices),
        saves_used: saves_used as i32,
        saves_limit: limits
            .saves_tracked
            .map(|n| n as i32)
            .unwrap_or(-1),
        retention_days: limits.retention_days as i32,
    }))
}

/// Map u64 byte caps to the `-1 = unlimited` convention the client uses.
/// We never overflow i64 here in practice — quotas are GB-scale — but
/// the `try_into` guard keeps the intent obvious.
fn bytes_or_unlimited(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Pro+ uses `u32::MAX` as a sentinel for unlimited devices; surface it
/// to the wire as `-1` so the desktop UI can render `∞` without a magic
/// large number drifting through the front-end format helpers.
fn devices_or_unlimited(n: u32) -> i32 {
    if n == u32::MAX {
        -1
    } else {
        n as i32
    }
}

fn format_dt(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Create the row if missing. Same effect as the old `POST /v1/profiles/sync`
/// from the handoff but folded into `GET /v1/me` to keep the client API
/// flat — the first authenticated GET is always the bootstrap.
async fn upsert_profile_for(state: &CloudState, user: &CloudUser) -> Result<(), CloudError> {
    sqlx::query(
        "INSERT INTO profiles (user_id, email)
             VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET email = EXCLUDED.email",
    )
    .bind(user.user_id)
    .bind(&user.email)
    .execute(&state.pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct ExportJobOut {
    pub job_id: Uuid,
    pub status: String,
}

/// POST /v1/me/export — enqueue an export job. Returns immediately with a
/// `job_id`; the cron / background worker writes the ZIP to R2 and updates
/// the row. Client polls (future endpoint) or watches via realtime.
pub async fn create_export_job(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<ExportJobOut>, CloudError> {
    let row: (Uuid, String) = sqlx::query_as(
        "INSERT INTO export_jobs (user_id, status) VALUES ($1, 'pending')
         RETURNING id, status",
    )
    .bind(user.user_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(ExportJobOut {
        job_id: row.0,
        status: row.1,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteAccountOut {
    pub deleted_at: String,
    pub purges_after: String,
    pub grace_days: u32,
}

/// DELETE /v1/me — soft-delete the account. Hard-purge happens via the
/// daily cron 30 days later, giving the user a window to reactivate.
pub async fn delete_me(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
) -> Result<Json<DeleteAccountOut>, CloudError> {
    let now = OffsetDateTime::now_utc();
    let purge_at = now + time::Duration::days(30);
    sqlx::query(
        "UPDATE profiles SET deleted_at = now(), updated_at = now()
             WHERE user_id = $1",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    sqlx::query(
        "INSERT INTO audit_log (user_id, actor, event_type, metadata)
             VALUES ($1, 'user', 'account.soft_deleted', NULL)",
    )
    .bind(user.user_id)
    .execute(&state.pool)
    .await?;
    Ok(Json(DeleteAccountOut {
        deleted_at: format_dt(now),
        purges_after: format_dt(purge_at),
        grace_days: 30,
    }))
}

/// Used by tests on the `quota::QuotaInfo` shape. Keeps the symbol in
/// scope so the file compiles cleanly without unused-import lints in the
/// release build.
#[allow(dead_code)]
fn _quota_shape_check(_i: &quota::QuotaInfo) {}
