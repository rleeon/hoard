//! Library half of the snapshot upload flow.
//!
//! The CLI and the desktop app both want to walk a directory, build a
//! multipart body, and POST it to the server. The two diverge on how they
//! present progress (indicatif progress bar vs a Tauri event stream), so we
//! expose the work as an async function with a `progress` callback.
//!
//! State-file bookkeeping (`saves` map in `state.json`) lives here too so the
//! GUI gets it for free.

use anyhow::{anyhow, bail, Context, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use futures::FutureExt;
use reqwest::multipart;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;

use crate::api::{
    ApiClient, ApiError, CasCommit, CasFile, CasInit, CloudCasFileEntry, CloudCasInit,
    CloudCasMissingBlob, RateLimitKind, Snapshot,
};
use crate::state::{CliState, SaveState};
use hoard_core::ids::SaveId;
use hoard_core::kernel::fileclass;
use hoard_core::wire::VersionOrigin;

/// Bounded fan-out for per-file work in the cloud path (hashing local files,
/// PUTting missing blobs). Saves are mostly many small files, so per-file
/// open/read and R2 round-trip latency dominates over raw throughput; a small
/// window hides that latency without saturating the disk or the uplink.
const TRANSFER_CONCURRENCY: usize = 4;

/// How many times one blob may be turned away by a request pacer before we stop
/// and let the whole attempt fail.
///
/// This exists so a genuinely misconfigured server (an operator who sets
/// `per_second = 1`, or a proxy that refuses everything) fails fast with a clear
/// error instead of crawling for an hour pretending to work.
const MAX_PACED_RETRIES_PER_BLOB: u32 = 6;

/// Shortest we'll wait after being paced, and the base of the per-blob backoff.
///
/// The pacer's own hint is in whole seconds, so at any sane rate limit it says
/// `0` — true, but not something to act on literally. Four workers each waiting
/// ~200 ms converges on roughly 20 requests/second, which is the default limit
/// the server is actually enforcing.
const PACED_RETRY_FLOOR: Duration = Duration::from_millis(200);

/// Ceiling for a single paced wait. Past this, a "slow down" is better handled
/// by failing the attempt and re-arming on the agent's long backoff.
const PACED_RETRY_CEILING: Duration = Duration::from_secs(10);

/// Total time one upload may spend sitting in pacer waits, summed across all
/// its blobs and all workers.
///
/// Summed rather than wall-clock on purpose: wall-clock would also count the
/// transfer itself, so a legitimately slow 4 GB upload would abort the moment
/// anything paced it. This counts only time actually spent blocked.
///
/// Generous, because the per-blob cap above is what really guards against a
/// hostile server — this one only has to stop a huge folder from crawling
/// indefinitely against a very tight limit. A folder of a few thousand small
/// files paced at 20 requests/second legitimately spends minutes here, and
/// aborting that would be the same bug in a new hat.
const PACED_WAIT_BUDGET: Duration = Duration::from_secs(900);

/// The wait a pacer asked for, if this error is one.
///
/// Only [`RateLimitKind::Paced`] retries here. A budget 429 (bandwidth window,
/// storage quota, loop brake) means the operation doesn't fit right now, and
/// re-sending the same PUT can only make it worse — those keep travelling up to
/// the agent, which parks the save and comes back later.
fn paced_wait_hint(e: &anyhow::Error) -> Option<u32> {
    if let Some(hint) = e
        .chain()
        .find_map(|c| c.downcast_ref::<ApiError>())
        .and_then(|api| match api {
            ApiError::RateLimited {
                kind: RateLimitKind::Paced,
                retry_after_seconds,
                ..
            } => Some(*retry_after_seconds),
            _ => None,
        })
    {
        return Some(hint);
    }
    // A pacer that answers 429 **without draining the body** does not read as a
    // 429: the socket dies while we are still writing the PUT and the response
    // goes with it. On Windows always — the stack discards whatever was already
    // buffered when the RST lands, so the 429 does not exist for us. That is
    // issue #17: a 173-file folder that never finished, with `error writing a
    // body to connection` for its only clue.
    //
    // Treated as pacing with no hint. A genuine network drop lands here too and
    // gets that blob retried a few times, which beats throwing away the whole
    // batch over one stumble; and if it is persistent,
    // `MAX_PACED_RETRIES_PER_BLOB` turns it back into the same failure as
    // before, just a few seconds later.
    is_body_write_reset(e).then_some(0)
}

/// Did the connection die while we were writing the request body?
///
/// Matched on the `io::Error` at the bottom of the chain, never on the text:
/// the message comes in the language of the Windows install — the issue's
/// arrived in German — and comparing localised strings is an expensive way to
/// detect nothing.
fn is_body_write_reset(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::ConnectionAborted   // WSAECONNABORTED (10053)
                    | std::io::ErrorKind::ConnectionReset // WSAECONNRESET (10054)
                    | std::io::ErrorKind::BrokenPipe // EPIPE, el mismo caso en unix
            )
        })
    })
}

/// Run one blob's upload, retrying it — and only it — while a pacer says
/// "too fast".
///
/// The upload of a save is N independent PUTs, one per missing blob, and N is
/// the user's file count: 122 for a Cyberpunk folder with 46 save slots. The
/// per-IP pacer allows a burst and then a steady rate, so on a fast link the
/// tail of a large upload is *expected* to be turned away a few times. Letting
/// that abort the set (`try_collect` cancels every sibling on the first error)
/// meant a large save could never finish: each attempt got roughly a burst's
/// worth of blobs through, kept none of them — a fresh `upload_id` stages from
/// zero — and re-uploaded everything on the next pass, forever.
///
/// `attempt` is a closure, not a future, because a retry needs a new body: the
/// file gets re-opened and re-hashed on the way out, so a save the game rewrote
/// mid-upload is still caught by the sha check rather than silently retried
/// with stale bytes.
async fn put_blob_paced<F, Fut>(
    relative_path: &str,
    paced_wait_ms: &AtomicU64,
    mut attempt: F,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut retries = 0u32;
    loop {
        let err = match attempt().await {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        let Some(hint_secs) = paced_wait_hint(&err) else {
            return Err(err);
        };
        if retries >= MAX_PACED_RETRIES_PER_BLOB {
            return Err(err.context(format!(
                "{relative_path}: still being rate limited after {retries} retries — \
                 the server's request limit is too tight for this save's file count"
            )));
        }
        // Honour the server's number when it gave a real one; otherwise back
        // off from the floor. Either way it's capped, so a bogus hint can't
        // park the upload.
        let wait = Duration::from_secs(u64::from(hint_secs))
            .max(PACED_RETRY_FLOOR * 2u32.pow(retries.min(5)))
            .min(PACED_RETRY_CEILING);
        let spent = paced_wait_ms.fetch_add(wait.as_millis() as u64, Ordering::Relaxed);
        if Duration::from_millis(spent) > PACED_WAIT_BUDGET {
            return Err(err.context(format!(
                "{relative_path}: gave up after {}s of rate-limit waiting",
                PACED_WAIT_BUDGET.as_secs()
            )));
        }
        retries += 1;
        tracing::debug!(
            file = relative_path,
            retries,
            wait_ms = wait.as_millis() as u64,
            hint_secs,
            "upload: paced by the server — retrying this blob"
        );
        tokio::time::sleep(wait).await;
    }
}

/// The source directory exists but holds no regular files to upload (only
/// empty subdirs, or nothing). Typed so the agent can treat it as "nothing to
/// back up" (a `BackupSkippedEmpty`) rather than a red "falló" — pushing an
/// empty snapshot would clobber the last good server copy. See
/// `agent::run_backup_with_retry`.
#[derive(Debug, thiserror::Error)]
#[error("no files found in {path}")]
pub struct EmptySource {
    pub path: PathBuf,
}

/// La carpeta rastreada no puede ser la de un juego: un perfil entero, una raíz
/// de sistema, un prefijo de Wine/Proton completo.
///
/// Existe porque la guarda estructural sólo corría **al dar de alta** (alta
/// manual, adopción, re-apuntar). Una fila envenenada de antes de que la guarda
/// existiera —o de una vía de alta que se olvidó de validar— no volvía a pasar
/// por ahí nunca, y seguía subiendo. Reportado en ago-2026: una Steam Deck
/// subiendo `steamapps/compatdata/423230/pfx`, el prefijo entero, 308 MB, para
/// una partida de unos KB.
///
/// Se comprueba en el camino del backup, antes de tocar el disco, para que
/// cubra a la vez lo viejo y cualquier alta futura que no valide.
#[derive(Debug, thiserror::Error)]
#[error("refusing to back up {path}: {reason}. Pick the game's own save folder inside it.")]
pub struct UnsafeSource {
    pub path: PathBuf,
    pub reason: String,
}

/// Ni un solo fichero de la carpeta se dejó leer.
///
/// El backup salta los ficheros ilegibles uno a uno ([`split_unreadable`]), pero
/// cuando no queda ninguno no hay snapshot que subir: publicar una versión vacía
/// borraría en la nube la última copia buena, igual que en [`EmptySource`]. La
/// diferencia con ése es el motivo, y el motivo es lo único accionable: "está
/// vacía" manda a mirar la ruta, "no se deja leer" manda a mirar el proveedor de
/// archivos —el disparador conocido es OneDrive Files On-Demand con el
/// proveedor parado, que deja los ficheros ahí, con su tamaño, y niega los
/// bytes.
#[derive(Debug, thiserror::Error)]
#[error("none of the {count} files in {path} could be read: {first}")]
pub struct UnreadableSource {
    pub path: PathBuf,
    /// Cuántos ficheros enumeró el recorrido (todos ilegibles).
    pub count: usize,
    /// El error del primero, que es el que explica al resto.
    pub first: String,
}

/// One file enumerated from the source directory.
#[derive(Debug, Clone)]
pub struct UploadFile {
    /// Forward-slash relative path used as the multipart filename header.
    pub relative_path: String,
    /// Absolute path on disk.
    pub absolute_path: PathBuf,
    /// File size in bytes (read once during the walk so progress totals are correct).
    pub size_bytes: u64,
    /// Last-modified time, captured during the walk. Used only to build the
    /// cheap skip-by-set-hash signature; `None` if the platform/FS didn't
    /// report one.
    pub modified: Option<SystemTime>,
}

/// What a per-save-cap trim left out of an upload. See
/// [`upload_directory_cloud`]'s trim-and-retry: when a save's logical size
/// exceeds the plan's per-save cap, the client uploads the newest files that
/// fit and reports the omitted tail here so the UI can tell the user their
/// plan isn't big enough (Free) — the backup succeeded, but *partial*.
#[derive(Debug, Clone)]
pub struct TrimInfo {
    pub kept_files: usize,
    pub kept_bytes: u64,
    pub omitted_files: usize,
    pub omitted_bytes: u64,
    /// Plan slug the cap belongs to (e.g. `"free"`), for the upgrade nudge.
    pub plan: String,
    /// The per-save cap in bytes that forced the trim.
    pub limit_bytes: u64,
}

/// Recorta `working` a lo que quepa bajo `limit`, quedándose con los ficheros
/// más nuevos, y describe lo que se quedó fuera.
///
/// `working` viene ordenado por mtime descendente, así que "lo que cabe" es
/// también "lo más reciente": una carpeta de partidas enorme sube parcial en
/// vez de fallar entera, y lo que se pierde es lo más viejo. Regla genérica a
/// propósito —recencia y tamaño, cero conocimiento por juego—.
///
/// `None` cuando ni el fichero más nuevo cabe: ahí no hay recorte posible y el
/// llamante tiene que tratarlo como un "demasiado grande" terminal.
///
/// Extraído para que el recorte preventivo (contra el tope ya conocido) y el
/// reactivo (contra el 413) sean literalmente el mismo código: dos criterios
/// que se separasen darían dos versiones distintas del mismo save según quién
/// hubiera recortado.
fn trim_to_cap(working: &mut Vec<&UploadFile>, limit: u64, plan: &str) -> Option<TrimInfo> {
    let mut kept: Vec<&UploadFile> = Vec::new();
    let mut kept_bytes = 0u64;
    for f in working.iter() {
        if kept_bytes + f.size_bytes <= limit {
            kept.push(*f);
            kept_bytes += f.size_bytes;
        }
    }
    if kept.is_empty() {
        return None;
    }
    let full_bytes: u64 = working.iter().map(|f| f.size_bytes).sum();
    let info = TrimInfo {
        kept_files: kept.len(),
        kept_bytes,
        omitted_files: working.len() - kept.len(),
        omitted_bytes: full_bytes - kept_bytes,
        plan: plan.to_string(),
        limit_bytes: limit,
    };
    *working = kept;
    Some(info)
}

/// Un fichero que el recorrido enumeró pero cuyos bytes no se pueden leer.
///
/// No es un fallo del backup: es contenido que **esta** copia no puede llevarse.
/// Viaja hasta el llamante porque una versión a la que le falta un fichero sin
/// que el usuario se entere es peor que un error a la cara — se sube lo que se
/// puede y se dice en voz alta lo que se ha quedado en tierra.
#[derive(Debug, Clone)]
pub struct UnreadableFile {
    /// Ruta relativa dentro del save, con la misma forma que en [`UploadFile`].
    pub relative_path: String,
    /// El error del sistema tal cual. Es lo único que distingue un placeholder
    /// de OneDrive sin hidratar de un permiso denegado o de un disco muriéndose,
    /// así que se transporta entero hasta la UI.
    pub error: String,
}

/// Result of a successful upload.
#[derive(Debug, Clone)]
pub struct UploadOutcome {
    pub snapshot: Snapshot,
    pub file_count: usize,
    pub total_bytes: u64,
    /// Ficheros que el recorrido vio y la subida **no** pudo llevarse porque sus
    /// bytes no se dejaron leer. Vacío en el caso normal. Que no esté vacío
    /// significa que esta versión es parcial, y el llamante tiene que decirlo:
    /// ver `AgentEvent::BackupFilesUnreadable`.
    pub unreadable: Vec<UnreadableFile>,
    /// `Some` when the save was too big for the plan's per-save cap and only
    /// its newest files were uploaded; `None` when the whole save went up.
    pub trimmed: Option<TrimInfo>,
    /// **Nada se subió: este contenido ya estaba en el server** y `snapshot`
    /// describe la versión que ya lo tenía (ADR 0021 D.8.3). Ver
    /// [`ServerHead`].
    pub landed: bool,
}

/// La cabeza que el server publica para un save: qué versión es y **qué
/// contenido tiene**, como digest de su manifiesto.
///
/// Es lo que hace posible el anti-relanzamiento robusto a caídas de ADR 0021
/// C.1: un flag local de "subida en curso" no sobrevive a un reinicio del
/// daemon —y con el servicio, reiniciar es rutina—, así que la pregunta "¿hace
/// falta subir esto?" se le hace **a la verdad del server**, que es
/// content-addressed. Si el digest de lo que íbamos a subir es el de la cabeza,
/// la subida anterior *aterrizó* y volver a subir sólo crearía una versión
/// duplicada: mismo contenido, número nuevo, cuota gastada y un pull inútil en
/// todos los demás equipos.
///
/// El digest lo trae el manifiesto de la nube (`latest_sha256`), que el motor ya
/// pide por su cuenta (D.12), así que el chequeo no cuesta ni una petición.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHead {
    pub version_num: i64,
    /// Digest del manifiesto de esa versión, tal y como lo calcula el server.
    /// Vacío = versión antigua (archivo entero, sin manifiesto por fichero): no
    /// se puede comparar y no se compara.
    pub digest: String,
}

