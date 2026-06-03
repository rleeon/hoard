//! Hoard Cloud (SaaS) commands — talk to `api.hoard.services`.
//!
//! These commands are deliberately separate from `auth.rs` (self-hosted
//! bearer-token flow) because the cloud session uses a Supabase JWT and a
//! refresh token, persisted to a different file. A user can have either
//! session, never both — the UI's router picks /dashboard vs /account.
//!
//! All HTTP calls go to `cloud_base_url()` (defaults to
//! `https://api.hoard.services` but overridable via the `HOARD_CLOUD_URL`
//! env var so the dev build can hit a localhost server).

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, State};

use crate::commands::cloud_pull;
use crate::state::AppState;

const CLOUD_DEFAULT_URL: &str = "https://api.hoard.services";
const KEYRING_SERVICE: &str = "hoard-desktop-cloud";
const KEYRING_USER: &str = "default";

// Supabase GoTrue project — the same public project the web app talks to
// (`web/.env` → `PUBLIC_SUPABASE_*`, baked into the static bundle). The anon
// key is a public, browser-exposed credential, so embedding it here is no more
// sensitive than shipping the web client. Both are overridable at runtime
// (env var) or build time (`option_env!`) so a dev build can point at a
// different project without touching code.
const SUPABASE_DEFAULT_URL: &str = "https://zddepgqdiuhhzqdimsks.supabase.co";
const SUPABASE_DEFAULT_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InpkZGVwZ3FkaXVoaHpxZGltc2tzIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Nzk2MzM2MTksImV4cCI6MjA5NTIwOTYxOX0.3nZebGwCzFO1byTqhowq9ip89GE9fMRxPscgYSlPzFk";

fn supabase_url() -> String {
    std::env::var("HOARD_SUPABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("HOARD_SUPABASE_URL").map(str::to_string))
        .unwrap_or_else(|| SUPABASE_DEFAULT_URL.to_string())
}

fn supabase_anon_key() -> String {
    std::env::var("HOARD_SUPABASE_ANON_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("HOARD_SUPABASE_ANON_KEY").map(str::to_string))
        .unwrap_or_else(|| SUPABASE_DEFAULT_ANON_KEY.to_string())
}

// ---- session model ----------------------------------------------------

/// Cached cloud session — Supabase JWT plus the most recent `/v1/me` snapshot.
/// Persisted to `<config>/desktop/cloud.toml`; the tokens go to the keyring
/// when available with a 0600 file fallback otherwise.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CloudSessionFile {
    #[serde(default)]
    server_url: String,
    #[serde(default)]
    user: Option<CloudAccount>,
    /// Fallback when the OS keyring is unavailable.
    #[serde(default)]
    auth: Option<AuthSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthSection {
    access_token: String,
    refresh_token: String,
}

/// Public account shape — what `/v1/me` returns and what the UI binds to.
///
/// Wire shape changed in 1.6.1: dropped `retention_days` (history is
/// forever on every tier now) and added the per-save + bandwidth-window
/// fields so the Account page can surface them in the usage card.
/// `serde` is forgiving on missing fields — an old server that still
/// emits `retention_days` simply won't populate the new ones, which the
/// UI handles by hiding the row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAccount {
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub plan: String,
    pub storage_used_bytes: i64,
    pub storage_limit_bytes: i64,
    pub devices_used: i32,
    pub devices_limit: i32,
    pub saves_used: i32,
    pub saves_limit: i32,
    #[serde(default = "default_forever")]
    pub version_history_forever: bool,
    #[serde(default)]
    pub max_save_size_bytes: i64,
    #[serde(default)]
    pub bandwidth_window_secs: i32,
    #[serde(default)]
    pub bandwidth_quota_bytes: i64,
    pub subscription_status: Option<String>,
    pub renews_at: Option<String>,
    pub cancel_at: Option<String>,
}

fn default_forever() -> bool {
    true
}

/// Carry the access_token through method calls without persisting it on every
/// hop. Loaded from the keyring at the top of each command.
#[derive(Debug, Clone)]
pub struct CloudCreds {
    pub access_token: String,
    /// Supabase refresh token. Used by [`refresh_active_session`] to mint a
    /// fresh access token when the short-lived JWT expires.
    pub refresh_token: String,
    pub server_url: String,
    /// Cached plan tier from the last `/v1/me`. Used by `cloud_pull` to
    /// label `quota-reached` events. `None` when the session file
    /// pre-dates this snapshot — callers default to "free".
    pub plan: Option<String>,
}

// ---- helpers ----------------------------------------------------------

fn cloud_base_url() -> String {
    std::env::var("HOARD_CLOUD_URL").unwrap_or_else(|_| CLOUD_DEFAULT_URL.to_string())
}

fn session_path() -> Result<PathBuf> {
    let dirs = hoard_agent::config::CliConfig::project_dirs()?;
    Ok(dirs.config_dir().join("desktop").join("cloud.toml"))
}

fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("hoard-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("building HTTP client")
}

fn read_session() -> Result<Option<CloudSessionFile>> {
    let path = session_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let s: CloudSessionFile =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(s))
}

