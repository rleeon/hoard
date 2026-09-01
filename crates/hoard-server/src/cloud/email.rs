//! Transactional email via Resend's HTTP API.
//!
//! Only one message today: "your account export is ready". Delivery is
//! best-effort and *optional*: with no `cloud.email.api_key` configured the
//! export worker still produces a downloadable ZIP (surfaced in-app via
//! `GET /v1/me/export`); it just skips the mail. That keeps a fresh deploy
//! working before an email provider is even provisioned.

use crate::config::EmailConfig;
use anyhow::{Context, Result};
use time::OffsetDateTime;

const RESEND_ENDPOINT: &str = "https://api.resend.com/emails";

/// True when the config has enough to actually send (key + from address).
pub fn is_configured(cfg: &EmailConfig) -> bool {
    !cfg.api_key.is_empty() && !cfg.from.is_empty()
}

/// Send the "export ready" email. Returns `Ok(false)` (a no-op, not an error)
/// when email isn't configured, so callers can fire-and-log without special
/// casing the disabled path.
pub async fn send_export_ready(
    cfg: &EmailConfig,
    to: &str,
    download_url: &str,
    expires: OffsetDateTime,
) -> Result<bool> {
    if !is_configured(cfg) {
        return Ok(false);
    }

    let expires_str = expires
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let subject = "Your Hoard Cloud export is ready";
    let html = format!(
        "<p>Your Hoard Cloud data export is ready to download.</p>\
         <p><a href=\"{url}\">Download your saves (ZIP)</a></p>\
         <p>This link expires on {expires}. If it lapses, request a fresh \
         export from the account page in the Hoard app.</p>",
        url = html_escape(download_url),
        expires = html_escape(&expires_str),
    );
    let text = format!(
        "Your Hoard Cloud data export is ready to download:\n\n{download_url}\n\n\
         This link expires on {expires_str}. If it lapses, request a fresh export \
         from the account page in the Hoard app."
    );

    let resp = reqwest::Client::new()
        .post(RESEND_ENDPOINT)
        .bearer_auth(&cfg.api_key)
        .json(&serde_json::json!({
            "from": cfg.from,
            "to": [to],
            "subject": subject,
            "html": html,
            "text": text,
        }))
        .send()
        .await
        .context("resend send")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("resend returned {status}: {body}");
    }
    Ok(true)
}

/// Minimal HTML-attribute/text escaping for the two values we interpolate. The
/// URL is our own presigned R2 link and the timestamp is RFC3339, so this is
/// defense-in-depth rather than untrusted-input handling.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
