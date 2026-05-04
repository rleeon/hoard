//! Authentication & onboarding commands.
//!
//! These handle the wizard's three round-trips to the server:
//!
//! 1. `health_check` — anonymous probe to confirm the URL points at a Hoard
//!    server before we ask for a token.
//! 2. `login` — exchanges (URL, token) for a verified `whoami` response and
//!    persists the credentials.
//! 3. `logout` — clears credentials.
//!
//! `is_logged_in` and `current_user` read from the in-memory cache that the
//! app populates at startup from the on-disk session.

use hoard_agent::api::{ApiClient, ApiError};
use hoard_agent::credentials::{self, Credentials, UserSection};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// Anonymous health probe response. Mirrors `hoard_agent::api::Health` but
/// kept as its own type so the frontend bindings don't reach into the agent
/// crate's internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthInfo {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

/// Verified user identity returned by `login` and `current_user`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
    pub server_url: String,
}

/// Probe `/v1/health` without auth. Frontend uses this in the wizard to give
/// the user fast feedback that the URL points at a working Hoard server
/// before they paste a token.
#[tauri::command]
pub async fn health_check(url: String) -> Result<HealthInfo, String> {
    let url = url.trim().to_string();
    validate_url(&url)?;

    // The token field is unused by `/v1/health`, but `ApiClient::new` requires
    // the field to exist. Pass an empty string — it never goes on the wire.
    let client = ApiClient::new(url, String::new()).map_err(pretty_error)?;
    let h = client.health().await.map_err(pretty_error)?;
    Ok(HealthInfo {
        status: h.status,
        version: h.version,
        uptime_secs: h.uptime_secs,
    })
}

/// Validate `(url, token)` against the server, persist credentials, and warm
/// the in-memory user cache.
#[tauri::command]
pub async fn login(
    url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<UserInfo, String> {
    let url = url.trim().to_string();
    validate_url(&url)?;

    if !credentials::is_valid_token(&token) {
        return Err(
            "That doesn't look like a Hoard access key. It should start with `hoard_v1_` \
             followed by 64 lowercase hex characters."
                .into(),
        );
    }

    let client = ApiClient::new(url.clone(), token.clone()).map_err(pretty_error)?;
    let who = client.whoami().await.map_err(pretty_error)?;

    let user = UserInfo {
        user_id: who.user_id.clone(),
        username: who.username.clone(),
        is_admin: who.is_admin,
        server_url: url.clone(),
    };

    credentials::save(&Credentials {
        url,
        token,
        user: Some(UserSection::from(who)),
    })
    .map_err(|e| format!("Couldn't save credentials: {e}"))?;

    *state.user.lock().unwrap() = Some(user.clone());
    Ok(user)
}

/// Cheap, synchronous check used by the router to decide whether to show
/// the onboarding wizard or the dashboard. Does not touch the network.
#[tauri::command]
pub fn is_logged_in(state: State<'_, AppState>) -> bool {
    state.user.lock().unwrap().is_some()
}

/// Cached user info populated at startup or by the most recent `login`.
#[tauri::command]
pub fn current_user(state: State<'_, AppState>) -> Option<UserInfo> {
    state.user.lock().unwrap().clone()
}

/// Clear stored credentials and the in-memory cache.
#[tauri::command]
pub fn logout(state: State<'_, AppState>) -> Result<(), String> {
    credentials::clear().map_err(|e| format!("Couldn't clear credentials: {e}"))?;
    *state.user.lock().unwrap() = None;
    Ok(())
}

// ---- helpers ----------------------------------------------------------

fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("Please enter the address of your Hoard server.".into());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("The server address must start with http:// or https://".into());
    }
    Ok(())
}

/// Translate an `anyhow::Error` from the agent into a sentence a non-developer
/// can act on. Falls through to the raw message for shapes we don't recognise
/// — the frontend logs those to the console for bug reports.
pub(crate) fn pretty_error(err: anyhow::Error) -> String {
    if let Some(api) = err.downcast_ref::<ApiError>() {
        return match api {
            ApiError::Unauthorized => {
                "The server didn't accept that access key. Double-check it and try again.".into()
            }
            ApiError::Forbidden => {
                "Your access key is valid but the server isn't letting it do that.".into()
            }
            ApiError::NotFound => {
                "The server is reachable but it doesn't look like a Hoard server. \
                 Did you copy the URL correctly?"
                    .into()
            }
            ApiError::Server { status, .. } if *status >= 500 => {
                format!("The server returned an error ({status}). Try again in a moment.")
            }
            ApiError::Server { status, body } => {
                format!("Server replied with {status}: {body}")
            }
            ApiError::Network(e) => network_message(e),
            ApiError::TooLarge => "The server says that's too big to upload.".into(),
            ApiError::Conflict(msg) | ApiError::BadRequest(msg) => msg.clone(),
        };
    }
    if let Some(req) = err.downcast_ref::<reqwest::Error>() {
        return network_message(req);
    }
    err.to_string()
}

fn network_message(err: &reqwest::Error) -> String {
    if err.is_connect() {
        return "Can't reach the server. Is the address correct and the server running?".into();
    }
    if err.is_timeout() {
        return "The server took too long to respond. Try again in a moment.".into();
    }
    if err.is_decode() {
        return "The server replied with something Hoard couldn't understand. \
                Are you sure it's a Hoard server?"
            .into();
    }
    if err.is_request() {
        return format!("Couldn't send the request: {err}");
    }
    err.to_string()
}
