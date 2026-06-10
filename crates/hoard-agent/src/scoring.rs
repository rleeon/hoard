//! DETECCIÓN — scoring multi-señal (fase 1, ADR 0020).
//!
//! Reemplaza el booleano name-only del walk agresivo
//! (`detection::classify_dir_as_save_like`) por un score acumulativo
//! `S ∈ [0,1]`. Aquí viven las señales **estáticas** — nombre, contenido,
//! recencia y negativas. La señal dominante de ADR 0020 (correlación
//! proceso↔escritura, +0.50) llega en la fase 3 y se sumará sobre este
//! score base.
//!
//! No toca el pipeline catalog-first (ADR 0009): su único consumidor es la
//! ruta de descubrimiento agresivo. `detection::SAVE_PATTERNS` se mantiene
//! aparte (inglés, match exacto) para la refinación de rutas de catálogo.

use std::path::Path;
use std::time::SystemTime;

/// Cutoffs del ADR 0020 §2.
///
/// * `S ≥ 0.60` → save confirmado automáticamente.
/// * `0.35 ≤ S < 0.60` → "posible": corroborar con catálogo / preguntar.
/// * `S < 0.35` → descartado.
pub const SCORE_CONFIRMED: f32 = 0.60;
pub const SCORE_POSSIBLE: f32 = 0.35;

/// Vocabulario multilingüe de nombres de carpeta-save. Superset de
/// `detection::SAVE_PATTERNS`; incluye términos de/fr/es/it/ru/ja/zh para
/// no perder saves cuyo nombre no esté en inglés.
pub const SAVE_NAME_VOCAB: &[&str] = &[
    "save",
    "saves",
    "savegame",
    "savegames",
    "save games",
    "save_games",
    "savedata",
    "save data",
    "save_data",
    "savefile",
    "savefiles",
    "autosave",
    "quicksave",
    // Multilingüe.
    "sauvegarde",
    "sauvegardes",
    "speichern",
    "spielstand",
    "spielstaende",
    "partida",
    "partidas",
    "guardado",
    "guardados",
    "salvataggi",
    "salvataggio",
    "сохранения",
    "セーブ",
    "存档",
];

/// Nombres que delatan NO-save (config/cache/logs/...). Señal negativa fuerte.
pub const NEGATIVE_NAME_VOCAB: &[&str] = &[
    "config",
    "cache",
    "logs",
    "log",
    "crashdumps",
    "crashpad",
    "shadercache",
    "gpucache",
    "code cache",
    "temp",
    "tmp",
    "telemetry",
    "screenshots",
];

/// Extensiones que casi siempre son saves.
const EXT_STRONG: &[&str] = &["sav", "save", "sl2", "ess", "dsav"];
/// Extensiones ambiguas: sólo suman si ya hay otra señal.
const EXT_WEAK: &[&str] = &["dat", "bin", "profile"];
/// Extensiones ruidosas: aporte casi nulo, abundan en configs.
const EXT_NOISY: &[&str] = &["json", "xml", "ini", "cfg"];
/// Imágenes: una carpeta sólo-imágenes es screenshots, no un save.
const EXT_IMAGE: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];
/// Comprimidos: muchos juegos guardan la partida dentro de un `.zip`
/// (Factorio, varios indies). NO puntúan por ser comprimidos — se abre el
/// índice (sin descomprimir) y se puntúa lo que hay DENTRO
/// (`archive_looks_like_save`).
const EXT_ARCHIVE: &[&str] = &["zip"];

/// `true` si la extensión cae en alguna categoría conocida (fuerte/débil/
/// ruidosa/imagen/comprimido). Lo que NO encaja aquí es una extensión
/// "desconocida" — candidata al heurístico del conjunto homogéneo.
fn is_known_ext(e: &str) -> bool {
    EXT_STRONG.contains(&e)
        || EXT_WEAK.contains(&e)
        || EXT_NOISY.contains(&e)
        || EXT_IMAGE.contains(&e)
        || EXT_ARCHIVE.contains(&e)
}

