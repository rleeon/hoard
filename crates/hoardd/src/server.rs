//! The IPC server: handshake, request dispatch and event push.
//!
//! One task per connection, and two halves inside it: the request reader and a
//! single writer fed by a channel. **Everything** going out through one writer is
//! what stops a response and a journal push from interleaving mid-frame.
//!
//! ## What happens when a connection dies (or panics)
//!
//! **That** connection dies and nothing else. The accept loop runs under
//! `supervisor::supervise` (D.12's rule: anything outliving a request runs
//! supervised), and the daemon's panic hook sends any panic to the log, so a
//! connection that dies leaves a trace. *Each connection* is not supervised because
//! restarting the body of a connection whose socket no longer exists means nothing:
//! the client reconnects and, thanks to the journal's cursor, recovers what it
//! missed with no gaps.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use hoard_agent::session::LendError;
use hoard_core::ipc::{
    ClientFrame, DaemonStatus, Hello, IpcError, JournalEntry, Payload, Rejected, Reply, Request,
    ServerFrame, Welcome, PROTOCOL_VERSION,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc};

use crate::codec::{read_frame, write_frame};
use crate::engine::{self, Engine};
use crate::journal::EventLog;
use crate::transport::Listener;

/// Outbound frames queued per connection. A client that does not read gets cut off
/// when it fills up, rather than growing the daemon's memory.
const OUTBOX: usize = 512;

/// Farewells queued. Only one is ever sent in the process's life; the channel
/// exists to hand it to the live connections, not to accumulate.
const FAREWELL_CHANNEL: usize = 1;

/// The shared state every connection sees.
pub struct Daemon {
    pub version: String,
    pub pid: u32,
    /// This run's identity: the journal's `seq` values are only comparable within
    /// the same epoch.
    pub epoch: String,
    pub started: Instant,
    pub log: Arc<EventLog>,
    pub engine: Engine,
    /// Se dispara con `Request::Shutdown`; `main` lo espera.
    shutdown: tokio::sync::Notify,
    /// Hands the farewell to the live connections when the shutdown is deliberate.
    /// Every connection has a task waiting here that puts the
    /// [`ServerFrame::Goodbye`] into its outbound queue.
    farewell: broadcast::Sender<String>,
    /// The reason, once it has been said. The channel only reaches whoever was
    /// already connected; this reaches **whoever arrives afterwards**, during the
    /// time we take to shut down (the engine sends its last beat over the network, so
    /// it is not instant). Without it, a client connecting in that window would get a
    /// normal greeting, take the earlier farewell as spent and relaunch the service
    /// when it lost the socket: the whole bug all over again.
    said: std::sync::OnceLock<String>,
    /// The automatic update. The daemon does not drive it (that is
    /// [`crate::updater::watch`]), it only shows it and passes on what the clients
    /// ask for.
    pub updater: crate::updater::Updater,
}

