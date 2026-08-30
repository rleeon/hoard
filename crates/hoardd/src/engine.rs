//! El motor dentro del daemon: arrancarlo, mantenerlo vivo y bombear sus
//! eventos al journal.
//!
//! Un motor por usuario, propiedad de este proceso. Tres piezas:
//!
//! - [`Engine`]: la ranura compartida. El servidor IPC pregunta por el
//!   `AgentHandle` y por el estado; nunca arranca ni para el motor por su cuenta.
//! - [`keeper`]: bucle supervisado que **asegura** que el motor está arriba —
//!   resuelve la sesión, hace `agent::spawn`, y si el motor muere lo vuelve a
//!   levantar. Nada aquí puede morir en silencio (D.12): el keeper detecta un
//!   `JoinHandle` terminado en vez de fiarse de un booleano, que es exactamente
//!   cómo se quedó clavado el gate del poller.
//! - [`pump`]: bucle supervisado que consume `AgentEvent`s, los persiste en
//!   `state.json` y los mete en el journal (que es quien empuja a los clientes).
//!
//! ## El pidfile, muerto (Slice 4d)
//!
//! Mientras el desktop (4b) y `hoard sync` (4c) embebían `agent::spawn`, el
//! árbitro entre daemon y motor embebido era un pidfile (`agent.pid`,
//! `hoard_agent::instance`): el keeper lo consultaba y, si otro lo tenía tomado,
//! no arrancaba motor. El 4c le quitó el chequeo (aceptaba como dueño vivo
//! cualquier proceso cuyo nombre contuviera "hoard", y desde el 4b todos los
//! clientes lo contienen) y este slice borra el fichero entero: **el árbitro es
//! la propiedad del socket**, un mutex con liveness real que el kernel suelta al
//! morir el proceso, no un fichero que hay que adivinar si miente.
//!
//! Por eso [`Running`] ya no guarda ningún lock: lo único que impide dos motores
//! es que sólo hay un daemon, y de eso responde el bind (`transport::Listener`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hoard_agent::agent::{self, AgentConfig, AgentEvent, AgentHandle};
use hoard_agent::api::ApiClient;
use hoard_agent::config::CliConfig;
use hoard_agent::prefs::Prefs;
use hoard_agent::presence::PresenceHandle;
use hoard_agent::state::CliState;
use hoard_agent::supervisor::Finished;
use hoard_agent::{cloud_live, library, presence};
use hoard_core::ipc::{EngineDownReason, EngineStatus, KeyringFault};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::journal::EventLog;

/// Cada cuánto comprueba el keeper que el motor sigue vivo.
const KEEPER_TICK: Duration = Duration::from_secs(5);

/// Backoff tras un arranque fallido del motor (sin sesión, red caída).
const START_BACKOFF_MIN: Duration = Duration::from_secs(5);
const START_BACKOFF_MAX: Duration = Duration::from_secs(5 * 60);

/// Cada cuánto se comprueba que las tareas del empuje Cloud siguen vivas.
const CLOUD_LIVE_CHECK: Duration = Duration::from_secs(15);

/// Margen que se le da al motor para atender su `shutdown` antes de abortarlo.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// Pasado esto, una transferencia "en vuelo" se da por muerta. Ver
/// [`Engine::transfers_in_flight`]: es el seguro contra un descuadre del
/// contador, no un tiempo de espera de red.
const TRANSFER_STALE: Duration = Duration::from_secs(30 * 60);

/// Espera máxima a que el motor conteste una consulta de estado. Sin tope, un
/// motor atascado colgaría al cliente que preguntó (y a la UI detrás).
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Aborta un grupo de tareas al soltarse. Sin esto, reiniciar el empuje Cloud
/// dejaría las tareas viejas corriendo: dos pollers es justo el fallo que D.12
/// documenta.
struct AbortOnDrop(Vec<JoinHandle<()>>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

/// Lo que compone un motor vivo. Todo se tira junto.
struct Running {
    handle: AgentHandle,
    task: JoinHandle<()>,
    presence: PresenceHandle,
    /// El cliente del motor vivo. `Reload` reconstruye el conjunto vigilado y
    /// para eso necesita volver a preguntar qué saves están archivados: sin
    /// esto, archivar una partida no surtiría efecto hasta el siguiente
    /// arranque. Comparte la celda del token con el resto de clones, así que
    /// el JWT que rote el refresher también vale aquí.
    client: ApiClient,
    /// Tareas auxiliares del motor (presencia, empuje Cloud, refresher del JWT).
    aux: Vec<JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        // Soltar un `JoinHandle` **no** cancela su task: detachearlas dejaría el
        // poller, el latido de presencia y el rotador del token del motor viejo
        // corriendo junto a los del nuevo. Dos pollers y dos rotadores del mismo
        // refresh token es la familia de bugs que D.12 y la Parte A documentan, así
        // que morir del todo es parte del contrato de este tipo.
        for task in &self.aux {
            task.abort();
        }
        self.task.abort();
    }
}

