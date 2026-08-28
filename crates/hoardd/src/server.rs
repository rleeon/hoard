//! El servidor IPC: handshake, despacho de peticiones y push de eventos.
//!
//! Una task por conexión, y dentro de ella dos mitades: el lector de peticiones
//! y un escritor único alimentado por un canal. Que **todo** lo que sale pase por
//! un solo escritor es lo que permite que una respuesta y un push del journal no
//! se entrelacen a media trama.
//!
//! ## Qué pasa si una conexión se cae (o entra en pánico)
//!
//! Se cae **esa** conexión y nada más. El bucle de accept va bajo
//! `supervisor::supervise` (regla de D.12: si vive más que una petición, va
//! supervisado), y el `panic hook` del daemon manda cualquier pánico al log, así
//! que una conexión que muere deja rastro. No se supervisa *cada conexión*
//! porque reiniciar el cuerpo de una conexión cuyo socket ya no existe no
//! significa nada: el cliente reconecta y, gracias al cursor del journal,
//! recupera lo que se perdió sin agujeros.

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

/// Tramas de salida en cola por conexión. Un cliente que no lee se corta cuando
/// se llena, en vez de hacer crecer la memoria del daemon.
const OUTBOX: usize = 512;

/// Despedidas en cola. Sólo se manda una en toda la vida del proceso; el canal
/// existe para repartirla a las conexiones vivas, no para acumular.
const FAREWELL_CHANNEL: usize = 1;

/// Estado compartido que ve cada conexión.
pub struct Daemon {
    pub version: String,
    pub pid: u32,
    /// Identidad de esta ejecución: los `seq` del journal sólo son comparables
    /// dentro de un mismo epoch.
    pub epoch: String,
    pub started: Instant,
    pub log: Arc<EventLog>,
    pub engine: Engine,
    /// Se dispara con `Request::Shutdown`; `main` lo espera.
    shutdown: tokio::sync::Notify,
    /// Reparte la despedida a las conexiones vivas cuando el apagado es
    /// deliberado. Cada conexión tiene una tarea esperando aquí que mete el
    /// [`ServerFrame::Goodbye`] en su cola de salida.
    farewell: broadcast::Sender<String>,
    /// El motivo, una vez dicho. El canal sólo alcanza a quien ya estaba
    /// conectado; esto alcanza a **quien llegue después**, durante el rato que
    /// tardamos en apagarnos (el motor manda su último latido por red, así que
    /// no es instantáneo). Sin ello, un cliente que conectara en esa ventana
    /// recibiría un saludo normal, daría por buena la despedida anterior y
    /// relanzaría el servicio al perder el socket: el bug entero otra vez.
    said: std::sync::OnceLock<String>,
    /// La actualización automática. El daemon no la conduce —eso es
    /// [`crate::updater::watch`]—, sólo la enseña y le pasa lo que piden los
    /// clientes.
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

    /// Despídete de todo cliente enganchado: **esto es un apagado deliberado**.
    ///
    /// Se manda antes de tocar el motor, y el llamante le da un respiro al socket
    /// para que salga (ver `run`). Un daemon que muere de verdad no pasa por
    /// aquí, y eso es exactamente la distinción que el cliente necesita para
    /// decidir si relanzarlo (ADR 0021 D.17 → 4d).
    pub fn say_goodbye(&self, reason: &str) {
        let _ = self.said.set(reason.to_string());
        let listeners = self.farewell.send(reason.to_string()).unwrap_or(0);
        tracing::info!(reason, listeners, "hoardd: saying goodbye to its clients");
    }

    /// El motivo de la despedida si ya la dijimos. Lo consulta el handshake: a
    /// quien llegue después del adiós se le contesta con el adiós, no con un
    /// saludo que dentro de un segundo será mentira.
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
        // Los slots son la verdad viva del motor, así que se preguntan en vez de
        // servir un contador guardado que puede haber quedado atrás.
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

    /// Guarda la sesión que un cliente acaba de acuñar. **La única escritura del
    /// par de tokens en todo el sistema**, junto con la del refresher.
    ///
    /// Va a `spawn_blocking` porque el llavero es síncrono: aunque ya está
    /// acotado (`KEYRING_TIMEOUT`), bloquea el hilo mientras espera, y ese hilo
    /// aquí es el de una conexión IPC — con un llavero lento se quedarían
    /// esperando también las demás peticiones de ese cliente (D.19).
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

    /// Olvida la sesión Cloud. `spawn_blocking` por lo mismo que
    /// [`Daemon::adopt_session`].
    async fn forget_session(&self) -> Result<()> {
        tokio::task::spawn_blocking(hoard_agent::cloud_auth::clear_session)
            .await
            .context("clearing the Cloud session")?
    }

    /// Guarda la sesión self-hosted que un cliente acaba de validar. El gemelo de
    /// [`Daemon::adopt_session`], y `spawn_blocking` por el mismo motivo.
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

    /// Olvida la sesión self-hosted.
    async fn forget_server_session(&self) -> Result<()> {
        tokio::task::spawn_blocking(hoard_agent::credentials::clear)
            .await
            .context("clearing the self-hosted session")?
    }

    /// Presta la sesión self-hosted. No rota nada (un token `hoard_v1_` es
    /// estático), así que es sólo leer el almacén — pero leerlo **aquí**, que es
    /// donde no hay que autorizar nada.
    async fn lend_server_session(&self) -> Result<Option<hoard_core::ipc::ServerSession>> {
        tokio::task::spawn_blocking(hoard_agent::session::lend_server_session)
            .await
            .context("reading the self-hosted session")?
    }

