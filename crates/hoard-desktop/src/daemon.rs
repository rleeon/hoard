//! El enlace del desktop con `hoardd` (ADR 0021, Parte A — Slice 4b).
//!
//! Hasta el 4a el desktop **embebía** el motor (`agent::spawn` dentro de
//! `start_agent`), y el árbitro para que no corriera a la vez que el de la CLI
//! era el pidfile de `hoard_agent::instance`, con sus tres fallos de diseño:
//! carrera de arranque, cero reclaim y el ciclo de vida del sync atado a una
//! ventana. Desde este slice el desktop **no tiene motor**: es un cliente del
//! servicio, le manda comandos por el socket local y pinta lo que el servicio
//! reporta. Cerrar la app ya no puede parar el sync — que es el punto de todo el
//! Slice 4.
//!
//! ## Dos conexiones, a propósito
//!
//! - **Comandos** ([`DaemonLink::request`]): una conexión perezosa bajo mutex.
//!   Cada `#[tauri::command]` que antes tocaba el `AgentHandle` manda aquí su
//!   petición.
//! - **Eventos** ([`pump`]): una conexión dedicada que sólo escucha.
//!
//! No es una por comodidad: `read_frame` lee cabecera y cuerpo en dos pasos, así
//! que **no es cancel-safe**. Una sola conexión obligaría a un `select!` entre
//! "espera un push" y "manda una petición", y cancelar la lectura a medias
//! dejaría el flujo desincronizado. Dos conexiones cuestan al daemon una task
//! más por cliente y nos dan lecturas que nunca se cancelan.
//!
//! ## Journal + push (D.14.2)
//!
//! Al conectar se pide el backlog desde el cursor y luego se escucha en vivo. El
//! cursor vive **en memoria**: una ejecución nueva de la app arranca con la UI
//! vacía, así que pedir el anillo entero es justo lo que reconstruye la historia
//! que no vio. Dentro de una misma ejecución, el cursor evita repetir lo ya
//! pintado al reconectar.
//!
//! Y cuando no se puede afirmar continuidad —el anillo perdió filas (`gap`), el
//! daemon se reinició (`epoch` distinto) o el canal de push nos dejó atrás
//! (`Resync`)— se le dice a la UI que **resincronice** en vez de coserlo en
//! silencio. Fingir continuidad ahí es el bug de las campanas mudas otra vez.
//!
//! ## Quién enciende el relevo: la UI, y no antes
//!
//! [`attach`] lo llama el store cuando **ya** tiene sus `listen()` puestos, no
//! `start_agent`. Es deliberado: `start_agent` también lo llama el escaneo de
//! Modo Automático, que corre en Rust y puede ganarle al montaje del webview —
//! y un backlog emitido antes de que exista el oyente es un historial que se
//! pierde en silencio, exactamente el bug que el journal existe para no tener.
//! Con el relevo atado a la suscripción, la primera emisión no puede llegar
//! pronto. (La conexión de comandos es independiente: el escaneo levanta el
//! servicio igual, sólo que sin nadie escuchando todavía.)

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use hoard_agent::agent::{AgentConfig, AgentEvent};
use hoard_agent::state::CliState;
use hoard_agent::supervisor::{self, Finished};
use hoard_core::ipc::{
    AdoptedSession, AgentSlotStatus, CloudToken, DaemonStatus, EngineDownReason, IpcError,
    KeyringFault, Payload, Request, ServerSession, UpdateState,
};
use hoardd::client::{Client, Push};
use hoardd::endpoint::Endpoint;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::state::AppState;

/// Cómo nos presentamos en el log del daemon.
fn client_name(role: &str) -> String {
    format!("hoard-desktop {} ({role})", env!("CARGO_PKG_VERSION"))
}

/// Espera entre reconexiones del bombeo de eventos. El caso normal es que el
/// daemon siga vivo y esto no llegue a usarse; cubre el reinicio del servicio
/// (una actualización) sin que la UI se quede muda para siempre.
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

