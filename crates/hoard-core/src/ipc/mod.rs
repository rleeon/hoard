//! # Protocolo IPC del servicio local (ADR 0021, Parte A — Slice 4)
//!
//! El motor deja de ir embebido en cada frontend y pasa a vivir en un proceso
//! propio (`hoardd`, un motor por usuario). Desktop y CLI se conectan a él por
//! un socket local — UDS en Linux/macOS, named pipe en Windows — y este módulo
//! es **lo que viaja por ese socket**: los sobres, las peticiones, las
//! respuestas, el journal de eventos y el encuadre.
//!
//! Vive en `hoard-core` por la misma razón que [`crate::wire`] (C.6): el
//! contrato no puede pertenecer a una de las dos puntas. `hoard-core` es el
//! kernel *leaf* —sólo `serde`, sin `tokio`—, así que aquí están **los tipos y
//! el encuadre**, y el transporte (bind, accept, permisos, leer/escribir del
//! socket) vive en el crate del daemon, que sí tiene runtime.
//!
//! ## Encuadre
//!
//! Cada mensaje va como `u32` big-endian con la longitud + ese número de bytes
//! de JSON. Framed y no line-delimited porque un evento lleva rutas de fichero
//! (`SaveConflictsBackedUp::conflict_dir`) y no quiero que un `\n` dentro de un
//! campo sea sintaxis. El tope ([`MAX_FRAME_BYTES`]) se comprueba **antes** de
//! reservar el buffer: un socket local sigue siendo entrada no confiable y un
//! prefijo de 4 GiB no puede convertirse en una reserva de 4 GiB.
//!
//! ## Handshake versionado
//!
//! Ahora hay 2+ artefactos actualizables (servicio + app + CLI) que tienen que
//! hablar el mismo protocolo, y no se actualizan a la vez: el usuario actualiza
//! la app y el servicio de usuario sigue siendo el viejo hasta que reinicia la
//! sesión. Así que el cliente manda [`Hello`] con su [`PROTOCOL_VERSION`] y el
//! daemon contesta [`Welcome`] o [`Rejected`] con **la suya**, para que el
//! cliente pueda decir "actualiza/reinicia el servicio" en vez de fallar con un
//! error de parseo.
//!
//! Dentro de una misma versión de protocolo se aplica la disciplina de
//! [`crate::wire`]: append-only, `#[serde(default)]` en todo campo nuevo, nunca
//! repurposear un campo. La versión sube sólo cuando el cambio no es
//! compatible.
//!
//! ## Entrega de eventos: journal + push
//!
//! Los dos, no uno u otro (D.14.2). El cliente conecta → pide todo lo posterior
//! a su cursor ([`Request::Subscribe`]) → recibe [`Payload::Backlog`] → y desde
//! ahí escucha [`ServerFrame::Event`] en vivo. Ver [`journal`].

pub mod events;
pub mod journal;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use events::{AgentEvent, AgentSlotStatus, BackupReason};
pub use journal::{Backlog, JournalEntry};

/// Versión del protocolo. Sube **sólo** ante un cambio incompatible; añadir un
/// campo con `#[serde(default)]` o una variante que el otro lado pueda ignorar
/// no lo es.
pub const PROTOCOL_VERSION: u32 = 1;

/// Bytes de cabecera de cada trama (el `u32` de longitud).
pub const HEADER_BYTES: usize = 4;

/// Tope de una trama. Los mensajes reales son cientos de bytes; el tope existe
/// para que un prefijo de longitud absurdo no se convierta en una reserva de
/// memoria absurda. El backlog más grande imaginable (1024 filas de journal)
/// cabe con holgura.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Fallo de encuadre. Distinto de un error de aplicación ([`IpcError`]): esto
/// significa que la conexión ya no es de fiar y se cierra.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame of {size} bytes exceeds the {MAX_FRAME_BYTES}-byte limit")]
    TooLarge { size: usize },
    #[error("malformed frame: {0}")]
    Malformed(#[from] serde_json::Error),
}

/// Serializa `msg` como trama completa (cabecera + JSON).
pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, FrameError> {
    let body = serde_json::to_vec(msg)?;
    if body.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { size: body.len() });
    }
    let mut out = Vec::with_capacity(HEADER_BYTES + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Longitud declarada por una cabecera, validada contra [`MAX_FRAME_BYTES`].
/// El lector la llama antes de reservar el buffer del cuerpo.
pub fn frame_len(header: [u8; HEADER_BYTES]) -> Result<usize, FrameError> {
    let size = u32::from_be_bytes(header) as usize;
    if size > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge { size });
    }
    Ok(size)
}

/// Deserializa el cuerpo de una trama.
pub fn decode_frame<T: DeserializeOwned>(body: &[u8]) -> Result<T, FrameError> {
    Ok(serde_json::from_slice(body)?)
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

/// Primera trama del cliente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    /// Quién llama, para los logs del daemon: `"hoard-desktop 7.7.16"`.
    pub client: String,
}

/// Handshake aceptado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: u32,
    pub daemon_version: String,
    pub pid: u32,
    /// Identidad de **esta ejecución** del daemon. Los `seq` del journal
    /// empiezan de nuevo en cada arranque, así que un cursor guardado sólo vale
    /// si el epoch coincide; si cambió, el cliente arranca de 0. Sin esto, un
    /// cliente con cursor 500 contra un daemon recién reiniciado se quedaría
    /// esperando eventos que ya pasaron.
    pub epoch: String,
    /// Cursor del journal ahora mismo, para que el cliente sepa cuánto hay sin
    /// pedirlo.
    pub cursor: u64,
}

/// Handshake rechazado. Lleva la versión del daemon para que el cliente pueda
/// decir qué hay que actualizar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rejected {
    pub reason: String,
    pub daemon_protocol: u32,
    pub daemon_version: String,
}

// ---------------------------------------------------------------------------
// Sobres
// ---------------------------------------------------------------------------

/// Trama cliente → daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ClientFrame {
    Hello(Hello),
    /// `id` lo elige el cliente y vuelve en [`ServerFrame::Reply`], para poder
    /// tener varias peticiones en vuelo por la misma conexión.
    Request {
        id: u64,
        request: Request,
    },
}

