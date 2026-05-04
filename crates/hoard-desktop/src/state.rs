//! Process-wide app state shared across Tauri commands.
//!
//! For phase 1 this is just the cached user info; later phases will add the
//! detection cache, the scheduler queue and so on.

use std::sync::Mutex;

use crate::commands::auth::UserInfo;

#[derive(Default)]
pub struct AppState {
    /// Cached identity from `whoami`. `None` means "not logged in" or "the
    /// session file was malformed/wiped".
    pub user: Mutex<Option<UserInfo>>,
}

impl AppState {
    /// Build an `AppState` and try to populate the user cache from the
    /// on-disk session. Failures are logged but never fatal — the user just
    /// goes back through the onboarding wizard.
    pub fn from_disk() -> Self {
        let user = match hoard_agent::credentials::load() {
            Ok(Some(creds)) => creds.user.map(|u| UserInfo {
                user_id: u.user_id,
                username: u.username,
                is_admin: u.is_admin,
                server_url: creds.url,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, "couldn't load saved credentials; starting fresh");
                None
            }
        };
        Self {
            user: Mutex::new(user),
        }
    }
}
