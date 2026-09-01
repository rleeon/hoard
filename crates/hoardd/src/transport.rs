//! The local socket: exclusive bind, user-only permissions, accept and connect.
//!
//! Two implementations behind the same pair of types (`UnixListener` on
//! Linux/macOS, a named pipe on Windows), both delivering three things ADR 0021
//! asks for explicitly:
//!
//! 1. **Owning the socket is the arbiter.** "The pidfile goes away: the arbiter
//!    becomes the service's *ownership of the socket*, a real mutex with liveness
//!    rather than a pidfile consulted once." Here that is literal:
//!    [`Listener::bind`] returns [`BindError::AlreadyRunning`] when another daemon
//!    holds it, and whoever loses the bind connects to the winner instead of
//!    starting a second engine.
//! 2. **Real liveness.** On unix the mutex is a `flock` on `hoardd.lock`: the
//!    kernel releases it when the process dies, whatever happens, so a daemon that
//!    crashes leaves no lock held (the pidfile's classic failure). On Windows
//!    `FILE_FLAG_FIRST_PIPE_INSTANCE` does the job, atomically, and its lifetime is
//!    the handle's.
//! 3. **User only.** 0700 on the directory plus 0600 on the socket; on Windows an
//!    ACL built from the user's SID (see [`crate::winsec`]).
//!
//! A **stale** socket (a file left by a dead daemon) is not detected by heuristic:
//! it is always deleted right after winning the lock. With the mutex in hand nobody
//! can be listening there, so the file is garbage by construction. That order (lock,
//! then unlink, then bind) is what stops two daemons starting at once from treading
//! on each other's socket.

use crate::endpoint::Endpoint;

/// Why listening failed. The distinction is what decides the daemon's behaviour:
/// `AlreadyRunning` is a **correct** ending (there is a service already, this
/// process was not needed), anything else is a failure.
#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("another hoardd already owns {address}")]
    AlreadyRunning { address: String },
    #[error(transparent)]
    Failed(#[from] anyhow::Error),
}

#[cfg(unix)]
mod imp {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::io::AsRawFd;
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result};
    use tokio::net::{UnixListener, UnixStream};

    use super::BindError;
    use crate::endpoint::Endpoint;

    pub type ServerStream = UnixStream;
    pub type ClientStream = UnixStream;

    /// El `flock` que hace de mutex del servicio. Vive mientras viva el
    /// [`Listener`]; el kernel lo suelta si el proceso muere sin llegar a
    /// `Drop`.
    #[derive(Debug)]
    struct LockFile {
        /// Just held: the lock lives as long as the open file does. Nobody reads
        /// it, and that is the point; unlike the pidfile, there is nothing here to
        /// interpret.
        _file: File,
    }

    impl LockFile {
        fn acquire(path: &Path) -> Result<Self, BindError> {
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(path)
                .with_context(|| format!("opening the lock file {}", path.display()))?;
            let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
            // SAFETY: `fd` comes from a live `File` and `flock` only uses it for
            // the duration of the call.
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    return Err(BindError::AlreadyRunning {
                        address: path.display().to_string(),
                    });
                }
                return Err(BindError::Failed(
                    anyhow::Error::new(err).context(format!("locking {} (flock)", path.display())),
                ));
            }
            let mut file = file;
            // The pid is diagnostics (`ps`, a log, a bug report). The lock does NOT
            // depend on reading it: that was the pidfile, and it is why a recycled
            // pid or a stale file used to hand us a phantom owner.
            let _ = file.set_len(0);
            let _ = writeln!(file, "{}", std::process::id());
            let _ = file.flush();
            Ok(Self { _file: file })
        }
    }

    #[derive(Debug)]
    pub struct Listener {
        inner: UnixListener,
        path: PathBuf,
        _lock: LockFile,
    }

    impl Listener {
        pub fn bind(endpoint: &Endpoint) -> Result<Self, BindError> {
            let path = endpoint.path().to_path_buf();
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
                // 0700: nobody else gets into the socket's directory. It is the
                // first of the two fences; the other is the 0600 below.
                let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
            }
            let lock = LockFile::acquire(&endpoint.lock_path())?;
            // With the lock in hand, any socket left here belongs to a dead
            // daemon.
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("removing the stale socket {}", path.display()))?;
            }
            let inner =
                UnixListener::bind(&path).with_context(|| format!("binding {}", path.display()))?;
            // `bind` creates the node with 0755 & !umask, so the 0600 goes on
            // afterwards. The directory's 0700 covers the window.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("tightening permissions on {}", path.display()))?;
            Ok(Self {
                inner,
                path,
                _lock: lock,
            })
        }

        pub async fn accept(&mut self) -> Result<ServerStream> {
            let (stream, _addr) = self.inner.accept().await.context("accepting a client")?;
            Ok(stream)
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            // Salida limpia: el socket no queda para que el siguiente arranque
            // tenga que barrerlo. El `flock` lo suelta el cierre del fichero.
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Connects to the daemon. An `Err` of `NotFound`/`ConnectionRefused` means
    /// "there is no daemon", which is information, not a failure.
    pub async fn connect(endpoint: &Endpoint) -> std::io::Result<ClientStream> {
        UnixStream::connect(endpoint.path()).await
    }

    /// Is the connection worth retrying (is the daemon still coming up)?
    pub fn is_transient(err: &std::io::Error) -> bool {
        matches!(
            err.kind(),
            std::io::ErrorKind::NotFound
                | std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::WouldBlock
        )
    }
}