#[derive(Default)]
struct Inner {
    running: Option<Running>,
    status: EngineStatus,
    /// Paramos a propósito (`Request::Shutdown`): el keeper no debe resucitarlo.
    stopping: bool,
    /// Un cliente pidió levantar el motor de cero (cambió la sesión en disco), y
    /// por qué. Lo atiende el keeper, que es el único dueño del ciclo de vida.
    restart_requested: Option<String>,
    /// Copias y restauraciones empezadas y todavía sin desenlace.
    ///
    /// Lo lleva la bomba de eventos, que es por donde pasan todas, y lo lee el
    /// updater: relevar los binarios con una subida a medias mata el proceso que
    /// la estaba haciendo y deja un blob a medio comprometer en el server. Es el
    /// único freno que ni siquiera el plazo levanta.
    ///
    /// Vive aquí y no en el motor porque tiene que **sobrevivir a un reinicio
    /// del motor**: si se fuera con él, un motor que rebota en mitad de una
    /// subida dejaría el contador clavado en 1 para siempre y el updater no
    /// volvería a aplicar nada. Al arrancar un motor nuevo se pone a cero (ver
    /// [`Engine::transfers_reset`]), que es la verdad: lo que hubiera en vuelo se
    /// fue con el proceso anterior.
    in_flight: usize,
    /// Desde cuándo hay algo en vuelo. Lo que hace caducar el contador.
    in_flight_since: Option<Instant>,
}

/// Ranura compartida del motor. Cheap to clone.
#[derive(Clone, Default)]
pub struct Engine {
    inner: Arc<Mutex<Inner>>,
    /// Despierta al keeper de su espera. Sin esto, un motor caído tras varios
    /// fallos duerme hasta [`START_BACKOFF_MAX`], y un login recién hecho
    /// tardaría hasta cinco minutos en sincronizar aunque el cliente ya haya
    /// avisado. `Notify` guarda el permiso si nadie escucha, así que un aviso
    /// que llega mientras el keeper trabaja no se pierde.
    wake: Arc<tokio::sync::Notify>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle del motor, si está arriba.
    pub fn handle(&self) -> Option<AgentHandle> {
        self.lock().running.as_ref().map(|r| r.handle.clone())
    }

    pub fn presence(&self) -> Option<PresenceHandle> {
        self.lock().running.as_ref().map(|r| r.presence.clone())
    }

    /// Cliente del motor vivo, si lo hay.
    pub fn client(&self) -> Option<ApiClient> {
        self.lock().running.as_ref().map(|r| r.client.clone())
    }

    pub fn status(&self) -> EngineStatus {
        self.lock().status.clone()
    }

    /// Motivo legible de que no haya motor, para el `IpcError::EngineDown` que
    /// recibe el cliente. Un cliente que sólo ve "error" reintenta para siempre
    /// sin poder decirle nada al usuario.
    pub fn down_reason(&self) -> String {
        let guard = self.lock();
        if guard.stopping {
            return "the daemon is shutting down".to_string();
        }
        guard
            .status
            .last_error
            .clone()
            .unwrap_or_else(|| "the engine is still starting".to_string())
    }

    pub fn set_watched(&self, count: usize) {
        self.lock().status.watched = count;
    }

    /// Empieza una transferencia (copia o restauración).
    pub fn transfer_started(&self) {
        let mut guard = self.lock();
        if guard.in_flight == 0 {
            guard.in_flight_since = Some(Instant::now());
        }
        guard.in_flight += 1;
    }

    /// Termina una transferencia, bien o mal. Satura en 0: un desenlace sin
    /// comienzo (el motor arrancó con una subida ya en vuelo, D.8.3) no puede
    /// dejar el contador en negativo y bloquear el updater para siempre.
    pub fn transfer_finished(&self) {
        let mut guard = self.lock();
        guard.in_flight = guard.in_flight.saturating_sub(1);
        if guard.in_flight == 0 {
            guard.in_flight_since = None;
        }
    }

    /// Olvida lo que hubiera en vuelo. Lo llama el keeper al levantar un motor
    /// nuevo: lo que estuviera a medias murió con el anterior.
    pub fn transfers_reset(&self) {
        let mut guard = self.lock();
        guard.in_flight = 0;
        guard.in_flight_since = None;
    }

    /// ¿Hay algo a medias ahora mismo?
    ///
    /// **Caduca.** El contador se lleva emparejando eventos, y emparejar eventos
    /// es exactamente la clase de cuenta que se descuadra en cuanto alguien
    /// añade una variante terminal y no la suma aquí. Un descuadre de uno
    /// bloquearía el updater **para siempre** en silencio —el mismo fallo que el
    /// gate sin RAII del poller (D.10)—, así que pasado [`TRANSFER_STALE`] se da
    /// por acabado lo que fuera. Ninguna copia legítima dura tanto.
    pub fn transfers_in_flight(&self) -> bool {
        let guard = self.lock();
        match guard.in_flight_since {
            Some(since) if guard.in_flight > 0 => since.elapsed() < TRANSFER_STALE,
            _ => false,
        }
    }

