//! El reductor reconciliador puro (ADR 0021, C.1 + C.2 — Slice 2, paso 1).
//!
//! ```text
//! reconcile(&State, &Observation, World) -> (State, Vec<Decision>)
//! ```
//!
//! Determinista y sans-IO: toda la no-determinación entra por [`World`] (`now`,
//! `seed`). La autoridad está **invertida**: el tick es la fuente de verdad. Cada
//! tick el shell muestrea el mundo → construye una [`Observation`] → llama a
//! `reconcile` → ejecuta las [`Decision`]s. Los eventos (fs, realtime) son
//! *hints* que sólo adelantan un tick (llegan como `obs.fs_event` /
//! `obs.op_result`), nunca deciden por su cuenta.
//!
//! El veto de sesión se compone reusando [`session::veto_reason`]: `reconcile`
//! **es** el reconciliador de alto nivel; el veto es su sub-decisor.
//!
//! ## Invariantes (property tests con shrinking, más abajo)
//! - convergido ⇒ sólo `Hold` (cero `Act`).
//! - ninguna `Act` sin un delta en la entrada que la cause (`now` cruzando un
//!   deadline **es** delta → el retry tras un 429 no la viola).
//! - nunca `Act(Backup)` a la vez que `Act(Restore)` (no se pelean por la
//!   carpeta) y nunca `Act(Restore)` mid-session (data-loss REPO).
//! - nunca perder un local más nuevo que el remoto (`Restore` ⇒ sin
//!   `has_pending`).
//! - `Act` de storage acotadas por tick (≤ 1).
//! - un pull diferido nunca encalla la subida que lo destrabaría (D.8.1).

use rand::{rngs::StdRng, Rng, SeedableRng};
use time::{Duration, OffsetDateTime};

use super::{
    session, Action, ConflictStall, Decision, Observation, Op, OpResult, RestoreFailures, State,
    World,
};

// ---- Constantes de ritmo (réplica sans-IO de las de `agent.rs`) ------------

/// Cooldown mínimo entre intentos de restore (éxito o fallo), igual que
/// `agent::AUTO_RESTORE_COOLDOWN_SECS`.
pub const RESTORE_COOLDOWN_SECS: i64 = 60;

/// Backoff largo cuando el restore da 404 (el save no está en el backend), igual
/// que `agent::AUTO_RESTORE_NOT_FOUND_BACKOFF_SECS`.
pub const NOT_FOUND_BACKOFF_SECS: i64 = 60 * 60;

/// Escalada del backoff de restore que sigue fallando en la MISMA versión cloud:
/// 60 s → 5 min → 15 min → 60 min, luego 60 min para siempre. Igual que
/// `agent::AUTO_RESTORE_FAILURE_BACKOFF_SECS`.
pub const FAILURE_BACKOFF_SECS: [i64; 4] = [60, 5 * 60, 15 * 60, 60 * 60];

/// Fallos consecutivos en la misma versión antes de marcar el save "stuck".
pub const STUCK_AFTER: u32 = 3;

/// Backoff largo tras agotar el presupuesto de reintentos internos de una
/// **subida**. Diez minutos, deliberadamente mucho más lento que ese presupuesto
/// (segundos): lo que sobrevive a los reintentos no es un paquete perdido sino
/// una avería real —server caído, sin red, disco ilegible, token caducado— y eso
/// se resuelve en la escala de minutos u horas. Largo para no martillear un
/// backend muerto (y no pintar el feed de rojo), corto para que la recuperación
/// sea desatendida. Era `agent::BACKUP_RETRY_BACKOFF`, política en el shell
/// (ADR 0021 D.8.2).
pub const BACKUP_FAILURE_BACKOFF_SECS: i64 = 10 * 60;

/// Escalada de la subida que choca contra un conflicto **irresoluble** (409 «vas
/// por detrás» + reconciliación sin nada que bajar): 10 min → 20 → 40 → 80.
///
/// Exponencial y no plana como [`BACKUP_FAILURE_BACKOFF_SECS`] porque no es la
/// misma avería. Un fallo normal se cura solo —vuelve la red, arranca el
/// server— y el backoff sólo tiene que no martillear mientras tanto. Un
/// conflicto sin salida no se cura con tiempo: cada reintento vuelve a hacer la
/// misma pregunta y recibe la misma respuesta. Reintentar sirve por si la nube
/// se mueve, y eso pasa en la escala de una sesión de juego, no de diez minutos.
pub const CONFLICT_STALL_BACKOFF_SECS: [i64; 4] = [10 * 60, 20 * 60, 40 * 60, 80 * 60];

/// Conflictos irresolubles seguidos contra la misma cabeza de nube tras los
/// cuales se deja de reintentar y el save pasa a pedir una persona.
///
/// Cinco: los cuatro escalones de arriba (dos horas y media en total) y basta.
/// El caso real llevaba 14 días a ~4,5 intentos/h y sobrevivió a tres versiones
/// de la app sin que nadie lo mirara, porque nada lo enseñaba. Un reintento
/// silencioso infinito no es tolerancia a fallos: es un fallo escondido.
pub const CONFLICT_STALL_GIVE_UP_AFTER: u32 = 5;

/// Motivo del `Hold` de una subida que agotó su presupuesto de conflictos. La
/// UI lo enseña como "necesita que mires esto", así que es una constante y no un
/// literal (mismo trato que [`HOLD_BACKUP_MIN_INTERVAL`]).
pub const HOLD_BACKUP_NEEDS_ATTENTION: &str = "backup conflict needs the user";

/// Reposo tras un 402 (cuenta llena). Mucho más largo que el de un fallo
/// normal: liberar espacio es una acción humana —archivar juegos, subir a
/// Pro—, no una racha de red que se cura sola en diez minutos. Con veinte
/// saves en la biblioteca, el backoff de fallo normal serían ~120 POST a la
/// hora que ya sabemos que van a devolver 402.
pub const QUOTA_FULL_BACKOFF_SECS: i64 = 60 * 60;

/// Ceiling on the wait a 429 can ask this client to sit out.
///
/// The cap exists so a malformed or hostile `retry_after` can't park a save
/// until the next restart, not to second-guess our own server. It used to be
/// 300 s, which did second-guess it: `loopguard::QUOTA_WAIT_SECS` answers a full
/// account with 3600 — the same hour as [`QUOTA_FULL_BACKOFF_SECS`], picked so
/// both ends behave alike — and the client silently shortened it to five
/// minutes. One account spent four days at ~170 refusals an hour against a
/// brake that had already told it to come back in one.
///
/// Derived from `QUOTA_FULL_BACKOFF_SECS` rather than written as another 3600:
/// the two numbers mean the same thing (how long a wall a human has to move
/// stays a wall) and drifting apart would put the loop straight back.
pub const MAX_THROTTLE_WAIT_SECS: i64 = QUOTA_FULL_BACKOFF_SECS;

/// Cadencia fija del poll airbag a `/v1/cloud/sync`. **Fuente de verdad del
/// número**: `hoard_agent::prefs::CLOUD_POLL_INTERVAL_SECS` la re-exporta, para
/// que el umbral de obsolescencia de abajo se derive de la cadencia real en vez
/// de duplicar un literal que puede driftar.
pub const CLOUD_POLL_INTERVAL_SECS: i64 = 60;

/// Cuántos intervalos de poll pueden perderse antes de declarar ciega la
/// observación de nube. Cinco: uno o dos fallos seguidos son un hipo de red o
/// una suspensión corta y no merecen ruido; cinco minutos sin contacto ya no son
/// un hipo, son una avería (ADR 0021 D.10 — el poller murió y estuvo 47 min sin
/// que nada lo dijera).
pub const CLOUD_STALE_AFTER_POLLS: i64 = 5;

// Un solo poll perdido jamás puede declarar ciega la observación: la red hipa.
// Chequeado en compilación, no en test, para que ni siquiera compile mal.
const _: () = assert!(CLOUD_STALE_AFTER_POLLS >= 2);

/// Edad a partir de la cual [`Observation::cloud_version_as_of`] deja de ser
/// creíble y el reductor emite [`CLOUD_STALE_REASON`] en vez de `"converged"`.
pub const CLOUD_STALE_AFTER_SECS: i64 = CLOUD_POLL_INTERVAL_SECS * CLOUD_STALE_AFTER_POLLS;

/// Motivo del `Hold` cuando la caché de versiones de nube envejeció: no estamos
/// convergidos, estamos **ciegos**. Mismo principio que loguear los vetos — un
/// fallo invisible pasa a ser observable.
pub const CLOUD_STALE_REASON: &str = "cloud state stale";

/// Edad de [`Observation::cloud_version_as_of`] a partir de la cual **el motor
/// va a buscar la cabeza de nube él mismo** (ADR 0021 D.12), en vez de esperar
/// a que el poller del cliente se la empuje.
///
/// Por encima de la cadencia del poller a propósito: un poller vivo rejuvenece
/// la marca antes de llegar aquí, así que su feed *suprime* esta consulta y el
/// coste se queda en UN manifiesto por intervalo, no dos. Cuando el poller muere
/// —la avería de D.12, la tarea desaparecía sin un log— el motor cubre el hueco
/// solo: la degradación es "tardo hasta el siguiente tick", no "ciego para
/// siempre". Vive aquí, junto a la cadencia de la que se deriva, aunque quien
/// hace la consulta sea el shell (el kernel no hace IO).
pub const CLOUD_SELF_OBSERVE_AFTER_SECS: i64 = CLOUD_POLL_INTERVAL_SECS * 3 / 2;

// El motor SIEMPRE intenta refrescar antes de declararse ciego. Si esta relación
// se invirtiera, el `Hold{"cloud state stale"}` acusaría una obsolescencia que
// nadie ha intentado remediar todavía. Chequeado en compilación.
const _: () = assert!(CLOUD_SELF_OBSERVE_AFTER_SECS < CLOUD_STALE_AFTER_SECS);
// Y nunca por debajo de la cadencia del poller: si no, un poller sano y el motor
// se pisarían el manifiesto cada intervalo (dos GET donde debe haber uno).
const _: () = assert!(CLOUD_SELF_OBSERVE_AFTER_SECS >= CLOUD_POLL_INTERVAL_SECS);

/// Ventana de gracia (sticky) tras dejar de ver el proceso vivo antes de
/// declararlo parado. 6 s — bajada desde los 90 s históricos
/// (`agent::STRONG_STOP_GRACE_FLOOR_SECS`, "Was 90 s"): como el veto de sesión
/// se ancla en `is_running`, esos 90 s se sumaban a CADA GameStopped, inflando
/// la latencia de detección de cierre y la de restore cross-device (el receptor
/// seguía vetando pulls 90 s tras cerrarse el juego). Este es el corpus D.4
/// «sticky 90s→6s»: aquí es un invariante testeable de latencia de veto.
pub const RUNNING_STICKY_GRACE_SECS: i64 = 6;

// ---- El reductor -----------------------------------------------------------