impl Daemon {
    pub fn new(log: Arc<EventLog>, engine: Engine) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
            epoch: uuid::Uuid::new_v4().to_string(),
            started: Instant::now(),
            log,
            engine,
            shutdown: tokio::sync::Notify::new(),
            farewell: broadcast::channel(FAREWELL_CHANNEL).0,
            said: std::sync::OnceLock::new(),
            updater: crate::updater::Updater::new(),
        }
    }

    /// Espera la orden de apagado.
    pub async fn wait_for_shutdown(&self) {
        self.shutdown.notified().await;
    }

    /// Say goodbye to every attached client: **this is a deliberate shutdown**.
    ///
    /// It is sent before the engine is touched, and the caller gives the socket a
    /// moment for it to get out (see `run`). A daemon that really dies never comes
    /// through here, and that is exactly the distinction the client needs in order to
    /// decide whether to relaunch it (ADR 0021 D.17, Slice 4d).
    pub fn say_goodbye(&self, reason: &str) {
        let _ = self.said.set(reason.to_string());
        let listeners = self.farewell.send(reason.to_string()).unwrap_or(0);
        tracing::info!(reason, listeners, "hoardd: saying goodbye to its clients");
    }

    /// The farewell's reason, once said. The handshake consults it: whoever arrives
    /// after the goodbye is answered with the goodbye, not with a greeting that will
    /// be a lie a second later.
    fn farewell_said(&self) -> Option<&str> {
        self.said.get().map(String::as_str)
    }

    fn welcome(&self) -> Welcome {
        Welcome {
            protocol: PROTOCOL_VERSION,
            daemon_version: self.version.clone(),
            pid: self.pid,
            epoch: self.epoch.clone(),
            cursor: self.log.cursor(),
        }
    }

    async fn status(&self) -> DaemonStatus {
        let mut engine_status = self.engine.status();
        // The slots are the engine's live truth, so they are asked for rather than
        // served from a stored counter that may have fallen behind.
        let slots = engine::slot_status(&self.engine).await;
        if engine_status.running {
            engine_status.watched = slots.len();
        }
        DaemonStatus {
            daemon_version: self.version.clone(),
            protocol: PROTOCOL_VERSION,
            pid: self.pid,
            epoch: self.epoch.clone(),
            uptime_secs: self.started.elapsed().as_secs(),
            cursor: self.log.cursor(),
            // Que el frontend sepa si avisamos nosotros. Es una constante de
            // este build (hay backend de notificaciones para esta plataforma o
            // no), no algo que cambie mientras corremos.
            notifications: crate::notify::SUPPORTED,
            engine: engine_status,
            slots,
        }
    }

    /// Stores the session a client has just minted. **The only write of the token
    /// pair in the whole system**, along with the refresher's.
    ///
    /// It goes to `spawn_blocking` because the keyring is synchronous: even bounded
    /// (`KEYRING_TIMEOUT`), it blocks the thread while it waits, and that thread here
    /// is an IPC connection's, so a slow keyring would leave that client's other
    /// requests waiting too (D.19).
    async fn adopt_session(&self, session: hoard_core::ipc::AdoptedSession) -> Result<()> {
        let tokens = hoard_agent::cloud_auth::Tokens {
            access: session.access_token,
            refresh: session.refresh_token,
        };
        let server_url = session.server_url;
        tokio::task::spawn_blocking(move || {
            hoard_agent::cloud_auth::store_tokens(&tokens, &server_url)
        })
        .await
        .context("storing the Cloud session")?
    }

    /// Forgets the Cloud session. `spawn_blocking` for the same reason as
    /// [`Daemon::adopt_session`].
    async fn forget_session(&self) -> Result<()> {
        tokio::task::spawn_blocking(hoard_agent::cloud_auth::clear_session)
            .await
            .context("clearing the Cloud session")?
    }

    /// Stores the self-hosted session a client has just validated. The twin of
    /// [`Daemon::adopt_session`], and `spawn_blocking` for the same reason.
    async fn adopt_server_session(&self, session: hoard_core::ipc::ServerSession) -> Result<()> {
        let creds = hoard_agent::Credentials {
            url: session.server_url,
            token: session.token,
            user: session.user.map(|u| hoard_agent::UserSection {
                user_id: u.user_id,
                username: u.username,
                is_admin: u.is_admin,
            }),
        };
        tokio::task::spawn_blocking(move || hoard_agent::credentials::save(&creds))
            .await
            .context("storing the self-hosted session")??;
        Ok(())
    }

    /// Forgets the self-hosted session.
    async fn forget_server_session(&self) -> Result<()> {
        tokio::task::spawn_blocking(hoard_agent::credentials::clear)
            .await
            .context("clearing the self-hosted session")?
    }

    /// Lends the self-hosted session. It rotates nothing (a `hoard_v1_` token is
    /// static), so it is only a read of the store, but a read **here**, which is
    /// where nothing has to be authorised.
    async fn lend_server_session(&self) -> Result<Option<hoard_core::ipc::ServerSession>> {
        tokio::task::spawn_blocking(hoard_agent::session::lend_server_session)
            .await
            .context("reading the self-hosted session")?
    }

    /// Dispatches everything except `Subscribe` (which needs the connection) and
    /// `Shutdown` (which triggers it). Every engine command is a send to the
    /// `AgentHandle`: what happens next arrives through the journal, not through the
    /// response.
    async fn dispatch(&self, request: Request) -> Reply {
        match request {
            Request::Ping => Reply::Ok(Payload::Pong {
                daemon_version: self.version.clone(),
                pid: self.pid,
            }),
            Request::Status => Reply::Ok(Payload::Status(self.status().await)),
            Request::Reload => match engine::reload(&self.engine).await {
                Ok(_) => Reply::Ok(Payload::Ack),
                Err(err) => self.engine_error(err),
            },
            Request::SetProbeCandidates { dirs } => {
                let dirs: Vec<std::path::PathBuf> =
                    dirs.into_iter().map(std::path::PathBuf::from).collect();
                self.with_engine(|h| async move { h.set_probe_candidates(dirs).await })
                    .await
            }
            // This does not go through `with_engine` either, and that is
            // load-bearing: the token's rotator belongs to **the daemon**, not to the
            // engine. An engine down for want of a session or over a network bump must
            // not leave the desktop unable to talk to the cloud, still less push it
            // into rotating on its own, which is exactly what this design kills.
            Request::CloudToken { rejected } => {
                match hoard_agent::session::lend_token(rejected.as_deref()).await {
                    Ok(token) => {
                        if token.rotated {
                            tracing::info!("hoardd: rotated the Cloud token for a client");
                        }
                        // Lending it meant reading it, and reading it is exactly
                        // what a engine down on a session fault couldn't do. Tell
                        // it, instead of letting it sleep out a five-minute
                        // backoff next to a session that now works.
                        self.engine.wake_if_a_session_would_help();
                        Reply::Ok(Payload::CloudToken(token))
                    }
                    Err(LendError::Gone(reason)) => {
                        tracing::warn!(reason = %reason, "hoardd: no Cloud token to lend");
                        Reply::Error(IpcError::CloudSessionExpired { reason })
                    }
                    Err(LendError::Transient(err)) => {
                        let message = format!("{err:#}");
                        tracing::warn!(error = %message, "hoardd: couldn't lend a Cloud token");
                        Reply::Error(IpcError::Internal { message })
                    }
                }
            }
            // Not through `with_engine` either, and for the same reason as
            // `CloudToken`: storing the session belongs to the daemon, not to the
            // engine. More than that: the engine is down *precisely* because there was
            // no session, and this is what fixes it.
            Request::AdoptSession { session } => {
                match self.adopt_session(session).await {
                    Ok(()) => {
                        tracing::info!("hoardd: adopted a Cloud session handed over by a client");
                        // Learning a new session is a session change: whatever engine
                        // there was is talking to the previous one.
                        self.engine
                            .request_restart("a client handed us a new Cloud session");
                        Reply::Ok(Payload::Ack)
                    }
                    Err(err) => {
                        let message = format!("{err:#}");
                        tracing::warn!(error = %message, "hoardd: couldn't store the Cloud session a client handed over");
                        Reply::Error(IpcError::Internal { message })
                    }
                }
            }
            Request::AdoptServerSession { session } => {
                match self.adopt_server_session(session).await {
                    Ok(()) => {
                        tracing::info!(
                            "hoardd: adopted a self-hosted session handed over by a client"
                        );
                        self.engine
                            .request_restart("a client handed us a new self-hosted session");
                        Reply::Ok(Payload::Ack)
                    }
                    Err(err) => {
                        let message = format!("{err:#}");
                        tracing::warn!(error = %message, "hoardd: couldn't store the self-hosted session a client handed over");
                        Reply::Error(IpcError::Internal { message })
                    }
                }
            }
            Request::ForgetServerSession => match self.forget_server_session().await {
                Ok(()) => {
                    tracing::info!("hoardd: forgot the self-hosted session at a client's request");
                    self.engine
                        .request_restart_if_signed_out(false, "a client signed out of its server");
                    Reply::Ok(Payload::Ack)
                }
                Err(err) => {
                    let message = format!("{err:#}");
                    tracing::warn!(error = %message, "hoardd: couldn't clear the self-hosted session");
                    Reply::Error(IpcError::Internal { message })
                }
            },
            // Like `CloudToken`: it belongs to the daemon, not to the engine. An
            // engine that is down must not leave the app unable to talk to its own
            // server.
            Request::ServerToken => match self.lend_server_session().await {
                Ok(Some(session)) => Reply::Ok(Payload::ServerSession(session)),
                Ok(None) => Reply::Error(IpcError::NoServerSession {
                    reason: "sign in to your server from the app, or run `hoard login --token`"
                        .to_string(),
                }),
                Err(err) => {
                    let message = format!("{err:#}");
                    tracing::warn!(error = %message, "hoardd: couldn't lend the self-hosted session");
                    Reply::Error(IpcError::Internal { message })
                }
            },
            Request::ForgetSession => match self.forget_session().await {
                Ok(()) => {
                    tracing::info!("hoardd: forgot the Cloud session at a client's request");
                    self.engine
                        .request_restart_if_signed_out(true, "a client signed out of Cloud");
                    Reply::Ok(Payload::Ack)
                }
                Err(err) => {
                    let message = format!("{err:#}");
                    tracing::warn!(error = %message, "hoardd: couldn't clear the Cloud session");
                    Reply::Error(IpcError::Internal { message })
                }
            },
            // Not through `with_engine`: restarting a downed engine is precisely
            // what can bring it back (the keeper resolves the session again), so an
            // `EngineDown` here would be answering "I cannot fix it because it is
            // broken".
            Request::RestartEngine => {
                self.engine
                    .request_restart("a client reported a session change");
                Reply::Ok(Payload::Ack)
            }
            Request::BackupNow { save_id } => {
                self.with_engine(|h| async move { h.backup_now(save_id).await })
                    .await
            }
            Request::SweepAll { window_secs } => {
                self.with_engine(|h| async move { h.sweep_all(window_secs).await })
                    .await
            }
            Request::ForceRestore {
                save_id,
                version_num,
            } => {
                self.with_engine(|h| async move { h.force_restore_at(save_id, version_num).await })
                    .await
            }
            Request::SetAutoRestore { enabled } => {
                self.with_engine(|h| async move { h.set_auto_restore(enabled).await })
                    .await
            }
            Request::SetGlobalSync { enabled } => {
                self.with_engine(|h| async move { h.set_global_sync(enabled).await })
                    .await
            }
            // How the update is going. Not through the engine: the updater belongs
            // to the daemon, and a downed engine (usually the very case where updating
            // fixes something) must not leave anybody unable to find out.
            Request::UpdateStatus => Reply::Ok(Payload::Update(self.updater.state())),
            // Apply **now**. It returns immediately with this instant's state:
            // applying can take a while (a native installer, a polkit dialog waiting
            // on a human) and leaving an IPC request hanging all that time would block
            // that client's others. Whoever asked asks again through `UpdateStatus`
            // and watches the phase move.
            Request::ApplyUpdate { version } => {
                tracing::info!(
                    version = version.as_deref().unwrap_or("latest"),
                    "hoardd: a client asked to apply the update now"
                );
                self.updater.apply_now(version);
                Reply::Ok(Payload::Update(self.updater.state()))
            }
            Request::SnoozeUpdate { hours } => {
                self.updater.snooze(hours);
                Reply::Ok(Payload::Update(self.updater.state()))
            }
            // A request from a client newer than this service. It is answered, not
            // hung up on: the client has just updated and we are seconds from being
            // relieved.
            Request::Unknown => Reply::Error(IpcError::Unsupported {
                op: "an operation this version of the Hoard service doesn't know".to_string(),
            }),
            // The two that never get here.
            Request::Subscribe { .. } | Request::Shutdown => Reply::Error(IpcError::Internal {
                message: "handled by the connection loop".to_string(),
            }),
        }
    }

    async fn with_engine<F, Fut>(&self, f: F) -> Reply
    where
        F: FnOnce(hoard_agent::agent::AgentHandle) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        let Some(handle) = self.engine.handle() else {
            return Reply::Error(IpcError::EngineDown {
                reason: self.engine.down_reason(),
            });
        };
        match f(handle).await {
            Ok(()) => Reply::Ok(Payload::Ack),
            Err(err) => self.engine_error(err),
        }
    }

    /// A command that does not reach the engine almost always means the engine is
    /// gone (a closed channel), so it is reported as `EngineDown` with whatever reason
    /// the keeper recorded, not as an opaque `Internal`.
    fn engine_error(&self, err: anyhow::Error) -> Reply {
        tracing::warn!(error = %format!("{err:#}"), "hoardd: a request failed");
        if self.engine.handle().is_none() {
            return Reply::Error(IpcError::EngineDown {
                reason: self.engine.down_reason(),
            });
        }
        Reply::Error(IpcError::Internal {
            message: format!("{err:#}"),
        })
    }
}