/// Outcome of a skip-aware backup ([`upload_directory_checked`]).
// One value per backup run, moved straight to the caller — never stored in
// bulk, so the size gap between variants costs nothing worth a Box.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum BackupResult {
    /// The cheap set signature matched the cached one — nothing was read or
    /// uploaded. (Fast path.)
    Skipped,
    /// The cheap signature drifted (the game rewrote its save files, bumping
    /// mtimes) but the actual bytes are identical to the last upload, so no
    /// new snapshot was created. `signature` is the refreshed composite the
    /// caller should persist so the *next* check hits the fast path again
    /// instead of re-hashing the whole save every cycle.
    Unchanged { signature: String },
    /// A new snapshot was created. `signature` is the freshly-computed
    /// composite signature the caller should persist for the next skip check.
    Uploaded {
        outcome: UploadOutcome,
        signature: String,
    },
    /// **Ya estaba en el server**: el contenido local es, byte por byte, el de
    /// la versión que la nube publica como cabeza (ADR 0021 D.8.3). No se subió
    /// nada; el llamante adopta `version_num` como la versión a la que está
    /// sincronizado y persiste `signature`.
    ///
    /// El caso que lo produce es un reinicio del daemon con una subida en
    /// vuelo que sí llegó a comprometerse: el `in_flight` en memoria se perdió,
    /// pero el contenido está arriba.
    AlreadyLanded { version_num: i64, signature: String },
}

