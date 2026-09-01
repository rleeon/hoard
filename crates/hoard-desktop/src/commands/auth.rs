//! Authentication & onboarding commands.
//!
//! These handle the wizard's three round-trips to the server:
//!
//! 1. `health_check`: an anonymous probe to confirm the URL points at a Hoard
//!    server before we ask for a token.
//! 2. `login`: exchanges (URL, token) for a verified `whoami` response and
//!    persists the credentials.
//! 3. `logout`: clears credentials.
//!
//! `is_logged_in` and `current_user` read from the in-memory cache that the
//! app populates at startup from the on-disk session.

use hoard_agent::api::{ApiClient, ApiError, RateLimitKind};
use hoard_agent::credentials::{self, Credentials, UserSection};
use hoard_core::ipc::{ServerSession, ServerUser};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

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
///
/// Quota fields come from the server's `whoami` response (extended in
/// v0.3, see `hoard-server/src/routes/auth.rs`). `is_local_server` is a
/// client-side classification the UI uses to pick MB display (self-hosted at
/// home) over % display (an external SaaS server); see [`classify_server`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
    pub server_url: String,
    pub storage_used_bytes: i64,
    pub storage_quota_bytes: i64,
    pub is_local_server: bool,
    /// True when the URL points at the managed Hoard Cloud backend
    /// (`*.hoard.services` / `*.fly.dev`). The cloud upgrades itself, so the
    /// UI hides the self-hosted "upgrade server" panel for these; see
    /// [`classify_cloud`]. It avoids the `/v1/admin/upgrade` 404 a cloud box
    /// returns (it has no such route).
    pub is_cloud_server: bool,
    /// The three limits the account page shows for a self-hosted server. All
    /// `None` until the first `whoami` of the session: the disk cache holds
    /// identity only, and a server too old to report them reads as "no limit
    /// known" rather than as zero.
    pub max_snapshot_size_bytes: Option<i64>,
    pub max_versions: Option<i64>,
    pub max_manual_versions: Option<i64>,
}