/// Trama daemon → cliente.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    Welcome(Welcome),
    Rejected(Rejected),
    Reply {
        id: u64,
        reply: Reply,
    },
    /// Push en vivo: una fila nueva del journal. Los colapsos (rachas del mismo
    /// reposo) no se empujan — ver [`journal::Appended`].
    Event(JournalEntry),
    /// El cliente no pudo seguir el ritmo del canal de push y se perdió filas.
    /// Debe volver a pedir [`Request::Subscribe`] desde su cursor. Es la
    /// alternativa honesta a tragarse el hueco en silencio.
    Resync {
        cursor: u64,
        dropped: u64,
    },
    /// **El servicio se está parando a propósito** y se despide antes de cerrar
    /// el socket (ADR 0021 D.17 → 4d).
    ///
    /// Sin esto, un cliente enganchado no puede distinguir "lo pararon" de "se
    /// cayó", y como su reconexión es "spawn if absent", un `hoard sync stop`
    /// resucitaba el servicio ~3 s después: el apagado deliberado no se quedaba
    /// apagado. Con la despedida, el cliente sigue reconectando —si alguien lo
    /// vuelve a arrancar, se engancha— pero **no lo arranca él**. Un daemon que
    /// muere de verdad (pánico, OOM, kill -9) no manda nada, así que ahí el
    /// cliente sigue levantándolo, que es lo correcto.
    Goodbye {
        reason: String,
    },
    /// Trama que este cliente no conoce: la manda un daemon más nuevo.
    ///
    /// Hay 2+ artefactos que se actualizan por separado, así que un daemon puede
    /// aprender una trama antes que el cliente. Sin esta variante, la primera
    /// trama desconocida sería un error de encuadre —y el encuadre roto **tira la
    /// conexión**, que es un fallo desproporcionado para "no sé qué es esto".
    /// Ignorarla es lo que permite añadir tramas dentro de la misma versión de
    /// protocolo, que es justo lo que [`ServerFrame::Goodbye`] acaba de hacer.
    #[serde(other)]
    Unknown,
}

