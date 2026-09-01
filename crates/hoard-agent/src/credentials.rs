//! Persistent storage for the desktop client's session.
//!
//! Two pieces are kept on disk:
//!
//! * The bearer token, which is sensitive and should live in the OS keychain
//!   when one is available (Secret Service on Linux, Credential Manager on
//!   Windows, Keychain on macOS, all surfaced by the `keyring` crate).
//! * The server URL and a cached copy of the last-seen user info, which are
//!   not sensitive and live in a TOML file at `<config>/desktop/session.toml`
//!   so we can show the username without hitting the network on startup. These
//!   are also mirrored into the keychain blob, so a lost or unreadable cache
//!   file no longer signs the user out.
//!
//! When the OS keychain is unavailable (e.g. headless Linux without
//! libsecret) the token falls back into the same TOML file, which is created
//! with `0600` permissions on Unix.
//!
//! The desktop app uses a separate file from `hoard-cli`'s `config.toml` so
//! that running the CLI does not stomp the GUI's session and vice versa.
//!
//! ## Who writes, who reads (D.20)
//!
//! The daemon is the owner, as in `cloud_auth`: [`save`] and [`clear`] touch the
//! keyring, and on macOS a keychain item authorises only the binary that created
//! it, so with the app writing and `hoardd` reading, every read by the service was
//! a password dialog. A client that has just validated a token hands it over
//! (`Request::AdoptServerSession`) and borrows it when it needs it
//! (`Request::ServerToken`).
//!
//! A client therefore does not call [`load`]: it uses [`current`], which returns
//! the loan somebody put in the slot ([`set_lent`]) and only falls back to the
//! store when nobody has filled it, which is the daemon's case, the owner's. For
//! what is not secret (the URL and the user) there is [`load_public`], which reads
//! the file and never touches the keyring.
//!
//! With no service to hand it to there are [`save_unlocked`] and
//! [`forget_unlocked`]: the 0600 file and never the keyring.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::api::Whoami;
use crate::config::CliConfig;
use crate::keychain::{keyring_op, KeyringTimeout, KeyringUnreadable, KEYRING_TIMEOUT};

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
    /// The user signed out and nobody could delete the keychain item (a client
    /// with no service in reach: deleting is the owner's job).
    ///
    /// It is load-bearing that this exists. [`load`] recovers the session from the
    /// keyring blob when the file has been lost, which is the fix for the ACL an
    /// old Windows build used to clamp down, and without this marker that would
    /// resurrect the session the user just closed. A deleted file and an unreadable
    /// one look too alike to tell apart by their absence, so the logout leaves word
    /// of what it did. [`save`] writes a fresh `Session`, so the next login clears
    /// it without having to remember it.
    #[serde(default, skip_serializing_if = "is_false")]
    signed_out: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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

/// What we stash in the OS keychain. Historically this was the bare token
/// string; it's now a small TOML document so the keychain alone can restore a
/// session (token + server URL + cached user) even when the on-disk cache is
/// missing or unreadable. Reads tolerate the legacy bare-token form.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct KeyringBlob {
    #[serde(default)]
    token: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    user: Option<UserSection>,
}

impl From<Whoami> for UserSection {
    fn from(w: Whoami) -> Self {
        Self {
            user_id: w.user_id,
            username: w.username.into_inner(),
            is_admin: w.is_admin,
        }
    }
}

/// Resolve the on-disk path of the session metadata file.
pub fn session_path() -> Result<PathBuf> {
    let dirs = CliConfig::project_dirs()?;
    Ok(dirs.config_dir().join("desktop").join("session.toml"))
}

