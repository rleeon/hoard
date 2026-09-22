//! `POST /v1/cloud/checkout`: create a Polar checkout session.
//!
//! Server-initiated so the user_id can never be forged: we read it (and the
//! email) from the verified Supabase JWT, then stamp `metadata.user_id` and
//! `external_customer_id` onto the Polar checkout. The webhook
//! (`cloud::polar::handle`) reads that same `metadata.user_id` back to attach
//! the resulting subscription to the right account, so the whole loop is
//! tamper-proof regardless of what the browser sends.
//!
//! The client only chooses *what* to buy by (plan, interval); the concrete
//! Polar product UUID is resolved from server config so product IDs never
//! travel through the browser.

use crate::cloud::auth::CloudUser;
use crate::cloud::errors::CloudError;
use crate::cloud::state::CloudState;
use axum::{extract::State, response::Json, Extension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::warn;

/// Request body: which tier + billing cycle the user picked.
#[derive(Debug, Deserialize)]
pub struct CheckoutIn {
    /// "pro" (only paid tier today). Free needs no checkout.
    pub plan: String,
    /// "month" | "year".
    pub interval: String,
    /// Storage tier in GB (Pro x1 = 25, x2 = 50, …). Omitted/`None` buys the
    /// base product. Resolved server-side to the concrete Polar product so
    /// product IDs never travel through the browser.
    #[serde(default)]
    pub storage_gb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutOut {
    /// Hosted Polar checkout URL to redirect the browser to.
    pub url: String,
}

/// Shape of the bits of Polar's checkout response we need.
#[derive(Debug, Deserialize)]
struct PolarCheckoutResp {
    url: String,
    /// Polar's checkout id. We don't redirect with it, we record it: it is the
    /// only key that joins "asked to pay" here with "paid" or "expired" in the
    /// webhook (`polar::record_checkout_event`). Without it an abandoned
    /// checkout is indistinguishable from one that never started.
    #[serde(default)]
    id: Option<String>,
}

pub async fn create_checkout(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<CheckoutIn>,
) -> Result<Json<CheckoutOut>, CloudError> {
    let cloud = state
        .config
        .cloud
        .as_ref()
        .ok_or_else(|| CloudError::Internal(anyhow::anyhow!("cloud config missing")))?;
    let polar = &cloud.polar;

    if polar.access_token.is_empty() {
        warn!("checkout: polar access_token not configured");
        return Err(CloudError::Internal(anyhow::anyhow!(
            "payments not configured"
        )));
    }

    // A specific tier resolves to its exact product; no tier falls back to the
    // base product for that (plan, interval).
    let product_id = match body.storage_gb {
        Some(gb) => polar.product_for_storage(&body.plan, &body.interval, Some(gb)),
        None => polar.product_for(&body.plan, &body.interval),
    }
    .ok_or_else(|| {
        warn!(plan = %body.plan, interval = %body.interval, storage_gb = ?body.storage_gb, "checkout: no product for plan/interval/tier");
        CloudError::BadRequest("unknown plan/interval/tier".into())
    })?;

    // Polar metadata values must be strings/numbers/bools; user_id as string.
    let mut metadata: HashMap<&str, String> = HashMap::new();
    metadata.insert("user_id", user.user_id.to_string());

    let payload = json!({
        "products": [product_id],
        "customer_email": user.email,
        "external_customer_id": user.user_id.to_string(),
        "success_url": polar.success(),
        "metadata": metadata,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| CloudError::Internal(anyhow::anyhow!("http client: {e}")))?;

    let url = format!("{}/v1/checkouts/", polar.base());
    let resp = client
        .post(&url)
        .bearer_auth(&polar.access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            warn!(error = %e, "checkout: polar request failed");
            CloudError::Internal(anyhow::anyhow!("payment provider unreachable"))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        warn!(%status, detail, "checkout: polar returned error");
        return Err(CloudError::Internal(anyhow::anyhow!(
            "checkout create failed ({status})"
        )));
    }

    let parsed: PolarCheckoutResp = resp.json().await.map_err(|e| {
        warn!(error = %e, "checkout: malformed polar response");
        CloudError::Internal(anyhow::anyhow!("malformed checkout response"))
    })?;

    // Record the intent. Both the app's Pro button and the web's /checkout page
    // come through this route, so one row here counts everybody, not just the
    // clients new enough to skip the second sign-in.
    //
    // Best-effort on purpose: the user is in the middle of paying, and a failed
    // INSERT must never cost them the redirect they asked for.
    let meta = json!({
        "plan": body.plan,
        "interval": body.interval,
        "storage_gb": body.storage_gb,
        "checkout_id": parsed.id,
    })
    .to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO audit_log (user_id, actor, event_type, metadata)
             VALUES ($1, 'user', 'checkout.requested', $2::jsonb)",
    )
    .bind(user.user_id)
    .bind(meta)
    .execute(&state.pool)
    .await
    {
        warn!(error = %e, user_id = %user.user_id, "checkout: couldn't record the intent");
    }

    Ok(Json(CheckoutOut { url: parsed.url }))
}