/// Cheap signature over the sorted `(relative_path, size, mtime)` set.
///
/// Deliberately *not* a content hash: it never reads file bytes, so it adds
/// no IO on top of the directory walk. Two walks with identical paths, sizes
/// and mtimes produce the same signature — which is exactly the "watcher
/// settled but nothing was actually written" case we want to skip. It will
/// not catch a rewrite that preserves size *and* mtime while changing bytes
/// (rare for game saves), trading that corner for zero read overhead.
pub fn compute_set_signature(files: &[UploadFile]) -> String {
    let mut h = Sha256::new();
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        h.update(f.size_bytes.to_le_bytes());
        let mtime_nanos = f
            .modified
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        h.update(mtime_nanos.to_le_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// Digest del manifiesto de una versión: **la identidad de contenido que el
/// server publica** (`save_versions.sha256` de una versión content-addressed, y
/// por tanto el `latest_sha256` del manifiesto de la nube).
///
/// Tiene que casar byte por byte con lo que hace el server al comprometer
/// (`cas_commit`): sha256 sobre las filas del manifiesto ordenadas por ruta,
/// cada una `ruta \0 sha \0 tamaño(le) \0`. Si esto se desviara, el chequeo de
/// D.8.3 no encontraría nunca una coincidencia y volveríamos a subir de más — un
/// fallo silencioso y caro, así que hay un test con un vector fijo.
///
/// `files` debe venir ordenado por ruta (es lo que devuelve [`walk_source`]).
/// Ordenar en Rust es orden de bytes y el server ordena en la base de datos, así
/// que una intercalación distinta puede darles digests distintos para el mismo
/// contenido: eso produce un falso negativo (subimos igual), nunca un falso
/// positivo (dos digests iguales sólo salen del mismo flujo de bytes).
pub fn manifest_digest<'a>(files: impl Iterator<Item = (&'a str, &'a str, i64)>) -> String {
    let mut h = Sha256::new();
    for (path, sha, size) in files {
        h.update(path.as_bytes());
        h.update([0u8]);
        h.update(sha.as_bytes());
        h.update([0u8]);
        h.update(size.to_le_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// Lo que entra en el digest de contenido en el sitio de los bytes de un fichero
/// que no se deja leer. Un valor cualquiera que no puede ser un prefijo de
/// contenido real, para que "ilegible" y "vacío" nunca den el mismo digest.
const UNREADABLE_MARKER: &[u8] = b"\x01hoard:unreadable\x01";

/// Content signature over the sorted `(relative_path, bytes)` set.
///
/// Unlike [`compute_set_signature`] this *reads every file*, so it's only used
/// as a fallback when the cheap signature drifted: many games (and some
/// background launchers / cloud-sync daemons) rewrite save files on a timer,
/// bumping the mtime without changing a single byte. The cheap check would
/// treat that as a change and cut a redundant snapshot every few hours; this
/// confirms whether the bytes actually moved before we upload.
///
/// **Un fichero ilegible no tumba la pasada**: se salta con un aviso y entra en
/// el digest por [`UNREADABLE_MARKER`] en vez de por sus bytes. La asimetría
/// anterior era el bug: [`walk_source`] ya salta a propósito lo que no puede
/// interrogar —"one unreadable transient file shouldn't lose the backup of
/// everything else"— y esta pasada propagaba cualquier error de lectura con `?`,
/// así que un solo fichero perdía el snapshot entero. Caso real: un placeholder
/// de OneDrive Files On-Demand ("the cloud file provider is not running") dentro
/// de un save de GTA San Andreas Definitive; 3.934 intentos en 13 días y ni una
/// versión subida.
///
/// El marcador —y no simplemente omitir la ruta— mantiene el digest **estable**
/// mientras el fichero siga ilegible, que es lo que hace que no se reintente en
/// bucle, y lo cambia en cuanto vuelve a leerse, que es exactamente cuando hay
/// que volver a subir.
async fn compute_content_signature(files: &[UploadFile]) -> String {
    use tokio::io::AsyncReadExt;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 128 * 1024];
    for f in files {
        h.update(f.relative_path.as_bytes());
        h.update([0u8]);
        // Los bytes se vuelcan al hash según llegan, igual que siempre: un save
        // de 2 GB no se materializa en RAM para firmarlo. Un fallo a mitad deja
        // en `h` el trozo que sí se leyó y luego el marcador, y eso no encalla
        // nada: quien decide si hay que re-leer es la firma *barata*, que sólo
        // mira rutas, tamaños y mtimes y no depende de esto.
        let read = async {
            let mut file = tokio::fs::File::open(&f.absolute_path)
                .await
                .with_context(|| format!("opening {}", f.absolute_path.display()))?;
            loop {
                let n = file
                    .read(&mut buf)
                    .await
                    .with_context(|| format!("reading {}", f.absolute_path.display()))?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(e) = read {
            tracing::warn!(
                path = %f.relative_path,
                error = %format!("{e:#}"),
                "hashing: skipping unreadable file"
            );
            h.update(UNREADABLE_MARKER);
        }
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// Aparta los ficheros cuyos bytes no se pueden leer **ahora mismo**, para que
/// la subida se lleve todo lo demás.
///
/// Es la otra mitad de la tolerancia de [`compute_content_signature`]: aquélla
/// evita que un ilegible tumbe la firma, y ésta evita que tumbe la
/// transferencia. Sin ella el fichero seguiría en la lista y reventaría más
/// abajo, en el tar del camino de nube o en el hasheo del CAS, que es donde
/// estaba media avería.
///
/// Se comprueba abriendo **y leyendo** el primer bloque, no sólo abriendo:
/// algunos proveedores de ficheros bajo demanda dejan abrir el handle y fallan
/// en la primera lectura. Cuesta un `open` por fichero encima del que hará la
/// subida, y sólo en el camino de subida —nunca en el muestreo L1 del motor, ni
/// en el restore, ni en la vista previa—, porque abrir un placeholder es lo que
/// dispara su hidratación: forzar eso en cada tick bajaría la carpeta entera
/// desde la nube del usuario para calcular un fingerprint.
///
/// Conserva el orden de entrada (`buffered`, no `buffer_unordered`): la lista
/// viene ordenada por ruta desde [`walk_source`] y el digest del manifiesto
/// depende de ese orden.
async fn split_unreadable(files: Vec<UploadFile>) -> (Vec<UploadFile>, Vec<UnreadableFile>) {
    let probes = files.into_iter().map(|f| {
        async move {
            match probe_readable(&f.absolute_path).await {
                Ok(()) => Ok(f),
                Err(e) => Err(UnreadableFile {
                    relative_path: f.relative_path.clone(),
                    error: format!("{e:#}"),
                }),
            }
        }
        .boxed()
    });
    let probed: Vec<_> = stream::iter(probes)
        .buffered(TRANSFER_CONCURRENCY)
        .collect()
        .await;
    let mut readable = Vec::with_capacity(probed.len());
    let mut unreadable = Vec::new();
    for outcome in probed {
        match outcome {
            Ok(f) => readable.push(f),
            Err(u) => {
                tracing::warn!(
                    path = %u.relative_path,
                    error = %u.error,
                    "upload: leaving out a file whose bytes can't be read"
                );
                unreadable.push(u);
            }
        }
    }
    (readable, unreadable)
}

/// ¿Se pueden leer los bytes de este fichero? Abre y lee un byte.
async fn probe_readable(path: &Path) -> Result<()> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    let mut byte = [0u8; 1];
    file.read(&mut byte)
        .await
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(())
}

/// Persisted skip signature is a composite `"<cheap>:<content>"`. We split it
/// back into its two halves; a legacy value with no `:` (pre-fallback state
/// files held only the cheap hash) is treated as cheap-only with no known
/// content hash, so the first drift after upgrading reads bytes once and then
/// stores the composite.
fn split_signature(sig: Option<&str>) -> (Option<&str>, Option<&str>) {
    match sig {
        None => (None, None),
        Some(s) => match s.split_once(':') {
            Some((cheap, content)) => (Some(cheap), Some(content)),
            None => (Some(s), None),
        },
    }
}

fn join_signature(cheap: &str, content: &str) -> String {
    format!("{cheap}:{content}")
}

/// Recorre `root` y devuelve los ficheros que **son dato de partida**.
///
/// `shields` son los patrones de fichero que el manifiesto declara para este
/// juego ([`crate::savefilter::shields_for_slug`]); pasar `&[]` deja al kernel
/// decidiendo sólo por nombre. Lo que
/// [`fileclass::classify`](hoard_core::kernel::fileclass::classify) marque como
/// [`Junk`](hoard_core::kernel::fileclass::FileClass::Junk) —basura del SO,
/// temporales, volcados de crash, telemetría del motor, cerrojos que el juego
/// tiene abiertos— no entra en el snapshot. La configuración sí entra: es en el
/// restore donde se decide si se escribe (ver `RestoreOptions::gate`).
///
/// **Todo el mundo tiene que pasar por aquí con los mismos `shields`.** La
/// firma barata de [`compute_set_signature`] se calcula sobre esta lista, y el
/// muestreo L1 del motor (`observe_local_fingerprint`) la compara contra la que
/// guardó el backup: dos filtros distintos dan dos firmas distintas para la
/// misma carpeta quieta, el reductor ve un cambio pendiente que nunca se
/// resuelve y queda un bucle caliente.
///
/// Symlinks are skipped on purpose: we don't want to follow links out of the
/// save directory, and tar archives with symlinks make restore ambiguous.
pub fn walk_source(root: &Path, shields: &[String]) -> Result<Vec<UploadFile>> {
    // Save de fichero suelto: el `local_path` ES el fichero. Sale un único
    // `UploadFile` con su nombre base como ruta relativa, de modo que el
    // snapshot tiene exactamente la misma forma que el de una carpeta con un
    // fichero dentro — y todo lo de aguas abajo (firma, dedup, restore) sigue
    // funcionando sin enterarse. Más de 8.000 entradas del manifiesto son así:
    // `<winAppData>/Game/save.dat`, `<base>/140.sav`.
    if root.is_file() {
        let meta =
            std::fs::metadata(root).with_context(|| format!("reading {}", root.display()))?;
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("save file has no usable name: {}", root.display()))?;
        // Un save de fichero suelto ES el fichero: la ruta se eligió apuntando
        // a él, así que aquí no se clasifica nada. Filtrarlo dejaría el save
        // vacío y el backup entero en `EmptySource` — el usuario señaló ese
        // fichero, y eso pesa más que cualquier regla por nombre.
        return Ok(vec![UploadFile {
            relative_path: name.to_string(),
            absolute_path: root.to_path_buf(),
            size_bytes: meta.len(),
            modified: meta.modified().ok(),
        }]);
    }

    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        // Un subdirectorio ilegible se salta; sólo la raíz es error duro.
        //
        // Antes esto era `?` para cualquier nivel, así que UNA carpeta sin
        // permiso abortaba el backup entero del juego. En Windows es la norma,
        // no la excepción: las junctions legacy del perfil
        // (`AppData\Local\Application Data`, que apunta a su propio padre)
        // devuelven acceso denegado y además son un ciclo. Perder una
        // subcarpeta ilegible es infinitamente mejor que perder el backup.
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if dir != root => {
                tracing::warn!(path = %dir.display(), error = %e, "skipping unreadable directory");
                continue;
            }
            Err(e) => {
                return Err(e).with_context(|| format!("reading dir {}", dir.display()));
            }
        };
        for entry in read {
            // Igual que arriba: una entrada que desaparece o no se puede
            // interrogar a mitad del paseo no invalida el resto.
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else {
                tracing::warn!(path = %path.display(), "skipping entry with unreadable type");
                continue;
            };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| anyhow!("strip_prefix: {e}"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                // Lo que no es dato de partida no entra en el snapshot.
                if !fileclass::classify(&rel, shields).is_backed_up() {
                    tracing::debug!(path = %rel, "skipping non-save file");
                    continue;
                }
                // A file we can't stat (locked, vanished mid-walk, permission)
                // is skipped with a warning rather than failing the whole
                // upload — one unreadable transient file shouldn't lose the
                // backup of everything else.
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "skipping unreadable file");
                        continue;
                    }
                };
                out.push(UploadFile {
                    relative_path: rel,
                    absolute_path: path,
                    size_bytes: meta.len(),
                    modified: meta.modified().ok(),
                });
            }
            // symlinks: ignored on purpose.
        }
    }
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

/// Upload a directory as a new snapshot for `save_id`.
///
/// `progress(uploaded, total)` is called once per file as it's added to the
/// multipart form. Both values are byte counts. The callback is `Fn` so the
/// caller can wire any UI on top.
///
/// `game_slug` and `label` are only consulted on the Hoard Cloud path, where
/// the server keys the save row on `(user_id, game_slug, label)` and the
/// snapshot list endpoints don't exist. They're ignored self-hosted.
///
/// **Un fichero cuyos bytes no se dejan leer se queda fuera y se reporta**, en
/// vez de perder el snapshot entero. De las dos salidas posibles —saltar el
/// fichero o aparcar el save— se elige saltar, porque aparcar es exactamente el
/// estado del que se viene: el caso que lo destapó (un placeholder de OneDrive
/// Files On-Demand con el proveedor parado, dentro de un save de GTA San Andreas
/// Definitive) llevaba 13 días y 3.934 intentos sin subir **nada**, y la causa
/// puede durar semanas. Vale más la partida entera menos un fichero que ninguna
/// partida.
///
/// El precio de esa elección se paga entero en [`UploadOutcome::unreadable`]: la
/// versión es parcial y quien la publica **tiene** que decirlo (el motor emite
/// `AgentEvent::BackupFilesUnreadable` y la UI deja un aviso pegajoso en la
/// tarjeta del juego). Una versión incompleta en silencio no es una opción: sólo
/// se descubriría al restaurar.
///
/// Si no queda **ningún** fichero legible no se sube nada
/// ([`UnreadableSource`]): publicar una versión vacía borraría en la nube la
/// última copia buena.
#[allow(clippy::too_many_arguments)]
pub async fn upload_directory<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    // `Sync` because the cloud path shares the callback by reference across
    // its in-flight uploads.
    F: Fn(u64, u64) + Send + Sync,
{
    let source = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    // Una carpeta o un fichero suelto; cualquier otra cosa (un socket, un
    // dispositivo) no es un save.
    if !source.is_dir() && !source.is_file() {
        bail!("source must be a folder or a file: {}", source.display());
    }

    let files = walk_source(&source, &crate::savefilter::shields_for_slug(game_slug))?;
    if files.is_empty() {
        return Err(EmptySource { path: source }.into());
    }
    // Un fichero que no se deja leer sale de la lista aquí, en el ÚNICO sitio por
    // el que pasan los cuatro caminos de subida (nube, CAS, pack, multipart), de
    // modo que ninguno se lo encuentra a mitad de la transferencia. Lo que se
    // quede fuera viaja en el `UploadOutcome` para que el llamante lo cuente: una
    // versión incompleta en silencio es el resultado que no vale.
    let (files, unreadable) = split_unreadable(files).await;
    if files.is_empty() {
        // No queda nada legible: subir aquí publicaría una versión vacía y
        // borraría en la nube la última copia buena.
        let first = unreadable
            .first()
            .map(|u| u.error.clone())
            .unwrap_or_default();
        return Err(UnreadableSource {
            path: source,
            count: unreadable.len(),
            first,
        }
        .into());
    }
    let total_bytes: u64 = files.iter().map(|f| f.size_bytes).sum();
    let file_count = files.len();

    // Elegir protocolo exige SABER cuál habla el servidor, no suponerlo.
    // `is_cloud()` colapsa "self-hosted" y "la sonda de `/v1/health` falló" en
    // el mismo `false` — cómodo para un adorno de UI, veneno aquí: con la sonda
    // caída se toma la rama self-hosted y se sube contra
    // `/v1/saves/:id/snapshots`, que en cloud **no existe**. El usuario ve
    // «uploading snapshot: not found (404)» y se va a buscar un save borrado
    // que está perfectamente. Reportado ago-2026, y el disparador es de lo más
    // corriente: la máquina de Fly se apaga por inactividad y la sonda pilla el
    // arranque en frío.
    //
    // Sin sonda resuelta no se elige: se falla como lo que es —el servidor no
    // está localizable— y el backoff de siempre lo reintenta.
    //
    // Las dos llamadas hacen falta y no son redundantes: `server_mode()` es
    // quien **sondea y cachea** (su propio `None` es igual de ambiguo, porque
    // se traga el error), y `probed_is_cloud()` es quien luego da la respuesta
    // honesta — sólo devuelve `Some` si hubo una sonda con éxito.
    let _ = client.server_mode().await;
    let Some(is_cloud) = client.probed_is_cloud() else {
        bail!(
            "can't tell which protocol this server speaks yet (the /v1/health probe hasn't \
             succeeded). Not guessing: uploading with the wrong one fails as a misleading 404."
        );
    };
    // Hoard Cloud (api.hoard.services) speaks a different protocol: the
    // self-hosted `/v1/saves/:id/snapshots` multipart endpoint doesn't exist
    // there. Pack the save into a single tar.zst, declare the upload, PUT the
    // bytes straight to R2 via a presigned URL, then commit.
    if is_cloud {
        let mut outcome = upload_directory_cloud(
            client,
            save_id,
            game_slug,
            label,
            &files,
            total_bytes,
            base_version,
            head,
            origin,
            progress,
        )
        .await?;
        outcome.unreadable = unreadable;
        return Ok(outcome);
    }

    // Self-hosted que sepa negociar el contenido: se le declara el manifiesto y
    // sólo viajan los blobs que no tenga. El multipart de aquí abajo se queda
    // para los servers anteriores a la 1.1.3, que no anuncian la capacidad.
    //
    // La condición es `Some(true)`, no `unwrap_or(false)`: un `None` significa
    // que la sonda no ha resuelto, y ese caso ya lo cortó el `bail!` de arriba.
    if client.probed_supports_cas() == Some(true) {
        let mut outcome = upload_directory_cas(
            client,
            save_id,
            &files,
            total_bytes,
            base_version,
            origin,
            progress,
        )
        .await?;
        outcome.unreadable = unreadable;
        return Ok(outcome);
    }

    // Ingesta adaptativa por forma del save (ADR 0019): muchos archivos
    // pequeños viajan mejor como un único tar (un round-trip, un handle) que
    // como N partes multipart. El umbral es por número de archivos; el server
    // desempaqueta el campo `pack` y deduplica por-archivo igual que el modo
    // normal, así que el modelo de almacenamiento no cambia.
    const PACK_THRESHOLD: usize = 500;

    let mut form = multipart::Form::new();
    // Declare the base version so the server can reject a non-fast-forward
    // (another device advanced this save since we last synced).
    if let Some(b) = base_version {
        form = form.text("base_version", b.to_string());
    }
    // Quién sube. La columna existe desde el primer día y el server la guarda y
    // la devuelve; lo que faltaba era que alguien la rellenara, así que el
    // historial no podía distinguir dos máquinas sincronizando la misma
    // partida.
    if let Some(device) = crate::logship::device_name() {
        form = form.text("device_name", device);
    }
    // Origen de la versión: el server acepta este campo desde siempre y nadie
    // lo rellenaba. Sin él la retención no puede distinguir la copia que el
    // usuario hizo antes del jefe de las cuarenta que hizo el temporizador.
    if let Some(note) = origin.as_note() {
        form = form.text("notes", note);
    }
    progress(0, total_bytes);

    if file_count > PACK_THRESHOLD {
        // Build the tar on the fly through an in-memory pipe and stream it as
        // the request body — never materialising the whole archive in RAM.
        let (writer, reader) = tokio::io::duplex(256 * 1024);
        let files_for_tar = files.clone();
        tokio::spawn(async move {
            let mut tar = tokio_tar::Builder::new(writer);
            for f in &files_for_tar {
                if let Err(e) = tar
                    .append_path_with_name(&f.absolute_path, &f.relative_path)
                    .await
                {
                    // Dropping the writer truncates the tar; the server then
                    // rejects it as a malformed pack, surfacing as an upload
                    // error rather than a silent partial snapshot.
                    tracing::warn!(error = %e, path = %f.relative_path, "pack tar build error");
                    return;
                }
            }
            if let Ok(mut inner) = tar.into_inner().await {
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut inner).await;
            }
        });
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let part = multipart::Part::stream(body)
            .file_name("pack.tar")
            .mime_str("application/x-tar")?;
        form = form.part("pack", part);
        progress(total_bytes, total_bytes);
    } else {
        let mut uploaded = 0u64;
        for f in &files {
            // Stream each file from disk instead of reading it whole into RAM:
            // open the handle, wrap it as a byte stream and hand it to reqwest
            // as a streaming multipart part. A 2 GB save no longer means 2 GB
            // of process memory.
            let file = tokio::fs::File::open(&f.absolute_path)
                .await
                .with_context(|| format!("reading {}", f.absolute_path.display()))?;
            let stream = tokio_util::io::ReaderStream::new(file);
            let body = reqwest::Body::wrap_stream(stream);
            let part = multipart::Part::stream_with_length(body, f.size_bytes)
                .file_name(f.relative_path.clone())
                .mime_str("application/octet-stream")?;
            // Server keys files by the field NAME = "files" and reads the
            // relative path from the multipart filename header.
            form = form.part("files", part);
            uploaded += f.size_bytes;
            progress(uploaded, total_bytes);
        }
    }

    let snap = client
        .snapshot_upload(save_id, form)
        .await
        .context("uploading snapshot")?;

    Ok(UploadOutcome {
        snapshot: snap,
        file_count,
        total_bytes,
        unreadable,
        // The self-hosted multipart path has no per-save cap trim.
        trimmed: None,
        landed: false,
    })
}