/// Reconcilia el estado durable con el mundo muestreado y devuelve el nuevo
/// estado más las decisiones a ejecutar este tick. Determinista: mismas
/// entradas ⇒ misma salida (incluido el jitter, vía `StdRng::seed_from_u64`).
pub fn reconcile(state: &State, obs: &Observation, world: World) -> (State, Vec<Decision>) {
    let mut next = state.clone();
    let mut decisions: Vec<Decision> = Vec::new();
    let now = world.now;

    // Entrada playtime-only: no tiene carpeta que sincronizar, nunca.
    if next.track_only {
        return (next, vec![hold("track-only entry")]);
    }

    // Hint fs (una escritura debounced aterrizó este tick): marca pendiente.
    // Es un *hint* — sólo adelanta el tick, no decide.
    if obs.fs_event {
        next.has_pending = true;
        next.last_fs_event_at = Some(now);
    }

    // Status de sesión viva desde la evidencia de proceso, con stickiness.
    apply_running_stickiness(&mut next, obs, now);

    // La nube publicó una versión distinta de la que venía fallando: es
    // información nueva, no un reintento, así que la escalada de fallos muere y
    // el freno se suelta (D.8.2). Antes lo hacía el shell al recibir
    // `SetCloudVersions` — política fuera del kernel, invisible al replay de C.5.
    clear_restore_backoff_on_new_version(&mut next, obs);
    clear_conflict_stall_on_new_version(&mut next, obs);

    // Ingerir el resultado de una op en vuelo que acaba de terminar. Limpia
    // `in_flight` y actualiza contabilidad/backoff. Puede emitir `Throttle`.
    if let Some(result) = obs.op_result {
        ingest_op_result(&mut next, result, obs, now, world.seed, &mut decisions);
    }

    // Anti-relaunch: si sigue habiendo una op en vuelo (no llegó resultado este
    // tick), NO relanzar — subir/bajar GB tarda minutos. Retén con motivo.
    if next.in_flight.is_some() {
        decisions.push(hold("operation in flight"));
        return (next, decisions);
    }

    // ---- Decisión de restore (nube → local) --------------------------------
    // Se restaura si la carpeta local está vacía (desinstalada/fresca), la nube
    // va por delante (otro dispositivo subió una versión mayor) o quedó un pull
    // diferido de un tick anterior: `cloud_ahead` puede haber dejado de ser
    // demostrable desde la caché, pero `pull_pending` recuerda la intención (el
    // pull sobrevive al veto y aterriza al cerrarse el juego — bug del Deck).
    let ahead = cloud_ahead(&next, obs);
    let want_restore = next.restore_enabled && (obs.local_empty || ahead || next.pull_pending);
    if want_restore {
        // Cooldown / backoff de restore todavía activo (el 429 tras throttle
        // aterriza aquí; `now` cruzando el deadline es el delta que lo libera).
        let cooling = next.next_restore_at.is_some_and(|t| now < t);
        if cooling {
            decisions.push(hold("restore cooldown"));
        } else {
            match session::veto_reason(&next, obs, &world) {
                // Mid-session: nunca pull dentro de una carpeta viva (data-loss
                // REPO). Si hay una actualización real esperando, el pull se
                // DIFIERE en vez de perderse.
                Some(reason) => {
                    if ahead || next.pull_pending {
                        next.pull_pending = true;
                        // `deferred_notified` de-duplica SÓLO el aviso de UI, no
                        // la acción: guardar la *acción* dentro de un reductor
                        // level-triggered era el one-shot de flanco que
                        // encallaba el par (has_pending, cloud_ahead) (D.8.1).
                        if next.deferred_notified {
                            decisions.push(hold(reason));
                        } else {
                            next.deferred_notified = true;
                            decisions.push(Decision::Act(Action::DeferPull));
                        }
                    } else {
                        decisions.push(hold(reason));
                    }
                }
                // Tranquilo: restaura ahora.
                None => {
                    start_restore(&mut next, now);
                    decisions.push(Decision::Act(Action::Restore));
                    return (next, decisions);
                }
            }
        }
        // El pull no procede este tick (cooldown o veto) — pero el backup SÍ
        // puede: `has_pending` sólo lo limpia una subida, así que retornar aquí
        // dejaba el slot encallado mientras la nube fuese por delante (el veto
        // mira `has_pending`, y `has_pending` esperaba un backup que nunca se
        // emitía). Ése era el deadlock que el ejecutor de `DeferPull`
        // desatascaba a mano en el shell — política fuera del kernel (D.8.1).
        // El backup mid-session es la feature (autobackup con debounce mientras
        // juegas), no un bug: el invariante duro es que no se restaure, no que
        // no se suba. Y es *urgente*: mientras no aterrice, el pull sigue vetado.
        let urgent = ahead || next.pull_pending;
        if let Some(d) = decide_backup(&mut next, obs, now, urgent) {
            decisions.push(d);
        }
        return (next, decisions);
    }

    // ---- Decisión de backup (local → nube) ---------------------------------
    // Convergido si no hay nada que subir: nada que hacer (invariante base C.1).
    // Salvo que no estemos convergidos sino **ciegos**: si la observación de la
    // nube envejeció —o nunca llegó, teniendo nube que observar—,
    // `cloud_version` es una entrada mentirosa y el
    // `cloud_ahead = false` de arriba no demuestra nada. Se dice con su propio
    // motivo (ADR 0021 D.10) — el fallo del poller deja de disfrazarse de
    // normalidad. Sólo cambia el motivo del reposo: la subida no se toca, que un
    // poller muerto detenga los backups sería cambiar un fallo invisible por
    // pérdida de datos.
    let idle = if cloud_state_stale(obs, now) {
        hold(CLOUD_STALE_REASON)
    } else {
        hold("converged")
    };
    decisions.push(decide_backup(&mut next, obs, now, false).unwrap_or(idle));
    (next, decisions)
}

// ---- Helpers puros ---------------------------------------------------------

fn hold(reason: &'static str) -> Decision {
    Decision::Hold { reason }
}

/// Decide la subida local→nube, aislada para poder tomarse también cuando el
/// pull no procede (ver el deadlock de D.8.1). Devuelve:
///
/// - `Some(Act(Backup))` con un delta de contenido REAL (fingerprint distinto
///   del ya sincronizado) y el ritmo cumplido — marca la op en vuelo;
/// - `Some(Hold(...))` si un freno de ritmo aún no venció;
/// - `None` si no hay nada que subir (el llamante decide qué motivo poner).
///
/// Exigir divergencia real es lo que mata el hot-loop de compresión: un
/// `has_pending` espurio con contenido idéntico NO sube (convergido ⇒ 0
/// acciones).
///
/// `urgent` = esta subida es el *flush* que destraba un pull cross-device
/// (nube por delante o pull diferido en espera). Sólo entonces se salta el suelo
/// de ahorro de datos — nunca un backoff de error.
fn decide_backup(
    next: &mut State,
    obs: &Observation,
    now: OffsetDateTime,
    urgent: bool,
) -> Option<Decision> {
    if !(next.has_pending && local_diverged(next, obs)) {
        return None;
    }
    // La subida agotó su presupuesto de conflictos irresolubles: se para y se
    // pide una persona. Antes que cualquier freno de ritmo porque no es un
    // freno de ritmo — no hay deadline que cruzar que lo levante, sólo una
    // acción del usuario, un backup con éxito o una cabeza de nube nueva.
    if next.backup_conflict.needs_attention {
        return Some(hold(HOLD_BACKUP_NEEDS_ATTENTION));
    }
    // El juego está escribiendo el save ahora mismo: subirlo capturaría un
    // fichero a medias. Es un freno de ritmo, no un error — en cuanto suelte
    // el fichero, el siguiente tick sube. Antes que los backoffs porque es más
    // específico: da el motivo real en vez de "esperando".
    if obs.save_files_locked {
        return Some(hold("save files are open in another process"));
    }
    // Backoff de error (429 de subida / reintentos de backup agotados): nunca se
    // salta — saltárselo es martillear un backend caído o quemar la cuota.
    if next.next_backup_at.is_some_and(|t| now < t) {
        return Some(hold("backup backoff"));
    }
    // Suelo de min-interval (ahorro de datos, ADR 0018 eje A): pacing, no error.
    // Un flush que destraba un pull sí puede saltárselo — si no, el progreso
    // local se queda sin versionar, el veto por `has_pending` sigue en pie y la
    // actualización cross-device espera un intervalo entero (hasta 10 min en el
    // preset `data_saver`) antes de poder aterrizar.
    if !urgent && backup_floor(next).is_some_and(|t| now < t) {
        // Dos motivos distintos a propósito: uno es el ritmo que el usuario
        // eligió, el otro es el que le pusimos nosotros por cómo se comporta su
        // juego. En un log valen lo mismo hasta que hay que explicarle a alguien
        // por qué su partida "tarda" — y entonces valen cosas muy distintas.
        return Some(hold(if next.min_backup_interval_secs > 0 {
            HOLD_BACKUP_MIN_INTERVAL
        } else {
            HOLD_BACKUP_BURST
        }));
    }
    next.in_flight = Some(Op::Backup);
    Some(Decision::Act(Action::Backup))
}

/// Los dos motivos de retención que significan "hay algo que subir y sube en un
/// rato", frente a los que significan "no se puede subir" (backoff de error,
/// fichero abierto). Son constantes y no literales porque el shell decide por
/// ellos si enseñar la espera en la UI, y un motivo que se renombra aquí y no
/// allí devuelve el suelo a ser invisible — que es justo por lo que hubo que
/// revertir el primero.
pub const HOLD_BACKUP_MIN_INTERVAL: &str = "backup min-interval";
pub const HOLD_BACKUP_BURST: &str = "backup autosave burst";

/// ¿Es este motivo una espera con hora, que la UI debería poder enseñar?
pub fn hold_is_paced_backup(reason: &str) -> bool {
    reason == HOLD_BACKUP_MIN_INTERVAL || reason == HOLD_BACKUP_BURST
}

/// Ventana en la que se cuentan los commits de un save para decidir si el juego
/// está reescribiendo su autoguardado en bucle.
pub const BURST_WINDOW_SECS: i64 = 600;
/// Commits dentro de esa ventana a partir de los cuales se impone el suelo. Tres
/// en diez minutos ya es más de lo que ningún historial aprovecha.
pub const BURST_THRESHOLD: u32 = 3;
/// El suelo que se impone entonces, y el **único** escalón que hay: no escala
/// con la frecuencia. Un juego que autoguarda cada seis segundos pasa de una
/// versión cada seis segundos a una por minuto, y ahí se queda.
pub const BURST_FLOOR_SECS: u64 = 60;

/// El suelo que rige de verdad para este save.
///
/// Un intervalo explícito manda siempre: lo puso un preset que el usuario ve y
/// eligió (`short_session` 30 s para un juego que se borra la carpeta entre
/// rondas, `data_saver` 600 s), y subírselo por su cuenta traicionaría justo lo
/// que se pidió. El adaptativo sólo rellena el hueco de "sin suelo ninguno", que
/// es el default y hasta ahora significaba literalmente ninguno: un save llegó a
/// 2.233 versiones en un día, 1.027 subidas en cuatro horas y media, porque el
/// juego reescribía `auto.sav` cada pocos segundos y cada reescritura era una
/// versión en la nube (ago-2026).
fn effective_min_interval(state: &State) -> u64 {
    if state.min_backup_interval_secs > 0 {
        return state.min_backup_interval_secs;
    }
    if state.burst_backups >= BURST_THRESHOLD {
        BURST_FLOOR_SECS
    } else {
        0
    }
}

/// El suelo de min-interval, **derivado** de `last_backup_at +
/// [`effective_min_interval`]` en vez de almacenado en `next_backup_at`.
/// Separarlo del backoff es lo que permite distinguir "pacing de ahorro"
/// (saltable por un flush cross-device) de "backoff de error" (jamás), y de paso
/// hace del ancla —`last_backup_at`, que sólo avanza con un commit real— la
/// única memoria del suelo: un no-op no puede empujarlo (regresión R.E.P.O.,
/// D.8.2).
///
/// Public because the shell needs the same number to *show* it: a wait nobody
/// can see reads as "Hoard isn't picking up my changes", which is why the first
/// attempt at a fixed floor had to be reverted. The shell asks for the deadline
/// and puts it in `next_scheduled_backup_at`, where the UI's "next copy in Xs"
/// already reads from.
pub fn backup_floor(state: &State) -> Option<OffsetDateTime> {
    let secs = effective_min_interval(state);
    if secs == 0 {
        return None;
    }
    state
        .last_backup_at
        .map(|t| t + Duration::seconds(secs as i64))
}

/// Cuenta este commit en la ventana de ráfaga, abriéndola de cero si la anterior
/// ya venció. Se llama **sólo** con un commit real, por lo mismo que
/// `last_backup_at`: un no-op no es actividad del juego y no puede empujar a un
/// save tranquilo al suelo adaptativo.
fn count_burst(state: &mut State, now: OffsetDateTime) {
    let open = state
        .burst_since
        .is_some_and(|t| now - t <= Duration::seconds(BURST_WINDOW_SECS));
    if open {
        state.burst_backups = state.burst_backups.saturating_add(1);
    } else {
        state.burst_since = Some(now);
        state.burst_backups = 1;
    }
}

