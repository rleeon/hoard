//! The IPC client plus **"spawn if absent"**.
//!
//! What the desktop and the CLI use to talk to the service. The interesting part is
//! [`Client::ensure_running`], idempotent by design (ADR 0021, Part A): "both
//! clients do the same thing: connect to the service; if there is none, start it.
//! Under a race, whoever loses the start simply connects to the winner (the socket
//! bind settles the tie)".
//!
//! There is no "is there a daemon already?" check followed by a start: that is a
//! TOCTOU and would produce two engines. The arbiter is the bind, inside the
//! daemon; this side only launches the process and connects again. Launching two
//! daemons at once is **correct**: one wins the socket and the other exits without
//! doing anything.
//!
//! ## The exception: a deliberate shutdown stays down
//!
//! "Start it if there is none" has one case where it is wrong: the service is
//! absent because **somebody just stopped it on purpose**. An attached client used
//! to bring it back about 3 s after a `hoard sync stop`, because its reconnect is
//! `ensure_running` and it had no way to tell "somebody stopped it" from "it
//! crashed". The daemon now states the difference ([`ServerFrame::Goodbye`]) and
//! this module remembers it ([`stopped_on_purpose`]): while it is set, clients keep
//! reconnecting but **start** nothing.
//!
//! It is **process** memory, not a file: a marker on disk would be the pidfile's
//! mistake all over again (it goes stale, and nobody knows whether it is lying). And
//! it heals itself, since any successful handshake clears it: if there is a service
//! to greet, "it is stopped" is no longer true.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use hoard_core::ipc::{
    AdoptedSession, Backlog, ClientFrame, CloudToken, DaemonStatus, Hello, JournalEntry, Payload,
    Reply, Request, ServerFrame, ServerSession, Welcome, PROTOCOL_VERSION,
};
use tokio::io::{ReadHalf, WriteHalf};

use crate::codec::{read_frame, write_frame};
use crate::endpoint::{Endpoint, ENDPOINT_ENV};
use crate::transport::{self, ClientStream};

/// Override de la ruta del binario del daemon. El empaquetado lo pone junto al
/// ejecutable del cliente (eso es lo que busca [`daemon_binary`]); esto es para
/// desarrollo y tests.
pub const DAEMON_BIN_ENV: &str = "HOARDD_BIN";

/// How long a freshly launched daemon gets to open its socket.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(10);

/// The service said goodbye: somebody stopped it on purpose and this process must
/// not bring it back. See the module header.
static STOPPED_ON_PURPOSE: AtomicBool = AtomicBool::new(false);

/// Do we know the service is stopped on purpose?
///
/// The clients that reconnect in a loop (the desktop's event relay, `hoard sync
/// run`'s `follow`) consult it to space their retries out: there is nobody to
/// connect to until somebody starts it by hand.
pub fn stopped_on_purpose() -> bool {
    STOPPED_ON_PURPOSE.load(Ordering::Relaxed)
}

/// Anota la despedida del daemon.
fn note_farewell(reason: &str) {
    if !STOPPED_ON_PURPOSE.swap(true, Ordering::Relaxed) {
        tracing::info!(
            reason,
            "the Hoard service said goodbye; it won't be restarted from here"
        );
    }
}

/// Forgets the farewell. The handshake calls it: if we managed to greet a daemon,
/// "it is stopped" has stopped being true.
fn clear_farewell() {
    if STOPPED_ON_PURPOSE.swap(false, Ordering::Relaxed) {
        tracing::info!("the Hoard service is up again");
    }
}

/// Algo que el daemon empuja sin que se lo pidan.
#[derive(Debug, Clone)]
pub enum Push {
    /// Fila nueva del journal.
    Event(JournalEntry),
    /// We have lagged and the channel dropped rows, so the backlog has to be asked
    /// for again from `cursor`. It is announced rather than left as an invisible
    /// gap.
    Resync { cursor: u64, dropped: u64 },
    /// The service is stopping on purpose. Whoever is listening decides what to do:
    /// the CLI finishes (its job was following a sync that no longer runs), the
    /// desktop paints the engine as stopped and waits. Neither relaunches it.
    Goodbye { reason: String },
}

