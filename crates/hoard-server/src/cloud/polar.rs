//! Polar webhooks: the billing provider. Polar is the Merchant of Record
//! and settles through Stripe underneath, so "the Stripe fee" and "the
//! Polar fee" are the same line item.
//!
//! Historical note: billing started on Lemon Squeezy and the schema still
//! carries its fingerprints (`subscriptions`, the 0015 variant→product
//! rename). Lemon Squeezy is not used: there is no LS code path, no LS
//! config field, and nothing reads the `HOARD__CLOUD__LEMONSQUEEZY__*`
//! secrets. Treat any surviving mention as dead history, not a second
//! provider to keep working.
//!
//! Polar signs with the Standard Webhooks scheme, not the hex `X-Signature`
//! Lemon Squeezy used. Three headers travel with each delivery:
//! `webhook-id`, `webhook-timestamp`, `webhook-signature`. The signed
//! content is `"{id}.{timestamp}.{raw_body}"`, HMAC-SHA256, base64-encoded,
//! and the signature header carries one or more space-separated
//! `v1,<base64sig>` tokens.
//!
//! Key derivation gotcha: Polar's SDK base64-*encodes* the `polar_whs_...`
//! secret string before handing it to the Standard Webhooks verifier, which
//! then base64-*decodes* it back. The net effect is that the HMAC key is the
//! raw secret string bytes verbatim, so we key directly on
//! `secret.as_bytes()` (the whole `polar_whs_...` string), no decoding.
//!
//! Events map onto the `subscriptions` table + `profiles.plan` cascade the
//! schema inherited from the Lemon Squeezy era.

use crate::cloud::errors::CloudError;
use crate::cloud::product_events::Event;
use crate::cloud::state::CloudState;
use crate::config::PolarProduct;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use hoard_core::product::ProductEvent;
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgPool;
use std::str;
use tracing::{info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// How far the `webhook-timestamp` may drift from our clock before we reject
/// the delivery as a possible replay. The timestamp is inside the signed
/// content (so it's authenticated), but without a freshness window a captured
/// valid delivery could be re-POSTed forever to re-apply a stale subscription
/// state. Standard Webhooks recommends ±5 minutes.
const WEBHOOK_TOLERANCE_SECS: i64 = 5 * 60;

#[derive(Debug, Deserialize)]
pub struct PolarEvent {
    /// e.g. "subscription.created", "subscription.active", "order.paid".
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: PolarData,
}

/// We only deserialise the fields we act on. Polar payloads carry far more;
/// unknown fields are ignored. For subscription.* events `data` is the
/// subscription object; product_id / customer_id / status / metadata are the
/// pieces we route on.
#[derive(Debug, Deserialize)]
pub struct PolarData {
    /// Subscription id for subscription.* events, order id for a pass. Used as
    /// the upsert conflict key, stored in `ls_subscription_id`.
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub product_id: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    /// Period end → mirrored to `renews_at`.
    #[serde(default)]
    pub current_period_end: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    /// Present on the orders a subscription bills; absent on the outright
    /// purchase of a pass. It is how a renewal is told from a sale.
    #[serde(default)]
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    /// On an order: `purchase`, `subscription_create` (the first month or
    /// year), `subscription_cycle` (a renewal) or `subscription_update`.
    #[serde(default)]
    pub billing_reason: Option<String>,
    /// Custom metadata set at checkout creation. We carry our user_id here,
    /// the same way Lemon Squeezy uses custom_data.user_id.
    #[serde(default)]
    pub metadata: Option<PolarMetadata>,
    /// Customer object. `external_id` is the recommended place to stash our
    /// internal user id; used as a fallback when metadata is absent.
    #[serde(default)]
    pub customer: Option<PolarCustomer>,
}

#[derive(Debug, Deserialize)]
pub struct PolarMetadata {
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PolarCustomer {
    #[serde(default)]
    pub external_id: Option<String>,
}

/// Verify a Polar Standard Webhooks signature. Returns true only on a
/// matching `v1` token. `signature_header` may contain several
/// space-separated `v1,<base64>` entries (key rotation); any match passes.
pub fn verify_signature(
    body: &[u8],
    secret: &str,
    webhook_id: &str,
    webhook_ts: &str,
    signature_header: &str,
) -> bool {
    if secret.is_empty() {
        return false;
    }
    let body_str = match str::from_utf8(body) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(webhook_id.as_bytes());
    mac.update(b".");
    mac.update(webhook_ts.as_bytes());
    mac.update(b".");
    mac.update(body_str.as_bytes());
    let expected = mac.finalize().into_bytes();

    for token in signature_header.split(' ') {
        // Each token is "v1,<base64sig>"; tolerate a bare base64 too.
        let sig_b64 = token.split_once(',').map(|(_, s)| s).unwrap_or(token);
        let got = match STANDARD.decode(sig_b64.trim()) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if got.len() == expected.len() {
            let mut diff: u8 = 0;
            for (a, b) in got.iter().zip(expected.iter()) {
                diff |= a ^ b;
            }
            if diff == 0 {
                return true;
            }
        }
    }
    false
}

/// Current Unix time in seconds. Pulled out so tests can inject a fixed `now`.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Is `webhook_ts` (a Unix-seconds string) within [`WEBHOOK_TOLERANCE_SECS`] of
/// `now`? An unparsable or empty timestamp fails closed.
fn timestamp_is_fresh(webhook_ts: &str, now: i64) -> bool {
    let ts: i64 = match webhook_ts.trim().parse() {
        Ok(t) => t,
        Err(_) => return false,
    };
    (now - ts).abs() <= WEBHOOK_TOLERANCE_SECS
}

/// Map a Polar event (and the subscription's own status) onto our
/// `subscriptions.status` enum: 'active' | 'grace' | 'expired'.
///
/// - `*.created` / `*.active` / `*.updated` / `*.uncanceled` defer to the
///   embedded status field.
/// - `*.canceled` → 'grace': the user cancelled but keeps access until the
///   period ends (Polar leaves status 'active' with cancel_at_period_end).
/// - `*.past_due` → 'grace'.
/// - `*.revoked` → 'expired': access pulled, period over.
pub fn status_for_event(event_type: &str, data_status: Option<&str>) -> Option<&'static str> {
    match event_type {
        "subscription.created"
        | "subscription.active"
        | "subscription.updated"
        | "subscription.uncanceled" => Some(match data_status {
            Some("past_due") | Some("unpaid") | Some("incomplete") => "grace",
            Some("canceled") | Some("revoked") => "expired",
            _ => "active",
        }),
        "subscription.canceled" | "subscription.past_due" => Some("grace"),
        "subscription.revoked" => Some("expired"),
        _ => None,
    }
}

/// Resolve a Polar product UUID → (plan, interval) from config.
pub fn resolve_product<'a>(
    products: &'a [PolarProduct],
    product_id: &str,
) -> Option<(&'a str, &'a str)> {
    products
        .iter()
        .find(|x| x.product_id == product_id)
        .map(|x| (x.plan.as_str(), x.interval.as_str()))
}

