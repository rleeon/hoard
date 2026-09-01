//! # `hoardd`, the local service that owns the engine (ADR 0021, Part A)
//!
//! The sync engine (`hoard_agent::agent::spawn`) is a **long-lived process of its
//! own**: exactly one per user, surviving the app being closed, with the desktop
//! and the CLI talking to it over a local socket. It used to be embedded in two
//! binaries, arbitrated by a pidfile with three design faults: a startup race, no
//! reclaim at all (you stopped the CLI and the desktop never took the engine back),
//! and the sync's lifetime tied to a UI.
//!
//! What lives here: the binary, the IPC (a user-only socket plus a versioned
//! handshake plus a journal with a cursor and a live push), starting and
//! supervising the engine, an idempotent "spawn if absent", **the only rotator of
//! the refresh token** (which also lends it over IPC to whoever needs it,
//! `Request::CloudToken`), starting at boot as a user service ([`autostart`]), the
//! explicit farewell ([`hoard_core::ipc::ServerFrame::Goodbye`]) that tells "it was
//! stopped" from "it crashed", and the **native notifications** ([`notify`]) the
//! service sends so they arrive with the app closed. Neither the desktop nor the
//! CLI carries an engine: `hoard sync` makes sure this service is up and attaches
//! to its journal. There is no pidfile; the socket is the arbiter.
//!
//! ## Map
//!
//! - [`autostart`]: starting at boot as a user service (systemd user, launchd
//!   agent, per-user Task Scheduler). Never system-wide.
//! - [`endpoint`]: where it listens (per-user socket, or named pipe).
//! - [`transport`]: exclusive bind (the socket **is** the arbiter), permissions,
//!   accept and connect.
//! - [`codec`]: frames over the socket (the format lives in `hoard_core::ipc`).
//! - [`journal`]: a journal with a cursor plus push; it stores transitions and
//!   actions, not repeated idles.
//! - [`notify`]: the OS's native notifications (Linux today; Windows and macOS
//!   behind the same interface).
//! - [`engine`]: starting, supervising and pumping the engine's events.
//! - [`server`]: handshake and dispatch.
//! - [`client`]: the client plus "spawn if absent" (what the desktop and the CLI
//!   use).
//! - [`updater`]: the automatic update: look, download, apply, get relieved.
//!
//! The session (resolution, refresher and lending the token) lives in
//! `hoard_agent::session`, one shared implementation for every client.

pub mod autostart;
pub mod client;
pub mod codec;
pub mod endpoint;
pub mod engine;
pub mod journal;
pub mod notify;
pub mod server;
pub mod transport;
pub mod updater;

#[cfg(windows)]
pub mod winsec;

use std::sync::Arc;

use anyhow::Result;
use hoard_agent::agent::AgentEvent;
use hoard_agent::supervisor;
use tokio::sync::mpsc;

use crate::endpoint::Endpoint;
use crate::engine::Engine;
use crate::journal::EventLog;
use crate::server::Daemon;
use crate::transport::{BindError, Listener};

/// Eventos del motor en cola antes de que la bomba los drene. Generoso: perder
/// un evento es perder una fila del journal, y el journal es lo que sostiene el
/// catch-up de los clientes.
const EVENT_CHANNEL: usize = 256;

/// The moment the socket is given so the farewell gets out before the process
/// starts tearing itself down. It is a frame of a few dozen bytes to a local
/// socket: more than enough, and paid once, on the way out.
const GOODBYE_FLUSH: std::time::Duration = std::time::Duration::from_millis(50);

/// How to start the daemon.
pub struct Options {
    /// An explicit endpoint. `None` means the user's (or `HOARDD_SOCKET`'s).
    pub endpoint: Option<Endpoint>,
    /// Start the engine. `false` serves the IPC and nothing else: diagnostics and
    /// tests (a test **cannot** bring the real engine up, it would sync the saves of
    /// whoever runs it).
    pub with_engine: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            endpoint: None,
            with_engine: true,
        }
    }
}