/// Peticiones. Espejo de la superficie pública de `AgentHandle`: el IPC es el
/// `AgentHandle` remoto, así que si aparece un comando nuevo en el motor,
/// aparece aquí y no en cada frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Latido: confirma que el otro extremo es un daemon vivo.
    Ping,
    /// Estado completo del daemon y de cada slot vigilado.
    Status,
    /// Backlog desde `since` (o desde el principio) + alta en el push en vivo.
    /// `None` = "no tengo cursor, dame lo que haya".
    Subscribe {
        since: Option<u64>,
    },
    BackupNow {
        save_id: String,
    },
    SweepAll {
        window_secs: u64,
    },
    ForceRestore {
        save_id: String,
        /// Head the caller already knows (SSE `save` frame, cloud poller).
        /// Kernel `cloud_ahead` needs this in cache; a bare `ForceRestore` is
        /// only a tick nudge and no-ops on self-hosted when heads were never
        /// observed. `None` from older clients — engine still reconciles.
        #[serde(default)]
        version_num: Option<i64>,
    },
    SetAutoRestore {
        enabled: bool,
    },
    SetGlobalSync {
        enabled: bool,
    },
    /// El conjunto de saves rastreados cambió en disco (`state.json`):
    /// re-hidrátalo. El daemon es el dueño del estado, así que el cliente
    /// **avisa**, no manda la lista — un `WatchedSave` por el cable sería el
    /// cliente decidiendo qué vigila el motor.
    Reload,
    /// Carpetas candidatas (detectadas pero no rastreadas) que el motor debe
    /// sondear para correlacionar proceso↔escritura. Es lo único que un cliente
    /// **sí** manda como lista: la detección vive en el frontend (Slice 8 la
    /// mueve) y el motor no puede adivinarla.
    ///
    /// Van como `String` porque el cable es JSON: una ruta que no sea UTF-8 no
    /// cabe y se queda fuera en el cliente, que es donde se puede decir.
    SetProbeCandidates {
        dirs: Vec<String>,
    },
    /// **Préstame un token Cloud válido.** El daemon es el único que rota
    /// `cloud.toml` (Parte A: "un único rotador"), así que quien necesite hablar
    /// con la nube —el desktop para sus llamadas REST, la CLI para un one-shot—
    /// pide aquí en vez de refrescar por su cuenta. Dos procesos rotando el
    /// mismo refresh token es la reuse-detection de GoTrue, que revoca la
    /// familia entera: 401 permanente y sesión muerta sin arreglo.
    ///
    /// `rejected` es el token que al cliente le acaban de rechazar con un 401.
    /// Sin él, un cliente que come un 401 con un token todavía "fresco" (revocado
    /// server-side, reloj desfasado) recibiría el mismo token una y otra vez y
    /// reintentaría en bucle. Con él, el daemon rota **sólo** si el que serviría
    /// es justo ese; si ya lo rotó otro, contesta con el nuevo sin gastar una
    /// rotación.
    CloudToken {
        #[serde(default)]
        rejected: Option<String>,
    },
    /// **Toma esta sesión Cloud recién acuñada y guárdala tú.** El cliente
    /// acaba de terminar un OAuth (o un login por email) y entrega el par en vez
    /// de escribirlo: el daemon es el único que toca el almacén de secretos.
    ///
    /// No es simetría por gusto con [`Request::CloudToken`], es lo que arregla
    /// los diálogos de contraseña en macOS. Ahí cada ítem del llavero lleva una
    /// ACL con los binarios autorizados, y quien **crea** el ítem es el único de
    /// la lista: con el login escribiendo desde la app y el motor leyendo desde
    /// `hoardd`, cada lectura del servicio era un binario ajeno pidiendo permiso
    /// → un diálogo por lectura, y el keeper reintentando cada pocos segundos.
    /// Escribiéndolo el daemon, creador y lector son el mismo binario y no hay
    /// nada que autorizar. En Linux (Secret Service) y Windows (Credential
    /// Manager) el secreto no está atado al binario, así que allí esto es sólo
    /// coherencia: "el motor es el dueño del secreto, los clientes son vistas".
    ///
    /// Implica el efecto de [`Request::RestartEngine`]: acabamos de aprender una
    /// sesión nueva, y el motor que hubiera está hablando con la anterior.
    AdoptSession {
        session: AdoptedSession,
    },
    /// **Olvida la sesión Cloud** (logout, o cuenta borrada). Lo pide el cliente
    /// por la misma razón que [`Request::AdoptSession`]: borrar un ítem del
    /// llavero también hay que autorizarlo, y el que puede hacerlo sin
    /// preguntarle nada al usuario es su dueño. Implica reiniciar el motor.
    ForgetSession,
    /// **Toma esta sesión self-hosted y guárdala tú.** El gemelo de
    /// [`Request::AdoptSession`] para un server propio: el cliente valida el
    /// token contra `/v1/auth/whoami` y entrega `(url, token, user)`.
    ///
    /// Arregla dos cosas de una, y la segunda es la gorda. La primera es la misma
    /// ACL del llavero de macOS que [`Request::AdoptSession`]. La segunda es que
    /// **el motor no veía la sesión del desktop en absoluto**: la app guardaba en
    /// `credentials` (llavero + `session.toml`) y el motor resolvía self-hosted
    /// leyendo `config.toml`, que sólo escribe `hoard login --token`. Dos
    /// almacenes disjuntos, así que quien entraba sólo por la app tenía un motor
    /// que no sincronizaba nada. Con un único dueño hay un único almacén.
    ///
    /// Implica el efecto de [`Request::RestartEngine`], como su gemelo.
    AdoptServerSession {
        session: ServerSession,
    },
    /// **Olvida la sesión self-hosted** (logout). Igual que
    /// [`Request::ForgetSession`], pero del server propio.
    ForgetServerSession,
    /// **Préstame el token del server propio.** El gemelo de
    /// [`Request::CloudToken`] para self-hosted, y mucho más simple: un token
    /// `hoard_v1_…` es estático (no caduca ni se rota), así que aquí no hay
    /// rotación que decidir — sólo el almacén, que es del daemon.
    ///
    /// Devuelve también `user` para que un cliente que perdió su `session.toml`
    /// (la ACL que un build viejo de Windows dejaba clavada) recupere quién es
    /// sin esperar a que el daemon le repare el fichero.
    ServerToken,
    /// La sesión en disco cambió: tira el motor y deja que el keeper lo levante
    /// resolviendo las credenciales de nuevo.
    ///
    /// Ya no lo necesita ningún login: las cuatro peticiones de sesión
    /// (`AdoptSession`/`ForgetSession` y sus gemelos self-hosted) lo llevan
    /// dentro. Se queda porque sigue siendo la forma de decir "he tocado el disco
    /// por debajo" — un `hoard login --token` sin servicio al alcance, un
    /// `config.toml` editado a mano.
    ///
    /// Distinto de [`Request::Reload`], que sólo re-hidrata el conjunto de
    /// saves: un cambio de cuenta invalida el `ApiClient`, el contexto de
    /// `state.json` y el rotador del token, y ninguno de los tres se arregla
    /// añadiendo y quitando slots. Y distinto de [`Request::Shutdown`]: el
    /// servicio sigue vivo, sólo cambia de sesión.
    RestartEngine,
    /// Para el motor y el daemon. Es una orden explícita del usuario
    /// (`hoard sync stop`), no un efecto de cerrar un cliente: cerrar la app
    /// nunca puede matar el sync, que es el punto de todo el slice.
    Shutdown,
    /// **¿Cómo va la actualización?** El servicio es el dueño del updater —es
    /// el único que está siempre—, así que los clientes no miran GitHub: se lo
    /// preguntan a él. Responde [`Payload::Update`].
    UpdateStatus,
    /// **Aplica ya lo que haya bajado.** Lo pide un cliente cuando hay alguien
    /// delante: el botón de Ajustes, `hoard upgrade`, o la ventana al abrirse.
    ///
    /// La diferencia con esperar al ciclo de fondo no es la prisa, es el
    /// permiso: con un humano delante, el servicio puede lanzar un `pkexec` y
    /// que el diálogo de polkit tenga a quién preguntarle. En el ciclo de fondo
    /// no puede, y por eso un `.deb` no se actualiza solo.
    ///
    /// `version` es la que el cliente creía estar aplicando. Si entretanto
    /// salió otra, el servicio contesta con su estado nuevo en vez de instalar
    /// a sabiendas algo que ya no es lo último.
    ApplyUpdate {
        #[serde(default)]
        version: Option<String>,
    },
    /// **Ahora no.** Calla lo que se puede posponer durante `hours`. No mueve
    /// la fecha límite: posponer retrasa la pregunta, no el plazo.
    SnoozeUpdate {
        hours: u32,
    },
    /// Petición que este daemon no conoce: la manda un cliente más nuevo.
    ///
    /// Sin esta variante, la primera petición desconocida sería un error de
    /// **encuadre**, y el encuadre roto tira la conexión — un cliente recién
    /// actualizado hablándole al servicio viejo de hace treinta segundos se
    /// quedaría sin servicio en vez de recibir un "eso no lo sé hacer". Con
    /// ella la respuesta es [`IpcError::Unsupported`] y la conexión sigue viva,
    /// que es lo que permite añadir peticiones sin subir la versión del
    /// protocolo (C.6).
    #[serde(other)]
    Unknown,
}

/// Respuesta a una petición.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Reply {
    Ok(Payload),
    Error(IpcError),
}

/// Carga útil de una respuesta correcta.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "payload", rename_all = "snake_case")]
pub enum Payload {
    /// Aceptado. Los comandos del motor son fire-and-forget: lo que pasó
    /// después llega por el journal.
    Ack,
    Pong {
        daemon_version: String,
        pid: u32,
    },
    Status(DaemonStatus),
    Backlog(Backlog),
    CloudToken(CloudToken),
    /// La sesión self-hosted prestada (respuesta a [`Request::ServerToken`]).
    ServerSession(ServerSession),
    /// Cómo va la actualización (respuesta a [`Request::UpdateStatus`] y a
    /// [`Request::ApplyUpdate`]).
    Update(UpdateState),
}

