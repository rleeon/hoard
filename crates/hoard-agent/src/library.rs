//! Lógica de biblioteca/tracking COMPARTIDA por desktop y CLI (paridad
//! CLI↔desktop). Aquí vive el negocio —mutar `CliState`, hablar con el server,
//! construir la lista— devolviendo DATOS; cada frontend solo pinta el resultado
//! y hace el disparo propio (attach/detach al agente vivo en desktop, restart
//! del daemon en CLI). Antes estaba atrapado en `hoard-desktop/commands/`, con
//! la CLI reimplementando un trozo en `track.rs` y el daemon copiando el hydrate.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use hoard_core::kernel::slots;
use hoard_manifest::ludusavi;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::agent::{dir_size_bytes, WatchedSave};
use crate::api::ApiClient;
use crate::config::CliConfig;
use crate::detection::{Confidence, DetectedGame, DetectionReport};
use crate::junkdirs;
use crate::manifest::Os;
use crate::presets::{self, SavePolicy};
use crate::state::{CliState, SaveState};
use crate::{launchers, playtime_catalog, steam};

// ---- tipos de wire compartidos ---------------------------------------------

/// Una fila de la lista "Juegos monitorizados". Idéntico wire para el desktop
/// (Tauri) y la CLI (impresión).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedSave {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    /// What the user calls this folder, carried in the label next to the
    /// number. `None` = never named; the UI shows just the number.
    #[serde(default)]
    pub name: Option<String>,
    /// Which slot of the title this folder is, derived from `label`
    /// ([`slots::slot_of`]). `None` = one of the older free-form labels, which
    /// renders with its text as-is.
    ///
    /// Computed here rather than in the frontend so the equivalences (`"main"`
    /// and `"default"` are slot 1) have a single owner; the CLI and the desktop
    /// render the same list.
    #[serde(default)]
    pub slot: Option<u32>,
    pub local_path: String,
    /// Cabeza del **server**: la última versión que existe en la nube (o en el
    /// server self-hosted), venga de donde venga — normalmente de otra máquina.
    ///
    /// Se llamaba `last_version_num`, un nombre que invitaba a leerlo como "la
    /// versión que tengo". El panel lo rotulaba «Guardado (v138)» con este
    /// equipo anclado en la v120 y el poller muerto (ADR 0021 D.10): en una
    /// herramienta de saves eso invita a jugar encima creyendo estar al día, y
    /// esa partida subiría como v139 haciendo retroceder la cabeza de la nube.
    /// El par con [`Self::local_version_num`] es lo que impide volver a
    /// confundirlos.
    pub cloud_version_num: Option<i64>,
    /// Versión a la que está sincronizado **este equipo** (el cursor
    /// `SaveState::last_version_num` del `CliState` local, que es lo que el
    /// kernel usa como `known_version`). `None` = esta máquina nunca subió ni
    /// bajó este save: existe en la nube pero no aquí.
    #[serde(default)]
    pub local_version_num: Option<i64>,
    pub last_backup_at: Option<String>,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub total_size_bytes: i64,
    /// `true` cuando el save existe en el server pero esta máquina no tiene fila
    /// `CliState` (reinstalación, cambio de PC, state borrado). El frontend lo
    /// marca "Sin estado local".
    #[serde(default)]
    pub orphan: bool,
    /// Bytes que ocupa el save EN ESTA máquina (tamaño de su carpeta local).
    /// `None` para huérfanos y filas recién creadas.
    #[serde(default)]
    pub local_size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// El "sí" del usuario a que se le escriba la config de este juego al
    /// restaurar. Ver [`crate::state::SaveState::allow_device_local`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_device_local: Option<bool>,
}

/// Args para [`add_to_tracking`].
#[derive(Debug, Clone, Deserialize)]
pub struct AddGameArgs {
    pub game_slug: String,
    pub label: Option<String>,
    /// Which slot of the title this folder goes in (see
    /// [`hoard_core::kernel::slots`]). Wins over `label`, which is derived from
    /// it.
    ///
    /// `None` = slot 1, the saved games. That is what an ordinary add means, and
    /// what the code did before slots existed.
    #[serde(default)]
    pub slot: Option<u32>,
    pub local_path: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub steam_app_id: Option<i64>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub processes: Option<Vec<String>>,
    /// Los procesos de este save los comparten otros saves rastreados, así que
    /// verlos correr no dice que se esté jugando a ÉSTE. Lo marca el alta de
    /// una consola emulada partida por juego. Ver
    /// [`crate::state::SaveState::shared_processes`].
    #[serde(default)]
    pub shared_processes: bool,
    /// What the user calls this folder ("Mods", "Ironman"). Goes into the label
    /// next to the number; `None` leaves the slot unnamed. See
    /// [`hoard_core::kernel::slots`].
    #[serde(default)]
    pub name: Option<String>,
    /// The user has already agreed to **move** this slot to another folder.
    /// Without it, finding the slot held by a different folder is an error (see
    /// [`occupied_slot`]) instead of a silent overwrite.
    #[serde(default)]
    pub repoint: bool,
}

/// Args para [`adopt`].
#[derive(Debug, Clone, Deserialize)]
pub struct AdoptArgs {
    pub save_id: String,
    pub game_slug: String,
    pub label: String,
    pub local_path: String,
}

/// Resultado de añadir/adoptar: la fila para pintar y el `WatchedSave` que el
/// frontend debe enganchar al agente vivo (o ignorar si no corre).
pub struct TrackOutcome {
    pub tracked: TrackedSave,
    pub watched: WatchedSave,
}

// ---- caché de detección en disco (compartida) ------------------------------

/// Snapshot de detección persistido junto a `state.json` para pintar la
/// biblioteca al instante en frío sin re-escanear.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDetection {
    pub report: DetectionReport,
    #[serde(with = "time::serde::rfc3339")]
    pub scanned_at: OffsetDateTime,
}

/// Ruta del caché de detección (mismo dir que `state.json`).
pub fn detection_cache_path() -> Result<PathBuf> {
    Ok(CliConfig::state_dir()?.join("detection.json"))
}

/// Carga el caché de disco. `None` si falta, está corrupto o es ilegible
/// (se degrada a arranque en frío en vez de crashear).
pub fn load_detection_from_disk() -> Option<CachedDetection> {
    let path = match detection_cache_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "couldn't resolve detection cache path");
            return None;
        }
    };
    if !path.exists() {
        return None;
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<CachedDetection>(&text) {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "detection cache malformed; ignoring");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "couldn't read detection cache");
            None
        }
    }
}

/// Escribe el caché atómicamente: serializa → tmp → `fs::rename`.
pub fn save_detection_to_disk_atomic(cached: &CachedDetection) -> Result<()> {
    let path = detection_cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cached)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

// ---- qué sabe la detección local de UN slug --------------------------------

/// Una ruta de save que la detección encontró en ESTA máquina, con la
/// confianza de esa ruta concreta (no la rolled-up del juego).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedPath {
    pub path: PathBuf,
    pub confidence: Confidence,
}

/// Lo que la detección local sabe de un slug, para vincular un save del cloud
/// a esta máquina sin obligar al usuario a buscar la carpeta a mano.
///
/// `scanned_at` es `None` cuando no hay caché de detección. Distinguirlo de
/// `paths` vacío es lo que deja al frontend ofrecer un escaneo en vez de
/// afirmar "no hay nada": el usuario que nunca activó el Modo Automático llega
/// aquí con la caché fría, y una lista vacía sin más sería mentira.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDetection {
    pub game_slug: String,
    /// Candidatas ordenadas strongest-first (mismo orden que `found_paths`).
    pub paths: Vec<DetectedPath>,
    /// Los **demás** juegos detectados aquí, para vincular por juego cuando el
    /// slug de la nube no casa con ninguno local. Ver [`link_candidates`].
    #[serde(default)]
    pub candidates: Vec<LinkCandidate>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub scanned_at: Option<OffsetDateTime>,
}

/// Un juego detectado en ESTA máquina ofrecido como destino de vínculo.
///
/// El emparejamiento por slug exacto ([`detected_paths_in`]) se rompe en cuanto
/// las dos máquinas nombran el juego distinto —la misma copia instalada por
/// caminos distintos, un Steam contra un suelto—, y entonces el usuario se
/// quedaba con el selector de carpetas como única salida: cazar a mano una ruta
/// que Hoard ya conoce. Estas son las candidatas para decir "es este juego" en
/// vez de "es esta carpeta".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCandidate {
    pub game_slug: String,
    pub display_name: String,
    /// Rutas de save del juego, strongest-first. Nunca vacío: un juego sin
    /// carpeta que ofrecer no es candidato.
    pub paths: Vec<DetectedPath>,
    /// Parecido entre el nombre del juego local y el slug que viene de la nube:
    /// `2` mismo nombre normalizado, `1` uno contiene al otro, `0` nada. Ordena
    /// la lista y deja al frontend destacar lo que casi seguro es el mismo
    /// juego.
    pub affinity: u8,
}

impl LocalDetection {
    /// La ruta a vincular cuando la detección es **no ambigua**: exactamente
    /// una candidata. Con dos o más el usuario tiene que elegir, y con cero no
    /// hay nada que ofrecer.
    pub fn unambiguous(&self) -> Option<&DetectedPath> {
        match self.paths.as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }
}

/// Rutas de save detectadas para `game_slug` dentro de un report ya calculado.
///
/// Solo rutas de save: `found_paths` nunca contiene el directorio de instalación
/// (eso es `install_dir`, y hacer backup del binario del juego sería un bug).
pub fn detected_paths_in(report: &DetectionReport, game_slug: &str) -> Vec<DetectedPath> {
    let Some(game) = report.games.iter().find(|g| g.slug == game_slug) else {
        return Vec::new();
    };
    game.found_paths
        .iter()
        .enumerate()
        .map(|(i, path)| DetectedPath {
            path: path.clone(),
            // `path_confidences` es `default` — una caché escrita por un build
            // viejo la trae vacía. Caer a la confianza del juego conserva la
            // ruta en vez de perderla por un campo ausente.
            confidence: game
                .path_confidences
                .get(i)
                .copied()
                .unwrap_or(game.confidence),
        })
        .collect()
}

/// Normaliza un nombre o un slug a solo alfanuméricos en minúscula, que es lo
/// único comparable entre "R.E.P.O.", "repo" y "R E P O".
fn normalized_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Parecido entre el slug de la nube y un juego local. Ver [`LinkCandidate::affinity`].
///
/// Se mira contra el nombre visible **y** contra el slug: el mismo juego puede
/// llegar como `raccoin` desde una máquina y `Raccoin` / `rac-coin` desde otra,
/// y los tres normalizan igual. La contención pide 4 caracteres para que un
/// nombre corto no se declare pariente de media biblioteca ("Ori" dentro de
/// "Origin", "GTA" dentro de cualquier cosa con esas letras seguidas).
fn name_affinity(cloud_slug: &str, game: &DetectedGame) -> u8 {
    let cloud = normalized_name(cloud_slug);
    if cloud.is_empty() {
        return 0;
    }
    let mut best = 0;
    for local in [
        normalized_name(&game.display_name),
        normalized_name(&game.slug),
    ] {
        if local.is_empty() {
            continue;
        }
        let score = if local == cloud {
            2
        } else if cloud.len() >= 4
            && local.len() >= 4
            && (local.contains(&cloud) || cloud.contains(&local))
        {
            1
        } else {
            0
        };
        best = best.max(score);
    }
    best
}

/// Los juegos detectados aquí a los que se puede enganchar el save `game_slug`
/// que vive en la nube, mejor parecido primero.
///
/// Fuera quedan tres grupos, y por motivos distintos:
///
/// * El propio `game_slug`, que ya va en [`LocalDetection::paths`] y saldría
///   duplicado.
/// * Los juegos sin ninguna ruta de save encontrada: no hay carpeta que
///   vincular, solo un nombre.
/// * Los que apuntan a una carpeta **ya rastreada** por otro save. Vincular ahí
///   pondría dos saves distintos a hacer backup de la misma carpeta, que es
///   justo lo que el escaneo automático evita con `paths_overlap`; ofrecerlo en
///   un desplegable no lo haría menos roto.
pub fn link_candidates(
    report: &DetectionReport,
    game_slug: &str,
    tracked_paths: &[PathBuf],
) -> Vec<LinkCandidate> {
    let mut out: Vec<LinkCandidate> = report
        .games
        .iter()
        .filter(|g| g.slug != game_slug && !g.found_paths.is_empty())
        .filter(|g| {
            !tracked_paths
                .iter()
                .any(|t| crate::detection::paths_overlap(&g.found_paths[0], t))
        })
        .map(|g| LinkCandidate {
            game_slug: g.slug.clone(),
            display_name: g.display_name.clone(),
            paths: detected_paths_in(report, &g.slug),
            affinity: name_affinity(game_slug, g),
        })
        .collect();
    // Parecido primero (lo que el usuario venía buscando), y el resto por
    // nombre: la lista larga se recorre con el ojo, no con la barra de scroll.
    out.sort_by(|a, b| {
        b.affinity.cmp(&a.affinity).then_with(|| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        })
    });
    out
}

/// Lo que la detección local sabe de `game_slug` según una caché ya cargada.
/// `cached` es `None` cuando nadie ha escaneado todavía en esta máquina.
///
/// El desktop pasa su caché en memoria (`AppState`) y la CLI la de disco
/// ([`load_detection_from_disk`]); la regla de qué es una candidata vive aquí,
/// una sola vez.
///
/// `tracked_paths` son las carpetas que esta máquina ya rastrea, para no
/// ofrecer como destino una carpeta que ya tiene dueño ([`link_candidates`]).
pub fn local_detection(
    cached: Option<&CachedDetection>,
    game_slug: &str,
    tracked_paths: &[PathBuf],
) -> LocalDetection {
    LocalDetection {
        game_slug: game_slug.to_string(),
        paths: cached
            .map(|c| detected_paths_in(&c.report, game_slug))
            .unwrap_or_default(),
        candidates: cached
            .map(|c| link_candidates(&c.report, game_slug, tracked_paths))
            .unwrap_or_default(),
        scanned_at: cached.map(|c| c.scanned_at),
    }
}

// ---- hydrate (UNIFICADO: antes duplicado desktop/daemon) --------------------

/// Match laxo entre el nombre de un juego de Steam ("Stardew Valley") y un slug
/// de Hoard ("stardew-valley").
pub fn name_matches(steam_name: &str, slug: &str) -> bool {
    let a: String = steam_name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let b: String = slug.chars().filter(|c| c.is_alphanumeric()).collect();
    !a.is_empty() && a == b
}

/// Política de sync efectiva: el preset fijado por el usuario gana; sin él, cae
/// al catálogo built-in (R.E.P.O. → short-session). Nombre desconocido ⇒ vacía.
///
/// The slot plays no part here on purpose. Slots 2+ briefly forced
/// `auto_restore = Some(false)`, which overrode the user's own preference and
/// left the second machine's folder empty while the first uploaded into it —
/// the opposite of what attaching several folders is for. Device-local files
/// are already held back per file by [`hoard_core::kernel::fileclass`]; a
/// per-slot rule on top of that only broke sync.
pub fn resolve_policy(game_slug: &str, stored_preset: Option<&str>) -> SavePolicy {
    let name = stored_preset.or_else(|| presets::builtin_preset_for(game_slug));
    SavePolicy::from_preset(name)
}

/// Nombres de proceso que marcan "jugando" para este slug.
///
/// Fuente principal: el bloque `launch:` del manifiesto Ludusavi, que trae el
/// ejecutable de ~18k juegos. Antes esto sólo devolvía el catálogo built-in —
/// dos entradas— y todo lo demás dependía del match por tokens del slug o de
/// una correlación que en frío vale cero; ahora la primera sesión de un juego
/// del catálogo ya dispara "arrancó" sin haberlo visto nunca.
///
/// **Sólo se aceptan ejecutables inequívocos**: `hoard_manifest` deja fuera del
/// índice los nombres que reclaman varios juegos (`game.exe`, `launcher.exe`,
/// `nw.exe`, `dosbox.exe`…), y aquí se exige además que el nombre resuelva de
/// vuelta a ESTE slug. Sin ese filtro, un `game.exe` cualquiera pondría a
/// jugar —y a acumular horas— a un juego al azar.
///
/// El catálogo built-in sigue teniendo la última palabra (Minecraft por
/// TLauncher no sale del manifiesto): se añade siempre, sin duplicar.
pub fn resolve_processes(game_slug: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(entry) = ludusavi::find_by_slug(game_slug) {
        for exe in &entry.launch_exes {
            let unambiguous = ludusavi::find_by_exe(exe).is_some_and(|e| e.slug == game_slug);
            if unambiguous && !out.iter().any(|p| p.eq_ignore_ascii_case(exe)) {
                out.push(exe.clone());
            }
        }
    }
    for p in presets::builtin_processes_for(game_slug) {
        if !out.iter().any(|x| x.eq_ignore_ascii_case(p)) {
            out.push((*p).to_string());
        }
    }
    out
}

