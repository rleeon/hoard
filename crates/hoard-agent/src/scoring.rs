//! Detection: multi-signal scoring (phase 1, ADR 0020).
//!
//! Replaces the name-only boolean of the aggressive walk
//! (`detection::classify_dir_as_save_like`) with a cumulative score `S` in
//! `[0,1]`. What lives here are the static signals: name, content, recency and
//! the negatives. ADR 0020's dominant signal, the process-to-write correlation
//! worth +0.50, arrives in phase 3 and will be added on top of this base score.
//!
//! It does not touch the catalogue-first pipeline (ADR 0009): its only consumer is
//! the aggressive discovery route. `detection::SAVE_PATTERNS` stays separate
//! (English, exact match) for refining catalogue paths.

use std::path::Path;
use std::time::SystemTime;

/// ADR 0020 §2's cutoffs.
///
/// * `S ≥ 0.60` → save confirmado automáticamente.
/// * `0.35 <= S < 0.60` is "possible": corroborate with the catalogue, or ask.
/// * `S < 0.35` → descartado.
pub const SCORE_CONFIRMED: f32 = 0.60;
pub const SCORE_POSSIBLE: f32 = 0.35;

/// A multilingual vocabulary of save-folder names. A superset of
/// `detection::SAVE_PATTERNS`, with German, French, Spanish, Italian, Russian,
/// Japanese and Chinese terms so saves named in another language are not lost.
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

/// Names that give away a NON-save (config, cache, logs). A strong negative.
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

/// Extensions that are almost always saves.
const EXT_STRONG: &[&str] = &["sav", "save", "sl2", "ess", "dsav"];
/// Ambiguous extensions: they only count when there is already another signal.
const EXT_WEAK: &[&str] = &["dat", "bin", "profile"];
/// Extensiones ruidosas: aporte casi nulo, abundan en configs.
const EXT_NOISY: &[&str] = &["json", "xml", "ini", "cfg"];
/// Images: a folder of nothing but images is screenshots, not a save.
const EXT_IMAGE: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];
/// Archives: plenty of games keep the save inside a `.zip` (Factorio, several
/// indies). They score nothing for being archives; the index is opened, without
/// decompressing, and what is INSIDE gets scored (`archive_looks_like_save`).
const EXT_ARCHIVE: &[&str] = &["zip"];

/// `true` when the extension falls into a known category (strong, weak, noisy,
/// image, archive). What does not fit here is an unknown extension, and a
/// candidate for the homogeneous-set heuristic.
fn is_known_ext(e: &str) -> bool {
    EXT_STRONG.contains(&e)
        || EXT_WEAK.contains(&e)
        || EXT_NOISY.contains(&e)
        || EXT_IMAGE.contains(&e)
        || EXT_ARCHIVE.contains(&e)
}

/// `true` when `path` was modified inside the save recency window (the same one
/// the pipeline uses, 180 days). Conservative about metadata errors: when the
/// mtime cannot be read it counts as not recent.
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

/// Entry names that give away a save inside an archive, whatever the extension
/// (Factorio: `level.dat`, `control.lua` and so on).
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

/// Opens an archive and looks at its INDEX, without decompressing, for save-like
/// content: an entry with a strong or weak save extension, or a marker name. It
/// returns `true` on the first sign. Bounded to the first entries so huge archives
/// are not penalised; reading the central directory inflates no bytes, so it is
/// cheap even on a large `.zip`.
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

/// The breakdown of a candidate directory's score: the number and the list of
/// reasons behind it (forwarded to the diagnostics panel).
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub score: f32,
    pub reasons: Vec<String>,
    /// `true` when the directory's content is direct evidence of a save rather
    /// than a guess from its name. Two sources: an archive whose index gives away
    /// a save inside (`level.dat`, `control.lua`, a save extension), or a rotating
    /// set of three or more strong-extension saves, here or in an `autosave/`-style
    /// subfolder. It counts as corroboration for granting `High` just like Steam
    /// or process correlation (ADR 0020), with no observed play session. A lone
    /// `.sav` does not corroborate: the conservative case (capped at `Medium`
    /// without correlation) stands.
    pub corroborated_by_content: bool,
}