/// **Lo que el servicio sabe de la actualización**, que es todo: qué corre, qué
/// hay publicado, qué está bajado y cuándo deja de ser opcional.
///
/// Es lo único que un cliente necesita para pintar la actualización, y a
/// propósito no incluye nada que un cliente pudiera *decidir*. La política vive
/// en `hoard_agent::install::auto` y la ejecuta el servicio; esto es la vista.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    /// La versión que corre en el servicio.
    pub current: String,
    /// La última publicada, si se ha podido preguntar.
    #[serde(default)]
    pub latest: Option<String>,
    /// La que está bajada y verificada, lista para aplicarse en lo que tarda un
    /// `rename`.
    #[serde(default)]
    pub staged: Option<String>,
    pub phase: UpdatePhase,
    /// Cuándo deja de ser opcional. `None` = no hay nada pendiente.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    /// El plazo venció: la ventana no debe dejar seguir sin actualizar.
    #[serde(default)]
    pub mandatory: bool,
    /// Esta máquina se releva sola (AppImage, NSIS por-usuario, núcleo en el
    /// home). `false` significa que hace falta un humano —un `.deb` quiere
    /// polkit, un `.dmg` quiere una mano—, y es lo que decide si el cliente
    /// tiene que enseñar algo o puede callarse.
    #[serde(default)]
    pub unattended: bool,
    /// Qué falló en el último intento, si falló.
    #[serde(default)]
    pub last_error: Option<String>,
}

/// En qué punto está.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum UpdatePhase {
    /// No hay nada más nuevo.
    UpToDate,
    /// Bajando y verificando.
    Downloading,
    /// Bajado. Esperando el momento, o a que alguien diga que sí.
    Ready,
    /// Bajado y frenado, con motivo.
    Waiting { hold: UpdateHold },
    /// Aplicándose ahora mismo.
    Applying,
    /// Aplicado. El servicio se está relevando con el binario nuevo.
    Restarting,
    /// El último intento falló (el motivo va en `last_error`).
    Failed,
    /// Aquí no actualizamos nada: lo mantiene un tercero (el gestor de paquetes
    /// de la distro, Flatpak, un `nix`).
    Managed,
    /// Fase que este cliente no conoce, de un daemon más nuevo.
    #[serde(other)]
    Unknown,
}

/// Por qué está frenada una actualización que ya está bajada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateHold {
    /// Hay una copia o una restauración a medias. Frena siempre, plazo o no.
    TransferInFlight,
    /// Hay un juego abierto. Frena lo silencioso, no lo obligatorio.
    GameRunning,
    /// Motivo que este cliente no conoce.
    #[serde(other)]
    Unknown,
}

/// Una sesión Cloud que un cliente **entrega** al daemon
/// ([`Request::AdoptSession`]).
///
/// Es el único sitio del protocolo por donde viaja un refresh token, y va en una
/// dirección: cliente → daemon, una vez, al acuñarse la sesión. De vuelta sólo
/// se presta el access token ([`CloudToken`]), nunca el refresh: un cliente que
/// no lo tiene no puede rotarlo, que es la regla que sostiene "un único rotador".
#[derive(Clone, Serialize, Deserialize)]
pub struct AdoptedSession {
    /// A qué Cloud pertenece la sesión.
    pub server_url: String,
    /// JWT recién emitido.
    pub access_token: String,
    /// Refresh token con el que el daemon renovará a partir de ahora.
    pub refresh_token: String,
}

/// A mano y **redactado**: el `Debug` derivado imprimiría los dos tokens, y basta
/// un `?request` en un log del daemon para que la sesión entera acabe en el
/// journal del sistema (que es texto plano y sobrevive al logout).
impl std::fmt::Debug for AdoptedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdoptedSession")
            .field("server_url", &self.server_url)
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .finish()
    }
}

/// Una sesión self-hosted: a qué server, con qué token y de quién.
///
/// Viaja en las dos direcciones, y eso es correcto aquí: el token de un server
/// propio es estático (`hoard_v1_` + 64 hex) y no se rota, así que "entregarlo"
/// y "prestarlo" son la misma forma. No es el caso de Cloud, donde el refresh
/// token sólo entra ([`AdoptedSession`]) y sólo sale el access ([`CloudToken`]).
#[derive(Clone, Serialize, Deserialize)]
pub struct ServerSession {
    /// URL del server propio.
    pub server_url: String,
    /// El bearer token.
    pub token: String,
    /// Snapshot de `/v1/auth/whoami` para que el cliente sepa quién es sin una
    /// llamada de red. `None` cuando no se pudo consultar.
    #[serde(default)]
    pub user: Option<ServerUser>,
}

/// Igual que [`AdoptedSession`]: `Debug` a mano para que el token no aparezca en
/// ningún log por accidente.
impl std::fmt::Debug for ServerSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerSession")
            .field("server_url", &self.server_url)
            .field("token", &"<redacted>")
            .field("user", &self.user)
            .finish()
    }
}

/// Quién es el usuario en su server propio. Espejo de
/// `hoard_agent::credentials::UserSection`, que es lo que la app cachea en disco;
/// vive aquí porque el cable no puede depender del agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUser {
    pub user_id: String,
    pub username: String,
    pub is_admin: bool,
}

/// Un token Cloud prestado por el daemon (respuesta a [`Request::CloudToken`]).
///
/// Es un **préstamo**, no una transferencia: el cliente lo usa para sus
/// peticiones y no lo persiste. El par completo (access + refresh) vive donde
/// siempre —keyring + `cloud.toml`— y sólo lo escribe el daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudToken {
    /// JWT de Supabase con vida suficiente para usarlo ahora mismo.
    pub access_token: String,
    /// A qué servidor Cloud pertenece (el cliente no tiene por qué asumir el
    /// default: un build de dev apunta a otro sitio por entorno).
    pub server_url: String,
    /// `exp` del JWT en segundos de época, si se pudo leer. El cliente puede
    /// adelantarse a la expiración sin decodificar nada.
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// El daemon rotó para servir esta respuesta. Sólo informativo (logs): al
    /// cliente le da igual, y precisamente por eso ya no es asunto suyo.
    #[serde(default)]
    pub rotated: bool,
}