    /// Marca que no habrá motor y por qué (`--no-engine`). El motivo viaja al
    /// cliente en `EngineDown`: un motor ausente **a propósito** tiene que
    /// distinguirse de uno que no arranca.
    pub fn disable(&self, reason: &str) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.status.last_error = Some(reason.to_string());
    }

    pub fn stopping(&self) -> bool {
        self.lock().stopping
    }

    /// ¿Motor arriba **y** su task viva? La segunda mitad importa: un pánico
    /// dentro del bucle del agente deja el handle intacto y los comandos se
    /// tragarían en un canal que nadie lee.
    pub fn alive(&self) -> bool {
        self.lock()
            .running
            .as_ref()
            .is_some_and(|r| !r.task.is_finished())
    }

    fn install(&self, running: Running, server: String, is_cloud: bool, watched: usize) {
        let mut guard = self.lock();
        guard.status = EngineStatus {
            running: true,
            server: Some(server),
            is_cloud,
            watched,
            since: Some(OffsetDateTime::now_utc()),
            last_error: None,
            reason: EngineDownReason::Unknown,
            keyring: None,
        };
        // Un motor previo (por ejemplo el que murió y estamos reemplazando) se
        // tira aquí: `Running::aux` aborta sus tareas al soltarse.
        guard.running = Some(running);
        // Lo que estuviera a medias se fue con el motor anterior. Sin esto, un
        // motor que rebota en mitad de una subida deja el contador clavado y el
        // updater no vuelve a aplicar nada.
        guard.in_flight = 0;
        guard.in_flight_since = None;
    }

    fn note_error(&self, error: String, reason: EngineDownReason, keyring: Option<KeyringFault>) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.status.last_error = Some(error);
        guard.status.reason = reason;
        // Which way the keyring failed, when that's what failed. Cleared
        // otherwise, or a machine that once hit a locked keyring would keep
        // explaining every later failure with it.
        guard.status.keyring = keyring;
        guard.running = None;
    }

    /// Suelta un motor que ya está muerto **antes** de arrancar otro: su `Drop`
    /// aborta las tareas auxiliares (rotador del token, poller, presencia), y
    /// dos juegos de ésas vivos a la vez es la familia 401 que este slice mata.
    fn forget(&self) {
        let mut guard = self.lock();
        guard.status.running = false;
        guard.running = None;
    }

    /// Espera `for_` o hasta que alguien pida atención, lo que llegue antes.
    async fn nap(&self, for_: Duration) {
        tokio::select! {
            _ = tokio::time::sleep(for_) => {}
            _ = self.wake.notified() => {}
        }
    }

    /// Pide que el motor se levante de cero con la sesión que haya ahora en
    /// disco. Responde a [`hoard_core::ipc::Request::RestartEngine`].
    ///
    /// **Lo pide, no lo hace.** El único que arranca y para el motor es el
    /// keeper: si el reinicio se ejecutara aquí, entre soltar el motor viejo y
    /// terminar su apagado el keeper vería la ranura vacía y arrancaría otro —
    /// y durante esa ventana habría dos motores en el mismo proceso, con dos
    /// rotadores del mismo refresh token. Dejarlo en una petición mantiene un
    /// solo dueño del ciclo de vida.
    ///
    /// El aviso vale aunque no haya motor: el caso típico es justo ése —no
    /// arrancaba por falta de sesión y el usuario acaba de entrar—, y esperar el
    /// backoff sería no enterarse.
    pub fn request_restart(&self, reason: &str) {
        self.lock().restart_requested = Some(reason.to_string());
        self.wake.notify_one();
    }

    /// A session was signed out. Restart only if it was *this* engine's.
    ///
    /// The two sessions are independent — a machine can hold a Cloud one and a
    /// self-hosted one at once — but the engine runs against exactly one of
    /// them, and dropping the other changes nothing it is doing. On 2026-08-28
    /// the desktop forgot the self-hosted session five seconds after the engine
    /// had finally come up on Cloud, and the engine was torn down and rebuilt
    /// for it: a second "watching…" for every save, and a gap in the middle of
    /// a sync that had nothing to do with the session that went.
    ///
    /// A engine that is *down* is restarted either way: the session that is left
    /// may be the one it was missing.
    pub fn request_restart_if_signed_out(&self, was_cloud: bool, reason: &str) {
        let mine = {
            let guard = self.lock();
            !guard.status.running || guard.status.is_cloud == was_cloud
        };
        if mine {
            self.request_restart(reason);
        } else {
            tracing::info!(
                signed_out_cloud = was_cloud,
                "hoardd: a session was signed out, but not the one the engine runs on — leaving it alone"
            );
        }
    }

    /// The session can be read *right now* — somebody just did it. Wake a
    /// engine that is down because it couldn't.
    ///
    /// The backoff after a failed start is five minutes, which is the right
    /// pace for a keyring that keeps refusing and the wrong one for a keyring
    /// that has started answering: on 2026-08-28 the desktop opened at 05:34:48
    /// and lent a Cloud token successfully, and the engine — down since 05:31:08
    /// for not being able to read that same session — slept until 05:36:10, its
    /// backoff to the second. Eighty-two seconds of "the sync service is
    /// stopped" with the session sitting there, readable.
    ///
    /// Gated on the reason so this can't turn into a retry loop: only the three
    /// session faults are unblocked by a session that reads, and the caller only
    /// calls after a read that worked. A keyring still refusing fails the lend
    /// first and never gets here.
    pub fn wake_if_a_session_would_help(&self) {
        let reason = {
            let guard = self.lock();
            if guard.status.running {
                return;
            }
            guard.status.reason
        };
        if matches!(
            reason,
            EngineDownReason::NoSession
                | EngineDownReason::KeyringUnreadable
                | EngineDownReason::SessionExpired
        ) {
            tracing::info!(
                ?reason,
                "hoardd: the session reads again — waking the engine instead of waiting out its backoff"
            );
            self.request_restart("the session became readable");
        }
    }

    fn take_restart_request(&self) -> Option<String> {
        self.lock().restart_requested.take()
    }

    /// Apagado limpio del motor vivo para volver a arrancarlo. Sólo el keeper.
    async fn stop_for_restart(&self, reason: &str) {
        let running = {
            let mut guard = self.lock();
            let taken = guard.running.take();
            if taken.is_some() {
                guard.status.running = false;
                guard.status.last_error = Some(reason.to_string());
            }
            taken
        };
        let Some(mut running) = running else { return };
        tracing::info!(reason, "hoardd: restarting the engine");
        // Último latido de presencia con el token viejo, que aún vale: deja este
        // equipo en gris en el panel de las otras máquinas en vez de que se
        // apague sin decir nada.
        running.presence.closing().await;
        if let Err(err) = running.handle.shutdown().await {
            tracing::warn!(error = %err, "hoardd: the engine didn't acknowledge the restart");
        }
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut running.task)
            .await
            .is_err()
        {
            tracing::warn!("hoardd: the engine didn't stop in time; aborting it");
        }
        // Al soltar `running` aquí se abortan sus tareas auxiliares, así que el
        // arranque siguiente no convive con el anterior.
    }

    /// Para el motor. Marca `stopping` **antes** de nada para que el keeper no lo
    /// resucite mientras se apaga.
    pub async fn shutdown(&self) {
        let running = {
            let mut guard = self.lock();
            guard.stopping = true;
            guard.status.running = false;
            guard.running.take()
        };
        let Some(mut running) = running else { return };
        // Último latido de presencia mientras el token vale: pone este equipo en
        // gris en el panel de las otras máquinas al instante.
        running.presence.closing().await;
        if let Err(err) = running.handle.shutdown().await {
            tracing::warn!(error = %err, "hoardd: the engine didn't acknowledge shutdown");
        }
        // `shutdown` sólo *manda* el comando: hay que darle al bucle del agente la
        // vuelta que necesita para atenderlo, o el `abort` del `Drop` lo cortaría
        // a media limpieza. Acotado, para que un motor colgado no bloquee el
        // apagado del servicio.
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut running.task)
            .await
            .is_err()
        {
            tracing::warn!("hoardd: the engine didn't stop in time; aborting it");
        }
        // Al soltarse `running` se abortan sus tareas auxiliares: nada del motor
        // sobrevive al apagado del servicio.
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Igual que en el journal: un pánico ajeno no puede dejar la ranura del
        // motor inaccesible para siempre.
        self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::error!("hoardd: the engine mutex was poisoned; recovering");
            poisoned.into_inner()
        })
    }
}