/// Suelta la escalada de fallos de restore cuando la nube publica una versión
/// distinta de aquella contra la que se estaba fallando (D.8.2). El backoff era
/// sobre *esa* versión; una nueva es contenido nuevo y una razón fresca para
/// reintentar ya, no para heredar la penalización. Sólo actúa con una escalada
/// viva, para no pisar el cooldown normal post-restore.
fn clear_restore_backoff_on_new_version(next: &mut State, obs: &Observation) {
    let active = next.restore_failures.consecutive > 0 || next.restore_failures.stuck_notified;
    if active && next.restore_failures.version != obs.cloud_version {
        next.restore_failures = RestoreFailures::default();
        next.next_restore_at = None;
    }
}

/// Arranca un restore: marca la op en vuelo y arma el cooldown. Un pull diferido
/// pendiente se considera consumido (lo estamos ejecutando).
fn start_restore(next: &mut State, now: OffsetDateTime) {
    next.in_flight = Some(Op::Restore);
    next.next_restore_at = Some(now + Duration::seconds(RESTORE_COOLDOWN_SECS));
    next.pull_pending = false;
    next.deferred_notified = false;
}

/// ¿La caché del poller dice que el save avanzó más allá de lo que este
/// dispositivo tiene? Una versión cacheada sin `known_version` cuenta como
/// adelantada (nunca sincronizamos este save). Sin entrada de caché: no sabemos,
/// nunca lo afirmamos. Réplica de `agent::cloud_ahead`.
fn cloud_ahead(state: &State, obs: &Observation) -> bool {
    match obs.cloud_version {
        Some(latest) => state.known_version.is_none_or(|known| latest > known),
        None => false,
    }
}

/// ¿La observación de la nube dejó de ser creíble? `true` cuando lo último que
/// sabemos de ella es más viejo que [`CLOUD_STALE_AFTER_SECS`].
///
/// Dos formas de estar ciego, una sola cuenta atrás:
///
/// - **Feed rancio** — hubo cabezas y dejaron de llegar: envejece desde la
///   marca ([`Observation::cloud_version_as_of`]).
/// - **Nunca hubo feed** — la ceguera más grave, y la que se colaba como
///   `converged` hasta el remate de D.11: envejece desde
///   [`Observation::cloud_feed_expected_since`], el momento en que el motor
///   empezó a esperar cabezas. Sin ese ancla (self-hosted, daemon CLI, contexto
///   sin resolver) no hay nube que observar y no se reporta nada: la distinción
///   es *contexto cloud vs self-hosted*, no `None` vs `Some`.
///
/// Un `now` anterior al ancla (salto de reloj hacia atrás) tampoco es
/// obsolescencia — la resta sale negativa.
fn cloud_state_stale(obs: &Observation, now: OffsetDateTime) -> bool {
    let anchor = match obs.cloud_version_as_of {
        Some(as_of) => Some(as_of),
        None => obs.cloud_feed_expected_since,
    };
    anchor.is_some_and(|t| (now - t).whole_seconds() > CLOUD_STALE_AFTER_SECS)
}

/// ¿El contenido local difiere del ya sincronizado? Con fingerprint L1 calculado,
/// compara; sin él (no se hasheó este tick), confía en `has_pending` (el hint fs
/// dijo que algo cambió). El caso `Some(fp) == synced` es el que hace convergido
/// ⇒ 0 acciones aunque `has_pending` esté puesto por un settle espurio.
fn local_diverged(state: &State, obs: &Observation) -> bool {
    match obs.local_fingerprint {
        Some(fp) => state.synced_fingerprint != Some(fp),
        None => true,
    }
}

/// Deriva `is_running` (status durable) de la evidencia de proceso con ventana
/// de gracia sticky: un match por correlación es CPU-gated y puede caer bajo el
/// umbral un tick; sin gracia eso flapea GameStarted/Stopped. Mantiene el slot
/// "corriendo" hasta que `last_running_seen` supere [`RUNNING_STICKY_GRACE_SECS`].
fn apply_running_stickiness(next: &mut State, obs: &Observation, now: OffsetDateTime) {
    if obs.process_alive {
        next.is_running = true;
        next.last_running_seen = Some(now);
    } else if next.is_running {
        let expired = next
            .last_running_seen
            .is_none_or(|seen| (now - seen).whole_seconds() >= RUNNING_STICKY_GRACE_SECS);
        if expired {
            next.is_running = false;
        }
    }
}

/// Ingiere el resultado de una op terminada: limpia `in_flight` y aplica la
/// disposición. Mapea 1:1 a `agent`'s `AutoRestoreDisposition` + `BackupDone`.
/// El 429 (`Throttled`) es **simétrico** backup/restore: frena la op correcta y
/// **no** toca el contador de fallos; `Failed` también distingue op (una subida
/// fallida se re-arma en su backoff largo, no escala la escalada del restore).
fn ingest_op_result(
    next: &mut State,
    result: OpResult,
    obs: &Observation,
    now: OffsetDateTime,
    seed: u64,
    decisions: &mut Vec<Decision>,
) {
    let op = next.in_flight.take();
    match result {
        OpResult::Ok {
            version,
            fingerprint,
            wrote,
        } => {
            // Un restore puede volver `Ok` sin haber movido nada: se baja el
            // snapshot, se difunde contra la carpeta y el diff decide que no hay
            // que escribir. Si además la carpeta sigue vacía, el disparador que
            // lo trajo —`local_empty`, que puentea a propósito la puerta de
            // versión— sigue siendo cierto en el tick siguiente y volvemos a
            // bajar el mismo snapshot. Para siempre, y al precio completo de la
            // descarga: un cliente se comió así 3.752 bajadas y 10,6 GB entre el
            // 2026-07-27 y el 08-03 sin escribir un byte en disco.
            //
            // La escalada de fallos es lo único capaz de frenar eso, así que un
            // "éxito" que no progresa no puede limpiarla. `!wrote` y carpeta
            // vacía es la única combinación inequívoca: si se escribió algo,
            // hubo progreso aunque la observación llegue tarde.
            let restore_stalled = matches!(op, Some(Op::Restore)) && !wrote && obs.local_empty;
            if !restore_stalled {
                next.restore_failures = RestoreFailures::default();
            }
            if version.is_some() {
                next.known_version = version;
            }
            if fingerprint.is_some() {
                next.synced_fingerprint = fingerprint;
            }
            match op {
                Some(Op::Backup) => {
                    // El contenido llegó a una versión (o ya estaba en una): los
                    // cambios dejan de estar sin versionar en ambos casos.
                    next.has_pending = false;
                    // Y sea commit o no-op, la subida ya no está atascada: el
                    // 409 irresoluble se resolvió. Se suelta la escalada entera,
                    // que es también lo que apaga el aviso en la UI.
                    next.backup_conflict = ConflictStall::default();
                    if wrote {
                        // Commit real: mueve el ancla del min-interval (ADR 0018).
                        // El suelo se deriva de ella ([`backup_floor`]); no hace
                        // falta —ni conviene— escribirlo en `next_backup_at`, que
                        // es el carril de los backoffs de error.
                        next.last_backup_at = Some(now);
                        count_burst(next, now);
                    } else {
                        // No-op (skip por firma, vacío, archived, too-large, el
                        // 409 asentado a la cabeza, o la subida que ya había
                        // aterrizado): **no** es un backup, así que no mueve el
                        // ancla del min-interval — hacerlo empujaría la siguiente
                        // subida real un intervalo entero y una sesión corta
                        // nunca volcaría su progreso (regresión R.E.P.O., D.8.2).
                        //
                        // Un no-op CON versión es normalmente el 409
                        // non-fast-forward asentado a la cabeza: el merge
                        // escribió en la carpeta igual que un restore, así que se
                        // sella `last_restore_at` para que ese toque nuestro no
                        // vete el siguiente pull.
                        //
                        // Salvo que la respuesta del chequeo content-addressed
                        // diga que el contenido **ya estaba** arriba (D.8.3): ahí
                        // no se escribió un solo byte en la carpeta, y sellar un
                        // toque que no existió falsearía la ventana de gracia del
                        // veto — el kernel se creería autor de una escritura
                        // ajena y dejaría pasar un pull que debía esperar.
                        if version.is_some() && obs.upload_landed != Some(true) {
                            next.last_restore_at = Some(now);
                        }
                    }
                }
                Some(Op::Restore) => {
                    // Sólo un write real toca la carpeta y debe sellar
                    // `last_restore_at` (evita auto-vetar el siguiente pull).
                    if wrote {
                        next.last_restore_at = Some(now);
                    }
                    next.pull_pending = false;
                    next.deferred_notified = false;
                    // Bajada que no progresó: escala por la misma escalera que un
                    // fallo (60 s → 5 min → 15 min → 60 min). No es un error —el
                    // servidor respondió— pero repetirlo cada tick tampoco es
                    // sincronizar, y la carpeta vacía volverá a pedirlo igual.
                    // Una versión cloud nueva resetea la escalada por
                    // `clear_restore_backoff_on_new_version`, así que un pull
                    // legítimo posterior no queda castigado.
                    if restore_stalled {
                        let delay = record_failure(&mut next.restore_failures, obs.cloud_version);
                        next.next_restore_at = Some(now + Duration::seconds(delay));
                    }
                }
                None => {}
            }
        }
        // 404: aparcar en el backoff largo (concepto de restore).
        OpResult::NotFound => {
            next.next_restore_at = Some(now + Duration::seconds(NOT_FOUND_BACKOFF_SECS));
        }
        // 401: no es culpa del save. Cooldown corto, contador intacto.
        OpResult::Unauthorized => {
            next.next_restore_at = Some(now + Duration::seconds(RESTORE_COOLDOWN_SECS));
        }
        // 429: backoff simétrico según la op; contador de fallos intacto.
        OpResult::Throttled { retry_after_secs } => {
            let until = throttle_until(now, retry_after_secs, seed);
            match op {
                Some(Op::Backup) => next.next_backup_at = Some(until),
                _ => next.next_restore_at = Some(until),
            }
            decisions.push(Decision::Act(Action::Throttle { until }));
        }
        // 402: la cuenta está llena. Sólo frena subidas —una bajada no consume
        // cuota, así que un restore pendiente debe seguir su camino— y deja
        // `has_pending` intacto para que el slot siga vetado del pull mientras
        // los cambios estén sólo en disco.
        OpResult::QuotaFull => {
            if matches!(op, Some(Op::Backup)) {
                next.next_backup_at = Some(now + Duration::seconds(QUOTA_FULL_BACKOFF_SECS));
            }
        }
        // 409 sin salida: el server dice que vamos por detrás y no hay nada que
        // bajar. Escala por su propia escalera y, agotada, deja de reintentar:
        // `needs_attention` es lo que `decide_backup` mira para no volver a
        // emitir la subida. Como el `Failed` de una subida, conserva
        // `has_pending` — los cambios locales siguen sin versionar.
        OpResult::ConflictStalled => {
            if let Some(delay) = record_conflict(&mut next.backup_conflict, obs.cloud_version) {
                next.next_backup_at = Some(now + Duration::seconds(delay));
            }
        }
        // Otro error, según la op:
        // - subida: agotó su presupuesto de reintentos internos → se re-arma en
        //   el backoff largo y **conserva** `has_pending` (los cambios nunca
        //   llegaron a una versión; perderlos dejaría que un restore los pisara).
        //   Antes lo hacía el shell en `RetryBackupAfterFailure` (D.8.2).
        // - bajada (o sin op en vuelo, como antes): escala el contador de fallos
        //   por versión cloud y el backoff de restore.
        OpResult::Failed => match op {
            Some(Op::Backup) => {
                next.next_backup_at = Some(now + Duration::seconds(BACKUP_FAILURE_BACKOFF_SECS));
            }
            _ => {
                let delay = record_failure(&mut next.restore_failures, obs.cloud_version);
                next.next_restore_at = Some(now + Duration::seconds(delay));
            }
        },
    }
}

