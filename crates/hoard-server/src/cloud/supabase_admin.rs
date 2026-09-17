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

/// One Supabase Auth account, with only what `auth_mirror` copies. Timestamps
/// stay text: Postgres parses them on the way in.
#[derive(Deserialize, Debug, Clone)]
pub struct AdminUser {
    pub id: uuid::Uuid,
    pub email: Option<String>,
    pub email_confirmed_at: Option<String>,
    pub banned_until: Option<String>,
    pub created_at: Option<String>,
    pub last_sign_in_at: Option<String>,
    #[serde(default)]
    pub app_metadata: serde_json::Value,
}

#[derive(Deserialize)]
struct UsersPage {
    users: Vec<AdminUser>,
}

/// Every account, a page of 1.000 at a time. 225 accounts in Sep 2026, so one
/// request. The page cap only stops a server that keeps answering full pages.
pub async fn list_users(
    http: &reqwest::Client,
    base_url: &str,
    service_role_key: &str,
) -> Result<Vec<AdminUser>> {
    const PER_PAGE: usize = 1000;
    let base = base_url.trim_end_matches('/');
    let mut all = Vec::new();
    for page in 1..=100 {
        let got: UsersPage = http
            .get(format!("{base}/auth/v1/admin/users"))
            .query(&[("page", page), ("per_page", PER_PAGE)])
            .header("apikey", service_role_key)
            .bearer_auth(service_role_key)
            .send()
            .await
            .context("supabase admin users request failed")?
            .error_for_status()
            .context("supabase admin users returned non-2xx")?
            .json()
            .await
            .context("parsing admin users response")?;
        let n = got.users.len();
        all.extend(got.users);
        if n < PER_PAGE {
            return Ok(all);
        }
    }
    bail!("admin users kept answering full pages past 100.000 accounts")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from a real GoTrue answer. `banned_until` and `last_sign_in_at`
    // are simply absent on accounts that never had them.
    #[test]
    fn an_admin_users_page_parses() {
        let raw = r#"{"aud":"authenticated","users":[
            {"id":"6a1c7c64-1d3c-4b8e-9d0b-2f7d5d1c0a11","aud":"authenticated","role":"authenticated",
             "email":"someone@example.test","email_confirmed_at":"2026-08-02T10:11:12.123456Z",
             "last_sign_in_at":"2026-09-15T08:00:00Z","app_metadata":{"provider":"google","providers":["google"]},
             "user_metadata":{"avatar_url":"x"},"identities":[],"created_at":"2026-08-02T10:11:00Z"},
            {"id":"0b8a7f55-2a44-4f55-8d19-3c1a2b4c5d6e","email":"otp@example.test",
             "app_metadata":{"provider":"email"},"created_at":"2026-09-01T00:00:00Z"}
        ]}"#;
        let page: UsersPage = serde_json::from_str(raw).unwrap();
        assert_eq!(page.users.len(), 2);
        assert_eq!(page.users[0].app_metadata["provider"], "google");
        assert!(page.users[1].last_sign_in_at.is_none());
        assert!(page.users[1].banned_until.is_none());
    }
}