/// Whole-file SHA-256 of every file in the manifest, a few in flight at once so
/// per-file open/read latency overlaps instead of adding up.
///
/// (The futures are built eagerly into a Vec of `BoxFuture`s rather than through
/// `iter().map(closure)`: a closure over borrowed items retained inside the
/// stream trips rustc's "Send/FnOnce is not general enough" false positive when
/// the whole upload future crosses a `tokio::spawn`. One small allocation per
/// file, all of them IO-bound.)
async fn hash_manifest(files: &[UploadFile]) -> Result<HashMap<&str, String>> {
    let mut hash_futs = Vec::with_capacity(files.len());
    for f in files {
        hash_futs.push(
            async move {
                let sha = hash_file(&f.absolute_path).await?;
                Ok::<_, anyhow::Error>((f.relative_path.as_str(), sha))
            }
            .boxed(),
        );
    }
    stream::iter(hash_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect()
        .await
}

/// Subida self-hosted direccionada por contenido: hashear → declarar el
/// manifiesto → subir sólo los blobs que al server le falten → confirmar.
///
/// Es el mismo trato que [`upload_directory_cloud`] con una diferencia que
/// manda: los bytes van **al server**, no a un bucket. Self-hosted no firma URLs
/// (ADR 0020) porque detrás puede haber disco, MinIO o un `rclone serve s3`
/// sobre OneDrive; el server siempre está en medio. A cambio, self-hosted no
/// tiene plan ni tope por partida, así que aquí no hay recorte-y-reintento.
///
/// Lo que esto le quita a un self-hoster es la subida repetida: hasta la 1.1.2
/// una copia mandaba la carpeta entera aunque el server ya tuviera el contenido
/// —deduplicaba al guardar, no al transmitir—, así que una partida de 3 GB que
/// cambia 10 MB costaba 3 GB de subida, y por el camino chocaba contra
/// `max_snapshot_size_mb` y contra el límite de cuerpo de cualquier proxy que
/// hubiera delante.
async fn upload_directory_cas<F>(
    client: &ApiClient,
    save_id: &str,
    files: &[UploadFile],
    total_bytes: u64,
    base_version: Option<i64>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64) + Send + Sync,
{
    use hoard_core::ids::Sha256 as Sha256Hex;

    progress(0, total_bytes);
    let sha_by_path = hash_manifest(files).await?;

    let mut manifest: Vec<CasFile> = Vec::with_capacity(files.len());
    for f in files {
        let sha = &sha_by_path[f.relative_path.as_str()];
        manifest.push(CasFile {
            relative_path: f.relative_path.clone(),
            sha256: Sha256Hex::parse(sha)
                .with_context(|| format!("hashing {}", f.relative_path))?,
            size_bytes: f.size_bytes as i64,
            modified_at: f
                .modified
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64),
        });
    }

    let init = client
        .cas_init(
            save_id,
            &CasInit {
                base_version,
                files: manifest.clone(),
            },
        )
        .await
        .context("cas init")?;

    // Varios ficheros con el mismo contenido comparten blob: se sube una vez.
    let mut by_sha: HashMap<&str, &UploadFile> = HashMap::new();
    for f in files {
        by_sha
            .entry(sha_by_path[f.relative_path.as_str()].as_str())
            .or_insert(f);
    }

    // Resolver cada blob que falta a su fichero de origen **antes** de mover un
    // byte, para que un manifiesto que no cuadra aborte de entrada y no a media
    // subida.
    let mut pending: Vec<(&UploadFile, String)> = Vec::with_capacity(init.missing.len());
    for blob in &init.missing {
        let Some(f) = by_sha.get(blob.sha256.as_str()) else {
            bail!(
                "server requested a blob not in the manifest: {}",
                blob.sha256.as_str()
            );
        };
        pending.push((*f, blob.sha256.as_str().to_string()));
    }

    let upload_total: u64 = init
        .missing
        .iter()
        .map(|b| b.size_bytes.max(0) as u64)
        .sum();
    tracing::info!(
        save_id,
        files = files.len(),
        upload_blobs = pending.len(),
        upload_bytes = upload_total,
        logical_bytes = total_bytes,
        "self-hosted upload negotiated: only the missing blobs travel"
    );

    // La barra mide lo que de verdad se transmite, no el tamaño de la partida:
    // es la cifra que el usuario está esperando.
    let denom = upload_total.max(1);
    let uploaded = AtomicU64::new(0);
    // Shared across every worker: the pacer waits are the same queue, so the
    // give-up budget has to be counted once for the whole upload.
    let paced_wait_ms = AtomicU64::new(0);
    progress(0, denom);
    let mut put_futs = Vec::with_capacity(pending.len());
    for (f, sha) in pending {
        let uploaded = &uploaded;
        let paced_wait_ms = &paced_wait_ms;
        let progress = &progress;
        let upload_id = init.upload_id.as_str();
        put_futs.push(
            async move {
                put_blob_paced(&f.relative_path, paced_wait_ms, || async {
                    let file = tokio::fs::File::open(&f.absolute_path)
                        .await
                        .with_context(|| format!("opening {}", f.absolute_path.display()))?;
                    let (stream, sent) = hashing_stream(file);
                    client
                        .cas_upload_blob(
                            upload_id,
                            &sha,
                            reqwest::Body::wrap_stream(stream),
                            f.size_bytes,
                        )
                        .await
                        .with_context(|| format!("uploading {}", f.relative_path))?;
                    // El server rechaza un blob cuyo contenido no case con su sha,
                    // así que aquí ya no puede colarse contenido cruzado. Se
                    // comprueba igual para dar el mensaje bueno —"el juego rotó el
                    // save mientras subía"— en vez del 400 crudo del server.
                    {
                        let sent = sent.lock().map_err(|_| anyhow!("upload hasher poisoned"))?;
                        verify_sent(&f.relative_path, &sha, f.size_bytes, &sent)?;
                    }
                    Ok(())
                })
                .await?;
                let done = uploaded.fetch_add(f.size_bytes, Ordering::Relaxed) + f.size_bytes;
                progress(done, denom);
                Ok::<_, anyhow::Error>(())
            }
            .boxed(),
        );
    }
    stream::iter(put_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;
    progress(denom, denom);

    let snapshot = client
        .cas_commit(
            save_id,
            &CasCommit {
                upload_id: init.upload_id,
                base_version,
                device_name: crate::logship::device_name(),
                notes: origin.as_note().map(str::to_string),
                files: manifest,
            },
        )
        .await
        .context("cas commit")?;

    Ok(UploadOutcome {
        snapshot,
        file_count: files.len(),
        total_bytes,
        // Lo rellena `upload_directory`, que es quien aparta los ilegibles.
        unreadable: Vec::new(),
        // Sin plan no hay tope por partida que recortar.
        trimmed: None,
        landed: false,
    })
}

/// Hoard Cloud upload (content-addressed): hash each file → declare manifest
/// → upload only the blobs the server is missing → commit.
///
/// Unlike the old archive path this never packs the whole save: each file is
/// its own R2 object keyed by its whole-file SHA-256, so a 600 MB save the
/// game rewrote in place with 10 MB of real change costs a 10 MB upload. Files
/// are never decompressed — the game's `.v3`/zip blobs are deduped whole.
#[allow(clippy::too_many_arguments)]
async fn upload_directory_cloud<F>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    files: &[UploadFile],
    total_bytes: u64,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
) -> Result<UploadOutcome>
where
    F: Fn(u64, u64) + Send + Sync,
{
    progress(0, total_bytes);

    // 1. Whole-file SHA-256 of every file — the dedup key. Hashed once up
    //    front and cached by path so a per-save-cap trim-and-retry (below)
    //    doesn't re-read the files.
    let sha_by_path = hash_manifest(files).await?;

    // 1b. **¿Ya está arriba?** (ADR 0021 D.8.3.) Con los hashes ya calculados,
    // preguntarle a la verdad del server si este contenido exacto es su cabeza
    // no cuesta ni una petición ni una lectura más — y si lo es, la subida que
    // un reinicio del daemon dejó a medias sí llegó a comprometerse, así que
    // subir otra vez sólo crearía una versión duplicada (cuota, ops de R2 y un
    // pull inútil en los demás equipos). Anti-relanzamiento contra el server,
    // no contra un flag local que no sobrevive a un reinicio.
    if let Some(head) = head.filter(|h| !h.digest.is_empty()) {
        let digest = manifest_digest(files.iter().map(|f| {
            (
                f.relative_path.as_str(),
                sha_by_path[f.relative_path.as_str()].as_str(),
                f.size_bytes as i64,
            )
        }));
        if digest == head.digest {
            tracing::info!(
                save_id,
                version_num = head.version_num,
                "cloud upload skipped — this exact content is already the server's head"
            );
            return Ok(UploadOutcome {
                snapshot: landed_snapshot(save_id, head, files.len(), total_bytes),
                file_count: files.len(),
                total_bytes,
                unreadable: Vec::new(),
                trimmed: None,
                landed: true,
            });
        }
    }

    // Working set, newest first, so if the save is too big for the plan's
    // per-save cap we keep the most recent saves and drop the oldest — a
    // generic rule (recency + size only, no per-game knowledge) that lets a
    // huge Paradox `save games` folder back up *partially* instead of failing
    // whole. `trimmed` records what was left out for the UI's "your plan isn't
    // big enough" nudge.
    let mut working: Vec<&UploadFile> = files.iter().collect();
    working.sort_by_key(|f| std::cmp::Reverse(f.modified));
    let mut trimmed: Option<TrimInfo> = None;

    // Recorte PREVENTIVO: si ya sabemos el tope de este plan, no hace falta que
    // el servidor nos lo recuerde otra vez.
    //
    // El tope sólo se aprende siendo rechazado —no hay endpoint que lo diga—,
    // así que la primera copia grande de la sesión sigue costando un 413. Las
    // demás no: se recortan aquí y suben a la primera. Es la diferencia entre
    // preguntar una vez y preguntar en cada autoguardado, que es lo que
    // convirtió a cinco usuarios en 12.996 rechazos semanales.
    if let Some(cap) = client.plan_cap() {
        if total_bytes > cap.limit_bytes {
            if let Some(info) = trim_to_cap(&mut working, cap.limit_bytes, &cap.plan) {
                tracing::debug!(
                    save_id,
                    game_slug,
                    limit_bytes = cap.limit_bytes,
                    kept_files = info.kept_files,
                    omitted_files = info.omitted_files,
                    "cloud: trimmed to the known per-save cap without asking"
                );
                trimmed = Some(info);
            }
        }
    }

    // 2/3/4. Declare manifest → upload missing blobs → commit. Wrapped in a
    // loop so a per-save-cap 413 can trim the working set and retry exactly
    // once (the trim can only shrink, so it converges).
    let (init, by_sha, file_count, total_bytes) = loop {
        let file_count = working.len();
        let logical: u64 = working.iter().map(|f| f.size_bytes).sum();

        let manifest: Vec<CloudCasFileEntry> = working
            .iter()
            .map(|f| CloudCasFileEntry {
                relative_path: f.relative_path.clone(),
                sha256: sha_by_path[f.relative_path.as_str()].clone(),
                size_bytes: f.size_bytes as i64,
                modified_at: f
                    .modified
                    .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64),
            })
            .collect();

        match client
            .cloud_cas_init(&CloudCasInit {
                save_id: save_id.to_string(),
                game_slug: game_slug.to_string(),
                label: Some(label.to_string()),
                device_name: crate::logship::device_name(),
                notes: origin.as_note().map(str::to_string),
                backup_only: false,
                base_version,
                files: manifest,
            })
            .await
        {
            Ok(init) => {
                // Files sharing a SHA upload once.
                let mut by_sha: HashMap<&str, &UploadFile> = HashMap::new();
                for f in &working {
                    by_sha
                        .entry(sha_by_path[f.relative_path.as_str()].as_str())
                        .or_insert(*f);
                }
                break (init, by_sha, file_count, logical);
            }
            Err(e) => {
                // Per-save size cap (413). Trim to the newest files that fit
                // under the cap and retry once. Only trim on the first hit
                // (`trimmed.is_none()`) so we can't loop.
                let cap = if trimmed.is_none() {
                    e.downcast_ref::<crate::api::ApiError>()
                        .and_then(|api_err| match api_err {
                            crate::api::ApiError::TooLarge(d) if d.limit_bytes > 0 => {
                                Some(d.clone())
                            }
                            _ => None,
                        })
                } else {
                    None
                };
                let Some(detail) = cap else {
                    return Err(e).context("cloud cas init");
                };
                // Apúntalo antes de nada: aunque este recorte falle, la próxima
                // copia ya no tendrá que preguntar.
                client.remember_plan_cap(detail.limit_bytes, &detail.plan);
                let Some(info) = trim_to_cap(&mut working, detail.limit_bytes, &detail.plan) else {
                    // Even the single newest file is over the cap — nothing to
                    // trim to; let the caller surface it as terminal too-large.
                    return Err(e).context("cloud cas init");
                };
                tracing::warn!(
                    save_id,
                    game_slug,
                    plan = %detail.plan,
                    limit_bytes = detail.limit_bytes,
                    kept_files = info.kept_files,
                    omitted_files = info.omitted_files,
                    "cloud: save exceeds plan per-save cap — uploading only the newest files that fit"
                );
                trimmed = Some(info);
                continue;
            }
        }
    };
    let upload_total: u64 = init
        .missing
        .iter()
        .map(|b| b.size_bytes.max(0) as u64)
        .sum();
    // Progress is reported against the bytes actually transferred, so the bar
    // reflects the real (deduped) upload rather than the whole save size.
    let denom = upload_total.max(1);
    // Resolve every missing blob to its source file before moving any bytes,
    // so a manifest mismatch aborts up front rather than mid-upload.
    let mut pending: Vec<(&CloudCasMissingBlob, &UploadFile)> =
        Vec::with_capacity(init.missing.len());
    for blob in &init.missing {
        let Some(f) = by_sha.get(blob.sha256.as_str()) else {
            bail!(
                "server requested a blob not in the manifest: {}",
                blob.sha256
            );
        };
        pending.push((blob, *f));
    }
    // A few PUTs in flight at once: presigned-URL round-trip latency, not
    // bandwidth, dominates the many-small-blobs shape. Completion order is
    // irrelevant — each blob is its own R2 object — so progress just counts
    // bytes as they land. (Eager Vec of boxed futures for the same
    // trait-inference reason as the hashing pass above.)
    let uploaded = AtomicU64::new(0);
    // Shared across every worker: the pacer waits are the same queue, so the
    // give-up budget has to be counted once for the whole upload.
    let paced_wait_ms = AtomicU64::new(0);
    progress(0, denom);
    let mut put_futs = Vec::with_capacity(pending.len());
    for (blob, f) in pending {
        let uploaded = &uploaded;
        let paced_wait_ms = &paced_wait_ms;
        let progress = &progress;
        put_futs.push(
            async move {
                put_blob_paced(&f.relative_path, paced_wait_ms, || async {
                    let file = tokio::fs::File::open(&f.absolute_path)
                        .await
                        .with_context(|| format!("opening {}", f.absolute_path.display()))?;
                    let (stream, sent) = hashing_stream(file);
                    client
                        .put_presigned(
                            &blob.upload,
                            reqwest::Body::wrap_stream(stream),
                            f.size_bytes,
                        )
                        .await
                        .with_context(|| format!("uploading {}", f.relative_path))?;
                    // El objeto ya está en el bucket, pero **sin commit no existe
                    // para nadie**: la fila de `cloud_blobs` se crea al confirmar la
                    // versión, y el dedup del servidor mira esa tabla, no el bucket.
                    // Así que abortar aquí deja el objeto huérfano (lo barre el GC) y
                    // el intento siguiente lo vuelve a pedir y lo sobreescribe con el
                    // contenido bueno. Lo que no puede pasar —y es lo que pasaba— es
                    // que se confirme una versión que apunta a bytes que no son.
                    {
                        let sent = sent.lock().map_err(|_| anyhow!("upload hasher poisoned"))?;
                        verify_sent(&f.relative_path, &blob.sha256, f.size_bytes, &sent)?;
                    }
                    Ok(())
                })
                .await?;
                let done = uploaded.fetch_add(f.size_bytes, Ordering::Relaxed) + f.size_bytes;
                progress(done, denom);
                Ok::<_, anyhow::Error>(())
            }
            .boxed(),
        );
    }
    stream::iter(put_futs)
        .buffer_unordered(TRANSFER_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;

    // 4. Commit — the server verifies the new blobs landed and finalizes.
    // The commit must target the *canonical* cloud save id: when another
    // device already created this (game, label) under a different id, the
    // server resolved ours to that one at init, and committing against our
    // local id would 404 forever.
    let canonical_id = init.save_id.as_deref().unwrap_or(save_id);
    if canonical_id != save_id {
        tracing::info!(
            local_save_id = save_id,
            canonical_save_id = canonical_id,
            game_slug,
            label,
            "cloud save id diverged — committing against the canonical cloud id"
        );
    }
    let commit = client
        .cloud_cas_commit(canonical_id, init.version_num)
        .await
        .context("cloud cas commit")?;

    // Synthesize a Snapshot for the shared `UploadOutcome` shape.
    // `total_size_bytes` is the logical save size (sum of file sizes), matching
    // self-hosted snapshot semantics.
    let snapshot = Snapshot {
        id: String::new(),
        // El id canónico lo devuelve el commit cloud; si viniera con una forma
        // que la puerta no reconoce, el `Snapshot` sintético se queda sin él en
        // vez de tumbar un backup que YA subió los bytes.
        save_id: SaveId::parse(&commit.save_id).ok(),
        version_num: commit.version_num,
        parent_version: base_version,
        device_name: crate::logship::device_name(),
        notes: origin.as_note().map(str::to_string),
        file_count: file_count as i64,
        total_size_bytes: total_bytes as i64,
        is_pinned: false,
        created_at: OffsetDateTime::now_utc(),
        deleted_at: None,
        // Derived server-side from the manifest, and the cloud commit response
        // doesn't carry it back. Nothing is lost: this synthetic snapshot only
        // reports what just landed, and the History view reads the real row.
        insight: None,
    };
    Ok(UploadOutcome {
        snapshot,
        file_count,
        total_bytes,
        unreadable: Vec::new(),
        trimmed,
        landed: false,
    })
}

