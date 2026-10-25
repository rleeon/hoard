//! Paid plans that ran out, and the drop back to Free.
//!
//! Polar's webhook is meant to say when a subscription ends, but a delivery can
//! be late or lost, and a pass never gets one at all: it is paid once and only
//! we know it lasts a year plus [`PASS_GRACE_DAYS`]. So the end is also read
//! off the dates, from two places. `/v1/me` does it for its own account on
//! every poll, so whoever keeps the app open drops on time; the daily sweep
//! does it for everybody else, so an account nobody opens again does not stay
//! Pro forever.

use crate::cloud::plans::Plan;
use crate::cloud::polar::{chained_pass_end, PASS_GRACE_DAYS, PASS_INTERVAL};
use crate::cloud::quota;
use sqlx::PgPool;
use uuid::Uuid;

/// How long a subscription that should have renewed is still taken at its word.
///
/// Its `renews_at` is when Polar charges again, not when it stops: the webhook
/// with the new period comes after it, about a second later on the renewals on
/// record, longer when the charge fails (the past-due notices of 23-10-2026
/// came a quarter of an hour on), and as late as Polar's retries when our
/// server was down at the time. Ending it on the dot dropped a paying account
/// to Free for that while, with the farewell in the app and, between 80 % and
/// 100 % of Free's 2 GB, the auto-purge eating its history. A cancelled one has
/// no such wait: `cancel_at` is the end.
pub const RENEWAL_LATE_DAYS: i32 = 7;

/// The rows whose paid time is over. A pass runs [`PASS_GRACE_DAYS`] past its
/// `renews_at`, a subscription [`RENEWAL_LATE_DAYS`], and either ends on a
/// scheduled cancellation. `$grace` and `$late` are the placeholders for both,
/// in days.
fn lapsed(grace: &str, late: &str) -> String {
    format!(
        r#"status IN ('active','grace')
           AND ((renews_at IS NOT NULL
                 AND renews_at + make_interval(days => CASE WHEN "interval" = 'once' THEN {grace} ELSE {late} END) <= now())
                OR (cancel_at IS NOT NULL AND cancel_at <= now()))"#
    )
}

/// Expire what ran out for `user_id` and, if nothing paid is left, put the
/// account on Free. Returns whether it went to Free.
///
/// "Nothing left" matters: an account can hold two rows, a monthly
/// subscription and a pass bought on top, and the one that lapses is not its
/// only claim to Pro. Dropping to Free on the first lapse would take Pro away
/// from somebody who paid for it.
pub async fn expire_lapsed(
    pool: &PgPool,
    user_id: Uuid,
    storage_grace_days: i64,
) -> Result<bool, sqlx::Error> {
    let expired = sqlx::query(&format!(
        "UPDATE subscriptions SET status = 'expired', updated_at = now()
          WHERE user_id = $1 AND {}",
        lapsed("$2", "$3")
    ))
    .bind(user_id)
    .bind(PASS_GRACE_DAYS as i32)
    .bind(RENEWAL_LATE_DAYS)
    .execute(pool)
    .await?
    .rows_affected();
    if expired == 0 {
        return Ok(false);
    }
    drop_if_unpaid(pool, user_id, storage_grace_days).await
}

/// `(status, renews_at, cancel_at, interval)` of the row that stands for the
/// account's Pro, the one `/v1/me` describes.
pub type Standing = (
    String,
    Option<time::OffsetDateTime>,
    Option<time::OffsetDateTime>,
    String,
);

/// The row the account page talks about, when there are two: the one that goes
/// on paying, then the one that lasts longest. By `updated_at` alone, a monthly
/// plan cancelled after buying a pass came first, and the app told somebody who
/// had just paid for a year that Pro was ending with the month.
pub async fn standing(pool: &PgPool, user_id: Uuid) -> Result<Option<Standing>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT status, renews_at, cancel_at, "interval" FROM subscriptions
            WHERE user_id = $1 AND status IN ('active','grace')
            ORDER BY (status = 'active' AND cancel_at IS NULL) DESC,
                     COALESCE(cancel_at, renews_at + make_interval(
                         days => CASE WHEN "interval" = 'once' THEN $2 ELSE 0 END)) DESC NULLS LAST,
                     updated_at DESC
            LIMIT 1"#,
    )
    .bind(user_id)
    .bind(PASS_GRACE_DAYS as i32)
    .fetch_optional(pool)
    .await
}

/// What a refund took away.
#[derive(Debug, PartialEq, Eq)]
pub struct Revoked {
    pub user_id: Uuid,
    /// The account went to Free: nothing else pays for Pro.
    pub dropped: bool,
    /// The devices kept for life went too: no payment of the account stands.
    pub devices_revoked: bool,
}

