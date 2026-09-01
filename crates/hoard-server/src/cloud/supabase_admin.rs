//! Minting a fresh Supabase session server-side, with the service-role key.
//!
//! Used only by the device-pairing flow ([`crate::cloud::routes::device`]).
//! When a signed-in phone approves a pairing we must hand the waiting CLI a
//! *session of its own*, not the phone's tokens. Supabase has no public
//! "create a second session for this user" call, but the admin API does the
//! job in two hops:
//!
//! 1. `POST /auth/v1/admin/generate_link` (`type=magiclink`), with the
//!    service-role key. This does **not** send an email; it just returns a
//!    one-time `email_otp` for that user.
//! 2. `POST /auth/v1/verify` (`type=magiclink`), redeeming that OTP for a brand
//!    new access+refresh pair. Independent refresh-token family, so Supabase's
//!    reuse detection never crosses it with the phone's session.
//!
//! The service-role key is a god key: it lives only in the server's env
//! (`HOARD__CLOUD__SUPABASE_SERVICE_ROLE_KEY`) and never leaves this module.

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// A freshly minted session for the CLI.
pub struct MintedSession {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
struct GenerateLinkResp {
    /// One-time code we immediately redeem below. Present for magiclink links.
    email_otp: Option<String>,
}

#[derive(Deserialize)]
struct VerifyResp {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

/// Mint a new session for `email`. `base_url` is the Supabase project URL
/// (`https://<ref>.supabase.co`); `service_role_key` the privileged key.
pub async fn mint_session(
    http: &reqwest::Client,
    base_url: &str,
    service_role_key: &str,
    email: &str,
) -> Result<MintedSession> {
    let base = base_url.trim_end_matches('/');

    // 1. Admin generate_link → email_otp (no email is sent).
    let gen: GenerateLinkResp = http
        .post(format!("{base}/auth/v1/admin/generate_link"))
        .header("apikey", service_role_key)
        .bearer_auth(service_role_key)
        .json(&serde_json::json!({ "type": "magiclink", "email": email }))
        .send()
        .await
        .context("supabase generate_link request failed")?
        .error_for_status()
        .context("supabase generate_link returned non-2xx")?
        .json()
        .await
        .context("parsing generate_link response")?;

    let otp = gen
        .email_otp
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("generate_link response had no email_otp"))?;

    // 2. Verify the OTP → a fresh, independent session.
    let verified: VerifyResp = http
        .post(format!("{base}/auth/v1/verify"))
        .header("apikey", service_role_key)
        .json(&serde_json::json!({
            "type": "magiclink",
            "email": email,
            "token": otp,
        }))
        .send()
        .await
        .context("supabase verify request failed")?
        .error_for_status()
        .context("supabase verify returned non-2xx")?
        .json()
        .await
        .context("parsing verify response")?;

    match (verified.access_token, verified.refresh_token) {
        (Some(access_token), Some(refresh_token))
            if !access_token.is_empty() && !refresh_token.is_empty() =>
        {
            Ok(MintedSession {
                access_token,
                refresh_token,
            })
        }
        _ => bail!("verify response missing tokens"),
    }
}
