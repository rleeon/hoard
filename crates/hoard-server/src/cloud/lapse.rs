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
use crate::cloud::polar::PASS_GRACE_DAYS;
use crate::cloud::quota;
use sqlx::PgPool;
use uuid::Uuid;

/// The rows whose paid time is over. A pass runs [`PASS_GRACE_DAYS`] past its
/// `renews_at`; anything else ends on `renews_at` or on a scheduled
/// cancellation. `$grace` is the placeholder for the grace in days.
fn lapsed(grace: &str) -> String {
    format!(
        r#"status IN ('active','grace')
           AND ((renews_at IS NOT NULL
                 AND renews_at + make_interval(days => CASE WHEN "interval" = 'once' THEN {grace} ELSE 0 END) <= now())
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
        lapsed("$2")
    ))
    .bind(user_id)
    .bind(PASS_GRACE_DAYS as i32)
    .execute(pool)
    .await?
    .rows_affected();
    if expired == 0 {
        return Ok(false);
    }
    drop_if_unpaid(pool, user_id, storage_grace_days).await
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
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE subscriptions
            SET status = 'expired', refunded_at = COALESCE(refunded_at, now()),
                last_event = 'order.refunded', last_event_at = now(), updated_at = now()
          WHERE ls_subscription_id = $1
      RETURNING user_id",
    )
    .bind(row_id)
    .fetch_optional(pool)
    .await?;
    let Some(user_id) = user_id else {
        return Ok(None);
    };
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
        lapsed("$1")
    ))
    .bind(PASS_GRACE_DAYS as i32)
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