/// El `Snapshot` que describe una subida que **ya había aterrizado**: la versión
/// es la del server (no una inventada) y el recuento, el del contenido local,
/// que por definición es el mismo. No se pide al server: la gracia de D.8.3 es
/// ahorrarse el viaje, y lo único que el llamante necesita es a qué versión
/// quedamos sincronizados.
fn landed_snapshot(
    save_id: &str,
    head: &ServerHead,
    file_count: usize,
    total_bytes: u64,
) -> Snapshot {
    Snapshot {
        id: String::new(),
        save_id: SaveId::parse(save_id).ok(),
        version_num: head.version_num,
        parent_version: None,
        device_name: crate::logship::device_name(),
        notes: None,
        file_count: file_count as i64,
        total_size_bytes: total_bytes as i64,
        is_pinned: false,
        created_at: OffsetDateTime::now_utc(),
        deleted_at: None,
        insight: None,
    }
}

/// SHA-256 of a file's bytes, read in fixed-size chunks.
///
/// Shared with the restore side: the same whole-file digest that keys the
/// upload's dedup against the server's blobs keys the download's dedup against
/// the local disk (ADR 0021 D.13). There is no per-file hash *cache* to reuse —
/// `state.json`'s `set_hash` is a signature over the whole set (paths + sizes +
/// mtimes, plus a content hash of the concatenation), not per-file digests — so
/// both sides hash on demand.
pub(crate) async fn hash_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("hashing {}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .await
            .with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// Wrap a file as a streaming reqwest body for the presigned PUT.
/// Lo que de verdad salió por el socket en un PUT.
#[derive(Default)]
pub(crate) struct Sent {
    digest: Sha256,
    len: u64,
}

impl Sent {
    fn sha256(&self) -> String {
        hex::encode(self.digest.clone().finalize())
    }
}

/// El fichero como stream, hasheando **lo que se manda** en vez de confiar en
/// lo que se leyó antes.
///
/// El hash y el PUT son dos lecturas distintas del mismo fichero, y entre las
/// dos el juego puede haber rotado el save (`save` → `save.bak`, y un `save`
/// nuevo en su sitio: es el patrón normal de autoguardado). Cuando eso pasa, al
/// bucket se suben los bytes NUEVOS bajo el sha de los VIEJOS: un blob cuyo
/// contenido no es el que su nombre promete. Restaurarlo devuelve otra partida
/// —o basura—, y nada en el camino se queja. Es la única corrupción silenciosa
/// que hemos encontrado (ago-2026, ~1,7% de la población en riesgo).
///
/// Hasheando el propio stream, después del PUT se puede comprobar. Ver
/// [`verify_sent`] para qué se hace con el veredicto.
fn hashing_stream(
    file: tokio::fs::File,
) -> (
    impl futures::Stream<Item = std::io::Result<bytes::Bytes>>,
    std::sync::Arc<std::sync::Mutex<Sent>>,
) {
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Sent::default()));
    let tap = sent.clone();
    let stream = tokio_util::io::ReaderStream::new(file).inspect_ok(move |chunk| {
        if let Ok(mut s) = tap.lock() {
            s.digest.update(chunk);
            s.len += chunk.len() as u64;
        }
    });
    (stream, sent)
}

