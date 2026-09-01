//! Hoard Cloud auth, browserless and shared between desktop and CLI.
//!
//! The desktop's web login (a browser redirect to `hoard://auth/callback`) is no
//! use in a terminal (SteamOS in gaming mode, a NAS, a server over SSH). What
//! lives here is the headless path: Supabase GoTrue by password grant or by an
//! emailed OTP code (`/auth/v1/otp` plus `/auth/v1/verify`), along with the JWT
//! refresh and the session's persistence.
//!
//! The persistence is deliberately interoperable with the desktop: the same
//! `keyring` (service `hoard-desktop-cloud`), the same
//! `<config>/desktop/cloud.toml` file and the same `AuthSection { access_token,
//! refresh_token }` shape. So a session started by the CLI is seen by the desktop
//! and the other way round: one Cloud session per machine. The `user` field (a
//! snapshot of `/v1/me`) is preserved verbatim as raw TOML across the CLI's
//! rewrites, so the desktop's account cache is never overwritten.
//!
//! ## Who writes the keyring (D.20)
//!
//! Only the daemon. A keychain item on macOS carries an ACL of authorised
//! binaries, and the only one on it is whichever binary creates it: with login
//! writing it from the app or the CLI, every read by the service was a foreign
//! binary asking the user for their keychain password, and since the keeper
//! retries the engine's start with backoff, a dialog came up every few seconds.
//! That is why [`store_tokens`] and [`clear_session`] belong to the daemon, and a
//! client that mints a session hands it over by IPC (`Request::AdoptSession`).
//! With no service to hand it to there are [`store_tokens_unlocked`] and
//! [`forget_tokens_unlocked`], which stop at the 0600 file and let the daemon lift
//! the pair into the keyring when it starts.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::keychain::{keyring_op, KeyringTimeout, KeyringUnreadable, KEYRING_TIMEOUT};

const CLOUD_DEFAULT_URL: &str = "https://api.hoard.services";
const KEYRING_SERVICE: &str = "hoard-desktop-cloud";
const KEYRING_USER: &str = "default";

// The public Supabase GoTrue project, the same one the web and the desktop use.
// The anon key is a public credential (it travels in the web's static bundle), so
// embedding it here exposes nothing new. All overridable by env so a dev build can
// point at another project.
const SUPABASE_DEFAULT_URL: &str = "https://zddepgqdiuhhzqdimsks.supabase.co";
const SUPABASE_DEFAULT_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InpkZGVwZ3FkaXVoaHpxZGltc2tzIiwicm9sZSI6ImFub24iLCJpYXQiOjE3Nzk2MzM2MTksImV4cCI6MjA5NTIwOTYxOX0.3nZebGwCzFO1byTqhowq9ip89GE9fMRxPscgYSlPzFk";

pub fn cloud_base_url() -> String {
    std::env::var("HOARD_CLOUD_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CLOUD_DEFAULT_URL.to_string())
}

pub fn supabase_url() -> String {
    std::env::var("HOARD_SUPABASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("HOARD_SUPABASE_URL").map(str::to_string))
        .unwrap_or_else(|| SUPABASE_DEFAULT_URL.to_string())
}

pub fn supabase_anon_key() -> String {
    std::env::var("HOARD_SUPABASE_ANON_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| option_env!("HOARD_SUPABASE_ANON_KEY").map(str::to_string))
        .unwrap_or_else(|| SUPABASE_DEFAULT_ANON_KEY.to_string())
}

/// A Supabase session's token pair. `access` is the short JWT (about an hour);
/// `refresh` is the long-lived one that renews it.
#[derive(Debug, Clone)]
pub struct Tokens {
    pub access: String,
    pub refresh: String,
}

/// Sesión Cloud activa cargada de disco: a qué servidor apunta y sus tokens.
#[derive(Debug, Clone)]
pub struct Session {
    pub server_url: String,
    pub access: String,
    pub refresh: String,
}

/// A sentinel: Supabase rejected the refresh because it had already been rotated
/// (`refresh_token_already_used` or `not_found`). Distinguished by type so
/// whoever refreshes can self-heal by adopting the tokens another run already left
/// on disk, rather than treating it as a dead session.
#[derive(Debug)]
pub struct RefreshTokenStale;

impl std::fmt::Display for RefreshTokenStale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("refresh token ya rotado por otra ejecución")
    }
}

impl std::error::Error for RefreshTokenStale {}

fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("construyendo cliente HTTP")
}

// ---- login sin navegador ----------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