/// Bucle que mantiene el motor arriba. No retorna nunca (por eso no puede
/// producir un [`Finished`] por accidente); `supervise` lo reinicia si entra en
/// pánico y el `abort()` del apagado lo mata.
pub async fn keeper(engine: Engine, events_tx: mpsc::Sender<AgentEvent>) -> Finished {
    let mut backoff = START_BACKOFF_MIN;
    loop {
        if engine.stopping() {
            // No se retorna `Finished`: apagar es cosa de `main`, que aborta esta
            // task. Dormir aquí evita quemar CPU en la ventana del apagado.
            tokio::time::sleep(KEEPER_TICK).await;
            continue;
        }
        // Un cambio de sesión pedido por un cliente. Se atiende **antes** que
        // nada: el motor vivo que haya está hablando con la cuenta que ya no es.
        // Sin `continue`, para caer en el arranque de abajo en la misma vuelta.
        if let Some(reason) = engine.take_restart_request() {
            engine.stop_for_restart(&reason).await;
            backoff = START_BACKOFF_MIN;
        }
        if engine.alive() {
            engine.nap(KEEPER_TICK).await;
            continue;
        }
        if engine.status().running {
            // Estaba arriba y su task ha muerto: eso es un incidente, no una
            // transición normal.
            tracing::error!("hoardd: the engine task is gone; restarting it");
            // Suelta el cadáver (y sus tareas) antes de intentar otro arranque.
            engine.forget();
        }
        match start(events_tx.clone()).await {
            Ok(started) => {
                tracing::info!(
                    server = %started.server,
                    watched = started.watched,
                    "hoardd: engine up"
                );
                engine.install(
                    started.running,
                    started.server,
                    started.is_cloud,
                    started.watched,
                );
                backoff = START_BACKOFF_MIN;
            }
            Err(err) => {
                let text = format!("{err:#}");
                let reason = classify(&err);
                let keyring = hoard_agent::keychain::fault(&err);
                tracing::warn!(
                    error = %text,
                    ?reason,
                    keyring = keyring.map(|f| f.as_str()).unwrap_or("-"),
                    retry_in_secs = backoff.as_secs(),
                    "hoardd: couldn't start the engine"
                );
                engine.note_error(text, reason, keyring);
                engine.nap(backoff).await;
                backoff = (backoff * 2).min(START_BACKOFF_MAX);
            }
        }
    }
}