/// The "autosave deque" threshold: a directory (or its immediate `autosave/` or
/// `slot/`-style subfolders) with at least this many strong-extension saves is,
/// with very high probability, a live save folder. Games rotate autosaves; config
/// and cache do not accumulate `.sav` files. It serves as corroboration for `High`
/// without loosening the lone-`.sav` case.
const STRONG_ROTATING_MIN: usize = 3;

/// Maximum depth when inspecting a candidate's subfolders for saves (for example
/// `save/profiles/slot1/*.sav`). Bounds the cost of the recursion.
const STRONG_SCAN_MAX_DEPTH: usize = 4;
/// Cap on files visited while counting saves in subfolders, shared across a
/// candidate's whole scan. Stops it walking enormous trees.
const STRONG_SCAN_FILE_BUDGET: usize = 4096;

/// A cheap count of a candidate's immediate, non-recursive content.
#[derive(Default)]
struct DirContent {
    files: usize,
    strong: usize,
    weak: usize,
    noisy: usize,
    image: usize,
    /// Comprimidos cuyo índice contiene contenido save-like.
    archive_save: usize,
    /// Strong-extension saves found in the subfolders (recursive, with depth and
    /// file caps). Catches `openttd/save/autosave/*.sav` and nested layouts
    /// (`save/profiles/slotN/*.sav`), where the catalogue path points at the
    /// container and the saves live further down.
    strong_subdir: usize,
    /// Recent files with an UNKNOWN extension (neither strong, weak, noisy, image
    /// nor archive). This backs the "dominant set" heuristic: a folder with an
    /// exact save name rotating three or more recent files of ONE proprietary
    /// extension (`.pss`, `.rsv`) is almost certainly a real save folder even when
    /// the extension is not in the catalogue. It counts per extension rather than
    /// demanding strict homogeneity, so a stray marker living alongside the saves,
    /// typically a `steam_autocloud.vdf` in the same folder, does not invalidate
    /// the set.
    unknown_recent_by_ext: std::collections::HashMap<String, usize>,
}

fn scan_content(dir: &Path) -> DirContent {
    let mut c = DirContent::default();
    let Ok(read) = std::fs::read_dir(dir) else {
        return c;
    };
    // A file budget shared by all of the candidate's subfolders, to bound the
    // recursion's total cost.
    let mut budget = STRONG_SCAN_FILE_BUDGET;
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            // Walk down the subfolders (autosave/, profiles/slotN/) counting
            // strong-extension saves, with depth and file caps.
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
            // An unknown, recent extension: count the recent ones per extension.
            // The dominant extension (three or more recent) under an exact save
            // name gives away a deque of proprietary saves.
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

/// The name signal. An exact vocabulary hit (+0.35) beats a token substring
/// (+0.20), which beats a slot, profile or user pattern (+0.15). The substring is
/// a cheap stand-in for the `strsim::jaro_winkler` phase 1+ will use once the
/// dependency is added.
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

    // A dominant set of an unknown extension: a folder with an EXACT save name
    // where some proprietary extension (not strong, weak, noisy, image or archive)
    // accumulates three or more recent files. Generic: it encodes no particular
    // extension, only the shape, being an exact name plus recent rotation.
    // Conservative behind a triple gate (exact name, recency, and three of the
    // SAME type) so a config folder of mixed json and ini, or one stray marker
    // such as a lone `.vdf`, never qualifies; but it does tolerate that marker
    // living alongside the real saves.
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
        // A save inside an archive (Factorio and company): the `.zip`'s index
        // gives away save-like content. The same weight as a strong extension.
        score += 0.30;
        reasons.push("save-like archive content".into());
    } else if homogeneous_unknown_set {
        // A rotating set of proprietary-extension saves under an exact save name.
        // The same weight as a strong extension.
        score += 0.30;
        reasons.push("homogeneous recent save set".into());
    } else if content.weak > 0 && has_signal {
        score += 0.08;
        reasons.push("weak ext + other signal".into());
    } else if content.noisy > 0 && !has_signal {
        score += 0.02;
        reasons.push("noisy ext only".into());
    }

    // Recency: reuses the pipeline's check (the window is already up to 180d).
    if crate::detection::dir_has_recent_save_file(path) {
        score += 0.10;
        reasons.push("recent save-like file".into());
    }

    // COPY name (P3, DETECCION-REVISION §8): a directory whose name ENDS in a
    // backup suffix (`SaveGamesBackup`, `SavesOld`, `NobodyT-bak`) is the
    // game's own archive rather than its save, but only when it also holds
    // rotating content (≥3 strong saves here or one level down). Suffix, never
    // prefix: `BackupSaves` pays nothing. Without the content gate a real
    // folder with an unlucky name would lose 0.20 for free; with it, the
    // Wukong case drops 0.50 → 0.30 and falls below the candidate floor.
    //
    // Note this penalty does NOT by itself rank the mirror under the real
    // save: it cancels the +0.20 the mirror got for containing "savegames"
    // and the two TIE. What actually reorders them is the structural veto in
    // `detection::is_backup_mirror`. Removing the cushion is not choosing.
    //
    // And it is capped at the bonus the name actually earned, never taking a
    // folder net-negative. The catalog lists plenty of games whose ONLY save
    // path is literally a `backup/`: Don't Starve Together, NIMBY Rails,
    // Morbid, Isles of Sea and Sky, The Last Caretaker. Those names score
    // nothing positive to begin with (`backup` is not in the vocabulary), so
    // a flat −0.20 would push the one real folder under the 0.35 floor and
    // lose the game outright. Cancelling a cushion is defensible; inventing a
    // debt is not.
    let suffix_penalty = name_pos.min(0.20);
    if suffix_penalty > 0.0
        && strong_total >= STRONG_ROTATING_MIN
        && crate::junkdirs::ends_with_backup_suffix(name)
    {
        score -= suffix_penalty;
        reasons.push("backup-suffix name on rotating content".into());
    }

    // Content negatives plus the hard rule: a folder of nothing but images, or
    // nothing but noise (config, logs), NEVER self-confirms, however well the name
    // matches.
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

    // Corroboration by content (which enables `High` without correlation),
    // provided the hard rule has not already demoted the folder to not-a-save:
    //   * an archive with a verified save-like index (Factorio and company), or
    //   * a rotating set of three or more strong-extension saves (openttd-style
    //     autosaves). A lone `.sav` is not enough, so the conservative case
    //     stands.
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