/// ¿Lo que salió es lo que se declaró? Puro para poder probarlo sin red.
///
/// No basta con el tamaño: una rotación puede dejar un fichero del mismo largo.
/// Manda el sha; el tamaño se comprueba también porque un desajuste ahí
/// significa además que el `content-length` del PUT mintió.
fn verify_sent(
    relative_path: &str,
    declared_sha: &str,
    declared_len: u64,
    sent: &Sent,
) -> Result<()> {
    let actual = sent.sha256();
    if actual == declared_sha && sent.len == declared_len {
        return Ok(());
    }
    bail!(
        "{relative_path} changed while it was being uploaded \
         (declared {declared_sha} / {declared_len} B, sent {actual} / {} B). \
         Nothing is committed; the next backup will pick up the new contents.",
        sent.len
    )
}

/// Skip-aware wrapper around [`upload_directory`] (ADR 0019).
///
/// Two-tier check against the persisted composite `prev_signature`:
/// 1. Cheap `(path, size, mtime)` signature matches → [`BackupResult::Skipped`],
///    no file read, no network.
/// 2. Cheap drifted (usually just an mtime bump from a game/daemon rewriting
///    saves on a timer) → read bytes once; if the content hash matches the
///    stored one → [`BackupResult::Unchanged`] carrying the refreshed composite
///    so the next cycle hits the fast path again instead of re-hashing.
/// 3. Bytes actually moved → upload and return [`BackupResult::Uploaded`].
///
/// **Both gates are skipped for a deliberate copy** ([`VersionOrigin::is_deliberate`]:
/// the user's own "back up now" and the safety net taken before a restore
/// overwrites the folder). A copy the user
/// asked for is a marker they placed — "right here, before the boss" — and
/// whether the bytes happen to match the last autosave is beside the point. It
/// used to fall through the same gate as a watcher no-op, so pressing the button
/// with nothing changed did nothing at all: no version, no message, just an INFO
/// line in a log file the user never opens (ago-2026). The rest of the design
/// already assumes a deliberate copy is worth keeping on its own terms — manual
/// versions have their own budget precisely so an autosave burst can't evict
/// them.
///
/// It cannot loop: the gates exist to stop the watcher re-cutting identical
/// snapshots on a timer, and this path only runs when a person presses a button.
/// It costs no transfer either — the content is addressed, so the blobs are
/// already there and the commit only adds a version row.
///
/// The signature persisted by the caller is `"<cheap>:<content>"`.
#[allow(clippy::too_many_arguments)]
pub async fn upload_directory_checked<F, G>(
    client: &ApiClient,
    save_id: &str,
    game_slug: &str,
    label: &str,
    source: &Path,
    prev_signature: Option<&str>,
    base_version: Option<i64>,
    head: Option<&ServerHead>,
    origin: VersionOrigin,
    progress: F,
    on_upload_start: G,
) -> Result<BackupResult>
where
    F: Fn(u64, u64) + Send + Sync,
    G: FnOnce(),
{
    // Antes de tocar el disco: una raíz imposible no se recorre. Recorrer un
    // prefijo de Proton entero para descubrir que no había que subirlo cuesta
    // justo lo que se quiere evitar.
    if let Some(reason) = crate::junkdirs::dangerous_sync_root(source) {
        return Err(UnsafeSource {
            path: source.to_path_buf(),
            reason,
        }
        .into());
    }
    let canonical = source
        .canonicalize()
        .with_context(|| format!("source path does not exist: {}", source.display()))?;
    if !canonical.is_dir() && !canonical.is_file() {
        bail!("source must be a folder or a file: {}", canonical.display());
    }
    // Y otra vez sobre la ruta resuelta: un enlace simbólico inocente puede
    // apuntar al perfil entero, y lo que se recorre es el destino.
    if let Some(reason) = crate::junkdirs::dangerous_sync_root(&canonical) {
        return Err(UnsafeSource {
            path: canonical,
            reason,
        }
        .into());
    }
    let files = walk_source(&canonical, &crate::savefilter::shields_for_slug(game_slug))?;
    if files.is_empty() {
        return Err(EmptySource { path: canonical }.into());
    }
    let (prev_cheap, prev_content) = split_signature(prev_signature);
    let cheap = compute_set_signature(&files);
    // `is_deliberate` y no `== Manual`: la red de seguridad previa a un restore
    // cuenta igual, y ahí saltársela es peor — es la copia que permite deshacer
    // un restore equivocado.
    let deliberate = origin.is_deliberate();
    if !deliberate && prev_cheap == Some(cheap.as_str()) {
        // Fast path: the cheap (path, size, mtime) signature is unchanged, so
        // the bytes can't have moved either — skip without reading any file.
        return Ok(BackupResult::Skipped);
    }
    // The cheap signature drifted. That's often just an mtime bump (a game or
    // background daemon rewriting save files on a timer), so confirm whether
    // the actual bytes changed before cutting a snapshot.
    let content = compute_content_signature(&files).await;
    if !deliberate && prev_content == Some(content.as_str()) {
        return Ok(BackupResult::Unchanged {
            signature: join_signature(&cheap, &content),
        });
    }
    // The bytes genuinely moved: we're about to push a real snapshot. Signal
    // it now (after every skip/unchanged check) so callers only surface a
    // "uploading…" notice when something actually uploads.
    on_upload_start();
    let outcome = upload_directory(
        client,
        save_id,
        game_slug,
        label,
        &canonical,
        base_version,
        head,
        origin,
        progress,
    )
    .await?;
    // El contenido ya estaba arriba (D.8.3): no hubo subida, pero sí una versión
    // a la que quedamos sincronizados. Se distingue de `Uploaded` porque el
    // llamante NO debe contarlo como backup con commit — mover el ancla del
    // min-interval con algo que no se subió es la regresión R.E.P.O.
    if outcome.landed {
        return Ok(BackupResult::AlreadyLanded {
            version_num: outcome.snapshot.version_num,
            signature: join_signature(&cheap, &content),
        });
    }
    Ok(BackupResult::Uploaded {
        outcome,
        signature: join_signature(&cheap, &content),
    })
}