/// Por qué no arrancó, para que la ventana pueda decirlo.
///
/// **Por downcast, nunca por el texto del error.** Un mensaje se reescribe sin
/// pensar —y este en concreto se ha reescrito ya— y con `contains("no session")`
/// la clasificación se rompería en silencio, que es justo el fallo invisible que
/// todo esto viene a matar. Cada rama cuelga de un tipo que existe precisamente
/// para ser reconocido aquí.
fn classify(err: &anyhow::Error) -> EngineDownReason {
    if err
        .downcast_ref::<hoard_agent::session::NoSession>()
        .is_some()
    {
        return EngineDownReason::NoSession;
    }
    // El llavero tiene dos formas de fallar (no contesta / contesta que no) y un
    // solo consejo para el usuario: vuelve a entrar, que reescribe el ítem a
    // nombre del servicio. Se separan en el log, no en la pantalla.
    if err
        .downcast_ref::<hoard_agent::keychain::KeyringTimeout>()
        .is_some()
        || err
            .downcast_ref::<hoard_agent::keychain::KeyringUnreadable>()
            .is_some()
    {
        return EngineDownReason::KeyringUnreadable;
    }
    if hoard_agent::cloud_auth::is_session_expired(err) {
        return EngineDownReason::SessionExpired;
    }
    EngineDownReason::Other
}

struct Started {
    running: Running,
    server: String,
    is_cloud: bool,
    watched: usize,
}

/// Arranca el motor: sesión → saves → `agent::spawn` → presencia, empuje Cloud y
/// refresher del JWT.
async fn start(events_tx: mpsc::Sender<AgentEvent>) -> anyhow::Result<Started> {
    // `resolve_owned`: el camino que rota. Es del servicio y de nadie más — los
    // clientes usan `resolve_borrowed` con el token que les prestamos.
    let active = hoard_agent::session::resolve_owned().await?;
    // Antes de hidratar nada: curar el estado contra el servidor. Es el único
    // punto por el que pasa toda máquina —arranque, login, actualización (el
    // instalador reinicia el servicio)— así que actualizar la app repara sola a
    // quien tenga filas apuntando a ids que su servidor ya no conoce. Un fallo
    // aquí no puede impedir arrancar: sin red el motor sigue teniendo trabajo
    // local que hacer.
    match library::reconcile_with_server(&active.client).await {
        Ok(r) if r.changed() => tracing::info!(
            relinked = r.relinked,
            dropped = r.dropped,
            "hoardd: reconciled tracked saves with the server"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "hoardd: couldn't reconcile with the server"),
    }
    let (state, _path) = CliState::load_default()?;
    let archived = archived_save_ids(&active.client).await;
    // Sin saves rastreados el motor arranca igual (a diferencia de `hoard sync`,
    // que es un comando y puede abortar): un servicio residente tiene que estar
    // ahí cuando el usuario rastree el primero, y `Request::Reload` lo recoge.
    let saves = library::watched_saves_from_state(&state, &archived);
    let watched = saves.len();
    let config = engine_config();

    let (presence_handle, presence_task) = presence::spawn(active.client.clone());
    // Dos clones antes de que `agent::spawn` consuma el cliente. `ApiClient`
    // comparte su celda de token entre clones, así que el JWT que rote el
    // refresher llega también al motor y al empuje Cloud.
    let live_client = active.client.clone();
    let refresh_client = active.client.clone();
    let reload_client = active.client.clone();
    let global_sync = config.global_sync;
    let (handle, task) = agent::spawn(active.client, config, saves, events_tx);

    let mut aux = vec![presence_task];
    // Empuje Cloud de baja latencia (Realtime + poll de respaldo), igual que el
    // daemon CLI. Sólo en Cloud y con sync global: `backup_only` nunca escribe.
    if active.is_cloud && global_sync {
        aux.push(spawn_cloud_live(live_client, handle.clone()));
    }
    // Un solo rotador del refresh token: éste. `owned()` es `Some` sólo porque
    // resolvimos como dueños; un cliente ni siquiera recibe el refresh token.
    if let Some(session) = active.cloud.as_ref().and_then(|c| c.owned()) {
        let shared = Arc::new(tokio::sync::Mutex::new(session));
        aux.push(tokio::spawn(hoard_agent::supervisor::supervise(
            "hoardd cloud refresh",
            move || hoard_agent::session::refresh_loop(refresh_client.clone(), shared.clone()),
        )));
    }

    Ok(Started {
        running: Running {
            handle,
            task,
            presence: presence_handle,
            client: reload_client,
            aux,
        },
        server: active.server,
        is_cloud: active.is_cloud,
        watched,
    })
}

/// Los saves congelados en la caja negra del servidor, para dejarlos fuera del
/// conjunto vigilado.
///
/// **Nunca falla hacia arriba.** Un servidor que no contesta, un self-hosted que
/// no tiene caja negra, una versión vieja sin ese endpoint: todos significan
/// aquí "no sé de ninguno archivado", que es el comportamiento de siempre.
/// Devolver un error en su lugar dejaría al motor sin arrancar por una consulta
/// accesoria, y devolver un conjunto a medias dejaría de vigilar saves que están
/// perfectamente vivos — de los dos errores posibles, vigilar de más es el
/// barato: un save archivado que se cuele lo para el 403 como hasta ahora.
async fn archived_save_ids(client: &ApiClient) -> HashSet<String> {
    if !client.is_cloud().await {
        return HashSet::new();
    }
    match client.cloud_archived_save_ids().await {
        Ok(ids) => {
            if !ids.is_empty() {
                tracing::info!(count = ids.len(), "hoardd: saves archived on the server");
            }
            ids
        }
        Err(err) => {
            tracing::warn!(
                error = %format!("{err:#}"),
                "hoardd: couldn't ask which saves are archived; watching them all"
            );
            HashSet::new()
        }
    }
}