/// A connection to the daemon.
pub struct Client {
    reader: ReadHalf<ClientStream>,
    writer: WriteHalf<ClientStream>,
    next_id: u64,
    welcome: Welcome,
    /// Pushes that arrived while we were waiting for a request's response. They are
    /// not dropped: an event lost because the status happened to be asked for at the
    /// same time is exactly the kind of hole this protocol exists not to have.
    pushes: VecDeque<Push>,
}

impl Client {
    /// Connects to a daemon that is already up. It fails when there is none; to
    /// start one, use [`Client::ensure_running`].
    pub async fn connect(endpoint: &Endpoint, client_name: &str) -> Result<Self> {
        let stream = transport::connect(endpoint)
            .await
            .with_context(|| format!("connecting to {endpoint}"))?;
        Self::handshake(stream, client_name).await
    }

    /// Connect; if there is no service, launch it and connect again.
    ///
    /// Unless we have been told it was stopped on purpose: then this is a plain
    /// [`Client::connect`] and the error explains it has to be started. An attached
    /// client must not undo a `hoard sync stop` by the mere act of reconnecting.
    pub async fn ensure_running(endpoint: &Endpoint, client_name: &str) -> Result<Self> {
        if let Ok(stream) = transport::connect(endpoint).await {
            return Self::handshake(stream, client_name).await;
        }
        if stopped_on_purpose() {
            bail!(
                "the Hoard service is stopped (someone stopped it on purpose); \
                 start it again with `hoard sync start`"
            );
        }
        // An update is replacing the binaries right now. Starting one here is
        // how a Windows update used to fail: the installer stops `hoardd.exe`,
        // this reconnect brings it back from the old binary two seconds later,
        // and NSIS then can't write the file it just made room for. Whoever
        // finishes the swap starts the service (the NSIS post-install hook, or
        // `relaunch` on the way out), so waiting is not waiting forever.
        if hoard_agent::install::Swap::in_progress() {
            bail!("Hoard is being updated right now; the service will be back in a moment");
        }
        spawn_daemon(endpoint)?;
        let stream = transport::connect_with_deadline(endpoint, Instant::now() + SPAWN_TIMEOUT)
            .await
            .with_context(|| {
                format!("waiting for the hoardd we just started to listen on {endpoint}")
            })?;
        Self::handshake(stream, client_name).await
    }

    async fn handshake(stream: ClientStream, client_name: &str) -> Result<Self> {
        let (reader, mut writer) = tokio::io::split(stream);
        write_frame(
            &mut writer,
            &ClientFrame::Hello(Hello {
                protocol: PROTOCOL_VERSION,
                client: client_name.to_string(),
            }),
        )
        .await
        .context("sending the hello")?;
        let mut reader = reader;
        match read_frame::<_, ServerFrame>(&mut reader).await? {
            Some(ServerFrame::Welcome(welcome)) => {
                // There is a service to greet, so whatever farewell we remembered no
                // longer describes reality (somebody started it again).
                clear_farewell();
                Ok(Self {
                    reader,
                    writer,
                    next_id: 1,
                    welcome,
                    pushes: VecDeque::new(),
                })
            }
            // We greeted a service that is shutting down on purpose. Not a service
            // to talk to, but not a crash either: writing it down here is what stops
            // the retry three seconds from now from relaunching it (the shutdown
            // window lasts as long as the last presence beat, which goes over the
            // network).
            Some(ServerFrame::Goodbye { reason }) => {
                note_farewell(&reason);
                bail!("the Hoard service is stopping: {reason}")
            }
            // The versioned handshake at work: the daemon states its version, so the
            // client can ask for the service to be updated or restarted rather than
            // showing a parse error.
            Some(ServerFrame::Rejected(rejected)) => bail!(
                "the daemon refused the connection: {} (daemon {} speaks protocol {})",
                rejected.reason,
                rejected.daemon_version,
                rejected.daemon_protocol
            ),
            Some(other) => bail!("the daemon answered the hello with {other:?}"),
            None => bail!("the daemon closed the connection during the handshake"),
        }
    }