/// Registra un fallo de restore contra la versión cloud observada
/// (`obs.cloud_version` — la cabeza que intentábamos traernos, igual que el
/// `latest_versions.get(id)` del motor original) y devuelve el backoff a
/// aplicar. Réplica sans-IO de `AutoRestoreFailures::record_failure`: una versión
/// distinta resetea la escalada, que es la otra mitad de
/// [`clear_restore_backoff_on_new_version`]. El segundo valor de la tupla del
/// original —"emit stuck"— lo decide el shell leyendo `stuck_notified`.
fn record_failure(f: &mut RestoreFailures, latest: Option<i64>) -> i64 {
    if f.version != latest {
        f.version = latest;
        f.consecutive = 0;
        f.stuck_notified = false;
    }
    f.consecutive = f.consecutive.saturating_add(1);
    if f.consecutive >= STUCK_AFTER {
        f.stuck_notified = true;
    }
    backoff_secs(f.consecutive)
}

/// Registra un conflicto irresoluble contra la cabeza de nube observada y
/// devuelve el backoff a aplicar, o `None` cuando el presupuesto se agotó y hay
/// que dejar de reintentar.
///
/// Una cabeza distinta de la contada resetea la escalada, por el mismo motivo
/// que en [`record_failure`]: la nube se movió, así que la pregunta ya no es la
/// misma y quizá ahora sí haya algo que bajar.
fn record_conflict(c: &mut ConflictStall, latest: Option<i64>) -> Option<i64> {
    if c.version != latest {
        *c = ConflictStall {
            version: latest,
            ..ConflictStall::default()
        };
    }
    c.consecutive = c.consecutive.saturating_add(1);
    if c.consecutive >= CONFLICT_STALL_GIVE_UP_AFTER {
        c.needs_attention = true;
        return None;
    }
    let idx = (c.consecutive as usize - 1).min(CONFLICT_STALL_BACKOFF_SECS.len() - 1);
    Some(CONFLICT_STALL_BACKOFF_SECS[idx])
}

/// Suelta la escalada de conflictos cuando la nube publica una cabeza distinta
/// de aquella contra la que se atascó. Gemelo de
/// [`clear_restore_backoff_on_new_version`], y hace falta por separado: un save
/// que ya se rindió no vuelve a ingerir un `ConflictStalled` —deja de
/// reintentar—, así que sin esto nada podría desatascarlo salvo el usuario, ni
/// siquiera el otro dispositivo publicando la versión que faltaba.
fn clear_conflict_stall_on_new_version(next: &mut State, obs: &Observation) {
    let active = next.backup_conflict.consecutive > 0 || next.backup_conflict.needs_attention;
    if active && next.backup_conflict.version != obs.cloud_version {
        next.backup_conflict = ConflictStall::default();
        next.next_backup_at = None;
    }
}

/// Backoff dado el nº de fallos consecutivos (1-based). Satura en el último
/// escalón. Igual que `agent::auto_restore_backoff`.
fn backoff_secs(failures: u32) -> i64 {
    let idx = (failures.max(1) as usize - 1).min(FAILURE_BACKOFF_SECS.len() - 1);
    FAILURE_BACKOFF_SECS[idx]
}