/// Persist (or refresh) the `(save_id → local_path)` mapping in `state.json`.
///
/// If `remember` is true, fetch the save's metadata from the server and write
/// a fresh entry. If false but an entry already exists, just bump the
/// `last_backup_at` and `last_version_num` fields.
pub async fn remember_save(
    client: &ApiClient,
    state: &mut CliState,
    save_id: &str,
    local_path: &Path,
    last_version_num: i64,
    remember: bool,
) -> Result<()> {
    if remember {
        let save = client.get_save(save_id).await?;
        // Preserve any user-set pause flag if the entry already existed —
        // re-fetching from the server shouldn't silently un-pause it.
        let was_paused = state.saves.get(save_id).map(|s| s.paused).unwrap_or(false);
        // Preserve the skip-by-hash signature across a metadata refresh too,
        // so re-remembering a save doesn't force a redundant next upload.
        let prev_hash = state.saves.get(save_id).and_then(|s| s.set_hash.clone());
        let prev_preset = state.saves.get(save_id).and_then(|s| s.preset.clone());
        let prev_processes = state
            .saves
            .get(save_id)
            .map(|s| s.processes.clone())
            .unwrap_or_default();
        let prev_shared = state.saves.get(save_id).is_some_and(|s| s.shared_processes);
        // Same as the pause flag and the preset: a metadata refresh cannot undo
        // a user setting. This one decides whether their config gets written on
        // restore, so losing it here would be losing it silently.
        let prev_allow_device_local = state.saves.get(save_id).and_then(|s| s.allow_device_local);
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                local_path: local_path.to_path_buf(),
                game_slug: save.game_slug.into_inner(),
                label: save.label,
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                paused: was_paused,
                preset: prev_preset,
                set_hash: prev_hash,
                processes: prev_processes,
                shared_processes: prev_shared,
                allow_device_local: prev_allow_device_local,
            },
        );
    } else if let Some(existing) = state.saves.get(save_id).cloned() {
        state.saves.insert(
            save_id.to_string(),
            SaveState {
                last_backup_at: Some(OffsetDateTime::now_utc()),
                last_version_num: Some(last_version_num),
                ..existing
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod trim_tests {
    use super::*;

    fn f(path: &str, size: u64, secs: u64) -> UploadFile {
        UploadFile {
            relative_path: path.to_string(),
            absolute_path: PathBuf::from(path),
            size_bytes: size,
            modified: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
        }
    }

    /// El recorte se queda con los más nuevos, no con los primeros que vea.
    #[test]
    fn keeps_the_newest_files_that_fit() {
        let files = vec![f("old", 60, 100), f("new", 30, 300), f("mid", 30, 200)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        working.sort_by_key(|f| std::cmp::Reverse(f.modified));

        let info = trim_to_cap(&mut working, 60, "free").expect("something fits");

        let kept: Vec<&str> = working.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(kept, ["new", "mid"]);
        assert_eq!(info.kept_bytes, 60);
        assert_eq!(info.omitted_files, 1);
        assert_eq!(info.omitted_bytes, 60);
        assert_eq!(info.limit_bytes, 60);
    }

    /// Ni el más nuevo cabe: no hay recorte, y el llamante debe tratarlo como
    /// un "demasiado grande" terminal en vez de subir una copia vacía.
    #[test]
    fn refuses_when_even_the_newest_file_is_over_the_cap() {
        let files = vec![f("huge", 500, 300)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        assert!(trim_to_cap(&mut working, 100, "free").is_none());
        // Y no toca el conjunto: nada se ha decidido todavía.
        assert_eq!(working.len(), 1);
    }

    /// Todo cabe: se conserva entero y el informe dice que no se omitió nada.
    #[test]
    fn keeps_everything_when_it_all_fits() {
        let files = vec![f("a", 10, 100), f("b", 10, 200)];
        let mut working: Vec<&UploadFile> = files.iter().collect();
        let info = trim_to_cap(&mut working, 1000, "pro").expect("all fits");
        assert_eq!(working.len(), 2);
        assert_eq!(info.omitted_files, 0);
        assert_eq!(info.omitted_bytes, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uf(rel: &str, size: u64, mtime_secs: u64) -> UploadFile {
        UploadFile {
            relative_path: rel.to_string(),
            absolute_path: PathBuf::from("/x").join(rel),
            size_bytes: size,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs)),
        }
    }

    /// The heart of the issue #17 fix: a server that answers and closes without
    /// draining the body must be recognised as pacing.
    ///
    /// Assuming the `io::Error` survives the `reqwest` → `hyper` → `anyhow`
    /// trip is not good enough, so the real failure is manufactured here: a
    /// listener that accepts, reads a little and aborts with RST while the
    /// client is still writing a large body. Exactly what the per-IP limiter
    /// does when it turns a PUT away.
    #[tokio::test]
    async fn a_reset_while_writing_the_body_reads_as_pacing() {
        use tokio::io::AsyncReadExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // Just enough for the client to have started writing the body.
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            // Closed with the body half-arrived. The client keeps writing,
            // those bytes land on a closed socket and the server's stack
            // answers RST: exactly what a client sees when its PUT is turned
            // away undrained.
            drop(sock);
        });

        // Large, lazy body: it has to still be writing when the RST lands, or
        // the failure would be reading the response rather than writing.
        let body = reqwest::Body::wrap_stream(stream::iter(
            (0..4096).map(|_| Ok::<_, std::io::Error>(vec![0u8; 16 * 1024])),
        ));
        let sent = reqwest::Client::new()
            .put(format!("http://{addr}/v1/cas/blobs/x/y"))
            .body(body)
            .send()
            .await;

        let err = anyhow::Error::from(sent.expect_err("the server aborts the connection"))
            .context("uploading promo/sandbox/junkyard.jpg");

        assert!(
            is_body_write_reset(&err),
            "an aborted body write must be recognised; got: {err:#}"
        );
        assert_eq!(
            paced_wait_hint(&err),
            Some(0),
            "and must be treated as pacing without a hint"
        );
    }

    /// The other half: an error that is not a socket teardown still aborts the
    /// batch. Without this the pacer would swallow any failure and retry a blob
    /// that is never going to upload six times over.
    #[test]
    fn an_unrelated_error_is_not_pacing() {
        let err = anyhow::anyhow!("opening /x/save.dat: no such file or directory");
        assert!(!is_body_write_reset(&err));
        assert_eq!(paced_wait_hint(&err), None);
    }

    /// El stream que alimenta el PUT tiene que hashear **lo que manda**, no lo
    /// que se leyó antes. Se comprueba contra `hash_file`, que es el hash que se
    /// declara en el manifiesto: si los dos coinciden sobre el mismo fichero, la
    /// comprobación posterior no puede dar falsos positivos.
    #[tokio::test]
    async fn the_upload_stream_hashes_exactly_what_it_sends() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("save.dat");
        // Más de un chunk de `ReaderStream` (8 KiB) para que el digest tenga que
        // acumular de verdad.
        let bytes: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &bytes).unwrap();

        let file = tokio::fs::File::open(&path).await.unwrap();
        let (stream, sent) = hashing_stream(file);
        let drained: u64 = stream
            .fold(0u64, |acc, chunk| async move {
                acc + chunk.unwrap().len() as u64
            })
            .await;

        // El hash del fichero se pide ANTES de coger el candado: sostenerlo a
        // través de un `await` es lo que denuncia `clippy::await_holding_lock`,
        // y la CI corre clippy con `-D warnings` sobre `--all-targets`.
        let want = hash_file(&path).await.unwrap();

        let sent = sent.lock().unwrap();
        assert_eq!(drained, bytes.len() as u64);
        assert_eq!(sent.len, bytes.len() as u64);
        assert_eq!(sent.sha256(), want);
        assert!(verify_sent("save.dat", &sent.sha256(), sent.len, &sent).is_ok());
    }

    /// La rotación, que es el caso real: se declara el sha del `save` viejo y
    /// por el socket salen los bytes del nuevo. Antes de esto se confirmaba la
    /// versión igual y el blob quedaba mintiendo sobre su contenido.
    #[tokio::test]
    async fn a_file_rotated_mid_upload_is_caught() {
        let tmp = tempfile::tempdir().unwrap();
        let viejo = tmp.path().join("viejo.dat");
        let nuevo = tmp.path().join("nuevo.dat");
        std::fs::write(&viejo, b"partida de ayer").unwrap();
        // Mismo tamaño a propósito: el largo no basta como árbitro.
        std::fs::write(&nuevo, b"partida de HOY!").unwrap();
        let sha_viejo = hash_file(&viejo).await.unwrap();
        let len = std::fs::metadata(&viejo).unwrap().len();
        assert_ne!(sha_viejo, hash_file(&nuevo).await.unwrap());

        // Se sube lo que hay ahora (el fichero rotado) bajo el sha declarado.
        let file = tokio::fs::File::open(&nuevo).await.unwrap();
        let (stream, sent) = hashing_stream(file);
        stream.for_each(|_| async {}).await;

        let sent = sent.lock().unwrap();
        assert_eq!(sent.len, len, "misma longitud: sólo el sha lo delata");
        let err = verify_sent("save.dat", &sha_viejo, len, &sent).unwrap_err();
        assert!(
            err.to_string()
                .contains("changed while it was being uploaded"),
            "{err}"
        );

        // Y un tamaño distinto también se caza (fichero truncado a media subida).
        let corto = Sent {
            digest: Sha256::new(),
            len: 3,
        };
        assert!(verify_sent("save.dat", &corto.sha256(), len, &corto).is_err());
    }

    #[test]
    fn signature_stable_for_identical_set() {
        let a = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        let b = vec![uf("a.sav", 10, 100), uf("b.sav", 20, 200)];
        assert_eq!(compute_set_signature(&a), compute_set_signature(&b));
    }

    #[test]
    fn signature_changes_on_size_mtime_or_path() {
        let base = [uf("a.sav", 10, 100)];
        let base_sig = compute_set_signature(&base);
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 11, 100)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("a.sav", 10, 101)]));
        assert_ne!(base_sig, compute_set_signature(&[uf("b.sav", 10, 100)]));
    }

    #[test]
    fn signature_distinguishes_extra_file() {
        let one = vec![uf("a.sav", 10, 100)];
        let two = vec![uf("a.sav", 10, 100), uf("b.sav", 5, 50)];
        assert_ne!(compute_set_signature(&one), compute_set_signature(&two));
    }

    #[test]
    fn split_join_round_trip() {
        assert_eq!(split_signature(None), (None, None));
        // Legacy cheap-only state (pre-fallback): no content half.
        assert_eq!(split_signature(Some("abc")), (Some("abc"), None));
        let composite = join_signature("cheap", "content");
        assert_eq!(composite, "cheap:content");
        assert_eq!(
            split_signature(Some(&composite)),
            (Some("cheap"), Some("content"))
        );
    }

    /// El digest del manifiesto tiene que ser **el mismo número** que calcula el
    /// server al comprometer una versión content-addressed
    /// (`hoard-server/src/cloud/routes/saves.rs`, `cas_commit`): sha256 de
    /// `ruta \0 sha \0 tamaño(i64 le) \0` por fila, ordenadas por ruta. Si
    /// nuestra mitad se desviara, el chequeo de D.8.3 no encontraría nunca una
    /// coincidencia y volveríamos a subir de más **en silencio** — no hay error
    /// que mirar, sólo factura. Por eso el vector va fijo y calculado aparte, no
    /// derivado de esta misma función.
    #[test]
    fn manifest_digest_matches_the_servers_algorithm() {
        let rows = [
            ("saves/autosave.sav", "9f".repeat(32), 4096i64),
            ("saves/slot1.sav", "ab".repeat(32), 12i64),
        ];
        let digest = manifest_digest(rows.iter().map(|(p, sha, size)| (*p, sha.as_str(), *size)));
        assert_eq!(
            digest, "729ed0eaf73d058e463dea699aa20a6d131b9a347d5ace1c4f93fdda86cac9fe",
            "the manifest digest drifted from the server's"
        );
    }

    /// Y las tres cosas que lo componen cuentan: el orden, el tamaño y la ruta.
    /// Un digest que ignorase cualquiera de ellas podría dar por "ya subido" un
    /// contenido que no está arriba, que es la única forma en que este chequeo
    /// puede perder datos.
    #[test]
    fn manifest_digest_is_sensitive_to_order_size_and_path() {
        let sha_a = "11".repeat(32);
        let sha_b = "22".repeat(32);
        let base =
            manifest_digest([("a", sha_a.as_str(), 1i64), ("b", sha_b.as_str(), 2i64)].into_iter());
        let swapped =
            manifest_digest([("b", sha_b.as_str(), 2i64), ("a", sha_a.as_str(), 1i64)].into_iter());
        let resized =
            manifest_digest([("a", sha_a.as_str(), 9i64), ("b", sha_b.as_str(), 2i64)].into_iter());
        let renamed = manifest_digest(
            [("a2", sha_a.as_str(), 1i64), ("b", sha_b.as_str(), 2i64)].into_iter(),
        );
        assert_ne!(base, swapped);
        assert_ne!(base, resized);
        assert_ne!(base, renamed);
    }

    #[tokio::test]
    async fn content_signature_ignores_mtime_but_tracks_bytes() {
        let dir = std::env::temp_dir().join(format!("hoard-sig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.dat");
        std::fs::write(&path, b"hello world").unwrap();
        let mk = |mtime: u64| UploadFile {
            relative_path: "save.dat".to_string(),
            absolute_path: path.clone(),
            size_bytes: 11,
            modified: Some(UNIX_EPOCH + std::time::Duration::from_secs(mtime)),
        };
        // Cheap signature drifts with mtime, content signature does not.
        let a = vec![mk(100)];
        let b = vec![mk(999)];
        assert_ne!(compute_set_signature(&a), compute_set_signature(&b));
        assert_eq!(
            compute_content_signature(&a).await,
            compute_content_signature(&b).await
        );
        // Changing the bytes does move the content signature.
        let before = compute_content_signature(&a).await;
        std::fs::write(&path, b"hello WORLD").unwrap();
        let after = compute_content_signature(&a).await;
        assert_ne!(before, after);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **El bug**: un solo fichero ilegible tumbaba la firma de contenido, y con
    /// ella el snapshot entero. El recorrido ya saltaba lo que no podía
    /// interrogar; la lectura propagaba el error con `?`. Un placeholder de
    /// OneDrive dentro de un save de GTA San Andreas Definitive bastó para 3.934
    /// intentos en 13 días sin subir una sola versión.
    #[cfg(unix)]
    #[tokio::test]
    async fn one_unreadable_file_does_not_kill_the_signature() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("good.sav"), b"real save").unwrap();
        let bad = root.join("bad.sav");
        std::fs::write(&bad, b"placeholder").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let files = walk_source(root, &[]).unwrap();
        assert_eq!(
            files.len(),
            2,
            "el walk sí ve el fichero: puede hacerle stat"
        );
        let sig = compute_content_signature(&files).await;
        // Y es estable mientras siga ilegible: si no lo fuera, cada pasada
        // vería un cambio y volveríamos al bucle de subidas.
        assert_eq!(sig, compute_content_signature(&files).await);

        // Los bytes del legible sí cuentan.
        std::fs::write(root.join("good.sav"), b"moved on").unwrap();
        let moved = walk_source(root, &[]).unwrap();
        assert_ne!(sig, compute_content_signature(&moved).await);

        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    /// Un ilegible no debe confundirse con un vacío: si el marcador no entrase
    /// en el digest, "no se deja leer" y "está vacío" firmarían igual y una
    /// carpeta que recupera el acceso a un fichero vacío no se re-subiría.
    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_and_empty_do_not_sign_the_same() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let f = root.join("a.sav");
        std::fs::write(&f, b"").unwrap();
        let as_empty = compute_content_signature(&walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
        let as_unreadable = compute_content_signature(&walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_ne!(as_empty, as_unreadable);
    }

    /// El filtro de la subida: lo ilegible sale de la lista y se reporta, y lo
    /// demás viaja. Es lo que impide que el fichero reaparezca dentro del tar
    /// del camino de nube o del hasheo del CAS.
    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_files_are_split_off_and_reported() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("a.sav"), b"one").unwrap();
        std::fs::write(root.join("c.sav"), b"three").unwrap();
        let bad = root.join("b.sav");
        std::fs::write(&bad, b"two").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let (ok, skipped) = split_unreadable(walk_source(root, &[]).unwrap()).await;
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(
            ok.iter()
                .map(|f| f.relative_path.as_str())
                .collect::<Vec<_>>(),
            ["a.sav", "c.sav"],
            "el orden por ruta se conserva: el digest del manifiesto depende de él"
        );
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].relative_path, "b.sav");
        assert!(
            !skipped[0].error.is_empty(),
            "el error del sistema es lo único accionable que ve el usuario"
        );
    }

    /// Una subcarpeta sin permiso no puede tumbar el backup del juego entero:
    /// en Windows las junctions legacy del perfil devuelven acceso denegado de
    /// forma rutinaria.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_subdir_is_skipped_not_fatal() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("save.dat"), b"real save").unwrap();
        let locked = root.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("inner.dat"), b"x").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let files = walk_source(root, &[]).expect("un subdir ilegible no debe abortar el walk");
        // Restaura permisos para que el tempdir se pueda limpiar.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            files
                .iter()
                .map(|f| f.relative_path.as_str())
                .collect::<Vec<_>>(),
            ["save.dat"],
            "el save legible debe seguir estando"
        );
    }

    /// La raíz sí es error duro: si no se puede leer, no hay backup que hacer
    /// y decirlo en voz alta es lo correcto.
    #[test]
    fn an_unreadable_root_is_still_an_error() {
        let missing = std::path::Path::new("/definitely/not/here/hoard-test");
        assert!(walk_source(missing, &[]).is_err());
    }

    /// Save de fichero suelto: 4.900 juegos del catálogo sólo tienen
    /// plantillas que apuntan a un fichero. El snapshot sale con la misma
    /// forma que el de una carpeta con un fichero dentro, así que firma,
    /// dedup y restore siguen funcionando sin cambios.
    /// La guarda del camino del backup: una raíz imposible se rechaza **antes**
    /// de recorrer nada. Es el caso de la Steam Deck de ago-2026 — el save
    /// apuntaba al prefijo de Proton entero (308 MB) y la guarda estructural
    /// sólo corría al dar de alta, así que esa fila no volvía a pasar por ella.
    ///
    /// Se comprueba con una carpeta REAL y poblada: si el rechazo dependiera de
    /// que la ruta no exista o esté vacía, este test pasaría por el motivo
    /// equivocado.
    #[tokio::test]
    async fn a_whole_proton_prefix_is_refused_before_walking_it() {
        let tmp = tempfile::tempdir().unwrap();
        // …/compatdata/423230/pfx con contenido dentro, como el real.
        let save_dir = tmp.path().join(
            "steamapps/compatdata/423230/pfx/drive_c/users/steamuser/AppData/LocalLow/TheGameBakers/Furi",
        );
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("algo.dat"), b"x").unwrap();
        let prefix_root = tmp.path().join("steamapps/compatdata/423230/pfx");

        let client = crate::api::ApiClient::new("http://127.0.0.1:1", "t").unwrap();
        let err = upload_directory_checked(
            &client,
            "save-1",
            "furi",
            "main",
            &prefix_root,
            None,
            None,
            None,
            VersionOrigin::Automatic,
            |_, _| {},
            || {},
        )
        .await
        .expect_err("un prefijo entero no puede subirse");

        let unsafe_src = err
            .chain()
            .find_map(|c| c.downcast_ref::<UnsafeSource>())
            .expect("debe ser UnsafeSource, no un error de red ni de walk");
        assert!(
            unsafe_src.reason.to_lowercase().contains("prefix"),
            "el motivo debe nombrar el prefijo: {}",
            unsafe_src.reason
        );

        // Y la carpeta buena DENTRO del prefijo no se rechaza aquí: fallará más
        // adelante por no poder hablar con el servidor, que es otra cosa. Ojo
        // al elegirla: `…/users/steamuser` NO vale como control, porque es un
        // perfil entero y la guarda lo rechaza con toda la razón (las reglas de
        // Windows se reusan dentro del prefijo). Tiene que ser la carpeta del
        // juego de verdad.
        let err = upload_directory_checked(
            &client,
            "save-1",
            "furi",
            "main",
            &save_dir,
            None,
            None,
            None,
            VersionOrigin::Automatic,
            |_, _| {},
            || {},
        )
        .await
        .expect_err("sin servidor, falla igual");
        assert!(
            !err.chain().any(|c| c.is::<UnsafeSource>()),
            "la carpeta de dentro no puede rechazarse por forma: {err:#}"
        );
    }

    /// La carpeta real de Cell to Singularity: partidas y telemetría de Unity
    /// mezcladas. Antes de esto el snapshot se llevaba las dos cosas, y el
    /// `Player.log` —reescrito en cada arranque— cortaba una versión nueva en
    /// la nube cada vez que el usuario abría el juego.
    #[test]
    fn the_walk_leaves_engine_telemetry_out_of_the_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (rel, body) in [
            ("savedGames.gd", "partida"),
            ("savedGames2.gd", "partida"),
            ("Player.log", "log"),
            ("Player-prev.log", "log"),
            ("steam_autocloud.vdf", "vdf"),
        ] {
            std::fs::write(root.join(rel), body).unwrap();
        }
        let analytics = root.join("Unity/0a8833bc-a8ad/Analytics");
        std::fs::create_dir_all(&analytics).unwrap();
        std::fs::write(analytics.join("values"), "telemetría").unwrap();

        let files = walk_source(root, &[]).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(names, vec!["savedGames.gd", "savedGames2.gd"], "{names:?}");
    }

    /// La config **sí** sube: perderla no es una opción, y la protección contra
    /// el crash está en el restore, no aquí.
    #[test]
    fn config_still_goes_up() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("slot1.sav"), "partida").unwrap();
        std::fs::write(root.join("graphics.ini"), "res=1920x1080").unwrap();

        let files = walk_source(root, &[]).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(names, vec!["graphics.ini", "slot1.sav"], "{names:?}");
    }

    /// El log que reescribe el juego en cada arranque ya no mueve la firma, así
    /// que deja de cortar una versión por sesión.
    #[test]
    fn a_rewritten_engine_log_no_longer_drifts_the_signature() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("slot1.sav"), "partida").unwrap();
        std::fs::write(root.join("Player.log"), "arranque 1").unwrap();
        let before = compute_set_signature(&walk_source(root, &[]).unwrap());

        std::fs::write(root.join("Player.log"), "arranque 2, más largo").unwrap();
        let after = compute_set_signature(&walk_source(root, &[]).unwrap());
        assert_eq!(before, after, "el log no debe mover la firma");

        // Y la partida sí la mueve, que es lo que tiene que seguir pasando.
        std::fs::write(root.join("slot1.sav"), "partida avanzada").unwrap();
        assert_ne!(
            before,
            compute_set_signature(&walk_source(root, &[]).unwrap())
        );
    }

    /// El blindaje del manifiesto rescata lo que las reglas por nombre se
    /// llevarían: `.log` es el patrón de save de 64 plantillas del catálogo.
    #[test]
    fn a_manifest_pattern_rescues_a_file_the_rules_would_drop() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("player.log"), "esto sí es la partida").unwrap();

        assert!(walk_source(root, &[]).unwrap().is_empty());
        let shielded = walk_source(root, &["*.log".to_string()]).unwrap();
        assert_eq!(shielded.len(), 1);
    }

    /// Tracking the folder that holds one subfolder per save takes every save
    /// under it, new ones included — the whole point of pointing Hoard at the
    /// parent instead of filing each save by hand. Detection is what used to
    /// stand in the way (see `detection::is_nest_of_save_dirs`); the walk never
    /// did, and this pins that down so no future depth cap quietly breaks it.
    #[test]
    fn tracking_the_parent_takes_every_save_folder_under_it() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("Cyberpunk 2077");
        for slot in ["AutoSave-0", "ManualSave-0", "QuickSave-0"] {
            let dir = game.join(slot);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("sav.dat"), b"save").unwrap();
            std::fs::write(dir.join("metadata.9.json"), b"{}").unwrap();
        }

        let files = walk_source(&game, &[]).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "AutoSave-0/metadata.9.json",
                "AutoSave-0/sav.dat",
                "ManualSave-0/metadata.9.json",
                "ManualSave-0/sav.dat",
                "QuickSave-0/metadata.9.json",
                "QuickSave-0/sav.dat",
            ]
        );
    }

    /// Un save de fichero suelto se sube aunque su nombre parezca config: el
    /// usuario apuntó a ese fichero, y eso pesa más que cualquier regla.
    #[test]
    fn a_single_file_save_is_never_filtered_out() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.ini");
        std::fs::write(&file, "en realidad es la partida").unwrap();
        let files = walk_source(&file, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "settings.ini");
    }

    #[test]
    fn a_single_file_save_walks_to_one_entry_named_after_it() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("ssr_save.bin");
        std::fs::write(&file, b"0123456789").unwrap();

        let files = walk_source(&file, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "ssr_save.bin");
        assert_eq!(files[0].absolute_path, file);
        assert_eq!(files[0].size_bytes, 10);
        assert!(files[0].modified.is_some());
    }

    /// Y su firma se comporta como cualquier otra: cambia con el contenido,
    /// que es lo que hace que el skip-by-set-hash siga siendo correcto.
    #[test]
    fn a_single_file_saves_signature_tracks_its_content() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("save.dat");
        std::fs::write(&file, b"a").unwrap();
        let before = compute_set_signature(&walk_source(&file, &[]).unwrap());
        // Un tamaño distinto mueve la firma aunque el mtime tenga poca
        // resolución en este sistema de ficheros.
        std::fs::write(&file, b"bbbb").unwrap();
        let after = compute_set_signature(&walk_source(&file, &[]).unwrap());
        assert_ne!(before, after);
    }
}

