//! Device-pairing endpoints: browserless CLI login.
//!
//! Flow: the CLI (headless box, no browser) calls [`start`] and prints the
//! returned `user_code` + URL. The user opens the URL on a phone already
//! signed into Hoard Cloud and confirms the code, which hits [`approve`]; that
//! mints a fresh session (see [`supabase_admin`]) and parks it on the pairing
//! row. Meanwhile the CLI polls [`poll`] and, once approved, collects the
//! tokens (single-use: the row is deleted on read).
//!
//! `start`/`poll` are unauthenticated (the CLI has no session yet) and keyed by
//! the secret `device_code`. `approve` requires the phone's Cloud JWT.

use anyhow::anyhow;
use axum::{extract::State, response::Json, Extension};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};

use crate::cloud::{auth::CloudUser, errors::CloudError, state::CloudState, supabase_admin};

/// Pairing lifetime. Long enough to grab your phone, short enough that a leaked
/// `user_code` is useless minutes later.
const PAIRING_TTL_SECS: i32 = 600;
/// How often the CLI should poll. Echoed back so the client doesn't hardcode it.
const POLL_INTERVAL_SECS: i64 = 3;
/// Where the user approves. The web app serves `/link`.
const VERIFICATION_URI: &str = "https://hoard.services/link";

// ---- start -------------------------------------------------------------

#[derive(Deserialize)]
pub struct StartReq {
    /// Optional machine name, shown on the phone so the user knows what they're
    /// authorizing ("Aprobar acceso a steam-deck?").
    #[serde(default)]
    pub hostname: Option<String>,
}

#[derive(Serialize)]
pub struct StartResp {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    interval_secs: i64,
    expires_in_secs: i32,
}