/// The `interval` a product sold outright carries in config. A card that can
/// be charged again is exactly what stops some people from paying at all, so
/// this one runs out instead of renewing.
pub const PASS_INTERVAL: &str = "once";

const PASS_DAYS: i64 = 365;

/// Pro outlives a pass by this much. The year ends on `renews_at`, the account
/// stays Pro for another month, and only then goes back to Free; the reminders
/// in `cloud::notify` run through that month.
pub const PASS_GRACE_DAYS: i64 = 30;

/// Whether this delivery is the outright purchase of a pass rather than the
/// order a subscription bills: Polar sends `order.paid` for both, and only the
/// renewal carries a subscription id. The product still has to be configured
/// as a pass; that check needs config and happens in [`handle`].
fn is_pass_order(event_type: &str, subscription_id: Option<&str>) -> bool {
    event_type == "order.paid" && subscription_id.is_none()
}

/// When a pass paid at `paid` runs out, given the end of the pass the account
/// already has.
///
/// Bought while that one still grants Pro, in its year or in the month after,
/// the new year follows straight on from the old one. Someone who renews 25
/// days into the grace month has had those 25 days of Pro already, and they are
/// not handed out a second time. Bought after Pro ran out, it starts the day it
/// was paid.
pub fn chained_pass_end(
    paid: time::OffsetDateTime,
    previous_end: Option<time::OffsetDateTime>,
) -> time::OffsetDateTime {
    match previous_end {
        Some(end) if paid < end + time::Duration::days(PASS_GRACE_DAYS) => {
            end + time::Duration::days(PASS_DAYS)
        }
        _ => paid + time::Duration::days(PASS_DAYS),
    }
}