#[cfg(windows)]
mod imp {
    use anyhow::{Context, Result};
    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_PIPE_BUSY};

    use super::BindError;
    use crate::endpoint::Endpoint;
    use crate::winsec::SecurityDescriptor;

    pub type ServerStream = NamedPipeServer;
    pub type ClientStream = NamedPipeClient;

    pub struct Listener {
        name: String,
        security: SecurityDescriptor,
        /// Instancia creada y esperando cliente. Siempre `Some` entre accepts.
        pending: Option<NamedPipeServer>,
    }

    fn create(
        name: &str,
        security: &SecurityDescriptor,
        first: bool,
    ) -> std::io::Result<NamedPipeServer> {
        let mut attrs = security.attributes();
        let mut options = ServerOptions::new();
        options.first_pipe_instance(first);
        // SAFETY: `attrs` (y el SD al que apunta) viven hasta que la llamada
        // retorna; Windows copia el descriptor de seguridad al objeto.
        unsafe {
            options.create_with_security_attributes_raw(
                name,
                &mut attrs as *mut _ as *mut std::ffi::c_void,
            )
        }
    }

    impl Listener {
        pub fn bind(endpoint: &Endpoint) -> Result<Self, BindError> {
            let name = endpoint.as_str().to_string();
            let security = SecurityDescriptor::user_only().map_err(BindError::Failed)?;
            // `first_pipe_instance` is the mutex: if the name already exists this
            // fails with ACCESS_DENIED and the loser connects to the winner. There is
            // no window between "check" and "create" because there is no check.
            match create(&name, &security, true) {
                Ok(pending) => {
                    let listener = Self {
                        name,
                        security,
                        pending: Some(pending),
                    };
                    // From the live object, not from the descriptor we handed it, so
                    // the ACL this machine really runs with ends up written in the log
                    // instead of being deduced from the code.
                    match listener.dacl_sddl() {
                        Ok(sddl) => tracing::info!(dacl = %sddl, "hoardd: named pipe ACL"),
                        Err(err) => {
                            tracing::warn!(error = %format!("{err:#}"), "hoardd: couldn't read back the pipe ACL")
                        }
                    }
                    Ok(listener)
                }
                Err(err) if err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => {
                    Err(BindError::AlreadyRunning { address: name })
                }
                Err(err) => Err(BindError::Failed(
                    anyhow::Error::new(err).context(format!("creating the named pipe {name}")),
                )),
            }
        }

        /// The DACL the OS created **this** pipe with, in SDDL. Used by the startup
        /// log and by the test that asserts `Everyone` is not in it.
        pub fn dacl_sddl(&self) -> Result<String> {
            use std::os::windows::io::AsRawHandle;
            let pipe = self
                .pending
                .as_ref()
                .context("the named pipe listener has no pending instance")?;
            // SAFETY: el handle es de un `NamedPipeServer` vivo (`pipe` lo
            // mantiene prestado durante toda la llamada).
            unsafe { crate::winsec::dacl_sddl(pipe.as_raw_handle() as _) }
        }

        pub async fn accept(&mut self) -> Result<ServerStream> {
            let server = self
                .pending
                .take()
                .context("the named pipe listener has no pending instance")?;
            server.connect().await.context("accepting a client")?;
            // The next instance is created *after* the client is in: for that
            // instant a new client sees ERROR_PIPE_BUSY, which `is_transient` marks
            // as retryable (which is what `connect_with_deadline`'s loop does).
            self.pending = Some(
                create(&self.name, &self.security, false)
                    .with_context(|| format!("re-arming the named pipe {}", self.name))?,
            );
            Ok(server)
        }
    }

    pub async fn connect(endpoint: &Endpoint) -> std::io::Result<ClientStream> {
        ClientOptions::new().open(endpoint.as_str())
    }

    pub fn is_transient(err: &std::io::Error) -> bool {
        if err.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) {
            return true;
        }
        matches!(
            err.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
        )
    }
}

pub use imp::{connect, is_transient, ClientStream, Listener, ServerStream};

