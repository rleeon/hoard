//! Lemon Squeezy webhooks.
//!
//! Signature verification first: every webhook ships with `X-Signature`,
//! a hex-encoded HMAC-SHA256 over the raw body, keyed by our webhook
//! secret. Anything that doesn't match → 401. Then map LS event name to a
//! subscription state transition, atomically.

use crate::cloud::errors::CloudError;
use crate::cloud::plans::Plan;
use crate::cloud::state::CloudState;
use crate::config::LemonSqueezyProduct;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgPool;
use std::str;
use tracing::{info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct LsEvent {
    pub meta: LsMeta,
    pub data: LsData,
}

#[derive(Debug, Deserialize)]
pub struct LsMeta {
    pub event_name: String,
    /// LS includes a `custom_data` we use to carry our user_id.
    pub custom_data: Option<LsCustom>,
}

#[derive(Debug, Deserialize)]
pub struct LsCustom {
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LsData {
    pub id: String,                    // subscription / order id
    pub attributes: LsAttributes,
}

#[derive(Debug, Deserialize)]
pub struct LsAttributes {
    pub status: Option<String>,        // 'active','cancelled','expired',...
    pub customer_id: Option<i64>,
    /// Each LS product has its own product_id and one variant_id. We
    /// key on product_id because the store uses two separate products
    /// (Pro Monthly and Pro Yearly) rather than one product with two
    /// variants. `variant_id` is still deserialised for logging /
    /// future debugging but the routing decision is product-based.
    pub product_id: Option<i64>,
    pub variant_id: Option<i64>,
    pub renews_at: Option<String>,
    pub ends_at: Option<String>,
    pub cancelled: Option<bool>,
}

/// Verify the HMAC-SHA256 signature ship'd by Lemon Squeezy. Returns true
/// only on match. Constant-time compare via `subtle` would be ideal — the
/// stdlib comparison via `==` on Vec<u8> is constant-time-ish on identical
/// lengths and "leaks length" is fine here (signature is always 32 bytes).
pub fn verify_signature(body: &[u8], secret: &str, signature_hex: &str) -> bool {
    if secret.is_empty() {
        return false;
    }
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let got = match hex::decode(signature_hex.trim()) {
        Ok(b) => b,
        Err(_) => return false,
    };
    if got.len() != expected.len() {
        return false;
    }
    // ct_eq via xor accumulator
    let mut diff: u8 = 0;
    for (a, b) in got.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Map LS event_name → desired status in our `subscriptions` table.
pub fn status_for_event(event: &str) -> Option<&'static str> {
    match event {
        "subscription_created" | "subscription_updated" | "subscription_resumed" => {
            Some("active")
        }
        "subscription_cancelled" => Some("grace"),
        "subscription_expired" => Some("expired"),
        "subscription_payment_failed" => Some("grace"),
        _ => None,
    }
}

/// Resolve a product_id → (plan, interval) from config.
pub fn resolve_product<'a>(
    products: &'a [LemonSqueezyProduct],
    product_id: i64,
) -> Option<(&'a str, &'a str)> {
    let p = product_id.to_string();
    products
        .iter()
        .find(|x| x.product_id == p)
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
    let sig = match headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return (StatusCode::UNAUTHORIZED, "missing signature").into_response(),
    };
    if !verify_signature(&body, &cloud.lemonsqueezy.webhook_secret, sig) {
        warn!("ls webhook: bad signature");
        return (StatusCode::UNAUTHORIZED, "bad signature").into_response();
    }
    let event: LsEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "ls webhook: malformed body");
            return (StatusCode::BAD_REQUEST, "malformed body").into_response();
        }
    };

    let user_id = match event
        .meta
        .custom_data
        .as_ref()
        .and_then(|c| c.user_id.as_ref())
        .and_then(|u| Uuid::parse_str(u).ok())
    {
        Some(u) => u,
        None => {
            warn!("ls webhook: missing/invalid custom_data.user_id");
            return (StatusCode::BAD_REQUEST, "missing user_id in custom_data")
                .into_response();
        }
    };

    let status = match status_for_event(&event.meta.event_name) {
        Some(s) => s,
        None => {
            info!(event = %event.meta.event_name, "ignored ls event");
            return (StatusCode::OK, "ignored").into_response();
        }
    };

    let product_id = event.data.attributes.product_id.unwrap_or(-1);
    let (plan, interval) = match resolve_product(&cloud.lemonsqueezy.products, product_id) {
        Some(x) => x,
        None => {
            warn!(product_id, "ls webhook: unknown product_id");
            return (StatusCode::BAD_REQUEST, "unknown product_id").into_response();
        }
    };

    if let Err(e) = upsert_subscription(
        &state.pool,
        user_id,
        &event.data.id,
        event.data.attributes.customer_id.unwrap_or(0),
        product_id,
        plan,
        interval,
        status,
        event.data.attributes.renews_at.as_deref(),
        event.data.attributes.ends_at.as_deref(),
        &event.meta.event_name,
    )
    .await
    {
        warn!(error = %e, "ls webhook: db upsert failed");
        return CloudError::Db(e).into_response();
    }

    // Cascade to profiles.plan when active. When grace/expired we leave
    // profiles.plan untouched until the daily cron pushes the user to free
    // — gives them the period they paid for.
    if status == "active" {
        if let Err(e) = sqlx::query(
            "UPDATE profiles SET plan = $1, updated_at = now() WHERE user_id = $2",
        )
        .bind(plan)
        .bind(user_id)
        .execute(&state.pool)
        .await
        {
            warn!(error = %e, "ls webhook: profile plan update failed");
        }
    }

    info!(
        event = %event.meta.event_name,
        user_id = %user_id,
        plan,
        status,
        "ls webhook processed"
    );
    (StatusCode::OK, "ok").into_response()
}