/// Config del motor a partir de las preferencias del usuario.
///
/// Mismas prefs que lee el desktop (`prefs.json` es del usuario, no del
/// frontend), con una excepción deliberada: si **no hay** fichero de prefs, la
/// máquina nunca ha visto la app de escritorio y estamos en el caso headless, que
/// es el que `hoard sync` sirve hoy con sync global y auto-restore encendidos. Un
/// servidor casero que sólo tiene la CLI no puede acabar en "sólo subo" por leer
/// unos defaults pensados para la GUI.
fn engine_config() -> AgentConfig {
    let (prefs, path) = Prefs::load_default()
        .map(|(p, path)| (p, Some(path)))
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "hoardd: couldn't read prefs; using defaults");
            (Prefs::default(), None)
        });
    let headless = path.map(|p| !p.exists()).unwrap_or(false);
    AgentConfig {
        auto_restore: prefs.auto_restore || headless,
        global_sync: prefs.global_sync || headless,
        conflict_retention_days: prefs.conflict_retention_days,
        // `data_saving` deliberately does NOT feed the floor any more. Its
        // slider left the UI on 2026-06-14 ("el backend mantiene sus defaults"),
        // but the pref already written to disk stayed at whatever the user had
        // last dragged it to — and the engine kept honouring it. On one machine
        // that was 1.0, a ten-minute floor between uploads that nothing could
        // show or change: edits were picked up in two seconds and then sat in
        // the queue, which reads as "it doesn't detect my changes". Worse, a
        // restore marks the next backup urgent and skips the floor, so changes
        // arriving from the other machine synced instantly while your own
        // waited — the two halves looked unrelated.
        //
        // Per-save pacing is still reachable where it is visible: the
        // `data_saver` preset sets its own 600s floor through
        // `SavePolicy::min_snapshot_interval_secs`, and that one the user picks
        // per game and can see.
        min_snapshot_interval_secs: 0,
        // Aparca la copia local antes de dejar que una remota más nueva la pise
        // (nunca destruye datos en silencio).
        conflict_root: CliConfig::state_dir().ok().map(|d| d.join("conflicts")),
        ..AgentConfig::default()
    }
}

/// Empuje Cloud bajo supervisión. `cloud_live::spawn` monta dos `tokio::spawn`
/// sueltos (poll + Realtime) que sobreviven a errores pero no a un pánico, así
/// que el keeper lo cubre desde fuera: si alguna de las dos tareas termina, se
/// tira el par y se rearma.
///
/// Desde el Slice 4c éste es su **único** llamante (el daemon CLI ya no existe),
/// así que la supervisión se puede meter dentro de `cloud_live` sin romper a
/// nadie. Se deja para el Slice 7 (cliente cloud único), que va a tocar esa
/// función entera; envolverla desde fuera ya cumple la regla de D.12.
fn spawn_cloud_live(client: ApiClient, handle: AgentHandle) -> JoinHandle<()> {
    tokio::spawn(hoard_agent::supervisor::supervise(
        "hoardd cloud-live",
        move || {
            let client = client.clone();
            let handle = handle.clone();
            async move {
                let mut tasks = AbortOnDrop(spawn_cloud_live_pair(&client, &handle));
                loop {
                    tokio::time::sleep(CLOUD_LIVE_CHECK).await;
                    if tasks.0.iter().any(|t| t.is_finished()) {
                        tracing::warn!("hoardd: a cloud-live task ended; restarting the pair");
                        // La asignación suelta el grupo anterior, que aborta lo que
                        // quedara vivo. Nunca dos pollers.
                        tasks = AbortOnDrop(spawn_cloud_live_pair(&client, &handle));
                    }
                }
            }
        },
    ))
}

fn spawn_cloud_live_pair(client: &ApiClient, handle: &AgentHandle) -> Vec<JoinHandle<()>> {
    cloud_live::spawn(
        client.clone(),
        handle.clone(),
        cloud_live::Config {
            poll_interval: Duration::from_secs(hoard_agent::prefs::CLOUD_POLL_INTERVAL_SECS as u64),
            global_sync: true,
        },
    )
}

/// Estado de los slots vigilados, con tope de espera. Lo usa el `Status` del IPC.
pub async fn slot_status(engine: &Engine) -> Vec<hoard_core::ipc::AgentSlotStatus> {
    let Some(handle) = engine.handle() else {
        return Vec::new();
    };
    match tokio::time::timeout(STATUS_TIMEOUT, handle.status()).await {
        Ok(Ok(slots)) => slots,
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "hoardd: the engine didn't answer a status query");
            Vec::new()
        }
        Err(_) => {
            tracing::warn!("hoardd: the engine took too long to answer a status query");
            Vec::new()
        }
    }
}