/// Extracts `(access, refresh)` from a GoTrue response, or a readable error
/// carrying whatever message Supabase returned.
async fn parse_token_response(resp: reqwest::Response) -> Result<Tokens> {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status.is_success() {
        let parsed: TokenResponse = serde_json::from_str(&body).with_context(|| {
            format!(
                "parseando respuesta de login (status {status}, {} bytes)",
                body.len()
            )
        })?;
        return Ok(Tokens {
            access: parsed.access_token,
            refresh: parsed.refresh_token,
        });
    }
    bail!("{}", supabase_error_message(status, &body));
}

/// A human message from GoTrue's error body. Covers the usual shapes
/// (`error_description`, `msg`, `error`) without dumping raw JSON.
fn supabase_error_message(status: StatusCode, body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        for key in ["error_description", "msg", "error", "message"] {
            if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
                return s.to_string();
            }
        }
    }
    format!("Supabase devolvió {status}")
}

/// Login by email and password (`grant_type=password`). Direct when the account
/// has a password; accounts created only through Google or GitHub have none, so
/// they use OTP.
pub async fn login_password(email: &str, password: &str) -> Result<Tokens> {
    let url = format!("{}/auth/v1/token?grant_type=password", supabase_url());
    let resp = http_client()?
        .post(&url)
        .header("apikey", supabase_anon_key())
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    parse_token_response(resp).await
}

/// Asks Supabase to email an OTP code (`/auth/v1/otp`). Whether the email carries
/// a six-digit code as well as the magic link depends on the project's mail
/// template; if it only carries a link, this path does not complete and the
/// password grant has to be used.
pub async fn otp_start(email: &str) -> Result<()> {
    let url = format!("{}/auth/v1/otp", supabase_url());
    let resp = http_client()?
        .post(&url)
        .header("apikey", supabase_anon_key())
        .json(&serde_json::json!({ "email": email, "create_user": true }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    bail!(
        "no pude enviar el código: {}",
        supabase_error_message(status, &body)
    );
}

/// Exchanges the emailed OTP code for a session (`/auth/v1/verify`).
pub async fn otp_verify(email: &str, code: &str) -> Result<Tokens> {
    let url = format!("{}/auth/v1/verify", supabase_url());
    let resp = http_client()?
        .post(&url)
        .header("apikey", supabase_anon_key())
        .json(&serde_json::json!({ "type": "email", "email": email, "token": code.trim() }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    parse_token_response(resp).await
}

// ---- emparejamiento por móvil (device flow) ---------------------------

/// The response of `/v1/cloud/device/start`. The CLI shows `user_code` and
/// `verification_uri` (or the `_complete` one with the code already filled in) and
/// polls with `device_code` until the phone approves it.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    #[serde(default = "default_poll_interval")]
    pub interval_secs: u64,
    #[serde(default)]
    pub expires_in_secs: u64,
}

fn default_poll_interval() -> u64 {
    3
}

/// Estado de un emparejamiento al hacer polling.
pub enum DeviceStatus {
    Pending,
    Approved(Tokens),
    Denied,
    Expired,
}

/// Starts a pairing on the Cloud server rather than on Supabase: it is the server
/// that mints the session when the phone approves.
///
/// Returns `Ok(None)` when the server does not support pairing: an earlier version
/// without the `/v1/cloud/device/*` routes (404), a server that puts them behind
/// auth (401), or a Cloud with no `service_role` configured ("not configured"). In
/// those cases the caller has to fall back to the email login rather than blow up.
/// `Err` is left for real transport or parse failures.
pub async fn device_start(hostname: Option<&str>) -> Result<Option<DeviceStart>> {
    let url = format!("{}/v1/cloud/device/start", cloud_base_url());
    let resp = http_client()?
        .post(&url)
        .json(&serde_json::json!({ "hostname": hostname }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    // A server without the feature: 404 (no such route), 401 (behind auth in an
    // earlier version) or 501. Treated as unsupported, so it falls back to email.
    if matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::UNAUTHORIZED | StatusCode::NOT_IMPLEMENTED
    ) {
        return Ok(None);
    }
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        // Cloud reachable but with no `service_role`: `approve` answers "not
        // configured", and some deployments return it from `start` already.
        if body.contains("not configured") {
            return Ok(None);
        }
        bail!(
            "no pude iniciar el emparejamiento: {}",
            supabase_error_message(status, &body)
        );
    }
    let start = serde_json::from_str(&body).context("parseando respuesta de device/start")?;
    Ok(Some(start))
}

/// Asks whether the pairing has been approved (`/v1/cloud/device/poll`). Once it
/// has, it returns the tokens exactly once, since the server deletes the row.
pub async fn device_poll(device_code: &str) -> Result<DeviceStatus> {
    #[derive(Deserialize)]
    struct PollBody {
        status: String,
        access_token: Option<String>,
        refresh_token: Option<String>,
    }
    let url = format!("{}/v1/cloud/device/poll", cloud_base_url());
    let resp = http_client()?
        .post(&url)
        .json(&serde_json::json!({ "device_code": device_code }))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "error consultando el emparejamiento: {}",
            supabase_error_message(status, &body)
        );
    }
    let p: PollBody = serde_json::from_str(&body).context("parseando respuesta de device/poll")?;
    Ok(match p.status.as_str() {
        "approved" => match (p.access_token, p.refresh_token) {
            (Some(access), Some(refresh)) if !access.is_empty() && !refresh.is_empty() => {
                DeviceStatus::Approved(Tokens { access, refresh })
            }
            _ => DeviceStatus::Expired,
        },
        "denied" => DeviceStatus::Denied,
        "expired" => DeviceStatus::Expired,
        _ => DeviceStatus::Pending,
    })
}

/// Exchanges the refresh token for a new pair (`grant_type=refresh_token`).
///
/// It retries on transient failures (network, timeout, 5xx, 429) inside GoTrue's
/// reuse grace window: if the rotation reached the server but we lost the
/// response, claiming the same token within about ten seconds recovers the pair
/// already minted rather than orphaning it (which would later trip reuse detection
/// and end the session). A genuine rejection (`already_used` or `not_found`) comes
/// back as [`RefreshTokenStale`] with no retry.
pub async fn refresh(refresh_token: &str) -> Result<Tokens> {
    let refresh_token = refresh_token.trim();
    if refresh_token.is_empty() {
        bail!("no hay refresh token guardado — vuelve a iniciar sesión");
    }
    let url = format!("{}/auth/v1/token?grant_type=refresh_token", supabase_url());
    let client = Client::builder()
        .timeout(Duration::from_secs(7))
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("construyendo cliente de refresh")?;

    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let sent = client
            .post(&url)
            .header("apikey", supabase_anon_key())
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await;
        let resp = match sent {
            Ok(r) => r,
            Err(e) => {
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(600)).await;
                    continue;
                }
                return Err(anyhow::Error::new(e).context(format!("POST {url}")));
            }
        };
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() {
            let parsed: TokenResponse = serde_json::from_str(&body).with_context(|| {
                format!("parseando refresh (status {status}, {} bytes)", body.len())
            })?;
            return Ok(Tokens {
                access: parsed.access_token,
                refresh: parsed.refresh_token,
            });
        }
        let low = body.to_lowercase();
        if low.contains("already_used")
            || low.contains("already used")
            || low.contains("not_found")
            || low.contains("not found")
        {
            return Err(anyhow::Error::new(RefreshTokenStale));
        }
        if (status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS) && attempt < 3 {
            tokio::time::sleep(Duration::from_millis(600)).await;
            continue;
        }
        bail!("no pude renovar la sesión ({status}): {body}");
    }
}