/// `true` si `path` fue modificado dentro de la ventana de recencia de saves
/// (la misma del pipeline, 180 días). Conservador ante errores de metadata:
/// si no se puede leer la mtime, cuenta como no-reciente.
fn file_is_recent(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    match SystemTime::now().duration_since(modified) {
        Ok(age) => age <= crate::detection::RECENT_SAVE_FILE_WINDOW,
        Err(_) => true, // mtime en el futuro: trátalo como reciente.
    }
}

/// Nombres de entrada que delatan un save dentro de un comprimido,
/// independientes de la extensión (Factorio: `level.dat`, `control.lua`...).
const ARCHIVE_SAVE_MARKERS: &[&str] = &[
    "level.dat",
    "level-init.dat",
    "control.lua",
    "blueprint-storage.dat",
    "gamestate",
    "savegame",
    "save.dat",
    "player.dat",
    "world.dat",
];

/// Abre un comprimido y mira su ÍNDICE (sin descomprimir) buscando contenido
/// save-like: una entrada con extensión fuerte/débil de save, o un nombre
/// marcador. Devuelve `true` al primer indicio. Acotado a las primeras
/// entradas para no penalizar archivos enormes; leer el directorio central no
/// inflа bytes, así que es barato aun en `.zip` grandes.
fn archive_looks_like_save(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return false;
    };
    let limit = zip.len().min(512);
    for i in 0..limit {
        let Ok(entry) = zip.by_index_raw(i) else {
            continue;
        };
        let name = entry.name().to_ascii_lowercase();
        let leaf = name.rsplit(['/', '\\']).next().unwrap_or(name.as_str());
        if ARCHIVE_SAVE_MARKERS.contains(&leaf) {
            return true;
        }
        if let Some((_, ext)) = leaf.rsplit_once('.') {
            if EXT_STRONG.contains(&ext) || EXT_WEAK.contains(&ext) {
                return true;
            }
        }
    }
    false
}

/// Desglose del score de un directorio candidato: el número y la lista de
/// razones que lo justifican (se reenvía al panel de diagnóstico).
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub score: f32,
    pub reasons: Vec<String>,
    /// `true` cuando el contenido del directorio es evidencia directa de save,
    /// no una suposición por nombre. Dos fuentes: (a) un comprimido cuyo índice
    /// delata un save dentro (`level.dat`, `control.lua`, extensión save…), o
    /// (b) un conjunto rotatorio de ≥3 saves de extensión fuerte (aquí o en una
    /// subcarpeta tipo `autosave/`). Cuenta como corroboración para conceder
    /// `High` igual que Steam o la correlación de proceso (ADR 0020), sin una
    /// sesión de juego observada. Un `.sav` suelto NO corrobora — el caso
    /// conservador (tope en `Medium` sin correlación) se mantiene.
    pub corroborated_by_content: bool,
}

/// Umbral del "deque de autosaves": un directorio (o sus subcarpetas
/// inmediatas tipo `autosave/`, `slot/`) con al menos esta cantidad de saves
/// de extensión fuerte es, con altísima probabilidad, una carpeta de saves
/// activa — los juegos rotan autosaves; config/cache no acumulan `.sav`. Sirve
/// como corroboración para `High` sin aflojar el caso del `.sav` suelto.
const STRONG_ROTATING_MIN: usize = 3;

/// Profundidad máxima al inspeccionar las subcarpetas de un candidato buscando
/// saves (p. ej. `save/profiles/slot1/*.sav`). Acota el coste de la recursión.
const STRONG_SCAN_MAX_DEPTH: usize = 4;
/// Tope de ficheros visitados al contar saves en subcarpetas, compartido por
/// todo el escaneo de un candidato. Evita pasear árboles enormes.
const STRONG_SCAN_FILE_BUDGET: usize = 4096;