// ---- carpetas descartadas por el usuario -----------------------------------

/// Descarta una carpeta: la detección deja de ofrecerla, y todo lo que cuelgue
/// de ella. Idempotente.
///
/// Es la respuesta al problema que ignorar-por-slug no resuelve: el nombre de
/// un hallazgo de fase 4 lo pone la correlación, y cambia entre escaneos, así
/// que la misma carpeta vuelve con slug nuevo una y otra vez. La ruta no
/// cambia.
pub fn exclude_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("Path can't be empty.");
    }
    let (mut state, file) = CliState::load_default()?;
    state.add_excluded_path(path.to_path_buf());
    state.save(&file)?;
    tracing::info!(path = %path.display(), "detection: folder excluded by the user");
    Ok(())
}

/// Deja de descartar exactamente esta carpeta. Mirror de [`exclude_path`].
pub fn unexclude_path(path: &Path) -> Result<()> {
    let (mut state, file) = CliState::load_default()?;
    state.remove_excluded_path(path);
    state.save(&file)?;
    Ok(())
}

/// Las carpetas descartadas en este equipo, para pintarlas en Ajustes.
pub fn list_excluded_paths() -> Result<Vec<PathBuf>> {
    Ok(CliState::load_default()?.0.excluded_paths)
}

/// Quita del informe las rutas que el usuario descartó, y con ellas los juegos
/// que se quedan sin ninguna.
///
/// La sutileza que importa: un juego **sin rutas desde el principio** NO se
/// toca. Esa fila es deliberada — significa "vi el juego en disco pero no sé
/// dónde guarda" y es la que pinta la alerta ámbar para que el usuario elija
/// carpeta. Borrarla le quitaría al usuario la única forma de arreglarlo.
/// Sólo desaparece el juego al que la exclusión le quitó TODAS las que tenía.
pub fn apply_excluded_paths(report: &mut DetectionReport, state: &CliState) {
    if state.excluded_paths.is_empty() {
        return;
    }
    report.games.retain_mut(|g| {
        let had = g.found_paths.len();
        g.found_paths.retain(|p| !state.is_path_excluded(p));
        g.path_confidences.truncate(g.found_paths.len());
        // Tenía rutas y ninguna sobrevivió ⇒ fuera. Nunca tuvo ⇒ se queda.
        had == 0 || !g.found_paths.is_empty()
    });
}

/// `save_id` sintético de un slot playtime-only. Prefijo anticolisión.
fn playtime_save_id(slug: &str) -> String {
    format!("playtime:{slug}")
}

/// Juegos instalados (cualquier launcher) que casan el catálogo de playtime,
/// por slug, con el dir de instalación. Primer match por slug gana.
fn installed_catalog_games(os: Os) -> Vec<(&'static str, Option<PathBuf>)> {
    let mut sources: Vec<(String, PathBuf)> = Vec::new();
    for app in steam::list_installed_steam_games(os).unwrap_or_default() {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_epic_games(os) {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_gog_games(os) {
        sources.push((app.name, app.install_dir));
    }
    for app in launchers::list_installed_msstore_games(os) {
        sources.push((app.name, app.install_dir));
    }
    let mut out: Vec<(&'static str, Option<PathBuf>)> = Vec::new();
    let mut seen: HashSet<&'static str> = HashSet::new();
    for (name, dir) in sources {
        if let Some(g) = playtime_catalog::game_for_store_name(&name) {
            if seen.insert(g.slug) {
                out.push((g.slug, Some(dir)));
            }
        }
    }
    out
}

/// Slot `track_only` para un slug del catálogo de playtime.
fn playtime_watched_save(slug: &str, install_dir: Option<PathBuf>) -> WatchedSave {
    let game = playtime_catalog::by_slug(slug);
    let display_name = game
        .map(|g| g.display_name.to_string())
        .unwrap_or_else(|| slug.to_string());
    let processes = game
        .map(|g| g.processes.iter().map(|p| p.to_string()).collect())
        .unwrap_or_default();
    WatchedSave {
        save_id: playtime_save_id(slug),
        game_slug: slug.to_string(),
        display_name,
        label: "playtime".to_string(),
        allow_device_local: None,
        local_path: PathBuf::new(),
        steam_install_dir: install_dir,
        processes,
        // Un slot de sólo-horas no comparte proceso con nadie: su lista viene
        // del catálogo de playtime, que es un juego por entrada.
        shared_processes: false,
        policy: SavePolicy::default(),
        known_version: None,
        set_hash: None,
        track_only: true,
    }
}

/// Slots `track_only` a sembrar: cada juego de catálogo instalado que no esté ya
/// rastreado como save real y que el usuario no haya excluido.
pub fn derive_playtime_saves(
    cli_state: &CliState,
    tracked_slugs: &HashSet<String>,
) -> Vec<WatchedSave> {
    installed_catalog_games(Os::current())
        .into_iter()
        .filter_map(|(slug, dir)| {
            if tracked_slugs.contains(slug) || cli_state.is_playtime_excluded(slug) {
                return None;
            }
            Some(playtime_watched_save(slug, dir))
        })
        .collect()
}

/// Comparison key for "the same folder on disk".
///
/// Windows is case-insensitive and the paths in `state.json` mix separators —
/// `C:\Users\x\Documents\My Games/Fallout4/Saves` is one real directory
/// written two ways — so comparing the strings would read two spellings of one
/// folder as two folders. Only normalised on Windows: a backslash is a legal
/// character in a Unix filename, and folding it there would merge folders that
/// really are different.
fn folder_key(p: &Path) -> String {
    let raw = p.to_string_lossy();
    let s = if cfg!(windows) {
        raw.replace('\\', "/").to_lowercase()
    } else {
        raw.into_owned()
    };
    match s.trim_end_matches('/') {
        "" => s,
        trimmed => trimmed.to_string(),
    }
}

/// The rows of `state.json` that deserve a watcher, one per folder.
///
/// Two rows can name the same directory under two save ids: the local one and
/// the id the server considers canonical for that `(slug, label)`. The upload
/// path already knows they are the same — it redirects the commit to the
/// canonical id rather than 404ing — but nothing upstream ever collapsed them,
/// so both got a watcher, both hashed the folder and both uploaded the same
/// bytes on every change. Seen ago-2026 on Jurassic World Evolution 3: two
/// watchers armed on one path 70 ms apart, 5.7 MB sent twice.
///
/// The key is (folder, slug, label), the tightest one that fixes that: a game
/// tracked in two different folders is a deliberate slot and stays, and two
/// different games sharing one folder stay too — collapsing those would stop
/// backing one of them up.
///
/// Which twin wins barely matters for the bytes, since the commit is redirected
/// either way; it matters for the work. A row carrying a set-hash knows what is
/// already synced, so it will not re-upload a baseline — that is the one to
/// keep. `save_id` breaks the remaining ties so a `HashMap`'s order cannot make
/// this pick a different winner on every pass.
fn rows_one_per_folder(cli_state: &CliState) -> Vec<(&String, &SaveState)> {
    let mut rows: Vec<(&String, &SaveState)> =
        cli_state.saves.iter().filter(|(_, s)| !s.paused).collect();
    rows.sort_by(|(a_id, a), (b_id, b)| {
        (a.set_hash.is_none(), a.last_version_num.is_none(), *a_id).cmp(&(
            b.set_hash.is_none(),
            b.last_version_num.is_none(),
            *b_id,
        ))
    });

    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut kept = Vec::with_capacity(rows.len());
    for (id, s) in rows {
        let key = (
            folder_key(&s.local_path),
            s.game_slug.clone(),
            s.label.clone(),
        );
        if seen.insert(key) {
            kept.push((id, s));
        } else {
            tracing::warn!(
                save_id = %id,
                game_slug = %s.game_slug,
                label = %s.label,
                path = %s.local_path.display(),
                "state: two rows name the same folder; watching it once"
            );
        }
    }
    kept
}

/// Construye la lista de vigilancia desde `state.json`: saves reales (enriquecidos
/// con su dir de Steam) + slots playtime-only. Salta los pausados y los
/// archivados. ÚNICA fuente: antes el desktop (`hydrate_watched_saves`) y el
/// daemon (`watched_from_state`) tenían dos copias que ya divergían.
///
/// `archived` are the save ids parked in the server-side black box. A frozen
/// save is refused at upload (403) by design, so watching one buys nothing and
/// costs a full re-hash of its folder on every reconcile plus a "Backing up…"
/// in the feed for work that was never going to happen. Treated exactly like
/// `paused`: not watched at all. An empty set is the honest answer when the
/// server could not be asked — never watch less because a request failed.
pub fn watched_saves_from_state(
    cli_state: &CliState,
    archived: &HashSet<String>,
) -> Vec<WatchedSave> {
    // Cachea los juegos de Steam una vez (no reescanear `.acf` por save).
    let steam_apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();

    let tracked_slugs: HashSet<String> = cli_state
        .saves
        .values()
        .map(|s| s.game_slug.clone())
        .collect();
    let playtime_saves = derive_playtime_saves(cli_state, &tracked_slugs);

    let rows = rows_one_per_folder(cli_state);
    let mut out = Vec::with_capacity(rows.len() + playtime_saves.len());
    for (save_id, s) in rows {
        if archived.contains(save_id) {
            tracing::info!(
                save_id = %save_id,
                game_slug = %s.game_slug,
                "state: save is archived on the server; not watching it"
            );
            continue;
        }
        let steam_install_dir = steam_apps
            .iter()
            .find(|a| name_matches(&a.name, &s.game_slug))
            .map(|a| a.install_dir.clone());
        let processes = if s.processes.is_empty() {
            resolve_processes(&s.game_slug)
        } else {
            s.processes.clone()
        };
        out.push(WatchedSave {
            save_id: save_id.clone(),
            game_slug: s.game_slug.clone(),
            // No guardamos display_name en state.json; el slug hace de stand-in.
            display_name: s.game_slug.clone(),
            label: s.label.clone(),
            local_path: s.local_path.clone(),
            steam_install_dir,
            processes,
            shared_processes: s.shared_processes,
            policy: resolve_policy(&s.game_slug, s.preset.as_deref()),
            allow_device_local: s.allow_device_local,
            known_version: s.last_version_num,
            set_hash: s.set_hash.clone(),
            track_only: false,
        });
    }
    out.extend(playtime_saves);
    out
}

/// `WatchedSave` para un save recién añadido/renombrado, desde inputs mínimos.
/// Resuelve dir de Steam, política y procesos igual que el hydrate.
#[allow(clippy::too_many_arguments)]
pub fn watched_save_from(
    save_id: String,
    game_slug: String,
    display_name: String,
    label: String,
    local_path: PathBuf,
    preset: Option<&str>,
    processes_override: Vec<String>,
    shared_processes: bool,
    allow_device_local: Option<bool>,
) -> WatchedSave {
    let steam_apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();
    let steam_install_dir = steam_apps
        .iter()
        .find(|a| name_matches(&a.name, &game_slug))
        .map(|a| a.install_dir.clone());
    let processes = if processes_override.is_empty() {
        resolve_processes(&game_slug)
    } else {
        processes_override
    };
    let policy = resolve_policy(&game_slug, preset);
    WatchedSave {
        allow_device_local,
        save_id,
        game_slug: game_slug.clone(),
        display_name,
        label,
        local_path,
        steam_install_dir,
        processes,
        shared_processes,
        policy,
        known_version: None,
        set_hash: None,
        track_only: false,
    }
}

// ---- add / adopt / list / rename / untrack / delete ------------------------

/// Comprobaciones de FORMA de una ruta de save, sin exigir que exista.
///
/// Se aplican también al destino de un restore, donde la carpeta legítimamente
/// puede no existir todavía (máquina nueva). Rechazan lo que no puede ser
/// nunca la carpeta de un juego: un perfil entero, una raíz de sistema, o el
/// propio directorio de estado de Hoard —cuyo backup se copiaría a sí mismo
/// en bucle—.
pub fn validate_path_shape(local_path: &Path) -> Result<()> {
    if local_path.as_os_str().is_empty() {
        anyhow::bail!("Save folder path can't be empty.");
    }
    if let Some(reason) = junkdirs::dangerous_sync_root(local_path) {
        anyhow::bail!(
            "Refusing to use {}: {reason}. Pick the game's own save folder inside it.",
            local_path.display()
        );
    }
    if let Ok(state_dir) = CliConfig::state_dir() {
        if local_path.starts_with(&state_dir) || state_dir.starts_with(local_path) {
            anyhow::bail!(
                "Refusing to use {}: that's Hoard's own data folder.",
                local_path.display()
            );
        }
    }
    Ok(())
}

/// Igual que [`validate_path_shape`] y además: la carpeta tiene que existir
/// (se crea si falta) y no puede estar ya rastreada por otro save.
///
/// Lo segundo evita dos watchers y dos historiales sobre los mismos bytes. El
/// escaneo automático ya lo comprobaba por su cuenta, pero un alta manual —o
/// la CLI— podían duplicar igual.
fn validate_folder(local_path: &Path, except_save_ids: &[&str]) -> Result<()> {
    validate_path_shape(local_path)?;
    if !local_path.exists() {
        // No existe todavía: se asume carpeta. Un save de fichero suelto
        // siempre se da de alta sobre un fichero que YA está (lo propone la
        // detección al encontrarlo), así que aquí no hay ambigüedad.
        std::fs::create_dir_all(local_path)
            .with_context(|| format!("Couldn't create {}", local_path.display()))?;
    } else if !local_path.is_dir() && !local_path.is_file() {
        anyhow::bail!("{} isn't a folder or a file.", local_path.display());
    }
    if let Ok((state, _)) = CliState::load_default() {
        if let Some(other) = conflicting_save(&state, local_path, except_save_ids) {
            // A tracked folder INSIDE the one being added gets its own line.
            // It is what a game with one folder per save leaves behind — a row
            // per slot, tracked back when the parent was not on offer — and
            // "one folder, one game" alone reads like a flat refusal there
            // instead of the two-step it is: untrack the slots, add the folder
            // that holds them.
            if other.local_path != local_path
                && crate::detection::path_is_inside(&other.local_path, local_path)
            {
                anyhow::bail!(
                    "'{}' already tracks {}, which is inside this folder. \
                     Untrack it first, then add this one.",
                    other.game_slug,
                    other.local_path.display()
                );
            }
            anyhow::bail!(
                "'{}' already tracks {} — one folder, one game.",
                other.game_slug,
                other.local_path.display()
            );
        }
    }
    Ok(())
}

/// El save —si lo hay— que ya cubre `local_path` y no es ninguno de los que se
/// están reapuntando. Puro para poder testearlo: `validate_folder` es un
/// envoltorio de IO (crea la carpeta, lee el state del disco) y esta decisión es
/// la que se ha equivocado en producción.
///
/// `except_save_ids` son varios a propósito. Una adopción releva **dos**
/// entradas a la vez: el `save_id` que trae la nube y el que esta máquina se
/// mintó en local para el mismo (juego, etiqueta). Excluyendo sólo el primero,
/// la entrada local choca contra sí misma y el juego queda encallado.
/// La entrada que esta máquina ya tiene **para esta misma carpeta**, si la hay.
///
/// La identidad de un save rastreado es la carpeta, no el slug. El slug no es
/// estable entre fuentes: el mismo juego sale `vrising` por el appid de Steam y
/// `v-rising` por el catálogo, o `dispatch` y `dispatch-2025` según quién lo
/// nombre. Buscando por slug, dos nombres del mismo juego se tratan como dos
/// juegos y la regla de "una carpeta, un juego" los bloquea mutuamente — que es
/// exactamente lo que le pasó a un usuario en ago-2026 con tres juegos a la vez.
///
/// Comparación exacta de carpeta (insensible a caja en Windows), **no**
/// solapamiento: una carpeta anidada dentro de otra sí es el conflicto legítimo
/// que la regla debe seguir denunciando.
fn row_for_same_folder<'a>(state: &'a CliState, local_path: &Path) -> Option<&'a str> {
    state
        .saves
        .iter()
        .find(|(_, st)| same_folder(&st.local_path, local_path))
        .map(|(id, _)| id.as_str())
}

/// Misma carpeta, con la caja de Windows en cuenta (`C:\Users` y `c:\users` son
/// la misma). Espeja el criterio de [`crate::detection::paths_overlap`], que ya
/// baja a minúsculas en Windows por la misma razón.
///
/// Delega en [`folder_key`] para que no haya dos reglas de "misma carpeta" en
/// este fichero: ésta comparaba las cadenas tal cual y por tanto leía
/// `…\My Games/Fallout4/Saves` y `…\My Games\Fallout4\Saves` como dos sitios
/// distintos, que es una forma que `state.json` trae de verdad.
fn same_folder(a: &Path, b: &Path) -> bool {
    folder_key(a) == folder_key(b)
}

/// El juego —distinto de `slug`— que ya reclama `path`, si lo hay. `None` = el
/// override manual es legítimo.
///
/// Un override manual gana a la heurística **para siempre**: vive en
/// `device.json`, sobrevive a desinstalar con "borrar datos", a borrar el estado
/// de saves y a dejar de monitorizar el juego, y cada escaneo vuelve a proponer
/// esa carpeta con confianza alta. Por eso apuntar un juego a la carpeta de otro
/// no es un error recuperable: es veneno permanente y casi invisible. Pasó en
/// ago-2026 —`horizon-forbidden-west` → `…\Saved Games\Surviving Mars
/// Relaunched`— y dejó al juego dueño de esa carpeta sin poder rastrearla.
///
/// Dos árbitros, porque el veneno se puede cuajar antes de que nadie rastree
/// nada: las filas ya rastreadas y el informe de detección en caché.
pub fn manual_override_conflict(
    state: &CliState,
    report: Option<&DetectionReport>,
    slug: &str,
    path: &Path,
) -> Option<String> {
    if let Some((_, st)) = state.saves.iter().find(|(_, st)| {
        st.game_slug != slug && crate::detection::paths_overlap(&st.local_path, path)
    }) {
        return Some(st.game_slug.clone());
    }
    report?
        .games
        .iter()
        .find(|g| {
            g.slug != slug
                && g.found_paths
                    .iter()
                    .any(|p| crate::detection::paths_overlap(p, path))
        })
        .map(|g| g.slug.clone())
}

/// Filas locales que un alta deja obsoletas: las que cubren el mismo (juego,
/// etiqueta) o **la misma carpeta** con un id distinto del que se va a insertar.
///
/// Existe porque en self-hosted el `save_id` lo pone el servidor, no el cliente.
/// Si su fila para este (juego, etiqueta) ya no es la que esta máquina tenía
/// mapeada —porque el servidor se reinstaló y repartió ids nuevos— insertar sin
/// más deja DOS filas sobre la misma carpeta.
fn superseded_rows(
    state: &CliState,
    slug: &str,
    label: &str,
    local_path: &Path,
    keep_id: &str,
) -> Vec<String> {
    state
        .saves
        .iter()
        .filter(|(id, st)| {
            let same_slot = st.game_slug == slug
                && match slots::slot_of(label) {
                    // By slot, for the same reason the cloud dedup above is:
                    // `"2"` and `"2 · Mods"` are one slot, and comparing the
                    // text leaves the other one behind as a second row.
                    Some(n) => slots::slot_of(&st.label) == Some(n),
                    None => st.label == label,
                };
            id.as_str() != keep_id && (same_slot || same_folder(&st.local_path, local_path))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Filas locales que el servidor no conoce, en self-hosted.
///
/// Ahí el `save_id` es del servidor: un id que no está en su lista no se puede
/// subir (todo snapshot contra él es un 404) y la biblioteca —que pinta lo que
/// lista el servidor— tampoco lo enseña, así que el usuario no puede ni verlo ni
/// quitarlo. Pero sigue contando para «una carpeta, un juego», de modo que
/// bloquea para siempre volver a dar de alta esa carpeta. Podarlas es la única
/// salida sin tocar ficheros a mano.
fn rows_unknown_to_server(state: &CliState, known: &HashSet<String>) -> Vec<String> {
    let mut stale: Vec<String> = state
        .saves
        .keys()
        .filter(|id| !known.contains(*id))
        .cloned()
        .collect();
    stale.sort();
    stale
}

/// The cloud's own row for a slot of this game, if it has one.
///
/// Best-effort on purpose: a manifest that won't load is not a reason to refuse
/// tracking a folder. Getting it wrong the safe way means minting an id and
/// having two rows, which `list_tracked` already prunes down; refusing the add
/// would leave the user with nothing.
async fn cloud_row_for_slot(
    client: &ApiClient,
    slug: &str,
    want: Option<u32>,
    label: &str,
) -> Option<String> {
    let manifest = match client.cloud_sync().await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "couldn't read the manifest before adding; minting a local id");
            return None;
        }
    };
    let found = manifest
        .saves
        .iter()
        .find(|e| {
            e.game_slug == slug
                && match want {
                    Some(n) => slots::slot_of(&e.label) == Some(n),
                    None => e.label == label,
                }
        })
        .map(|e| e.save_id.clone());
    if let Some(id) = &found {
        tracing::info!(save_id = %id, slug, "adopting the cloud row that already holds this slot");
    }
    found
}

/// Prefix of the error the UI turns into the "move it or add it?" dialog. The
/// shape is `slot_occupied:<label>:<free slot>:<current folder>`, with the
/// folder last because on Windows it has a `:` inside.
pub const ERR_SLOT_OCCUPIED: &str = "slot_occupied";

/// Fails when the requested slot of the game is already held by a **different**
/// folder and nobody said they wanted it moved.
///
/// Without this the add reused the (game, label) row and overwrote its
/// `local_path`: pointing at a second folder added nothing, it **moved** the
/// first one, silently and with no undo. That is what happened to a user in
/// aug-2026 with Factorio, which ended up backing up a loose desktop folder
/// while the game's real folder stopped uploading with nothing to say so.
///
/// Re-pointing is still legitimate — a game reinstalled on another drive does
/// move its folder — so this does not forbid it: it takes it out of silence.
/// The error carries the folder that is there now and the lowest free number,
/// which is all the UI needs to ask "do I move slot 1, or is this your slot
/// 2?".
fn occupied_slot(
    state: &CliState,
    slug: &str,
    label: &str,
    local_path: &Path,
    repoint: bool,
) -> Result<()> {
    if repoint {
        return Ok(());
    }
    // Por número y no por cadena: desde que la etiqueta lleva nombre, `"2"` y
    // `"2 · Mods"` son la MISMA ranura, y comparar el texto dejaría colar una
    // segunda carpeta en el 2 sólo por llamarse distinto.
    let want = slots::slot_of(label);
    let Some(current) = state
        .saves
        .values()
        .find(|st| {
            st.game_slug == slug
                && match want {
                    Some(n) => slots::slot_of(&st.label) == Some(n),
                    None => st.label == label,
                }
        })
        .map(|st| st.local_path.clone())
    else {
        return Ok(());
    };
    if same_folder(&current, local_path) {
        return Ok(());
    }
    let taken = state
        .saves
        .values()
        .filter(|st| st.game_slug == slug)
        .filter_map(|st| slots::slot_of(&st.label));
    anyhow::bail!(
        "{ERR_SLOT_OCCUPIED}:{label}:{}:{}",
        slots::next_free(taken),
        current.display()
    )
}

fn conflicting_save<'a>(
    state: &'a CliState,
    local_path: &Path,
    except_save_ids: &[&str],
) -> Option<&'a SaveState> {
    state
        .saves
        .iter()
        .find(|(id, st)| {
            !except_save_ids.contains(&id.as_str())
                && crate::detection::paths_overlap(&st.local_path, local_path)
        })
        .map(|(_, st)| st)
}