#[allow(clippy::too_many_arguments)]
async fn upsert_subscription(
    pool: &PgPool,
    user_id: Uuid,
    ls_sub_id: &str,
    customer_id: i64,
    product_id: i64,
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
    .bind(customer_id.to_string())
    .bind(ls_sub_id)
    .bind(product_id.to_string())
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

// Silence the unused warning on `Plan` when only constants of it are used
// at compile time elsewhere; keeps the module shape consistent.
const _: fn() = || {
    let _ = Plan::Free;
};

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_sign(secret: &[u8], body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn verify_signature_accepts_good_hmac() {
        let secret = "shhh";
        let body = b"hello world";
        let sig = hex_sign(secret.as_bytes(), body);
        assert!(verify_signature(body, secret, &sig));
    }

    #[test]
    fn verify_signature_rejects_bad_hmac() {
        let body = b"hello world";
        assert!(!verify_signature(body, "shhh", "deadbeef"));
        // Wrong length
        assert!(!verify_signature(body, "shhh", ""));
    }

    #[test]
    fn verify_signature_rejects_empty_secret() {
        let sig = hex_sign(b"x", b"hello");
        assert!(!verify_signature(b"hello", "", &sig));
    }

    #[test]
    fn event_mapping_known() {
        assert_eq!(status_for_event("subscription_created"), Some("active"));
        assert_eq!(status_for_event("subscription_updated"), Some("active"));
        assert_eq!(status_for_event("subscription_cancelled"), Some("grace"));
        assert_eq!(status_for_event("subscription_expired"), Some("expired"));
        assert_eq!(
            status_for_event("subscription_payment_failed"),
            Some("grace")
        );
        assert_eq!(status_for_event("order_created"), None);
    }

    #[test]
    fn product_resolution() {
        // Post-1.6.1 only Pro exists; the store uses two products (Pro
        // Monthly + Pro Yearly), each with its own product_id. We keep
        // both in the test so a single-product test wouldn't accidentally
        // miss the `interval` column wiring.
        let products = vec![
            LemonSqueezyProduct {
                product_id: "1087714".into(),
                plan: "pro".into(),
                interval: "month".into(),
            },
            LemonSqueezyProduct {
                product_id: "1087715".into(),
                plan: "pro".into(),
                interval: "year".into(),
            },
        ];
        assert_eq!(resolve_product(&products, 1087714), Some(("pro", "month")));
        assert_eq!(resolve_product(&products, 1087715), Some(("pro", "year")));
        assert_eq!(resolve_product(&products, 333), None);
    }
}
