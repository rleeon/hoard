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

/// Desglose del score de un directorio candidato: el número y la lista de
/// razones que lo justifican (se reenvía al panel de diagnóstico).
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub score: f32,
    pub reasons: Vec<String>,
}

/// Conteo barato del contenido inmediato (no recursivo) de un candidato.
#[derive(Default)]
struct DirContent {
    files: usize,
    strong: usize,
    weak: usize,
    noisy: usize,
    image: usize,
}

fn scan_content(dir: &Path) -> DirContent {
    let mut c = DirContent::default();
    let Ok(read) = std::fs::read_dir(dir) else {
        return c;
    };
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
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
            _ => {}
        }
    }
    c
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
    if SAVE_NAME_VOCAB.iter().any(|v| v.len() >= 4 && lower.contains(v)) {
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

    if content.strong > 0 {
        score += 0.30;
        reasons.push("strong save ext".into());
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

    ScoreBreakdown { score, reasons }
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

/// `true` si la señal de nombre por sí sola reconoce esta carpeta como
/// save (exacto, substring de token, o patrón slot/profile/user). Aislado
/// para el benchmark de §(scoring) — mide el techo de recall del nombre sin
/// confundirlo con señales de contenido.
pub fn name_recognised(name: &str) -> bool {
    let lower = name.to_lowercase();
    SAVE_NAME_VOCAB.iter().any(|v| *v == lower)
        || SAVE_NAME_VOCAB.iter().any(|v| v.len() >= 4 && lower.contains(v))
        || crate::detection::name_matches_slot_profile_user(name)
}