/// Espera entre reintentos cuando al servicio lo pararon **a propósito**. Ya no
/// lo relanzamos (ADR 0021 4d), así que reintentar rápido sólo sirve para llenar
/// el log; sigue reintentándose para engancharse solo si alguien lo arranca.
const STOPPED_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Tope de una petición ya conectada. Generoso para el peor caso real (el
/// `Status` pregunta al motor, que puede estar hasheando), pero finito: sin él,
/// un servicio atascado colgaría el comando de la UI y, tras él, todos los que
/// esperan la conexión de comandos.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Cada cuánto se re-pregunta el estado del motor.
///
/// El motor arriba/abajo **no es un evento del journal** (`AgentEvent` no tiene
/// esa variante), así que sin esto la UI se quedaría con el estado del instante
/// en que conectó: un motor que arranca 20 s más tarde —lo normal, el daemon
/// resuelve la sesión primero— dejaría el icono en "parado" toda la sesión. Es
/// un round-trip por socket local; el push sigue siendo quien trae los eventos.
const STATUS_EVERY: Duration = Duration::from_secs(20);

/// Lo que la UI conoce como `AgentStatus`. Su forma es contrato con los stores
/// (`ui/src/lib/stores/agent.ts`), que no deben notar que detrás ya no hay un
/// motor embebido sino un servicio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentStatus {
    pub running: bool,
    pub watched_count: usize,
    /// El servicio manda él mismo las notificaciones nativas del SO (ADR 0021
    /// D.14.1), así que la UI **no** debe mandar las suyas o el usuario vería el
    /// aviso dos veces con la app abierta. En una plataforma donde el servicio
    /// todavía no sabe notificar (Windows, macOS) llega `false` y el aviso sigue
    /// siendo del frontend, igual que antes de este slice.
    pub service_notifies: bool,
    /// **Por qué** no hay motor, cuando no lo hay. Hasta la 1.1.0 este dato moría
    /// aquí: el daemon lo tenía tipado, este struct lo tiraba, y la ventana sólo
    /// podía enseñar "el servicio está desconectado" — con lo que dos usuarios de
    /// self-hosted pasaron días sin backups sin manera de saber que lo que les
    /// faltaba era la sesión. Lo que la UI pinta sale de aquí.
    pub reason: EngineDownReason,
    /// El texto crudo del último fallo, para el detalle y para que el usuario
    /// pueda copiarlo en un reporte. La frase traducida sale de `reason`.
    pub last_error: Option<String>,
    /// Which way the keyring failed, when `reason` is `KeyringUnreadable`. One
    /// reason, four next steps: a machine with no secret-service daemon is not a
    /// locked one, and telling that user to unlock their login keyring sends them
    /// after something that isn't installed.
    pub keyring: Option<KeyringFault>,
}

impl AgentStatus {
    /// Lo que la UI debe pintar cuando no sabemos nada del motor.
    ///
    /// `service_notifies: false` es el default seguro: sin servicio tampoco
    /// llegan eventos, así que no hay nada que duplicar, y el peor caso posible
    /// es un aviso repetido — nunca uno perdido.
    pub fn down() -> Self {
        Self {
            running: false,
            watched_count: 0,
            service_notifies: false,
            // We never reached the service, so we don't know whether the engine
            // is up either: `Unreachable` says that and nothing more. This used
            // to be `Unknown`, which in the window is the sentence "the sync
            // service is stopped" — a claim we have no grounds for, and one
            // that on 2026-08-28 was simply false: the service had been up for
            // thirteen hours.
            reason: EngineDownReason::Unreachable,
            last_error: None,
            keyring: None,
        }
    }

    /// Traduce el estado del daemon a lo que la UI conoce. Un solo sitio: había
    /// dos construcciones a mano y la del bucle de estado se olvidaba de la mitad
    /// de los campos nuevos.
    pub fn from_daemon(status: &hoard_core::ipc::DaemonStatus) -> Self {
        Self {
            running: status.engine.running,
            watched_count: status.slots.len().max(status.engine.watched),
            service_notifies: status.notifications,
            reason: status.engine.reason,
            // Sólo cuando hay algo roto: el último error de un motor que ya está
            // arriba es ruido que la ventana no debe enseñar.
            last_error: (!status.engine.running)
                .then(|| status.engine.last_error.clone())
                .flatten(),
            keyring: (!status.engine.running)
                .then_some(status.engine.keyring)
                .flatten(),
        }
    }
}

/// Una fila del journal camino de la UI.
#[derive(Debug, Clone, Serialize)]
pub struct BacklogRow {
    /// Identidad de la fila dentro de esta ejecución del daemon. La ventana
    /// principal no la necesita —va cosiendo el feed evento a evento— pero
    /// cualquier superficie que **relea** la instantánea entera sí: sin una clave
    /// estable, cada relectura sería una lista nueva para Svelte.
    pub seq: u64,
    /// Cuándo pasó, en ms de época. Va incluido porque **la hora importa** al
    /// reproducir: un `game_started` de hace dos horas tiene que pintar dos
    /// horas de sesión, no arrancar el contador de cero. Es la última ocurrencia
    /// (`last_at`), que en una fila colapsada es la que sigue siendo cierta.
    pub at: i64,
    pub event: AgentEvent,
}

/// Backlog del journal, tal como lo recibe la UI.
#[derive(Debug, Clone, Serialize)]
struct BacklogPayload {
    /// En orden cronológico (lo más viejo primero), como salió del journal.
    rows: Vec<BacklogRow>,
    /// No hay continuidad que respetar: el cliente debe reconstruir su estado
    /// desde este backlog en vez de parchear el que tenía.
    resync: bool,
}

impl From<hoard_core::ipc::JournalEntry> for BacklogRow {
    fn from(entry: hoard_core::ipc::JournalEntry) -> Self {
        Self {
            seq: entry.seq,
            at: entry.last_at.unix_timestamp() * 1000,
            event: entry.event,
        }
    }
}