pub async fn start(
    State(state): State<CloudState>,
    body: Option<Json<StartReq>>,
) -> Result<Json<StartResp>, CloudError> {
    require_pairing_cfg(&state)?;

    let hostname = body
        .and_then(|b| b.0.hostname)
        .map(|h| truncate(h.trim(), 64))
        .filter(|h| !h.is_empty());
    let device_code = random_hex();

    // Retry on the (rare) user_code collision: the UNIQUE index rejects it.
    for _ in 0..6 {
        let user_code = gen_user_code();
        let res = sqlx::query(
            "INSERT INTO device_pairings (device_code, user_code, hostname, expires_at)
             VALUES ($1, $2, $3, now() + $4 * interval '1 second')",
        )
        .bind(&device_code)
        .bind(&user_code)
        .bind(&hostname)
        .bind(PAIRING_TTL_SECS)
        .execute(&state.pool)
        .await;
        match res {
            Ok(_) => {
                return Ok(Json(StartResp {
                    verification_uri: VERIFICATION_URI.to_string(),
                    verification_uri_complete: format!("{VERIFICATION_URI}?code={user_code}"),
                    device_code,
                    user_code,
                    interval_secs: POLL_INTERVAL_SECS,
                    expires_in_secs: PAIRING_TTL_SECS,
                }));
            }
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(CloudError::Internal(anyhow!(
        "could not allocate a unique user_code"
    )))
}

// ---- poll --------------------------------------------------------------

#[derive(Deserialize)]
pub struct PollReq {
    pub device_code: String,
}

#[derive(Serialize)]
pub struct PollResp {
    /// pending | approved | denied | expired.
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

impl PollResp {
    fn bare(status: &'static str) -> Json<Self> {
        Json(Self {
            status,
            access_token: None,
            refresh_token: None,
        })
    }
}

pub async fn poll(
    State(state): State<CloudState>,
    Json(body): Json<PollReq>,
) -> Result<Json<PollResp>, CloudError> {
    let row: Option<(String, Option<String>, Option<String>, bool)> = sqlx::query_as(
        "SELECT status, access_token, refresh_token, (expires_at < now()) AS expired
         FROM device_pairings WHERE device_code = $1",
    )
    .bind(&body.device_code)
    .fetch_optional(&state.pool)
    .await?;

    // Unknown code: indistinguishable from expired/consumed on purpose.
    let Some((status, access, refresh, expired)) = row else {
        return Ok(PollResp::bare("expired"));
    };

    if expired {
        delete_pairing(&state, &body.device_code).await;
        return Ok(PollResp::bare("expired"));
    }

    match status.as_str() {
        "approved" => {
            // Single-use: hand over the tokens exactly once, then wipe the row.
            delete_pairing(&state, &body.device_code).await;
            match (access, refresh) {
                (Some(a), Some(r)) => Ok(Json(PollResp {
                    status: "approved",
                    access_token: Some(a),
                    refresh_token: Some(r),
                })),
                // "approved" without tokens shouldn't happen; treat as gone.
                _ => Ok(PollResp::bare("expired")),
            }
        }
        "denied" => {
            delete_pairing(&state, &body.device_code).await;
            Ok(PollResp::bare("denied"))
        }
        _ => Ok(PollResp::bare("pending")),
    }
}

// ---- approve (phone, authed) ------------------------------------------

#[derive(Deserialize)]
pub struct ApproveReq {
    pub user_code: String,
}

#[derive(Serialize)]
pub struct ApproveResp {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
}

pub async fn approve(
    State(state): State<CloudState>,
    Extension(user): Extension<CloudUser>,
    Json(body): Json<ApproveReq>,
) -> Result<Json<ApproveResp>, CloudError> {
    let (supabase_url, service_key) = require_pairing_cfg(&state)?;
    if user.email.is_empty() {
        return Err(CloudError::BadRequest(
            "this account has no email address, so it can't be paired".into(),
        ));
    }

    let user_code = normalize_user_code(&body.user_code);
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT device_code, hostname FROM device_pairings
         WHERE user_code = $1 AND status = 'pending' AND expires_at > now()",
    )
    .bind(&user_code)
    .fetch_optional(&state.pool)
    .await?;
    let Some((device_code, hostname)) = row else {
        return Err(CloudError::NotFound(
            "no pending pairing for that code (expired or wrong code)",
        ));
    };

    // Mint a *new* session for this user (independent refresh family), so the
    // CLI never shares the phone's tokens.
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| CloudError::Internal(e.into()))?;
    let minted = supabase_admin::mint_session(&http, supabase_url, service_key, &user.email)
        .await
        .map_err(CloudError::Internal)?;

    sqlx::query(
        "UPDATE device_pairings
         SET status = 'approved', user_id = $1, access_token = $2, refresh_token = $3
         WHERE device_code = $4 AND status = 'pending'",
    )
    .bind(user.user_id)
    .bind(&minted.access_token)
    .bind(&minted.refresh_token)
    .bind(&device_code)
    .execute(&state.pool)
    .await?;

    Ok(Json(ApproveResp { ok: true, hostname }))
}

// ---- helpers -----------------------------------------------------------

/// Both minting endpoints need Supabase admin creds. Absent = feature off.
fn require_pairing_cfg(state: &CloudState) -> Result<(&str, &str), CloudError> {
    let cc = state
        .config
        .cloud
        .as_ref()
        .ok_or_else(|| CloudError::Internal(anyhow!("cloud config missing")))?;
    if cc.supabase_url.is_empty() || cc.supabase_service_role_key.is_empty() {
        return Err(CloudError::NotFound("device pairing is not configured"));
    }
    Ok((&cc.supabase_url, &cc.supabase_service_role_key))
}

async fn delete_pairing(state: &CloudState, device_code: &str) {
    let _ = sqlx::query("DELETE FROM device_pairings WHERE device_code = $1")
        .bind(device_code)
        .execute(&state.pool)
        .await;
}

/// 32 random bytes, hex. The CLI's secret handle on the pairing.
fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Short human code, `XXXX-XXXX`, from an unambiguous alphabet (no O/0/I/1).
fn gen_user_code() -> String {
    const ALPHA: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let s: String = (0..8)
        .map(|_| ALPHA[rng.gen_range(0..ALPHA.len())] as char)
        .collect();
    format!("{}-{}", &s[0..4], &s[4..8])
}

/// Accept what the user types ("wdxk 7q2p", "WDXK-7Q2P") and canonicalize to
/// the stored `XXXX-XXXX` form.
fn normalize_user_code(raw: &str) -> String {
    let compact: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if compact.len() == 8 {
        format!("{}-{}", &compact[0..4], &compact[4..8])
    } else {
        compact
    }
}

fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