/// Probe `/v1/health` without auth. Frontend uses this in the wizard to give
/// the user fast feedback that the URL points at a working Hoard server
/// before they paste a token.
#[tauri::command]
pub async fn health_check(url: String) -> Result<HealthInfo, String> {
    let url = hoard_agent::serverclass::normalize_server_url(&url);
    validate_url(&url)?;

    // The token field is unused by `/v1/health`, but `ApiClient::new` requires
    // the field to exist. Pass an empty string: it never goes on the wire.
    let client = ApiClient::new(url, String::new()).map_err(pretty_error)?;
    let h = client.health().await.map_err(probe_error)?;
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
    app: AppHandle,
    url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<UserInfo, String> {
    let url = hoard_agent::serverclass::normalize_server_url(&url);
    validate_url(&url)?;

    if !credentials::is_valid_token(&token) {
        return Err(
            "That doesn't look like a Hoard access key. It should start with `hoard_v1_` \
             followed by 64 lowercase hex characters."
                .into(),
        );
    }

    let client = ApiClient::new(url.clone(), token.clone()).map_err(pretty_error)?;
    let who = client.whoami().await.map_err(probe_error)?;

    let user = UserInfo {
        user_id: who.user_id.clone(),
        username: who.username.to_string(),
        is_admin: who.is_admin,
        server_url: url.clone(),
        storage_used_bytes: who.storage_used_bytes,
        storage_quota_bytes: who.storage_quota_bytes,
        is_local_server: classify_server(&url),
        is_cloud_server: classify_cloud(&url),
        max_snapshot_size_bytes: who.max_snapshot_size_bytes,
        max_versions: who.max_versions,
        max_manual_versions: who.max_manual_versions,
    };

    hand_over(
        &app,
        &Credentials {
            url,
            token,
            user: Some(UserSection::from(who)),
        },
    )
    .await
    .map_err(|e| format!("Couldn't save credentials: {e}"))?;

    *state.user.lock().unwrap() = Some(user.clone());
    // Point per-context state at this self-hosted server.
    crate::commands::library::sync_active_context(state.inner());
    // Rehydrate the automatic-mode schedulers for this session if the toggle
    // was left on. A cold start does this in Tauri `setup()`; a hot login would
    // otherwise leave the periodic scan/track/sweep dead until the next launch.
    // (The UI also boots the agent via `signIn`; `run_scan` here is idempotent.)
    if let Err(e) = crate::commands::automatic::restart_if_enabled(&app).await {
        tracing::warn!(error = %e, "login: couldn't rehydrate automatic schedulers");
    }
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
pub async fn logout(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    forget(&app)
        .await
        .map_err(|e| format!("Couldn't clear credentials: {e}"))?;
    *state.user.lock().unwrap() = None;
    // Repoint at whatever session remains (a cloud login, or none).
    crate::commands::library::sync_active_context(state.inner());
    Ok(())
}

/// Hands the session to the service, which owns it, and keeps it in the loan slot
/// too so it does not have to be asked for back.
///
/// The app **does not write** the keyring (D.20): on macOS its ACL only authorises
/// the binary that creates the item, and what reads it on every engine start is
/// `hoardd`. With the service writing it, creator and reader are the same binary.
async fn hand_over(app: &AppHandle, creds: &Credentials) -> anyhow::Result<()> {
    let session = ServerSession {
        server_url: creds.url.clone(),
        token: creds.token.clone(),
        user: creds.user.as_ref().map(|u| ServerUser {
            user_id: u.user_id.clone(),
            username: u.username.clone(),
            is_admin: u.is_admin,
        }),
    };
    let handed = match app.try_state::<AppState>() {
        Some(state) => match state.daemon.adopt_server_session(session).await {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!(error = %format!("{err:#}"), "login: the service didn't take the new session");
                false
            }
        },
        None => false,
    };
    if !handed {
        // With no service: to the 0600 file and **not** to the keyring. The service
        // reads it from there when it starts and promotes it into the keyring itself,
        // as its owner.
        credentials::save_unlocked(creds)?;
        // And since it did not learn from the handover, it gets nudged in case it
        // started in between: an engine down for want of a session would keep waiting
        // out its backoff (up to 5 min) after the user has already signed in.
        crate::commands::agent::notify_session_changed(app);
    }
    // The log shipper re-reads the session every few seconds from its own thread and
    // cannot ask for anything over IPC, so it is left in place for it.
    credentials::set_lent(Some(creds.clone()));
    Ok(())
}

/// The self-hosted session this process may use: the one already lent to us, or one
/// we ask the service for. **Never the keyring**, since the item is its own (D.20).
///
/// `Ok(None)` means there is no self-hosted session on this machine, which is what
/// the callers used to translate into "not logged in". A downed service is `Err`:
/// that is not the same as being signed out, and saying it is sends the user to the
/// onboarding wizard with their session intact.
///
/// It is asked for **once per run** because a `hoard_v1_` token is static: it does
/// not expire, it is not rotated, there is nothing to refresh.
pub(crate) async fn server_session(app: &AppHandle) -> anyhow::Result<Option<Credentials>> {
    if let Some(creds) = credentials::lent() {
        return Ok(Some(creds));
    }
    let Some(state) = app.try_state::<AppState>() else {
        anyhow::bail!("the Hoard service link isn't up yet");
    };
    match state.daemon.server_session().await {
        Ok(session) => {
            let creds = Credentials {
                url: session.server_url,
                token: session.token,
                user: session.user.map(|u| UserSection {
                    user_id: u.user_id,
                    username: u.username,
                    is_admin: u.is_admin,
                }),
            };
            credentials::set_lent(Some(creds.clone()));
            Ok(Some(creds))
        }
        Err(err) => match err.downcast_ref::<hoard_core::ipc::IpcError>() {
            Some(hoard_core::ipc::IpcError::NoServerSession { .. }) => Ok(None),
            _ => Err(err),
        },
    }
}

/// [`server_session`] o el `String` que la UI ya esperaba.
pub(crate) async fn require_server_session(app: &AppHandle) -> Result<Credentials, String> {
    match server_session(app).await {
        Ok(Some(creds)) => Ok(creds),
        Ok(None) => Err("Not logged in.".into()),
        Err(err) => Err(format!("{err:#}")),
    }
}

/// Signs out of the self-hosted session: we mark the file, and the keyring item's
/// owner deletes it.
pub(crate) async fn forget(app: &AppHandle) -> anyhow::Result<()> {
    credentials::set_lent(None);
    let mut forgotten = false;
    if let Some(state) = app.try_state::<AppState>() {
        match state.daemon.forget_server_session().await {
            Ok(()) => forgotten = true,
            Err(err) => {
                tracing::warn!(error = %format!("{err:#}"), "logout: the service didn't clear its stored session");
            }
        }
    }
    if !forgotten {
        // The marker, not a delete: `credentials::load` recovers the session from
        // the keyring blob when the file is missing, so deleting it would resurrect
        // it.
        credentials::forget_unlocked()?;
        crate::commands::agent::notify_session_changed(app);
    }
    Ok(())
}

/// Re-fetch quota from the server. Cheap (one HTTP round-trip, no body): the
/// dashboard polls this every 30s or so while open, so the % bar tracks
/// reality without forcing a full re-login. Updates the cached
/// `UserInfo` in place and returns the new copy for convenience.
#[tauri::command]
pub async fn refresh_quota(app: AppHandle, state: State<'_, AppState>) -> Result<UserInfo, String> {
    let snapshot = state.user.lock().unwrap().clone();
    let Some(current) = snapshot else {
        return Err("Not logged in.".into());
    };
    let creds = require_server_session(&app).await?;
    let url = creds.url.clone();
    let client = ApiClient::new(creds.url, creds.token).map_err(pretty_error)?;
    let who = match client.whoami().await {
        Ok(who) => who,
        Err(e) => {
            // A self-hosted access key is static: it doesn't expire on a
            // timer like the cloud JWT. So a 401 here means the key was
            // revoked or the server was reset/replaced: the session is dead,
            // not stale. Clear it so the router drops back to the onboarding
            // wizard instead of looping forever on a dashboard that can't
            // talk to the server (the "didn't accept that access key" toast
            // on every 30s poll). Other failures (network blips, 5xx) leave
            // the session intact and keep the last known numbers on screen.
            if matches!(e.downcast_ref::<ApiError>(), Some(ApiError::Unauthorized)) {
                let _ = forget(&app).await;
                *state.user.lock().unwrap() = None;
                crate::commands::library::sync_active_context(state.inner());
            }
            return Err(pretty_error(e));
        }
    };

    let updated = UserInfo {
        storage_used_bytes: who.storage_used_bytes,
        storage_quota_bytes: who.storage_quota_bytes,
        // Reclassify each poll so a heuristic change, or a server that moved onto a
        // LAN or Tailscale name, takes effect without forcing a re-login.
        is_local_server: classify_server(&url),
        // The operator can raise any of these and restart; the account page
        // follows on the next poll instead of until the next sign-in.
        max_snapshot_size_bytes: who.max_snapshot_size_bytes,
        max_versions: who.max_versions,
        max_manual_versions: who.max_manual_versions,
        ..current
    };
    *state.user.lock().unwrap() = Some(updated.clone());
    Ok(updated)
}

// The "local versus external SaaS" and "is this Hoard Cloud" heuristics live in the
// agent (`hoard_agent::serverclass`) so the CLI shares exactly the same rule. The
// historic names stay as thin aliases.
pub(crate) use hoard_agent::serverclass::is_cloud_host as classify_cloud;
pub(crate) use hoard_agent::serverclass::is_local_server as classify_server;

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
/// can act on. Falls through to the raw message for shapes we don't recognise; the
/// frontend logs those to the console for bug reports.
pub(crate) fn pretty_error(err: anyhow::Error) -> String {
    if let Some(api) = err.downcast_ref::<ApiError>() {
        return match api {
            ApiError::Unauthorized => {
                // Shared by self-hosted (bearer token) and cloud (expired
                // Supabase JWT). The old copy assumed a self-hosted "access
                // key", which read as nonsense to a cloud user whose session had
                // simply expired, so keep it neutral and cover both.
                "The server rejected your session. Sign in again, or on a self-hosted server, double-check your access key.".into()
            }
            ApiError::Forbidden => {
                "Your access key is valid but the server isn't letting it do that.".into()
            }
            ApiError::Archived => {
                "This game is archived. Reactivate it from your Library to sync it again.".into()
            }
            ApiError::NotFound => {
                // A 404 on a data fetch (history, library, a single save) means
                // the resource is gone server-side: typically a save that was
                // deleted, or stale local state pointing at one. The "wrong URL,
                // not a Hoard server" reading only holds for the setup probe,
                // which uses `probe_error` instead.
                "That save no longer exists on the server (it may have been deleted).".into()
            }
            ApiError::Server { status, .. } if *status >= 500 => {
                format!("The server returned an error ({status}). Try again in a moment.")
            }
            ApiError::Server { status, body } => {
                format!("Server replied with {status}: {body}")
            }
            ApiError::Network(e) => network_message(e),
            ApiError::TooLarge(detail) => detail.human(),
            ApiError::QuotaExceeded(detail) => detail.human(),
            // Two different sentences because they're two different problems:
            // a budget is something the account ran out of and has to wait out,
            // while pacing is the server asking this machine to send requests
            // more slowly. Telling someone they hit "the bandwidth limit" when
            // the server only wanted them to slow down sends them looking at
            // their plan for a problem that isn't there.
            ApiError::RateLimited {
                kind: RateLimitKind::Budget,
                retry_after_seconds,
                ..
            } => format!(
                "You've hit the bandwidth limit for now. Try again in about {retry_after_seconds}s."
            ),
            ApiError::RateLimited {
                kind: RateLimitKind::Paced,
                ..
            } => {
                "The server is limiting how fast requests can arrive. Try again in a moment.".into()
            }
            // Names the host and says whose problem it is. The bytes go
            // straight to the bucket, so this failure has nothing to do with
            // the Hoard server the user just typed in, and every second spent
            // checking that address is a second not spent on the network that
            // is actually broken.
            ApiError::StorageUnreachable { host, .. } => format!(
                "Can't reach the storage endpoint ({host}). Hoard's server answered fine, \
                 so this is the connection between this machine and the storage: a VPN, \
                 a firewall, or the network's route to it."
            ),
            ApiError::NonFastForward(d) => d.human(),
            ApiError::Conflict(msg) | ApiError::BadRequest(msg) => msg.clone(),
        };
    }
    if let Some(req) = err.downcast_ref::<reqwest::Error>() {
        return network_message(req);
    }
    err.to_string()
}

/// The error formatter for the URL-validation moments: the wizard health probe,
/// `login`, and the periodic `whoami`. A 404 *here* means the address doesn't
/// resolve to a Hoard server (the `/v1/health` or auth route is missing), so we say
/// exactly that. Every other error shape defers to [`pretty_error`], which reads a
/// 404 as "the resource is gone": the right call for data fetches but wrong when the
/// user is still validating a URL.
pub(crate) fn probe_error(err: anyhow::Error) -> String {
    if matches!(err.downcast_ref::<ApiError>(), Some(ApiError::NotFound)) {
        return "The server is reachable but it doesn't look like a Hoard server. \
                Did you copy the URL correctly?"
            .into();
    }
    pretty_error(err)
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