fn write_session(s: &CloudSessionFile) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(s).context("serializing cloud session")?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

fn keyring_set(access: &str, refresh: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    // Pack both tokens as TOML so we don't need two keyring entries.
    let blob = toml::to_string(&AuthSection {
        access_token: access.to_string(),
        refresh_token: refresh.to_string(),
    })?;
    entry.set_password(&blob)?;
    Ok(())
}

fn keyring_get() -> Result<Option<AuthSection>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.get_password() {
        Ok(blob) => {
            let parsed: AuthSection = toml::from_str(&blob)?;
            Ok(Some(parsed))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn keyring_delete() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn load_creds() -> Result<Option<CloudCreds>> {
    let Some(session) = read_session()? else {
        return Ok(None);
    };
    let auth = match keyring_get() {
        Ok(Some(a)) => Some(a),
        _ => session.auth.clone(),
    };
    let Some(auth) = auth else {
        return Ok(None);
    };
    if auth.access_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(CloudCreds {
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        server_url: if session.server_url.is_empty() {
            cloud_base_url()
        } else {
            session.server_url
        },
        plan: session.user.as_ref().map(|u| u.plan.clone()),
    }))
}

/// Public wrapper used by the cloud-pull poller. Returns the active cloud
/// session credentials (access token + server URL + cached plan label) or
/// `None` when the user is signed out.
pub fn load_active_creds() -> Result<Option<CloudCreds>> {
    load_creds()
}

fn save_creds(access: &str, refresh: &str, server_url: &str, user: &CloudAccount) -> Result<()> {
    // Try the keyring first; fall back to the file if it isn't available.
    let mut file = CloudSessionFile {
        server_url: server_url.to_string(),
        user: Some(user.clone()),
        auth: None,
    };
    match keyring_set(access, refresh) {
        Ok(()) => write_session(&file),
        Err(_) => {
            file.auth = Some(AuthSection {
                access_token: access.to_string(),
                refresh_token: refresh.to_string(),
            });
            write_session(&file)
        }
    }
}

/// Rewrite just the token pair, keeping the cached `/v1/me` snapshot and
/// server URL already on disk. Used after a Supabase refresh, which rotates
/// both tokens but doesn't change the account shape.
fn update_tokens(access: &str, refresh: &str) -> Result<()> {
    let mut session = read_session()?.unwrap_or_default();
    if session.server_url.is_empty() {
        session.server_url = cloud_base_url();
    }
    match keyring_set(access, refresh) {
        Ok(()) => session.auth = None,
        Err(_) => {
            session.auth = Some(AuthSection {
                access_token: access.to_string(),
                refresh_token: refresh.to_string(),
            })
        }
    }
    write_session(&session)
}

fn clear_creds() -> Result<()> {
    let _ = keyring_delete();
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

/// Exchange the stored refresh token for a fresh Supabase session and persist
/// the rotated tokens. Returns the refreshed creds (cached plan/server URL
/// carried over). Callers that need a fresh `/v1/me` fetch it themselves with
/// the new access token.
///
/// This is the fix for "the account expires every time I restart": the
/// access token (a short-lived Supabase JWT) was never renewed, so any call
/// after it expired — the Refresh button's `/v1/me` or the poller's
/// `/v1/cloud/sync` — came back 401 and looked like a dead session, even
/// though the long-lived refresh token was sitting right there unused.
pub async fn refresh_active_session() -> Result<CloudCreds> {
    let Some(creds) = load_creds()? else {
        anyhow::bail!("Not signed in to Hoard Cloud.");
    };
    let (access, refresh) = refresh_supabase_session(&creds.refresh_token).await?;
    update_tokens(&access, &refresh)?;
    Ok(CloudCreds {
        access_token: access,
        refresh_token: refresh,
        ..creds
    })
}

// ---- commands ---------------------------------------------------------

/// Resolve the public URL the OAuth login flow starts from. The desktop UI
/// opens this in the system browser; the web app (hosted at `hoard.services`)
/// renders provider buttons that redirect via Supabase and hand the session
/// back to the app.
///
/// We start a loopback HTTP listener and pass its `port` to the web flow so the
/// browser returns the tokens via `http://127.0.0.1:<port>/callback`. This is
/// the only handoff that works with snap/flatpak-confined browsers (Ubuntu's
/// default Firefox is a snap), which silently drop custom `hoard://` schemes.
/// If the listener can't bind we fall back to the `hoard://` scheme, which
/// still works with non-confined browsers and on macOS.
#[tauri::command]
pub async fn cloud_login_url(app: AppHandle) -> String {
    let base = std::env::var("HOARD_CLOUD_PUBLIC_URL")
        .unwrap_or_else(|_| "https://hoard.services".to_string());
    match crate::commands::loopback::start(app).await {
        Ok(port) => format!("{base}/login?desktop=1&port={port}"),
        Err(e) => {
            tracing::warn!(error = %e, "loopback listener failed; using hoard:// scheme");
            format!("{base}/login?desktop=1")
        }
    }
}

/// Browser → `hoard://auth/callback?...` → deep-link handler → here.
/// Verifies the tokens by calling `/v1/me`, persists them, and returns the
/// account so the frontend can route into /account.
#[tauri::command]
pub async fn cloud_complete_login(
    app: AppHandle,
    access_token: String,
    refresh_token: String,
    state: State<'_, AppState>,
) -> Result<CloudAccount, String> {
    let access = access_token.trim().to_string();
    let refresh = refresh_token.trim().to_string();
    if access.is_empty() {
        return Err("Missing access token from auth callback.".into());
    }
    let base = cloud_base_url();
    let me = fetch_me(&base, &access).await.map_err(prettify)?;
    save_creds(&access, &refresh, &base, &me).map_err(|e| format!("Couldn't save session: {e}"))?;
    *state.cloud_account.lock().unwrap() = Some(me.clone());

    // Boot the cloud-pull poller so LiveStatus has fresh manifest data
    // within `prefs.cloud_poll_interval_secs`. Read the interval from
    // disk so the just-logged-in session honours the user's last choice.
    let secs = hoard_agent::prefs::Prefs::load_default()
        .map(|(p, _)| p.cloud_poll_interval_secs)
        .unwrap_or(10);
    cloud_pull::start(&app, secs);

    // A login completed, so any buffered deep-link URL has served its purpose.
    // Clearing it stops a stale (and by now expired) token from being replayed
    // the next time the frontend drains the buffer on mount.
    *state.pending_deep_link.lock().unwrap() = None;

    Ok(me)
}

/// Drain the buffered `hoard://` deep-link URL captured before the frontend
/// listener was ready (cold start). Returns `None` when there's nothing
/// pending. The frontend calls this once on mount, then relies on the live
/// `deep-link://new-url` event for anything that arrives afterwards.
#[tauri::command]
pub fn cloud_take_pending_deep_link(state: State<'_, AppState>) -> Option<String> {
    state.pending_deep_link.lock().unwrap().take()
}

/// Cached account from the on-disk session, or `None` when signed out.
#[tauri::command]
pub fn cloud_current_account(state: State<'_, AppState>) -> Option<CloudAccount> {
    state.cloud_account.lock().unwrap().clone()
}

/// True iff there's a cloud session on disk. Cheap; no network.
#[tauri::command]
pub fn cloud_is_logged_in(state: State<'_, AppState>) -> bool {
    state.cloud_account.lock().unwrap().is_some()
}

/// Re-fetch `/v1/me`. Updates the in-memory cache and the persisted
/// session snapshot. Returns the fresh account.
#[tauri::command]
pub async fn cloud_refresh_account(state: State<'_, AppState>) -> Result<CloudAccount, String> {
    let Some(creds) = load_creds().map_err(|e| e.to_string())? else {
        return Err("Not signed in to Hoard Cloud.".into());
    };
    // Try with the cached access token; if it expired, transparently renew it
    // with the refresh token and retry once instead of declaring the session
    // dead (the bug: a restart outlived the ~1h JWT, so this always 401'd).
    let (me, creds) = match fetch_me_raw(&creds.server_url, &creds.access_token).await {
        Ok(me) => (me, creds),
        Err(MeError::Unauthorized) => {
            let fresh = refresh_active_session().await.map_err(prettify)?;
            let me = fetch_me(&fresh.server_url, &fresh.access_token)
                .await
                .map_err(prettify)?;
            (me, fresh)
        }
        Err(other) => return Err(other.into_message()),
    };
    // Refresh both the disk and the in-memory cache so the bar updates.
    save_creds(
        &creds.access_token,
        &creds.refresh_token,
        &creds.server_url,
        &me,
    )
    .map_err(|e| format!("Couldn't update session: {e}"))?;
    *state.cloud_account.lock().unwrap() = Some(me.clone());
    Ok(me)
}

/// Drop the cloud session: clears the keyring entry, the on-disk file, and
/// the in-memory cache. Idempotent.
#[tauri::command]
pub fn cloud_logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    clear_creds().map_err(|e| format!("Couldn't clear session: {e}"))?;
    *state.cloud_account.lock().unwrap() = None;
    // Stop the cloud-pull poller — otherwise it would keep tickling
    // `load_active_creds` and quietly do nothing forever.
    cloud_pull::stop(&app);
    Ok(())
}

/// Kick off a server-side export job. Returns the job id; the frontend can
/// poll a future `/v1/me/export/:id` endpoint or just trust the email that
/// the server sends when it's done.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub job_id: String,
    pub status: String,
}