/// Expire the row a refunded payment belongs to and drop the account to Free
/// if nothing else pays for Pro. `row_id` is the pass's order id or the
/// subscription's id. `None` when no row matches: a refund of something this
/// server never granted.
///
/// Unlimited devices are for life once an account has paid anything, a single
/// month included. So they go only when `first_payment` (the refund is of the
/// row's only payment, a pass or a subscription's first one) and no other row
/// of the account still stands unrefunded. Machines already paired keep
/// working either way; only the next one beyond the Free allowance is refused.
pub async fn revoke_refunded(
    pool: &PgPool,
    row_id: &str,
    storage_grace_days: i64,
    first_payment: bool,
) -> Result<Option<Revoked>, sqlx::Error> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"UPDATE subscriptions
            SET status = 'expired', refunded_at = COALESCE(refunded_at, now()),
                last_event = 'order.refunded', last_event_at = now(), updated_at = now()
          WHERE ls_subscription_id = $1
      RETURNING user_id, "interval""#,
    )
    .bind(row_id)
    .fetch_optional(pool)
    .await?;
    let Some((user_id, interval)) = row else {
        return Ok(None);
    };
    if interval == PASS_INTERVAL {
        rechain_passes(pool, user_id, row_id).await?;
    }
    let dropped = drop_if_unpaid(pool, user_id, storage_grace_days).await?;
    let devices_revoked = first_payment
        && sqlx::query(
            "UPDATE profiles SET first_pro_at = NULL, updated_at = now()
              WHERE user_id = $1 AND first_pro_at IS NOT NULL
                AND NOT EXISTS (SELECT 1 FROM subscriptions
                                 WHERE user_id = $1 AND refunded_at IS NULL)",
        )
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected()
            == 1;
    Ok(Some(Revoked {
        user_id,
        dropped,
        devices_revoked,
    }))
}

/// Take a refunded pass out of the account's chain of passes.
///
/// A pass bought before the previous one ran out starts where that one ends,
/// and the previous one is retired (`polar::supersede_passes`). So the refund
/// moves things on both sides. Every later pass loses the time it had carried
/// over from the refunded one. And when the refunded one was the newest, the
/// pass before it is the account's again, if its own year and grace month
/// aren't over: without that, refunding a renewal bought early dropped the
/// account to Free with months of a paid year still to run.
///
/// A later pass is redated from its row's `created_at`, the moment the webhook
/// wrote it, which follows the payment by seconds.
async fn rechain_passes(pool: &PgPool, user_id: Uuid, refunded: &str) -> Result<(), sqlx::Error> {
    type Pass = (
        String,
        time::OffsetDateTime,
        Option<time::OffsetDateTime>,
        Option<String>,
    );
    let passes: Vec<Pass> = sqlx::query_as(
        r#"SELECT ls_subscription_id, created_at, renews_at, last_event FROM subscriptions
            WHERE user_id = $1 AND "interval" = 'once'
              AND (refunded_at IS NULL OR ls_subscription_id = $2)
            ORDER BY created_at, id"#,
    )
    .bind(user_id)
    .bind(refunded)
    .fetch_all(pool)
    .await?;
    let Some(at) = passes.iter().position(|p| p.0 == refunded) else {
        return Ok(());
    };
    let (before, after) = (&passes[..at], &passes[at + 1..]);

    let mut end = before.last().and_then(|p| p.2);
    for (id, created, _, _) in after {
        let new_end = chained_pass_end(*created, end);
        sqlx::query(
            "UPDATE subscriptions SET renews_at = $2, updated_at = now()
              WHERE ls_subscription_id = $1",
        )
        .bind(id)
        .bind(new_end)
        .execute(pool)
        .await?;
        end = Some(new_end);
    }

    if !after.is_empty() {
        return Ok(());
    }
    let Some((id, _, Some(prev_end), last_event)) = before.last() else {
        return Ok(());
    };
    let still_pro =
        *prev_end + time::Duration::days(PASS_GRACE_DAYS) > time::OffsetDateTime::now_utc();
    if still_pro && last_event.as_deref() == Some("superseded") {
        sqlx::query(
            "UPDATE subscriptions SET status = 'active', last_event = 'restored',
                    last_event_at = now(), updated_at = now()
              WHERE ls_subscription_id = $1",
        )
        .bind(id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Put the account on Free unless another row still pays for Pro.
async fn drop_if_unpaid(
    pool: &PgPool,
    user_id: Uuid,
    storage_grace_days: i64,
) -> Result<bool, sqlx::Error> {
    let still_paid: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM subscriptions
                         WHERE user_id = $1 AND status IN ('active','grace'))",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if still_paid {
        return Ok(false);
    }
    // Grace window first, plan flip second: `settle_storage_limit` reads the
    // old plan off the row to size the grant (same ordering as the webhook).
    if let Err(e) =
        quota::settle_storage_limit(pool, user_id, Plan::Free, None, storage_grace_days).await
    {
        tracing::warn!(%user_id, error = ?e, "lapse: settling storage failed");
    }
    sqlx::query("UPDATE profiles SET plan = 'free', updated_at = now() WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(true)
}

/// [`expire_lapsed`] for every account that has something to expire.
pub async fn sweep(pool: &PgPool, storage_grace_days: i64) -> Result<usize, sqlx::Error> {
    let users: Vec<Uuid> = sqlx::query_scalar(&format!(
        "SELECT DISTINCT user_id FROM subscriptions WHERE {}",
        lapsed("$1", "$2")
    ))
    .bind(PASS_GRACE_DAYS as i32)
    .bind(RENEWAL_LATE_DAYS)
    .fetch_all(pool)
    .await?;
    let mut dropped = 0;
    for user_id in users {
        if expire_lapsed(pool, user_id, storage_grace_days).await? {
            dropped += 1;
        }
    }
    Ok(dropped)
}