/// `true` when the name signal alone recognises this folder as a save (exact, a
/// token substring, or a slot, profile or user pattern). Isolated for the scoring
/// benchmark: it measures the ceiling of name recall without confusing it with
/// content signals.
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

    /// Writes a `.zip` (Stored method, no codec) with the given entries.
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
        // A replica of the inside of a Factorio save.
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
        // The openttd shape: the path points at `save/` and the `.sav` files live
        // in `save/autosave/`. An autosave deque (three or more strong ext.)
        // corroborates.
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
        // A `saves` folder with three or more recent files of ONE unknown
        // proprietary extension. It has to self-confirm and corroborate, without
        // the extension being hard-coded.
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
        // The decoy: a `saves` folder with a single `steam_autocloud.vdf`. One
        // file (fewer than three) does not qualify.
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
        // A real case: 10 genuine proprietary saves plus a `steam_autocloud.vdf`
        // that slipped into the same folder. The stray marker must not invalidate
        // the dominant set of saves.
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
        // A mix of unknown extensions (not homogeneous): typical of a
        // miscellaneous data folder, not a save deque. It must not corroborate.
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
        // A folder with no save name: a zip of photos must not score as a save.
        let b = score_dir(dir.path(), "downloads");
        assert!(b.score < SCORE_POSSIBLE, "score {} too high", b.score);
    }
}

#[cfg(test)]
mod backup_suffix_tests {
    use super::*;

    /// Fixture: `n` strong-extension saves sitting directly in the folder.
    fn dir_with_strong_saves(n: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..n {
            std::fs::write(dir.path().join(format!("slot{i}.sav")), b"x").unwrap();
        }
        dir
    }