/// Registra que el usuario quiere respaldar un juego/carpeta. Crea la fila en el
/// server (cloud la materializa en el primer upload) y escribe el mapping local.
/// Devuelve la fila + el `WatchedSave` a enganchar. La CLI (`track.rs`) y el
/// desktop llaman aquí en vez de reimplementar el flujo.
/// Reject a save whose slug is plumbing rather than a game — `user`, `desktop`,
/// `appdata` and friends (`GENERIC_IDENTITY_TOKENS`).
///
/// **This is the only gate that can actually prevent one.** `CliState::cleanse`
/// spots the same slugs, but it runs when the state is *loaded*: by then the
/// save exists, it has been uploaded, and the server has a row for it. Fourteen
/// such rows across thirteen accounts reached production that way — the
/// detection was never wrong, it was always too late. Refusing at the door is
/// what keeps them out.
///
/// Deliberately uses `GameSlug::repair` and nothing else. The wider
/// `agent::is_generic_identity_token` also vetoes the components of *this*
/// machine's home directory, which is right for matching live processes and
/// wrong here: a save's identity is `(user, game_slug, label)` on the server and
/// must mean the same thing on every device. Judging it against the local
/// username would quarantine a save on one machine and wave it through on
/// another.
fn reject_degenerate_slug(slug: &str) -> Result<()> {
    match hoard_core::ids::GameSlug::repair(slug) {
        hoard_core::ids::Repair::Quarantined { reason, .. } => anyhow::bail!(
            "'{slug}' doesn't name a game ({reason}) — it looks like part of a folder path. \
             Pick the game by name, or point Hoard at the save folder and let it identify it."
        ),
        _ => Ok(()),
    }
}

pub async fn add_to_tracking(client: &ApiClient, args: AddGameArgs) -> Result<TrackOutcome> {
    reject_degenerate_slug(&args.game_slug)?;
    // The slot outranks the label: `label` only survives for the older
    // free-form labels (and for the CLI, which still accepts them).
    let label = args
        .slot
        .map(|n| slots::label_for(n, args.name.as_deref()))
        .or_else(|| args.label.clone())
        .unwrap_or_else(|| slots::label_for(slots::SAVES, args.name.as_deref()));
    let pinned_processes = args.processes.clone().unwrap_or_default();
    let preset_name: Option<String> = args
        .preset
        .clone()
        .or_else(|| presets::builtin_preset_for(&args.game_slug).map(str::to_string));

    let local_path = PathBuf::from(&args.local_path);
    // Re-añadir el MISMO (juego, etiqueta) es un flujo legítimo (re-track,
    // re-onboarding, re-alta por detección) y reusa la fila existente más
    // abajo; sólo tiene que fallar si la carpeta es de OTRO save.
    //
    // Y "el mismo juego" no se decide sólo por el slug: el mismo título sale
    // `vrising` o `v-rising`, `dispatch` o `dispatch-2025`, según lo nombre el
    // appid de Steam, el catálogo o el launcher. Si además de por (slug,
    // etiqueta) se reconoce **la carpeta**, un re-alta bajo otro nombre reusa
    // la fila en vez de estrellarse contra ella.
    if let Ok((state, _)) = CliState::load_default() {
        occupied_slot(&state, &args.game_slug, &label, &local_path, args.repoint)?;
    }
    let reusing = CliState::load_default().ok().and_then(|(state, _)| {
        state
            .saves
            .iter()
            .find(|(_, st)| st.game_slug == args.game_slug && st.label == label)
            .map(|(id, _)| id.clone())
            .or_else(|| {
                // The same folder under another name for the SAME game
                // (`vrising` vs `v-rising`) is a re-add and reuses the row.
                // Under a different slot it is not: there the user is filing an
                // already-tracked folder in a second place, and that is two
                // histories over the same bytes. Let the rule fire.
                row_for_same_folder(&state, &local_path)
                    .filter(|id| state.saves.get(*id).is_some_and(|st| st.label == label))
                    .map(str::to_string)
            })
    });
    let except: Vec<&str> = reusing.iter().map(String::as_str).collect();
    validate_folder(&local_path, &except)?;

    // Cloud no tiene `create_save` server-side: la fila se materializa en el
    // primer upload (UPSERT en (user_id, game_slug, label)). El cliente minta un
    // save_id local, guarda el path y empieza a vigilar.
    if client.is_cloud().await {
        let (mut cli_state, path) = CliState::load_default()?;
        // Reuse the row this game already has for the slot instead of minting
        // another (re-track, re-onboarding and re-add from detection each used
        // to leave a second one).
        //
        // Matched by **slot**, not by the label string. Comparing the text meant
        // that giving a folder a name minted a brand new save: the local row
        // said `"2 - shit"`, the add composed `"2 · shit2"`, no string matched,
        // and out came a fresh uuid and a third cloud row for one folder. Two of
        // those piled up on one account in aug-2026 before anybody noticed,
        // because each looked like a normal save until you counted them.
        let want = slots::slot_of(&label);
        let local_match = cli_state
            .saves
            .iter()
            .find(|(_, st)| {
                st.game_slug == args.game_slug
                    && match want {
                        Some(n) => slots::slot_of(&st.label) == Some(n),
                        None => st.label == label,
                    }
            })
            .map(|(id, _)| id.clone());
        // Nothing local, but the cloud may still hold this slot — and if it
        // does, that row is the save, not a new one. Local state alone is not
        // enough to answer this: untracking the folder empties it, so the
        // untrack-then-add-again that people reach for after any error found no
        // row to match, minted a fresh uuid, and split one folder into a second
        // cloud save. That happened three times on one account in aug-2026,
        // each time looking like the sync had simply stopped working, because
        // the two machines were then uploading to different rows.
        let save_id = match local_match {
            Some(id) => id,
            None => cloud_row_for_slot(client, &args.game_slug, want, &label)
                .await
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        };
        cli_state.saves.insert(
            save_id.clone(),
            SaveState {
                local_path: local_path.clone(),
                game_slug: args.game_slug.clone(),
                label: label.clone(),
                last_backup_at: None,
                last_version_num: None,
                paused: false,
                preset: preset_name.clone(),
                set_hash: None,
                processes: pinned_processes.clone(),
                shared_processes: args.shared_processes,
                allow_device_local: None,
            },
        );
        cli_state.save(&path)?;

        let watched = watched_save_from(
            save_id.clone(),
            args.game_slug.clone(),
            args.game_slug.clone(),
            label.clone(),
            local_path.clone(),
            preset_name.as_deref(),
            pinned_processes.clone(),
            args.shared_processes,
            None,
        );
        return Ok(TrackOutcome {
            tracked: TrackedSave {
                save_id,
                game_slug: args.game_slug,
                name: slots::name_of(&label).map(str::to_string),
                slot: slots::slot_of(&label),
                label,
                local_path: local_path.to_string_lossy().into_owned(),
                cloud_version_num: None,
                local_version_num: None,
                last_backup_at: None,
                paused: false,
                total_size_bytes: 0,
                orphan: false,
                local_size_bytes: None,
                preset: preset_name,
                allow_device_local: None,
            },
            watched,
        });
    }

    // Self-hosted: crea (o re-vincula en 409) la fila server-side.
    let save = match client
        .create_save_with_meta(
            &args.game_slug,
            &label,
            args.display_name.as_deref(),
            args.steam_app_id,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            // 409 = ya existe la fila (game_slug,label): destrack + retrack.
            // Recupera vinculando la existente para no perder el histórico.
            let is_conflict = e
                .downcast_ref::<crate::api::ApiError>()
                .map(|api| matches!(api, crate::api::ApiError::Conflict(_)))
                .unwrap_or(false);
            if !is_conflict {
                return Err(e);
            }
            let existing = client.list_saves(Some(&args.game_slug)).await?;
            existing
                .into_iter()
                .find(|s| s.game_slug.as_str() == args.game_slug && s.label == label)
                .context("Couldn't re-link the existing save on the server.")?
        }
    };

    let (mut cli_state, path) = CliState::load_default()?;
    // El alta self-hosted es un REEMPLAZO, no una suma. La fila la identifica
    // la carpeta, y el id lo pone el servidor: si reparte uno nuevo para la
    // misma carpeta —reinstalar el servidor rehace la base y con ella los ids—
    // insertar sin podar deja dos filas sobre los mismos bytes. La vieja ya no
    // existe en el servidor (todo lo que suba da 404), no sale en la biblioteca
    // y aun así hace saltar «una carpeta, un juego» en cada intento posterior:
    // el juego queda encallado sin ninguna salida por la UI. Le pasó a un
    // self-hoster en ago-2026 con ~40 juegos a la vez tras rehacer su stack.
    let new_id = save.id.to_string();
    for stale in superseded_rows(&cli_state, &args.game_slug, &label, &local_path, &new_id) {
        tracing::info!(
            save_id = %stale, slug = %args.game_slug,
            "dropping the local row this add supersedes"
        );
        cli_state.saves.remove(&stale);
    }
    cli_state.saves.insert(
        new_id,
        SaveState {
            local_path: local_path.clone(),
            game_slug: save.game_slug.to_string(),
            label: save.label.clone(),
            last_backup_at: None,
            last_version_num: None,
            paused: false,
            preset: preset_name.clone(),
            set_hash: None,
            processes: pinned_processes.clone(),
            shared_processes: args.shared_processes,
            allow_device_local: None,
        },
    );
    cli_state.save(&path)?;

    let watched = watched_save_from(
        save.id.to_string(),
        save.game_slug.to_string(),
        args.game_slug.clone(),
        save.label.clone(),
        local_path.clone(),
        preset_name.as_deref(),
        pinned_processes.clone(),
        args.shared_processes,
        None,
    );

    Ok(TrackOutcome {
        tracked: TrackedSave {
            save_id: save.id.into_inner(),
            game_slug: save.game_slug.into_inner(),
            name: slots::name_of(&save.label).map(str::to_string),
            slot: slots::slot_of(&save.label),
            label: save.label,
            local_path: local_path.to_string_lossy().into_owned(),
            cloud_version_num: save.latest_version_num,
            // Recién insertado en `CliState` con el cursor a cero: la cabeza
            // del server puede existir ya (re-vínculo por 409) pero esta
            // máquina todavía no tiene ninguna versión.
            local_version_num: None,
            last_backup_at: None,
            paused: false,
            total_size_bytes: save.total_size_bytes.unwrap_or(0),
            orphan: false,
            local_size_bytes: None,
            preset: preset_name,
            allow_device_local: None,
        },
        watched,
    })
}