// ---- /v1/me -----------------------------------------------------------

/// The minimal shape of `/v1/me` the CLI needs. `serde` ignores the fields we do
/// not list, so there is no need to mirror the desktop's whole `CloudAccount`.
#[derive(Debug, Clone, Deserialize)]
pub struct Me {
    pub user_id: String,
    pub email: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub storage_used_bytes: i64,
    #[serde(default)]
    pub storage_limit_bytes: i64,
}

/// GET `{base}/v1/me` with the JWT. It validates the token and fetches the account.
pub async fn fetch_me(base: &str, access: &str) -> Result<Me> {
    let url = format!("{}/v1/me", base.trim_end_matches('/'));
    let resp = http_client()?
        .get(&url)
        .bearer_auth(access)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED {
        bail!("la sesión Cloud caducó — vuelve a iniciar sesión");
    }
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("/v1/me devolvió {status}: {body}");
    }
    serde_json::from_str::<Me>(&body).with_context(|| format!("parseando /v1/me: {body}"))
}

// ---- persistencia (interoperable con el desktop) ----------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SessionFile {
    #[serde(default)]
    server_url: String,
    /// The `/v1/me` snapshot the desktop writes. We preserve it verbatim (raw
    /// TOML) so its cache is not overwritten; the CLI does not need it typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user: Option<toml::Value>,
    /// The fallback for when the keyring is unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth: Option<AuthSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthSection {
    access_token: String,
    refresh_token: String,
}