#[tauri::command]
pub async fn cloud_export_all() -> Result<ExportJob, String> {
    let Some(creds) = load_creds().map_err(|e| e.to_string())? else {
        return Err("Not signed in to Hoard Cloud.".into());
    };
    let url = format!("{}/v1/me/export", creds.server_url);
    let client = http_client().map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .bearer_auth(&creds.access_token)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_http_error(status, &body));
    }
    serde_json::from_str::<ExportJob>(&body)
        .map_err(|e| format!("Couldn't parse server response: {e}"))
}

/// Permanently delete the cloud account. The server soft-deletes for 30
/// days, after which a background job purges R2 + DB. The desktop clears
/// the session locally either way.
#[tauri::command]
pub async fn cloud_delete_account(state: State<'_, AppState>) -> Result<(), String> {
    let Some(creds) = load_creds().map_err(|e| e.to_string())? else {
        return Err("Not signed in to Hoard Cloud.".into());
    };
    let url = format!("{}/v1/me", creds.server_url);
    let client = http_client().map_err(|e| e.to_string())?;
    let resp = client
        .delete(&url)
        .bearer_auth(&creds.access_token)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format_http_error(status, &body));
    }
    // Clear local state regardless of body contents.
    clear_creds().map_err(|e| format!("Couldn't clear local session: {e}"))?;
    *state.cloud_account.lock().unwrap() = None;
    Ok(())
}

