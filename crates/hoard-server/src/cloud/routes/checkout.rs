//! `POST /v1/cloud/checkout` — create a Polar checkout session.
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

    let product_id = polar.product_for(&body.plan, &body.interval).ok_or_else(|| {
        warn!(plan = %body.plan, interval = %body.interval, "checkout: no product for plan/interval");
        CloudError::BadRequest("unknown plan/interval".into())
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

    Ok(Json(CheckoutOut { url: parsed.url }))
}