    /// P3: the copy suffix only penalises alongside rotating content, and a
    /// prefix never does. Each pair shares a name-signal class so the delta
    /// isolates the penalty and nothing else.
    #[test]
    fn suffix_penalises_but_only_with_rotating_content() {
        let rotating = dir_with_strong_saves(4);
        let lonely = dir_with_strong_saves(1);

        // Same content, copy name vs original name (both +0.20 from the
        // substring): the mirror loses exactly the 0.20 of the penalty.
        let mirror = score_dir(rotating.path(), "savegamesxbackup");
        let plain = score_dir(rotating.path(), "savegamesx");
        assert!(
            mirror.reasons.iter().any(|r| r.contains("backup-suffix")),
            "expected the penalty reason: {:?}",
            mirror.reasons
        );
        assert!((mirror.score - (plain.score - 0.20)).abs() < 1e-5);

        // A single strong save is NOT rotation: no penalty.
        let lone_mirror = score_dir(lonely.path(), "savegamesxbackup");
        let lone_plain = score_dir(lonely.path(), "savegamesx");
        assert_eq!(lone_mirror.score, lone_plain.score);
        assert!(!lone_mirror
            .reasons
            .iter()
            .any(|r| r.contains("backup-suffix")));
    }

    #[test]
    fn prefix_never_penalises() {
        let dir = dir_with_strong_saves(4);
        // `BackupSaves` shape: the suffix rules, and here it isn't at the end.
        let prefixed = score_dir(dir.path(), "backupsavesgamesx");
        let plain = score_dir(dir.path(), "savesgamesx2");
        assert!(
            !prefixed.reasons.iter().any(|r| r.contains("backup-suffix")),
            "prefix must not be penalised: {:?}",
            prefixed.reasons
        );
        assert_eq!(prefixed.score, plain.score);
    }

    #[test]
    fn wukong_shape_loses_its_static_edge_over_the_real_save() {
        // The incident's shape: the mirror carried a +0.20 name bonus the
        // real save (a numeric id) never had. The penalty cancels it exactly,
        // so with equal strong content the two TIE, and the tiebreak falls to
        // discovery order (the real one comes first in the catalog) or to P2's
        // structural veto. This test asserts the tie on purpose: an earlier
        // run claimed the mirror ended up lower, and it does not.
        let mirror_dir = dir_with_strong_saves(4);
        let real_dir = dir_with_strong_saves(4);
        let mirror = score_dir(mirror_dir.path(), "SaveGamesBackup");
        let real = score_dir(real_dir.path(), "76561199002555123");
        assert!(
            mirror.reasons.iter().any(|r| r.contains("backup-suffix")),
            "mirror must carry the penalty: {:?}",
            mirror.reasons
        );
        assert!(
            (mirror.score - real.score).abs() < 1e-5,
            "penalty must neutralise the name edge: mirror {} vs real {}",
            mirror.score,
            real.score
        );
    }

    /// The catalog lists games whose ONLY save path is a folder literally
    /// called `backup`: Don't Starve Together, NIMBY Rails, Morbid, Isles of
    /// Sea and Sky, The Last Caretaker. They earn no name bonus (`backup` is
    /// not in the vocabulary), so a flat penalty would take them net-negative
    /// and drop the game's one real folder under the candidate floor. The
    /// penalty is capped at the bonus actually granted, so these are untouched.
    #[test]
    fn a_lone_backup_folder_is_never_pushed_under_the_floor() {
        let dir = dir_with_strong_saves(4);
        for name in ["backup", "backups", "save_backups", "SaveGameBackups"] {
            let scored = score_dir(dir.path(), name);
            let neutral = score_dir(dir.path(), "zzqqx");
            assert!(
                scored.score >= neutral.score,
                "{name} must not score below a meaningless name: {} vs {}",
                scored.score,
                neutral.score
            );
            assert!(
                scored.score >= SCORE_POSSIBLE,
                "{name} is a real game's only save folder and must stay a candidate: {}",
                scored.score
            );
        }
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use hoard_manifest::ludusavi;

    /// Extracts the deepest save-folder name from a Ludusavi template. It skips
    /// glob segments (`*`, `**`) and placeholders (`<...>`), and discards the
    /// segment when it looks like a file (it has an extension). Returns `None`
    /// when no usable directory name is left.
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

    /// Detection benchmark (ADR 0020): the recall ceiling of the NAME signal over
    /// the real save-folder names in the embedded manifest.
    ///
    /// It measures neither content nor correlation (phase 3's jewel), only how
    /// much the name vocabulary recovers. Expected to be low, and honestly so,
    /// because a great many games name the folder after the game's title rather
    /// than after "save". That is exactly what motivates the name-independent
    /// signals.
    ///
    /// Run with: `cargo test -p hoard-agent --lib -- --ignored --nocapture`
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
