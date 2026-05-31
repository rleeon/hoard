//! Process-wide app state shared across Tauri commands.
//!
//! For phase 1 this is just the cached user info; later phases will add the
//! detection cache, the scheduler queue and so on.

use std::sync::Mutex;

use hoard_agent::agent::AgentHandle;

use crate::commands::auth::{classify_cloud, classify_server, UserInfo};
use crate::commands::cloud::CloudAccount;
use crate::commands::library::{self, DetectionCache};

#[derive(Default)]
pub struct AppState {
    /// Cached identity from `whoami`. `None` means "not logged in" or "the
    /// session file was malformed/wiped".
    pub user: Mutex<Option<UserInfo>>,
    /// Cached `/v1/me` snapshot for Hoard Cloud. Independent of `user` —
    /// a user can be signed in to cloud, self-hosted, or neither.
    pub cloud_account: Mutex<Option<CloudAccount>>,
    /// Last successful auto-detection report. Lets the Library page render
    /// immediately on revisit without forcing another disk sweep.
    pub detection_cache: DetectionCache,
    /// Live agent handle. Populated lazily by the agent bootstrapper; tests
    /// and the logged-out state both leave it `None`.
    pub agent: Mutex<Option<AgentHandle>>,
    /// Buffered `hoard://` deep-link URL captured before the frontend's
    /// listener was ready (cold start passes the OAuth callback as a launch
    /// argument; the webview registers its `deep-link://new-url` listener only
    /// after it mounts). The frontend drains this on mount via
    /// `cloud_take_pending_deep_link`. Cleared on a successful login.
    pub pending_deep_link: Mutex<Option<String>>,
}

impl AppState {
    /// Build an `AppState` and try to populate the user cache from the
    /// on-disk session. Failures are logged but never fatal — the user just
    /// goes back through the onboarding wizard.
    pub fn from_disk() -> Self {
        let user = match hoard_agent::credentials::load() {
            Ok(Some(creds)) => {
                let server_url = creds.url.clone();
                creds.user.map(|u| UserInfo {
                    user_id: u.user_id,
                    username: u.username,
                    is_admin: u.is_admin,
                    is_local_server: classify_server(&server_url),
                    is_cloud_server: classify_cloud(&server_url),
                    // Quota isn't cached on disk — the UI calls
                    // `refresh_quota` shortly after boot to fill it in.
                    storage_used_bytes: 0,
                    storage_quota_bytes: 0,
                    server_url,
                })
            }
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, "couldn't load saved credentials; starting fresh");
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
            agent: Mutex::new(None),
            pending_deep_link: Mutex::new(None),
        };
        crate::commands::cloud::rehydrate(&state);
        state
    }
}