/// Bombea los eventos del motor: presencia, `state.json`, journal y aviso
/// nativo.
///
/// El canal es **del daemon**, no del motor: se crea una vez y cada arranque del
/// motor recibe un clon del emisor. Así este bucle puede reiniciarse bajo
/// `supervise` sin perder el receptor, y un motor reiniciado sigue escribiendo en
/// el mismo journal (los cursores de los clientes no se rompen porque el motor
/// haya rebotado).
///
/// Las notificaciones nativas salen de aquí y no del ejecutor de cada acción
/// porque éste es el **único** sitio por el que pasan todos los eventos del
/// motor: un aviso colgado de la rama de backup y otro de la de restore es
/// exactamente cómo el 429 acabó manejado en un camino y no en el otro (D.7).
pub async fn pump(
    engine: Engine,
    log: Arc<EventLog>,
    notifier: Arc<crate::notify::Notifier>,
    events_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<AgentEvent>>>,
) -> Finished {
    let mut rx = events_rx.lock().await;
    while let Some(event) = rx.recv().await {
        // Presencia (panel Eye): espeja las transiciones de juego.
        match &event {
            AgentEvent::GameStarted { game_slug, .. } => {
                if let Some(p) = engine.presence() {
                    p.game_started(game_slug.clone());
                }
            }
            AgentEvent::GameStopped { game_slug, .. } => {
                if let Some(p) = engine.presence() {
                    p.game_stopped(game_slug.clone());
                }
            }
            _ => {}
        }
        // Lo que el updater necesita saber para no relevar los binarios en mitad
        // de una subida. Se cuenta aquí, que es el único sitio por el que pasan
        // **todas** las transferencias — por el mismo motivo por el que se
        // notifica aquí y no en cada rama (D.7).
        match &event {
            AgentEvent::BackupStarted { .. } => engine.transfer_started(),
            AgentEvent::BackupSuccess { .. }
            | AgentEvent::BackupFailed { .. }
            | AgentEvent::BackupThrottled { .. }
            | AgentEvent::BackupTooLarge { .. }
            | AgentEvent::BackupQuotaFull { .. }
            | AgentEvent::BackupSkippedEmpty { .. }
            | AgentEvent::SaveAutoRestored { .. }
            | AgentEvent::SaveAutoRestoreFailed { .. } => engine.transfer_finished(),
            _ => {}
        }
        persist(&event);
        // Antes de meterlo en el journal: el aviso es del evento **vivo**, y un
        // colapso (una racha del mismo reposo) no debe cambiar si suena o no.
        notifier.consider(&event).await;
        log.record(OffsetDateTime::now_utc(), event);
    }
    // El canal sólo se cierra cuando ya no queda ningún emisor, y el daemon
    // guarda uno vivo mientras corre. Llegar aquí es el apagado.
    tracing::info!("hoardd: the event channel closed");
    Finished
}

/// Persiste en `state.json` lo que el motor sólo tiene en memoria: el cursor de
/// versión y la firma anti-resubida. Sin esto, cada reinicio del daemon
/// re-subiría snapshots idénticos y re-bajaría para diferenciar.
fn persist(event: &AgentEvent) {
    let (save_id, version, set_hash) = match event {
        AgentEvent::BackupSuccess {
            save_id,
            version_num,
            set_hash,
            ..
        } => (save_id, Some(*version_num), set_hash.clone()),
        // Tras un restore el slot está sincronizado a esa versión: recordarlo es
        // lo que hace que el version-gate sobreviva a un reinicio.
        AgentEvent::SaveAutoRestored {
            save_id,
            version_num,
            ..
        } => (save_id, Some(*version_num), None),
        _ => return,
    };

    let (mut state, path) = match CliState::load_default() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(error = %err, "hoardd: couldn't load state.json to persist an event");
            return;
        }
    };
    let Some(entry) = state.saves.get_mut(save_id) else {
        // Un save de la nube respaldado antes de adoptarlo no tiene fila local.
        return;
    };
    if let Some(v) = version {
        entry.last_version_num = Some(v);
    }
    if let Some(hash) = set_hash {
        entry.set_hash = Some(hash);
    }
    if matches!(event, AgentEvent::BackupSuccess { .. }) {
        entry.last_backup_at = Some(OffsetDateTime::now_utc());
    }
    if let Err(err) = state.save(&path) {
        tracing::warn!(error = %err, "hoardd: couldn't write state.json");
    }
}

