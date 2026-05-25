//! Rolling-window bandwidth limiter.
//!
//! Every Free / Pro account gets a 15-minute rolling quota (500 MB / 1 GB
//! respectively). We can't observe the bytes on-the-wire because the
//! payload goes directly to R2 via presigned URLs — so the design is:
//!
//! 1. Before minting a presigned URL, check the SUM of `bytes_used` over
//!    the last `bandwidth_window_secs`. If it would exceed the quota,
//!    return a structured 429 with `retry_after_seconds` derived from the
//!    oldest bucket about to fall off the window.
//! 2. After the upload commits (or the download presign mints — we credit
//!    optimistically since the URL has a TTL and the client *almost
//!    always* uses it), record the bytes against the current minute
//!    bucket. UPSERT on `(user_id, bucket_start)`.
//! 3. A periodic cron deletes rows older than 1 hour so the table stays
//!    O(users * 60).
//!
//! Bucket granularity is a minute — coarse enough to keep the row count
//! down, fine enough that retry_after is meaningful. The window query
//! sums at most ~15 rows per user.

use crate::cloud::plans::PlanLimits;
use crate::cloud::state::CloudState;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use tracing::warn;
use uuid::Uuid;

/// Wire shape of a 429 bandwidth-limit response.
#[derive(Debug, Serialize)]
pub struct BandwidthLimitResponse {
    pub error: &'static str,
    pub code: &'static str,
    pub plan: &'static str,
    pub limit_bytes: u64,
    pub used_bytes: u64,
    pub requested_bytes: u64,
    pub window_secs: u32,
    pub retry_after_seconds: u32,
}

impl IntoResponse for BandwidthLimitResponse {
    fn into_response(self) -> Response {
        let retry = self.retry_after_seconds;
        let mut resp = (StatusCode::TOO_MANY_REQUESTS, Json(self)).into_response();
        // RFC 6585 — clients should honour Retry-After. axum's typed header
        // helpers want a Duration; raw header is simpler here.
        if let Ok(v) = retry.to_string().parse() {
            resp.headers_mut().insert("Retry-After", v);
        }
        resp
    }
}

/// Truncate a timestamp to the start of its minute. We use this as the
/// PRIMARY KEY component, so the same bucket can be UPSERTed concurrently
/// from many uploads without lock contention beyond row-level.
fn floor_to_minute(t: OffsetDateTime) -> OffsetDateTime {
    let secs = t.second() as i64;
    let nanos = t.nanosecond() as i64;
    t - time::Duration::seconds(secs) - time::Duration::nanoseconds(nanos)
}

/// Sum bytes spent in the rolling window. Returns the running total.
pub async fn used_in_window(
    pool: &PgPool,
    user_id: Uuid,
    window_secs: u32,
) -> Result<u64, sqlx::Error> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(window_secs as i64);
    let row: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT SUM(bytes_used)::bigint
           FROM bandwidth_usage
          WHERE user_id = $1 AND bucket_start >= $2",
    )
    .bind(user_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .and_then(|r| r.0)
        .map(|n| n.max(0) as u64)
        .unwrap_or(0))
}

/// Record `bytes` against the current minute bucket. Idempotent for the
/// (user_id, bucket_start) row — concurrent calls just add into the
/// running counter atomically.
pub async fn record(
    pool: &PgPool,
    user_id: Uuid,
    bytes: u64,
) -> Result<(), sqlx::Error> {
    let bucket = floor_to_minute(OffsetDateTime::now_utc());
    sqlx::query(
        r#"
        INSERT INTO bandwidth_usage (user_id, bucket_start, bytes_used)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, bucket_start)
        DO UPDATE SET bytes_used = bandwidth_usage.bytes_used + EXCLUDED.bytes_used
        "#,
    )
    .bind(user_id)
    .bind(bucket)
    .bind(bytes as i64)
    .execute(pool)
    .await?;
    Ok(())
}

/// "How many seconds until the oldest bucket falls off the window?"
/// Used to populate `Retry-After`. Falls back to the full window if no
/// bucket rows exist (shouldn't happen on the 429 path, but defensive).
pub async fn retry_after_secs(
    pool: &PgPool,
    user_id: Uuid,
    window_secs: u32,
) -> u32 {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::seconds(window_secs as i64);
    let oldest: Option<(OffsetDateTime,)> = sqlx::query_as(
        "SELECT bucket_start FROM bandwidth_usage
          WHERE user_id = $1 AND bucket_start >= $2
          ORDER BY bucket_start ASC LIMIT 1",
    )
    .bind(user_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    match oldest {
        Some((t,)) => {
            let falls_off_at = t + time::Duration::seconds(window_secs as i64);
            let delta = falls_off_at - OffsetDateTime::now_utc();
            delta.whole_seconds().max(1).min(window_secs as i64) as u32
        }
        None => window_secs,
    }
}

/// Helper used by upload/download handlers: check the rolling window
/// against the user's plan quota. On overrun returns a structured 429
/// Response; on pass returns Ok with the running window total so callers
/// can log it.
pub async fn check(
    state: &CloudState,
    user_id: Uuid,
    requested: u64,
    limits: &PlanLimits,
) -> Result<u64, Response> {
    let used = match used_in_window(&state.pool, user_id, limits.bandwidth_window_secs).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, "bandwidth: window query failed");
            // Fail open — a DB hiccup shouldn't block a paid customer's
            // upload. The cron will catch any drift.
            return Ok(0);
        }
    };
    if used.saturating_add(requested) > limits.bandwidth_quota_bytes {
        let retry =
            retry_after_secs(&state.pool, user_id, limits.bandwidth_window_secs).await;
        let body = BandwidthLimitResponse {
            error: "bandwidth quota exceeded",
            code: "bandwidth_limit",
            plan: limits.plan.as_str(),
            limit_bytes: limits.bandwidth_quota_bytes,
            used_bytes: used,
            requested_bytes: requested,
            window_secs: limits.bandwidth_window_secs,
            retry_after_seconds: retry,
        };
        return Err(body.into_response());
    }
    Ok(used)
}

/// Delete rows older than 1h. Keeps the table bounded — at 60 rows/user/h
/// peak this is enough headroom for any window we'll ever set. Called by
/// the cleanup task spawned in `run.rs`.
pub async fn cleanup_old(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::hours(1);
    let res = sqlx::query("DELETE FROM bandwidth_usage WHERE bucket_start < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_to_minute_zeroes_subminute() {
        let t = time::macros::datetime!(2026-05-25 14:37:42.123 UTC);
        let f = floor_to_minute(t);
        assert_eq!(f, time::macros::datetime!(2026-05-25 14:37:00 UTC));
    }

    #[test]
    fn floor_to_minute_already_aligned() {
        let t = time::macros::datetime!(2026-05-25 14:37:00 UTC);
        assert_eq!(floor_to_minute(t), t);
    }
}
