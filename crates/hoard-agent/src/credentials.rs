//! Persistent storage for the desktop client's session.
//!
//! Two pieces are kept on disk:
//!
//! * The bearer token, which is sensitive and should live in the OS keychain
//!   when one is available (Secret Service on Linux, Credential Manager on
//!   Windows, Keychain on macOS — all surfaced by the `keyring` crate).
//! * The server URL and a cached copy of the last-seen user info, which are
//!   not sensitive and live in a TOML file at `<config>/desktop/session.toml`
//!   so we can show the username without hitting the network on startup.
//!
//! When the OS keychain is unavailable (e.g. headless Linux without
//! libsecret) the token falls back into the same TOML file, which is created
//! with `0600` permissions on Unix.
//!
//! The desktop app uses a separate file from `hoard-cli`'s `config.toml` so
//! that running the CLI does not stomp the GUI's session and vice versa.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::api::Whoami;
use crate::config::CliConfig;

const KEYRING_SERVICE: &str = "hoard-desktop";
const KEYRING_USER: &str = "default";

/// In-memory view of the desktop client's saved session.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub url: String,
    pub token: String,
    pub user: Option<UserSection>,
}

/// Where the token actually ended up after `save`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStorage {
    /// Stored via the OS secret service (preferred).
    Keyring,
    /// Stored in the TOML file at 0600 because the keyring was unavailable.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Session {
    #[serde(default)]
    server: ServerSection,
    #[serde(default)]
    user: Option<UserSection>,
    /// Filesystem fallback when the OS keyring is unavailable. In normal
    /// operation this is `None` and the token lives in the keyring.
    #[serde(default)]
    auth: Option<AuthSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ServerSection {
    #[serde(default)]
    url: String,
}

/// Subset of `/v1/auth/whoami` we cache locally so the dashboard can show the
/// username without an extra round-trip on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSection {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthSection {
    token: String,
}

impl From<Whoami> for UserSection {
    fn from(w: Whoami) -> Self {
        Self {
            user_id: w.user_id,
            username: w.username,
            is_admin: w.is_admin,
        }
    }
}

/// Resolve the on-disk path of the session metadata file.
pub fn session_path() -> Result<PathBuf> {
    let dirs = CliConfig::project_dirs()?;
    Ok(dirs.config_dir().join("desktop").join("session.toml"))
}

/// Persist credentials. Token goes to the OS keychain when available, with a
/// transparent file fallback otherwise.
pub fn save(creds: &Credentials) -> Result<TokenStorage> {
    let session = Session {
        server: ServerSection {
            url: creds.url.clone(),
        },
        user: creds.user.clone(),
        auth: None,
    };
    write_session(&session)?;

    match try_keyring_set(&creds.token) {
        Ok(()) => {
            // Belt and braces: if the file had a stale token from a previous
            // fallback run, scrub it now that the keyring took over.
            scrub_file_token().ok();
            Ok(TokenStorage::Keyring)
        }
        Err(_) => {
            let mut session = read_session()?.unwrap_or_default();
            session.auth = Some(AuthSection {
                token: creds.token.clone(),
            });
            write_session(&session)?;
            Ok(TokenStorage::File)
        }
    }
}

/// Load credentials if any are stored. Returns `Ok(None)` when no session is
/// present yet (e.g. fresh install) — that is not an error.
pub fn load() -> Result<Option<Credentials>> {
    let Some(session) = read_session()? else {
        return Ok(None);
    };
    if session.server.url.is_empty() {
        return Ok(None);
    }

    // Prefer the keyring; if it has nothing, fall back to the file.
    let token = match try_keyring_get() {
        Ok(Some(t)) => Some(t),
        _ => session.auth.as_ref().map(|a| a.token.clone()),
    };

    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return Ok(None);
    };

    Ok(Some(Credentials {
        url: session.server.url,
        token,
        user: session.user,
    }))
}

/// Wipe stored credentials. Idempotent — clearing twice is fine.
pub fn clear() -> Result<()> {
    // Best-effort: errors here mean the entry didn't exist, which is fine.
    let _ = try_keyring_delete();
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

/// Cheap shape check on a token string — `hoard_v1_` followed by 64 lowercase
/// hex characters. Avoids round-tripping obviously-wrong input through the
/// network.
pub fn is_valid_token(token: &str) -> bool {
    const PREFIX: &str = "hoard_v1_";
    if token.len() != PREFIX.len() + 64 {
        return false;
    }
    if !token.starts_with(PREFIX) {
        return false;
    }
    token[PREFIX.len()..]
        .chars()
        .all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

// ---- internals ---------------------------------------------------------

fn read_session() -> Result<Option<Session>> {
    let path = session_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let s: Session =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(s))
}

fn write_session(s: &Session) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(s).context("serializing session")?;
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

fn scrub_file_token() -> Result<()> {
    let Some(mut session) = read_session()? else {
        return Ok(());
    };
    if session.auth.is_some() {
        session.auth = None;
        write_session(&session)?;
    }
    Ok(())
}

fn try_keyring_set(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.set_password(token)?;
    Ok(())
}

fn try_keyring_get() -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn try_keyring_delete() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_validation_accepts_canonical() {
        let good = format!("hoard_v1_{}", "a".repeat(64));
        assert!(is_valid_token(&good));
    }

    #[test]
    fn token_validation_rejects_wrong_prefix() {
        let bad = format!("hoard_v2_{}", "a".repeat(64));
        assert!(!is_valid_token(&bad));
    }

    #[test]
    fn token_validation_rejects_short() {
        assert!(!is_valid_token("hoard_v1_abcd"));
    }

    #[test]
    fn token_validation_rejects_uppercase_hex() {
        let bad = format!("hoard_v1_{}", "A".repeat(64));
        assert!(!is_valid_token(&bad));
    }

    #[test]
    fn token_validation_rejects_non_hex() {
        let bad = format!("hoard_v1_{}", "z".repeat(64));
        assert!(!is_valid_token(&bad));
    }
}