/// Adopta (vincula) un save cloud de otra máquina: asocia una carpeta local de
/// ESTA máquina al `save_id` existente en vez de mintar uno nuevo. Deja el
/// cursor de versión abierto para que el auto-restore on-add baje el último
/// snapshot (sync). Núcleo del cross-device sync.
pub async fn adopt(client: &ApiClient, args: AdoptArgs) -> Result<TrackOutcome> {
    // Same door, same guard: adopting a cloud row is still how a bad slug
    // enters this machine's state.
    reject_degenerate_slug(&args.game_slug)?;
    // La sesión debe existir (el caller ya construyó `client`); no hay llamada
    // server aquí, la fila cloud ya existe.
    let _ = client;
    let local_path = PathBuf::from(&args.local_path);
    // Adoptar es reapuntar un save que ya existe en la nube: solaparse consigo
    // mismo no es un conflicto de "una carpeta, un juego".
    //
    // Y "consigo mismo" son DOS entradas, no una. Esta máquina pudo dar de alta
    // el juego por su cuenta —detección, alta manual— mintándose un `save_id`
    // local; la nube trae el suyo. Mismo juego, misma carpeta, ids distintos:
    // excluyendo sólo el de la nube, la entrada local se bloquea a sí misma y el
    // juego queda encallado para siempre. El escaneo automático falla en cada
    // vuelta con «'furi' already tracks … — one folder, one game» (chocando
    // contra sí mismo), el "+" manual falla igual, y reapuntar la carpeta desde
    // la ficha también: no queda ni una vía en la UI para salir del estado.
    // Y no basta con buscar por slug: el mismo juego llega con nombres distintos
    // según la fuente (`vrising` por el appid de Steam, `v-rising` por el
    // catálogo; `dispatch` y `dispatch-2025`). Lo que identifica a la fila que
    // se está relevando es **la carpeta**, que es la unidad de la propia regla.
    let superseded = CliState::load_default().ok().and_then(|(state, _)| {
        state
            .saves
            .iter()
            .find(|(id, st)| {
                id.as_str() != args.save_id
                    && st.game_slug == args.game_slug
                    && st.label == args.label
            })
            .map(|(id, _)| id.clone())
            .or_else(|| {
                row_for_same_folder(&state, &local_path)
                    .filter(|id| *id != args.save_id)
                    .map(str::to_string)
            })
    });
    let except: Vec<&str> = std::iter::once(args.save_id.as_str())
        .chain(superseded.iter().map(String::as_str))
        .collect();
    validate_folder(&local_path, &except)?;

    let (mut cli_state, path) = CliState::load_default()?;
    // El relevo es un reemplazo, no una suma: dejar viva la entrada local
    // dejaría dos watchers y dos historiales sobre los mismos bytes — justo lo
    // que la regla de "una carpeta, un juego" existe para impedir— y el panel
    // pintaría el juego dos veces, una de ellas sin versiones ni tamaño.
    if let Some(old) = superseded.as_deref() {
        cli_state.saves.remove(old);
    }
    let preset = presets::builtin_preset_for(&args.game_slug).map(str::to_string);
    cli_state.saves.insert(
        args.save_id.clone(),
        SaveState {
            local_path: local_path.clone(),
            game_slug: args.game_slug.clone(),
            label: args.label.clone(),
            last_backup_at: None,
            last_version_num: None,
            paused: false,
            preset: preset.clone(),
            set_hash: None,
            processes: Vec::new(),
            shared_processes: false,
            allow_device_local: None,
        },
    );
    cli_state.save(&path)?;

    let watched = watched_save_from(
        args.save_id.clone(),
        args.game_slug.clone(),
        args.game_slug.clone(),
        args.label.clone(),
        local_path.clone(),
        preset.as_deref(),
        Vec::new(),
        false,
        None,
    );

    Ok(TrackOutcome {
        tracked: TrackedSave {
            save_id: args.save_id,
            game_slug: args.game_slug,
            name: slots::name_of(&args.label).map(str::to_string),
            slot: slots::slot_of(&args.label),
            label: args.label,
            local_path: local_path.to_string_lossy().into_owned(),
            cloud_version_num: None,
            local_version_num: None,
            last_backup_at: None,
            paused: false,
            total_size_bytes: 0,
            orphan: false,
            local_size_bytes: None,
            preset,
            allow_device_local: None,
        },
        watched,
    })
}

fn format_optional_time(t: Option<OffsetDateTime>) -> Option<String> {
    use time::format_description::well_known::Rfc3339;
    t.and_then(|x| x.format(&Rfc3339).ok())
}

/// Rellena `local_size_bytes` de cada fila no huérfana caminando su carpeta
/// (solo metadata). Los huérfanos se quedan `None`.
fn fill_local_sizes(out: &mut [TrackedSave]) {
    for t in out.iter_mut() {
        if t.orphan || t.local_path.is_empty() {
            continue;
        }
        let p = Path::new(&t.local_path);
        if p.is_dir() {
            t.local_size_bytes = Some(dir_size_bytes(p) as i64);
        }
    }
}

/// Poda las filas ENVENENADAS por la correlación y las elimina del estado.
/// Devuelve sus `save_id` (el caller persiste y las despega del agente vivo).
///
/// El nombre de un descubrimiento de fase 4 sale del proceso que la correlación
/// atribuyó a la carpeta, así que una atribución mala rastrea el save con el
/// nombre de una app: el informe de jul-2026 traía `ChatGPT`, `opencode` y
/// `code` apuntando los tres a la carpeta de Planet S. Como cada nombre da un
/// slug distinto, la poda por (slug,label) no las ve.
///
/// Sólo cae lo DEMOSTRABLEMENTE basura: una fila cuyo slug no pasa
/// [`crate::correlation::is_game_like`] **y** cuya carpeta ya está cubierta por
/// otra fila que sí parece un juego. Podar sólo por nombre se comería juegos
/// reales — la lista negra casa por substring, así que "Hoard" o
/// "Reaper: Tale of a Pale Swordsman" darían falso positivo. Una fila
/// envenenada que sea la ÚNICA de su carpeta se queda: ahí no hay a quién
/// devolverle el save, y renombrarla o soltarla es decisión del usuario.
fn prune_poisoned_rows(state: &mut CliState) -> Vec<String> {
    let looks_like_game = |slug: &str| crate::correlation::is_game_like(slug, None);
    let rows: Vec<(String, String, PathBuf)> = state
        .saves
        .iter()
        .map(|(id, st)| (id.clone(), st.game_slug.clone(), st.local_path.clone()))
        .collect();

    let mut poisoned: Vec<String> = Vec::new();
    for (id, slug, local) in &rows {
        if looks_like_game(slug) || local.as_os_str().is_empty() {
            continue;
        }
        let covered = rows.iter().any(|(other_id, other_slug, other_local)| {
            other_id != id
                && looks_like_game(other_slug)
                && !other_local.as_os_str().is_empty()
                && crate::detection::paths_overlap(local, other_local)
        });
        if covered {
            tracing::info!(
                save_id = %id,
                slug = %slug,
                path = %local.display(),
                "library: fila con nombre de app sobre una carpeta ya rastreada por un juego; se despega"
            );
            poisoned.push(id.clone());
        }
    }
    for id in &poisoned {
        state.saves.remove(id);
    }
    poisoned.sort();
    poisoned
}

/// Qué hacer con una carpeta recién detectada en un alta **automática**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoTrack {
    /// Darla de alta ahora.
    Track,
    /// Todavía no: no hay ni un fichero dentro y el servidor tampoco tiene nada
    /// de este juego, así que no hay nada que respaldar ni que restaurar.
    SkipEmpty,
}

/// Decide si una carpeta detectada se da de alta sola.
///
/// Rastrear una carpeta vacía no respalda nada: el motor la mira, no encuentra
/// bytes y avisa «nothing to back up and this save has never had a snapshot» en
/// cada vuelta (72 veces en tres días en el log de un self-hoster, ago-2026,
/// por carpetas de Goldberg que el juego nunca llegó a usar). No se pierde
/// nada por esperar: el escaneo automático vuelve a pasar cada pocos minutos y
/// la da de alta en cuanto el juego escriba su primer fichero — que es también
/// el primer instante en el que había algo que guardar.
///
/// **La excepción que importa**: si el servidor ya tiene ese save (otra máquina
/// lo subió), la carpeta vacía es justo el caso bueno —máquina nueva esperando
/// una restauración— y se da de alta igual.
///
/// Ante la duda, dar de alta: un error de lectura, un permiso, un árbol
/// gigantesco… cualquier cosa que impida contestar cuenta como "tiene
/// contenido". Este filtro sólo puede quitar ruido, nunca vigilancia.
pub fn auto_track_decision(path: &Path, has_server_row: bool) -> AutoTrack {
    if has_server_row || dir_has_any_file(path) {
        AutoTrack::Track
    } else {
        AutoTrack::SkipEmpty
    }
}

/// ¿Hay al menos un fichero en el árbol? Recorrido acotado —se para en el
/// primero— y **fail-open**: si no se puede contestar (permisos, un árbol más
/// grande que los topes) devuelve `true`.
fn dir_has_any_file(root: &Path) -> bool {
    const MAX_ENTRIES: usize = 512;
    const MAX_DEPTH: usize = 6;

    if root.is_file() {
        return true;
    }
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0usize;
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return true; // no se puede mirar ⇒ no se decide en contra
        };
        for entry in entries {
            let Ok(entry) = entry else { return true };
            seen += 1;
            if seen > MAX_ENTRIES {
                return true;
            }
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => {
                    if depth < MAX_DEPTH {
                        pending.push((entry.path(), depth + 1));
                    } else {
                        return true; // más hondo de lo que miramos
                    }
                }
                Ok(_) => return true, // fichero (o symlink): hay contenido
                Err(_) => return true,
            }
        }
    }
    false
}

/// Una fila tal y como la lista el servidor, reducida a lo que decide la
/// reconciliación. Existe para poder probar la decisión sin un servidor.
#[derive(Debug, Clone)]
pub struct ServerRow {
    pub id: String,
    pub game_slug: String,
    pub label: String,
}

/// La decisión de [`reconcile_with_server`], sin IO: por cada fila local que el
/// servidor no conoce, el id al que re-apuntarla — o `None` para tirarla.
fn reconcile_plan(state: &CliState, server: &[ServerRow]) -> Vec<(String, Option<String>)> {
    let known: HashSet<String> = server.iter().map(|r| r.id.clone()).collect();
    let by_key: std::collections::HashMap<(&str, &str), &str> = server
        .iter()
        .map(|r| ((r.game_slug.as_str(), r.label.as_str()), r.id.as_str()))
        .collect();
    rows_unknown_to_server(state, &known)
        .into_iter()
        .filter_map(|id| {
            let row = state.saves.get(&id)?;
            let reissued = by_key
                .get(&(row.game_slug.as_str(), row.label.as_str()))
                .map(|s| s.to_string());
            Some((id, reissued))
        })
        .collect()
}

/// Qué hizo [`reconcile_with_server`]. Sólo para el log: el estado ya está en
/// disco cuando esto vuelve.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    /// Filas re-apuntadas al `save_id` que el servidor tiene ahora para ese
    /// mismo (juego, etiqueta).
    pub relinked: usize,
    /// Filas tiradas: el servidor no sabe nada de ese juego.
    pub dropped: usize,
}

impl Reconciliation {
    pub fn changed(&self) -> bool {
        self.relinked > 0 || self.dropped > 0
    }
}

/// Cura el estado local contra lo que el servidor dice tener. **Self-hosted
/// only**; en Cloud el `save_id` lo minta el cliente y no puede quedar huérfano
/// así, y su duplicado ya lo poda `list_tracked` con el manifiesto.
///
/// Rehacer el servidor —perder la base, migrar el stack, empezar de cero— le
/// reparte ids nuevos a los mismos juegos. Las filas locales se quedan
/// apuntando a ids que ya no existen: cada subida devuelve 404 y se reintenta
/// en bucle (1.353 intentos en tres días en el caso de ago-2026), el juego no
/// se pinta en la biblioteca y encima bloquea su carpeta. Reconciliar al
/// arrancar el motor significa que **actualizar la app repara la máquina sola**,
/// sin que el usuario tenga que borrar nada a mano.
///
/// Re-apuntar y no borrar cuando se puede: si el servidor tiene una fila para
/// el mismo (juego, etiqueta), la fila local se queda con su carpeta y sus
/// ajustes y sólo cambia de id. Se reinician el cursor de versión y el
/// `set_hash` porque son del servidor viejo: el nuevo empieza en cero, y con el
/// `set_hash` puesto la primera subida se saltaría por "bytes sin cambios".
pub async fn reconcile_with_server(client: &ApiClient) -> Result<Reconciliation> {
    if client.is_cloud().await {
        return Ok(Reconciliation::default());
    }
    let server = client.list_saves(None).await?;
    let (mut state, path) = CliState::load_default()?;

    let rows: Vec<ServerRow> = server
        .iter()
        .map(|s| ServerRow {
            id: s.id.to_string(),
            game_slug: s.game_slug.to_string(),
            label: s.label.clone(),
        })
        .collect();

    let mut out = Reconciliation::default();
    for (id, reissued) in reconcile_plan(&state, &rows) {
        let Some(row) = state.saves.get(&id).cloned() else {
            continue;
        };
        let slug = row.game_slug.clone();
        state.saves.remove(&id);
        match reissued {
            // El servidor tiene el mismo juego con otro id: se releva la fila.
            // Si el id nuevo ya estaba mapeado (el gemelo del alta), esto
            // sobreescribe una sola vez y la vieja desaparece igual.
            Some(new_id) => {
                state.saves.insert(
                    new_id.clone(),
                    SaveState {
                        last_version_num: None,
                        set_hash: None,
                        ..row
                    },
                );
                tracing::info!(
                    old = %id, new = %new_id, slug = %slug,
                    "state: re-linked a save the server re-issued"
                );
                out.relinked += 1;
            }
            None => {
                tracing::info!(
                    save_id = %id, slug = %slug,
                    "state: dropped a save the server doesn't have"
                );
                out.dropped += 1;
            }
        }
    }
    if out.changed() {
        state.save(&path)?;
    }
    Ok(out)
}

