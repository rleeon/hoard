//! Quota enforcement.
//!
//! Two consumers:
//!
//! 1. `check_storage`: called from upload handlers *before* presigning a
//!    PUT URL. Returns `Err(QuotaResponse)` (which serializes to 402) if
//!    the user would exceed their plan, otherwise `Ok(())`.
//! 2. `check_devices`: called from `/v1/profiles/sync` and similar
//!    bootstrap endpoints. Same shape.

use crate::cloud::errors::CloudError;
use crate::cloud::plans::{Plan, PlanLimits};
use crate::cloud::state::CloudState;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct QuotaInfo {
    pub plan: &'static str,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    pub devices_used: u32,
    pub devices_limit: u32,
}

#[derive(Debug, Serialize)]
pub struct QuotaResponse {
    pub error: &'static str,
    pub code: &'static str,
    pub plan: &'static str,
    pub used_bytes: u64,
    pub limit_bytes: u64,
    pub requested_bytes: u64,
    pub upgrade_url: String,
}

impl IntoResponse for QuotaResponse {
    fn into_response(self) -> Response {
        (StatusCode::PAYMENT_REQUIRED, Json(self)).into_response()
    }
}

/// Fetch plan + storage_bytes for a user. Returns None if no profile row
/// exists yet; callers should treat that as a 404 or "log in first".
pub async fn load(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(PlanLimits, QuotaInfo)>, CloudError> {
    let row: Option<(
        String,
        i64,
        i32,
        Option<i64>,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
    )> = sqlx::query_as(
        "SELECT plan, storage_bytes, devices_count, storage_limit_bytes, \
         storage_limit_change_at, first_pro_at FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((plan_s, used, devs, limit_override, change_at, first_pro_at)) = row else {
        return Ok(None);
    };
    let plan = Plan::from_str(&plan_s).unwrap_or(Plan::Free);
    let mut limits = plan.limits();
    // Devices are kept for life once bought; see `resolved_devices_limit`.
    limits.devices = super::plans::resolved_devices_limit(plan, first_pro_at.is_some());
    // Apply the per-user storage tier (Pro xN) and any live downgrade grace
    // grant. Overriding here means every downstream consumer, `check_storage`
    // included, which reads `limits.storage_bytes` directly, sees the
    // limit actually being enforced, not the one the plan column implies.
    limits.storage_bytes = super::plans::resolved_storage_limit(
        plan,
        limit_override,
        change_at.map(|t| t.unix_timestamp()),
        time::OffsetDateTime::now_utc().unix_timestamp(),
    );
    let info = QuotaInfo {
        plan: plan.as_str(),
        used_bytes: used.max(0) as u64,
        limit_bytes: limits.storage_bytes,
        devices_used: devs.max(0) as u32,
        devices_limit: limits.devices,
    };
    Ok(Some((limits, info)))
}

/// What [`settle_storage_limit`] did, so the caller can log it and tests can
/// assert on it without re-reading the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleOutcome {
    /// No profile row yet, so nothing to settle.
    NoProfile,
    /// The new limit fits the current footprint (or is bigger): applied now,
    /// any pending change cleared.
    Applied { limit_bytes: i64 },
    /// The new limit is *below* what the user already stores: the old limit is
    /// granted until `days` from now, then it shrinks to `target_bytes`.
    Scheduled { grant_bytes: i64, target_bytes: i64 },
    /// A window was already running; its deadline and grant stay put (only the
    /// target is refreshed). Keeps webhook retries and the `/v1/me` expiry
    /// sweep from pushing the deadline forward forever.
    AlreadyScheduled { target_bytes: i64 },
}