/// Deadline del backoff de throttle: espera del server (clamp 1..=300, +2) más
/// jitter por-save. El jitter usa `StdRng::seed_from_u64(seed)` — **nunca**
/// `thread_rng` (ADR C.2: la sim y el replay deben ser deterministas). En el
/// motor invertido el shell deriva `seed` del `save_id`, replicando el
/// `hash(id) % 6` original de forma inyectable.
fn throttle_until(now: OffsetDateTime, retry_after_secs: u32, seed: u64) -> OffsetDateTime {
    let wait = (u64::from(retry_after_secs)).clamp(1, MAX_THROTTLE_WAIT_SECS as u64) + 2;
    let mut rng = StdRng::seed_from_u64(seed);
    let jitter: u64 = rng.gen_range(0..6);
    now + Duration::seconds((wait + jitter) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const BASE: OffsetDateTime = OffsetDateTime::UNIX_EPOCH;

    fn at(off: i64) -> OffsetDateTime {
        BASE + Duration::seconds(off)
    }

    fn world(now_off: i64) -> World {
        World {
            now: at(now_off),
            seed: 0,
        }
    }

    /// Slot real (no track-only) con restore habilitado y nada en curso.
    fn base_state() -> State {
        State {
            restore_enabled: true,
            ..Default::default()
        }
    }

    /// Observación "quiescente": sin señales puntuales (fs/op), carpeta poblada,
    /// nube no adelantada, proceso muerto. El punto de partida de "convergido".
    fn quiet_obs() -> Observation {
        Observation {
            folder_mtime: Some(at(-10_000)), // muy vieja: el fallback de disco no salta
            ..Default::default()
        }
    }

    fn acts(ds: &[Decision]) -> Vec<&Action> {
        ds.iter().filter_map(Decision::action).collect()
    }

    fn storage_act_count(ds: &[Decision]) -> usize {
        ds.iter()
            .filter(|d| matches!(d.action(), Some(Action::Backup) | Some(Action::Restore)))
            .count()
    }

    // ==== Corpus D.4 (escenarios deterministas fijos) =======================

    /// D.4 — «hot-loop de compresión (1,29M ops R2)»: convergido ⇒ 0 acciones.
    /// El bug: se emitían acciones (comprimir/subir) sin ningún delta de entrada.
    /// Aquí, con el fingerprint local IGUAL al sincronizado, ni un `has_pending`
    /// espurio dispara backup: sólo `Hold { "converged" }`.
    #[test]
    fn d4_converged_emits_zero_actions() {
        let state = State {
            has_pending: true, // settle espurio del watcher
            synced_fingerprint: Some(0xABCD),
            known_version: Some(7),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(0xABCD), // contenido idéntico a lo ya subido
            cloud_version: Some(7),          // nube no adelantada
            ..quiet_obs()
        };
        let (_next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            acts(&ds).is_empty(),
            "convergido debe emitir cero Act, salió: {ds:?}"
        );
        assert_eq!(ds, vec![hold("converged")]);
    }

    /// D.4 — «hot-loop», forma dinámica: dos ticks idénticos seguidos no emiten
    /// una segunda acción (ninguna `Act` sin delta). Un backup arranca una vez;
    /// el segundo tick lo ve en vuelo y retiene.
    #[test]
    fn d4_no_action_without_a_delta() {
        let state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2), // difiere → hay delta real la 1ª vez
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs, world(0));
        assert_eq!(acts(&d1), vec![&Action::Backup], "el delta real sube");
        // Mismo mundo, mismo now: sin nuevo delta no hay segunda acción.
        let (_s2, d2) = reconcile(&s1, &obs, world(0));
        assert!(
            acts(&d2).is_empty(),
            "sin nuevo delta no debe re-actuar, salió: {d2:?}"
        );
        assert_eq!(d2, vec![hold("operation in flight")]);
    }

    /// D.4 — «429 en restore» + simetría backup/restore. El throttle frena la op
    /// correcta y NO toca el contador de fallos; `now` cruzando el deadline lo
    /// libera. Antes el throttle sólo se manejaba en backup (asimétrico).
    #[test]
    fn d4_throttle_is_symmetric_and_does_not_count_as_failure() {
        // Restore throttled.
        let state = State {
            in_flight: Some(Op::Restore),
            known_version: Some(3),
            ..base_state()
        };
        let obs = Observation {
            local_empty: true, // querríamos restaurar
            cloud_version: Some(5),
            op_result: Some(OpResult::Throttled {
                retry_after_secs: 30,
            }),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            matches!(
                ds.iter().find_map(Decision::action),
                Some(Action::Throttle { .. })
            ),
            "un 429 de restore emite Throttle: {ds:?}"
        );
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "un throttle NO cuenta como fallo"
        );
        let until = next
            .next_restore_at
            .expect("restore frenado hasta un deadline");
        assert!(until > at(0), "el backoff mira al futuro");

        // Antes del deadline: cooldown, sin restore.
        let obs_after = Observation {
            local_empty: true,
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let mid = (until - at(0)).whole_seconds() / 2;
        let (_n, ds_mid) = reconcile(&next, &obs_after, world(mid));
        assert_eq!(ds_mid, vec![hold("restore cooldown")]);

        // Cruzado el deadline (delta legítimo): el restore procede.
        let past = (until - at(0)).whole_seconds() + 1;
        let (_n2, ds_past) = reconcile(&next, &obs_after, world(past));
        assert_eq!(
            acts(&ds_past),
            vec![&Action::Restore],
            "tras el backoff, restaura"
        );

        // Simetría: el MISMO throttle en un backup frena `next_backup_at`, no el
        // restore.
        let bstate = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let bobs = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: 30,
            }),
            ..quiet_obs()
        };
        let (bn, bds) = reconcile(&bstate, &bobs, world(0));
        assert!(
            bn.next_backup_at.is_some(),
            "el throttle de backup frena el backup"
        );
        assert!(bn.next_restore_at.is_none(), "sin tocar el lado restore");
        assert!(
            matches!(
                bds.iter().find_map(Decision::action),
                Some(Action::Throttle { .. })
            ),
            "backup también emite Throttle: {bds:?}"
        );
    }

    /// The hour the server's brake asks for has to survive the cap.
    ///
    /// `loopguard::QUOTA_WAIT_SECS` answers a full account with 3600 —
    /// deliberately the same as [`QUOTA_FULL_BACKOFF_SECS`] — and the 300 s cap
    /// that used to live here silently shortened it to five minutes: twelve
    /// retries an hour, per save, against a wall only a person can move.
    #[test]
    fn a_server_asking_for_an_hour_gets_an_hour() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: QUOTA_FULL_BACKOFF_SECS as u32,
            }),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        let waited = (next.next_backup_at.expect("parked") - at(0)).whole_seconds();
        assert!(
            waited >= QUOTA_FULL_BACKOFF_SECS,
            "the cap must not shorten what the server asked for: {waited}s"
        );

        // And the cap still exists: an absurd `retry_after` can't park the save
        // until the next restart.
        let obs_bogus = Observation {
            op_result: Some(OpResult::Throttled {
                retry_after_secs: u32::MAX,
            }),
            ..quiet_obs()
        };
        let (bogus, _) = reconcile(&state, &obs_bogus, world(0));
        let capped = (bogus.next_backup_at.expect("parked") - at(0)).whole_seconds();
        assert!(
            capped <= MAX_THROTTLE_WAIT_SECS + 8,
            "a junk retry_after gets capped: {capped}s"
        );
    }

    /// Cuenta llena (402): aparca la subida una hora, conserva `has_pending`
    /// (los bytes siguen sólo en disco) y no cuenta como fallo del save. Y no
    /// toca el lado restore: bajar no consume cuota.
    #[test]
    fn quota_full_parks_the_upload_without_blaming_the_save() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            op_result: Some(OpResult::QuotaFull),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        let until = next.next_backup_at.expect("la subida queda aparcada");
        assert_eq!(
            (until - at(0)).whole_seconds(),
            QUOTA_FULL_BACKOFF_SECS,
            "el park del 402 es el largo, no el de un fallo cualquiera"
        );
        assert!(next.has_pending, "los cambios locales siguen sin versión");
        assert!(next.next_restore_at.is_none(), "sin tocar el lado restore");
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "una cuenta llena NO es un save roto"
        );

        // Antes del deadline no se reintenta; cruzado, la subida vuelve a salir.
        let obs_quiet = quiet_obs();
        let (_m, ds_mid) = reconcile(&next, &obs_quiet, world(QUOTA_FULL_BACKOFF_SECS / 2));
        assert!(
            !acts(&ds_mid).contains(&&Action::Backup),
            "dentro del park no se reintenta: {ds_mid:?}"
        );
        let (_p, ds_past) = reconcile(&next, &obs_quiet, world(QUOTA_FULL_BACKOFF_SECS + 1));
        assert!(
            acts(&ds_past).contains(&&Action::Backup),
            "pasada la hora vuelve a intentarlo: {ds_past:?}"
        );
    }

    /// D.4 — «deferred-pull que no aterrizaba». Mid-session con la nube por
    /// delante ⇒ se DIFIERE (una sola notificación) y sobrevive al veto; al
    /// cerrarse el juego (sin pendientes) el pull ATERRIZA.
    #[test]
    fn d4_deferred_pull_survives_veto_and_lands_on_close() {
        // Mid-session: proceso vivo, nube por delante.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            known_version: Some(4),
            ..base_state()
        };
        let obs_playing = Observation {
            process_alive: true,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs_playing, world(0));
        assert_eq!(
            acts(&d1),
            vec![&Action::DeferPull],
            "1ª vez: difiere y notifica"
        );
        assert!(s1.pull_pending && s1.deferred_notified);

        // Sigue jugando: ya no re-notifica, retiene con el motivo del veto.
        let (s2, d2) = reconcile(&s1, &obs_playing, world(1));
        assert!(acts(&d2).is_empty(), "no re-notifica cada tick");
        assert_eq!(d2, vec![hold("game process is running")]);
        assert!(s2.pull_pending, "el pull diferido sobrevive");

        // Juego cerrado hace >6 s (sticky expira) y nada pendiente: aterriza.
        let obs_closed = Observation {
            process_alive: false,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (s3, d3) = reconcile(&s2, &obs_closed, world(10));
        assert_eq!(
            acts(&d3),
            vec![&Action::Restore],
            "al cerrar, el pull aterriza"
        );
        assert!(!s3.pull_pending && !s3.deferred_notified, "consumido");
    }

    /// D.4 — «sticky 90s→6s» como invariante de latencia de veto. El proceso
    /// muere; dentro de la ventana de 6 s el veto de sesión aún retiene (gracia
    /// anti-flapeo), pero JUSTO pasada la ventana se levanta — no a los 90 s.
    #[test]
    fn d4_veto_latency_is_six_seconds_not_ninety() {
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            known_version: Some(1),
            ..base_state()
        };
        let obs = Observation {
            process_alive: false, // el juego se cerró
            local_empty: true,    // hay algo que restaurar
            cloud_version: Some(2),
            ..quiet_obs()
        };
        // A los 5 s: dentro de la gracia, sigue "corriendo" → difiere/retiene.
        let (_n5, d5) = reconcile(&state, &obs, world(5));
        assert!(
            !acts(&d5).contains(&&Action::Restore),
            "dentro de la gracia el veto aún retiene: {d5:?}"
        );
        // A los 7 s: pasada la gracia de 6 s, el veto se levanta y restaura.
        let (n7, d7) = reconcile(&state, &obs, world(7));
        assert!(!n7.is_running, "pasada la gracia, deja de correr");
        assert_eq!(
            acts(&d7),
            vec![&Action::Restore],
            "el veto se levanta a los 6 s, no a los 90"
        );
    }

    /// Regresión: un restore que vuelve `Ok` sin escribir y deja la carpeta
    /// vacía NO es progreso. `local_empty` puentea la puerta de versión a
    /// propósito, así que sin frenar aquí el tick siguiente vuelve a pedir el
    /// mismo snapshot y el par (bajar, no escribir) se repite eternamente al
    /// precio completo de la descarga — 3.752 bajadas / 10,6 GB en producción
    /// entre 2026-07-27 y 08-03.
    #[test]
    fn restore_ok_sin_escribir_y_carpeta_vacia_no_reintenta_de_inmediato() {
        let state = State {
            known_version: Some(2),
            in_flight: Some(Op::Restore),
            ..base_state()
        };
        // La op vuelve OK, sin escribir, y la carpeta sigue vacía.
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(2),
                fingerprint: None,
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (n1, d1) = reconcile(&state, &obs, world(0));
        assert!(
            !acts(&d1).contains(&&Action::Restore),
            "no puede relanzar el restore en el mismo tick: {d1:?}"
        );
        assert_eq!(
            n1.restore_failures.consecutive, 1,
            "el 'éxito' que no progresa cuenta como intento, no limpia la escalada"
        );
        assert!(n1.next_restore_at.is_some(), "queda un backoff armado");

        // Tick siguiente con la carpeta aún vacía: retenido por el cooldown, no
        // otra descarga.
        let obs2 = Observation {
            local_empty: true,
            cloud_version: Some(2),
            ..quiet_obs()
        };
        let (_n2, d2) = reconcile(&n1, &obs2, world(1));
        assert!(
            !acts(&d2).contains(&&Action::Restore),
            "sigue frenado mientras dura el backoff: {d2:?}"
        );

        // Y un restore que SÍ escribe limpia la escalada: el freno es sólo para
        // el que no progresa.
        let obs_ok = Observation {
            local_empty: false,
            cloud_version: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(2),
                fingerprint: None,
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (n3, _d3) = reconcile(&state, &obs_ok, world(0));
        assert_eq!(
            n3.restore_failures.consecutive, 0,
            "un restore con escritura real sí es progreso"
        );
    }

    /// D.4 — nunca `Act(Restore)` con cambios locales sin versionar (never lose
    /// newer local): `has_pending` es motivo de veto; con la nube por delante se
    /// difiere en vez de pisar el progreso local. Y —desde D.8.1— el mismo tick
    /// suelta la subida: diferir el pull no puede dejar el progreso local sin
    /// versionar, porque `has_pending` sólo lo limpia un backup.
    #[test]
    fn d4_never_restore_over_unflushed_local() {
        let state = State {
            has_pending: true,
            known_version: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_empty: false,
            cloud_version: Some(9),
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &obs, world(0));
        assert!(
            !acts(&ds).contains(&&Action::Restore),
            "no restaurar sobre local sin versionar: {ds:?}"
        );
        assert_eq!(
            acts(&ds),
            vec![&Action::DeferPull, &Action::Backup],
            "se difiere el pull y se vuelca lo local"
        );
    }

    // ==== Corpus D.8 (revisión de 2b: la política que faltaba en el kernel) ==

    /// D.8.1 — «deadlock `has_pending` + `cloud_ahead`». Dos adelantos de nube
    /// en la MISMA sesión, sin cierre de juego de por medio, no encallan el slot.
    ///
    /// El bug: el reductor retenía el pull (correcto) y retornaba antes de la
    /// rama de backup, así que `has_pending` —que sólo limpia una subida— se
    /// quedaba puesto para siempre; y como `has_pending` es a su vez motivo de
    /// veto, ni se subía ni se bajaba. Lo desatascaba el *ejecutor* de
    /// `DeferPull` en el shell (`agent.rs`), política fuera del kernel e
    /// invisible al replay de C.5.
    #[test]
    fn d8_two_cloud_advances_in_one_session_do_not_wedge() {
        // Sesión viva: el juego corre y hay progreso local sin versionar.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            has_pending: true,
            known_version: Some(4),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        // 1er adelanto de nube (v6 > v4) mientras se juega.
        let obs1 = Observation {
            process_alive: true,
            cloud_version: Some(6),
            local_fingerprint: Some(2), // contenido local divergente
            ..quiet_obs()
        };
        let (s1, d1) = reconcile(&state, &obs1, world(0));
        assert!(
            acts(&d1).contains(&&Action::DeferPull),
            "1ª vez: difiere y notifica: {d1:?}"
        );
        assert!(
            acts(&d1).contains(&&Action::Backup),
            "y suelta el backup que destraba `has_pending`: {d1:?}"
        );
        assert!(s1.pull_pending, "el pull diferido sobrevive");
        assert_eq!(s1.in_flight, Some(Op::Backup));

        // La subida choca 409 y se asienta a la cabeza remota (v7): sin commit,
        // pero `known_version` avanza y `has_pending` se limpia.
        let obs_done = Observation {
            process_alive: true,
            cloud_version: Some(6),
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(7),
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (s2, _d2) = reconcile(&s1, &obs_done, world(1));
        assert!(!s2.has_pending, "la subida destrabó los cambios locales");
        assert_eq!(s2.known_version, Some(7));
        assert!(
            s2.pull_pending,
            "el pull sigue pendiente: el juego no ha cerrado"
        );

        // El usuario sigue jugando y guarda otra vez; la nube se adelanta OTRA
        // vez (v8) sin cierre de juego de por medio. El slot NO debe encallarse.
        let s3 = State {
            has_pending: true,
            ..s2
        };
        let obs2 = Observation {
            process_alive: true,
            cloud_version: Some(8),
            local_fingerprint: Some(3),
            ..quiet_obs()
        };
        let (s4, d4) = reconcile(&s3, &obs2, world(2));
        assert!(
            acts(&d4).contains(&&Action::Backup),
            "el 2º adelanto tampoco encalla la subida: {d4:?}"
        );
        assert!(
            !acts(&d4).contains(&&Action::DeferPull),
            "pero NO re-notifica: `deferred_notified` de-duplica sólo el aviso: {d4:?}"
        );
        assert!(
            !acts(&d4).contains(&&Action::Restore),
            "y jamás restaura mid-session: {d4:?}"
        );
        assert!(s4.pull_pending, "la intención de pull sigue viva");
    }

    /// D.8.1, la otra mitad: entre dos adelantos, con el pull ya diferido y la
    /// nube ya NO por delante, el autobackup mid-session sigue funcionando (antes
    /// la rama de `pull_pending` también retornaba antes del backup, matando la
    /// subida durante el resto de la sesión).
    #[test]
    fn d8_deferred_pull_does_not_starve_mid_session_backups() {
        let state = State {
            is_running: true,
            last_running_seen: Some(at(0)),
            has_pending: true,
            pull_pending: true,
            deferred_notified: true,
            known_version: Some(7),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            process_alive: true,
            cloud_version: Some(7), // la nube ya no va por delante
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert!(
            acts(&ds).contains(&&Action::Backup),
            "un pull pendiente no debe matar el autobackup de la sesión: {ds:?}"
        );
        assert!(next.pull_pending, "y el pull sigue esperando al cierre");
    }

    /// D.8.2 — backoff de fallo de *backup* dentro del kernel. Antes lo reponía
    /// el shell (`RetryBackupAfterFailure`): limpiaba `in_flight`, armaba el
    /// backoff largo y conservaba `has_pending`. Un fallo de subida no escala la
    /// escalada del restore.
    #[test]
    fn d8_backup_failure_backs_off_inside_the_kernel() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Failed),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.in_flight, None, "la op terminó");
        assert!(
            next.has_pending,
            "los cambios nunca llegaron a una versión: siguen pendientes"
        );
        assert_eq!(
            next.next_backup_at,
            Some(at(BACKUP_FAILURE_BACKOFF_SECS)),
            "re-armado en el backoff largo"
        );
        assert_eq!(
            next.restore_failures,
            RestoreFailures::default(),
            "un fallo de subida no escala la escalada del restore"
        );
        assert!(next.next_restore_at.is_none(), "ni frena el lado restore");
        assert_eq!(ds.last(), Some(&hold("backup backoff")));
        assert!(
            !acts(&ds).contains(&&Action::Backup),
            "no se relanza dentro del backoff: {ds:?}"
        );

        // Cruzado el backoff (`now` cruzando un deadline ES delta): reintenta.
        let obs_after = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (_n, ds_after) = reconcile(&next, &obs_after, world(BACKUP_FAILURE_BACKOFF_SECS + 1));
        assert_eq!(acts(&ds_after), vec![&Action::Backup]);
    }

    /// **El bug del 409 sin salida**: el shell contestaba a "vas por detrás,
    /// pero no hay nada que bajar" reponiendo el reintento cada diez minutos,
    /// sin contador y sin escalada. 1.701 eventos, 5 usuarios, y un save clavado
    /// 14 días a ~4,5 intentos/h que sobrevivió a tres versiones de la app.
    ///
    /// Ahora escala 10 → 20 → 40 → 80 min y, al quinto, para: `needs_attention`
    /// y ni una `Act(Backup)` más por su cuenta.
    #[test]
    fn an_unresolvable_conflict_escalates_and_then_stops() {
        let mut state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let mut clock = 0i64;
        for (attempt, backoff) in CONFLICT_STALL_BACKOFF_SECS.iter().enumerate() {
            // La subida sale, choca contra el conflicto y vuelve con él.
            state.in_flight = Some(Op::Backup);
            let obs = Observation {
                local_fingerprint: Some(2),
                op_result: Some(OpResult::ConflictStalled),
                ..quiet_obs()
            };
            let (next, ds) = reconcile(&state, &obs, world(clock));
            assert_eq!(
                next.backup_conflict.consecutive,
                attempt as u32 + 1,
                "el contador tiene que ir subiendo"
            );
            assert!(
                !next.backup_conflict.needs_attention,
                "aún queda presupuesto en el intento {}",
                attempt + 1
            );
            assert_eq!(
                next.next_backup_at,
                Some(at(clock + backoff)),
                "cada choque espera más que el anterior"
            );
            assert!(next.has_pending, "los cambios siguen sin versionar");
            assert!(
                !acts(&ds).contains(&&Action::Backup),
                "no se relanza dentro del backoff: {ds:?}"
            );
            // Y dentro del backoff sigue callado, aunque pase el tiempo.
            let quiet = Observation {
                local_fingerprint: Some(2),
                ..quiet_obs()
            };
            let (_m, mid) = reconcile(&next, &quiet, world(clock + backoff / 2));
            assert!(
                !acts(&mid).contains(&&Action::Backup),
                "el backoff manda hasta que vence: {mid:?}"
            );
            // Vencido, sí reintenta: `now` cruzando el deadline ES delta.
            clock += backoff + 1;
            let (after, retried) = reconcile(&next, &quiet, world(clock));
            assert!(
                acts(&retried).contains(&&Action::Backup),
                "vencido el backoff hay que volver a intentarlo: {retried:?}"
            );
            state = after;
        }

        // Quinto choque: se acabó el presupuesto.
        state.in_flight = Some(Op::Backup);
        let obs = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::ConflictStalled),
            ..quiet_obs()
        };
        let (given_up, ds) = reconcile(&state, &obs, world(clock));
        assert_eq!(
            given_up.backup_conflict.consecutive,
            CONFLICT_STALL_GIVE_UP_AFTER
        );
        assert!(
            given_up.backup_conflict.needs_attention,
            "al quinto, el save pide una persona"
        );
        assert_eq!(ds.last(), Some(&hold(HOLD_BACKUP_NEEDS_ATTENTION)));

        // Y ya no reintenta **nunca** solo: ni al minuto ni a la semana.
        let quiet = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        for later in [clock + 60, clock + 86_400, clock + 7 * 86_400] {
            let (_n, ds) = reconcile(&given_up, &quiet, world(later));
            assert!(
                !acts(&ds).contains(&&Action::Backup),
                "un save rendido no puede volver a reintentar solo (t={later}): {ds:?}"
            );
            assert_eq!(ds.last(), Some(&hold(HOLD_BACKUP_NEEDS_ATTENTION)));
        }
    }

    /// El número que lo destapó, puesto a prueba: catorce días de reloj con el
    /// conflicto respondiendo siempre lo mismo. Antes eran ~1.500 intentos (uno
    /// cada diez minutos, para siempre); ahora son cinco y se acabó.
    ///
    /// Se simula tick a tick en vez de razonar sobre el backoff porque el bucle
    /// vivía precisamente en la costura entre "el reductor arma el deadline" y
    /// "el tick siguiente lo cruza".
    #[test]
    fn fourteen_days_of_the_same_conflict_cost_five_attempts() {
        let mut state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let quiet = Observation {
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let conflicted = Observation {
            op_result: Some(OpResult::ConflictStalled),
            ..quiet.clone()
        };

        let mut attempts = 0;
        let mut in_flight = false;
        // Un tick por minuto durante 14 días.
        for minute in 0..(14 * 24 * 60) {
            let now = minute * 60;
            let obs = if in_flight { &conflicted } else { &quiet };
            let (next, ds) = reconcile(&state, obs, world(now));
            in_flight = false;
            if acts(&ds).contains(&&Action::Backup) {
                attempts += 1;
                // La subida sale y vuelve con el mismo conflicto de siempre.
                in_flight = true;
            }
            state = next;
        }

        assert_eq!(
            attempts, CONFLICT_STALL_GIVE_UP_AFTER,
            "el 409 sin salida sólo puede costar el presupuesto, no catorce días de intentos"
        );
        assert!(
            state.backup_conflict.needs_attention,
            "y acaba pidiendo una persona"
        );
        assert!(
            state.has_pending,
            "sin perder los cambios locales por el camino"
        );
    }

    /// Rendirse no puede ser una condena: si la nube publica otra cabeza, la
    /// pregunta ya no es la misma —quizá ahora sí hay algo que bajar— y el save
    /// vuelve a intentarlo solo. Sin esto nada podría desatascarlo salvo el
    /// usuario, ni siquiera el otro dispositivo subiendo lo que faltaba.
    #[test]
    fn a_new_cloud_head_un_stalls_a_save_that_gave_up() {
        let state = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            known_version: Some(4),
            backup_conflict: ConflictStall {
                consecutive: CONFLICT_STALL_GIVE_UP_AFTER,
                version: Some(4),
                needs_attention: true,
            },
            next_backup_at: Some(at(9_000)),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(
            next.backup_conflict,
            ConflictStall::default(),
            "la escalada muere con la cabeza contra la que se contaba"
        );
        // La nube va por delante, así que este tick toca bajar; lo que importa
        // es que el freno de la subida se soltó.
        assert!(next.next_backup_at.is_none(), "y su freno con ella");
        assert!(!ds.is_empty());
    }

    /// Una copia que sale bien suelta la escalada entera — incluido el no-op,
    /// que también significa que el conflicto se resolvió.
    #[test]
    fn a_backup_that_lands_clears_the_conflict_escalation() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            backup_conflict: ConflictStall {
                consecutive: 3,
                version: Some(4),
                needs_attention: false,
            },
            ..base_state()
        };
        let obs = Observation {
            cloud_version: Some(4),
            op_result: Some(OpResult::Ok {
                version: Some(5),
                fingerprint: Some(2),
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.backup_conflict, ConflictStall::default());
    }

    /// D.8.2 — commit vs no-op en `OpResult::Ok`. **La** regresión R.E.P.O.: un
    /// pase no-op no es un backup y no debe mover el ancla del min-interval, o
    /// la siguiente subida real se empuja un intervalo entero (y con la carpeta
    /// vaciándose por restore, el ancla avanzaba sobre backups fantasma y una
    /// sesión corta nunca volcaba su progreso).
    #[test]
    fn a_calm_save_never_waits() {
        // Lo que se rompió en junio y no se puede volver a romper: sin preset y
        // sin ráfaga, una copia sale en cuanto el debounce asienta. Un suelo que
        // nadie ve ni puede cambiar se lee como "no detecta mis cambios".
        let state = State {
            min_backup_interval_secs: 0,
            burst_since: Some(at(0)),
            burst_backups: BURST_THRESHOLD - 1,
            last_backup_at: Some(at(0)),
            ..State::default()
        };
        assert_eq!(backup_floor(&state), None);
    }

    /// Un juego que reescribe su autoguardado cada pocos segundos: al tercer
    /// commit dentro de la ventana entra el suelo, y se queda en 60 s por muchos
    /// más que haga. Ghost of Tsushima hizo 1.027 subidas en 4½ h sin esto.
    #[test]
    fn an_autosave_burst_gets_one_minute_and_no_more() {
        let mut state = State {
            min_backup_interval_secs: 0,
            ..State::default()
        };
        for i in 0..3 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(state.burst_backups, 3);
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);

        // Diez commits más no lo suben ni un segundo: un solo escalón.
        for i in 3..13 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);
    }

    /// Al pasar la ventana sin actividad la cuenta se abre de cero, así que el
    /// save vuelve a subir inmediato: el suelo dura lo que dura la ráfaga.
    #[test]
    fn the_burst_forgets_itself_once_the_game_calms_down() {
        let mut state = State {
            min_backup_interval_secs: 0,
            ..State::default()
        };
        for i in 0..5 {
            count_burst(&mut state, at(i * 6));
        }
        assert_eq!(effective_min_interval(&state), BURST_FLOOR_SECS);

        count_burst(&mut state, at(BURST_WINDOW_SECS + 60));
        assert_eq!(state.burst_backups, 1);
        assert_eq!(effective_min_interval(&state), 0);
    }

    /// El preset que el usuario eligió manda sobre el adaptativo, en los dos
    /// sentidos: `short_session` mantiene sus 30 s aunque el juego esté en plena
    /// ráfaga (es un juego que se borra la carpeta entre rondas y perder una es
    /// perder la partida), y `data_saver` mantiene sus 600 s.
    #[test]
    fn an_explicit_preset_wins_over_the_adaptive_floor() {
        let mut short = State {
            min_backup_interval_secs: 30,
            ..State::default()
        };
        for i in 0..10 {
            count_burst(&mut short, at(i * 6));
        }
        assert_eq!(effective_min_interval(&short), 30);

        let saver = State {
            min_backup_interval_secs: 600,
            burst_backups: 0,
            ..State::default()
        };
        assert_eq!(effective_min_interval(&saver), 600);
    }

    #[test]
    fn d8_no_op_backup_does_not_anchor_the_min_interval() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            min_backup_interval_secs: 600,
            synced_fingerprint: Some(1),
            ..base_state()
        };

        // No-op puro (skip por firma / vacío / archived): sin versión.
        let obs_noop = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: None,
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (noop, _) = reconcile(&state, &obs_noop, world(0));
        assert!(
            noop.last_backup_at.is_none(),
            "un no-op no ancla el min-interval (R.E.P.O.)"
        );
        assert!(noop.next_backup_at.is_none(), "ni arma el suelo");
        assert!(!noop.has_pending, "pero sí destraba los cambios");
        assert_eq!(noop.synced_fingerprint, Some(2), "y adopta la firma");
        assert!(
            noop.last_restore_at.is_none(),
            "un no-op sin versión no tocó la carpeta"
        );

        // Commit real: ancla el suelo.
        let obs_commit = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: true,
            }),
            ..quiet_obs()
        };
        let (committed, _) = reconcile(&state, &obs_commit, world(0));
        assert_eq!(committed.last_backup_at, Some(at(0)));
        assert_eq!(committed.known_version, Some(9));

        // Y ese ancla es lo que frena de verdad la siguiente subida: escritura
        // nueva a los 100 s ⇒ retenida; pasado el suelo (600 s) ⇒ sube. Con el
        // ancla del no-op (nunca puesta) no habría freno ninguno, que es
        // justamente lo correcto: nada se subió.
        let obs_more = Observation {
            fs_event: true,
            local_fingerprint: Some(7),
            ..quiet_obs()
        };
        let (_n, held) = reconcile(&committed, &obs_more, world(100));
        assert_eq!(held, vec![hold("backup min-interval")]);
        let (_n, freed) = reconcile(&committed, &obs_more, world(601));
        assert_eq!(acts(&freed), vec![&Action::Backup]);
        let (_n, no_floor) = reconcile(&noop, &obs_more, world(100));
        assert_eq!(
            acts(&no_floor),
            vec![&Action::Backup],
            "un no-op no dejó ancla, así que no frena nada"
        );

        // No-op CON versión = 409 asentado a la cabeza: el merge escribió en la
        // carpeta como un restore → sella `last_restore_at` (que ese toque
        // nuestro no vete el siguiente pull) pero sigue sin anclar el suelo.
        let obs_settled = Observation {
            local_fingerprint: Some(2),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: false,
            }),
            ..quiet_obs()
        };
        let (settled, _) = reconcile(&state, &obs_settled, world(0));
        assert_eq!(settled.known_version, Some(9));
        assert_eq!(settled.last_restore_at, Some(at(0)));
        assert!(
            settled.last_backup_at.is_none(),
            "asentarse a la cabeza no es un commit propio"
        );
    }

    /// D.8.3 — anti-relanzamiento **contra la verdad del server**.
    ///
    /// El caso real: el daemon se reinicia (rutina desde el Slice 4) con una
    /// subida en vuelo que sí llegó a comprometerse. El `in_flight` en memoria se
    /// perdió, así que el motor vuelve a lanzar la subida; el chequeo
    /// content-addressed descubre que ese contenido **ya es la cabeza** y no sube
    /// nada. Lo que el reductor tiene que hacer con esa respuesta:
    ///
    /// - adoptar la versión y la firma (converger),
    /// - **no** anclar el min-interval (no hubo commit propio: R.E.P.O.),
    /// - y **no** sellar `last_restore_at`, porque a diferencia del 409 asentado
    ///   a la cabeza aquí no se escribió un solo byte en la carpeta.
    #[test]
    fn d8_3_an_upload_that_already_landed_converges_without_faking_a_local_touch() {
        let state = State {
            in_flight: Some(Op::Backup),
            has_pending: true,
            min_backup_interval_secs: 600,
            known_version: Some(8),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(9),
            op_result: Some(OpResult::Ok {
                version: Some(9),
                fingerprint: Some(2),
                wrote: false,
            }),
            upload_landed: Some(true),
            ..quiet_obs()
        };

        let (next, ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.in_flight, None, "la op terminó");
        assert!(!next.has_pending, "el contenido está en una versión");
        assert_eq!(
            next.known_version,
            Some(9),
            "adopta la versión que ya lo tenía"
        );
        assert_eq!(next.synced_fingerprint, Some(2));
        assert!(
            next.last_backup_at.is_none(),
            "no se subió nada: anclar el suelo aquí es la regresión R.E.P.O."
        );
        assert!(
            next.last_restore_at.is_none(),
            "y no se escribió en la carpeta: sellar un toque que no existió \
             falsearía la ventana de gracia del veto"
        );
        assert!(
            !acts(&ds).iter().any(|a| matches!(a, Action::Backup)),
            "y sobre todo: no se relanza la subida ({ds:?})"
        );

        // El tick siguiente, ya sin op en vuelo y con el mismo contenido:
        // convergido ⇒ cero acciones. Es la mitad que importa — si el reductor no
        // hubiera adoptado versión y firma, aquí volvería a emitir `Backup` y
        // tendríamos el bucle que D.8.3 viene a matar.
        let quiet = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(9),
            ..quiet_obs()
        };
        let (_after, ds_after) = reconcile(&next, &quiet, world(1));
        assert_eq!(ds_after, vec![hold("converged")]);
    }

    /// D.8.1/D.8.2 — el flush que destraba un pull cross-device se salta el suelo
    /// de *ahorro de datos* (como hacía el ejecutor de 2b, que iba directo al
    /// backup), pero NO un backoff de error. Sin esto, con el preset `data_saver`
    /// (600 s) la actualización de otro dispositivo esperaría el intervalo entero:
    /// el pull sigue vetado mientras `has_pending` no se limpie.
    #[test]
    fn d8_cross_device_flush_skips_the_savings_floor_but_not_a_backoff() {
        // Commit hace 100 s con suelo de 600 s, y el usuario ha vuelto a guardar.
        let state = State {
            is_running: true,
            last_running_seen: Some(at(100)),
            has_pending: true,
            min_backup_interval_secs: 600,
            last_backup_at: Some(at(0)),
            known_version: Some(4),
            synced_fingerprint: Some(1),
            ..base_state()
        };
        let quiet_cloud = Observation {
            process_alive: true,
            cloud_version: Some(4), // nube al día
            local_fingerprint: Some(2),
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &quiet_cloud, world(100));
        assert_eq!(
            ds,
            vec![hold("backup min-interval")],
            "sin urgencia el suelo de ahorro manda"
        );

        // La nube se adelanta: el flush ya no es pacing, es lo que destraba el pull.
        let ahead = Observation {
            cloud_version: Some(6),
            ..quiet_cloud.clone()
        };
        let (_n, ds_urgent) = reconcile(&state, &ahead, world(100));
        assert!(
            acts(&ds_urgent).contains(&&Action::Backup),
            "el flush cross-device no espera al suelo de ahorro: {ds_urgent:?}"
        );

        // Pero un backoff de error sí lo frena: eso no es pacing.
        let backing_off = State {
            next_backup_at: Some(at(700)),
            ..state
        };
        let (_n, ds_backoff) = reconcile(&backing_off, &ahead, world(100));
        assert!(
            !acts(&ds_backoff).contains(&&Action::Backup),
            "un backoff de error no se salta ni por urgencia: {ds_backoff:?}"
        );
        assert_eq!(ds_backoff.last(), Some(&hold("backup backoff")));
    }

    /// D.8.2 — una versión cloud nueva limpia el backoff de restore. El backoff
    /// era sobre la versión que fallaba; que el server publique otra es
    /// información nueva, no un reintento. Antes lo hacía el shell al recibir
    /// `SetCloudVersions`.
    #[test]
    fn d8_new_cloud_version_clears_the_restore_backoff() {
        // Tres fallos contra v5 → stuck y aparcado una hora.
        let state = State {
            known_version: Some(3),
            restore_failures: RestoreFailures {
                consecutive: 3,
                version: Some(5),
                stuck_notified: true,
            },
            next_restore_at: Some(at(3600)),
            ..base_state()
        };

        // Misma versión: la escalada aguanta y el freno sigue.
        let obs_same = Observation {
            local_empty: true,
            cloud_version: Some(5),
            ..quiet_obs()
        };
        let (same, ds_same) = reconcile(&state, &obs_same, world(0));
        assert!(
            same.restore_failures.stuck_notified,
            "sin novedad, sigue stuck"
        );
        assert_eq!(ds_same, vec![hold("restore cooldown")]);

        // El server publica v6: la escalada muere y el pull sale ya.
        let obs_new = Observation {
            local_empty: true,
            cloud_version: Some(6),
            ..quiet_obs()
        };
        let (fresh, ds_new) = reconcile(&state, &obs_new, world(0));
        assert_eq!(
            fresh.restore_failures,
            RestoreFailures::default(),
            "versión nueva ⇒ escalada reseteada (el shell lo lee para 'recovered')"
        );
        assert_eq!(
            acts(&ds_new),
            vec![&Action::Restore],
            "y el reintento no espera al backoff viejo: {ds_new:?}"
        );
    }

    /// D.8.2 — la escalada de fallos de restore se ancla en la versión CLOUD
    /// observada (la cabeza que intentábamos traernos), no en la local: es lo
    /// que hace coherente el reseteo por versión nueva.
    #[test]
    fn d8_restore_failures_anchor_on_the_observed_cloud_version() {
        let state = State {
            in_flight: Some(Op::Restore),
            known_version: Some(3),
            ..base_state()
        };
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(9),
            op_result: Some(OpResult::Failed),
            ..quiet_obs()
        };
        let (next, _ds) = reconcile(&state, &obs, world(0));
        assert_eq!(next.restore_failures.version, Some(9));
        assert_eq!(next.restore_failures.consecutive, 1);
        assert_eq!(
            next.next_restore_at,
            Some(at(FAILURE_BACKOFF_SECS[0])),
            "primer escalón del backoff"
        );
    }

    // ==== Corpus D.10 (el poller de nube enmudece) ==========================

    /// D.10 — «convergido» vs «ciego». Con la caché de versiones de nube
    /// envejecida, el reposo deja de rotularse `converged` y pasa a decir por
    /// qué no sabe nada: es el fallo invisible (poller muerto) hecho
    /// observable. Sin marca de feed —self-hosted/CLI, sin poller— no se
    /// reporta obsolescencia ninguna.
    #[test]
    fn d10_stale_cloud_cache_is_not_convergence() {
        let state = State {
            known_version: Some(120),
            synced_fingerprint: Some(0xABCD),
            ..base_state()
        };
        let fed_at = |off: i64| Observation {
            local_fingerprint: Some(0xABCD),
            cloud_version: Some(120),
            cloud_version_as_of: Some(at(off)),
            ..quiet_obs()
        };

        // Feed recién llegado: convergido de verdad.
        let (_n, fresh) = reconcile(&state, &fed_at(0), world(1));
        assert_eq!(fresh, vec![hold("converged")]);

        // Justo en el umbral: todavía se le concede el beneficio de la duda
        // (la comparación es estrictamente mayor, así que el tick que cae
        // exacto sobre el deadline aún no acusa).
        let (_n, edge) = reconcile(&state, &fed_at(0), world(CLOUD_STALE_AFTER_SECS));
        assert_eq!(
            edge,
            vec![hold("converged")],
            "el umbral no muerde antes de tiempo"
        );

        // Un segundo más: ciego, con motivo propio.
        let (_n, stale) = reconcile(&state, &fed_at(0), world(CLOUD_STALE_AFTER_SECS + 1));
        assert_eq!(
            stale,
            vec![hold(CLOUD_STALE_REASON)],
            "una caché de nube envejecida no es convergencia"
        );
        assert!(
            acts(&stale).is_empty(),
            "pero sigue sin inventarse acciones"
        );

        // Sin nube que observar (self-hosted / daemon CLI): no hay nada que
        // declarar obsoleto, por muy lejos que esté `now` del epoch.
        let no_feed = Observation {
            local_fingerprint: Some(0xABCD),
            ..quiet_obs()
        };
        let (_n, headless) = reconcile(&state, &no_feed, world(100_000));
        assert_eq!(
            headless,
            vec![hold("converged")],
            "sin contexto de nube no se reporta obsolescencia"
        );
    }

    /// D.11, remate — «nunca supe nada de la nube» es la ceguera MÁS grave y
    /// hasta aquí era la única que se colaba como `converged`: el viejo
    /// `is_some_and` sobre la marca del feed dejaba pasar el `None`. Con
    /// contexto cloud, la cuenta atrás corre desde que el motor empezó a
    /// esperar cabezas.
    #[test]
    fn d11_never_heard_from_the_cloud_is_stale_too() {
        let state = State {
            known_version: Some(120),
            synced_fingerprint: Some(0xABCD),
            ..base_state()
        };
        // Contexto cloud, cero feeds: el motor arrancó en `at(0)`.
        let blind = Observation {
            local_fingerprint: Some(0xABCD),
            cloud_feed_expected_since: Some(at(0)),
            ..quiet_obs()
        };

        // Dentro del margen de arranque: silencio, todavía es normal.
        let (_n, booting) = reconcile(&state, &blind, world(CLOUD_STALE_AFTER_SECS));
        assert_eq!(
            booting,
            vec![hold("converged")],
            "el margen de arranque no acusa antes de tiempo"
        );

        // Pasado el margen sin una sola cabeza: ciego, con el mismo motivo que
        // un feed rancio (para la UI y el replay es la misma avería).
        let (_n, blind_ds) = reconcile(&state, &blind, world(CLOUD_STALE_AFTER_SECS + 1));
        assert_eq!(blind_ds, vec![hold(CLOUD_STALE_REASON)]);
        assert!(
            acts(&blind_ds).is_empty(),
            "y sigue sin inventarse acciones"
        );

        // Un feed real manda sobre el ancla de arranque: la marca fresca
        // rejuvenece la observación aunque el motor lleve horas arriba.
        let fed = Observation {
            cloud_version_as_of: Some(at(10_000)),
            ..blind.clone()
        };
        let (_n, ds_fed) = reconcile(&state, &fed, world(10_001));
        assert_eq!(ds_fed, vec![hold("converged")]);

        // Y sin contexto cloud (self-hosted), el mismo silencio no acusa nunca:
        // la distinción es el contexto, no `None` vs `Some`.
        let selfhosted = Observation {
            cloud_feed_expected_since: None,
            ..blind
        };
        let (_n, ds_self) = reconcile(&state, &selfhosted, world(100_000));
        assert_eq!(ds_self, vec![hold("converged")]);
    }

    /// D.12 — la relación "el motor refresca antes de declararse ciego" NO vive
    /// aquí: es un `const _: () = assert!(…)` junto a
    /// [`CLOUD_SELF_OBSERVE_AFTER_SECS`], que es estrictamente más fuerte que un
    /// test (el crate no compila si alguien invierte los números). Este test
    /// sólo fija que el umbral nuevo sigue derivándose de la cadencia real del
    /// poll (como el de obsolescencia, ver el test de D.10) y no de un literal
    /// suelto que pueda driftar.
    #[test]
    fn d12_self_observation_threshold_derives_from_the_poll_cadence() {
        assert_eq!(
            CLOUD_SELF_OBSERVE_AFTER_SECS,
            CLOUD_POLL_INTERVAL_SECS * 3 / 2
        );
    }

    /// D.10 — el umbral se deriva de la cadencia del poll, no de un número
    /// suelto: si mañana cambia el intervalo, el umbral lo sigue. (El suelo de
    /// "más de un poll perdido" se chequea en compilación, junto a la
    /// constante.)
    #[test]
    fn d10_stale_threshold_derives_from_the_poll_cadence() {
        assert_eq!(
            CLOUD_STALE_AFTER_SECS,
            CLOUD_POLL_INTERVAL_SECS * CLOUD_STALE_AFTER_POLLS
        );
    }

    /// D.10 — la obsolescencia **sólo** cambia el motivo del reposo. Un poller
    /// muerto no puede frenar la subida: eso cambiaría un fallo invisible por
    /// pérdida de datos (el progreso local se quedaría sin versionar). Ni frena
    /// un restore que ya sabemos que toca.
    #[test]
    fn d10_stale_cloud_cache_does_not_stop_syncing() {
        let ancient = Some(at(-10 * CLOUD_STALE_AFTER_SECS));

        // Hay progreso local divergente: se sube igual.
        let pending = State {
            has_pending: true,
            synced_fingerprint: Some(1),
            known_version: Some(120),
            ..base_state()
        };
        let obs = Observation {
            local_fingerprint: Some(2),
            cloud_version: Some(120),
            cloud_version_as_of: ancient,
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&pending, &obs, world(0));
        assert_eq!(
            acts(&ds),
            vec![&Action::Backup],
            "una caché ciega no puede dejar el progreso local sin versionar: {ds:?}"
        );

        // Y la carpeta vacía sigue disparando el restore con la caché vieja: lo
        // que sabemos sigue valiendo, sólo que sabemos que puede haber más.
        let empty = Observation {
            local_empty: true,
            cloud_version: Some(121),
            cloud_version_as_of: ancient,
            ..quiet_obs()
        };
        let (_n, ds_empty) = reconcile(&base_state(), &empty, world(0));
        assert_eq!(acts(&ds_empty), vec![&Action::Restore]);
    }

    /// track-only: nunca sincroniza nada.
    #[test]
    fn track_only_never_acts() {
        let state = State {
            track_only: true,
            has_pending: true,
            ..base_state()
        };
        let obs = Observation {
            local_empty: true,
            cloud_version: Some(99),
            fs_event: true,
            ..quiet_obs()
        };
        let (_n, ds) = reconcile(&state, &obs, world(0));
        assert!(acts(&ds).is_empty());
        assert_eq!(ds, vec![hold("track-only entry")]);
    }

    // ==== Invariantes (proptest + shrinking) ================================

    prop_compose! {
        fn arb_failures()(
            consecutive in 0u32..6,
            version in prop::option::of(0i64..20),
            stuck in any::<bool>(),
        ) -> RestoreFailures {
            RestoreFailures { consecutive, version, stuck_notified: stuck }
        }
    }

    prop_compose! {
        /// Escalada de conflictos arbitraria. Entra en el generador porque su
        /// `needs_attention` **frena la subida**: los invariantes tienen que
        /// aguantar un slot rendido igual que uno sano.
        fn arb_conflicts()(
            consecutive in 0u32..8,
            version in prop::option::of(0i64..20),
            needs_attention in any::<bool>(),
        ) -> ConflictStall {
            ConflictStall { consecutive, version, needs_attention }
        }
    }

    prop_compose! {
        /// Estado arbitrario con tiempos anclados a `BASE` (offsets acotados).
        fn arb_state()(
            track_only in any::<bool>(),
            restore_enabled in any::<bool>(),
            is_running in any::<bool>(),
            running_seen in prop::option::of(-100i64..100),
            has_pending in any::<bool>(),
            fs_at in prop::option::of(-100i64..100),
            restore_at in prop::option::of(-100i64..100),
            known_version in prop::option::of(0i64..20),
            synced_fp in prop::option::of(0u64..8),
            backup_at in prop::option::of(-100i64..100),
            in_flight in prop::option::of(prop_oneof![Just(Op::Backup), Just(Op::Restore)]),
            next_backup in prop::option::of(-100i64..200),
            next_restore in prop::option::of(-100i64..200),
            pull_pending in any::<bool>(),
            deferred_notified in any::<bool>(),
            min_interval in 0u64..120,
            // La ráfaga entra en el generador y no como default: el suelo
            // adaptativo cambia cuándo se puede subir, así que los invariantes
            // (idempotencia, convergencia) tienen que aguantarlo también.
            burst_since in prop::option::of(-1200i64..0),
            burst_backups in 0u32..8,
            failures in arb_failures(),
            conflicts in arb_conflicts(),
        ) -> State {
            State {
                track_only,
                restore_enabled,
                is_running,
                last_running_seen: running_seen.map(at),
                has_pending,
                last_fs_event_at: fs_at.map(at),
                last_restore_at: restore_at.map(at),
                known_version,
                synced_fingerprint: synced_fp,
                last_backup_at: backup_at.map(at),
                in_flight,
                next_backup_at: next_backup.map(at),
                next_restore_at: next_restore.map(at),
                pull_pending,
                deferred_notified,
                min_backup_interval_secs: min_interval,
                burst_since: burst_since.map(at),
                burst_backups,
                restore_failures: failures,
                backup_conflict: conflicts,
            }
        }
    }

    prop_compose! {
        /// Observación arbitraria. `quiescent` fuerza a `false`/`None` las
        /// señales puntuales (fs/op/upload) — el mundo estable para el invariante
        /// de idempotencia.
        fn arb_obs(quiescent: bool)(
            mtime in prop::option::of(-100i64..100),
            size in prop::option::of(0u64..1_000),
            local_empty in any::<bool>(),
            local_fp in prop::option::of(0u64..8),
            process_alive in any::<bool>(),
            cloud_version in prop::option::of(0i64..20),
            // Cubre el feed fresco, el rancio y el despliegue sin poller, para
            // que los invariantes valgan también con la caché de nube ciega.
            cloud_as_of in prop::option::of(-2 * CLOUD_STALE_AFTER_SECS..100),
            // Ídem para el contexto: con nube que observar (dentro y fuera del
            // margen de arranque) y sin ella.
            cloud_expected in prop::option::of(-2 * CLOUD_STALE_AFTER_SECS..100),
            fs_event in any::<bool>(),
            retry in 0u32..600,
            has_op in any::<bool>(),
            op_kind in 0u8..5,
            ok_ver in prop::option::of(0i64..20),
            ok_fp in prop::option::of(0u64..8),
            ok_wrote in any::<bool>(),
        ) -> Observation {
            let op_result = if quiescent || !has_op {
                None
            } else {
                Some(match op_kind {
                    0 => OpResult::Ok { version: ok_ver, fingerprint: ok_fp, wrote: ok_wrote },
                    1 => OpResult::NotFound,
                    2 => OpResult::Unauthorized,
                    3 => OpResult::Throttled { retry_after_secs: retry },
                    _ => OpResult::Failed,
                })
            };
            Observation {
                folder_mtime: mtime.map(at),
                folder_size: size,
                local_empty,
                local_fingerprint: local_fp,
                process_alive,
                // El proptest no modela la sonda de bloqueo: es un freno de
                // ritmo del shell (sólo Windows la puede afirmar) y dejarla
                // siempre en falso mantiene el espacio de estados en lo que
                // este test cubre.
                save_files_locked: false,
                cloud_version,
                cloud_version_as_of: cloud_as_of.map(at),
                cloud_feed_expected_since: cloud_expected.map(at),
                fs_event: if quiescent { false } else { fs_event },
                op_result,
                upload_landed: None,
            }
        }
    }

    fn arb_world() -> impl Strategy<Value = World> {
        (-100i64..300, any::<u64>()).prop_map(|(now_off, seed)| World {
            now: at(now_off),
            seed,
        })
    }

    proptest! {
        /// Invariante: ≤ 1 acción de storage (Backup/Restore) por tick.
        #[test]
        fn inv_storage_acts_bounded(state in arb_state(), obs in arb_obs(false), w in arb_world()) {
            let (_n, ds) = reconcile(&state, &obs, w);
            prop_assert!(storage_act_count(&ds) <= 1, "más de una acción de storage: {ds:?}");
        }

        /// Invariante: Backup y Restore nunca en el mismo tick (no se pelean).
        #[test]
        fn inv_backup_restore_mutually_exclusive(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            let a = acts(&ds);
            prop_assert!(
                !(a.contains(&&Action::Backup) && a.contains(&&Action::Restore)),
                "backup y restore juntos: {ds:?}"
            );
        }

        /// Invariante: nunca `Act(Restore)` mid-session / sobre local sin
        /// versionar (data-loss REPO + never-lose-newer-local). Si se restaura,
        /// el estado resultante no está corriendo ni tiene pendientes.
        #[test]
        fn inv_restore_never_mid_session(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (next, ds) = reconcile(&state, &obs, w);
            if acts(&ds).contains(&&Action::Restore) {
                prop_assert!(!next.is_running, "restore con juego corriendo: {ds:?}");
                prop_assert!(!next.has_pending, "restore sobre local sin versionar: {ds:?}");
            }
        }

        /// Invariante base + dinámico (C.1/C.2): bajo entrada quiescente el
        /// reductor es idempotente — reaplicarlo sobre su propia salida, mismo
        /// `now`, no emite ninguna `Act`. Mata el hot-loop: ninguna acción sin un
        /// delta nuevo. (Los deltas de un tick —fs/op— se excluyen por ser justo
        /// eso, deltas.)
        #[test]
        fn inv_idempotent_under_quiescence(
            state in arb_state(), obs in arb_obs(true), w in arb_world()
        ) {
            let (s1, _d1) = reconcile(&state, &obs, w);
            let (_s2, d2) = reconcile(&s1, &obs, w);
            prop_assert!(
                acts(&d2).is_empty(),
                "acción sin delta al reconciliar sobre la propia salida: {d2:?}"
            );
        }

        /// Invariante D.8.1: con cambios locales sin versionar, contenido
        /// divergente, nada en vuelo y el ritmo cumplido, el tick **siempre**
        /// emite la subida — la única forma de limpiar `has_pending`. Ninguna
        /// rama de restore (cooldown, veto, pull diferido) puede tragársela: eso
        /// era el deadlock que el shell desatascaba a mano.
        ///
        /// La **única** excepción es un save rendido (`needs_attention`): ahí no
        /// emitir la subida es la decisión, no un descuido. Es la diferencia
        /// entre las dos formas de estar parado — encallado sin que nada lo diga
        /// (el bug) frente a parado, dicho en voz alta y con tres salidas
        /// (copia manual, copia con éxito, cabeza de nube nueva).
        #[test]
        fn inv_pending_local_changes_always_get_a_backup(
            state in arb_state(), obs in arb_obs(false), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            let eligible = !state.track_only
                && state.in_flight.is_none()
                && obs.op_result.is_none()
                && !state.backup_conflict.needs_attention
                && (state.has_pending || obs.fs_event)
                && local_diverged(&state, &obs)
                && state.next_backup_at.is_none_or(|t| w.now >= t)
                && backup_floor(&state).is_none_or(|t| w.now >= t);
            if eligible {
                prop_assert!(
                    acts(&ds).contains(&&Action::Backup),
                    "cambios pendientes sin subida: el slot queda encallado: {ds:?}"
                );
            }
        }

        /// Invariante: nunca `Act(Backup)` con un restore en vuelo (no subir
        /// mientras se baja). El anti-relaunch retiene toda op en vuelo.
        #[test]
        fn inv_no_backup_while_restoring(
            state in arb_state(), obs in arb_obs(true), w in arb_world()
        ) {
            let (_n, ds) = reconcile(&state, &obs, w);
            if state.in_flight == Some(Op::Restore) && obs.op_result.is_none() {
                prop_assert!(
                    !acts(&ds).contains(&&Action::Backup),
                    "backup mientras un restore está en vuelo: {ds:?}"
                );
            }
        }
    }
}