/// Cuántas filas del journal se guardan aquí para quien llegue tarde.
///
/// El anillo del daemon tiene 1024; éste es sólo el espejo local de lo que ya se
/// relevó, y quien lo lee recorta a `MAX_FEED_ENTRIES` (80). Ciento veinte dejan
/// margen para las filas que no son de feed sin que la instantánea engorde el
/// puente del webview, que se cruza entero en cada lectura.
const JOURNAL_MIRROR: usize = 120;

/// Estado del bucle de nube tal como lo ve la UI. Es el mismo vocabulario que
/// `CloudStatus` en `stores/live.ts`: quien lo pinta no traduce nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloudPulse {
    /// Todavía no ha terminado ninguna pasada.
    #[default]
    Unknown,
    Online,
    Offline,
    Throttled,
}

/// Lo que la UI sabe **ahora mismo**, sin preguntarle a nadie.
///
/// Existe para las superficies que sólo leen. La ventana principal construye su
/// estado escuchando: enciende el relevo, recibe el backlog una vez y va
/// cosiendo los eventos que llegan. Una ventana que nace después —el HUD— no
/// puede hacer eso: el backlog ya se emitió, [`attach`] es idempotente y
/// [`emit_status`] sólo habla cuando algo cambia, así que escuchar le daría un
/// panel en blanco y una cabecera en rojo con el servicio vivo.
///
/// Así que no escucha: lee esto. Es una copia de lo que este proceso ya tenía en
/// memoria —cero E/S, cero red, cero peticiones al servicio— y por eso abrir el
/// HUD no puede arrancar, despertar ni alterar nada.
#[derive(Debug, Clone, Serialize)]
pub struct UiSnapshot {
    pub status: AgentStatus,
    /// Lo que el servicio dice de cada partida vigilada: si el juego está
    /// corriendo y cuándo le toca la próxima copia.
    ///
    /// Va aparte del journal a propósito. Quién está jugando y qué copia viene
    /// son **estado**, no historia: reconstruirlos replicando eventos obliga a
    /// guardar el `game_started` para siempre, y el día que se caiga del anillo
    /// el HUD dirá que nadie juega con el juego delante. El servicio ya lleva la
    /// cuenta; esto la copia.
    pub slots: Vec<AgentSlotStatus>,
    /// En orden cronológico, lo más viejo primero (como el backlog).
    pub rows: Vec<BacklogRow>,
    pub cloud: CloudPulse,
    /// Segundos hasta que se levante el freno, cuando `cloud == Throttled`.
    pub cloud_retry_in: Option<u32>,
}

/// Cursor del journal dentro de **una** ejecución del daemon.
#[derive(Debug, Clone)]
struct Cursor {
    /// Identidad de la ejecución del daemon. Un `seq` sólo es comparable dentro
    /// del mismo epoch: si cambió, el daemon se reinició y el cursor no vale.
    epoch: String,
    seq: u64,
}

/// Estado del enlace, vivo mientras viva la app.
#[derive(Default)]
pub struct DaemonLink {
    /// Conexión de comandos. Perezosa: no se abre hasta que hay algo que pedir,
    /// y se vuelve a abrir sola si el daemon se reinicia.
    cmd: tokio::sync::Mutex<Option<Client>>,
    /// Bombeo de eventos y refresco de estado. No vacío mientras la UI está
    /// escuchando (entre [`attach`] y [`detach`]).
    tasks: Mutex<Vec<JoinHandle<()>>>,
    cursor: Arc<Mutex<Option<Cursor>>>,
    /// Último estado publicado a la UI. Está aquí y no dentro del bucle de
    /// estado porque hay **dos** emisores —el bucle y el bombeo, que sabe antes
    /// que nadie que se cayó el socket— y con una memoria por emisor el segundo
    /// nunca se entera de lo que publicó el primero: el bombeo pinta "parado", el
    /// bucle sigue creyendo que ya publicó "arriba" y la UI se queda en parado
    /// hasta que el motor cambie de estado por su cuenta.
    last_status: Mutex<Option<AgentStatus>>,
    /// Espejo de las últimas filas relevadas, para quien llegue tarde
    /// ([`UiSnapshot`]). Se escribe en el mismo gesto que se emiten: lo que hay
    /// aquí es exactamente lo que la ventana principal recibió, ni más ni menos.
    journal: Mutex<std::collections::VecDeque<BacklogRow>>,
    /// Últimos slots que reportó el servicio, del mismo `Status` del que sale
    /// [`Self::last_status`].
    slots: Mutex<Vec<AgentSlotStatus>>,
    /// Último pulso del bucle de nube, que vive en `commands::cloud_pull` y
    /// emite por su cuenta. Mismo motivo que el journal: sus eventos son
    /// momentáneos y quien no estaba escuchando no puede recuperarlos.
    cloud: Mutex<(CloudPulse, Option<u32>)>,
}

impl DaemonLink {
    /// Endpoint del usuario (o el de `HOARDD_SOCKET`). Se resuelve en cada uso:
    /// es leer una variable de entorno y componer una ruta, y así un override no
    /// se queda cacheado de un arranque anterior.
    fn endpoint() -> Result<Endpoint> {
        Endpoint::resolve().context("resolving the hoardd endpoint")
    }