/// Accepts connections for ever. It does not return (it cannot produce a
/// `Finished`), so `supervise` only restarts it on a panic and `main` kills it by
/// aborting.
pub async fn accept_loop(
    listener: Arc<tokio::sync::Mutex<Listener>>,
    daemon: Arc<Daemon>,
) -> hoard_agent::supervisor::Finished {
    loop {
        let accepted = {
            let mut guard = listener.lock().await;
            guard.accept().await
        };
        match accepted {
            Ok(stream) => {
                let daemon = daemon.clone();
                tokio::spawn(async move {
                    if let Err(err) = serve_connection(stream, daemon).await {
                        tracing::debug!(error = %format!("{err:#}"), "hoardd: connection ended");
                    }
                });
            }
            Err(err) => {
                tracing::warn!(error = %format!("{err:#}"), "hoardd: accept failed");
                // Un accept que falla en bucle (fd agotados) no debe quemar CPU.
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

/// Serves one connection: handshake, then requests until the client closes.
pub async fn serve_connection<S>(stream: S, daemon: Arc<Daemon>) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (out_tx, mut out_rx) = mpsc::channel::<ServerFrame>(OUTBOX);

    // The single writer: responses and pushes go out through here, never in parallel.
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if let Err(err) = write_frame(&mut writer, &frame).await {
                tracing::debug!(error = %format!("{err:#}"), "hoardd: write failed; dropping the client");
                return;
            }
        }
    });

    // The farewell cannot wait for this client to send something: the loop below is
    // blocked on a read, and the shutdown happens with nobody writing. So it lives in
    // its own task, queueing into the same single writer.
    let farewell_task = tokio::spawn({
        let mut farewell = daemon.farewell.subscribe();
        let out = out_tx.clone();
        async move {
            if let Ok(reason) = farewell.recv().await {
                let _ = out.send(ServerFrame::Goodbye { reason }).await;
            }
        }
    });

    let result = handshake_and_serve(&mut reader, &out_tx, &daemon).await;
    farewell_task.abort();
    drop(out_tx);
    let _ = writer_task.await;
    result
}