fn session_path() -> Result<PathBuf> {
    let dirs = crate::config::CliConfig::project_dirs()?;
    Ok(dirs.config_dir().join("desktop").join("cloud.toml"))
}

fn read_session_file() -> Result<Option<SessionFile>> {
    let path = session_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("leyendo {}", path.display()))?;
    let s: SessionFile =
        toml::from_str(&text).with_context(|| format!("parseando {}", path.display()))?;
    Ok(Some(s))
}

fn write_session_file(s: &SessionFile) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creando {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(s).context("serializando sesión Cloud")?;
    // An atomic write, temp plus rename, so a cut halfway through does not leave a
    // truncated TOML that looks like a broken session on start.
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, &text).with_context(|| format!("escribiendo {}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&tmp, perms)?;
    }
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("renombrando {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

// ---- the keyring, always bounded
//
// The cap, the dedicated thread and the typed reason live in `crate::keychain`,
// which is the same path the self-hosted token takes (`credentials`): one keyring,
// one thread.

fn keyring_set(access: &str, refresh: &str) -> Result<()> {
    let blob = toml::to_string(&AuthSection {
        access_token: access.to_string(),
        refresh_token: refresh.to_string(),
    })?;
    keyring_op("saving the Cloud session", KEYRING_TIMEOUT, move || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        entry.set_password(&blob)?;
        Ok(())
    })
}

fn keyring_get() -> Result<Option<AuthSection>> {
    keyring_op("reading the Cloud session", KEYRING_TIMEOUT, || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        match entry.get_password() {
            Ok(blob) => Ok(Some(toml::from_str(&blob)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

fn keyring_delete() -> Result<()> {
    keyring_op("deleting the Cloud session", KEYRING_TIMEOUT, || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    })
}

/// Loads the active Cloud session (the keyring first, the file as a fallback), or
/// `None` when there is no session.
pub fn load_session() -> Result<Option<Session>> {
    let Some(file) = read_session_file()? else {
        return Ok(None);
    };
    let Some(auth) = pick_auth(keyring_get(), file.auth.clone())? else {
        return Ok(None);
    };
    if auth.access_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(Session {
        server_url: if file.server_url.is_empty() {
            cloud_base_url()
        } else {
            file.server_url
        },
        access: auth.access_token,
        refresh: auth.refresh_token,
    }))
}

/// [`load_session`] off the runtime's thread.
///
/// Reading the keyring is synchronous and, although it is already bounded
/// ([`KEYRING_TIMEOUT`]), it blocks the thread doing it while it waits: on a
/// single-threaded runtime that stops everything else, and on any runtime an
/// `abort()` on the task that made the call goes unnoticed until it returns. With
/// `spawn_blocking` the wait lives on the blocking pool and the task awaiting it
/// can be cancelled at once, which is what keeps the engine's start from hanging
/// the daemon's shutdown (D.19).
///
/// Whoever resolves the session from a task, meaning the engine's start, uses this;
/// the synchronous paths (local CLI commands) stay on [`load_session`], which
/// cannot wait forever either.
pub async fn load_session_async() -> Result<Option<Session>> {
    match tokio::task::spawn_blocking(load_session).await {
        Ok(result) => result,
        // Only happens if `keyring` panicked. Saying so is infinitely better than
        // a start that hangs with no reason.
        Err(join) => Err(anyhow::Error::new(join).context("leyendo la sesión Cloud")),
    }
}

/// Which tokens count: the keyring's when it answers, the file's when the keyring
/// fails for something repairable (locked, no D-Bus in a headless session). That
/// is not "there is no session".
///
/// Swallowing the `Err` as though it were `NoEntry` fell back to the file, which
/// with a healthy keyring carries `auth = None` (see `store_tokens`), so
/// `load_session` returned `Ok(None)` and the user appeared signed out with their
/// tokens intact in the keyring. Only `NoEntry` falls back silently; a real error
/// propagates if the file has no tokens to offer either.
fn pick_auth(
    from_keyring: Result<Option<AuthSection>>,
    from_file: Option<AuthSection>,
) -> Result<Option<AuthSection>> {
    match from_keyring {
        Ok(Some(a)) => Ok(Some(a)),
        Ok(None) => Ok(from_file),
        Err(e) => match from_file {
            Some(a) => {
                tracing::debug!(error = %e, "keyring unreadable; using the tokens from the file");
                Ok(Some(a))
            }
            // With nothing in the file, the keyring's error IS the answer and has
            // to reach `last_error` and the log whole: it is the only clue that
            // the keyring is locked. An exhausted cap explains itself; any other
            // failure gets the typed reason attached.
            //
            // That typed reason is what the Cloud path was missing while the
            // self-hosted one had it: everything that wasn't our own cap arrived
            // as plain context, classified as `EngineDownReason::Other`, and the
            // window showed the generic "the service is offline" banner. Seven
            // users on Linux, no session, and the one thing that would have
            // explained it dropped one line before it could be said.
            None if e.is::<KeyringTimeout>() => Err(e),
            None => Err(e.context(KeyringUnreadable {
                doing: "reading the Cloud session",
            })),
        },
    }
}

/// Persists the token pair, keeping the `user` and `server_url` already on disk
/// (read-modify-write). The keyring first; failing that, the 0600 file.
///
/// Only the daemon calls this. It is the write that creates the keychain item, and
/// on macOS whoever creates it is the only binary its ACL authorises: if a client
/// wrote it, every read by the service would be a foreign binary asking the user
/// for their password (ADR 0021 D.20). A client that has just minted a session
/// hands it over by IPC (`Request::AdoptSession`); with no service to hand it to,
/// it uses [`store_tokens_unlocked`].
pub fn store_tokens(tokens: &Tokens, server_url: &str) -> Result<()> {
    let mut session = read_session_file()?.unwrap_or_default();
    if session.server_url.is_empty() {
        session.server_url = server_url.to_string();
    }
    match store_in_keyring(tokens) {
        Ok(()) => session.auth = None,
        Err(err) => {
            tracing::warn!(
                error = %format!("{err:#}"),
                "keyring: keeping the Cloud session in the protected file instead"
            );
            session.auth = Some(AuthSection {
                access_token: tokens.access.clone(),
                refresh_token: tokens.refresh.clone(),
            })
        }
    }
    write_session_file(&session)
}

/// Write the pair to the keyring **and read it back**, so that "the keyring took
/// it" is something we know rather than something we assumed.
///
/// A `set_password` that returns `Ok` is not proof the entry can be read: the
/// error users actually hit is `Platform secure storage failure: Crypto error:
/// Unpad Error`, a keyring that accepts writes and can't decrypt what it holds.
/// Trusting the write is what turned that into a lockout — the caller sets
/// `auth = None`, so the only copy of the session lives in a store that will
/// never give it back, and the machine stops syncing with nothing on disk to
/// recover from. The read-back is one extra call on a healthy keyring, in the
/// milliseconds it answers in, and it is the whole difference between degrading
/// to the 0600 file and having no session at all.
fn store_in_keyring(tokens: &Tokens) -> Result<()> {
    keyring_set(&tokens.access, &tokens.refresh)?;
    match keyring_get() {
        Ok(Some(saved)) if saved.access_token == tokens.access => Ok(()),
        Ok(_) => bail!("the keyring accepted the session and didn't give it back"),
        Err(err) => Err(err.context("reading back the session we just saved")),
    }
}

/// Persists the pair without touching the keyring: the 0600 file and nothing else.
///
/// This is the path for a client that has just minted a session and has no service
/// to hand it to (the daemon never started, or is updating). Writing the keyring
/// here would "work" and would be exactly D.20's bug: the item would end up in the
/// client's name and the service would ask permission on every read. Leaving it in
/// the file, the daemon picks it up as-is on start ([`pick_auth`] falls back to the
/// file when the keyring has no entry) and on its first refresh moves it into the
/// keyring itself, as the owner. So it heals on its own, and with no dialog.
///
/// The file is 0600 in the user's config directory, the same protection the
/// fallback for keyring-less machines already has.
pub fn store_tokens_unlocked(tokens: &Tokens, server_url: &str) -> Result<()> {
    let mut session = read_session_file()?.unwrap_or_default();
    if session.server_url.is_empty() {
        session.server_url = server_url.to_string();
    }
    session.auth = Some(AuthSection {
        access_token: tokens.access.clone(),
        refresh_token: tokens.refresh.clone(),
    });
    write_session_file(&session)
}

/// Deletes the Cloud session (keyring and file).
///
/// The daemon's, for the same reason as [`store_tokens`]: deleting a keychain item
/// is also authorised, and only its owner does it without asking. A client sends
/// `Request::ForgetSession` and, with no service, [`forget_tokens_unlocked`].
pub fn clear_session() -> Result<()> {
    let _ = keyring_delete();
    // And the log shipper's slot: it is an in-memory copy of the JWT, so a logout
    // that does not empty it leaves the shipper sending with the session that was
    // just closed.
    crate::credentials::set_lent_cloud(None);
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("borrando {}", path.display()))?;
    }
    Ok(())
}

/// Signs out without touching the keyring: it deletes the file and stops there.
///
/// The partner of [`store_tokens_unlocked`], for a logout with no service. It is
/// enough to leave the machine disconnected: with no session file there is no
/// session ([`load_session`] starts there), even though the keychain item still
/// exists holding a pair that is no good. The next login overwrites it, and
/// meanwhile it authorises nothing: an orphaned refresh token is not a session.
pub fn forget_tokens_unlocked() -> Result<()> {
    // The log shipper's slot, as in [`clear_session`]: it is an in-memory copy of
    // the JWT and deleting the file does not empty it. Both logout paths have to do
    // it, or `hoard logout` with no service would leave the process shipping with
    // the session it just closed.
    crate::credentials::set_lent_cloud(None);
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("borrando {}", path.display()))?;
    }
    Ok(())
}