/// How the process ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Servimos hasta que nos pidieron parar.
    Served,
    /// There was a daemon already, so this process was not needed. **Not an
    /// error**: it is the "loses the bind" half of "spawn if absent".
    AlreadyRunning,
    /// An update was applied and the new binary has to be started. The process has
    /// already released the engine and the socket; the caller decides **who** starts
    /// it again (see `main`).
    Relaunching { version: String },
}

/// The relief queue. One slot is enough: two updates applied before the process
/// hears about the first do not exist, since the loop stops on the first request.
const RELAUNCH_CHANNEL: usize = 1;

/// Starts the service and serves until shutdown is asked for (by IPC or signal).
pub async fn run(options: Options) -> Result<Outcome> {
    let endpoint = match options.endpoint {
        Some(ep) => ep,
        None => Endpoint::resolve()?,
    };

    let listener = match Listener::bind(&endpoint) {
        Ok(listener) => listener,
        Err(BindError::AlreadyRunning { address }) => {
            tracing::info!(address = %address, "hoardd: another daemon already owns the socket; exiting");
            return Ok(Outcome::AlreadyRunning);
        }
        Err(BindError::Failed(err)) => return Err(err),
    };
    tracing::info!(
        address = %endpoint,
        pid = std::process::id(),
        version = env!("CARGO_PKG_VERSION"),
        engine = options.with_engine,
        "hoardd: listening"
    );
    sweep_legacy_pidfile();
    // We're serving, so nobody is halfway through replacing the binaries, and on
    // Windows the installer kills the daemon that set the marker, so the guard's
    // `Drop` never runs and only a fresh service can say it's over.
    hoard_agent::install::Swap::forget();

    let log = Arc::new(EventLog::new());
    let engine = Engine::new();
    let daemon = Arc::new(Daemon::new(log.clone(), engine.clone()));
    // The service is what tells the user, not the window (D.14.1): with the app
    // closed there is nobody else who can.
    let notifier = Arc::new(notify::Notifier::for_this_platform());

    // The event channel belongs to the daemon, not to the engine: it is created once
    // and every engine start gets a clone of the sender. That way a restarted engine
    // keeps writing to the same journal and the clients' cursors do not break.
    let (events_tx, events_rx) = mpsc::channel::<AgentEvent>(EVENT_CHANNEL);
    let events_rx = Arc::new(tokio::sync::Mutex::new(events_rx));

    // D.12's rule: anything that outlives a request runs supervised. A panic here is
    // a logged incident and a restart with backoff, not 36 minutes of silence.
    let mut tasks = Vec::new();
    tasks.push(tokio::spawn({
        let engine = engine.clone();
        let log = log.clone();
        let notifier = notifier.clone();
        let events_rx = events_rx.clone();
        supervisor::supervise("hoardd event pump", move || {
            engine::pump(
                engine.clone(),
                log.clone(),
                notifier.clone(),
                events_rx.clone(),
            )
        })
    }));
    if options.with_engine {
        tasks.push(tokio::spawn({
            let engine = engine.clone();
            let events_tx = events_tx.clone();
            supervisor::supervise("hoardd engine keeper", move || {
                engine::keeper(engine.clone(), events_tx.clone())
            })
        }));
    } else {
        engine.disable("the daemon was started with --no-engine");
    }

    let listener = Arc::new(tokio::sync::Mutex::new(listener));
    tasks.push(tokio::spawn({
        let listener = listener.clone();
        let daemon = daemon.clone();
        supervisor::supervise("hoardd ipc", move || {
            server::accept_loop(listener.clone(), daemon.clone())
        })
    }));

    // The automatic update. Supervised like everything else: a panic here would
    // leave the machine never updating and never saying so, which is exactly the
    // failure this loop exists to kill.
    let (relaunch_tx, mut relaunch_rx) = mpsc::channel::<updater::Relaunch>(RELAUNCH_CHANNEL);
    tasks.push(tokio::spawn({
        let updater = daemon.updater.clone();
        let engine = engine.clone();
        let notifier = notifier.clone();
        let relaunch_tx = relaunch_tx.clone();
        supervisor::supervise("hoardd updater", move || {
            updater::watch(
                updater.clone(),
                engine.clone(),
                notifier.clone(),
                relaunch_tx.clone(),
            )
        })
    }));

    let mut relaunching: Option<String> = None;
    tokio::select! {
        _ = daemon.wait_for_shutdown() => {
            tracing::info!("hoardd: stopping on request");
            daemon.say_goodbye("stopped on request");
        }
        _ = shutdown_signal() => {
            tracing::info!("hoardd: stopping on signal");
            daemon.say_goodbye("the service manager stopped it");
        }
        Some(r) = relaunch_rx.recv() => {
            tracing::info!(version = %r.version, "hoardd: stopping to relaunch on the new binary");
            // **There is no farewell here, deliberately.** A farewell means "I was
            // stopped on purpose, do not relaunch me" (D.17), and this is the
            // opposite: we want everybody to bring us back. An attached client sees
            // the socket close with no goodbye, treats it as a crash and does "spawn
            // if absent", which with the binary already replaced on disk starts the
            // new version.
            relaunching = Some(r.version);
        }
    }
    // Both routes are deliberate, so both say goodbye: an attached client has to be
    // able to tell them from a crash, or its reconnect ("spawn if absent") undoes the
    // shutdown three seconds later. The pause is so the frame gets out of the socket
    // before the process leaves; `engine.shutdown()` below usually takes considerably
    // longer (it talks to the network), but it *guarantees* no delay at all when
    // there is no engine.
    tokio::time::sleep(GOODBYE_FLUSH).await;

    // Shutdown order: the engine first (its last presence beat needs a live token
    // and its `shutdown` is clean), then the tasks, then the socket.
    engine.shutdown().await;
    for task in &tasks {
        task.abort();
    }
    for task in tasks {
        let _ = task.await;
    }
    // Releases the socket and the lock, so the file is not left for the next start
    // to sweep. With a relief pending this is also **a condition for it to work**:
    // the new process has to be able to win the bind, and the arbiter is ownership of
    // the socket.
    drop(listener);
    if let Some(version) = relaunching {
        return Ok(Outcome::Relaunching { version });
    }
    Ok(Outcome::Served)
}