    /// Despacha todo menos `Subscribe` (que necesita la conexión) y `Shutdown`
    /// (que la dispara). Cada comando del motor es un envío al `AgentHandle`: lo
    /// que pasa después llega por el journal, no por la respuesta.
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
            // Tampoco pasa por `with_engine`, y es load-bearing: el rotador del
            // token es **del daemon**, no del motor. Un motor caído por falta de
            // sesión o por un bache de red no puede dejar al desktop sin poder
            // hablar con la nube — y menos aún empujarle a rotar por su cuenta,
            // que es justo lo que este slice viene a matar.
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
            // Tampoco pasa por `with_engine`, y por el mismo motivo que
            // `CloudToken`: guardar la sesión es del daemon, no del motor. Es
            // más: el motor está caído *precisamente* porque no había sesión, y
            // esto es lo que lo arregla.
            Request::AdoptSession { session } => {
                match self.adopt_session(session).await {
                    Ok(()) => {
                        tracing::info!("hoardd: adopted a Cloud session handed over by a client");
                        // Aprender una sesión nueva es un cambio de sesión: el
                        // motor que hubiera está hablando con la anterior.
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
            // Como `CloudToken`: es del daemon, no del motor. Un motor caído no
            // puede dejar a la app sin poder hablar con su propio server.
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
            // No pasa por `with_engine`: reiniciar un motor caído es
            // precisamente lo que puede hacer que vuelva (el keeper resuelve la
            // sesión otra vez), así que un `EngineDown` aquí sería contestar
            // "no puedo arreglarlo porque está roto".
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
            // Cómo va la actualización. No pasa por el motor: el updater es del
            // daemon, y un motor caído —que suele ser justo el caso en que
            // actualizar arregla algo— no puede dejar a nadie sin saberlo.
            Request::UpdateStatus => Reply::Ok(Payload::Update(self.updater.state())),
            // Aplicar **ahora**. Vuelve al momento con el estado de este
            // instante: aplicar puede tardar (un instalador nativo, un diálogo
            // de polkit esperando a un humano) y dejar una petición IPC colgada
            // todo ese rato bloquearía las demás de ese cliente. Quien pregunta
            // vuelve a preguntar por `UpdateStatus` y ve la fase avanzar.
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
            // Una petición de un cliente más nuevo que este servicio. Se
            // contesta, no se tira la conexión: el cliente acaba de
            // actualizarse y a nosotros nos queda un relevo de segundos.
            Request::Unknown => Reply::Error(IpcError::Unsupported {
                op: "an operation this version of the Hoard service doesn't know".to_string(),
            }),
            // Las dos que no llegan aquí.
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

    /// Un comando que no llega al motor casi siempre significa que el motor se
    /// fue (canal cerrado), así que se reporta como `EngineDown` con el motivo
    /// que el keeper haya registrado, no como un `Internal` opaco.
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

/// Acepta conexiones para siempre. No retorna (no puede producir un `Finished`),
/// así que `supervise` sólo lo reinicia por pánico y `main` lo mata abortando.
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

/// Atiende una conexión: handshake, luego peticiones hasta que el cliente cierre.
pub async fn serve_connection<S>(stream: S, daemon: Arc<Daemon>) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (out_tx, mut out_rx) = mpsc::channel::<ServerFrame>(OUTBOX);

    // Escritor único: respuestas y pushes salen por aquí, nunca en paralelo.
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if let Err(err) = write_frame(&mut writer, &frame).await {
                tracing::debug!(error = %format!("{err:#}"), "hoardd: write failed; dropping the client");
                return;
            }
        }
    });

    // La despedida no puede esperar a que este cliente mande algo: el bucle de
    // abajo está bloqueado leyendo, y el apagado ocurre sin que nadie escriba.
    // Por eso va en su propia tarea, encolando en el mismo escritor único.
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
        // Sin handshake no se atiende nada: el protocolo empieza por decir quién
        // eres y qué versión hablas.
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
    // Nos estamos apagando: la verdad que este cliente necesita no es "hola",
    // es "adiós" — si no, tomaría por sano un servicio que se va y lo relanzaría
    // en cuanto perdiera el socket.
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

    // Alta en el push: se guarda para cuando llegue el `Subscribe`. `None` hasta
    // entonces — un cliente que sólo manda comandos no paga el coste.
    let mut pusher: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(frame) = read_frame::<_, ClientFrame>(reader).await? {
        let ClientFrame::Request { id, request } = frame else {
            // Un segundo hello es ruido; ignorarlo es más amable que cortar.
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
                // El orden importa: primero se abre el canal de push, luego se
                // lee el backlog. Al revés, un evento que ocurriese entre las dos
                // cosas no aparecería ni en el backlog ni en el push — el hueco
                // silencioso que este diseño existe para cerrar.
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
                    // Re-suscribirse reemplaza la suscripción anterior; dos
                    // pushers por conexión duplicarían cada evento.
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

/// ¿Hablamos el mismo protocolo? Hoy es igualdad estricta. Cuando haya una
/// versión 2 este es el sitio donde decidir qué versiones viejas se siguen
/// atendiendo — el handshake existe precisamente para poder hacerlo sin que el
/// cliente vea un error de parseo.
fn accepts(hello: &Hello) -> bool {
    hello.protocol == PROTOCOL_VERSION
}

/// Reenvía filas nuevas del journal al cliente. Salta lo que ya iba en el
/// backlog (por `seq`) y, si el cliente se retrasa tanto que el canal descarta
/// filas, le manda un `Resync` en vez de dejarle un hueco invisible.
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