async fn handshake_and_serve<R>(
    reader: &mut R,
    out: &mpsc::Sender<ServerFrame>,
    daemon: &Arc<Daemon>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let first: Option<ClientFrame> = read_frame(reader).await?;
    let Some(ClientFrame::Hello(hello)) = first else {
        // Nothing is served without a handshake: the protocol starts by saying who
        // you are and which version you speak.
        let _ = out
            .send(ServerFrame::Rejected(Rejected {
                reason: "the first frame must be a hello".to_string(),
                daemon_protocol: PROTOCOL_VERSION,
                daemon_version: daemon.version.clone(),
            }))
            .await;
        return Ok(());
    };
    if !accepts(&hello) {
        tracing::warn!(
            client = %hello.client,
            client_protocol = hello.protocol,
            daemon_protocol = PROTOCOL_VERSION,
            "hoardd: rejected a client speaking another protocol"
        );
        let _ = out
            .send(ServerFrame::Rejected(Rejected {
                reason: format!(
                    "this daemon speaks protocol {PROTOCOL_VERSION}, the client speaks {}",
                    hello.protocol
                ),
                daemon_protocol: PROTOCOL_VERSION,
                daemon_version: daemon.version.clone(),
            }))
            .await;
        return Ok(());
    }
    // We are shutting down: the truth this client needs is not "hello", it is
    // "goodbye". Otherwise it would take a departing service for a healthy one and
    // relaunch it the moment it lost the socket.
    if let Some(reason) = daemon.farewell_said() {
        tracing::info!(client = %hello.client, "hoardd: a client connected while stopping; sending the farewell");
        let _ = out
            .send(ServerFrame::Goodbye {
                reason: reason.to_string(),
            })
            .await;
        return Ok(());
    }
    tracing::info!(client = %hello.client, protocol = hello.protocol, "hoardd: client connected");
    out.send(ServerFrame::Welcome(daemon.welcome()))
        .await
        .context("sending the welcome")?;

    // Signing up to the push: kept for when the `Subscribe` arrives. `None` until
    // then, so a client that only sends commands pays nothing for it.
    let mut pusher: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(frame) = read_frame::<_, ClientFrame>(reader).await? {
        let ClientFrame::Request { id, request } = frame else {
            // A second hello is noise; ignoring it is kinder than hanging up.
            continue;
        };
        match request {
            Request::Shutdown => {
                tracing::info!("hoardd: shutdown requested over IPC");
                let _ = out
                    .send(ServerFrame::Reply {
                        id,
                        reply: Reply::Ok(Payload::Ack),
                    })
                    .await;
                daemon.shutdown.notify_one();
                break;
            }
            Request::Subscribe { since } => {
                // The order matters: the push channel is opened first, then the
                // backlog is read. The other way round, an event happening between the
                // two would appear in neither the backlog nor the push, which is the
                // silent gap this design exists to close.
                let rx = daemon.log.subscribe();
                let backlog = daemon.log.since(since.unwrap_or(0));
                let cursor = backlog.cursor;
                if backlog.gap {
                    tracing::info!(
                        requested = since.unwrap_or(0),
                        cursor,
                        "hoardd: a client asked for journal rows we no longer have"
                    );
                }
                let _ = out
                    .send(ServerFrame::Reply {
                        id,
                        reply: Reply::Ok(Payload::Backlog(backlog)),
                    })
                    .await;
                if let Some(old) = pusher.replace(tokio::spawn(push_loop(
                    rx,
                    out.clone(),
                    cursor,
                    daemon.log.clone(),
                ))) {
                    // Re-subscribing replaces the previous subscription; two pushers
                    // per connection would double every event.
                    old.abort();
                }
            }
            other => {
                let reply = daemon.dispatch(other).await;
                if out.send(ServerFrame::Reply { id, reply }).await.is_err() {
                    break;
                }
            }
        }
    }

    if let Some(task) = pusher {
        task.abort();
    }
    Ok(())
}

/// Do we speak the same protocol? Today it is strict equality. When there is a
/// version 2, this is the place to decide which old versions are still served: the
/// handshake exists precisely so that can be done without the client seeing a parse
/// error.
fn accepts(hello: &Hello) -> bool {
    hello.protocol == PROTOCOL_VERSION
}

/// Forwards new journal rows to the client. It skips what was already in the
/// backlog (by `seq`) and, when the client lags far enough that the channel drops
/// rows, sends it a `Resync` instead of leaving it an invisible gap.
async fn push_loop(
    mut rx: broadcast::Receiver<JournalEntry>,
    out: mpsc::Sender<ServerFrame>,
    mut cursor: u64,
    log: Arc<EventLog>,
) {
    loop {
        match rx.recv().await {
            Ok(entry) => {
                if entry.seq <= cursor {
                    continue;
                }
                cursor = entry.seq;
                if out.send(ServerFrame::Event(entry)).await.is_err() {
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(dropped)) => {
                let _ = out
                    .send(ServerFrame::Resync {
                        cursor: log.cursor(),
                        dropped,
                    })
                    .await;
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}