/// Conteo barato del contenido inmediato (no recursivo) de un candidato.
#[derive(Default)]
struct DirContent {
    files: usize,
    strong: usize,
    weak: usize,
    noisy: usize,
    image: usize,
    /// Comprimidos cuyo índice contiene contenido save-like.
    archive_save: usize,
    /// Saves de extensión fuerte hallados en las subcarpetas (recursivo, con
    /// tope de profundidad/ficheros). Captura `openttd/save/autosave/*.sav` y
    /// layouts anidados (`save/profiles/slotN/*.sav`), donde la ruta del
    /// catálogo apunta al contenedor y los saves viven más abajo.
    strong_subdir: usize,
    /// Ficheros recientes por extensión DESCONOCIDA (ni fuerte/débil/ruidosa/
    /// imagen/comprimido). Soporta el heurístico del "conjunto dominante":
    /// una carpeta con nombre-save exacto que rota ≥3 ficheros recientes de UNA
    /// misma extensión propietaria (`.pss`, `.rsv`, …) es, casi con seguridad,
    /// una carpeta de saves real aunque la extensión no esté en el catálogo. Se
    /// cuenta por extensión (no "homogéneo estricto") para tolerar marcadores
    /// sueltos que conviven con los saves —típicamente un `steam_autocloud.vdf`
    /// en la misma carpeta— sin que invaliden el conjunto.
    unknown_recent_by_ext: std::collections::HashMap<String, usize>,
}

fn scan_content(dir: &Path) -> DirContent {
    let mut c = DirContent::default();
    let Ok(read) = std::fs::read_dir(dir) else {
        return c;
    };
    // Presupuesto de ficheros compartido por todas las subcarpetas del
    // candidato, para acotar el coste total de la recursión.
    let mut budget = STRONG_SCAN_FILE_BUDGET;
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            // Baja por las subcarpetas (autosave/, profiles/slotN/…) contando
            // saves de extensión fuerte, con tope de profundidad y de ficheros.
            c.strong_subdir +=
                count_strong_recursive(&entry.path(), STRONG_SCAN_MAX_DEPTH, &mut budget);
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        c.files += 1;
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some(e) if EXT_STRONG.contains(&e) => c.strong += 1,
            Some(e) if EXT_WEAK.contains(&e) => c.weak += 1,
            Some(e) if EXT_NOISY.contains(&e) => c.noisy += 1,
            Some(e) if EXT_IMAGE.contains(&e) => c.image += 1,
            Some(e) if EXT_ARCHIVE.contains(&e) && archive_looks_like_save(&path) => {
                c.archive_save += 1
            }
            // Extensión desconocida y reciente: cuenta los recientes por
            // extensión. La extensión dominante (≥3 recientes) bajo un
            // nombre-save exacto delata un deque de saves propietarios.
            Some(e) if !is_known_ext(e) && file_is_recent(&path) => {
                *c.unknown_recent_by_ext.entry(e.to_string()).or_default() += 1;
            }
            _ => {}
        }
    }
    c
}

/// Cuenta ficheros de extensión fuerte bajo `dir` recursivamente, hasta
/// `depth` niveles y mientras quede `budget` de ficheros. No sigue symlinks
/// (evita ciclos). Devuelve el número de saves de extensión fuerte hallados.
fn count_strong_recursive(dir: &Path, depth: usize, budget: &mut usize) -> usize {
    if depth == 0 || *budget == 0 {
        return 0;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut n = 0usize;
    for entry in read.flatten() {
        if *budget == 0 {
            break;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            n += count_strong_recursive(&path, depth - 1, budget);
        } else if ft.is_file() {
            *budget -= 1;
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if EXT_STRONG.contains(&ext.to_ascii_lowercase().as_str()) {
                    n += 1;
                }
            }
        }
    }
    n
}

/// Señal de nombre. Exacto en vocab (+0.35) > substring de un token
/// (+0.20) > patrón slot/profile/user (+0.15). El substring es un
/// sustituto barato del `strsim::jaro_winkler` que la fase 1+ usará cuando
/// se añada la dep.
fn name_signal(name: &str, reasons: &mut Vec<String>) -> f32 {
    let lower = name.to_lowercase();
    if SAVE_NAME_VOCAB.iter().any(|v| *v == lower) {
        reasons.push("name exact".into());
        return 0.35;
    }
    if SAVE_NAME_VOCAB
        .iter()
        .any(|v| v.len() >= 4 && lower.contains(v))
    {
        reasons.push("name contains save token".into());
        return 0.20;
    }
    if crate::detection::name_matches_slot_profile_user(name) {
        reasons.push("slot/profile/user".into());
        return 0.15;
    }
    0.0
}