    /// What the daemon said on connect: version, pid, epoch and cursor.
    pub fn welcome(&self) -> &Welcome {
        &self.welcome
    }

    /// Sends a request and waits for **its** response, queueing any push that
    /// arrives along the way.
    pub async fn request(&mut self, request: Request) -> Result<Payload> {
        let id = self.next_id;
        self.next_id += 1;
        write_frame(&mut self.writer, &ClientFrame::Request { id, request })
            .await
            .context("sending a request")?;
        loop {
            match read_frame::<_, ServerFrame>(&mut self.reader).await? {
                Some(ServerFrame::Reply { id: got, reply }) if got == id => {
                    return match reply {
                        Reply::Ok(payload) => Ok(payload),
                        // Typed, not `{err:?}`: this message ends up in front of the
                        // user (a desktop toast, a line of CLI output).
                        Reply::Error(err) => Err(anyhow::Error::new(err)),
                    };
                }
                Some(ServerFrame::Event(entry)) => self.pushes.push_back(Push::Event(entry)),
                Some(ServerFrame::Resync { cursor, dropped }) => {
                    self.pushes.push_back(Push::Resync { cursor, dropped })
                }
                // The farewell is noted **right here**, not when the push is
                // consumed: this connection is about to close and whoever was waiting
                // for a response may never get to read the queue.
                Some(ServerFrame::Goodbye { reason }) => {
                    note_farewell(&reason);
                    self.pushes.push_back(Push::Goodbye { reason });
                }
                // A response to another in-flight request, a repeated handshake, or a
                // frame from a newer daemon: none of that is this wait's business.
                Some(_) => continue,
                None => bail!("the daemon closed the connection"),
            }
        }
    }

    pub async fn ping(&mut self) -> Result<(String, u32)> {
        match self.request(Request::Ping).await? {
            Payload::Pong {
                daemon_version,
                pid,
            } => Ok((daemon_version, pid)),
            other => bail!("unexpected answer to ping: {other:?}"),
        }
    }

    pub async fn status(&mut self) -> Result<DaemonStatus> {
        match self.request(Request::Status).await? {
            Payload::Status(status) => Ok(status),
            other => bail!("unexpected answer to status: {other:?}"),
        }
    }

    /// Borrows a valid Cloud token. `rejected` is the token this client just got a
    /// 401 for, so the daemon knows handing back the same one is no use.
    ///
    /// The client persists **none** of this: the daemon writes the full pair, being
    /// the only rotator (ADR 0021, Part A).
    pub async fn cloud_token(&mut self, rejected: Option<String>) -> Result<CloudToken> {
        match self.request(Request::CloudToken { rejected }).await? {
            Payload::CloudToken(token) => Ok(token),
            other => bail!("unexpected answer to cloud_token: {other:?}"),
        }
    }

    /// Hands the daemon a freshly minted Cloud session so **it** stores it. The
    /// counterpart to [`Client::cloud_token`]: the client mints (it finishes the
    /// OAuth) and lends; the daemon stores, rotates and lends back.
    ///
    /// Writing it here is the macOS bug this exists to kill: the keyring item ends up
    /// in the name of whoever creates it, and the service, being another binary,
    /// would have to ask the user for permission on every read.
    pub async fn adopt_session(&mut self, session: AdoptedSession) -> Result<()> {
        match self.request(Request::AdoptSession { session }).await? {
            Payload::Ack => Ok(()),
            other => bail!("unexpected answer to adopt_session: {other:?}"),
        }
    }

    /// Tells the daemon to forget the Cloud session (logout). Deleting the keyring
    /// item also has to be authorised, so its owner does it.
    pub async fn forget_session(&mut self) -> Result<()> {
        match self.request(Request::ForgetSession).await? {
            Payload::Ack => Ok(()),
            other => bail!("unexpected answer to forget_session: {other:?}"),
        }
    }

