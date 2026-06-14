//! Pro entitlements + per-feature trial gating.
//!
//! The build-time `pro` feature only decides whether the Pro *code* is
//! compiled into the binary. The real lock is here: every Pro content endpoint
//! calls [`require_feature`] and gets a `402` unless the caller is on paid Pro
//! or inside an active one-month trial. Value is anchored server-side (wrapple
//! generated here, screen assets served from R2) so a patched OSS client can't
//! fake entitlement — it can show the locked UI but never produce the content.
//!
//! Two entry points:
//! - [`require_feature`] — mutating; used by content endpoints. Starts the
//!   trial on a Free account's first use (idempotent), then enforces it.
//! - [`feature_state`] — read-only; used by `GET /v1/cloud/entitlements` to
//!   paint the UI badge/lock. Never starts a trial.

use crate::cloud::errors::CloudError;
use crate::cloud::plans::Plan;
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Trial length for Free accounts, in days. One month.
pub const TRIAL_DAYS: i64 = 30;

/// Pro features that can be gated and trialed independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Screen,
    Wrapple,
}

impl Feature {
    pub fn as_str(self) -> &'static str {
        match self {
            Feature::Screen => "screen",
            Feature::Wrapple => "wrapple",
        }
    }

    pub fn from_str(s: &str) -> Option<Feature> {
        match s {
            "screen" => Some(Feature::Screen),
            "wrapple" => Some(Feature::Wrapple),
            _ => None,
        }
    }
}

/// Resolved access state for one `(user, feature)`, as sent to the UI.
/// Serializes with a `state` tag, e.g. `{"state":"trial","expires_at":"..."}`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FeatureState {
    /// Paid Pro — full access, no expiry.
    Entitled,
    /// Free account that hasn't used the feature yet; the trial will start on
    /// first use. `days` is the full trial length.
    TrialAvailable { days: i64 },
    /// Free account inside its active trial.
    Trial { expires_at: String },
    /// Free account whose trial has elapsed. UI renders this as "función Pro".
    TrialExpired,
}

/// Pure decision so the branching is unit-testable without a database.
fn decide(plan: Plan, trial_expires_at: Option<OffsetDateTime>, now: OffsetDateTime) -> FeatureState {
    if plan == Plan::Pro {
        return FeatureState::Entitled;
    }
    match trial_expires_at {
        None => FeatureState::TrialAvailable { days: TRIAL_DAYS },
        Some(exp) if now < exp => FeatureState::Trial {
            expires_at: fmt_dt(exp),
        },
        Some(_) => FeatureState::TrialExpired,
    }
}

/// Gate a Pro *content* endpoint. Paid Pro passes immediately. A Free account
/// gets its trial started on first call (idempotent via `ON CONFLICT`), then is
/// allowed while inside the window and rejected with `402` once it elapses.
pub async fn require_feature(
    pool: &PgPool,
    user_id: Uuid,
    plan: Plan,
    feature: Feature,
) -> Result<(), CloudError> {
    if plan == Plan::Pro {
        return Ok(());
    }
    let now = OffsetDateTime::now_utc();
    let expires = now + time::Duration::days(TRIAL_DAYS);
    // Start the trial on first use; ON CONFLICT keeps the original window so the
    // clock can never be restarted by hammering the endpoint.
    sqlx::query(
        "INSERT INTO pro_trials (user_id, feature, expires_at)
             VALUES ($1, $2, $3)
         ON CONFLICT (user_id, feature) DO NOTHING",
    )
    .bind(user_id)
    .bind(feature.as_str())
    .bind(expires)
    .execute(pool)
    .await?;

    let (expires_at,): (OffsetDateTime,) = sqlx::query_as(
        "SELECT expires_at FROM pro_trials WHERE user_id = $1 AND feature = $2",
    )
    .bind(user_id)
    .bind(feature.as_str())
    .fetch_one(pool)
    .await?;

    if now < expires_at {
        Ok(())
    } else {
        Err(CloudError::PaymentRequired {
            feature: feature.as_str(),
        })
    }
}

/// Read-only entitlement snapshot for the UI. Never starts a trial.
pub async fn feature_state(
    pool: &PgPool,
    user_id: Uuid,
    plan: Plan,
    feature: Feature,
) -> Result<FeatureState, CloudError> {
    if plan == Plan::Pro {
        return Ok(FeatureState::Entitled);
    }
    let row: Option<(OffsetDateTime,)> = sqlx::query_as(
        "SELECT expires_at FROM pro_trials WHERE user_id = $1 AND feature = $2",
    )
    .bind(user_id)
    .bind(feature.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(decide(plan, row.map(|r| r.0), OffsetDateTime::now_utc()))
}

/// The caller's current plan, read from `profiles`. Defaults to Free when the
/// profile row doesn't exist yet (e.g. the client hit `/entitlements` before
/// the first `GET /v1/me` that bootstraps the row).
pub async fn current_plan(pool: &PgPool, user_id: Uuid) -> Result<Plan, CloudError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT plan FROM profiles WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| Plan::from_str(&r.0)).unwrap_or(Plan::Free))
}

fn fmt_dt(dt: OffsetDateTime) -> String {
    dt.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(offset_days: i64) -> OffsetDateTime {
        OffsetDateTime::now_utc() + time::Duration::days(offset_days)
    }

    #[test]
    fn pro_is_always_entitled() {
        let s = decide(Plan::Pro, None, OffsetDateTime::now_utc());
        assert!(matches!(s, FeatureState::Entitled));
        // Even with a long-expired trial row, paid Pro stays entitled.
        let s = decide(Plan::Pro, Some(t(-100)), OffsetDateTime::now_utc());
        assert!(matches!(s, FeatureState::Entitled));
    }

    #[test]
    fn free_never_used_is_trial_available() {
        let s = decide(Plan::Free, None, OffsetDateTime::now_utc());
        assert!(matches!(s, FeatureState::TrialAvailable { days } if days == TRIAL_DAYS));
    }

    #[test]
    fn free_within_window_is_trial() {
        let s = decide(Plan::Free, Some(t(5)), OffsetDateTime::now_utc());
        assert!(matches!(s, FeatureState::Trial { .. }));
    }

    #[test]
    fn free_after_window_is_expired() {
        let s = decide(Plan::Free, Some(t(-1)), OffsetDateTime::now_utc());
        assert!(matches!(s, FeatureState::TrialExpired));
    }
}
