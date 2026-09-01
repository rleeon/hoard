//! Process-wide app state shared across Tauri commands.
//!
//! For phase 1 this is just the cached user info; later phases will add the
//! detection cache, the scheduler queue and so on.

use std::sync::Mutex;

use crate::commands::auth::{classify_cloud, classify_server, UserInfo};
use crate::commands::cloud::CloudAccount;
use crate::commands::library::{self, DetectionCache};
use crate::daemon::DaemonLink;

#[derive(Default)]
pub struct AppState {
    /// Cached identity from `whoami`. `None` means "not logged in" or "the
    /// session file was malformed/wiped".
    pub user: Mutex<Option<UserInfo>>,
    /// Cached `/v1/me` snapshot for Hoard Cloud. Independent of `user`: a user can
    /// be signed in to cloud, self-hosted, or neither.
    pub cloud_account: Mutex<Option<CloudAccount>>,
    /// Last successful auto-detection report. Lets the Library page render
    /// immediately on revisit without forcing another disk sweep.
    pub detection_cache: DetectionCache,
    /// The link to `hoardd`, the service that **owns** the sync engine (ADR 0021).
    /// It replaces the embedded `AgentHandle`, presence (the service's now) and the
    /// "one agent per machine" pidfile: the arbiter is ownership of the socket, and
    /// the sync's lifetime is no longer tied to this window.
    pub daemon: DaemonLink,
    /// Buffered `hoard://` deep-link URL captured before the frontend's
    /// listener was ready (cold start passes the OAuth callback as a launch
    /// argument; the webview registers its `deep-link://new-url` listener only
    /// after it mounts). The frontend drains this on mount via
    /// `cloud_take_pending_deep_link`. Cleared on a successful login.
    pub pending_deep_link: Mutex<Option<String>>,
    /// The in-progress cloud login handoff. Minted by `cloud_login_url` (which
    /// reuses a still-live attempt instead of clobbering it when the user clicks
    /// "Sign in" again) and validated by `cloud_complete_login`, which clears it
    /// only on success. `None` means "no login in progress", so a spontaneous deep
    /// link with attacker tokens can never match.
    pub pending_login: Mutex<Option<PendingLogin>>,
}

/// One in-progress cloud login attempt (see `cloud_login_url`).
pub struct PendingLogin {
    /// CSRF `state` nonce echoed through both handoff paths (loopback and the
    /// `hoard://` fallback) and re-checked by `cloud_complete_login`.
    pub nonce: String,
    /// Loopback port this attempt's listener bound, `None` when the bind
    /// failed and the attempt rides the `hoard://` scheme only.
    pub port: Option<u16>,
    /// When the attempt started. It expires after the loopback listener's
    /// window (`loopback::LISTEN_TIMEOUT`) so an abandoned nonce can't be
    /// completed hours later.
    pub started: std::time::Instant,
}

impl AppState {
    /// Build an `AppState` and try to populate the user cache from the
    /// on-disk session. Failures are logged but never fatal: the user just goes
    /// back through the onboarding wizard.
    pub fn from_disk() -> Self {
        // This process is a client of the service: nothing here touches the secret
        // store, not even the readers that live in the agent and run on both sides
        // (`logship`). See `credentials::mark_client`.
        hoard_agent::credentials::mark_client();
        // `load_public` and not `load`: the start needs the URL and the user (which
        // are not secrets and live in `session.toml`) and the service lends the token
        // when it is needed. Reading the keyring here is what asked the user for their
        // password on macOS, and this runs before the link to the service even exists
        // (D.20).
        let user = match hoard_agent::credentials::load_public() {
            Ok(Some((server_url, cached))) => {
                cached.map(|u| UserInfo {
                    user_id: u.user_id,
                    username: u.username,
                    is_admin: u.is_admin,
                    is_local_server: classify_server(&server_url),
                    is_cloud_server: classify_cloud(&server_url),
                    // Quota isn't cached on disk; the UI calls
                    // `refresh_quota` shortly after boot to fill it in. Same
                    // for the server's limits: `None` reads as "not asked yet",
                    // which is what the account page renders as a dash.
                    storage_used_bytes: 0,
                    storage_quota_bytes: 0,
                    max_snapshot_size_bytes: None,
                    max_versions: None,
                    max_manual_versions: None,
                    server_url,
                })
            }
            Ok(None) => None,
            Err(e) => {
                // `{:#}` so the anyhow cause chain (e.g. the underlying OS
                // error) shows up instead of just the top-level context.
                tracing::warn!(error = %format!("{e:#}"), "couldn't load saved credentials; starting fresh");
                None
            }
        };
        let detection_cache = DetectionCache::default();
        if let Some(cached) = library::load_detection_from_disk() {
            *detection_cache.last.lock().unwrap() = Some(cached);
        }
        let state = Self {
            user: Mutex::new(user),
            cloud_account: Mutex::new(None),
            detection_cache,
            daemon: DaemonLink::default(),
            pending_deep_link: Mutex::new(None),
            pending_login: Mutex::new(None),
        };
        crate::commands::cloud::rehydrate(&state);
        // Install the active sync context from the restored session BEFORE any
        // command loads `CliState`, so per-context saves resolve to the right
        // file (and the legacy monolithic `state.json` migrates into it).
        crate::commands::library::sync_active_context(&state);
        state
    }
}