/// The stored session's JWT `user_id` (the `sub` claim), with no network. It lets
/// local commands (`hoard saves`) pin the right Cloud context without refreshing
/// the token or calling `/v1/me`. `None` when there is no session or the JWT
/// cannot be decoded.
pub fn session_user_id() -> Result<Option<String>> {
    let Some(sess) = load_session()? else {
        return Ok(None);
    };
    Ok(jwt_sub(&sess.access))
}

/// Decodes a JWT's `sub` claim (the second segment, base64url with no padding).
/// It does not verify the signature; it only reads the `user_id`, which the server
/// revalidates on every request.
fn jwt_sub(jwt: &str) -> Option<String> {
    jwt_claims(jwt)?.get("sub")?.as_str().map(String::from)
}

/// A JWT's `exp` in epoch seconds. `None` when the token cannot be decoded or does
/// not carry one, and whoever decides on this has to treat that as "I do not know
/// how long it has left", never as "it has plenty".
pub fn jwt_expiry(jwt: &str) -> Option<i64> {
    jwt_claims(jwt)?.get("exp")?.as_i64()
}

fn jwt_claims(jwt: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Is this refresh failure one only a fresh `hoard login` fixes (GoTrue revoked
/// the family), as against a network bump that deserves a retry?
pub fn is_session_expired(err: &anyhow::Error) -> bool {
    err.downcast_ref::<RefreshTokenStale>().is_some()
}

// ---- refresh centralizado ---------------------------------------------

/// The window in which a just-completed refresh is reused rather than another one
/// being asked for. GoTrue rotates the refresh token on every use and revokes the
/// previous one, so a burst of callers (the periodic refresher and a token
/// rejected by realtime, say) has to collapse into a single trip: the second
/// replay would trip reuse detection on an already-rotated token.
const REFRESH_REUSE_WINDOW: Duration = Duration::from_secs(30);

/// Serialises every refresh in the process and remembers the last rotated pair.
fn refresh_gate() -> &'static tokio::sync::Mutex<Option<(Instant, Tokens)>> {
    static GATE: OnceLock<tokio::sync::Mutex<Option<(Instant, Tokens)>>> = OnceLock::new();
    GATE.get_or_init(|| tokio::sync::Mutex::new(None))
}

/// Refreshes the Cloud session with the freshest token on disk and persists the
/// rotated pair. The process's only refresh path, on purpose.
///
/// The reason it does not accept a `Session`: each caller used to refresh with its
/// own in-memory copy, and a copy captured minutes earlier (realtime takes one on
/// connect and its connection lives up to `CONNECTION_MAX_SECS`) could replay a
/// token the periodic refresher had already rotated. Outside GoTrue's grace window
/// that is not a retry but reuse detection, and the answer is to revoke the whole
/// token family: a dead session with no recovery, not even by restarting.
///
/// Three layers: the mutex serialises concurrent refreshes, re-reading the disk
/// inside the lock stops an old copy going out, and [`REFRESH_REUSE_WINDOW`]
/// collapses bursts. The heal covers what falls outside the process (the desktop
/// shares the same session file): if GoTrue says stale, the disk is re-read and
/// somebody else's rotation is adopted.
pub async fn refresh_freshest() -> Result<Tokens> {
    // Holding the lock across the network call is what serialises this.
    let mut last = refresh_gate().lock().await;

    if let Some((at, tokens)) = last.as_ref() {
        if at.elapsed() < REFRESH_REUSE_WINDOW {
            return Ok(tokens.clone());
        }
    }

    let Some(sess) = load_session()? else {
        bail!("no hay sesión Cloud — vuelve a iniciar sesión");
    };
    let attempted = sess.refresh.clone();
    match refresh(&sess.refresh).await {
        Ok(tokens) => {
            store_tokens(&tokens, &sess.server_url)?;
            *last = Some((Instant::now(), tokens.clone()));
            Ok(tokens)
        }
        Err(e) if e.downcast_ref::<RefreshTokenStale>().is_some() => {
            match adoptable(&attempted, load_session().ok().flatten().as_ref()) {
                Some(tokens) => {
                    tracing::debug!(
                        "cloud: another run rotated the refresh token; adopting the one on disk"
                    );
                    *last = Some((Instant::now(), tokens.clone()));
                    Ok(tokens)
                }
                None => Err(e.context("la sesión Cloud caducó — vuelve a iniciar sesión")),
            }
        }
        Err(e) => Err(e),
    }
}

/// Decides whether the session on disk is any use for healing a
/// [`RefreshTokenStale`]: only when it carries a non-empty refresh token that is
/// different from the one just tried. If it were the same, retrying would give
/// stale again, since there is nothing to adopt.
fn adoptable(attempted: &str, on_disk: Option<&Session>) -> Option<Tokens> {
    let s = on_disk?;
    (!s.refresh.trim().is_empty() && s.refresh != attempted).then(|| Tokens {
        access: s.access.clone(),
        refresh: s.refresh.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(refresh: &str) -> Session {
        Session {
            server_url: "https://api.hoard.services".to_string(),
            access: "fresh-jwt".to_string(),
            refresh: refresh.to_string(),
        }
    }

    #[test]
    fn adopts_a_rotation_left_by_another_process() {
        let got = adoptable("ours", Some(&disk("rotated-by-the-desktop"))).expect("adoptable");
        assert_eq!(got.refresh, "rotated-by-the-desktop");
        assert_eq!(got.access, "fresh-jwt");
    }

    #[test]
    fn refuses_to_replay_the_token_that_just_failed() {
        // Mismo token en disco: nadie rotó nada, la sesión está muerta de verdad.
        assert!(adoptable("ours", Some(&disk("ours"))).is_none());
    }

    #[test]
    fn ignores_an_empty_or_absent_session() {
        assert!(adoptable("ours", Some(&disk("   "))).is_none());
        assert!(adoptable("ours", None).is_none());
    }

    /// El `exp` se lee sin verificar firma, y un token que no se puede decodificar
    /// devuelve `None` (no un número optimista): quien presta el token trata
    /// `None` como "rota por si acaso".
    #[test]
    fn reads_the_expiry_of_a_jwt_and_admits_ignorance() {
        use base64::Engine;
        let body = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"sub":"u-1","exp":1800000000}"#);
        let jwt = format!("header.{body}.signature");
        assert_eq!(jwt_expiry(&jwt), Some(1_800_000_000));
        assert_eq!(jwt_sub(&jwt).as_deref(), Some("u-1"));

        assert_eq!(jwt_expiry("not-a-jwt"), None);
        let no_exp = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"sub":"u"}"#);
        assert_eq!(jwt_expiry(&format!("h.{no_exp}.s")), None);
    }

    // ---- el llavero bloqueado (D.19) ----------------------------------
    //
    // El tope en sí se prueba en `crate::keychain`, que es donde vive. Aquí sólo
    // lo que es de esta sesión: que un llavero bloqueado no se lea como "no hay
    // sesión Cloud".

    fn stuck() -> anyhow::Error {
        anyhow::Error::new(KeyringTimeout {
            doing: "reading the Cloud session",
            after: KEYRING_TIMEOUT,
        })
    }

    fn tokens_in_the_file() -> AuthSection {
        AuthSection {
            access_token: "jwt-del-fichero".to_string(),
            refresh_token: "refresh-del-fichero".to_string(),
        }
    }

    /// Sin tokens en el fichero, un llavero bloqueado **no** puede parecer "no hay
    /// sesión": el motivo sale entero para que el motor lo publique en vez de
    /// quedarse en `starting` sin una línea de log.
    #[test]
    fn a_locked_keyring_surfaces_the_reason_instead_of_looking_logged_out() {
        let err = pick_auth(Err(stuck()), None).expect_err("no puede ser Ok(None)");
        assert!(err.is::<KeyringTimeout>(), "{err:#}");
        assert!(format!("{err:#}").contains("locked"), "{err:#}");
    }

    /// Pero con tokens en el fichero (el fallback 0600) un llavero bloqueado no
    /// desloguea a nadie: se sigue con lo que hay, que es lo que ya hacía.
    #[test]
    fn a_locked_keyring_still_falls_back_to_the_file_tokens() {
        let got = pick_auth(Err(stuck()), Some(tokens_in_the_file()))
            .expect("el fichero salva la sesión")
            .expect("tokens");
        assert_eq!(got.access_token, "jwt-del-fichero");
    }

    /// The gap the Cloud path had that the self-hosted one didn't: a keyring that
    /// **answers and refuses** carried no typed reason, so it arrived at the
    /// window as `EngineDownReason::Other` and got the generic "the service is
    /// offline" banner. Seven Linux users, no Cloud session, and the sentence
    /// that would have explained it dropped one line short.
    #[test]
    fn a_refusing_keyring_carries_a_typed_reason_too() {
        let refused = anyhow::Error::new(keyring::Error::PlatformFailure(
            "Crypto error: Unpad Error".into(),
        ));
        let err = pick_auth(Err(refused), None).expect_err("no puede ser Ok(None)");
        assert!(
            err.downcast_ref::<KeyringUnreadable>().is_some(),
            "the reason has to be typed, not just in the text: {err:#}"
        );
        // And the finer classification survives the wrapping, so the window can
        // say *which* way it failed instead of the general keyring line.
        assert_eq!(
            crate::keychain::fault(&err),
            Some(crate::keychain::KeyringFault::Damaged)
        );
    }

    /// Y un llavero sano gana al fichero, con o sin fichero.
    #[test]
    fn a_healthy_keyring_wins_and_an_empty_one_falls_back() {
        let from_keyring = AuthSection {
            access_token: "jwt-del-llavero".to_string(),
            refresh_token: "refresh-del-llavero".to_string(),
        };
        let got = pick_auth(Ok(Some(from_keyring)), Some(tokens_in_the_file()))
            .expect("ok")
            .expect("tokens");
        assert_eq!(got.access_token, "jwt-del-llavero");

        // `NoEntry`: no hay entrada, no hay fallo. Cae al fichero en silencio.
        let got = pick_auth(Ok(None), Some(tokens_in_the_file()))
            .expect("ok")
            .expect("tokens");
        assert_eq!(got.access_token, "jwt-del-fichero");
        assert!(pick_auth(Ok(None), None).expect("ok").is_none());
    }

    /// Aísla el directorio de config en un tempdir. Sólo Linux: es donde
    /// `ProjectDirs` mira `XDG_CONFIG_HOME`. En macOS y Windows la ruta sale de
    /// APIs del sistema y un test así escribiría en la sesión de verdad de quien
    /// ejecuta los tests, que es exactamente lo que no puede pasar.
    #[cfg(target_os = "linux")]
    fn with_isolated_config(f: impl FnOnce()) {
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        f();
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    /// El camino degradado de D.20: un cliente que acuña una sesión y no tiene
    /// servicio a quien entregarla la deja en el fichero 0600 y **no** en el
    /// llavero. Lo que se comprueba es que el par queda donde el daemon lo va a
    /// encontrar (`pick_auth` cae al fichero cuando el llavero no tiene entrada),
    /// que es lo que hace que se cure solo en el primer refresh del servicio.
    ///
    /// Que no toque el llavero no se puede afirmar desde un test sin leer el
    /// llavero de verdad; lo sostiene el tipo: esta función no tiene ninguna
    /// llamada a `keyring_*`.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_session_stored_without_a_service_lands_in_the_file() {
        with_isolated_config(|| {
            let tokens = Tokens {
                access: "jwt-sin-servicio".to_string(),
                refresh: "refresh-sin-servicio".to_string(),
            };
            store_tokens_unlocked(&tokens, "https://api.hoard.services").expect("escribe");

            let file = read_session_file().expect("lee").expect("hay fichero");
            assert_eq!(file.server_url, "https://api.hoard.services");
            let auth = file.auth.clone().expect("el par está en el fichero");
            assert_eq!(auth.access_token, "jwt-sin-servicio");
            assert_eq!(auth.refresh_token, "refresh-sin-servicio");

            // Con el llavero sin entrada (lo normal en este camino), el par del
            // fichero es el que gana: el daemon arranca con la sesión.
            let picked = pick_auth(Ok(None), file.auth)
                .expect("ok")
                .expect("hay tokens");
            assert_eq!(picked.refresh_token, "refresh-sin-servicio");

            // Y 0600: es el mismo grado de protección que el fallback histórico.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(session_path().unwrap())
                    .unwrap()
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o600, "modo {:o}", mode & 0o777);
            }

            // El logout sin servicio: sin fichero no hay sesión que resolver, dé
            // lo que dé el llavero.
            forget_tokens_unlocked().expect("olvida");
            assert!(read_session_file().expect("lee").is_none());
        });
    }
}