/// When the pass bought by `order_id` runs out.
///
/// A retry of an order already written keeps the date it got the first time.
/// Recomputing would go wrong: the pass it continued is superseded by then and
/// no longer found, so the year would restart from the payment date.
///
/// `paid` is the order's own `created_at`, not now, for the same reason: Polar
/// retries a delivery it thinks failed, hours later sometimes.
pub async fn pass_end_for(
    pool: &PgPool,
    user_id: Uuid,
    order_id: &str,
    paid: time::OffsetDateTime,
) -> Result<time::OffsetDateTime, sqlx::Error> {
    let written: Option<Option<time::OffsetDateTime>> =
        sqlx::query_scalar("SELECT renews_at FROM subscriptions WHERE ls_subscription_id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await?;
    if let Some(Some(end)) = written {
        return Ok(end);
    }
    let previous: Option<time::OffsetDateTime> = sqlx::query_scalar(
        r#"SELECT max(renews_at) FROM subscriptions
            WHERE user_id = $1 AND "interval" = 'once'
              AND status IN ('active','grace') AND ls_subscription_id <> $2"#,
    )
    .bind(user_id)
    .bind(order_id)
    .fetch_one(pool)
    .await?;
    Ok(chained_pass_end(paid, previous))
}

/// Retire every other pass of the account once a new one is written. The new
/// row already carries their remaining time, so they have nothing left to
/// grant, and leaving them active is what would drop the account to Free the
/// day the old one lapsed.
pub async fn supersede_passes(
    pool: &PgPool,
    user_id: Uuid,
    order_id: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        r#"UPDATE subscriptions
              SET status = 'expired', last_event = 'superseded',
                  last_event_at = now(), updated_at = now()
            WHERE user_id = $1 AND "interval" = 'once'
              AND status IN ('active','grace') AND ls_subscription_id <> $2"#,
    )
    .bind(user_id)
    .bind(order_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// A refunded order takes Pro away, whatever it paid for. The row is found by
/// what the order bought: a pass is stored under its own order id, and the
/// orders of a subscription point at the subscription.
///
/// A partial refund is left alone. It is a gesture on a payment that still
/// stands, not the payment going back.
async fn refund(
    state: &CloudState,
    storage_grace_days: i64,
    event: &PolarEvent,
) -> axum::response::Response {
    if event.data.status.as_deref() != Some("refunded") {
        info!(
            order = %event.data.id,
            status = ?event.data.status,
            "polar webhook: partial refund, Pro left in place"
        );
        return (StatusCode::OK, "ignored").into_response();
    }
    let row_id = event
        .data
        .subscription_id
        .as_deref()
        .unwrap_or(&event.data.id);
    // Whether the money going back may be all this row ever took: a pass, or
    // the first payment of a subscription. A refunded renewal leaves earlier
    // months paid, and with them the devices kept for life.
    let first_payment = event.data.subscription_id.is_none()
        || event.data.billing_reason.as_deref() == Some("subscription_create");
    match crate::cloud::lapse::revoke_refunded(
        &state.pool,
        row_id,
        storage_grace_days,
        first_payment,
    )
    .await
    {
        Ok(Some(r)) => {
            let user_id = r.user_id;
            info!(
                %user_id, row_id, dropped = r.dropped, devices_revoked = r.devices_revoked,
                "polar webhook: refund revoked Pro"
            );
            Event::new(user_id, ProductEvent::SubscriptionEnded)
                .scope(row_id)
                .props(serde_json::json!({ "reason": "refund" }))
                .record(&state.pool)
                .await;
            (StatusCode::OK, "revoked").into_response()
        }
        Ok(None) => {
            warn!(row_id, "polar webhook: refund of something never granted");
            (StatusCode::OK, "ignored").into_response()
        }
        Err(e) => {
            warn!(error = %e, "polar webhook: refund revocation failed");
            CloudError::Db(e).into_response()
        }
    }
}

/// Whether a refund on this subscription still stands against an incoming
/// event that would set it to `status`, ending at `renews_at`.
///
/// It stands against everything but a new paid period, `active` with an end
/// past the refunded one. Cancelling in Polar sends `subscription.updated`
/// still `active` until the period ends, and letting that through would give
/// back the Pro the refund took. A new period is a new payment: the mark is
/// cleared and the event goes through.
pub async fn refund_holds(
    pool: &PgPool,
    row_id: &str,
    status: &str,
    renews_at: Option<&str>,
) -> Result<bool, sqlx::Error> {
    use time::format_description::well_known::Rfc3339;
    let row: Option<(Option<time::OffsetDateTime>, Option<time::OffsetDateTime>)> =
        sqlx::query_as(
            "SELECT refunded_at, renews_at FROM subscriptions WHERE ls_subscription_id = $1",
        )
        .bind(row_id)
        .fetch_optional(pool)
        .await?;
    let Some((Some(_), refunded_end)) = row else {
        return Ok(false);
    };
    let new_end = renews_at.and_then(|s| time::OffsetDateTime::parse(s, &Rfc3339).ok());
    let new_period = status == "active"
        && match (new_end, refunded_end) {
            (Some(new), Some(old)) => new > old,
            (Some(_), None) => true,
            _ => false,
        };
    if !new_period {
        return Ok(true);
    }
    sqlx::query("UPDATE subscriptions SET refunded_at = NULL WHERE ls_subscription_id = $1")
        .bind(row_id)
        .execute(pool)
        .await?;
    Ok(false)
}

/// The account a Polar payload is about: the metadata we stamp at checkout
/// creation, falling back to the customer's `external_id`. Both are set by
/// `routes::checkout`, so anything born from our own checkout carries one.
fn user_id_of(event: &PolarEvent) -> Option<Uuid> {
    event
        .data
        .metadata
        .as_ref()
        .and_then(|m| m.user_id.as_deref())
        .or_else(|| {
            event
                .data
                .customer
                .as_ref()
                .and_then(|c| c.external_id.as_deref())
        })
        .and_then(|u| Uuid::parse_str(u).ok())
}

/// Append a checkout event to `audit_log`.
///
/// Best-effort, and the caller answers 200 either way: nothing downstream reads
/// these rows, so a delivery we fail to store is a gap in the funnel, not a
/// reason for Polar to keep retrying. A checkout opened outside our own flow
/// has no user id to attach and is dropped with a warning rather than guessed
/// at, because a row on the wrong account is worse than a missing one.
async fn record_checkout_event(pool: &PgPool, event: &PolarEvent) {
    let Some(user_id) = user_id_of(event) else {
        warn!(
            event = %event.event_type,
            "polar webhook: checkout event with no user id, not recorded"
        );
        return;
    };
    let meta = serde_json::json!({
        "checkout_id": event.data.id,
        "status": event.data.status,
        "product_id": event.data.product_id,
    })
    .to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO audit_log (user_id, actor, event_type, metadata)
             VALUES ($1, 'polar', $2, $3::jsonb)",
    )
    .bind(user_id)
    .bind(&event.event_type)
    .bind(meta)
    .execute(pool)
    .await
    {
        warn!(
            error = %e, %user_id, event = %event.event_type,
            "polar webhook: couldn't record the checkout event"
        );
    }
    // Polar retries a delivery it thinks failed; the checkout id and status
    // make the retry the same row.
    let status = event.data.status.as_deref().unwrap_or_default();
    Event::new(user_id, ProductEvent::CheckoutStatus)
        .scope(format!("{}:{status}", event.data.id))
        .props(serde_json::json!({
            "checkout_id": event.data.id,
            "status": event.data.status,
        }))
        .record(pool)
        .await;
}

/// The subscription's turn in `product_events`: once per subscription for each
/// of start, cancellation and end. Renewals come in as `subscription.updated`
/// with the same id and collapse into the start.
async fn record_subscription_event(
    pool: &PgPool,
    user_id: Uuid,
    sub_id: &str,
    event_type: &str,
    status: &str,
    plan: &str,
    interval: &str,
) {
    let event = match (event_type, status) {
        ("subscription.canceled", _) => ProductEvent::SubscriptionCanceled,
        (_, "active") => ProductEvent::SubscriptionStarted,
        (_, "expired") => ProductEvent::SubscriptionEnded,
        _ => return,
    };
    Event::new(user_id, event)
        .scope(sub_id)
        .props(serde_json::json!({ "plan": plan, "interval": interval }))
        .record(pool)
        .await;
}

pub async fn handle(
    State(state): State<CloudState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let cloud = match state.config.cloud.as_ref() {
        Some(c) => c,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "cloud config missing").into_response(),
    };

    let hget = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
    };
    let webhook_id = hget("webhook-id");
    let webhook_ts = hget("webhook-timestamp");
    let signature = hget("webhook-signature");
    if signature.is_empty() {
        return (StatusCode::UNAUTHORIZED, "missing signature").into_response();
    }
    // Replay guard: reject deliveries whose timestamp is stale or skewed. Done
    // before signature verification so we don't even HMAC an obviously-replayed
    // body. (The signature still covers the timestamp, so this can't be forged
    // to slip a stale event through.)
    if !timestamp_is_fresh(webhook_ts, now_unix()) {
        warn!(
            webhook_ts,
            "polar webhook: stale/invalid timestamp, rejecting"
        );
        return (StatusCode::UNAUTHORIZED, "stale timestamp").into_response();
    }
    if !verify_signature(
        &body,
        &cloud.polar.webhook_secret,
        webhook_id,
        webhook_ts,
        signature,
    ) {
        warn!("polar webhook: bad signature");
        return (StatusCode::UNAUTHORIZED, "bad signature").into_response();
    }

    let event: PolarEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "polar webhook: malformed body");
            return (StatusCode::BAD_REQUEST, "malformed body").into_response();
        }
    };

    // Checkout events grant nothing (`subscription.*` below is what moves a
    // plan), but they are the only place an abandoned payment leaves a mark.
    // Polar opens a session with `checkout.created` and then `checkout.updated`
    // carries it to `succeeded`, `expired` or `failed`. Paired with the
    // `checkout.requested` row that `routes::checkout` writes, that is what
    // separates "nobody presses the button" from "they press it and don't
    // finish", which are opposite problems and were indistinguishable while
    // `status_for_event` dropped these on the floor.
    if event.event_type.starts_with("checkout.") {
        record_checkout_event(&state.pool, &event).await;
        return (StatusCode::OK, "recorded").into_response();
    }

    if event.event_type == "order.refunded" {
        return refund(&state, cloud.storage_downgrade_grace_days as i64, &event).await;
    }

    // A pass is paid once and arrives as `order.paid`, an event no
    // subscription state can be read off. Polar sends that same event for
    // every renewal of a real subscription, hence the two guards: an order
    // that belongs to no subscription, and (below, once config has spoken) a
    // product actually sold as a pass.
    let paid_order = is_pass_order(&event.event_type, event.data.subscription_id.as_deref());

    let status = match status_for_event(&event.event_type, event.data.status.as_deref()) {
        Some(s) => s,
        None if paid_order => "active",
        None => {
            info!(event = %event.event_type, "ignored polar event");
            return (StatusCode::OK, "ignored").into_response();
        }
    };

    let user_id = match user_id_of(&event) {
        Some(u) => u,
        None => {
            warn!(
                "polar webhook: missing/invalid user_id (metadata.user_id / customer.external_id)"
            );
            return (StatusCode::BAD_REQUEST, "missing user_id").into_response();
        }
    };

    let product_id = match event.data.product_id.as_deref() {
        Some(p) => p,
        None => {
            warn!("polar webhook: missing product_id");
            return (StatusCode::BAD_REQUEST, "missing product_id").into_response();
        }
    };
    let (plan, interval) = match resolve_product(&cloud.polar.products, product_id) {
        Some(x) => x,
        None => {
            warn!(product_id, "polar webhook: unknown product_id");
            return (StatusCode::BAD_REQUEST, "unknown product_id").into_response();
        }
    };

    if paid_order && interval != PASS_INTERVAL {
        info!(
            product_id,
            "polar webhook: order for a subscription product, left to its subscription events"
        );
        return (StatusCode::OK, "ignored").into_response();
    }

    let pass_end = if paid_order {
        use time::format_description::well_known::Rfc3339;
        let paid = event
            .data
            .created_at
            .as_deref()
            .and_then(|s| time::OffsetDateTime::parse(s, &Rfc3339).ok())
            .unwrap_or_else(time::OffsetDateTime::now_utc);
        match pass_end_for(&state.pool, user_id, &event.data.id, paid).await {
            Ok(end) => Some(end.format(&Rfc3339).unwrap_or_default()),
            Err(e) => {
                warn!(error = %e, "polar webhook: couldn't date the pass");
                return CloudError::Db(e).into_response();
            }
        }
    } else {
        None
    };
    let renews_at = pass_end
        .as_deref()
        .or(event.data.current_period_end.as_deref())
        .or(event.data.ends_at.as_deref());

    if !paid_order {
        match refund_holds(&state.pool, &event.data.id, status, renews_at).await {
            Ok(false) => {}
            Ok(true) => {
                info!(
                    subscription = %event.data.id,
                    event = %event.event_type,
                    "polar webhook: refunded subscription, ignored until a new period is paid"
                );
                return (StatusCode::OK, "refunded").into_response();
            }
            Err(e) => {
                warn!(error = %e, "polar webhook: couldn't check for a refund");
                return CloudError::Db(e).into_response();
            }
        }
    }

    if let Err(e) = upsert_subscription(
        &state.pool,
        user_id,
        &event.data.id,
        event.data.customer_id.as_deref().unwrap_or(""),
        product_id,
        plan,
        interval,
        status,
        renews_at,
        event.data.ends_at.as_deref(),
        &event.event_type,
    )
    .await
    {
        warn!(error = %e, "polar webhook: db upsert failed");
        return CloudError::Db(e).into_response();
    }
    if paid_order {
        if let Err(e) = supersede_passes(&state.pool, user_id, &event.data.id).await {
            warn!(error = %e, "polar webhook: couldn't retire the previous pass");
            return CloudError::Db(e).into_response();
        }
    }
    record_subscription_event(
        &state.pool,
        user_id,
        &event.data.id,
        &event.event_type,
        status,
        plan,
        interval,
    )
    .await;

    // Order matters in both branches: `settle_storage_limit` reads the *old*
    // plan off the profile row to know how much room the user has today, so the
    // `plan` column can only be flipped after it has run. Doing it the other way
    // round is what silently disabled the Pro→Free grace window.
    if status == "active" {
        // Storage tier (Pro xN) this product grants. `None` (base/unconfigured)
        // means the plan default. Upgrades apply at once; a downgrade below the
        // user's footprint gets a grace window before it shrinks; see
        // `settle_storage_limit`. (Downgrades should be issued with
        // `proration_behavior=next_period` so the user also keeps the bigger
        // tier until their paid period ends.)
        let storage_override: Option<i64> = cloud
            .polar
            .storage_bytes_for_product(product_id)
            .map(|b| b as i64);
        let plan_enum =
            crate::cloud::plans::Plan::from_str(plan).unwrap_or(crate::cloud::plans::Plan::Pro);
        match crate::cloud::quota::settle_storage_limit(
            &state.pool,
            user_id,
            plan_enum,
            storage_override,
            cloud.storage_downgrade_grace_days as i64,
        )
        .await
        {
            Ok(outcome) => {
                info!(?outcome, user_id = %user_id, "polar webhook: storage tier settled")
            }
            Err(e) => warn!(error = ?e, "polar webhook: settling storage tier failed"),
        }
        // `first_pro_at` is a one-way marker: COALESCE keeps the *first* time
        // this account ever went Pro, so a re-subscribe doesn't reset it and a
        // later lapse doesn't clear it. It's what lets a downgraded account keep
        // its device allowance for life (`plans::resolved_devices_limit`).
        let becoming_pro = crate::cloud::plans::Plan::from_str(plan)
            .is_some_and(|p| p != crate::cloud::plans::Plan::Free);
        if let Err(e) = sqlx::query(
            "UPDATE profiles SET plan = $1, \
             first_pro_at = CASE WHEN $3 THEN COALESCE(first_pro_at, now()) ELSE first_pro_at END, \
             updated_at = now() WHERE user_id = $2",
        )
        .bind(plan)
        .bind(user_id)
        .bind(becoming_pro)
        .execute(&state.pool)
        .await
        {
            warn!(error = %e, "polar webhook: profile plan update failed");
        }
    } else if status == "expired" {
        // Subscription expired: drop to Free. If the user is over Free's limit
        // they keep their Pro room for `storage_downgrade_grace_days`, with
        // nothing purged and `/v1/me` counting down, before it shrinks to 2 GB.
        match crate::cloud::quota::settle_storage_limit(
            &state.pool,
            user_id,
            crate::cloud::plans::Plan::Free,
            None, // Free has no per-user tier override
            cloud.storage_downgrade_grace_days as i64,
        )
        .await
        {
            Ok(outcome) => {
                info!(?outcome, user_id = %user_id, "polar webhook: storage settled on expiry")
            }
            Err(e) => warn!(error = ?e, "polar webhook: settling storage on expiry failed"),
        }
        if let Err(e) =
            sqlx::query("UPDATE profiles SET plan = 'free', updated_at = now() WHERE user_id = $1")
                .bind(user_id)
                .execute(&state.pool)
                .await
        {
            warn!(error = %e, "polar webhook: plan reset to free failed");
            return CloudError::Db(e).into_response();
        }
    }

    info!(
        event = %event.event_type,
        user_id = %user_id,
        plan,
        status,
        "polar webhook processed"
    );
    (StatusCode::OK, "ok").into_response()
}