/// Lista los saves que Hoard rastrea para el usuario logueado. El server manda
/// en `latest_version_num`; el path local sale de `CliState`. Devuelve también
/// los `save_id` "perdedores" que se podaron (duplicados o envenenados) para que
/// el frontend los despegue del agente vivo.
pub async fn list_tracked(client: &ApiClient) -> Result<(Vec<TrackedSave>, Vec<String>)> {
    let mut detached: Vec<String> = Vec::new();

    if client.is_cloud().await {
        let manifest = client.cloud_sync().await?;
        let (mut cli_state, path) = CliState::load_default()?;

        // Self-heal de filas duplicadas: el cloud fuerza una por (slug,label).
        // Ganador = con versión subida (en manifest), luego con carpeta local.
        let score = |id: &str, local: &Path| -> u8 {
            let in_manifest = manifest.saves.iter().any(|e| e.save_id == id) as u8;
            let exists = local.exists() as u8;
            in_manifest * 2 + exists
        };
        // Recorrido ORDENADO por id: el de un HashMap no lo es, así que en un
        // empate cada listado podaba una fila distinta y el churn no acababa
        // nunca. Con el orden fijo gana siempre el id menor.
        let mut rows: Vec<(String, String, String, PathBuf)> = cli_state
            .saves
            .iter()
            .map(|(id, st)| {
                (
                    id.clone(),
                    st.game_slug.clone(),
                    st.label.clone(),
                    st.local_path.clone(),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));

        let mut winners: std::collections::HashMap<(String, String), (String, u8)> =
            std::collections::HashMap::new();
        let mut losers: Vec<String> = Vec::new();
        for (id, slug, label, local) in &rows {
            let key = (slug.clone(), label.clone());
            let s = score(id, local);
            match winners.get(&key) {
                None => {
                    winners.insert(key, (id.clone(), s));
                }
                Some((cur_id, cur_s)) => {
                    if s > *cur_s {
                        losers.push(cur_id.clone());
                        winners.insert(key, (id.clone(), s));
                    } else {
                        losers.push(id.clone());
                    }
                }
            }
        }
        for id in &losers {
            cli_state.saves.remove(id);
        }
        losers.extend(prune_poisoned_rows(&mut cli_state));
        if !losers.is_empty() {
            cli_state.save(&path)?;
            detached = losers;
        }

        // The server's label wins over the local copy. A rename travels through
        // the row, so a machine that keeps its old copy uploads under the old
        // label, the server UPSERTs on (user, slug, label) and the save forks in
        // two — one row per machine, each with half the history. Renaming was
        // rare enough for this to stay hidden; naming a folder makes it routine.
        let mut relabelled = false;
        for (id, st) in cli_state.saves.iter_mut() {
            let Some(entry) = manifest.saves.iter().find(|e| &e.save_id == id) else {
                continue;
            };
            if st.label != entry.label {
                tracing::info!(
                    save_id = %id, from = %st.label, to = %entry.label,
                    "adopting the server's label for this save"
                );
                st.label = entry.label.clone();
                relabelled = true;
            }
        }
        if relabelled {
            cli_state.save(&path)?;
        }

        let mut out = Vec::with_capacity(cli_state.saves.len());
        for (id, st) in &cli_state.saves {
            let entry = manifest.saves.iter().find(|e| &e.save_id == id);
            out.push(TrackedSave {
                save_id: id.clone(),
                game_slug: st.game_slug.clone(),
                name: slots::name_of(&st.label).map(str::to_string),
                slot: slots::slot_of(&st.label),
                label: st.label.clone(),
                local_path: st.local_path.to_string_lossy().into_owned(),
                cloud_version_num: entry.map(|e| e.latest_version_num),
                local_version_num: st.last_version_num,
                // Manifest `updated_at` bumps on every committed upload, so it
                // doubles as "last backup" for cloud rows (the panel sorts on it).
                last_backup_at: entry.map(|e| e.updated_at.clone()),
                paused: st.paused,
                total_size_bytes: entry.map(|e| e.latest_size_bytes).unwrap_or(0),
                orphan: false,
                local_size_bytes: None,
                preset: st.preset.clone(),
                allow_device_local: st.allow_device_local,
            });
        }

        // Visibilidad cross-device: un save subido desde OTRA máquina vive en el
        // manifest sin fila local aquí. Emítelo como huérfano para que el usuario
        // pueda adoptarlo/restaurarlo.
        for entry in &manifest.saves {
            if cli_state.saves.contains_key(&entry.save_id) {
                continue;
            }
            out.push(TrackedSave {
                save_id: entry.save_id.clone(),
                game_slug: entry.game_slug.clone(),
                name: slots::name_of(&entry.label).map(str::to_string),
                slot: slots::slot_of(&entry.label),
                label: entry.label.clone(),
                local_path: String::new(),
                cloud_version_num: Some(entry.latest_version_num),
                local_version_num: None,
                last_backup_at: Some(entry.updated_at.clone()),
                total_size_bytes: entry.latest_size_bytes,
                paused: false,
                orphan: true,
                local_size_bytes: None,
                preset: None,
                allow_device_local: None,
            });
        }
        fill_local_sizes(&mut out);
        return Ok((out, detached));
    }

    // Self-hosted: el server lista todas las filas; enriquecemos con CliState.
    let saves = client.list_saves(None).await?;
    let (mut cli_state, state_path) = CliState::load_default()?;
    // La poda por (slug,label) es cloud-only (el manifest es su árbitro), pero
    // la de filas envenenadas no necesita nube: su árbitro es el nombre y la
    // carpeta. Un self-hoster sufre el mismo churn de atribución.
    let mut pruned = prune_poisoned_rows(&mut cli_state);
    // Y la poda que el cloud hacía con su manifiesto, aquí con la lista del
    // servidor: un id que no está en ella es papel mojado —no se puede subir
    // (404), no se pinta— pero sigue bloqueando su carpeta. La biblioteca es el
    // único sitio por el que esa fila puede desaparecer sola.
    let known: HashSet<String> = saves.iter().map(|s| s.id.to_string()).collect();
    let unknown = rows_unknown_to_server(&cli_state, &known);
    if !unknown.is_empty() {
        tracing::warn!(
            count = unknown.len(),
            "pruning tracked rows the server doesn't know about"
        );
    }
    for id in &unknown {
        cli_state.saves.remove(id);
    }
    pruned.extend(unknown);
    if !pruned.is_empty() {
        cli_state.save(&state_path)?;
        detached = pruned;
    }
    let mut out = Vec::with_capacity(saves.len());
    for s in saves {
        match cli_state.saves.get(s.id.as_str()) {
            Some(st) => out.push(TrackedSave {
                save_id: s.id.into_inner(),
                game_slug: s.game_slug.into_inner(),
                name: slots::name_of(&s.label).map(str::to_string),
                slot: slots::slot_of(&s.label),
                label: s.label,
                local_path: st.local_path.to_string_lossy().into_owned(),
                cloud_version_num: s.latest_version_num,
                local_version_num: st.last_version_num,
                last_backup_at: format_optional_time(Some(s.updated_at)),
                paused: st.paused,
                total_size_bytes: s.total_size_bytes.unwrap_or(0),
                orphan: false,
                local_size_bytes: None,
                preset: st.preset.clone(),
                allow_device_local: st.allow_device_local,
            }),
            None => out.push(TrackedSave {
                save_id: s.id.into_inner(),
                game_slug: s.game_slug.into_inner(),
                name: slots::name_of(&s.label).map(str::to_string),
                slot: slots::slot_of(&s.label),
                label: s.label,
                local_path: String::new(),
                cloud_version_num: s.latest_version_num,
                local_version_num: None,
                last_backup_at: format_optional_time(Some(s.updated_at)),
                paused: false,
                total_size_bytes: s.total_size_bytes.unwrap_or(0),
                orphan: true,
                local_size_bytes: None,
                preset: None,
                allow_device_local: None,
            }),
        }
    }
    fill_local_sizes(&mut out);
    Ok((out, detached))
}

/// Renombra la etiqueta de un save en server + estado local. Un 409 (otra save
/// del mismo juego ya usa esa etiqueta) sube como `ApiError::Conflict` para que
/// el frontend muestre el mensaje localizado. Devuelve la fila + el `WatchedSave`
/// a re-enganchar (la etiqueta es parte de la clave de upload), o `None` si no
/// hay path local.
/// Name (or un-name) a folder without touching its number.
///
/// The number is what pairs this folder with the same one on the other
/// machines, so it is never up for editing as text: this recomposes the label
/// around the slot the save already has. Letting the user type the whole label
/// is what made naming a slot `"2 - Mods"` silently drop it out of slot 2.
pub async fn set_slot_name(
    client: &ApiClient,
    save_id: &str,
    name: Option<&str>,
) -> Result<(TrackedSave, Option<WatchedSave>)> {
    let current = local_label(save_id)?;
    let label = match slots::slot_of(&current) {
        // A free-form label from before slots existed has no number to preserve;
        // naming it is still just a rename.
        None => slots::sanitise_name(name.unwrap_or_default()),
        Some(slot) => slots::label_for(slot, name),
    };
    if label.trim().is_empty() {
        anyhow::bail!("Name can't be empty on a save that has no slot number.");
    }
    rename_label(client, save_id, &label).await
}

/// Error the UI turns into "that number is already in use". Carries the number
/// so it can offer linking to it instead — on the machine that owns it, that is
/// a different folder; in the cloud, it is the folder to pair with.
pub const ERR_SLOT_TAKEN: &str = "slot_taken";

/// Move a folder to another number, keeping whatever name it has.
///
/// Renumbering is how a folder that came out as 3 on the second machine gets
/// paired with the 2 on the first. It only works while the target number is
/// free: if the other machine already has a 2, the row for it already exists in
/// the cloud and *that* row is the one to join — a rename would only collide
/// with it (409 on `UNIQUE(user_id, game_slug, label)`), and joining it means
/// adopting its history, not renaming into its name.
pub async fn renumber(
    client: &ApiClient,
    save_id: &str,
    slot: u32,
) -> Result<(TrackedSave, Option<WatchedSave>)> {
    let current = local_label(save_id)?;
    if slots::slot_of(&current) == Some(slot) {
        anyhow::bail!("That folder is already number {slot}.");
    }
    let label = slots::label_for(slot, slots::name_of(&current));
    match rename_label(client, save_id, &label).await {
        Err(e) if is_conflict(&e) => anyhow::bail!("{ERR_SLOT_TAKEN}:{slot}"),
        other => other,
    }
}

fn local_label(save_id: &str) -> Result<String> {
    let (state, _) = CliState::load_default()?;
    state
        .saves
        .get(save_id)
        .map(|st| st.label.clone())
        .context("That save isn't tracked on this machine.")
}

fn is_conflict(e: &anyhow::Error) -> bool {
    e.downcast_ref::<crate::api::ApiError>()
        .is_some_and(|api| matches!(api, crate::api::ApiError::Conflict(_)))
}

pub async fn rename_label(
    client: &ApiClient,
    save_id: &str,
    new_label: &str,
) -> Result<(TrackedSave, Option<WatchedSave>)> {
    let trimmed = new_label.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Label can't be empty.");
    }
    let updated = client.rename_save_label(save_id, trimmed).await?;

    let (mut cli_state, path) = CliState::load_default()?;
    let (local_path_string, preset, processes, shared_processes, local_cursor, allow_device_local) =
        if let Some(entry) = cli_state.saves.get_mut(save_id) {
            entry.label = updated.label.clone();
            (
                entry.local_path.to_string_lossy().into_owned(),
                entry.preset.clone(),
                entry.processes.clone(),
                entry.shared_processes,
                entry.last_version_num,
                entry.allow_device_local,
            )
        } else {
            (String::new(), None, Vec::new(), false, None, None)
        };
    cli_state.save(&path)?;

    let watched = (!local_path_string.is_empty()).then(|| {
        watched_save_from(
            updated.id.to_string(),
            updated.game_slug.to_string(),
            updated.game_slug.to_string(),
            updated.label.clone(),
            PathBuf::from(&local_path_string),
            preset.as_deref(),
            processes,
            shared_processes,
            allow_device_local,
        )
    });

    Ok((
        TrackedSave {
            save_id: updated.id.into_inner(),
            game_slug: updated.game_slug.into_inner(),
            name: slots::name_of(&updated.label).map(str::to_string),
            slot: slots::slot_of(&updated.label),
            label: updated.label,
            local_path: local_path_string,
            cloud_version_num: updated.latest_version_num,
            local_version_num: local_cursor,
            last_backup_at: None,
            paused: false,
            total_size_bytes: updated.total_size_bytes.unwrap_or(0),
            orphan: false,
            local_size_bytes: None,
            preset,
            allow_device_local,
        },
        watched,
    ))
}

/// Deja de rastrear un save: borra la fila local pero deja los datos del server
/// intactos. El frontend despega el save del agente vivo.
pub fn untrack(save_id: &str) -> Result<()> {
    let (mut cli_state, path) = CliState::load_default()?;
    let dropped = cli_state.saves.remove(save_id);
    cli_state.save(&path)?;
    // Una desmentida: el pipeline propuso esta carpeta y el usuario la echó.
    if let Some(save) = dropped {
        crate::telemetry::untracked(&save.game_slug, &save.local_path);
    }
    Ok(())
}

/// Blacklist a game **and stop tracking it here**, in one state write.
///
/// Blacklisting used to be detection-only: the slug was filtered out of every
/// future scan while the save it named went on being watched, synced and
/// counted as playing. That reads as a bug from the outside — a user whose
/// library had a bogus game blacklisted it, saw nothing change, and had no way
/// to tell that the row doing the damage was a *tracked* one, not a detected
/// one. So the blacklist now means what people take it to mean: this game is
/// not to be watched on this machine.
///
/// Server data is untouched, exactly like [`untrack`]: the snapshots stay, the
/// row can be re-tracked from the Library, and [`unignore_slug`] puts the game
/// back in front of detection. Returns the ids that stopped being tracked so
/// the caller can detach them from the live engine.
pub fn ignore_slug(slug: &str) -> Result<Vec<String>> {
    let slug = slug.trim();
    if slug.is_empty() {
        anyhow::bail!("slug is empty");
    }
    let (mut cli_state, path) = CliState::load_default()?;
    let dropped = ignore_slug_in_state(&mut cli_state, slug);
    cli_state.save(&path)?;

    for (_, save) in &dropped {
        // Same denial `untrack` records: the pipeline proposed this folder and
        // the user threw it out.
        crate::telemetry::untracked(&save.game_slug, &save.local_path);
    }
    Ok(dropped.into_iter().map(|(id, _)| id).collect())
}

/// The state mutation behind [`ignore_slug`], split out so it can be tested
/// without a state file on disk. Returns the rows it dropped.
fn ignore_slug_in_state(
    cli_state: &mut CliState,
    slug: &str,
) -> Vec<(String, crate::state::SaveState)> {
    cli_state.add_ignored_slug(slug.to_string());

    let dropped: Vec<(String, crate::state::SaveState)> = cli_state
        .saves
        .iter()
        .filter(|(_, save)| save.game_slug == slug)
        .map(|(id, save)| (id.clone(), save.clone()))
        .collect();
    for (id, _) in &dropped {
        cli_state.saves.remove(id);
    }
    // The override is what would bounce a re-add straight back to the folder
    // the user just rejected, so it goes with the row.
    if !dropped.is_empty() {
        cli_state.clear_manual_path(slug);
    }
    dropped
}

/// Undo [`ignore_slug`]: the next scan offers the game again. The saves it
/// untracked are **not** restored — re-tracking is the user's call, and the
/// Library offers the game as soon as detection sees it.
pub fn unignore_slug(slug: &str) -> Result<()> {
    let slug = slug.trim();
    if slug.is_empty() {
        anyhow::bail!("slug is empty");
    }
    let (mut cli_state, path) = CliState::load_default()?;
    cli_state.remove_ignored_slug(slug);
    cli_state.save(&path)?;
    Ok(())
}

/// Borrado duro: elimina fila + todos los snapshots del server Y purga el estado
/// local, incluido el override `manual_paths` (para que un re-add no rebote a la
/// carpeta mala). El frontend despega el save del agente vivo.
pub async fn delete_completely(client: &ApiClient, save_id: &str) -> Result<()> {
    client.delete_save(save_id).await?;
    let (mut cli_state, path) = CliState::load_default()?;
    let slug = cli_state.saves.get(save_id).map(|s| s.game_slug.clone());
    cli_state.saves.remove(save_id);
    if let Some(slug) = slug {
        cli_state.clear_manual_path(&slug);
    }
    cli_state.save(&path)?;
    Ok(())
}

// ---- ajustes por-save (pausa / preset / ruta) ------------------------------

/// Qué debe hacer el frontend con el agente EN VIVO tras un cambio de ajustes.
/// La lógica de negocio (mutar estado) ya la hizo la función; el desktop traduce
/// esto a attach/detach sobre su agente in-process. La CLI lo ignora: un daemon
/// en otro proceso recoge el cambio en su próximo arranque.
pub enum LiveReseat {
    /// Deja de vigilar este `save_id`.
    Detach(String),
    /// Empieza a vigilar con este `WatchedSave` (sin detach previo).
    Attach(Box<WatchedSave>),
    /// Despega `save_id` y vuelve a engancharlo con el `WatchedSave` fresco.
    Reseat(String, Box<WatchedSave>),
    /// Nada que hacer (p.ej. save pausado: el agente no lo vigila igualmente).
    Noop,
}

/// Construye el `WatchedSave` fresco desde un snapshot de `SaveState` (reseat
/// tras editar ajustes). Arrastra los process pins persistidos para que un
/// emulador re-enganchado conserve su detección sin esperar un reinicio.
fn watched_from_snapshot(save_id: String, s: &SaveState) -> WatchedSave {
    watched_save_from(
        save_id,
        s.game_slug.clone(),
        s.game_slug.clone(),
        s.label.clone(),
        s.local_path.clone(),
        s.preset.as_deref(),
        s.processes.clone(),
        s.shared_processes,
        s.allow_device_local,
    )
}

/// Pausa/reanuda la vigilancia de un save. Pausado sigue en la lista pero el
/// agente no lo toca (reorganizar ficheros, modding sin backups ruidosos).
pub fn set_paused(save_id: &str, paused: bool) -> Result<LiveReseat> {
    let (mut cli_state, path) = CliState::load_default()?;
    let entry = cli_state
        .saves
        .get_mut(save_id)
        .context("That save isn't tracked on this machine — nothing to pause.")?;
    entry.paused = paused;
    let snapshot = entry.clone();
    cli_state.save(&path)?;

    Ok(if paused {
        LiveReseat::Detach(save_id.to_string())
    } else {
        LiveReseat::Attach(Box::new(watched_from_snapshot(
            save_id.to_string(),
            &snapshot,
        )))
    })
}