/// Settle a user's storage limit after a plan or tier change, given the plan
/// they're moving *to* and the tier size that plan grants (`new_override`,
/// bytes; `None` = the plan base). Called from the Polar webhook (subscription
/// active *and* expired) and from the `/v1/me` expiry sweep.
///
/// - Upgrade, or a downgrade that still fits the current footprint → apply
///   immediately and clear any pending change.
/// - Downgrade *below* the current footprint → don't shrink now. Freeze the
///   limit the user has today into `storage_limit_bytes` as an absolute grant,
///   stash the target in `pending_storage_limit_bytes`, and set the
///   `storage_limit_change_at` deadline `grace_days` out. Until then the user
///   keeps the larger limit, nothing is purged, and `/v1/me` counts down.
///
/// **The old plan is read from the row, not taken from the caller.** Reading it
/// from the incoming plan is what made this a no-op for the only downgrade that
/// matters: on Pro→Free both sides resolved to Free's 2 GB, `new_eff >=
/// current_eff` held, and the window was never scheduled, so the limit collapsed
/// the same second the webhook landed and the auto-purge ate the user's history
/// with no warning. So callers must **not** flip `profiles.plan` before calling
/// this.
pub async fn settle_storage_limit(
    pool: &PgPool,
    user_id: Uuid,
    new_plan: Plan,
    new_override: Option<i64>,
    grace_days: i64,
) -> Result<SettleOutcome, CloudError> {
    use crate::cloud::plans::{effective_storage_limit, resolved_storage_limit};

    // Promote a window that already came due, so what we read below is the
    // limit actually in force rather than a stale grant.
    apply_due_downgrade(pool, user_id).await?;

    let row: Option<(String, i64, Option<i64>, Option<time::OffsetDateTime>)> = sqlx::query_as(
        "SELECT plan, storage_bytes, storage_limit_bytes, storage_limit_change_at \
         FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((old_plan_s, used, current_override, change_at)) = row else {
        return Ok(SettleOutcome::NoProfile);
    };
    let old_plan = Plan::from_str(&old_plan_s).unwrap_or(Plan::Free);
    let now = time::OffsetDateTime::now_utc();
    let used = used.max(0);
    let current_eff = resolved_storage_limit(
        old_plan,
        current_override,
        change_at.map(|t| t.unix_timestamp()),
        now.unix_timestamp(),
    ) as i64;
    let new_eff = effective_storage_limit(new_plan, new_override) as i64;

    if new_eff >= current_eff || used <= new_eff {
        sqlx::query(
            "UPDATE profiles SET storage_limit_bytes = $1, pending_storage_limit_bytes = NULL, \
             storage_limit_change_at = NULL, updated_at = now() WHERE user_id = $2",
        )
        .bind(new_override)
        .bind(user_id)
        .execute(pool)
        .await?;
        return Ok(SettleOutcome::Applied {
            limit_bytes: new_eff,
        });
    }

    // A window is already running (the promote above cleared any due one, so
    // this is necessarily in the future): keep its deadline and its grant,
    // refresh only where it's headed.
    if change_at.is_some() {
        sqlx::query(
            "UPDATE profiles SET pending_storage_limit_bytes = $1, updated_at = now() \
             WHERE user_id = $2 AND storage_limit_change_at IS NOT NULL",
        )
        .bind(new_override)
        .bind(user_id)
        .execute(pool)
        .await?;
        return Ok(SettleOutcome::AlreadyScheduled {
            target_bytes: new_eff,
        });
    }

    // Freeze today's limit as an absolute grant. It has to be absolute because
    // the plan column is about to say "free", and Free's limit ignores the tier
    // override; see `plans::resolved_storage_limit`.
    sqlx::query(
        "UPDATE profiles SET storage_limit_bytes = $1, pending_storage_limit_bytes = $2, \
         storage_limit_change_at = now() + ($3::int * interval '1 day'), \
         updated_at = now() WHERE user_id = $4",
    )
    .bind(current_eff)
    .bind(new_override)
    .bind(grace_days.max(0) as i32)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(SettleOutcome::Scheduled {
        grant_bytes: current_eff,
        target_bytes: new_eff,
    })
}

/// Promote a pending downgrade whose grace window has elapsed into the live
/// limit. Idempotent and cheap; call before reading or enforcing the limit
/// (`get_me`, `maybe_purge`). Rows without a due change are untouched.
pub async fn apply_due_downgrade(pool: &PgPool, user_id: Uuid) -> Result<(), CloudError> {
    sqlx::query(
        "UPDATE profiles SET storage_limit_bytes = pending_storage_limit_bytes, \
         pending_storage_limit_bytes = NULL, storage_limit_change_at = NULL, updated_at = now() \
         WHERE user_id = $1 AND storage_limit_change_at IS NOT NULL \
         AND storage_limit_change_at <= now()",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// True if `used + extra` would exceed `limit`. Saturating add to avoid
/// pathological u64 overflow on a malformed request.
pub fn would_exceed(used: u64, extra: u64, limit: u64) -> bool {
    used.saturating_add(extra) > limit
}

pub fn quota_response(info: &QuotaInfo, requested: u64, upgrade_url: String) -> QuotaResponse {
    QuotaResponse {
        error: "storage quota exceeded",
        code: "quota_exceeded",
        plan: info.plan,
        used_bytes: info.used_bytes,
        limit_bytes: info.limit_bytes,
        requested_bytes: requested,
        upgrade_url,
    }
}

/// Helper used by upload handlers: load + check + return a quota-shaped
/// 402 response if exceeded.
pub async fn check_storage(
    state: &CloudState,
    user_id: Uuid,
    requested: u64,
) -> Result<QuotaInfo, Response> {
    let (limits, info) = match load(&state.pool, user_id).await {
        Ok(Some(x)) => x,
        Ok(None) => return Err(CloudError::NotFound("no profile").into_response()),
        Err(e) => return Err(e.into_response()),
    };
    if would_exceed(info.used_bytes, requested, limits.storage_bytes) {
        let url = state
            .config
            .cloud
            .as_ref()
            .map(|c| c.upgrade_url.clone())
            .unwrap_or_else(crate::config::default_upgrade_url);
        return Err(quota_response(&info, requested, url).into_response());
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn would_exceed_basic() {
        assert!(would_exceed(100, 10, 100));
        assert!(!would_exceed(100, 0, 100));
        assert!(!would_exceed(99, 1, 100));
        assert!(would_exceed(99, 2, 100));
    }

    #[test]
    fn would_exceed_overflow_safe() {
        // saturating_add(u64::MAX, 1) == u64::MAX; that should compare to
        // limit and exceed it.
        assert!(would_exceed(u64::MAX, 1, 100));
    }

    #[test]
    fn quota_response_body_shape() {
        let info = QuotaInfo {
            plan: "free",
            used_bytes: 400 * 1024 * 1024,
            limit_bytes: 500 * 1024 * 1024,
            devices_used: 1,
            devices_limit: 1,
        };
        let resp = quota_response(&info, 200 * 1024 * 1024, "https://x/upgrade".into());
        assert_eq!(resp.code, "quota_exceeded");
        assert_eq!(resp.plan, "free");
        assert_eq!(resp.requested_bytes, 200 * 1024 * 1024);
    }
}