/// Re-hidrata el conjunto de saves vigilados desde `state.json` y le pasa al
/// motor la diferencia. Es lo que responde a [`hoard_core::ipc::Request::Reload`]:
/// el cliente avisa de que el conjunto cambió y el daemon —dueño del estado—
/// decide qué vigilar.
pub async fn reload(engine: &Engine) -> anyhow::Result<usize> {
    let Some(handle) = engine.handle() else {
        anyhow::bail!("the engine isn't running");
    };
    let (state, _path) = CliState::load_default()?;
    let archived = match engine.client() {
        Some(client) => archived_save_ids(&client).await,
        None => HashSet::new(),
    };
    let desired = library::watched_saves_from_state(&state, &archived);
    let current: std::collections::HashSet<String> =
        tokio::time::timeout(STATUS_TIMEOUT, handle.status())
            .await
            .map_err(|_| anyhow::anyhow!("the engine didn't answer in {STATUS_TIMEOUT:?}"))??
            .into_iter()
            .map(|s| s.save_id)
            .collect();
    let desired_ids: std::collections::HashSet<String> =
        desired.iter().map(|s| s.save_id.clone()).collect();

    for save in desired
        .into_iter()
        .filter(|s| !current.contains(&s.save_id))
    {
        handle.add_save(save).await?;
    }
    for gone in current.difference(&desired_ids) {
        handle.remove_save(gone.clone()).await?;
    }
    let watched = desired_ids.len();
    engine.set_watched(watched);
    Ok(watched)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un motor caído con un motivo y sin `Running`, que es como queda tras un
    /// `note_error`. Suficiente para las dos políticas de abajo, que sólo miran
    /// el estado.
    fn down_with(reason: EngineDownReason, is_cloud: bool) -> Engine {
        let engine = Engine::new();
        {
            let mut guard = engine.lock();
            guard.status.running = false;
            guard.status.reason = reason;
            guard.status.is_cloud = is_cloud;
        }
        engine
    }

    fn up_on(is_cloud: bool) -> Engine {
        let engine = Engine::new();
        {
            let mut guard = engine.lock();
            guard.status.running = true;
            guard.status.is_cloud = is_cloud;
        }
        engine
    }

    fn restart_asked(engine: &Engine) -> bool {
        engine.lock().restart_requested.is_some()
    }

    /// Prestar el token prueba que la sesión se lee. Un motor caído por no poder
    /// leerla tiene que reintentar ya, no agotar cinco minutos de backoff junto a
    /// una sesión que ya funciona — los 82 s del 28-ago-2026.
    #[test]
    fn a_readable_session_wakes_an_engine_that_was_missing_one() {
        for reason in [
            EngineDownReason::NoSession,
            EngineDownReason::KeyringUnreadable,
            EngineDownReason::SessionExpired,
        ] {
            let engine = down_with(reason, true);
            engine.wake_if_a_session_would_help();
            assert!(
                restart_asked(&engine),
                "{reason:?} lo desbloquea una sesión"
            );
        }
    }

    /// Y sólo esos. Un motor que cayó por otra cosa no se arregla porque alguien
    /// haya podido leer la sesión, y despertarlo en cada préstamo de token
    /// convertiría el backoff en un bucle.
    #[test]
    fn a_readable_session_doesnt_wake_an_engine_that_failed_for_another_reason() {
        for reason in [EngineDownReason::Other, EngineDownReason::Unknown] {
            let engine = down_with(reason, true);
            engine.wake_if_a_session_would_help();
            assert!(
                !restart_asked(&engine),
                "{reason:?} no lo arregla una sesión"
            );
        }
    }

    /// Un motor vivo no se toca: el token se presta constantemente, y reiniciar
    /// en cada préstamo sería cortar la sincronización cada pocos minutos.
    #[test]
    fn a_live_engine_is_never_woken() {
        let engine = up_on(true);
        engine.wake_if_a_session_would_help();
        assert!(!restart_asked(&engine));
    }

    /// Las dos sesiones son independientes y el motor corre contra una sola:
    /// tirarlo porque se fue la otra es un corte gratis y una segunda tanda de
    /// "vigilando" por cada save.
    #[test]
    fn signing_out_of_the_other_session_leaves_the_engine_alone() {
        let engine = up_on(true);
        engine.request_restart_if_signed_out(false, "self-hosted signed out");
        assert!(!restart_asked(&engine), "el motor va con Cloud");

        let engine = up_on(false);
        engine.request_restart_if_signed_out(true, "cloud signed out");
        assert!(!restart_asked(&engine), "el motor va con el self-hosted");
    }

    /// La suya sí lo tira: está hablando con un servidor cuya sesión ya no
    /// existe.
    #[test]
    fn signing_out_of_its_own_session_restarts_the_engine() {
        let engine = up_on(true);
        engine.request_restart_if_signed_out(true, "cloud signed out");
        assert!(restart_asked(&engine));
    }

    /// Y un motor caído se reinicia venga de donde venga la baja: la sesión que
    /// queda puede ser justo la que le faltaba.
    #[test]
    fn a_down_engine_restarts_on_either_sign_out() {
        let engine = down_with(EngineDownReason::NoSession, true);
        engine.request_restart_if_signed_out(false, "self-hosted signed out");
        assert!(restart_asked(&engine));
    }

    /// El motivo tiene que sobrevivir a las capas de contexto que le pone el
    /// camino real: `resolve_owned` envuelve el error un par de veces antes de
    /// llegar aquí. Si la clasificación sólo mirase la capa de fuera, el caso más
    /// importante —no hay sesión— saldría como `Other` y la ventana volvería a
    /// enseñar el banner genérico.
    #[test]
    fn no_session_survives_the_context_layers() {
        let err = anyhow::Error::new(hoard_agent::session::NoSession)
            .context("resolviendo la sesión del servicio")
            .context("arrancando el motor");
        assert_eq!(classify(&err), EngineDownReason::NoSession);
    }

    /// Las dos formas de fallar del llavero comparten motivo: el consejo al
    /// usuario es el mismo.
    #[test]
    fn both_keyring_failures_read_as_unreadable() {
        let stuck = anyhow::Error::new(hoard_agent::keychain::KeyringTimeout {
            doing: "reading the self-hosted session",
            after: std::time::Duration::from_secs(5),
        })
        .context("leyendo la sesión");
        assert_eq!(classify(&stuck), EngineDownReason::KeyringUnreadable);

        let refused =
            anyhow::anyhow!("access denied").context(hoard_agent::keychain::KeyringUnreadable {
                doing: "reading the self-hosted session",
            });
        assert_eq!(classify(&refused), EngineDownReason::KeyringUnreadable);
    }

    /// Y lo que no reconocemos se dice que no se reconoce, en vez de disfrazarse
    /// del último motivo que se nos ocurra: `last_error` lleva el detalle y el
    /// banner cae al texto genérico, que para un fallo desconocido es honesto.
    #[test]
    fn anything_else_stays_other() {
        let err = anyhow::anyhow!("the server hung up").context("arrancando el motor");
        assert_eq!(classify(&err), EngineDownReason::Other);
    }
}