/// Puntúa un directorio candidato combinando nombre + contenido + recencia
/// + señales negativas. Score en `[0,1]`.
pub fn score_dir(path: &Path, name: &str) -> ScoreBreakdown {
    let mut reasons: Vec<String> = Vec::new();
    let mut score = 0.0_f32;

    let name_pos = name_signal(name, &mut reasons);
    score += name_pos;

    let lower = name.to_lowercase();
    if NEGATIVE_NAME_VOCAB.iter().any(|v| *v == lower) {
        score -= 0.45;
        reasons.push("negative name".into());
    }

    let content = scan_content(path);
    let has_signal = name_pos > 0.0;
    let name_exact = SAVE_NAME_VOCAB.iter().any(|v| *v == lower);
    // Saves de extensión fuerte aquí o un nivel más abajo (autosave/, slot/).
    let strong_total = content.strong + content.strong_subdir;

    // Conjunto dominante de extensión desconocida: una carpeta con nombre-save
    // EXACTO donde alguna extensión propietaria (no fuerte/débil/ruidosa/
    // imagen/comprimido) acumula ≥3 ficheros recientes. Genérico: no codifica
    // ninguna extensión concreta, sólo la forma (nombre exacto + rotación
    // reciente). Conservador por triple compuerta —nombre exacto, recencia y
    // ≥3 del MISMO tipo— así que una carpeta de config (json/ini mezclados) o
    // un único marcador suelto (p. ej. un solo `.vdf`) nunca cualifican; pero
    // sí tolera ese marcador conviviendo con los saves reales.
    let unknown_dominant_recent = content
        .unknown_recent_by_ext
        .values()
        .copied()
        .max()
        .unwrap_or(0);
    let homogeneous_unknown_set = name_exact && unknown_dominant_recent >= STRONG_ROTATING_MIN;

    if strong_total > 0 {
        score += 0.30;
        reasons.push("strong save ext".into());
    } else if content.archive_save > 0 {
        // Save dentro de un comprimido (Factorio y cía): el índice del .zip
        // delata contenido save-like. Mismo peso que la extensión fuerte.
        score += 0.30;
        reasons.push("save-like archive content".into());
    } else if homogeneous_unknown_set {
        // Conjunto rotatorio de saves de extensión propietaria bajo un
        // nombre-save exacto. Mismo peso que la extensión fuerte.
        score += 0.30;
        reasons.push("homogeneous recent save set".into());
    } else if content.weak > 0 && has_signal {
        score += 0.08;
        reasons.push("weak ext + other signal".into());
    } else if content.noisy > 0 && !has_signal {
        score += 0.02;
        reasons.push("noisy ext only".into());
    }

    // Recencia: reutiliza el chequeo del pipeline (ventana ya subida a 180d).
    if crate::detection::dir_has_recent_save_file(path) {
        score += 0.10;
        reasons.push("recent save-like file".into());
    }

    // Negativas de contenido + hard rule (§4): una carpeta sólo-imágenes o
    // sólo-ruido (config/log) NUNCA se auto-confirma, por más que el nombre
    // matchee.
    let only_images = content.files > 0 && content.image == content.files;
    let only_noisy = content.files > 0
        && content.noisy == content.files
        && content.strong == 0
        && content.weak == 0;
    if only_images {
        score -= 0.40;
        reasons.push("screenshots only".into());
    } else if only_noisy {
        score -= 0.35;
        reasons.push("config/noisy only".into());
    }

    let mut score = score.clamp(0.0, 1.0);
    if only_images || only_noisy {
        score = score.min(SCORE_POSSIBLE - 0.001);
    }

    // Corroboración por contenido (habilita `High` sin correlación), siempre
    // que la hard-rule no haya degradado la carpeta a no-save:
    //   * un comprimido con índice save-like verificado (Factorio y cía), o
    //   * un conjunto rotatorio de ≥3 saves de extensión fuerte (autosaves de
    //     openttd y similares) — un `.sav` suelto NO basta, así que el caso
    //     conservador se mantiene.
    let corroborated_by_content = !(only_images || only_noisy)
        && (content.archive_save > 0
            || strong_total >= STRONG_ROTATING_MIN
            || homogeneous_unknown_set);

    ScoreBreakdown {
        score,
        reasons,
        corroborated_by_content,
    }
}