/// Fija (o limpia) el preset de sync de un save. `None`/`"standard"` limpia el
/// override a los defaults globales. Reasienta el agente para aplicar la nueva
/// política (intervalo, debounce, restore) en el acto — salvo si está pausado.
pub fn set_preset(save_id: &str, preset: Option<String>) -> Result<LiveReseat> {
    // Normaliza: vacío / "standard" = sin override.
    let preset = preset.filter(|p| !p.is_empty() && p != presets::PRESET_STANDARD);
    if let Some(p) = &preset {
        if !presets::ALL_PRESETS.contains(&p.as_str()) {
            anyhow::bail!("Unknown preset '{p}'.");
        }
    }

    let (mut cli_state, path) = CliState::load_default()?;
    let entry = cli_state
        .saves
        .get_mut(save_id)
        .context("That save isn't tracked on this machine.")?;
    entry.preset = preset;
    let snapshot = entry.clone();
    cli_state.save(&path)?;

    Ok(if snapshot.paused {
        LiveReseat::Noop
    } else {
        LiveReseat::Reseat(
            save_id.to_string(),
            Box::new(watched_from_snapshot(save_id.to_string(), &snapshot)),
        )
    })
}

/// Decide, para este juego, si un restore escribe su config
/// (`FileClass::DeviceLocal`) o la deja pasar.
///
/// `None` devuelve el save a "sin decidir": no se escribe y el diálogo de
/// restore vuelve a preguntar cada vez. Ver
/// [`crate::state::SaveState::allow_device_local`].
/// Writes the decision onto every row of `slug` and returns one **live**
/// (non-paused) row to reseat, if there is one.
///
/// Split out of [`set_allow_device_local`] because that one enters through
/// `CliState::load_default()`, which reads the user's real paths and takes no
/// override: without this seam there is no way to test the per-game spread.
///
/// A paused save is not in the agent, so there is nothing to reseat — it
/// re-reads the state when it resumes, same as `set_preset`. One live row is
/// enough to make the engine reload all of them.
fn spread_allow_device_local(
    cli_state: &mut CliState,
    slug: &str,
    allow: Option<bool>,
) -> Option<(String, SaveState)> {
    let ids: Vec<String> = cli_state
        .saves
        .iter()
        .filter(|(_, s)| s.game_slug == slug)
        .map(|(id, _)| id.clone())
        .collect();

    let mut live: Option<(String, SaveState)> = None;
    for id in &ids {
        let Some(entry) = cli_state.saves.get_mut(id) else {
            continue;
        };
        entry.allow_device_local = allow;
        if !entry.paused && live.is_none() {
            live = Some((id.clone(), entry.clone()));
        }
    }
    live
}

pub fn set_allow_device_local(save_id: &str, allow: Option<bool>) -> Result<LiveReseat> {
    let (mut cli_state, path) = CliState::load_default()?;
    let slug = cli_state
        .saves
        .get(save_id)
        // A cloud-only row reaches this point with an id that is not in
        // `state.json`. That is not user error: this machine simply has no
        // folder to apply the decision to, and the message says so.
        .context(
            "This game isn't tracked on this machine — link a local folder before choosing this.",
        )?
        .game_slug
        .clone();

    // The decision belongs to **the game**, not the folder. The question it
    // answers — does this game's config carry this monitor's resolution, or does
    // it carry the save inside? — has one answer per title, so a game with two
    // tracked folders cannot have two. Answering it on one and leaving the other
    // asking would be the same old trap: the user believes they already said it,
    // and the second folder's automatic restore writes them nothing.
    //
    // It is still stored **row by row** rather than in a per-slug table, because
    // the slug is NOT stable identity (the same game arrives as `vrising` from
    // Steam and `v-rising` from the catalog); the folder is. The slug is only
    // used here to group at the moment of the click.
    let live = spread_allow_device_local(&mut cli_state, &slug, allow);
    cli_state.save(&path)?;

    Ok(match live {
        Some((id, snapshot)) => {
            LiveReseat::Reseat(id.clone(), Box::new(watched_from_snapshot(id, &snapshot)))
        }
        None => LiveReseat::Noop,
    })
}