/// Error de aplicación. La conexión sigue viva (a diferencia de
/// [`FrameError`]).
///
/// Implementa `Error` (y por tanto `Display`) a propósito: el cliente lo
/// propaga tal cual y el mensaje acaba en un toast del desktop o en el stdout de
/// la CLI. Volcarlo con `{:?}` enseñaría `EngineDown { reason: … }` al usuario.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum IpcError {
    /// El daemon está arriba pero el motor no. `reason` lo explica (sin sesión,
    /// otro agente tiene el motor, arranque fallando) — un cliente que sólo
    /// viera "error" volvería a intentarlo para siempre sin poder decirle nada
    /// al usuario.
    #[error("the Hoard service has no engine: {reason}")]
    EngineDown { reason: String },
    /// No hay sesión Cloud que prestar y **rotar no lo arregla**: no hay sesión
    /// en disco, o GoTrue revocó la familia entera de tokens (reuse-detection).
    /// Sólo un login nuevo la recupera.
    ///
    /// Es una variante propia porque el cliente actúa distinto: ante un fallo
    /// transitorio reintenta, ante esto cierra sesión localmente y le pide al
    /// usuario que vuelva a entrar. Antes del Slice 4c esa distinción la hacía
    /// cada frontend downcasteando su propio `RefreshTokenStale`; ahora viaja por
    /// el cable porque quien lo descubre es el daemon.
    #[error("the Hoard Cloud session is gone: {reason}")]
    CloudSessionExpired { reason: String },
    /// No hay sesión self-hosted en esta máquina. El gemelo de
    /// [`IpcError::CloudSessionExpired`] para un server propio, y **variante
    /// aparte a propósito**: un token `hoard_v1_` no caduca, así que esto sólo
    /// significa "aquí no hay sesión", nunca "caducó". Mezclarlas haría que un
    /// cliente self-hosted disparara la limpieza de sesión *Cloud*, que es lo que
    /// `CloudSessionExpired` desencadena en el desktop.
    #[error("there is no self-hosted session on this machine: {reason}")]
    NoServerSession { reason: String },
    /// Esa petición no existe en esta versión del protocolo.
    #[error("this Hoard service doesn't support `{op}`")]
    Unsupported { op: String },
    #[error("the Hoard service couldn't do it: {message}")]
    Internal { message: String },
}

/// Estado del daemon. Lo que un cliente necesita para pintar sin haber visto
/// ningún evento — el snapshot que a las campanas mudas les faltaba.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub daemon_version: String,
    pub protocol: u32,
    pub pid: u32,
    pub epoch: String,
    pub uptime_secs: u64,
    /// Cursor del journal, para arrancar una suscripción sin backlog.
    pub cursor: u64,
    /// **El servicio manda él mismo las notificaciones nativas del SO** (ADR
    /// 0021 D.14.1). Un cliente que lea `true` no debe mandar las suyas o el
    /// usuario vería el aviso dos veces con la app abierta.
    ///
    /// `false` significa "este build del daemon todavía no sabe notificar en
    /// esta plataforma" (hoy: Windows y macOS), y entonces el aviso sigue
    /// siendo del frontend — que es exactamente como funcionaba antes. Campo
    /// nuevo con `default`, así que un cliente anterior lo lee como `false` y
    /// sigue notificando como hasta ahora (C.6: append-only).
    #[serde(default)]
    pub notifications: bool,
    pub engine: EngineStatus,
    pub slots: Vec<AgentSlotStatus>,
}

/// Estado del motor dentro del daemon.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    /// Servidor al que habla el motor (`"Cloud · …"` o la URL self-hosted).
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub is_cloud: bool,
    #[serde(default)]
    pub watched: usize,
    /// Desde cuándo lleva vivo este motor.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub since: Option<OffsetDateTime>,
    /// Por qué no está arriba (o el último intento fallido). Un motor caído
    /// **con motivo** es diagnosticable; sin motivo es el fallo invisible que
    /// D.11/D.12 costaron dos sesiones.
    #[serde(default)]
    pub last_error: Option<String>,
    /// El mismo motivo, en un tipo que la UI puede traducir y sobre el que puede
    /// ofrecer el botón que arregla el caso. Campo nuevo con `default`, así que
    /// un cliente viejo lo lee como [`EngineDownReason::Unknown`] y pinta lo que
    /// pintaba antes (C.6: append-only, el protocolo no sube).
    #[serde(default)]
    pub reason: EngineDownReason,
    /// Which way the keyring failed, when [`EngineDownReason::KeyringUnreadable`]
    /// is the reason. `None` for any other reason, and on a service too old to
    /// classify it — the window then shows the general keyring sentence, which is
    /// what it showed before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyring: Option<KeyringFault>,
}