/// Deletes the `agent.pid` older versions used to leave behind.
///
/// Nobody reads it any more (ownership of the socket is the arbiter), so leaving it
/// on disk only serves to have somebody look at it again in a year and believe it
/// means something. It runs with the bind already won (we are *the* service for
/// this user) and it is best-effort throughout: if it cannot be done, nothing
/// happens.
fn sweep_legacy_pidfile() {
    let Ok(dir) = hoard_agent::config::CliConfig::state_dir() else {
        return;
    };
    let path = dir.join("agent.pid");
    if !path.exists() {
        return;
    }
    match std::fs::remove_file(&path) {
        Ok(()) => {
            tracing::info!(path = %path.display(), "hoardd: removed the legacy agent pidfile")
        }
        Err(err) => {
            tracing::debug!(error = %err, path = %path.display(), "hoardd: couldn't remove the legacy agent pidfile")
        }
    }
}

/// Resolves when the OS asks us to stop: Ctrl-C, plus SIGTERM on unix, which is
/// the signal `systemctl --user stop` and `launchctl bootout` send. Without the
/// SIGTERM arm the service manager would have to send us SIGKILL, skipping the
/// engine's clean shutdown.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Sends every panic to the log, not just to stderr.
///
/// The same reason as in the desktop (ADR 0021 D.12): a user service has nowhere to
/// print (stderr is `/dev/null` under systemd with no journal, or nothing at all
/// under Task Scheduler), so a task dying of a panic left **no trace whatsoever**.
/// That is exactly what D.11/D.12's poller did.
pub fn install_panic_logger() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("<unnamed>").to_string();
        tracing::error!(
            location = %location,
            thread = %thread,
            "PANIC: a task or thread died"
        );
        previous(info);
    }));
}