    /// Hands the daemon the self-hosted session this client has just validated. The
    /// twin of [`Client::adopt_session`].
    pub async fn adopt_server_session(&mut self, session: ServerSession) -> Result<()> {
        match self
            .request(Request::AdoptServerSession { session })
            .await?
        {
            Payload::Ack => Ok(()),
            other => bail!("unexpected answer to adopt_server_session: {other:?}"),
        }
    }

    /// Tells the daemon to forget the self-hosted session (logout).
    pub async fn forget_server_session(&mut self) -> Result<()> {
        match self.request(Request::ForgetServerSession).await? {
            Payload::Ack => Ok(()),
            other => bail!("unexpected answer to forget_server_session: {other:?}"),
        }
    }

    /// Borrows the self-hosted session (URL, token, who you are). A `hoard_v1_`
    /// token does not expire, so asking once per process is enough.
    pub async fn server_session(&mut self) -> Result<ServerSession> {
        match self.request(Request::ServerToken).await? {
            Payload::ServerSession(session) => Ok(session),
            other => bail!("unexpected answer to server_token: {other:?}"),
        }
    }

    /// Pide el backlog desde `since` y queda suscrito al push en vivo.
    pub async fn subscribe(&mut self, since: Option<u64>) -> Result<Backlog> {
        match self.request(Request::Subscribe { since }).await? {
            Payload::Backlog(backlog) => Ok(backlog),
            other => bail!("unexpected answer to subscribe: {other:?}"),
        }
    }

    /// The next push. It returns what was queued during a request first. `None`
    /// means the daemon closed.
    pub async fn next_push(&mut self) -> Result<Option<Push>> {
        if let Some(push) = self.pushes.pop_front() {
            return Ok(Some(push));
        }
        loop {
            match read_frame::<_, ServerFrame>(&mut self.reader).await? {
                Some(ServerFrame::Event(entry)) => return Ok(Some(Push::Event(entry))),
                Some(ServerFrame::Resync { cursor, dropped }) => {
                    return Ok(Some(Push::Resync { cursor, dropped }))
                }
                Some(ServerFrame::Goodbye { reason }) => {
                    note_farewell(&reason);
                    return Ok(Some(Push::Goodbye { reason }));
                }
                Some(_) => continue,
                None => return Ok(None),
            }
        }
    }
}

/// The daemon binary's path, in order of authority: the override, **the one the
/// installed service runs**, the sibling of the current executable (which is how it
/// is packaged), and last the `PATH`.
///
/// The second step is not a preference: with the app and the terminal installer
/// living side by side there can be two `hoardd` on disk (the package's in
/// `/usr/bin`, the tarball's in `~/.local/bin`), and "sibling, else PATH" made the
/// chosen binary depend on **who** asked, with the app bringing up its own and the
/// terminal its own. There is only one daemon per user, so there can only be one
/// answer: the one the service manager already picked. It is the same class of
/// failure as the old `hoard-server` on the `PATH` eclipsing the good one, solved at
/// the root instead of by cleaning binaries up by hand.
pub fn daemon_binary() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os(DAEMON_BIN_ENV).filter(|v| !v.is_empty()) {
        return std::path::PathBuf::from(path);
    }
    if let Some(path) = crate::autostart::installed_exec_start() {
        if path.is_file() {
            return path;
        }
    }
    own_daemon_binary()
}

/// **This installation's** daemon: the override, this executable's sibling, and
/// otherwise the `PATH`. Deliberately blind to the installed service.
///
/// It is what [`crate::autostart`] puts in the `ExecStart`, and that is why it
/// cannot look at the unit: the unit is what we are declaring. If it did, an update
/// that moved the binary would rewrite the unit with the **old** path it had just
/// read, and the service would keep starting the previous binary for ever. Clients
/// use [`daemon_binary`], which does consult the unit; whoever declares it uses this
/// one.
pub fn own_daemon_binary() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os(DAEMON_BIN_ENV).filter(|v| !v.is_empty()) {
        return std::path::PathBuf::from(path);
    }
    let name = format!("hoardd{}", std::env::consts::EXE_SUFFIX);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(sibling) = exe.parent().map(|d| d.join(&name)) {
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    std::path::PathBuf::from(name)
}