#[cfg(test)]
mod paced_upload_tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    fn paced_error() -> anyhow::Error {
        anyhow!(ApiError::RateLimited {
            kind: RateLimitKind::Paced,
            retry_after_seconds: 0,
            body: "Too Many Requests! Wait for 0s".into(),
        })
        .context("uploading AutoSave-6/sav.dat")
    }

    fn budget_error() -> anyhow::Error {
        anyhow!(ApiError::RateLimited {
            kind: RateLimitKind::Budget,
            retry_after_seconds: 420,
            body: r#"{"code":"bandwidth_limit","retry_after_seconds":420}"#.into(),
        })
        .context("uploading AutoSave-6/sav.dat")
    }

    /// The bug this whole path exists for: a save with more blobs than the
    /// server's burst allows used to lose every blob it had already uploaded
    /// the moment the pacer turned one away.
    #[tokio::test(start_paused = true)]
    async fn a_paced_blob_is_retried_rather_than_dropped() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            match calls.fetch_add(1, Ordering::Relaxed) {
                0 | 1 => Err(paced_error()),
                _ => Ok(()),
            }
        })
        .await;
        assert!(r.is_ok(), "{:?}", r.err());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
    }

    /// A budget is the opposite instruction: the operation doesn't fit right
    /// now, and re-sending the same PUT can only make it worse. It has to
    /// travel up untouched so the agent parks the save and comes back later.
    #[tokio::test(start_paused = true)]
    async fn a_budget_429_is_not_retried() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(budget_error())
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let api = r.unwrap_err();
        let api = api.chain().find_map(|c| c.downcast_ref::<ApiError>());
        assert!(matches!(
            api,
            Some(ApiError::RateLimited {
                kind: RateLimitKind::Budget,
                ..
            })
        ));
    }

    /// A server whose limit is simply too tight for this save has to fail
    /// loudly and quickly. Crawling for an hour while pretending to work is
    /// worse than a clear error the operator can act on.
    #[tokio::test(start_paused = true)]
    async fn a_pacer_that_never_relents_gives_up() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(paced_error())
        })
        .await;
        assert!(r.is_err());
        assert_eq!(
            calls.load(Ordering::Relaxed),
            MAX_PACED_RETRIES_PER_BLOB + 1
        );
        assert!(format!("{:#}", r.unwrap_err()).contains("too tight"));
    }

    /// Anything that isn't a pacer keeps its old behaviour: straight up, no
    /// retry. A blob whose bytes stopped matching its sha must not be re-sent.
    #[tokio::test(start_paused = true)]
    async fn a_real_failure_is_not_retried() {
        let calls = AtomicU32::new(0);
        let waited = AtomicU64::new(0);
        let r = put_blob_paced("AutoSave-6/sav.dat", &waited, || async {
            calls.fetch_add(1, Ordering::Relaxed);
            Err::<(), _>(anyhow!("the game rotated the save while it was uploading"))
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