/// Cambia la ruta local de un save (el usuario movió la carpeta: reinstaló en
/// otro disco, pasó de Steam a GOG…). Crea la carpeta si falta. Reasienta el
/// watcher a la nueva ubicación.
pub fn set_local_path(save_id: &str, new_path: &str) -> Result<LiveReseat> {
    let path_buf = PathBuf::from(new_path.trim());
    if path_buf.as_os_str().is_empty() {
        anyhow::bail!("Path can't be empty.");
    }
    validate_folder(&path_buf, &[save_id])?;

    let (mut cli_state, path) = CliState::load_default()?;
    let entry = cli_state
        .saves
        .get_mut(save_id)
        .context("That save isn't tracked on this machine.")?;
    let previous = std::mem::replace(&mut entry.local_path, path_buf);
    let snapshot = entry.clone();
    cli_state.save(&path)?;

    // De dónde a dónde: la desmentida que más enseña, porque trae la respuesta
    // buena además del fallo. Sólo cuenta si la ruta cambió de verdad.
    if previous != snapshot.local_path {
        crate::telemetry::repointed(&snapshot.game_slug, &previous, &snapshot.local_path);
    }

    // Siempre despega; sólo reengancha si no está pausado.
    Ok(if snapshot.paused {
        LiveReseat::Detach(save_id.to_string())
    } else {
        LiveReseat::Reseat(
            save_id.to_string(),
            Box::new(watched_from_snapshot(save_id.to_string(), &snapshot)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_excluded_paths, auto_track_decision, conflicting_save, detected_paths_in, folder_key,
        local_detection, manual_override_conflict, occupied_slot, prune_poisoned_rows,
        reconcile_plan, resolve_processes, row_for_same_folder, rows_one_per_folder,
        rows_unknown_to_server, spread_allow_device_local, superseded_rows,
        watched_saves_from_state, AutoTrack, CachedDetection, ServerRow, ERR_SLOT_OCCUPIED,
    };
    use crate::detection::{
        Confidence, DetectedGame, DetectionReport, DetectionSource, DetectionStats,
    };
    use crate::state::{CliState, SaveState};
    use std::path::{Path, PathBuf};
    use time::OffsetDateTime;

    fn save_state(slug: &str, path: &str) -> SaveState {
        SaveState {
            local_path: PathBuf::from(path),
            game_slug: slug.to_string(),
            label: "main".to_string(),
            last_backup_at: None,
            last_version_num: None,
            paused: false,
            preset: None,
            allow_device_local: None,
            set_hash: None,
            processes: Vec::new(),
            shared_processes: false,
        }
    }

    // ---- one folder, one watcher --------------------------------------

    /// The bug this exists for (ago-2026, Jurassic World Evolution 3): the same
    /// folder tracked under two save ids — the local one and the id the server
    /// calls canonical for that `(slug, label)` — armed two watchers 70 ms
    /// apart and uploaded the same 5.7 MB twice on every change. The commit
    /// path already redirected both to the canonical id, so nothing looked
    /// broken on the server; the cost was all on the client.
    #[test]
    fn one_folder_gets_one_row_even_under_two_save_ids() {
        let mut state = CliState::default();
        let path = "/home/u/Saved Games/Jurassic World Evolution 3/76561197960287930/Saves";
        state.saves.insert(
            "34fb6027".into(),
            save_state("jurassic-world-evolution-3", path),
        );
        let mut local = save_state("jurassic-world-evolution-3", path);
        local.set_hash = Some("abc:def".into());
        local.last_version_num = Some(1);
        state.saves.insert("b3ebb909".into(), local);

        let kept = rows_one_per_folder(&state);
        assert_eq!(kept.len(), 1, "one folder must not produce two watchers");
        // The row carrying a set-hash wins: it knows what is already synced, so
        // keeping it avoids re-uploading a baseline that is already up there.
        assert_eq!(kept[0].0, "b3ebb909");
    }

    /// The other half of the same rule: a game deliberately tracked in two
    /// different folders is a slot, not a duplicate. This is what the fix must
    /// not break — the report that found the bug came from a machine that has
    /// exactly this (Factorio in `Desktop/saves` and in `AppData/Factorio`).
    #[test]
    fn the_same_game_in_two_folders_keeps_both_rows() {
        let mut state = CliState::default();
        let mut a = save_state("factorio", "/home/u/Desktop/saves");
        a.label = "2".into();
        state.saves.insert("row-a".into(), a);
        state.saves.insert(
            "row-b".into(),
            save_state("factorio", "/home/u/.factorio/saves"),
        );

        assert_eq!(rows_one_per_folder(&state).len(), 2);
    }

    /// And two *different* games that share one folder both keep their row.
    /// Collapsing on the path alone would have stopped backing one of them up,
    /// which is a worse bug than the one being fixed.
    #[test]
    fn two_games_sharing_a_folder_keep_both_rows() {
        let mut state = CliState::default();
        state.saves.insert(
            "row-a".into(),
            save_state("game-one", "/home/u/shared/saves"),
        );
        state.saves.insert(
            "row-b".into(),
            save_state("game-two", "/home/u/shared/saves"),
        );

        assert_eq!(rows_one_per_folder(&state).len(), 2);
    }

    /// A `HashMap`'s iteration order is not stable, so without an explicit
    /// ordering the surviving twin would differ between two passes over the
    /// same file — and the engine would drop and re-arm a different watcher on
    /// every reload. Ordering by `save_id` after the set-hash preference makes
    /// the answer the same every time.
    #[test]
    fn the_surviving_twin_is_the_same_on_every_pass() {
        let path = "/home/u/saves";
        let winner = |ids: [&str; 3]| {
            let mut state = CliState::default();
            for id in ids {
                state.saves.insert(id.to_string(), save_state("g", path));
            }
            let kept = rows_one_per_folder(&state);
            assert_eq!(kept.len(), 1);
            kept[0].0.clone()
        };
        assert_eq!(winner(["ccc", "aaa", "bbb"]), "aaa");
        assert_eq!(winner(["aaa", "bbb", "ccc"]), "aaa");
    }

    /// A paused row is not watched, so it must not be the twin that survives —
    /// otherwise pausing one of two rows on a folder would stop the folder
    /// being backed up at all.
    #[test]
    fn a_paused_twin_never_wins_over_a_live_one() {
        let mut state = CliState::default();
        let path = "/home/u/saves";
        let mut paused = save_state("g", path);
        paused.paused = true;
        paused.set_hash = Some("abc:def".into());
        state.saves.insert("aaa".into(), paused);
        state.saves.insert("zzz".into(), save_state("g", path));

        let kept = rows_one_per_folder(&state);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, "zzz");
    }

    /// `state.json` really does carry both separators in one path — the rows
    /// this machine wrote include `C:\Users\u\Documents\My Games/Fallout4/Saves`.
    /// On Windows that is one directory spelled two ways and must key the same.
    #[test]
    fn mixed_separators_name_the_same_folder_on_windows() {
        let a = Path::new(r"C:\Users\u\Documents\My Games\Fallout4\Saves");
        let b = Path::new(r"C:\Users\u\Documents\My Games/Fallout4/Saves");
        if cfg!(windows) {
            assert_eq!(folder_key(a), folder_key(b));
        } else {
            // On Unix a backslash is a legal filename character, so these are
            // genuinely two different names and folding them would be wrong.
            assert_ne!(folder_key(a), folder_key(b));
        }
    }

    // ---- archived saves are not watched --------------------------------

    /// A save parked in the server-side black box refuses every upload with a
    /// 403 by design. Watching one bought nothing and cost a full re-hash of
    /// its folder on every reconcile — 30 of them in two days on the machine
    /// that reported this — plus a "Backing up…" in the feed that never
    /// resolved, because the archived branch is the one terminal outcome that
    /// emits no event. Treated like `paused`: not watched at all.
    #[test]
    fn an_archived_save_is_not_watched() {
        let mut state = CliState::default();
        state.saves.insert(
            "frozen".into(),
            save_state("fallout-4", "/home/u/fallout4/Saves"),
        );
        state.saves.insert(
            "live".into(),
            save_state("stellaris", "/home/u/stellaris/save games"),
        );

        let archived = std::collections::HashSet::from(["frozen".to_string()]);
        let watched = watched_saves_from_state(&state, &archived);
        assert!(watched.iter().all(|w| w.save_id != "frozen"));
        assert!(watched.iter().any(|w| w.save_id == "live"));
    }

    /// The engine asks the server which saves are frozen, and that question can
    /// go unanswered — no network, a self-hosted server with no black box, a
    /// version that predates the endpoint. All of those mean "I don't know of
    /// any", never "watch nothing": of the two possible mistakes, watching too
    /// much is the cheap one, since a frozen save that slips through is stopped
    /// by the same 403 as before.
    #[test]
    fn an_unanswered_archive_query_watches_everything() {
        let mut state = CliState::default();
        state.saves.insert(
            "a".into(),
            save_state("fallout-4", "/home/u/fallout4/Saves"),
        );

        let watched = watched_saves_from_state(&state, &std::collections::HashSet::new());
        assert!(watched.iter().any(|w| w.save_id == "a"));
    }

    /// Blacklisting is what a user reaches for when a bogus game appears, and
    /// until aug-2026 it only filtered future scans: the row already tracked
    /// under that slug went on being watched, so the user saw nothing change
    /// (report: a phantom game that kept claiming to be running). Now it
    /// untracks too — and takes the manual override with it, or a re-add would
    /// bounce straight back to the rejected folder.
    #[test]
    fn blacklisting_a_slug_also_stops_tracking_it() {
        let mut state = CliState::default();
        state.saves.insert(
            "row-1".into(),
            save_state("storage", "/home/u/Emulation/storage"),
        );
        state
            .saves
            .insert("row-2".into(), save_state("stardew-valley", "/home/u/sv"));
        state.set_manual_path("storage", PathBuf::from("/home/u/Emulation/storage"));

        let dropped = super::ignore_slug_in_state(&mut state, "storage");

        assert_eq!(dropped.len(), 1, "only the blacklisted slug's row goes");
        assert_eq!(dropped[0].0, "row-1");
        assert!(state.is_ignored("storage"));
        assert!(!state.saves.contains_key("row-1"));
        assert!(
            state.saves.contains_key("row-2"),
            "another game's row is untouched"
        );
        assert!(
            !state.manual_paths.contains_key("storage"),
            "the override goes with the row"
        );

        // Idempotent: blacklisting again finds nothing left to untrack.
        assert!(super::ignore_slug_in_state(&mut state, "storage").is_empty());
    }

    /// A slug with nothing tracked under it keeps the old behaviour exactly:
    /// blacklist the name, touch no row, and leave any override alone (it
    /// belongs to a folder the user picked, not to a row we just dropped).
    #[test]
    fn blacklisting_an_untracked_slug_only_blacklists() {
        let mut state = CliState::default();
        state
            .saves
            .insert("row-1".into(), save_state("stardew-valley", "/home/u/sv"));
        state.set_manual_path("some-game", PathBuf::from("/home/u/some-game"));

        assert!(super::ignore_slug_in_state(&mut state, "some-game").is_empty());
        assert!(state.is_ignored("some-game"));
        assert_eq!(state.saves.len(), 1);
        assert!(state.manual_paths.contains_key("some-game"));
    }

    /// The aug-2026 Factorio incident, as a test.
    ///
    /// The user pointed at a loose desktop folder for a game already tracked at
    /// its real save folder. The add reused the (game, "main") row, overwrote
    /// its path, and left the real folder silently unsynced. Now the same move
    /// is an error carrying everything the UI needs to ask what was meant.
    #[test]
    fn a_second_folder_never_moves_the_slot_in_silence() {
        let real = "/home/rl261/.factorio/saves";
        let desktop = "/home/rl261/Desktop/saves";
        let mut state = CliState::default();
        state
            .saves
            .insert("factorio-1".into(), save_state("factorio", real));

        let err = occupied_slot(&state, "factorio", "main", Path::new(desktop), false)
            .expect_err("slot 1 is held by another folder");
        let msg = err.to_string();
        assert!(msg.starts_with(ERR_SLOT_OCCUPIED), "{msg}");
        assert!(
            msg.ends_with(real),
            "carries the folder that is there now: {msg}"
        );
        assert!(msg.contains(":2:"), "offers the lowest free number: {msg}");

        // Saying yes to the move is what makes it go through.
        occupied_slot(&state, "factorio", "main", Path::new(desktop), true)
            .expect("an explicit re-point is still allowed");
    }

    /// Naming a folder must not mint a second save for it. The local row said
    /// `"2 - shit"`, the add composed `"2 · shit2"`, and matching on the text
    /// meant no row matched — so out came a fresh uuid and a third cloud row for
    /// one folder, aug-2026.
    #[test]
    fn a_renamed_slot_is_still_the_same_slot() {
        let mut state = CliState::default();
        let mut row = save_state("factorio", "/home/rl261/Desktop/sx");
        row.label = "2 - shit".into();
        state.saves.insert("existente".into(), row);

        // Same slot under a different name: no clash, it is that folder.
        occupied_slot(
            &state,
            "factorio",
            "2 · shit2",
            Path::new("/home/rl261/Desktop/sx"),
            false,
        )
        .expect("renaming slot 2 is not a second folder");

        // And a genuinely different folder on that slot still gets stopped.
        let err = occupied_slot(
            &state,
            "factorio",
            "2 · otra",
            Path::new("/home/rl261/Desktop/otra"),
            false,
        )
        .expect_err("slot 2 is taken by another folder");
        assert!(err.to_string().starts_with(ERR_SLOT_OCCUPIED));
    }

    /// Re-adding the very same folder is a re-track, not a move: it must not
    /// stop to ask anything.
    #[test]
    fn re_adding_the_same_folder_is_not_a_clash() {
        let real = "/home/rl261/.factorio/saves";
        let mut state = CliState::default();
        state
            .saves
            .insert("factorio-1".into(), save_state("factorio", real));

        occupied_slot(&state, "factorio", "main", Path::new(real), false)
            .expect("same folder, same slot");
        // And an empty slot has nothing to clash with.
        occupied_slot(&state, "factorio", "2", Path::new("/tmp/config"), false)
            .expect("slot 2 is free");
    }

    /// Un informe de detección con un solo juego en una sola carpeta.
    fn report_with(slug: &str, path: &str) -> DetectionReport {
        DetectionReport {
            games: vec![DetectedGame {
                slug: slug.to_string(),
                display_name: slug.to_string(),
                found_paths: vec![PathBuf::from(path)],
                confidence: Confidence::High,
                path_confidences: vec![Confidence::High],
                path_reasons: vec![String::new()],
                source: DetectionSource::FilesystemHeuristic,
                steam_app_id: None,
                install_dir: None,
                needs_folder: false,
                steam_cloud: false,
            }],
            catalog_size: 0,
            steam_apps_found: 0,
            scanned_at_ms: 0,
            stats: DetectionStats::default(),
            mirror_warnings: Vec::new(),
        }
    }

    /// El informe de ago-2026 (Furi, Windows + Steam Deck): la máquina ya tenía
    /// el juego dado de alta con un id local y la nube traía el suyo. Adoptar
    /// excluía sólo el id de la nube, así que la entrada local **chocaba contra
    /// sí misma** y saltaba «'furi' already tracks … — one folder, one game».
    /// El escaneo automático lo reintentaba en cada vuelta, el "+" manual daba
    /// lo mismo y reapuntar la carpeta también: cero salidas por la UI.
    #[test]
    fn adopting_a_cloud_save_doesnt_collide_with_this_machines_own_row() {
        let folder = r"C:\Users\angel\AppData\LocalLow\TheGameBakers\Furi";
        let mut state = CliState::default();
        state
            .saves
            .insert("local-minted-id".into(), save_state("furi", folder));

        // Excluyendo sólo el id de la nube: la fila local se interpone.
        let blocked = conflicting_save(&state, &PathBuf::from(folder), &["cloud-id"]);
        assert_eq!(
            blocked.map(|s| s.game_slug.as_str()),
            Some("furi"),
            "reproduce el bug: sin relevar la fila local, choca consigo misma"
        );

        // Excluyendo ambas —lo que hace `adopt` ahora— la adopción pasa.
        assert!(
            conflicting_save(
                &state,
                &PathBuf::from(folder),
                &["cloud-id", "local-minted-id"]
            )
            .is_none(),
            "el mismo juego en la misma carpeta no es un conflicto consigo mismo"
        );

        // Y la regla que de verdad importa sigue en pie: OTRO juego sobre la
        // misma carpeta se rechaza igual.
        state
            .saves
            .insert("otro".into(), save_state("skyrim", folder));
        assert_eq!(
            conflicting_save(
                &state,
                &PathBuf::from(folder),
                &["cloud-id", "local-minted-id"]
            )
            .map(|s| s.game_slug.as_str()),
            Some("skyrim"),
            "«una carpeta, un juego» sigue protegiendo contra juegos distintos"
        );
    }

    /// El informe self-hosted de ago-2026: rehizo el servidor de cero y desde
    /// entonces el escaneo fallaba para ~40 juegos con «ya la rastrea», siempre
    /// contra el MISMO slug y la MISMA carpeta —o sea, contra su propio gemelo—.
    /// El id lo pone el servidor y la base nueva repartió otros; el alta
    /// insertaba el nuevo sin quitar el viejo.
    #[test]
    fn a_self_hosted_add_supersedes_the_row_it_replaces() {
        let folder = r"D:\SteamUnlock\userdata\866681748\3768760\remote";
        let mut state = CliState::default();
        state.saves.insert(
            "id-de-la-base-vieja".into(),
            save_state("007-first-light", folder),
        );

        assert_eq!(
            superseded_rows(
                &state,
                "007-first-light",
                "main",
                &PathBuf::from(folder),
                "id-de-la-base-nueva"
            ),
            vec!["id-de-la-base-vieja".to_string()],
            "la fila vieja se releva; si no, bloquea su propia carpeta para siempre"
        );

        // Y no se lleva por delante a nadie más: otro juego, otra carpeta.
        state.saves.insert(
            "otro".into(),
            save_state("thymesia", r"C:\Users\angel\AppData\Roaming\FLT\1343240"),
        );
        let relevadas = superseded_rows(
            &state,
            "007-first-light",
            "main",
            &PathBuf::from(folder),
            "id-de-la-base-nueva",
        );
        assert_eq!(relevadas, vec!["id-de-la-base-vieja".to_string()]);

        // Re-alta idéntica (mismo id): no hay nada que relevar.
        assert!(superseded_rows(
            &state,
            "007-first-light",
            "main",
            &PathBuf::from(folder),
            "id-de-la-base-vieja"
        )
        .is_empty());
    }

    /// El caso de ago-2026 en la máquina del autor: `horizon-forbidden-west`
    /// quedó apuntado a `…\Saved Games\Surviving Mars Relaunched` —la carpeta
    /// madre de los saves de Surviving Mars—, así que Horizon vigilaba bytes
    /// ajenos y Surviving Mars no podía rastrear los suyos. El override manual
    /// no lo borra ni desinstalar con "borrar datos": vive en `device.json`.
    #[test]
    fn a_manual_override_cant_steal_another_games_folder() {
        // Rutas POSIX aunque el caso real fuese en Windows: en un runner Linux
        // una ruta con `\` es UN componente, así que el anidamiento —que es lo
        // que se está probando— no existiría. La regla es la misma en ambos.
        let madre = "/home/u/Saved Games/Surviving Mars Relaunched";
        let hija = "/home/u/Saved Games/Surviving Mars Relaunched/76561197960271";
        let state = CliState::default();
        let report = report_with("surviving-mars-relaunched", hija);

        assert_eq!(
            manual_override_conflict(
                &state,
                Some(&report),
                "horizon-forbidden-west",
                &PathBuf::from(madre)
            )
            .as_deref(),
            Some("surviving-mars-relaunched"),
            "la carpeta madre contiene los saves de otro juego: no es de Horizon"
        );

        // Reapuntar el juego a SU propia carpeta sigue siendo legítimo —es para
        // lo que existe el override— y una carpeta que no reclama nadie también.
        assert!(manual_override_conflict(
            &state,
            Some(&report),
            "surviving-mars-relaunched",
            &PathBuf::from(madre)
        )
        .is_none());
        assert!(manual_override_conflict(
            &state,
            Some(&report),
            "horizon-forbidden-west",
            &PathBuf::from("/home/u/Saved Games/Horizon Forbidden West")
        )
        .is_none());
    }

    /// El otro árbitro: una fila ya rastreada, sin necesidad de caché de
    /// detección (recién borrada, primer arranque…).
    #[test]
    fn a_manual_override_respects_whats_already_tracked() {
        let folder = "/home/u/Saved Games/Planet S/saves";
        let mut state = CliState::default();
        state
            .saves
            .insert("id".into(), save_state("planet-s", folder));

        assert_eq!(
            manual_override_conflict(&state, None, "otro-juego", &PathBuf::from(folder)).as_deref(),
            Some("planet-s")
        );
        assert!(
            manual_override_conflict(&state, None, "planet-s", &PathBuf::from(folder)).is_none(),
            "reapuntar el mismo juego no choca consigo mismo"
        );
    }

    /// El alta automática espera a que haya algo que guardar. Los cuatro casos
    /// que decide, sobre carpetas de verdad.
    #[test]
    fn empty_folders_wait_but_nothing_else_does() {
        let tmp = tempfile::tempdir().unwrap();

        // 1. Vacía del todo: espera.
        let vacia = tmp.path().join("magicka-2");
        std::fs::create_dir_all(&vacia).unwrap();
        assert_eq!(auto_track_decision(&vacia, false), AutoTrack::SkipEmpty);

        // 2. Vacía pero el servidor ya tiene el juego: es una máquina nueva
        //    esperando restaurar. Se da de alta igual — el caso que NO se puede
        //    romper por quitar ruido.
        assert_eq!(auto_track_decision(&vacia, true), AutoTrack::Track);

        // 3. Con un fichero dentro: se da de alta.
        let conmigo = tmp.path().join("celeste");
        std::fs::create_dir_all(&conmigo).unwrap();
        std::fs::write(conmigo.join("save0.celeste"), b"x").unwrap();
        assert_eq!(auto_track_decision(&conmigo, false), AutoTrack::Track);

        // 4. El fichero está en un subdirectorio —la forma real de Goldberg,
        //    `<appid>/remote/…`—: cuenta igual.
        let anidada = tmp.path().join("962130");
        std::fs::create_dir_all(anidada.join("remote")).unwrap();
        std::fs::write(anidada.join("remote/profile.dat"), b"x").unwrap();
        assert_eq!(auto_track_decision(&anidada, false), AutoTrack::Track);

        // 5. Sólo subcarpetas vacías: sigue sin haber nada que guardar.
        let hueca = tmp.path().join("hueca");
        std::fs::create_dir_all(hueca.join("remote")).unwrap();
        assert_eq!(auto_track_decision(&hueca, false), AutoTrack::SkipEmpty);

        // 6. Y lo que hace que esperar no sea perder: en cuanto el juego
        //    escribe, el escaneo siguiente la da de alta. No se aplaza nada
        //    durable, se re-decide cada vuelta.
        std::fs::write(vacia.join("Player.sav"), b"x").unwrap();
        assert_eq!(auto_track_decision(&vacia, false), AutoTrack::Track);
    }

    /// Ante la duda, dar de alta. Una carpeta que ni siquiera existe no se puede
    /// leer, y ese `Err` no puede convertirse en "no la vigiles".
    #[test]
    fn an_unreadable_folder_is_never_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let fantasma = tmp.path().join("no-existe");
        assert_eq!(auto_track_decision(&fantasma, false), AutoTrack::Track);

        // Un save de fichero suelto tampoco es una carpeta vacía.
        let suelto = tmp.path().join("partida.sav");
        std::fs::write(&suelto, b"x").unwrap();
        assert_eq!(auto_track_decision(&suelto, false), AutoTrack::Track);
    }

    /// La reparación automática: al arrancar el motor —o sea, al actualizar—
    /// las filas con ids que el servidor ya no conoce se re-apuntan a la fila
    /// que ese servidor tenga hoy para el mismo (juego, etiqueta), y sólo se
    /// tiran las que no tienen equivalente. El caso real: doctorase rehízo su
    /// servidor y furi acabó con DOS ids muertos sobre la misma carpeta.
    #[test]
    fn a_reissued_server_row_relinks_instead_of_dropping() {
        let furi = "/home/angel/AppData/LocalLow/TheGameBakers/Furi";
        let mut state = CliState::default();
        state
            .saves
            .insert("furi-viejo".into(), save_state("furi", furi));
        state
            .saves
            .insert("furi-gemelo".into(), save_state("furi", furi));
        state
            .saves
            .insert("007-viejo".into(), save_state("007-first-light", "/d/007"));

        let server = vec![ServerRow {
            id: "furi-nuevo".into(),
            game_slug: "furi".into(),
            label: "main".into(),
        }];

        let plan = reconcile_plan(&state, &server);
        assert_eq!(
            plan,
            vec![
                ("007-viejo".to_string(), None),
                ("furi-gemelo".to_string(), Some("furi-nuevo".to_string())),
                ("furi-viejo".to_string(), Some("furi-nuevo".to_string())),
            ],
            "las dos filas de furi convergen en el id nuevo; 007 no existe en el servidor y se tira"
        );

        // Nada que hacer cuando el servidor conoce lo que hay.
        let server = vec![ServerRow {
            id: "furi-viejo".into(),
            game_slug: "furi".into(),
            label: "main".into(),
        }];
        let mut solo_furi = CliState::default();
        solo_furi
            .saves
            .insert("furi-viejo".into(), save_state("furi", furi));
        assert!(reconcile_plan(&solo_furi, &server).is_empty());
    }

    /// La salida sola: en self-hosted el servidor es el registro, así que una
    /// fila con un id que él no conoce sólo puede dar 404 al subir, no se pinta
    /// en la biblioteca —y aun así bloquea su carpeta—. Se poda al listar.
    #[test]
    fn rows_the_server_never_heard_of_are_pruned() {
        let mut state = CliState::default();
        state
            .saves
            .insert("viva".into(), save_state("thymesia", "/games/thymesia"));
        state
            .saves
            .insert("fantasma".into(), save_state("furi", "/games/furi"));

        let known: std::collections::HashSet<String> = ["viva".to_string()].into_iter().collect();
        assert_eq!(
            rows_unknown_to_server(&state, &known),
            vec!["fantasma".to_string()]
        );

        // Servidor con todo: no se toca nada.
        let known: std::collections::HashSet<String> = ["viva".to_string(), "fantasma".to_string()]
            .into_iter()
            .collect();
        assert!(rows_unknown_to_server(&state, &known).is_empty());
    }

    /// Los otros dos del mismo informe, que el arreglo por slug NO cubría: el
    /// mismo juego llega con nombres distintos según la fuente y la regla de
    /// "una carpeta, un juego" los trataba como juegos distintos.
    ///
    ///   slug=dispatch  ↔ fila `dispatch-2025`  (…\Dispatch\Saved\SaveGames)
    ///   slug=v-rising  ↔ fila `vrising`        (…\VRising\Saves)
    ///
    /// La identidad de un save rastreado es la carpeta, no cómo se llame.
    #[test]
    fn the_same_folder_is_the_same_save_however_the_slug_is_spelled() {
        let dispatch = r"C:\Users\angel\AppData\Local\Dispatch\Saved\SaveGames";
        let vrising = r"C:\Users\angel\AppData\LocalLow\Stunlock Studios\VRising\Saves";
        let mut state = CliState::default();
        state
            .saves
            .insert("row-dispatch".into(), save_state("dispatch-2025", dispatch));
        state
            .saves
            .insert("row-vrising".into(), save_state("vrising", vrising));

        assert_eq!(
            row_for_same_folder(&state, &PathBuf::from(dispatch)),
            Some("row-dispatch"),
            "la carpeta identifica la fila aunque el slug lleve el año"
        );
        assert_eq!(
            row_for_same_folder(&state, &PathBuf::from(vrising)),
            Some("row-vrising"),
            "…y aunque el slug lleve o no el guion"
        );

        // Relevando esa fila, el alta bajo el nombre nuevo ya no choca.
        assert!(
            conflicting_save(&state, &PathBuf::from(dispatch), &["row-dispatch"]).is_none(),
            "reusar la fila de la misma carpeta desbloquea el alta"
        );

        // Pero una carpeta ANIDADA sigue siendo el conflicto legítimo: ahí no
        // hay una fila que reusar, hay dos ámbitos distintos y hay que avisar.
        //
        // Con ruta POSIX a propósito: `paths_overlap` compara por COMPONENTES, y
        // en un runner Linux una ruta con backslashes es un único componente, así
        // que dos rutas Windows nunca anidarían aquí. En producción no importa
        // —esas rutas sólo existen en Windows, donde sí anidan— pero el test
        // tiene que probar el anidamiento de verdad, no un artefacto del host.
        let base = "/home/u/.local/share/Dispatch/Saved/SaveGames";
        let mut posix = CliState::default();
        posix
            .saves
            .insert("row-dispatch".into(), save_state("dispatch-2025", base));
        let nested = PathBuf::from(format!("{base}/Slot1"));
        assert!(
            row_for_same_folder(&posix, &nested).is_none(),
            "anidada no es la misma carpeta"
        );
        assert!(
            conflicting_save(&posix, &nested, &[]).is_some(),
            "y sigue denunciándose como solape"
        );
    }

    #[test]
    fn prune_poisoned_rows_drops_app_named_rows_sharing_a_tracked_folder() {
        // El informe de jul-2026: ChatGPT/opencode/code rastreados los tres
        // sobre la carpeta de Planet S porque la atribución de la correlación
        // cambió entre escaneos y cada nombre dio un slug nuevo.
        let folder = "/home/u/Documentos/Saved Games/PlanetS";
        let mut state = CliState::default();
        state
            .saves
            .insert("a-planet".into(), save_state("planet-s", folder));
        state
            .saves
            .insert("b-chatgpt".into(), save_state("chatgpt", folder));
        state
            .saves
            .insert("c-opencode".into(), save_state("opencode", folder));
        state
            .saves
            .insert("d-code".into(), save_state("code", folder));

        let pruned = prune_poisoned_rows(&mut state);

        assert_eq!(pruned, vec!["b-chatgpt", "c-opencode", "d-code"]);
        assert_eq!(state.saves.len(), 1);
        assert!(state.saves.contains_key("a-planet"));
    }

    #[test]
    fn prune_poisoned_rows_keeps_rows_no_real_game_covers() {
        // Sin un juego que cubra la carpeta no se poda: la lista negra casa por
        // substring, así que podar sólo por nombre se comería el juego "Hoard".
        let mut state = CliState::default();
        state.saves.insert(
            "a".into(),
            save_state("chatgpt", "/home/u/Saved Games/PlanetS"),
        );
        state
            .saves
            .insert("b".into(), save_state("hoard", "/home/u/Saved Games/Hoard"));

        assert!(prune_poisoned_rows(&mut state).is_empty());
        assert_eq!(state.saves.len(), 2);
    }

    /// The question this answers ("does this game's config carry the monitor's
    /// resolution, or the save itself?") has one answer per title, so a game
    /// tracked in two folders must not end up with two. Answering it on one
    /// folder and leaving the other asking is how the user believes they said
    /// it while the second folder's automatic restore keeps writing nothing.
    #[test]
    fn allowing_config_covers_every_folder_of_that_game_only() {
        let mut state = CliState::default();
        state.saves.insert(
            "a".into(),
            save_state("factorio", "/home/u/.factorio/saves"),
        );
        state
            .saves
            .insert("b".into(), save_state("factorio", "/home/u/Desktop/saves"));
        state
            .saves
            .insert("c".into(), save_state("stardew-valley", "/home/u/sdv"));

        let live = spread_allow_device_local(&mut state, "factorio", Some(true));

        assert_eq!(state.saves["a"].allow_device_local, Some(true));
        assert_eq!(
            state.saves["b"].allow_device_local,
            Some(true),
            "la otra carpeta del mismo juego"
        );
        assert_eq!(
            state.saves["c"].allow_device_local, None,
            "otro juego, intacto"
        );
        let (id, _) = live.expect("hay filas vivas que reasentar");
        assert!(id == "a" || id == "b");
    }

    /// A paused save is not in the agent, so it must not be picked as the row to
    /// reseat — but it still has to receive the flag, or resuming it would
    /// silently drop the decision.
    #[test]
    fn allowing_config_still_writes_paused_rows_but_reseats_a_live_one() {
        let mut state = CliState::default();
        let mut paused = save_state("factorio", "/home/u/.factorio/saves");
        paused.paused = true;
        state.saves.insert("paused".into(), paused);
        state.saves.insert(
            "live".into(),
            save_state("factorio", "/home/u/Desktop/saves"),
        );

        let live = spread_allow_device_local(&mut state, "factorio", Some(true));

        assert_eq!(state.saves["paused"].allow_device_local, Some(true));
        assert_eq!(live.map(|(id, _)| id).as_deref(), Some("live"));
    }

    /// Every row paused = nothing to reseat, and `set_allow_device_local` turns
    /// that into `LiveReseat::Noop` instead of waking the engine for nothing.
    #[test]
    fn allowing_config_on_an_all_paused_game_has_nothing_to_reseat() {
        let mut state = CliState::default();
        let mut paused = save_state("factorio", "/home/u/.factorio/saves");
        paused.paused = true;
        state.saves.insert("only".into(), paused);

        assert!(spread_allow_device_local(&mut state, "factorio", Some(true)).is_none());
        assert_eq!(state.saves["only"].allow_device_local, Some(true));
    }

    #[test]
    fn prune_poisoned_rows_covers_nested_folders_too() {
        // La fila envenenada puede colgar de la del juego (el walk de fase 4
        // emite subcarpetas), no sólo coincidir exactamente.
        let mut state = CliState::default();
        state
            .saves
            .insert("a".into(), save_state("planet-s", "/home/u/Saves/PlanetS"));
        state.saves.insert(
            "b".into(),
            save_state("chatgpt", "/home/u/Saves/PlanetS/profile1"),
        );

        assert_eq!(prune_poisoned_rows(&mut state), vec!["b"]);
    }

    fn game(
        slug: &str,
        paths: &[&str],
        per_path: &[Confidence],
        rolled: Confidence,
    ) -> DetectedGame {
        DetectedGame {
            slug: slug.to_string(),
            display_name: slug.to_string(),
            found_paths: paths.iter().map(PathBuf::from).collect(),
            confidence: rolled,
            path_confidences: per_path.to_vec(),
            path_reasons: vec![String::new(); per_path.len()],
            source: DetectionSource::FilesystemHeuristic,
            steam_app_id: None,
            install_dir: None,
            needs_folder: false,
            steam_cloud: false,
        }
    }

    fn report(games: Vec<DetectedGame>) -> DetectionReport {
        DetectionReport {
            games,
            catalog_size: 0,
            steam_apps_found: 0,
            scanned_at_ms: 0,
            stats: DetectionStats::default(),
            mirror_warnings: Vec::new(),
        }
    }

    fn cached(games: Vec<DetectedGame>) -> CachedDetection {
        CachedDetection {
            report: report(games),
            scanned_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn pairs_each_path_with_its_own_confidence() {
        let r = report(vec![game(
            "stardew-valley",
            &["/saves/sdv", "/steam/cloud/sdv"],
            &[Confidence::High, Confidence::Low],
            Confidence::High,
        )]);
        let paths = detected_paths_in(&r, "stardew-valley");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, PathBuf::from("/saves/sdv"));
        assert_eq!(paths[0].confidence, Confidence::High);
        // El stub casi vacío de Steam Cloud NO hereda la High del juego.
        assert_eq!(paths[1].confidence, Confidence::Low);
    }

    #[test]
    fn falls_back_to_rolled_up_confidence_on_old_caches() {
        // Caché escrita por un build sin `path_confidences`: la ruta se
        // conserva con la confianza del juego en vez de perderse.
        let r = report(vec![game(
            "hollow-knight",
            &["/saves/hk"],
            &[],
            Confidence::Medium,
        )]);
        let paths = detected_paths_in(&r, "hollow-knight");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].confidence, Confidence::Medium);
    }

    #[test]
    fn unknown_slug_yields_nothing() {
        let r = report(vec![game(
            "celeste",
            &["/saves/celeste"],
            &[],
            Confidence::High,
        )]);
        assert!(detected_paths_in(&r, "hades").is_empty());
    }

    #[test]
    fn unambiguous_only_when_exactly_one_path() {
        let one = cached(vec![game(
            "celeste",
            &["/saves/celeste"],
            &[],
            Confidence::High,
        )]);
        let d = local_detection(Some(&one), "celeste", &[]);
        assert_eq!(
            d.unambiguous().unwrap().path,
            PathBuf::from("/saves/celeste")
        );

        let two = cached(vec![game(
            "celeste",
            &["/a", "/b"],
            &[Confidence::High, Confidence::Medium],
            Confidence::High,
        )]);
        // Dos candidatas: elige el usuario, la card no ofrece atajo.
        assert!(local_detection(Some(&two), "celeste", &[])
            .unambiguous()
            .is_none());

        let none = cached(vec![game("celeste", &[], &[], Confidence::High)]);
        assert!(local_detection(Some(&none), "celeste", &[])
            .unambiguous()
            .is_none());
    }

    /// Mismo helper que [`game`] pero con nombre visible propio: el parecido se
    /// mide contra el nombre, no solo contra el slug.
    fn named(slug: &str, display: &str, paths: &[&str]) -> DetectedGame {
        DetectedGame {
            display_name: display.to_string(),
            ..game(slug, paths, &[Confidence::High; 1], Confidence::High)
        }
    }

    /// El caso del informe de jul-2026: la misma copia de un juego rastreada en
    /// dos equipos que la nombran distinto. El slug de la nube no casa con
    /// ninguno local, y antes de esto la única salida era el selector de
    /// carpetas — cazar a mano una ruta que la detección ya tenía.
    #[test]
    fn offers_other_detected_games_when_the_slug_doesnt_match() {
        let c = cached(vec![
            named("raccoin-gog", "Raccoin", &["/home/u/.local/share/raccoin"]),
            named("celeste", "Celeste", &["/saves/celeste"]),
        ]);
        let d = local_detection(Some(&c), "raccoin", &[]);
        // Nada bajo ese slug exacto…
        assert!(d.paths.is_empty());
        // …pero sí un juego que se llama igual, y va primero.
        assert_eq!(d.candidates.len(), 2);
        assert_eq!(d.candidates[0].game_slug, "raccoin-gog");
        assert_eq!(d.candidates[0].affinity, 2);
        assert_eq!(
            d.candidates[0].paths[0].path,
            PathBuf::from("/home/u/.local/share/raccoin")
        );
        assert_eq!(d.candidates[1].affinity, 0);
    }

    /// Una carpeta que ya rastrea otro save no se ofrece: dos saves sobre una
    /// misma carpeta es justo lo que el escaneo automático evita.
    #[test]
    fn candidates_skip_already_tracked_folders() {
        let c = cached(vec![
            named("celeste", "Celeste", &["/saves/celeste"]),
            named("hades", "Hades", &["/saves/hades"]),
        ]);
        let d = local_detection(Some(&c), "raccoin", &[PathBuf::from("/saves/celeste")]);
        assert_eq!(d.candidates.len(), 1);
        assert_eq!(d.candidates[0].game_slug, "hades");
    }

    /// Sin carpeta que ofrecer no hay candidata, y el propio slug no se
    /// duplica: ese ya sale en `paths`.
    #[test]
    fn candidates_exclude_pathless_games_and_the_slug_itself() {
        let c = cached(vec![
            game("celeste", &["/saves/celeste"], &[], Confidence::High),
            game("hades", &[], &[], Confidence::High),
        ]);
        let d = local_detection(Some(&c), "celeste", &[]);
        assert_eq!(d.paths.len(), 1);
        assert!(d.candidates.is_empty());
    }

    /// La contención pide 4 caracteres —si no, un nombre corto se declara
    /// pariente de media biblioteca—, pero la IGUALDAD no mide longitud: «Ori»
    /// es «ori» por corto que sea.
    #[test]
    fn short_names_match_exactly_but_never_by_containment() {
        let c = cached(vec![
            named("origin-story", "Origin Story", &["/saves/origin"]),
            named("ori-and-the-blind-forest", "Ori", &["/saves/ori"]),
        ]);
        let d = local_detection(Some(&c), "ori", &[]);
        assert_eq!(d.candidates[0].display_name, "Ori");
        assert_eq!(d.candidates[0].affinity, 2);
        // «ori» dentro de «originstory» NO cuenta: bajo 4 caracteres la
        // contención empareja demasiado.
        assert_eq!(d.candidates[1].display_name, "Origin Story");
        assert_eq!(d.candidates[1].affinity, 0);
    }

    #[test]
    fn never_scanned_is_distinct_from_scanned_and_empty() {
        // Sin caché: no lo sabemos ⇒ el frontend ofrece escanear.
        let cold = local_detection(None, "celeste", &[]);
        assert!(cold.scanned_at.is_none());
        assert!(cold.paths.is_empty());

        // Con caché pero sin el slug: sí lo sabemos, y la respuesta es "nada".
        let scanned = local_detection(Some(&cached(vec![])), "celeste", &[]);
        assert!(scanned.scanned_at.is_some());
        assert!(scanned.paths.is_empty());
    }

    /// El manifiesto declara el ejecutable de ~18k juegos; antes de cablearlo
    /// esto devolvía lista vacía para todo salvo minecraft y factorio, y la
    /// primera sesión de un juego nunca disparaba "arrancó".
    #[test]
    fn processes_come_from_the_manifest_launch_block() {
        let procs = resolve_processes("stardew-valley");
        assert!(
            procs.iter().any(|p| p.contains("stardew")),
            "expected the manifest executable, got {procs:?}"
        );
    }

    /// El catálogo built-in no se pierde al añadir el manifiesto, y no duplica.
    #[test]
    fn builtin_processes_survive_and_dont_duplicate() {
        let procs = resolve_processes("factorio");
        assert!(procs.iter().any(|p| p == "factorio.exe"));
        assert!(procs.iter().any(|p| p == "factorio"));
        let mut sorted = procs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), procs.len(), "duplicados en {procs:?}");
    }

    /// Un slug que no está en el catálogo no inventa procesos.
    #[test]
    fn an_unknown_slug_yields_no_processes() {
        assert!(resolve_processes("not-a-real-game-slug-xyzzy").is_empty());
    }

    fn excl_game(slug: &str, paths: &[&str]) -> DetectedGame {
        DetectedGame {
            slug: slug.into(),
            display_name: slug.into(),
            found_paths: paths.iter().map(PathBuf::from).collect(),
            path_confidences: vec![Confidence::High; paths.len()],
            path_reasons: vec![String::new(); paths.len()],
            confidence: Confidence::High,
            source: DetectionSource::FilesystemHeuristic,
            steam_app_id: None,
            install_dir: None,
            needs_folder: false,
            steam_cloud: false,
        }
    }

    fn report_of(games: Vec<DetectedGame>) -> DetectionReport {
        DetectionReport {
            games,
            catalog_size: 0,
            steam_apps_found: 0,
            scanned_at_ms: 0,
            stats: Default::default(),
            mirror_warnings: Vec::new(),
        }
    }

    /// Regresión (Windows, 30-jul-2026): el filtro de exclusión borraba también
    /// las filas que NUNCA tuvieron rutas — que son justo la alerta ámbar "elige
    /// carpeta". El usuario perdía la única forma de arreglar esos juegos.
    #[test]
    fn excluding_paths_never_removes_a_pick_a_folder_row() {
        let mut state = CliState::default();
        state.add_excluded_path(PathBuf::from("/junk"));
        let mut report = report_of(vec![
            excl_game("sin-rutas", &[]),                 // alerta ámbar: se queda
            excl_game("todo-descartado", &["/junk/x"]),  // pierde todo: fuera
            excl_game("parcial", &["/junk/y", "/real"]), // conserva la buena
            excl_game("intacto", &["/real/z"]),
        ]);
        apply_excluded_paths(&mut report, &state);

        let slugs: Vec<&str> = report.games.iter().map(|g| g.slug.as_str()).collect();
        assert_eq!(slugs, ["sin-rutas", "parcial", "intacto"]);
        let parcial = &report.games[1];
        assert_eq!(parcial.found_paths, vec![PathBuf::from("/real")]);
        assert_eq!(parcial.path_confidences.len(), 1);
    }

    /// Sin exclusiones el informe no se toca en absoluto.
    #[test]
    fn no_exclusions_is_a_no_op() {
        let before = report_of(vec![excl_game("a", &[]), excl_game("b", &["/x"])]);
        let mut after = report_of(vec![excl_game("a", &[]), excl_game("b", &["/x"])]);
        apply_excluded_paths(&mut after, &CliState::default());
        assert_eq!(after.games.len(), before.games.len());
        assert_eq!(after.games[1].found_paths, before.games[1].found_paths);
    }
}

