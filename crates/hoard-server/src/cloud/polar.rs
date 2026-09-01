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
    /// Subscription id (subscription.* events). Used as the upsert conflict
    /// key, stored in `ls_subscription_id`.
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

    let status = match status_for_event(&event.event_type, event.data.status.as_deref()) {
        Some(s) => s,
        None => {
            info!(event = %event.event_type, "ignored polar event");
            return (StatusCode::OK, "ignored").into_response();
        }
    };

    // user_id: prefer checkout metadata, fall back to customer.external_id.
    let user_id = event
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
        .and_then(|u| Uuid::parse_str(u).ok());
    let user_id = match user_id {
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

    let renews_at = event
        .data
        .current_period_end
        .as_deref()
        .or(event.data.ends_at.as_deref());

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