/// Por qué no hay motor, clasificado en origen.
///
/// `last_error` es para el log y para nosotros; esto es para la pantalla. Nace de
/// dos hilos de soporte (jul-2026, self-hosted 1.1.0) en los que el usuario sólo
/// podía decir "the sync service is offline": el motivo existía aquí dentro y se
/// perdía antes de llegar a la ventana, así que ninguno de los dos pudo contar lo
/// único que hacía falta para diagnosticarlo.
///
/// Se clasifica por **downcast tipado**, no mirando el texto del error: un
/// mensaje se reescribe sin pensar y nadie se entera de que la clasificación se
/// rompió, que es exactamente la clase de fallo silencioso que esto viene a matar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineDownReason {
    /// Nadie ha dicho nada: el motor está arriba, arrancando, o lo reporta un
    /// daemon anterior a este campo.
    #[default]
    Unknown,
    /// No hay ninguna sesión que usar. Lo arregla entrar; nada más lo arregla.
    NoSession,
    /// Hay sesión guardada y el llavero no la suelta: bloqueado, sin D-Bus en una
    /// sesión sin escritorio, o una ACL de macOS que no autoriza a este binario.
    /// Se distingue de [`Self::NoSession`] porque el consejo es el contrario:
    /// aquí el usuario **sí** entró, y volver a entrar reescribe el ítem a nombre
    /// de quien lo lee.
    KeyringUnreadable,
    /// Sesión terminalmente caducada (Cloud): sólo un login nuevo la arregla.
    SessionExpired,
    /// We couldn't ask. The service didn't answer a status query, so nothing is
    /// known about the engine — including whether it is running.
    ///
    /// Never sent by the daemon: it is what a *client* fills in when its own
    /// read failed, and it exists so that "I couldn't ask" stops borrowing the
    /// sentence for "it is stopped". They are not the same fact, and on
    /// 2026-08-28 the difference was the whole complaint: the service had been
    /// up for thirteen hours and the window said it was stopped.
    Unreachable,
    /// Cualquier otra cosa. `last_error` lleva el detalle.
    Other,
}

/// Cómo falló el llavero, cuando [`EngineDownReason::KeyringUnreadable`] es el
/// motivo.
///
/// The reason is one; the advice is not. A machine with no secret-service daemon
/// at all is never going to answer, and telling that user to unlock their login
/// keyring sends them looking for something that isn't installed. A locked one
/// unlocks. A damaged entry is rewritten by signing in again. Four errors from
/// production, four different next steps:
/// `The name is not activatable` (nothing to talk to), `Did not receive a reply`
/// (there, mute), `Crypto error: Unpad Error` (there, answering, and what it
/// holds can't be decrypted) and our own five-second cap.
///
/// Travels as a **new optional field** on [`EngineStatus`] rather than as new
/// [`EngineDownReason`] variants, and that is deliberate: a field an older client
/// doesn't know is a field it ignores, while a variant it doesn't know fails the
/// parse of the whole status and leaves it with no data at all about the daemon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyringFault {
    /// There is no secret-service daemon on this machine to talk to.
    Missing,
    /// It's there and it doesn't answer: locked and waiting on an unlock prompt
    /// nobody can see, or a session bus that swallowed the call.
    Locked,
    /// It answered and said no — a macOS ACL that authorises a different binary,
    /// a denied access rule.
    Refused,
    /// It answered, and what it holds can't be read back: a corrupt entry, a
    /// crypto session that won't negotiate.
    Damaged,
    /// A fault a newer service classified and this build doesn't know. Keeps an
    /// older client parsing a newer status instead of dropping it whole.
    #[serde(other)]
    #[default]
    Unknown,
}