/// Lanza el daemon desasido de nosotros. Que dos clientes lo lancen a la vez es
/// correcto: uno gana el socket y el otro sale.
fn spawn_daemon(endpoint: &Endpoint) -> Result<()> {
    let binary = daemon_binary();
    let mut command = std::process::Command::new(&binary);
    // El endpoint viaja por entorno para que un daemon lanzado por un cliente con
    // socket propio (tests, dos instalaciones) escuche donde el cliente mira.
    command
        .env(ENDPOINT_ENV, endpoint.as_str())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    detach(&mut command);
    command.spawn().map_err(|e| {
        // `NotFound` here is not "the start failed", it is "the engine is missing".
        // It is said in those words: this is the message somebody sees when they open
        // the app or type `hoard track`, and without the hint the symptom is
        // indistinguishable from a permissions failure or a daemon that started and
        // died.
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "the sync engine ({}) isn't there. `hoard` is a thin client of `hoardd` and the \
                 two ship together, so reinstall the core (https://hoard.services/install.sh) or \
                 drop `hoardd` beside `hoard`.",
                binary.display()
            )
        } else {
            anyhow::Error::new(e).context(format!("starting the daemon ({})", binary.display()))
        }
    })?;
    Ok(())
}

/// Starts **our relief** after an update that replaced this binary.
///
/// It is `spawn_daemon` without the endpoint from the environment: what gets
/// relieved is the service itself, and the endpoint it should use is the one it
/// resolves on its own (we inherit `HOARDD_SOCKET` when there was one, so a daemon
/// with its own socket is relieved on its socket). It returns the child's pid, which
/// is all that can be asserted here: if the new binary were broken, what says so is
/// the child's log, not us.
///
/// The caller must have **released the socket** first: the arbiter is ownership of
/// it, and a child that arrives and finds it taken exits with 0 without serving
/// anything (`Outcome::AlreadyRunning`).
pub fn respawn_service() -> Result<u32> {
    // `own_daemon_binary` and not `daemon_binary`: what has to start is the binary
    // we just replaced in **our** place. `daemon_binary` prefers the installed unit's
    // `ExecStart`, which on a machine with two installations would point at the
    // other.
    let binary = own_daemon_binary();
    let mut command = std::process::Command::new(&binary);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    detach(&mut command);
    let child = command
        .spawn()
        .with_context(|| format!("starting the updated daemon ({})", binary.display()))?;
    Ok(child.id())
}

/// The service has to outlive whoever started it, which is the whole point:
/// closing the app (or Ctrl-C in the CLI) must not kill the sync.
#[cfg(unix)]
fn detach(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: `setsid` es async-signal-safe y no toca memoria del padre, que es
    // exactamente lo que `pre_exec` exige.
    unsafe {
        command.pre_exec(|| {
            // Its own session: the Ctrl-C from the client's terminal does not reach
            // here. If it fails (we are the session leader already) we carry on: it is
            // not fatal.
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(windows)]
fn detach(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, DETACHED_PROCESS};
    // Sin consola y sin heredar la del cliente: si el desktop lo lanza no debe
    // aparecer una ventana negra, y si lo lanza la CLI no debe morir con ella.
    command.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The binary override rules: it is what lets a test launch the freshly built
    /// daemon rather than an installed one.
    #[test]
    fn the_binary_override_wins() {
        let name = daemon_binary();
        // Sin override, el nombre acaba en `hoardd` (con sufijo de la plataforma).
        assert!(
            name.to_string_lossy().contains("hoardd"),
            "unexpected daemon path: {}",
            name.display()
        );
    }
}