/// Persist credentials. The token goes to the OS keychain when available, with a
/// transparent file fallback otherwise.
///
/// Only the daemon. It is the write that creates the keychain item, and on macOS
/// its ACL authorises only the binary that creates it: if a client wrote it, every
/// read by the service would ask the user for their password (D.20). A client
/// hands the session over by IPC (`Request::AdoptServerSession`), or uses
/// [`save_unlocked`] when there is no service to hand it to.
pub fn save(creds: &Credentials) -> Result<TokenStorage> {
    let session = Session {
        server: ServerSection {
            url: creds.url.clone(),
        },
        user: creds.user.clone(),
        auth: None,
        signed_out: false,
    };
    write_session(&session)?;

    match store_in_keyring(creds) {
        Ok(()) => {
            // Belt and braces: if the file had a stale token from a previous
            // fallback run, scrub it now that the keyring took over.
            scrub_file_token().ok();
            Ok(TokenStorage::Keyring)
        }
        Err(err) => {
            tracing::warn!(
                error = %format!("{err:#}"),
                "keyring: keeping the self-hosted session in the protected file instead"
            );
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
/// present yet (a fresh install, say), and that is not an error. A locked keyring
/// is one, though: see [`pick_token`].
pub fn load() -> Result<Option<Credentials>> {
    Ok(load_detailed()?.map(|(creds, _)| creds))
}

/// Like [`load`], but saying where the token came from.
///
/// The daemon needs it: a token that came from the file means the keychain item
/// either does not exist or is not its own, and then it has to lift it itself
/// ([`promote_to_keyring`]) to own the ACL. Without that distinction the promotion
/// would have to rewrite the keyring on every start, a pointless write per start
/// and another dialog on macOS, or never happen at all, which is what leaves a
/// macOS user with an item their service cannot read.
pub fn load_detailed() -> Result<Option<(Credentials, TokenStorage)>> {
    // A logout that could not delete the keychain item (a client with no service)
    // leaves word here. It is checked before anything else: the recovery below
    // would resurrect the session from the orphaned blob.
    if matches!(read_session(), Ok(Some(s)) if s.signed_out) {
        return Ok(None);
    }
    match read_session() {
        // Normal path: the on-disk cache is readable and has a server URL. The
        // token comes from the keychain, falling back to the file copy.
        Ok(Some(session)) if !session.server.url.is_empty() => {
            let from_file = session.auth.as_ref().map(|a| a.token.clone());
            match pick_token(try_keyring_get(), from_file)? {
                Some((token, storage)) => Ok(Some((
                    Credentials {
                        url: session.server.url,
                        token,
                        user: session.user,
                    },
                    storage,
                ))),
                None => Ok(None),
            }
        }
        // Cache absent, empty, or unreadable (an ACL a previous Windows build
        // clamped down and `read_session` couldn't repair, say). Don't drop the
        // session over a disk hiccup: the keychain now carries the URL too, so it
        // can restore everything on its own.
        //
        // Here a keyring failure IS swallowed, unlike above: with no session file
        // nobody has ever signed in on this machine (`save` always writes it, even
        // when the keyring is missing), so its absence is the answer and `Ok(None)`
        // sends the user to the wizard, which is what they want on a first run,
        // locked keyring or not. An unreadable file still returns its read error.
        read => {
            if let Ok(Some(blob)) = try_keyring_get() {
                if !blob.token.is_empty() && !blob.url.is_empty() {
                    let creds = Credentials {
                        url: blob.url,
                        token: blob.token,
                        user: blob.user,
                    };
                    // Best-effort: rewrite the cache so it's healthy again, with
                    // sane inherited permissions.
                    let _ = write_session(&Session {
                        server: ServerSection {
                            url: creds.url.clone(),
                        },
                        user: creds.user.clone(),
                        auth: None,
                        signed_out: false,
                    });
                    return Ok(Some((creds, TokenStorage::Keyring)));
                }
            }
            // Nothing recoverable from the keychain. Surface a real read error;
            // treat "absent/empty" as simply not-logged-in.
            match read {
                Err(e) => Err(e),
                _ => Ok(None),
            }
        }
    }
}

/// Which token counts: the keyring's when it answers, the 0600 file's when the
/// keyring fails for something repairable (locked, no D-Bus in a headless
/// session). That is not "there is no session".
///
/// It is `cloud_auth::pick_auth`'s twin, for the same reason: swallowing the `Err`
/// as though it were `NoEntry` returned `Ok(None)` with the token intact in the
/// keyring, meaning a user who appears signed out with not a line to explain it.
/// With the session file in front of us (there is a URL, so somebody did sign in
/// here once) a mute keyring with no disk fallback has to come out whole: it is
/// the only clue that it is locked.
fn pick_token(
    from_keyring: Result<Option<KeyringBlob>>,
    from_file: Option<String>,
) -> Result<Option<(String, TokenStorage)>> {
    let from_file = from_file.filter(|t| !t.is_empty());
    match from_keyring {
        Ok(Some(blob)) if !blob.token.is_empty() => Ok(Some((blob.token, TokenStorage::Keyring))),
        // No entry, or an empty one: there is no failure to report, so it falls
        // back to the file.
        Ok(_) => Ok(from_file.map(|t| (t, TokenStorage::File))),
        Err(e) => match from_file {
            Some(token) => {
                tracing::debug!(error = %e, "keyring unreadable; using the token from the file");
                Ok(Some((token, TokenStorage::File)))
            }
            // An exhausted cap explains itself; any other failure gets the typed
            // reason attached, which is what the UI reads to say "sign in again"
            // instead of the generic banner.
            None if e.is::<KeyringTimeout>() => Err(e),
            None => Err(e.context(KeyringUnreadable {
                doing: "reading the self-hosted session",
            })),
        },
    }
}

/// Lifts a session that was only in the file into the keyring, as its owner.
///
/// The daemon calls it after starting with a token that came from the 0600 file:
/// the one a client with no service left there ([`save_unlocked`]), or the one
/// left behind when the keyring was locked. From the next read on the item is its
/// own, which is the only thing that avoids a password dialog per start on macOS
/// (the ACL authorises the binary that creates the item, D.20).
///
/// Genuinely best-effort: it returns `false` and touches nothing else when the
/// keyring is not there. A service that refused to sync because it could not store
/// the token where it prefers would be far worse than one that carries on reading
/// from the file.
///
/// It does not delete the file's copy, unlike [`save`]. Here the keyring has just
/// proved it was either unreadable or absent, so removing the one backup that
/// works is exactly the move that leaves the user with no sync the next time it
/// locks. The file is 0600 and already held that token.
pub fn promote_to_keyring(creds: &Credentials) -> bool {
    match store_in_keyring(creds) {
        Ok(()) => {
            tracing::info!(
                "credentials: the self-hosted session moves into the keyring in the service's name"
            );
            true
        }
        Err(err) => {
            tracing::debug!(error = %format!("{err:#}"), "credentials: the keyring will not take the session; it stays in the file");
            false
        }
    }
}

/// Wipe stored credentials. Idempotent: clearing twice is fine.
///
/// The daemon's, for the same reason as [`save`]: deleting a keychain item is also
/// authorised. A client sends `Request::ForgetServerSession` and, with no service,
/// [`forget_unlocked`].
pub fn clear() -> Result<()> {
    // Best-effort: errors here mean the entry didn't exist, which is fine.
    let _ = try_keyring_delete();
    let path = session_path()?;
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

/// Persists the session without touching the keyring: the 0600 file and nothing
/// else.
///
/// The path for a client that has just validated a token and has no service to
/// hand it to. Writing the keyring here would be D.20's bug, with the item ending
/// up in the client's name and the service asking permission on every read,
/// whereas leaving it in the file lets the daemon read it as-is on start and lift
/// it into the keyring itself, as the owner.
pub fn save_unlocked(creds: &Credentials) -> Result<()> {
    write_session(&Session {
        server: ServerSection {
            url: creds.url.clone(),
        },
        user: creds.user.clone(),
        auth: Some(AuthSection {
            token: creds.token.clone(),
        }),
        signed_out: false,
    })
}

/// Signs out without touching the keyring: it leaves the tombstone (`signed_out`)
/// in the file.
///
/// Deleting the file is not enough, and that is the difference from Cloud: [`load`]
/// recovers the session from the keyring blob when the file is missing, so deleting
/// it would resurrect the session. The marker says "this is not a lost file, it is
/// a logout", and the orphaned item left in the keyring authorises nothing on its
/// own: the next login overwrites it, and until then nobody reads it.
pub fn forget_unlocked() -> Result<()> {
    write_session(&Session {
        server: ServerSection::default(),
        user: None,
        auth: None,
        signed_out: true,
    })
}

/// What is not secret about the session: which server, and who. It comes from the
/// file, without touching the keyring.
///
/// It is what a client can read on its own, and enough to get going: the desktop
/// draws the user and the URL on open (synchronously, before the link to the
/// service exists) and borrows the token when it actually goes to call the server.
pub fn load_public() -> Result<Option<(String, Option<UserSection>)>> {
    match read_session()? {
        Some(s) if s.signed_out => Ok(None),
        Some(s) if !s.server.url.is_empty() => Ok(Some((s.server.url, s.user))),
        _ => Ok(None),
    }
}

/// The loan slot: the session the service has lent us.
///
/// It exists because there is one reader that cannot ask over IPC: the log shipper
/// (`logship`), which runs on its own thread with its own runtime and re-reads the
/// session every few seconds. In the daemon the slot is empty and it reads the
/// store, which is its own; in a client whoever borrows fills it, and that way
/// nobody touches somebody else's keyring.
static LENT: std::sync::RwLock<Option<Credentials>> = std::sync::RwLock::new(None);

/// Stores, or clears with `None`, the borrowed session. The client calls it as
/// soon as the service lends it one, and with `None` on sign-out.
pub fn set_lent(creds: Option<Credentials>) {
    let mut slot = LENT.write().unwrap_or_else(|p| p.into_inner());
    *slot = creds;
}

/// The Cloud twin of the slot above, and it exists for the same reader: `logship`.
///
/// The Cloud session does not live here; it lives in `cloud_auth` and `cloud.toml`,
/// and its JWT is rotated by the service, so a reader looking only at [`current`]
/// never sees it. That was the bug: with the app on Cloud, the log shipper resolved
/// `None` on every pass and has not sent a single line since it existed.
///
/// Whoever holds a fresh token fills it: the service on every rotation
/// ([`crate::session::refresh_loop`]) and a client as soon as it borrows one. With
/// `None` on sign-out.
static LENT_CLOUD: std::sync::RwLock<Option<CloudLease>> = std::sync::RwLock::new(None);

/// Which Cloud and which JWT, for the reader that cannot ask over IPC.
#[derive(Debug, Clone)]
pub struct CloudLease {
    pub url: String,
    pub token: String,
}

/// Guarda (o borra, con `None`) el token Cloud prestado.
pub fn set_lent_cloud(lease: Option<CloudLease>) {
    let mut slot = LENT_CLOUD.write().unwrap_or_else(|p| p.into_inner());
    *slot = lease;
}

/// The lent Cloud token, when a Cloud session is alive in this process.
pub fn lent_cloud() -> Option<CloudLease> {
    LENT_CLOUD.read().unwrap_or_else(|p| p.into_inner()).clone()
}

/// This process is a client: it never touches the store, only the loan.
static CLIENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Declares this process a client of the service, so [`current`] never falls back
/// to the store.
///
/// The desktop calls it on start. Without this, a reader that runs in both
/// processes, `logship`, would read the keyring in the client during the window
/// where the loan is not in place yet, and on macOS that read IS the password
/// dialog D.20 exists to kill. With the marker, a client with no loan simply has no
/// session, and ships no logs, rather than asking permission: losing a batch of
/// optional diagnostics is infinitely better than a dialog.
///
/// `hoardd` never calls it: it is the owner.
pub fn mark_client() {
    CLIENT.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// The borrowed session, without falling back to the store. Used by a client that
/// cannot touch the keyring: with an empty slot, the right move is to borrow.
pub fn lent() -> Option<Credentials> {
    LENT.read().unwrap_or_else(|p| p.into_inner()).clone()
}

/// The session this process may use: the borrowed one when there is one, the store
/// otherwise.
///
/// For readers that run in both processes and cannot ask for anything over IPC
/// (`logship`): in a client the slot is filled, and in the daemon it is empty and
/// the store is its own. A client arriving here with an empty slot would read
/// somebody else's keyring, so whoever can wait should use the loan.
pub fn current() -> Result<Option<Credentials>> {
    if let Some(creds) = lent() {
        return Ok(Some(creds));
    }
    if CLIENT.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(None);
    }
    load()
}

/// A cheap shape check on a token string: `hoard_v1_` followed by 64 lowercase hex
/// characters. Avoids round-tripping obviously wrong input through the network.
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
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        // A session file written by an older build can carry a broken ACL
        // (icacls /inheritance:r granted to a principal that doesn't resolve to
        // this process's identity) → the file exists but reads back "access
        // denied". The owner can always rewrite the DACL, so reset inherited
        // permissions and retry once before giving up.
        #[cfg(windows)]
        Err(_) if reset_acl_windows(&path) => std::fs::read_to_string(&path)
            .with_context(|| format!("reading {} after ACL reset", path.display()))?,
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
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

    // Atomic write: a plain truncate+write leaves the session file half-written
    // if the process dies mid-write, and a truncated TOML fails to parse on next
    // launch → spurious sign-out. Write to a sibling temp file then rename over the
    // target (atomic on the same filesystem), so a reader only ever sees the old or
    // the new file. Solves Windows issues with inherited ACLs on partially-written
    // files and sync-folder interference (OneDrive, Dropbox).
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, &text).with_context(|| format!("writing {}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&tmp, perms)?;
    }

    std::fs::rename(&tmp, &path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;

    Ok(())
}

/// Repair a session file a previous build's ACL-hardening left unreadable.
///
/// Older versions ran `icacls /inheritance:r /grant:r %USERNAME%:F` on the file.
/// When `%USERNAME%` didn't resolve to the process's actual identity (Microsoft
/// accounts, a same-named local account, roaming or redirected profiles) the file
/// ended up owned by the user but granting access to the wrong principal, so a
/// later launch reads it back as "access denied". The owner keeps the implicit
/// right to rewrite the DACL, so `icacls /reset` restores the inherited, per-user
/// permissions and the retry read then succeeds. Best-effort: it returns whether
/// the reset ran cleanly so the caller only retries the read when it is worth it.
#[cfg(windows)]
fn reset_acl_windows(path: &std::path::Path) -> bool {
    match std::process::Command::new("icacls")
        .arg(path)
        .arg("/reset")
        .output()
    {
        Ok(out) if out.status.success() => {
            tracing::info!(path = %path.display(), "credentials: reset stale ACL on session file");
            true
        }
        Ok(out) => {
            tracing::warn!(
                status = ?out.status.code(),
                stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                "credentials: icacls /reset did not repair the session file",
            );
            false
        }
        Err(e) => {
            tracing::warn!(error = %e, "credentials: failed to run icacls /reset");
            false
        }
    }
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

// All three operations go through `keychain::keyring_op`: its own thread, a
// [`KEYRING_TIMEOUT`] cap, and [`KeyringTimeout`] as the reason when it runs out. A
// locked keyring does not fail, it waits for an unlock nobody in a desktop-less
// session is going to answer, and an uncapped synchronous call hangs whoever made
// it (ADR 0021 D.19, the same thing that already happened with the Cloud session).

fn try_keyring_set(creds: &Credentials) -> Result<()> {
    // Store the whole session (token, URL and cached user) as TOML so the keychain
    // can restore it without the on-disk cache. See `KeyringBlob`. It is serialised
    // here, off the keyring thread: the operation has to be `'static` and `creds`
    // is borrowed.
    let blob = toml::to_string(&KeyringBlob {
        token: creds.token.clone(),
        url: creds.url.clone(),
        user: creds.user.clone(),
    })
    .context("serializing keychain blob")?;
    keyring_op(
        "saving the self-hosted session",
        KEYRING_TIMEOUT,
        move || {
            let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
            entry.set_password(&blob)?;
            Ok(())
        },
    )
}

/// Write the token to the keyring **and read it back**. The twin of
/// `cloud_auth::store_in_keyring`, and for the same reason: a `set_password`
/// that returns `Ok` isn't proof the entry can be read again, and this caller
/// scrubs the file copy on success. Trust the write and a keyring that accepts
/// what it can't decrypt leaves the machine with no token anywhere.
fn store_in_keyring(creds: &Credentials) -> Result<()> {
    try_keyring_set(creds)?;
    match try_keyring_get() {
        Ok(Some(blob)) if blob.token == creds.token => Ok(()),
        Ok(_) => anyhow::bail!("the keyring accepted the session and didn't give it back"),
        Err(err) => Err(err.context("reading back the session we just saved")),
    }
}

fn try_keyring_get() -> Result<Option<KeyringBlob>> {
    keyring_op("reading the self-hosted session", KEYRING_TIMEOUT, || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        match entry.get_password() {
            Ok(raw) => Ok(Some(parse_keyring_blob(&raw))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
}

/// Parse a keychain payload, tolerating the legacy format where the entry was
/// just the bare token string (no TOML wrapper).
fn parse_keyring_blob(raw: &str) -> KeyringBlob {
    match toml::from_str::<KeyringBlob>(raw) {
        Ok(blob) if !blob.token.is_empty() => blob,
        _ => KeyringBlob {
            token: raw.trim().to_string(),
            url: String::new(),
            user: None,
        },
    }
}

fn try_keyring_delete() -> Result<()> {
    keyring_op("deleting the self-hosted session", KEYRING_TIMEOUT, || {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    })
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

    // ---- the locked keyring, self-hosted path (D.19)

    fn stuck() -> anyhow::Error {
        anyhow::Error::new(KeyringTimeout {
            doing: "reading the self-hosted session",
            after: KEYRING_TIMEOUT,
        })
    }

    fn in_the_keyring(token: &str) -> Result<Option<KeyringBlob>> {
        Ok(Some(KeyringBlob {
            token: token.to_string(),
            url: "https://saves.example".to_string(),
            user: None,
        }))
    }

    /// The D.19 failure on the self-hosted route: with the token only in the
    /// keyring, a locked one returned `Ok(None)` and the user showed up as logged
    /// out, indistinguishable from a fresh install and with nothing to look at.
    /// Now the reason comes out whole and typed.
    #[test]
    fn a_locked_keyring_is_not_a_logged_out_user() {
        let err = pick_token(Err(stuck()), None).expect_err("cannot be Ok(None)");
        assert!(err.is::<KeyringTimeout>(), "{err:#}");
        assert!(format!("{err:#}").contains("locked"), "{err:#}");
    }

    /// But with a token in the file (the 0600 fallback for when there is no
    /// keyring) a locked keyring logs nobody out: it carries on with what is there.
    #[test]
    fn a_locked_keyring_still_falls_back_to_the_file_token() {
        let got = pick_token(Err(stuck()), Some("hoard_v1_del-fichero".to_string()))
            .expect("the file saves the session")
            .expect("token");
        assert_eq!(
            got,
            ("hoard_v1_del-fichero".to_string(), TokenStorage::File)
        );
    }

    /// The origin travels with the token, and it is not cosmetic: it is what tells
    /// the daemon that the keyring item is not its own (or is missing) and that it
    /// has to push it up with [`promote_to_keyring`]. Without it the promotion
    /// would be one write per start, and on macOS one dialog per start.
    #[test]
    fn the_token_says_where_it_came_from() {
        let (_, storage) = pick_token(in_the_keyring("hoard_v1_x"), None)
            .expect("ok")
            .expect("token");
        assert_eq!(storage, TokenStorage::Keyring);

        let (_, storage) = pick_token(Ok(None), Some("hoard_v1_x".to_string()))
            .expect("ok")
            .expect("token");
        assert_eq!(storage, TokenStorage::File);
    }

    /// A keyring that answers "no" (the macOS ACL authorising another binary, a
    /// session with no D-Bus) comes out with **its own** typed reason, distinct
    /// from the timeout. The UI paints them the same ("sign in again"), but the log
    /// has to be able to tell a locked keyring from one that refuses.
    #[test]
    fn a_refusing_keyring_is_typed_as_unreadable() {
        let err = pick_token(Err(anyhow::anyhow!("access denied")), None)
            .expect_err("with no file to save it, it comes out whole");
        assert!(err.downcast_ref::<KeyringUnreadable>().is_some(), "{err:#}");
        assert!(err.downcast_ref::<KeyringTimeout>().is_none());
    }

    /// A keyring failure that is not the timeout (no D-Bus, corrupt entry) arrives
    /// just as whole, with the context of where it happened.
    #[test]
    fn another_keyring_failure_also_surfaces() {
        let err = pick_token(Err(anyhow::anyhow!("no D-Bus session bus")), None)
            .expect_err("the keyring failure propagates");
        assert!(err.downcast_ref::<KeyringTimeout>().is_none());
        assert!(
            format!("{err:#}").contains("no D-Bus session bus"),
            "{err:#}"
        );
    }

    /// And a healthy keyring beats the file; with no entry (or an empty one, which
    /// is how a half-deleted session leaves them) it falls back to the file.
    #[test]
    fn a_healthy_keyring_wins_and_an_empty_one_falls_back() {
        let (got, _) = pick_token(
            in_the_keyring("hoard_v1_del-llavero"),
            Some("hoard_v1_del-fichero".to_string()),
        )
        .expect("ok")
        .expect("token");
        assert_eq!(got, "hoard_v1_del-llavero");

        let from_file = Some("hoard_v1_del-fichero".to_string());
        assert_eq!(
            pick_token(Ok(None), from_file.clone())
                .expect("ok")
                .map(|(t, _)| t)
                .as_deref(),
            Some("hoard_v1_del-fichero")
        );
        assert_eq!(
            pick_token(in_the_keyring(""), from_file)
                .expect("ok")
                .map(|(t, _)| t)
                .as_deref(),
            Some("hoard_v1_del-fichero")
        );
        assert!(pick_token(Ok(None), None).expect("ok").is_none());
        // A file with an empty token is the same as having none.
        assert!(pick_token(Ok(None), Some(String::new()))
            .expect("ok")
            .is_none());
    }

    /// Isolates the config directory. Linux only, because that is where
    /// `ProjectDirs` looks at `XDG_CONFIG_HOME`: on macOS and Windows the path comes
    /// from the system and the test would write into the real session of whoever
    /// runs it.
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

    /// The service-less path from D.20: it is saved to the 0600 file, with no
    /// keyring, and read back whole, including what a client can read on its own
    /// ([`load_public`]).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_session_stored_without_a_service_lands_in_the_file() {
        with_isolated_config(|| {
            let creds = Credentials {
                url: "https://hoard.example".to_string(),
                token: format!("hoard_v1_{}", "a".repeat(64)),
                user: Some(UserSection {
                    user_id: "u1".to_string(),
                    username: "rai".to_string(),
                    is_admin: true,
                }),
            };
            save_unlocked(&creds).expect("writes");

            let session = read_session().expect("reads").expect("there is a file");
            assert_eq!(session.server.url, "https://hoard.example");
            assert_eq!(
                session.auth.expect("the token is in the file").token,
                creds.token
            );

            let (url, user) = load_public().expect("reads").expect("there is a session");
            assert_eq!(url, "https://hoard.example");
            assert_eq!(user.expect("user").username, "rai");

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(session_path().unwrap())
                    .unwrap()
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o600, "modo {:o}", mode & 0o777);
            }
        });
    }

    /// The logout tombstone for the service-less path, which is the difference from
    /// Cloud: here **deleting the file is not enough**, because [`load`] recovers the
    /// session from the keyring blob when the file is missing (the Windows ACL fix)
    /// and would resurrect exactly what the user just closed. With the marker in
    /// place, `load` answers "no session" **without ever looking at the keyring**,
    /// which is what makes this test deterministic even on a machine that has the
    /// real item.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_logout_without_a_service_cannot_be_resurrected_from_the_keyring() {
        with_isolated_config(|| {
            let creds = Credentials {
                url: "https://hoard.example".to_string(),
                token: format!("hoard_v1_{}", "b".repeat(64)),
                user: None,
            };
            save_unlocked(&creds).expect("writes");
            assert!(load_public().expect("reads").is_some());

            forget_unlocked().expect("forgets");
            assert!(
                load_public().expect("reads").is_none(),
                "there is still a session"
            );
            assert!(
                load().expect("reads").is_none(),
                "the tombstone was not honoured"
            );
            // And the next login clears it without remembering it.
            save_unlocked(&creds).expect("signs back in");
            assert!(load_public().expect("reads").is_some());
        });
    }

    /// The gap in the loan: in a client, `current` must not fall back to the store
    /// even when it is empty, because that read is the macOS password dialog.
    #[test]
    fn a_client_without_a_loan_has_no_session_instead_of_reading_the_store() {
        let creds = Credentials {
            url: "https://hoard.example".to_string(),
            token: format!("hoard_v1_{}", "c".repeat(64)),
            user: None,
        };
        set_lent(Some(creds.clone()));
        assert_eq!(lent().expect("lent").token, creds.token);
        assert_eq!(current().expect("ok").expect("lent").token, creds.token);

        set_lent(None);
        mark_client();
        assert!(lent().is_none());
        assert!(
            current().expect("ok").is_none(),
            "a client with no loan cannot read the store"
        );
    }
}