impl KeyringFault {
    /// Stable tag for the wire and for the UI to key its sentence on.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Locked => "locked",
            Self::Refused => "refused",
            Self::Damaged => "damaged",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let msg = ClientFrame::Request {
            id: 7,
            request: Request::BackupNow {
                save_id: "s1".into(),
            },
        };
        let bytes = encode_frame(&msg).unwrap();
        let header: [u8; HEADER_BYTES] = bytes[..HEADER_BYTES].try_into().unwrap();
        let len = frame_len(header).unwrap();
        assert_eq!(len, bytes.len() - HEADER_BYTES);
        let back: ClientFrame = decode_frame(&bytes[HEADER_BYTES..]).unwrap();
        assert!(matches!(
            back,
            ClientFrame::Request {
                id: 7,
                request: Request::BackupNow { .. }
            }
        ));
    }

    /// Una cabecera absurda se rechaza **antes** de reservar nada. Un socket
    /// local con permisos 0600 sigue siendo entrada que hay que validar.
    #[test]
    fn an_absurd_header_is_rejected_before_allocating() {
        let err = frame_len(u32::MAX.to_be_bytes()).unwrap_err();
        assert!(matches!(err, FrameError::TooLarge { .. }));
    }

    #[test]
    fn a_truncated_body_is_a_frame_error_not_a_panic() {
        let bytes = encode_frame(&Hello {
            protocol: PROTOCOL_VERSION,
            client: "test".into(),
        })
        .unwrap();
        let body = &bytes[HEADER_BYTES..bytes.len() - 3];
        assert!(decode_frame::<Hello>(body).is_err());
    }

    /// La forma del JSON de los eventos es el contrato que el Slice 4a movió de
    /// `hoard_agent::agent` a [`events`]. Si alguien renombra una variante o un
    /// campo, esto cae: el desktop lleva ese payload a la UI por su nombre de
    /// `type` y el daemon lo guarda en el journal.
    #[test]
    fn events_wire_shape_is_frozen() {
        let ev = AgentEvent::BackupSuccess {
            save_id: "s1".into(),
            version_num: 42,
            total_bytes: 1024,
            set_hash: Some("cheap:content".into()),
            already_landed: false,
            deliberate: true,
        };
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "backup_success");
        assert_eq!(json["version_num"], 42);
        assert_eq!(json["set_hash"], "cheap:content");

        // Campo nuevo con `default` (D.8.3): el payload de un daemon anterior,
        // sin `already_landed`, sigue deserializando — la disciplina append-only
        // es lo que permite añadirlo sin subir la versión de protocolo.
        let legacy: AgentEvent = serde_json::from_str(
            r#"{"type":"backup_success","save_id":"s1","version_num":7,"total_bytes":10,"set_hash":null}"#,
        )
        .unwrap();
        assert_eq!(json["deliberate"], true);
        assert!(matches!(
            legacy,
            AgentEvent::BackupSuccess {
                already_landed: false,
                // Igual que `already_landed`: un daemon anterior no lo manda y
                // se lee como "automática", que es como se comportaba.
                deliberate: false,
                ..
            }
        ));

        let scheduled = AgentEvent::BackupScheduled {
            save_id: "s1".into(),
            delay_ms: 5000,
            reason: BackupReason::FilesystemSettled,
        };
        let json = serde_json::to_value(&scheduled).unwrap();
        assert_eq!(json["type"], "backup_scheduled");
        assert_eq!(json["reason"], "filesystem_settled");

        let deferred: AgentEvent = serde_json::from_str(
            r#"{"type":"restore_deferred","save_id":"s1","game_slug":"factorio","reason":"game is running"}"#,
        )
        .unwrap();
        assert!(matches!(deferred, AgentEvent::RestoreDeferred { .. }));
    }

    /// La despedida viaja con su motivo: el cliente la enseña ("lo paró
    /// `hoard sync stop`") en vez de reportar una conexión perdida.
    #[test]
    fn the_farewell_carries_its_reason() {
        let frame = ServerFrame::Goodbye {
            reason: "stopped on request".into(),
        };
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["frame"], "goodbye");
        assert_eq!(json["reason"], "stopped on request");
    }

    /// Una trama de un daemon más nuevo se ignora en vez de romper el encuadre
    /// (y con él la conexión). Es lo que hace que añadir tramas —como la
    /// despedida— no sea un cambio incompatible de protocolo.
    #[test]
    fn an_unknown_frame_degrades_instead_of_breaking_the_connection() {
        let frame: ServerFrame =
            serde_json::from_str(r#"{"frame":"invented_in_2027","payload":{"a":1}}"#).unwrap();
        assert!(matches!(frame, ServerFrame::Unknown));
    }

    /// El handshake dice **su** versión al rechazar, para que el cliente pueda
    /// decirle al usuario qué actualizar.
    #[test]
    fn a_rejection_carries_the_daemon_version() {
        let frame = ServerFrame::Rejected(Rejected {
            reason: "protocol 2 not supported".into(),
            daemon_protocol: PROTOCOL_VERSION,
            daemon_version: "7.7.16".into(),
        });
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["frame"], "rejected");
        assert_eq!(json["daemon_protocol"], PROTOCOL_VERSION);
    }

    /// El nombre por cable de cada petición es contrato: el daemon despacha por
    /// `op`, así que renombrar una variante rompe a un cliente ya instalado sin
    /// que el handshake se entere (la versión sólo sube ante un cambio
    /// incompatible, y añadir variantes no lo es).
    #[test]
    fn request_op_names_are_frozen() {
        let cases: Vec<(Request, &str)> = vec![
            (Request::Ping, "ping"),
            (Request::Status, "status"),
            (Request::Subscribe { since: Some(7) }, "subscribe"),
            (Request::Reload, "reload"),
            (
                Request::SetProbeCandidates {
                    dirs: vec!["/tmp/x".into()],
                },
                "set_probe_candidates",
            ),
            (Request::RestartEngine, "restart_engine"),
            (Request::CloudToken { rejected: None }, "cloud_token"),
            (
                Request::AdoptSession {
                    session: AdoptedSession {
                        server_url: "https://api.hoard.services".into(),
                        access_token: "jwt".into(),
                        refresh_token: "r0".into(),
                    },
                },
                "adopt_session",
            ),
            (Request::ForgetSession, "forget_session"),
            (
                Request::AdoptServerSession {
                    session: ServerSession {
                        server_url: "https://hoard.example".into(),
                        token: "hoard_v1_dead".into(),
                        user: None,
                    },
                },
                "adopt_server_session",
            ),
            (Request::ForgetServerSession, "forget_server_session"),
            (Request::ServerToken, "server_token"),
            (Request::Shutdown, "shutdown"),
        ];
        for (request, op) in cases {
            let json = serde_json::to_value(&request).unwrap();
            assert_eq!(json["op"], op, "wire name changed for {request:?}");
        }
    }

    /// Older desktops send `force_restore` without `version_num`. New daemon
    /// must still accept that — missing field is "tick only", not a handshake
    /// break.
    #[test]
    fn force_restore_version_num_defaults_when_absent() {
        let v: Request = serde_json::from_str(r#"{"op":"force_restore","save_id":"abc"}"#).unwrap();
        match v {
            Request::ForceRestore {
                save_id,
                version_num,
            } => {
                assert_eq!(save_id, "abc");
                assert_eq!(version_num, None);
            }
            other => panic!("{other:?}"),
        }
    }

    /// La sesión entregada va entera por el cable (si no, el daemon no podría
    /// guardarla) pero **no** por los logs: el `Debug` es a mano justo para eso.
    /// Un `?request` en un `tracing::` del daemon no puede acabar publicando el
    /// refresh token en el journal del sistema.
    #[test]
    fn an_adopted_session_travels_whole_but_never_prints() {
        let session = AdoptedSession {
            server_url: "https://api.hoard.services".into(),
            access_token: "the-jwt".into(),
            refresh_token: "the-refresh".into(),
        };
        let wire = serde_json::to_string(&Request::AdoptSession {
            session: session.clone(),
        })
        .unwrap();
        assert!(wire.contains("the-jwt") && wire.contains("the-refresh"));

        let printed = format!("{session:?}");
        assert!(!printed.contains("the-jwt"), "{printed}");
        assert!(!printed.contains("the-refresh"), "{printed}");
        // El servidor sí es útil verlo: es lo que distingue un login de dev de uno
        // de producción cuando algo no cuadra.
        assert!(printed.contains("api.hoard.services"), "{printed}");
    }

    /// Y lo mismo para la sesión self-hosted, que viaja en las dos direcciones:
    /// entera por el cable, nunca por el log. Aquí el riesgo es mayor que en
    /// Cloud — un token `hoard_v1_` no caduca, así que uno filtrado en el journal
    /// vale para siempre hasta que alguien lo revoque.
    #[test]
    fn a_server_session_travels_whole_but_never_prints() {
        let session = ServerSession {
            server_url: "https://hoard.example".into(),
            token: "hoard_v1_secret".into(),
            user: Some(ServerUser {
                user_id: "u1".into(),
                username: "rai".into(),
                is_admin: true,
            }),
        };
        let wire = serde_json::to_string(&Payload::ServerSession(session.clone())).unwrap();
        assert!(wire.contains("hoard_v1_secret"));

        let printed = format!("{session:?}");
        assert!(!printed.contains("hoard_v1_secret"), "{printed}");
        assert!(printed.contains("hoard.example"), "{printed}");
        // El usuario no es secreto y es justo lo que hace útil el log.
        assert!(printed.contains("rai"), "{printed}");
    }

    /// Campos nuevos con `default`: un daemon viejo que no emite `server` ni
    /// `since` sigue deserializando en un cliente nuevo. Y al revés — el campo
    /// que el 4d borró (`blocked_by_pid`, el pidfile) llega como sobrante de un
    /// daemon anterior y se ignora en vez de romper la conexión.
    #[test]
    fn older_payloads_still_deserialize() {
        let engine: EngineStatus = serde_json::from_str(r#"{"running":true}"#).unwrap();
        assert!(engine.running);
        assert!(engine.server.is_none());
        assert!(engine.since.is_none());

        let legacy: EngineStatus =
            serde_json::from_str(r#"{"running":false,"blocked_by_pid":4242}"#).unwrap();
        assert!(!legacy.running);
    }

    /// Un daemon anterior a las notificaciones nativas no manda el campo, y el
    /// `false` que se asume es justo el lado seguro: el frontend sigue avisando
    /// él. Al revés (bandera nueva, cliente viejo) el campo sobra y se ignora.
    /// Un default invertido dejaría al usuario sin ningún aviso mientras
    /// conviven versiones.
    #[test]
    fn a_daemon_that_doesnt_notify_reads_as_not_notifying() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"7.7.17","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,"engine":{"running":false},"slots":[]}"#,
        )
        .unwrap();
        assert!(!old.notifications);
    }

    /// El motivo tipado del motor caído es append-only: un daemon anterior no lo
    /// manda y el cliente lo lee como `Unknown` (banner genérico, como hasta
    /// ahora) en vez de fallar el parseo entero del estado — que dejaría al
    /// cliente sin *ningún* dato del daemon por un campo informativo.
    #[test]
    fn an_engine_without_a_reason_reads_as_unknown() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"1.1.0","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"last_error":"no session"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(old.engine.reason, EngineDownReason::Unknown);
        assert_eq!(old.engine.last_error.as_deref(), Some("no session"));
    }

    /// The keyring fault is append-only in the way that matters: a service too
    /// old to classify it doesn't send the field, and the client reads `None` and
    /// shows the general keyring sentence — exactly what it showed before the
    /// field existed. This is the reason it is a field and not two more
    /// `EngineDownReason` variants: an unknown *variant* fails the parse of the
    /// whole status and leaves the client with no data about the daemon at all.
    #[test]
    fn an_engine_without_a_keyring_fault_reads_as_none() {
        let old: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"1.1.4","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"reason":"keyring_unreadable"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(old.engine.reason, EngineDownReason::KeyringUnreadable);
        assert_eq!(old.engine.keyring, None);
    }

    /// And the same protection the other way round: a fault a newer service
    /// classifies and this build has no name for reads as `Unknown` instead of
    /// dropping the status. Without `#[serde(other)]` every future variant would
    /// be a client that goes blind against a newer daemon.
    #[test]
    fn an_unknown_keyring_fault_doesnt_sink_the_status() {
        let newer: DaemonStatus = serde_json::from_str(
            r#"{"daemon_version":"9.9.9","protocol":1,"pid":1,"epoch":"e",
                "uptime_secs":0,"cursor":0,
                "engine":{"running":false,"reason":"keyring_unreadable",
                          "keyring":"eaten_by_a_grue"},"slots":[]}"#,
        )
        .unwrap();
        assert_eq!(newer.engine.keyring, Some(KeyringFault::Unknown));

        let json = serde_json::to_value(&EngineStatus {
            running: false,
            reason: EngineDownReason::KeyringUnreadable,
            keyring: Some(KeyringFault::Missing),
            ..EngineStatus::default()
        })
        .unwrap();
        assert_eq!(json["keyring"], "missing");
    }

    /// Y en el cable va en `snake_case`, que es lo que la UI compara.
    #[test]
    fn the_engine_reason_travels_in_snake_case() {
        let json = serde_json::to_string(&EngineStatus {
            running: false,
            reason: EngineDownReason::KeyringUnreadable,
            ..EngineStatus::default()
        })
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["reason"], "keyring_unreadable");
    }

    /// El préstamo del token: el `rejected` es opcional en el cable (un cliente
    /// que sólo quiere "uno válido" no manda nada) y la respuesta lleva la
    /// caducidad para que el cliente no tenga que decodificar el JWT.
    #[test]
    fn the_cloud_token_loan_round_trips() {
        let asked: Request = serde_json::from_str(r#"{"op":"cloud_token"}"#).unwrap();
        assert!(matches!(asked, Request::CloudToken { rejected: None }));

        let lent = Payload::CloudToken(CloudToken {
            access_token: "jwt".into(),
            server_url: "https://api.hoard.services".into(),
            expires_at: Some(1_800_000_000),
            rotated: true,
        });
        let json = serde_json::to_value(&lent).unwrap();
        assert_eq!(json["payload"], "cloud_token");
        assert_eq!(json["access_token"], "jwt");
        assert_eq!(json["expires_at"], 1_800_000_000i64);

        // Un daemon que no sepa la caducidad sigue siendo respuesta válida.
        let minimal: CloudToken =
            serde_json::from_str(r#"{"access_token":"jwt","server_url":"u"}"#).unwrap();
        assert!(minimal.expires_at.is_none());
        assert!(!minimal.rotated);
    }

    /// "La sesión Cloud se acabó" tiene variante propia porque el cliente actúa
    /// distinto que ante un fallo transitorio: cierra sesión en vez de
    /// reintentar. Su nombre por cable es contrato.
    #[test]
    fn a_dead_cloud_session_is_its_own_error() {
        let err = IpcError::CloudSessionExpired {
            reason: "the refresh token family was revoked".into(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "cloud_session_expired");
        let back: IpcError = serde_json::from_value(json).unwrap();
        assert!(matches!(back, IpcError::CloudSessionExpired { .. }));
        // Llega legible al usuario (toast / stdout), no como `{:?}`.
        assert!(back.to_string().contains("revoked"));
    }
}