#[cfg(test)]
mod slug_gate_tests {
    use super::reject_degenerate_slug;

    /// Los catorce saves basura que llegaron a producción se llamaban así.
    #[test]
    fn plumbing_never_gets_tracked() {
        for bad in [
            "user",
            "users",
            "desktop",
            "appdata",
            "roaming",
            "documents",
        ] {
            assert!(
                reject_degenerate_slug(bad).is_err(),
                "'{bad}' should never be trackable as a game"
            );
        }
    }

    #[test]
    fn real_games_pass() {
        for good in [
            "stardew-valley",
            "factorio",
            "the-witcher-3-wild-hunt",
            "hearts-of-iron-iv",
        ] {
            assert!(reject_degenerate_slug(good).is_ok(), "'{good}' is a game");
        }
    }

    /// El veredicto no puede depender de esta máquina: la identidad de un save
    /// es la misma en todos los equipos de la cuenta. `insider` es el usuario
    /// de la máquina de desarrollo y aun así tiene que pasar, porque en el
    /// portátil de al lado sería un juego perfectamente válido.
    #[test]
    fn the_local_username_is_not_a_verdict() {
        assert!(reject_degenerate_slug("insider").is_ok());
    }

    /// El mensaje tiene que decir qué hacer, no sólo que no.
    #[test]
    fn the_error_tells_the_user_what_to_do() {
        let err = reject_degenerate_slug("user").expect_err("rejected");
        let msg = err.to_string();
        assert!(msg.contains("user"), "names the offending slug: {msg}");
        assert!(msg.contains("folder"), "points at the folder flow: {msg}");
    }
}