// ---- HTTP helpers -----------------------------------------------------

/// Error from `/v1/me` that distinguishes "token expired" (recoverable via a
/// Supabase refresh) from everything else.
enum MeError {
    Unauthorized,
    Other(String),
}

impl MeError {
    fn into_message(self) -> String {
        match self {
            MeError::Unauthorized => {
                "Your Hoard Cloud session expired. Please sign in again.".into()
            }
            MeError::Other(m) => m,
        }
    }
}

async fn fetch_me_raw(base: &str, token: &str) -> Result<CloudAccount, MeError> {
    let url = format!("{base}/v1/me");
    let client = http_client().map_err(|e| MeError::Other(e.to_string()))?;
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| MeError::Other(format!("Network error: {e}")))?;
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED {
        return Err(MeError::Unauthorized);
    }
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(MeError::Other(format_http_error(status, &body)));
    }
    serde_json::from_str::<CloudAccount>(&body)
        .map_err(|e| MeError::Other(format!("parsing /v1/me response: {e}: {body}")))
}

async fn fetch_me(base: &str, token: &str) -> Result<CloudAccount> {
    fetch_me_raw(base, token)
        .await
        .map_err(|e| anyhow::anyhow!(e.into_message()))
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
}

/// Hit Supabase GoTrue's `token?grant_type=refresh_token` endpoint to swap a
/// refresh token for a new access/refresh pair. The `apikey` header (anon
/// key) is required by the Supabase API gateway even for token refresh.
async fn refresh_supabase_session(refresh_token: &str) -> Result<(String, String)> {
    let refresh_token = refresh_token.trim();
    if refresh_token.is_empty() {
        anyhow::bail!("no refresh token stored — please sign in again");
    }
    let url = format!("{}/auth/v1/token?grant_type=refresh_token", supabase_url());
    let client = http_client()?;
    let resp = client
        .post(&url)
        .header("apikey", supabase_anon_key())
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("couldn't renew session ({status}): {body}");
    }
    let parsed: RefreshResponse = serde_json::from_str(&body)
        .with_context(|| format!("parsing token refresh response: {body}"))?;
    Ok((parsed.access_token, parsed.refresh_token))
}

fn format_http_error(status: StatusCode, body: &str) -> String {
    if status == StatusCode::UNAUTHORIZED {
        return "Your Hoard Cloud session expired. Please sign in again.".into();
    }
    if status == StatusCode::PAYMENT_REQUIRED {
        // /v1/me itself won't 402, but if a downstream call lands here we
        // surface the quota body as-is so the UI can render it.
        return body.to_string();
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = v.get("error").and_then(|x| x.as_str()) {
            return format!("Hoard Cloud: {msg} ({status})");
        }
    }
    format!("Hoard Cloud returned {status}: {body}")
}

fn prettify(err: anyhow::Error) -> String {
    err.to_string()
}

/// Restore the in-memory cache at boot. Best-effort; logs and shrugs if
/// the file is missing or unreadable so the rest of the app still starts.
pub fn rehydrate(state: &AppState) {
    match read_session() {
        Ok(Some(s)) => {
            if let Some(account) = s.user {
                *state.cloud_account.lock().unwrap() = Some(account);
            }
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "couldn't read cloud session"),
    }
}