    /// Manda una petición al daemon, levantándolo si no lo hay.
    ///
    /// Un fallo de **transporte** tira la conexión y reintenta una vez: el caso
    /// real es un servicio que se actualizó y reinició entre dos comandos, y
    /// quien pulsó el botón no tiene por qué enterarse.
    ///
    /// Un [`IpcError`] **no** se reintenta: es un daemon sano contestando "no
    /// puedo, y por esto". Reintentarlo reconectaría en cada `EngineDown` —dos
    /// conexiones y dos líneas de log por cada comando mientras no hay motor—
    /// para volver a recibir exactamente la misma respuesta.
    pub async fn request(&self, request: Request) -> Result<Payload> {
        match self.request_once(request.clone()).await {
            Ok(payload) => Ok(payload),
            Err(err) if err.downcast_ref::<IpcError>().is_some() => Err(err),
            Err(err) => {
                tracing::debug!(error = %format!("{err:#}"), "daemon: retrying on a fresh connection");
                *self.cmd.lock().await = None;
                self.request_once(request).await
            }
        }
    }

    async fn request_once(&self, request: Request) -> Result<Payload> {
        let mut guard = self.cmd.lock().await;
        if guard.is_none() {
            let endpoint = Self::endpoint()?;
            // `ensure_running` no comprueba "¿hay daemon?" antes de lanzar: eso
            // es un TOCTOU y produce dos motores. Lanza y reconecta; si dos
            // clientes lo hacen a la vez, uno gana el bind y el otro sale.
            *guard = Some(
                Client::ensure_running(&endpoint, &client_name("commands"))
                    .await
                    .with_context(|| format!("connecting to the Hoard service at {endpoint}"))?,
            );
        }
        let client = guard.as_mut().expect("just connected");
        // Con tope: un servicio que acepta la conexión y luego no contesta
        // colgaría el botón que la disparó **para siempre**, y con él a todos los
        // comandos que esperan este mutex. Cortar la lectura a medias desordena
        // el flujo, así que la conexión se tira en el mismo gesto.
        let result = match tokio::time::timeout(REQUEST_TIMEOUT, client.request(request)).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "the Hoard service didn't answer in {}s",
                REQUEST_TIMEOUT.as_secs()
            )),
        };
        if result
            .as_ref()
            .err()
            .is_some_and(|err| err.downcast_ref::<IpcError>().is_none())
        {
            // Un fallo que no es del protocolo casi siempre es la conexión: que
            // la siguiente petición empiece por reconectar. Un `IpcError` no lo
            // es —la conexión funcionó y trajo una respuesta— y tirarla sería
            // reconectar por cada comando que llega con el motor caído.
            *guard = None;
        }
        result
    }

    /// Estado del daemon: motor, slots y cursor.
    pub async fn status(&self) -> Result<DaemonStatus> {
        match self.request(Request::Status).await? {
            Payload::Status(status) => Ok(status),
            other => Err(anyhow!("unexpected answer to status: {other:?}")),
        }
    }

    /// Cómo va la actualización, según el servicio.
    ///
    /// La ventana no mira GitHub para esto: **el updater es del servicio** (es
    /// lo único que está siempre, así que es lo único que puede actualizar una
    /// máquina cuya app lleva dos semanas cerrada). Aquí sólo se lee lo que ya
    /// sabe y se pinta.
    pub async fn update_state(&self) -> Result<UpdateState> {
        match self.request(Request::UpdateStatus).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to update status: {other:?}")),
        }
    }

    /// Dile al servicio que aplique ya lo que tenga bajado.
    ///
    /// Lo pide la ventana porque **hay alguien delante**, y esa es toda la
    /// diferencia: con un humano al teclado el servicio puede abrir el diálogo
    /// de privilegios que su ciclo de fondo no puede abrir. Vuelve enseguida;
    /// aplicar sigue en marcha y se sigue con `update_state`.
    pub async fn apply_update(&self, version: Option<String>) -> Result<UpdateState> {
        match self.request(Request::ApplyUpdate { version }).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to apply update: {other:?}")),
        }
    }

    /// "Ahora no", durante `hours`. No mueve la fecha límite.
    pub async fn snooze_update(&self, hours: u32) -> Result<UpdateState> {
        match self.request(Request::SnoozeUpdate { hours }).await? {
            Payload::Update(state) => Ok(state),
            other => Err(anyhow!("unexpected answer to snooze update: {other:?}")),
        }
    }

    /// Pide prestado un token Cloud válido al servicio.
    ///
    /// **El desktop ya no rota.** El servicio es el único que toca el refresh
    /// token de `cloud.toml` (ADR 0021, Parte A: "un único rotador"), así que
    /// aquí sólo se pide uno prestado y se usa. Que dos procesos rotaran el mismo
    /// refresh token es la causa raíz de la familia 401/realtime-mudo: GoTrue
    /// revoca la familia entera al detectar el reuso, y eso no se recupera ni
    /// reiniciando.
    ///
    /// `rejected` es el token con el que acabamos de comer un 401: sin él, un
    /// token revocado server-side pero aún "fresco" volvería una y otra vez.
    pub async fn cloud_token(&self, rejected: Option<String>) -> Result<CloudToken> {
        match self.request(Request::CloudToken { rejected }).await? {
            Payload::CloudToken(token) => {
                // De paso se lo dejamos puesto al enviador de logs, que corre en
                // su propio hilo y no puede pedir nada por IPC. Este es el único
                // sitio del desktop por donde entra un token Cloud fresco.
                hoard_agent::credentials::set_lent_cloud(Some(
                    hoard_agent::credentials::CloudLease {
                        url: token.server_url.clone(),
                        token: token.access_token.clone(),
                    },
                ));
                Ok(token)
            }
            other => Err(anyhow!("unexpected answer to cloud_token: {other:?}")),
        }
    }

    /// Entrega al servicio la sesión Cloud que el OAuth acaba de acuñar.
    ///
    /// El desktop no la escribe: el dueño del secreto es el servicio. En macOS eso
    /// es la diferencia entre una app que funciona y una que pide la contraseña
    /// del llavero cada pocos segundos — el ítem lo autoriza sólo el binario que
    /// lo creó, y el que lo lee (el motor) vive en `hoardd`.
    pub async fn adopt_session(&self, session: AdoptedSession) -> Result<()> {
        match self.request(Request::AdoptSession { session }).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!("unexpected answer to adopt_session: {other:?}")),
        }
    }

    /// Dile al servicio que olvide la sesión Cloud (logout).
    pub async fn forget_session(&self) -> Result<()> {
        match self.request(Request::ForgetSession).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!("unexpected answer to forget_session: {other:?}")),
        }
    }

    /// Entrega al servicio la sesión self-hosted que la app acaba de validar.
    pub async fn adopt_server_session(&self, session: ServerSession) -> Result<()> {
        match self
            .request(Request::AdoptServerSession { session })
            .await?
        {
            Payload::Ack => Ok(()),
            other => Err(anyhow!(
                "unexpected answer to adopt_server_session: {other:?}"
            )),
        }
    }

    /// Dile al servicio que olvide la sesión self-hosted (logout).
    pub async fn forget_server_session(&self) -> Result<()> {
        match self.request(Request::ForgetServerSession).await? {
            Payload::Ack => Ok(()),
            other => Err(anyhow!(
                "unexpected answer to forget_server_session: {other:?}"
            )),
        }
    }

    /// Pide prestada la sesión del server propio: URL, token y quién eres.
    ///
    /// El token `hoard_v1_` es estático —no caduca ni se rota—, así que esto se
    /// pide una vez y se guarda en el hueco de `hoard_agent::credentials` para que
    /// lo vea también el enviador de logs, que no puede pedir nada por IPC.
    pub async fn server_session(&self) -> Result<ServerSession> {
        match self.request(Request::ServerToken).await? {
            Payload::ServerSession(session) => Ok(session),
            other => Err(anyhow!("unexpected answer to server_token: {other:?}")),
        }
    }

    /// Petición best-effort: la loguea y sigue. Para los sitios donde el fallo
    /// no debe abortar lo que el usuario pidió (persistir una preferencia,
    /// re-hidratar tras añadir un juego).
    pub async fn tell(&self, what: &'static str, request: Request) {
        if let Err(err) = self.request(request).await {
            tracing::warn!(error = %format!("{err:#}"), "daemon: couldn't {what}");
        }
    }

    /// El conjunto de saves vigilados cambió en disco: que el daemon lo relea.
    /// El cliente **avisa**, no manda la lista — el dueño del estado es el
    /// servicio.
    pub async fn notify_reload(&self) {
        self.tell("ask the service to reload its watch list", Request::Reload)
            .await;
    }

    fn cursor(&self) -> Option<Cursor> {
        self.cursor.lock().unwrap().clone()
    }

    fn set_cursor(&self, epoch: &str, seq: u64) {
        *self.cursor.lock().unwrap() = Some(Cursor {
            epoch: epoch.to_string(),
            seq,
        });
    }

    /// Guarda una fila ya relevada. `resync` la trata como lo que dice ser: no
    /// hay continuidad que respetar, así que lo de antes se tira en vez de
    /// coserse con lo nuevo — igual que hacen los stores.
    fn remember(&self, rows: &[BacklogRow], resync: bool) {
        let mut journal = self.journal.lock().unwrap();
        if resync {
            journal.clear();
        }
        for row in rows {
            journal.push_back(row.clone());
        }
        while journal.len() > JOURNAL_MIRROR {
            journal.pop_front();
        }
    }

    /// Anota el pulso del bucle de nube. Lo llama `commands::cloud_pull` en el
    /// mismo gesto en que emite, para que el espejo no pueda contar otra cosa
    /// que lo que oyó la ventana principal.
    pub fn note_cloud(&self, pulse: CloudPulse, retry_in: Option<u32>) {
        *self.cloud.lock().unwrap() = (pulse, retry_in);
    }

    /// Anota los slots del último `Status`. Con la lista vacía cuando no hay
    /// servicio: no saber es un dato, y fingir que los últimos que vimos siguen
    /// vigentes sería pintar «jugando» sobre un motor que ya no mira nada.
    fn note_slots(&self, slots: &[AgentSlotStatus]) {
        *self.slots.lock().unwrap() = slots.to_vec();
    }

    /// Todo lo que este proceso sabe, copiado. Sin E/S y sin tocar el servicio:
    /// ver [`UiSnapshot`].
    pub fn snapshot(&self) -> UiSnapshot {
        let (cloud, cloud_retry_in) = *self.cloud.lock().unwrap();
        UiSnapshot {
            status: self
                .last_status
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(AgentStatus::down),
            slots: self.slots.lock().unwrap().clone(),
            rows: self.journal.lock().unwrap().iter().cloned().collect(),
            cloud,
            cloud_retry_in,
        }
    }
}