/// Connects, retrying until `deadline`. This is what "spawn if absent" uses to
/// wait for the daemon we just launched (or whoever won the race) to open its
/// socket.
pub async fn connect_with_deadline(
    endpoint: &Endpoint,
    deadline: std::time::Instant,
) -> std::io::Result<ClientStream> {
    loop {
        match connect(endpoint).await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                if !is_transient(&err) || std::time::Instant::now() >= deadline {
                    return Err(err);
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
}

/// What Slice 4a could not check: that the named pipe's ACL is the one we think it
/// is. It runs on Windows only, against a real pipe.
#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    /// Neither `Everyone` (`WD`) nor "authenticated users" (`AU`) in the pipe's
    /// DACL. Game names, local paths and the sync's commands travel over it: with no
    /// explicit descriptor Windows grants `Everyone` read access, which is exactly
    /// what this test stops from coming back.
    #[tokio::test]
    async fn the_pipe_acl_excludes_everyone() {
        let dir = std::env::temp_dir();
        let endpoint = Endpoint::scoped(&dir, &format!("acl-{}", std::process::id()));
        let listener = Listener::bind(&endpoint).expect("bind");
        let sddl = listener.dacl_sddl().expect("read the pipe DACL back");

        assert!(
            !sddl.contains(";;;WD)"),
            "Everyone must not be in the pipe DACL: {sddl}"
        );
        assert!(
            !sddl.contains(";;;AU)"),
            "authenticated users must not be in the pipe DACL: {sddl}"
        );
        // And what should be there: us and LocalSystem. Our own SID comes out
        // numeric (`S-1-5-21-...`) except when it is an account with a known alias,
        // which SDDL abbreviates: on the CI runners the session is the built-in
        // Administrator and the descriptor reads `(A;;FA;;;LA)`. `LA` is that
        // ACCOUNT, not the Administrators group (`BA`), so it is still "one specific
        // user" and the property this test guards holds.
        assert!(
            sddl.contains(";;;S-1-5-21") || sddl.contains(";;;LA)"),
            "the pipe must be granted to a specific user account: {sddl}"
        );
        assert!(sddl.contains(";;;SY)"), "SYSTEM keeps access: {sddl}");
        // DACL protegido: sin herencia que reabra lo que acabamos de cerrar.
        assert!(
            sddl.starts_with("D:P"),
            "the DACL must be protected: {sddl}"
        );
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The second bind of the same endpoint does not start a second service: it is
    /// told there is an owner already. This is the daemon's half of "spawn if
    /// absent" being idempotent (the other half, the loser connecting to the winner,
    /// is proved by `tests/spawn_if_absent.rs` with real processes).
    #[tokio::test]
    async fn a_second_bind_reports_the_owner() {
        let dir = tempfile::tempdir().unwrap();
        let ep = Endpoint::new(
            dir.path()
                .join("hoardd.sock")
                .to_string_lossy()
                .into_owned(),
        );
        let _first = Listener::bind(&ep).expect("first bind wins");
        match Listener::bind(&ep) {
            Err(BindError::AlreadyRunning { .. }) => {}
            other => panic!("expected AlreadyRunning, got {other:?}"),
        }
    }

    /// Permisos: 0600 el socket, 0700 su directorio. Otro usuario local no
    /// maneja tu sync ni te lee los eventos.
    #[tokio::test]
    async fn the_socket_is_user_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("run");
        let ep = Endpoint::new(sub.join("hoardd.sock").to_string_lossy().into_owned());
        let _listener = Listener::bind(&ep).unwrap();
        let sock_mode = std::fs::metadata(ep.path()).unwrap().permissions().mode() & 0o777;
        let dir_mode = std::fs::metadata(&sub).unwrap().permissions().mode() & 0o777;
        assert_eq!(sock_mode, 0o600, "socket must be user-only");
        assert_eq!(dir_mode, 0o700, "socket dir must be user-only");
    }

    /// A socket left by a dead daemon does not block startup: it is deleted on
    /// winning the lock. Without this, one crash left the service unstartable until
    /// somebody deleted the file by hand.
    #[tokio::test]
    async fn a_stale_socket_file_does_not_block_the_bind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hoardd.sock");
        let ep = Endpoint::new(path.to_string_lossy().into_owned());
        {
            let listener = Listener::bind(&ep).unwrap();
            // Simulating the dirty death: the file stays, and closing the fd is
            // what releases the lock. `mem::forget` avoids the `Drop` that would
            // clean it up.
            std::mem::forget(listener);
        }
        assert!(path.exists(), "the stale socket must still be there");
        // The `flock` is still held by this very process (we leaked it), and flock
        // is per *open file*: a second `open` plus `flock` from the same process
        // blocks too, so what is checked here is the part that matters, that the
        // stale file is deleted once the lock can be taken.
        std::fs::remove_file(ep.lock_path()).unwrap();
        let _second = Listener::bind(&ep).expect("a stale socket must not block a new daemon");
        assert!(path.exists());
    }

    /// Sin daemon, conectar falla con un error *transitorio* (que es lo que hace
    /// que "spawn if absent" decida lanzarlo en vez de rendirse).
    #[tokio::test]
    async fn connecting_to_nothing_is_transient() {
        let dir = tempfile::tempdir().unwrap();
        let ep = Endpoint::new(dir.path().join("nope.sock").to_string_lossy().into_owned());
        let err = connect(&ep).await.expect_err("nothing is listening");
        assert!(is_transient(&err), "unexpected error: {err}");
    }
}