#[allow(clippy::too_many_arguments)]
async fn upsert_subscription(
    pool: &PgPool,
    user_id: Uuid,
    sub_id: &str,
    customer_id: &str,
    product_id: &str,
    plan: &str,
    interval: &str,
    status: &str,
    renews_at: Option<&str>,
    ends_at: Option<&str>,
    event_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            user_id, ls_customer_id, ls_subscription_id, ls_product_id,
            plan, interval, status, renews_at, cancel_at, cancelled_at,
            last_event, last_event_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7,
                $8::timestamptz, $9::timestamptz,
                CASE WHEN $7 = 'grace' OR $7 = 'expired' THEN now() ELSE NULL END,
                $10, now())
        ON CONFLICT (ls_subscription_id) DO UPDATE SET
            status        = EXCLUDED.status,
            renews_at     = EXCLUDED.renews_at,
            cancel_at     = EXCLUDED.cancel_at,
            cancelled_at  = COALESCE(subscriptions.cancelled_at, EXCLUDED.cancelled_at),
            last_event    = EXCLUDED.last_event,
            last_event_at = now(),
            updated_at    = now()
        "#,
    )
    .bind(user_id)
    .bind(customer_id)
    .bind(sub_id)
    .bind(product_id)
    .bind(plan)
    .bind(interval)
    .bind(status)
    .bind(renews_at)
    .bind(ends_at)
    .bind(event_name)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(secret: &str, id: &str, ts: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{id}.{ts}.", id = id, ts = ts).as_bytes());
        mac.update(body);
        format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn verify_accepts_good_signature() {
        let secret = "polar_whs_test";
        let (id, ts, body) = ("msg_1", "1700000000", br#"{"type":"x"}"#.as_slice());
        let header = sign(secret, id, ts, body);
        assert!(verify_signature(body, secret, id, ts, &header));
    }

    #[test]
    fn verify_accepts_multi_token_header() {
        let secret = "polar_whs_test";
        let (id, ts, body) = ("msg_1", "1700000000", br#"{"type":"x"}"#.as_slice());
        let good = sign(secret, id, ts, body);
        let header = format!("v1,bm90X3ZhbGlk {good}", good = good);
        assert!(verify_signature(body, secret, id, ts, &header));
    }

    #[test]
    fn timestamp_freshness_window() {
        let now = 1_700_000_000;
        assert!(timestamp_is_fresh("1700000000", now));
        assert!(timestamp_is_fresh("1699999800", now)); // -200s, within ±300
        assert!(timestamp_is_fresh("1700000200", now)); // +200s
        assert!(!timestamp_is_fresh("1699999600", now)); // -400s, stale
        assert!(!timestamp_is_fresh("1700000400", now)); // +400s, skewed
        assert!(!timestamp_is_fresh("", now)); // missing → fail closed
        assert!(!timestamp_is_fresh("not-a-number", now));
    }

    #[test]
    fn verify_rejects_bad_signature() {
        let secret = "polar_whs_test";
        assert!(!verify_signature(
            b"{}",
            secret,
            "id",
            "1",
            "v1,bm90X3ZhbGlk"
        ));
        // Empty secret never verifies.
        assert!(!verify_signature(b"{}", "", "id", "1", "v1,x"));
    }

    #[test]
    fn checkout_events_carry_a_user_and_never_move_a_plan() {
        // Our own checkouts stamp metadata.user_id; the customer's external_id
        // is the fallback for payloads that lost it.
        let uid = "3f2b6c1e-0a4d-4f7e-9c8b-1d2e3f4a5b6c";
        let from_meta: PolarEvent = serde_json::from_str(&format!(
            r#"{{"type":"checkout.updated","data":{{"id":"co_1","status":"succeeded",
                 "metadata":{{"user_id":"{uid}"}}}}}}"#
        ))
        .unwrap();
        assert_eq!(
            user_id_of(&from_meta),
            Some(Uuid::parse_str(uid).unwrap()),
            "metadata.user_id is the primary source"
        );

        let from_customer: PolarEvent = serde_json::from_str(&format!(
            r#"{{"type":"checkout.created","data":{{"id":"co_2",
                 "customer":{{"external_id":"{uid}"}}}}}}"#
        ))
        .unwrap();
        assert_eq!(
            user_id_of(&from_customer),
            Some(Uuid::parse_str(uid).unwrap()),
            "customer.external_id is the fallback"
        );

        // A checkout opened outside our flow has nothing to attach to, and is
        // dropped rather than guessed at.
        let anonymous: PolarEvent =
            serde_json::from_str(r#"{"type":"checkout.created","data":{"id":"co_3"}}"#).unwrap();
        assert_eq!(user_id_of(&anonymous), None);

        // And none of them grants anything: only subscription.* maps to a plan
        // status, so recording a checkout can never upgrade an account.
        assert_eq!(
            status_for_event("checkout.updated", Some("succeeded")),
            None
        );
        assert_eq!(status_for_event("checkout.created", Some("open")), None);
    }

    #[test]
    fn event_status_mapping() {
        assert_eq!(
            status_for_event("subscription.created", Some("active")),
            Some("active")
        );
        assert_eq!(
            status_for_event("subscription.active", Some("active")),
            Some("active")
        );
        assert_eq!(
            status_for_event("subscription.updated", Some("past_due")),
            Some("grace")
        );
        assert_eq!(
            status_for_event("subscription.canceled", Some("active")),
            Some("grace")
        );
        assert_eq!(
            status_for_event("subscription.revoked", Some("revoked")),
            Some("expired")
        );
        assert_eq!(status_for_event("order.paid", None), None);
        assert_eq!(status_for_event("product.updated", None), None);
    }

    #[test]
    fn a_renewal_order_is_not_a_pass() {
        assert!(is_pass_order("order.paid", None));
        assert!(!is_pass_order("order.paid", Some("sub_123")));
        assert!(!is_pass_order("subscription.active", None));

        // Polar sends the field as an explicit null on an outright purchase.
        let order: PolarEvent = serde_json::from_str(
            r#"{"type":"order.paid","data":{"id":"ord_1","status":"paid",
                 "subscription_id":null,"created_at":"2026-10-21T10:00:00.123456Z"}}"#,
        )
        .unwrap();
        assert!(is_pass_order(
            &order.event_type,
            order.data.subscription_id.as_deref()
        ));
    }

    #[test]
    fn a_renewed_pass_continues_the_old_year() {
        use time::macros::datetime;
        let bought = datetime!(2026-10-22 10:00 UTC);
        let ends = datetime!(2027-10-22 10:00 UTC);

        // The first pass: a year from the payment.
        assert_eq!(chained_pass_end(bought, None), ends);
        // Renewed early, the year is added on top, nothing is lost.
        assert_eq!(
            chained_pass_end(datetime!(2027-09-01 10:00 UTC), Some(ends)),
            datetime!(2028-10-21 10:00 UTC)
        );
        // Renewed 25 days into the grace month: same end, those 25 days were
        // already Pro.
        assert_eq!(
            chained_pass_end(datetime!(2027-11-16 10:00 UTC), Some(ends)),
            datetime!(2028-10-21 10:00 UTC)
        );
        // Bought after Pro ran out: a fresh year from the payment.
        assert_eq!(
            chained_pass_end(datetime!(2027-12-01 10:00 UTC), Some(ends)),
            datetime!(2028-11-30 10:00 UTC)
        );
    }

    #[test]
    fn product_resolution() {
        // Real UUIDs copied from the production config, so don't read them as
        // a source of truth: `deploy/config.cloud.toml.example` is that, and
        // it has already moved on. The yearly one here
        // (77d81015-7c9d-4009-b2a6-e0c0a0901899) is RETIRED: it was created in
        // Polar with recurring_interval=month, so it billed €19.99 monthly
        // under a "Pro anual" label. It was replaced by
        // ee6d9395-cc3a-4b0c-96a5-4a008292ffc6 and archived. The monthly one
        // is still live. Nothing here needs updating when products change,
        // this test is about `resolve_product`'s lookup, and any two distinct
        // strings would do.
        let products = vec![
            PolarProduct {
                product_id: "4404a328-9289-4077-b25b-162ecd1d6ed3".into(),
                plan: "pro".into(),
                interval: "month".into(),
                storage_gb: Some(25),
            },
            PolarProduct {
                // Retired; see the note above.
                product_id: "77d81015-7c9d-4009-b2a6-e0c0a0901899".into(),
                plan: "pro".into(),
                interval: "year".into(),
                storage_gb: Some(25),
            },
        ];
        assert_eq!(
            resolve_product(&products, "4404a328-9289-4077-b25b-162ecd1d6ed3"),
            Some(("pro", "month"))
        );
        assert_eq!(
            resolve_product(&products, "77d81015-7c9d-4009-b2a6-e0c0a0901899"),
            Some(("pro", "year"))
        );
        assert_eq!(resolve_product(&products, "nope"), None);
    }
}