/// Empieza a relevar los eventos del servicio a la UI. Lo llama el store cuando
/// sus oyentes ya están puestos. Idempotente.
pub fn attach(app: &AppHandle) {
    let link = app.state::<AppState>();
    let mut tasks = link.daemon.tasks.lock().unwrap();
    if !tasks.is_empty() {
        return;
    }
    // Regla de D.12: si vive más que una petición, va supervisado. Un pánico
    // aquí dejaría la UI muda sin una línea de log, que es exactamente el fallo
    // invisible que costó dos sesiones.
    tasks.push(tokio::spawn({
        let app = app.clone();
        supervisor::supervise("desktop daemon event pump", move || pump(app.clone()))
    }));
    tasks.push(tokio::spawn({
        let app = app.clone();
        supervisor::supervise("desktop daemon status", move || status_loop(app.clone()))
    }));
}

/// Deja de relevar eventos: para las tasks y cierra la conexión de eventos.
///
/// **No** manda `Shutdown` ni toca el motor. Que el desktop pueda parar el
/// servicio sería volver al ciclo de vida atado a una ventana; parar el sync es
/// una orden explícita del usuario (`hoard sync stop`), no un efecto de cerrar
/// sesión o la app.
///
/// El cursor **se conserva**: si la UI se vuelve a suscribir dentro de la misma
/// ejecución, pedir desde él evita repetirle lo que ya pintó.
pub fn detach(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tasks: Vec<JoinHandle<()>> = std::mem::take(&mut *state.daemon.tasks.lock().unwrap());
    for task in tasks {
        task.abort();
    }
}