/// `true` si la señal de nombre por sí sola reconoce esta carpeta como
/// save (exacto, substring de token, o patrón slot/profile/user). Aislado
/// para el benchmark de §(scoring) — mide el techo de recall del nombre sin
/// confundirlo con señales de contenido.
pub fn name_recognised(name: &str) -> bool {
    let lower = name.to_lowercase();
    SAVE_NAME_VOCAB.iter().any(|v| *v == lower)
        || SAVE_NAME_VOCAB
            .iter()
            .any(|v| v.len() >= 4 && lower.contains(v))
        || crate::detection::name_matches_slot_profile_user(name)
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    /// Escribe un `.zip` (método Stored, sin codec) con las entradas dadas.
    fn write_zip(path: &Path, entries: &[&str]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for name in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn factorio_zip_save_scores_high() {
        let dir = tempfile::tempdir().unwrap();
        // Réplica del interior de un save de Factorio.
        write_zip(
            &dir.path().join("my-world.zip"),
            &["my-world/level.dat", "my-world/control.lua"],
        );
        let b = score_dir(dir.path(), "saves");
        assert!(
            b.reasons.iter().any(|r| r.contains("archive")),
            "expected archive signal, got {:?}",
            b.reasons
        );
        assert!(b.score >= SCORE_CONFIRMED, "score {} too low", b.score);
        // El contenido verificado corrobora → habilita `High` sin correlación.
        assert!(b.corroborated_by_content);
    }

    #[test]
    fn rotating_autosave_set_in_subdir_corroborates_high() {
        // Réplica de openttd: la ruta apunta a `save/`, y los `.sav` viven en
        // `save/autosave/`. Un deque de autosaves (≥3 ext. fuerte) corrobora.
        let dir = tempfile::tempdir().unwrap();
        let auto = dir.path().join("autosave");
        std::fs::create_dir(&auto).unwrap();
        for i in 0..12 {
            std::fs::write(auto.join(format!("autosave{i}.sav")), b"x").unwrap();
        }
        let b = score_dir(dir.path(), "save");
        assert!(b.score >= SCORE_CONFIRMED, "score {} too low", b.score);
        assert!(
            b.corroborated_by_content,
            "rotating save set should corroborate: {:?}",
            b.reasons
        );
    }

    #[test]
    fn single_loose_sav_does_not_corroborate() {
        // Un `.sav` suelto sigue siendo conservador: puntúa pero NO corrobora.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.sav"), b"x").unwrap();
        let b = score_dir(dir.path(), "save");
        assert!(
            !b.corroborated_by_content,
            "a single loose .sav must not corroborate: {:?}",
            b.reasons
        );
    }

    #[test]
    fn homogeneous_unknown_ext_set_corroborates_high() {
        // Carpeta `saves` con ≥3 ficheros recientes de UNA extensión
        // propietaria desconocida (`.pss` de Planet S, p. ej.). Debe
        // auto-confirmar y corroborar, sin codificar la extensión.
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            std::fs::write(dir.path().join(format!("slot{i}.pss")), b"x").unwrap();
        }
        let b = score_dir(dir.path(), "saves");
        assert!(b.score >= SCORE_CONFIRMED, "score {} too low", b.score);
        assert!(
            b.corroborated_by_content,
            "homogeneous recent save set should corroborate: {:?}",
            b.reasons
        );
    }

    #[test]
    fn single_unknown_marker_file_does_not_corroborate() {
        // El señuelo de Planet S: una carpeta `saves` con un solo
        // `steam_autocloud.vdf`. Un único fichero (<3) no cualifica.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("steam_autocloud.vdf"), b"x").unwrap();
        let b = score_dir(dir.path(), "saves");
        assert!(
            !b.corroborated_by_content,
            "a single marker file must not corroborate: {:?}",
            b.reasons
        );
        assert!(b.score < SCORE_CONFIRMED, "score {} too high", b.score);
    }

    #[test]
    fn stray_marker_alongside_real_saves_still_corroborates() {
        // Caso real de Planet S: 10 `.pss` reales + un `steam_autocloud.vdf`
        // colado en la misma carpeta. El marcador suelto NO debe invalidar el
        // conjunto dominante de saves.
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            std::fs::write(dir.path().join(format!("slot{i}.pss")), b"x").unwrap();
        }
        std::fs::write(dir.path().join("steam_autocloud.vdf"), b"x").unwrap();
        let b = score_dir(dir.path(), "saves");
        assert!(b.score >= SCORE_CONFIRMED, "score {} too low", b.score);
        assert!(
            b.corroborated_by_content,
            "stray marker must not break the dominant save set: {:?}",
            b.reasons
        );
    }

    #[test]
    fn mixed_unknown_exts_do_not_corroborate() {
        // Mezcla de extensiones desconocidas (no homogéneo): típico de una
        // carpeta de datos varios, no un deque de saves. No debe corroborar.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.foo"), b"x").unwrap();
        std::fs::write(dir.path().join("b.bar"), b"x").unwrap();
        std::fs::write(dir.path().join("c.baz"), b"x").unwrap();
        std::fs::write(dir.path().join("d.qux"), b"x").unwrap();
        let b = score_dir(dir.path(), "saves");
        assert!(
            !b.corroborated_by_content,
            "mixed unknown exts must not corroborate: {:?}",
            b.reasons
        );
    }

    #[test]
    fn random_zip_does_not_score() {
        let dir = tempfile::tempdir().unwrap();
        write_zip(
            &dir.path().join("photos.zip"),
            &["photos/img1.png", "photos/readme.txt"],
        );
        // Carpeta sin nombre-save: un zip de fotos no debe puntuar como save.
        let b = score_dir(dir.path(), "downloads");
        assert!(b.score < SCORE_POSSIBLE, "score {} too high", b.score);
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use hoard_manifest::ludusavi;

    /// Extrae el nombre de la carpeta-save más profunda de un template
    /// Ludusavi. Salta segmentos glob (`*`, `**`) y placeholders (`<...>`),
    /// y descarta el segmento si parece un fichero (tiene extensión).
    /// Devuelve `None` cuando no queda un nombre de directorio utilizable.
    fn leaf_dir_name(template: &str) -> Option<String> {
        for seg in template.split(['/', '\\']).rev() {
            let seg = seg.trim();
            if seg.is_empty() || seg.contains('*') || seg.starts_with('<') {
                continue;
            }
            // Saltar si es claramente un fichero (extensión corta conocida-ish).
            if let Some((_, ext)) = seg.rsplit_once('.') {
                if (1..=5).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                    continue;
                }
            }
            return Some(seg.to_string());
        }
        None
    }

    /// BENCHMARK DETECCIÓN (ADR 0020): techo de recall de la señal de NOMBRE
    /// sobre los nombres de carpeta-save reales del manifest embebido.
    ///
    /// No mide contenido ni correlación (la joya de fase 3) — sólo cuánto
    /// recupera el vocabulario por nombre. Esperado/honesto: bajo, porque
    /// muchísimos juegos nombran la carpeta con el título del juego, no con
    /// "save". Justamente eso motiva las señales independientes del nombre.
    ///
    /// Correr: `cargo test -p hoard-agent --lib -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn name_signal_recall_over_manifest() {
        use std::collections::HashMap;

        let mut leaves: HashMap<String, ()> = HashMap::new();
        for entry in ludusavi::catalog() {
            for set in [&entry.paths.windows, &entry.paths.linux, &entry.paths.mac] {
                for p in set {
                    if let Some(leaf) = leaf_dir_name(&p.path) {
                        leaves.insert(leaf.to_lowercase(), ());
                    }
                }
            }
        }

        let total = leaves.len();
        assert!(total > 0, "manifest yielded no leaf names");

        let mut recognised = 0usize;
        let mut neg_collisions = 0usize; // reconocidos que también son config/cache
        for name in leaves.keys() {
            if name_recognised(name) {
                recognised += 1;
                if NEGATIVE_NAME_VOCAB.iter().any(|v| name.contains(v)) {
                    neg_collisions += 1;
                }
            }
        }

        let recall = recognised as f32 / total as f32 * 100.0;
        eprintln!("=== BENCHMARK name-signal recall (ADR 0020) ===");
        eprintln!("manifest entries:        {}", ludusavi::catalog().len());
        eprintln!("unique save-leaf names:  {total}");
        eprintln!("name-recognised:         {recognised} ({recall:.1}%)");
        eprintln!("  of which config-ish:   {neg_collisions} (precision risk)");
        eprintln!("=> recall del NAME-signal solo; contenido+correlación suben esto en fases 2/3");
    }
}
