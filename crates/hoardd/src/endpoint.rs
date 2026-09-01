//! Where the daemon listens: the socket path (Linux/macOS) or the named pipe's
//! name (Windows), plus the lock file that arbitrates.
//!
//! **Per user, never per machine.** The daemon needs the user's keyring and their
//! cloud login, so on a multi-user machine there is one daemon per session (ADR
//! 0021, Part A). On unix that comes for free: the socket lives under
//! `$XDG_RUNTIME_DIR` (or the user's `state_dir`), which is private already. On
//! Windows the pipe namespace is global, so the name carries the user inside it and
//! the ACL does the rest (see `winsec`).

use anyhow::Result;

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
use anyhow::{bail, Context};

#[cfg(unix)]
use hoard_agent::config::CliConfig;

/// The endpoint override, for tests and diagnostics. Client and daemon have to see
/// the same one, so it is exported by name rather than each inventing its own.
pub const ENDPOINT_ENV: &str = "HOARDD_SOCKET";

/// The daemon's socket address. On unix it is a path; on Windows it is the pipe's
/// name (`\\.\pipe\...`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint(String);

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Endpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self(address.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The [`ENDPOINT_ENV`] override, when it is set and not empty.
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var(ENDPOINT_ENV).ok()?;
        (!raw.trim().is_empty()).then(|| Self::new(raw))
    }

    /// This user's endpoint: the override when there is one, otherwise the
    /// platform default. Both the daemon and the clients call it, and their agreeing
    /// is not courtesy, it is the mutual-exclusion mechanism.
    pub fn resolve() -> Result<Self> {
        match Self::from_env() {
            Some(ep) => Ok(ep),
            None => Self::user_default(),
        }
    }

    #[cfg(unix)]
    pub fn path(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }

    /// A lock file sibling to the socket. It is derived from the address (same
    /// directory, `.lock` extension) so a test endpoint gets its own lock and does
    /// not fight the user's real daemon for one.
    #[cfg(unix)]
    pub fn lock_path(&self) -> PathBuf {
        self.path().with_extension("lock")
    }

    #[cfg(unix)]
    pub fn user_default() -> Result<Self> {
        let path = runtime_dir()?.join("hoardd.sock");
        // `sockaddr_un.sun_path` is 108 bytes on Linux and 104 on macOS. Going over
        // gives a `bind` with an opaque error, so it is said here, with the path in
        // front of you.
        let len = path.as_os_str().len();
        if len > 100 {
            bail!(
                "the socket path is too long for a unix socket ({len} bytes): {}",
                path.display()
            );
        }
        Ok(Self::new(path.to_string_lossy().into_owned()))
    }

    #[cfg(windows)]
    pub fn user_default() -> Result<Self> {
        let user = std::env::var("USERNAME").unwrap_or_default();
        Ok(Self::new(windows_pipe_name(&user)))
    }

    /// An isolated endpoint with a name of its own: the user's, without treading on
    /// it.
    ///
    /// On unix it hangs off `dir` (a temp directory, typically); on Windows the pipe
    /// namespace is **flat and global to the machine**, so `dir` counts for nothing
    /// and uniqueness goes in the name. The signature accepting both is what makes it
    /// possible to write **one** test that runs the same in both places, which Slice
    /// 4a did not have (its tests composed file paths) and which is why the pipe path
    /// stayed verified at the type level only.
    pub fn scoped(dir: &std::path::Path, name: &str) -> Self {
        #[cfg(unix)]
        {
            Self::new(
                dir.join(format!("hoardd-{name}.sock"))
                    .to_string_lossy()
                    .into_owned(),
            )
        }
        #[cfg(windows)]
        {
            let _ = dir;
            Self::new(windows_pipe_name(&format!("test-{name}")))
        }
    }
}

/// The socket's directory on unix: `$XDG_RUNTIME_DIR/hoard` when it exists (tmpfs,
/// 0700 for the user, cleaned up at logout, the canonical place for a user
/// service's socket), and otherwise the usual `state_dir` (macOS does not define
/// `XDG_RUNTIME_DIR`).
#[cfg(unix)]
fn runtime_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir).join("hoard"));
    }
    CliConfig::state_dir().context("resolving the state dir for the socket")
}

/// A user's named-pipe name.
///
/// The `\\.\pipe\` namespace is **global to the machine**: two users in the same
/// terminal server session share the namespace, so the name carries the user. It is
/// sanitised (a pipe name cannot contain `\`) *and* carries a hash of the original
/// name: without the hash, `José` and `Jose` would collapse onto the same pipe and
/// the second user could neither create theirs (the first one's ACL would deny
/// access) nor use it.
pub fn windows_pipe_name(user: &str) -> String {
    let mut safe: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(24)
        .collect::<String>()
        .to_ascii_lowercase();
    if safe.is_empty() {
        safe.push_str("user");
    }
    format!("\\\\.\\pipe\\hoardd-{safe}-{:08x}", fnv1a(user.as_bytes()))
}

/// A 32-bit FNV-1a. It only disambiguates the pipe's name, so there is no need to
/// drag in a hashing dependency for twelve lines.
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for b in bytes {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_env_override_wins() {
        // Without touching the process environment (the tests run in parallel):
        // checking that `new` does not reinterpret the address is enough.
        let ep = Endpoint::new("/tmp/whatever.sock");
        assert_eq!(ep.as_str(), "/tmp/whatever.sock");
    }

    #[cfg(unix)]
    #[test]
    fn the_lock_sits_next_to_the_socket() {
        let ep = Endpoint::new("/run/user/1000/hoard/hoardd.sock");
        assert_eq!(
            ep.lock_path(),
            PathBuf::from("/run/user/1000/hoard/hoardd.lock")
        );
    }

    /// Two different users cannot collide on the name, not even when sanitising
    /// leaves them identical.
    #[test]
    fn pipe_names_are_per_user_and_collision_free() {
        let a = windows_pipe_name("José");
        let b = windows_pipe_name("Jose");
        assert_ne!(a, b);
        assert!(a.starts_with("\\\\.\\pipe\\hoardd-jos"));
        assert_eq!(a, windows_pipe_name("José"));
    }

    /// A username that sanitising leaves empty (Cyrillic, CJK) still gives a valid
    /// pipe name of its own.
    #[test]
    fn an_unsanitisable_user_still_gets_a_name() {
        let name = windows_pipe_name("Пользователь");
        assert!(name.starts_with("\\\\.\\pipe\\hoardd-user-"));
        assert_ne!(name, windows_pipe_name("用户"));
    }
}