/// Conecta, pide el backlog y reenvía el push en vivo. No retorna nunca: una
/// conexión que se cae se reintenta, porque el servicio puede reiniciarse
/// (actualización) sin que la app se entere de otra forma.
async fn pump(app: AppHandle) -> Finished {
    loop {
        if let Err(err) = pump_once(&app).await {
            tracing::warn!(error = %format!("{err:#}"), "daemon: the event stream ended");
        }
        // La UI no puede quedarse creyendo que el motor sigue: si perdimos el
        // socket, no sabemos nada de él.
        app.state::<AppState>().daemon.note_slots(&[]);
        emit_status(&app, &AgentStatus::down());
        tokio::time::sleep(reconnect_delay()).await;
    }
}

/// Cuánto esperar antes de volver a intentarlo. Con el servicio parado a
/// propósito no hay prisa: nadie va a contestar hasta que alguien lo arranque, y
/// nosotros ya no lo arrancamos. Sondear cada 3 s sólo llenaría el log.
fn reconnect_delay() -> Duration {
    if hoardd::client::stopped_on_purpose() {
        STOPPED_RETRY_DELAY
    } else {
        RECONNECT_DELAY
    }
}

async fn pump_once(app: &AppHandle) -> Result<()> {
    let endpoint = DaemonLink::endpoint()?;
    let mut client = Client::ensure_running(&endpoint, &client_name("events"))
        .await
        .with_context(|| format!("connecting to the Hoard service at {endpoint}"))?;
    let epoch = client.welcome().epoch.clone();
    tracing::info!(
        pid = client.welcome().pid,
        version = %client.welcome().daemon_version,
        "desktop: attached to the Hoard service"
    );

    let state = app.state::<AppState>();
    // Un cursor de otra ejecución del daemon no es un cursor: pedir desde él
    // dejaría a la UI esperando eventos que ya pasaron.
    let since = state
        .daemon
        .cursor()
        .filter(|c| c.epoch == epoch)
        .map(|c| c.seq);
    let fresh = since.is_none();
    // Con tope, como los comandos: si el servicio acepta y luego calla, la UI se
    // quedaría sin backlog y sin push, y sin una línea que lo dijera. Al fallar,
    // el bucle de arriba tira esta conexión y reconecta.
    let backlog = tokio::time::timeout(REQUEST_TIMEOUT, client.subscribe(since))
        .await
        .map_err(|_| anyhow!("the Hoard service didn't answer the subscribe"))??;
    state.daemon.set_cursor(&epoch, backlog.cursor);
    let resync = fresh || backlog.gap;
    if backlog.gap {
        tracing::warn!(
            requested = since.unwrap_or(0),
            cursor = backlog.cursor,
            "desktop: the service no longer has the journal rows we asked for; resyncing"
        );
    }
    emit_backlog(
        app,
        backlog.entries.into_iter().map(BacklogRow::from).collect(),
        resync,
    );

    while let Some(push) = client.next_push().await? {
        match push {
            Push::Event(entry) => {
                state.daemon.set_cursor(&epoch, entry.seq);
                // Guardar y emitir en el mismo gesto, como hace el journal del
                // daemon: si se separan, un día uno de los dos se olvida y la
                // superficie que lee la instantánea se queda contando una
                // historia distinta de la que oyó la ventana principal.
                let row = BacklogRow::from(entry);
                state.daemon.remember(std::slice::from_ref(&row), false);
                emit_event(app, &row.event);
            }
            // Nos hemos retrasado y el canal descartó filas. El daemon lo
            // confiesa en vez de dejar el hueco invisible; nosotros volvemos a
            // pedir desde nuestro cursor.
            Push::Resync { cursor, dropped } => {
                tracing::warn!(
                    dropped,
                    cursor,
                    "desktop: fell behind the service's event push; re-reading the journal"
                );
                let since = state.daemon.cursor().map(|c| c.seq);
                let backlog = tokio::time::timeout(REQUEST_TIMEOUT, client.subscribe(since))
                    .await
                    .map_err(|_| anyhow!("the Hoard service didn't answer the re-subscribe"))??;
                state.daemon.set_cursor(&epoch, backlog.cursor);
                emit_backlog(
                    app,
                    backlog.entries.into_iter().map(BacklogRow::from).collect(),
                    true,
                );
            }
            // Lo pararon a propósito (`hoard sync stop`, `systemctl --user
            // stop`). El cliente ya se ha anotado la despedida, así que el
            // reintento de abajo se limitará a mirar si vuelve: cerrar esta
            // ventana no puede parar el sync, pero abrirla tampoco puede
            // deshacer una orden de pararlo.
            Push::Goodbye { reason } => {
                tracing::info!(reason, "desktop: the Hoard service was stopped on purpose");
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Re-pregunta el estado y lo publica cuando cambia. La primera vuelta va sin
/// esperar: es la que pone el dot del watcher en su sitio justo después de que
/// la UI se suscriba.
async fn status_loop(app: AppHandle) -> Finished {
    let mut armed: HashSet<String> = HashSet::new();
    loop {
        let state = app.state::<AppState>();
        match state.daemon.status().await {
            Ok(status) => {
                announce_slots(&app, &status.slots, &mut armed);
                state.daemon.note_slots(&status.slots);
                let now = AgentStatus::from_daemon(&status);
                if !now.running {
                    tracing::debug!(
                        reason = status.engine.last_error.as_deref().unwrap_or("starting"),
                        "desktop: the service has no engine"
                    );
                }
                emit_status(&app, &now);
            }
            // Sin servicio no sabemos nada del motor, así que la UI tampoco debe
            // creer que sigue: el icono no puede quedarse en verde por inercia.
            Err(err) => {
                tracing::debug!(error = %format!("{err:#}"), "desktop: couldn't read the service status");
                state.daemon.note_slots(&[]);
                emit_status(&app, &AgentStatus::down());
            }
        }
        tokio::time::sleep(STATUS_EVERY).await;
    }
}

/// Publica el estado del motor con la forma que la UI ya conoce, **si cambió**.
/// Repetirlo cada 20 s despertaría al webview para no decirle nada.
pub fn emit_status(app: &AppHandle, status: &AgentStatus) {
    let state = app.state::<AppState>();
    {
        let mut last = state.daemon.last_status.lock().unwrap();
        if last.as_ref() == Some(status) {
            return;
        }
        *last = Some(status.clone());
    }
    let _ = app.emit("agent://daemon-status", status);
}

/// Anuncia los slots que el servicio dice estar vigilando.
///
/// El slug lo resuelve el cliente contra `state.json` porque es cosa de
/// presentación: el daemon reporta por `save_id`, que es su identidad real.
pub fn announce_slots(app: &AppHandle, slots: &[AgentSlotStatus], seen: &mut HashSet<String>) {
    let fresh: Vec<&AgentSlotStatus> = slots
        .iter()
        .filter(|s| !seen.contains(&s.save_id))
        .collect();
    if fresh.is_empty() {
        return;
    }
    let slugs = CliState::load_default()
        .map(|(state, _)| state)
        .ok()
        .map(|state| {
            state
                .saves
                .into_iter()
                .map(|(id, save)| (id, save.game_slug))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .unwrap_or_default();
    for slot in fresh {
        seen.insert(slot.save_id.clone());
        let _ = app.emit(
            "agent://watcher-armed",
            WatcherArmed {
                save_id: slot.save_id.clone(),
                game_slug: slugs
                    .get(&slot.save_id)
                    .cloned()
                    .unwrap_or_else(|| slot.display_name.clone()),
            },
        );
    }
}

#[derive(Debug, Clone, Serialize)]
struct WatcherArmed {
    save_id: String,
    game_slug: String,
}

fn emit_backlog(app: &AppHandle, rows: Vec<BacklogRow>, resync: bool) {
    // El espejo se escribe **antes** del early-return y aunque no haya filas: un
    // `resync` sin nada que traer sigue siendo la orden de tirar lo de antes.
    app.state::<AppState>().daemon.remember(&rows, resync);
    if rows.is_empty() && !resync {
        return;
    }
    tracing::debug!(
        count = rows.len(),
        resync,
        "desktop: seeding the UI from the service's journal"
    );
    let _ = app.emit("agent://backlog", BacklogPayload { rows, resync });
}

/// Reenvía un evento del motor a la UI por su canal Tauri de siempre.
///
/// Este mapeo es el contrato con los stores: cambia el backend (motor embebido →
/// servicio) sin que las pantallas se enteren, que es la restricción dura de
/// D.3. Sólo los eventos **en vivo** pasan por aquí; el backlog va por su propio
/// canal para que un historial recuperado no dispare toasts ni escaneos.
fn emit_event(app: &AppHandle, ev: &AgentEvent) {
    let topic = match ev {
        AgentEvent::GameStarted { .. } => "agent://game-started",
        AgentEvent::GameStopped { .. } => "agent://game-stopped",
        AgentEvent::BackupScheduled { .. } => "agent://backup-scheduled",
        AgentEvent::BackupStarted { .. } => "agent://backup-started",
        AgentEvent::BackupSuccess { .. } => "agent://backup-success",
        AgentEvent::BackupFailed { .. } => "agent://backup-failed",
        AgentEvent::BackupThrottled { .. } => "agent://backup-throttled",
        AgentEvent::BackupTooLarge { .. } => "agent://backup-too-large",
        AgentEvent::BackupQuotaFull { .. } => "agent://backup-quota-full",
        AgentEvent::BackupTrimmed { .. } => "agent://backup-trimmed",
        AgentEvent::BackupFilesUnreadable { .. } => "agent://backup-files-unreadable",
        AgentEvent::BackupNeedsAttention { .. } => "agent://backup-needs-attention",
        AgentEvent::BackupAttentionCleared { .. } => "agent://backup-attention-cleared",
        AgentEvent::SaveAutoRestored { .. } => "agent://save-auto-restored",
        AgentEvent::SaveAutoRestoreFailed { .. } => "agent://save-auto-restore-failed",
        AgentEvent::BackupSkippedEmpty { .. } => "agent://backup-skipped-empty",
        AgentEvent::SaveConflictsBackedUp { .. } => "agent://save-conflicts-backed-up",
        AgentEvent::HeavyProcessDetected { .. } => "agent://heavy-process-detected",
        AgentEvent::RestoreDeferred { .. } => "agent://restore-deferred",
        AgentEvent::SaveAutoRestoreStuck { .. } => "agent://save-auto-restore-stuck",
        AgentEvent::SaveAutoRestoreRecovered { .. } => "agent://save-auto-restore-recovered",
    };
    let _ = app.emit(topic, ev);

    // Un juego pesado sin rastrear acaba de aparecer: adelanta el escaneo en vez
    // de esperar al temporizador. `request_scan` no hace nada si Modo Automático
    // está apagado, y agrupa ráfagas.
    if let AgentEvent::HeavyProcessDetected { name } = ev {
        tracing::info!(process = %name, "desktop: heavy untracked game suspected; requesting immediate scan");
        crate::commands::automatic::request_scan(app.clone());
    }

    // Alias con nombres semánticos para la superficie LiveStatus/ActivityFeed.
    // Mismo payload, canal más legible; los canales originales siguen vivos.
    match ev {
        AgentEvent::BackupStarted { .. } => {
            let _ = app.emit("agent://upload-started", ev);
        }
        AgentEvent::BackupSuccess { .. } => {
            let _ = app.emit("agent://upload-completed", ev);
        }
        AgentEvent::BackupScheduled {
            reason: hoard_agent::agent::BackupReason::FilesystemSettled,
            delay_ms,
            ..
        } if *delay_ms > debounce_ms() => {
            // Sólo cuando el min-interval aplazó la subida más allá del debounce
            // hay una espera de verdad que enseñar; el debounce rutinario de
            // cada autosave no es "en cola — esperando".
            //
            // Esta rama estuvo muerta hasta ago-2026: el único `BackupScheduled`
            // que se emitía venía del temporizador de debounce y su `delay_ms`
            // era el debounce exacto, así que nunca lo superaba. Ahora el motor
            // anuncia también la espera del suelo (`agent::announce_backup_wait`,
            // 60 s como mínimo), que es la que de verdad hay que enseñar.
            let _ = app.emit("agent://throttled", ev);
        }
        _ => {}
    }
}

/// Debounce con el que corre el motor del servicio. El daemon construye su
/// `AgentConfig` con `..AgentConfig::default()` para este campo, así que el
/// default **es** el valor vivo; si algún día lo hace configurable, esto tiene
/// que venir del `Status` en vez de calcularse aquí.
fn debounce_ms() -> u64 {
    AgentConfig::default().debounce_secs.saturating_mul(1000)
}
