//! Detection: which folders are NOT a save, and which are too wide to offer.
//!
//! Three pieces the rest of the pipeline shares:
//!
//! * [`is_cache_dir_name`]: regenerable cache (shaders, DX12, logs).
//!   `scoring::NEGATIVE_NAME_VOCAB`'s exact set did not catch `AnvilDX12Cache` or
//!   `FortniteShaderCache`, because the game prefixes them with its own name; here
//!   the rule is by suffix and normalises separators, so `Shader Cache`,
//!   `shader_cache` and `ShaderCache` are the same name.
//! * [`save_dirs_under`]: finding save folders by name inside a bounded tree, for
//!   games that save next to the executable.
//! * [`blocked_roots`]: roots that are never offered, meaning the user profile,
//!   `Documents`, and the shared engine roots (RenPy, Godot, LOVE) where a match
//!   has to point at ONE game's folder inside rather than at the root holding them
//!   all.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::expand_path;

/// Folders that are regenerable cache or derived state: never a save, and syncing
/// them would move hundreds of megabytes of machine-specific junk.
const CACHE_DIR_NAMES: &[&str] = &[
    // Cachés de API gráfica.
    "dx12cache",
    "dxcache",
    "d3dcache",
    "d3dscache",
    "dxil",
    "dxbc",
    "pipelinecache",
    "psocache",
    // Cachés de motor y de vendor.
    "shadercache",
    "shadercachedb",
    "shaders",
    "shadercompiler",
    "derivedatacache",
    "ddc",
    "gpucache",
    "glcache",
    "vulkancache",
    "nvidiacache",
    // Genéricas y estado regenerado.
    "cache",
    "caches",
    "cacheddata",
    "temp",
    "tmp",
    "logs",
    "log",
    "crashes",
    "crashdumps",
    "crashreports",
    "webcache",
    "mediacache",
];

/// Suffixes that give away the same family when the game prefixes them with its
/// own name (`FortniteShaderCache`, `AnvilDX12Cache`, `DerivedDataCache`). Ending
/// in "cache" is signal enough on its own: this is checked BEFORE the save names,
/// so an oddity like `SaveCache` counts as cache, which is what it is.
const CACHE_DIR_SUFFIXES: &[&str] = &["cache"];

/// Lowercase and stripped of the separators people use interchangeably, so one
/// entry covers every spelling.
pub fn normalize_dir_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-' | '.'))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// `true` when the folder name is regenerable cache rather than save data.
pub fn is_cache_dir_name(name: &str) -> bool {
    let n = normalize_dir_name(name);
    if n.is_empty() {
        return false;
    }
    CACHE_DIR_NAMES.contains(&n.as_str()) || CACHE_DIR_SUFFIXES.iter().any(|s| n.ends_with(s))
}

/// Folders that are added content rather than player data: mods, Workshop
/// subscriptions, screenshots. Not junk, since the user is fond of them, but not
/// their save either, and orders of magnitude heavier than it.
///
/// They exist for [`holds_foreign_subdir`]: never used to exclude files from a
/// save already being tracked, only to decide that a folder does not deserve to be
/// adopted whole. See issue #17: `AppData\Local\Teardown` keeps a `savegame.xml`
/// of a few KB next to a 42 MB `mods\`.
const FOREIGN_DIR_NAMES: &[&str] = &[
    "mods",
    "mod",
    "modding",
    "workshop",
    "addons",
    "addon",
    "plugins",
    "screenshots",
    "screenshot",
    "videos",
    "replays",
    "recordings",
];

/// `true` when the name suggests save data: `Saves`, `savegames`, `SaveData`,
/// `AutoSave`, `SAVE`. The comparison ignores case and separators. Looser than
/// `detection::SAVE_PATTERNS`, which demands exact equality, on purpose: by here we
/// already come from a bounded tree.
pub fn looks_like_save_dir_name(name: &str) -> bool {
    !is_cache_dir_name(name) && normalize_dir_name(name).contains("save")
}

/// `true` when the name gives away added content (mods, Workshop, screenshots)
/// rather than player data. See [`FOREIGN_DIR_NAMES`].
///
/// A name that also sounds like saves wins: `SaveMods` is odd enough that whoever
/// created it knew what they were doing, and being wrong here costs a save that
/// never gets backed up.
pub fn is_foreign_dir_name(name: &str) -> bool {
    if looks_like_save_dir_name(name) {
        return false;
    }
    FOREIGN_DIR_NAMES.contains(&normalize_dir_name(name).as_str())
}

/// `true` when `dir` has, directly below it, a folder that is added content or
/// regenerable cache: the sign that `dir` is the game's folder rather than its
/// saves' folder, and that adopting it whole would drag in hundreds of megabytes
/// nobody asked for.
///
/// One level only: what is being decided is whether `dir` itself gets offered, and
/// a `mods\` buried three levels down does not change that answer. An IO error
/// answers `false`, since not being able to read a folder is no reason to stop
/// backing it up.
pub fn holds_foreign_subdir(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_foreign_dir_name(&name) || is_cache_dir_name(&name) {
            return true;
        }
    }
    false
}

/// Suffixes that give away a COPY of saves rather than the live save:
/// `SaveGamesBackup`, `SavesOld`, `NobodyT-bak`. A suffix, never a prefix, since
/// `BackupSaves` is a save folder with an odd prefix and must not match.
///
/// [`normalize_dir_name`] has already eaten the separators by the time we compare,
/// so `_bak`, `-bak` and `.bak` all arrive as `...bak`. `old` is the one risky term
/// (any word ending in -old matches), which is why callers treat this as a weak
/// signal, a penalty or a warning, and never as a veto on its own (see
/// `scoring::score_dir` and `detection::is_backup_mirror`).
pub const BACKUP_DIR_SUFFIXES: &[&str] = &["backup", "backups", "bak", "old"];

/// The subset that needs a word boundary to count. `backup`, `backups` and `bak`
/// are unambiguous enough to match even when a name runs straight into them
/// (`savegamesbackup`): checked against the whole catalog, every leaf ending in
/// those letters really is a copy. `old` is the opposite, being the tail of
/// ordinary words, and demanding the boundary is the only thing standing between
/// the rule and `Stranglehold`.
const BOUNDED_SUFFIXES: &[&str] = &["old"];

/// `true` when the name ends in a copy suffix **at a real word boundary**.
///
/// The boundary is the whole point. Matching the bare letters against a
/// separator-stripped name is what a first cut did, and the catalog is full of
/// counter-examples it would have condemned: `Sunday Gold`, `Stranglehold`,
/// `Stikbold`, `Defold`, `Making History Gold`, `Castle of Heart_ Retold`,
/// and `wildlife-park-gold-remastered`, whose only save path is a `savegold/`
/// folder of `.sav` files that clears the rotating-content gate and would have
/// eaten the penalty for nothing. All of them merely *end in the letters*
/// "old".
///
/// So an ambiguous suffix ([`BOUNDED_SUFFIXES`]) counts only when the name IS
/// it (`old`), or when what precedes it is a separator (`Saves_Old`) or a case
/// change (`SavesOld`). Lowercase letters running straight into it are part of
/// a longer word, not a marker. The unambiguous ones match either way.
pub fn ends_with_backup_suffix(name: &str) -> bool {
    let raw: Vec<char> = name.chars().collect();
    let lower: String = name.to_lowercase();
    for suf in BACKUP_DIR_SUFFIXES {
        let Some(head) = lower.strip_suffix(suf) else {
            continue;
        };
        // The whole name is the suffix.
        if head.is_empty() {
            return true;
        }
        if !BOUNDED_SUFFIXES.contains(suf) {
            return true;
        }
        // `head` is a char-count prefix only while the name is ASCII, which
        // every one of these markers is; index defensively all the same.
        let cut = head.chars().count();
        let Some(&prev) = raw.get(cut.wrapping_sub(1)) else {
            continue;
        };
        if matches!(prev, ' ' | '_' | '-' | '.') {
            return true;
        }
        // Case change: `SaveGames|Backup`. The suffix's own first character
        // must be the uppercase one, or we are inside a word.
        if prev.is_lowercase() && raw.get(cut).is_some_and(|c| c.is_uppercase()) {
            return true;
        }
    }
    false
}

/// Maximum depth under the install root. It reaches the layouts games really use
/// (`<install>/savegames/<id>`, `<install>/Binaries/Saves`) without walking a whole
/// tree of assets.
const SAVE_SCAN_MAX_DEPTH: usize = 3;
/// A directory with an implausible number of subfolders is an asset dump, not
/// somewhere saves live.
const SAVE_SCAN_MAX_FANOUT: usize = 120;
/// Cap on the save folders one install can contribute, so a pathological tree
/// cannot flood the results.
const SAVE_SCAN_MAX_HITS: usize = 4;

/// Looks for save folders by NAME inside `root`.
///
/// For the games that save next to the executable rather than in a location known
/// by engine or launcher, such as the Ubisoft titles with
/// `<install>/savegames/<numeric id>`, which no template enumerates.
///
/// Deliberately conservative: bounded depth and fan-out, caches excluded, empty
/// folders ignored, and it does not descend after a hit (so a save folder's
/// subfolders do not each become an entry of their own). When the folder that hit
/// contains a more specific child, Unreal's `Saved/SaveGames` shape, the child
/// wins.
pub fn save_dirs_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if root.as_os_str().is_empty() || !root.is_dir() {
        return out;
    }
    walk(root, 1, &mut out);
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > SAVE_SCAN_MAX_DEPTH || out.len() >= SAVE_SCAN_MAX_HITS {
        return;
    }
    let subs = subdirs(dir);
    if subs.len() > SAVE_SCAN_MAX_FANOUT {
        return;
    }
    for sub in subs {
        if out.len() >= SAVE_SCAN_MAX_HITS {
            return;
        }
        let name = sub.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if is_cache_dir_name(name) {
            continue;
        }
        if looks_like_save_dir_name(name) {
            out.extend(resolve_save_dir(&sub));
            continue; // nunca desciende tras un acierto
        }
        walk(&sub, depth + 1, out);
    }
}

/// What to offer for a folder whose name hit: a container like `Saved` holding a
/// more specific `SaveGames` resolves to the child; otherwise the folder itself,
/// provided it has something in it.
fn resolve_save_dir(dir: &Path) -> Vec<PathBuf> {
    let deeper: Vec<PathBuf> = subdirs(dir)
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            looks_like_save_dir_name(name) && dir_non_empty(p)
        })
        .collect();
    if !deeper.is_empty() {
        return deeper;
    }
    if dir_non_empty(dir) {
        vec![dir.to_path_buf()]
    } else {
        Vec::new()
    }
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect()
}

fn dir_non_empty(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut r| r.next().is_some())
}

/// Roots that must NEVER be offered as a game's save folder.
///
/// Two families:
///
/// * The user profile and its top-level folders (`Documents`, `AppData/*`,
///   `Saved Games`). A loose template resolving there would propose syncing the
///   entire profile.
/// * Shared engine roots: `AppData/Roaming/RenPy` holds the saves of *every* RenPy
///   game on the machine, as do Godot, LOVE and `LocalLow/DefaultCompany`. A hit
///   has to point at the game's folder inside them; the root mixes different games
///   into one save.
///
/// Compared by exact path equality: a game's folder INSIDE a blocked root is
/// perfectly valid and must not be filtered out.
pub fn blocked_roots(os: Os) -> HashSet<PathBuf> {
    let mut out: HashSet<PathBuf> = HashSet::new();
    let mut add = |tmpl: &str| {
        for p in expand_path(tmpl, os) {
            out.insert(p);
        }
    };
    for tmpl in [
        "<home>",
        "<home>/Documents",
        "<home>/Desktop",
        "<home>/Downloads",
        "<home>/Saved Games",
        "<home>/Documents/My Games",
        "<winAppData>",
        "<winLocalAppData>",
        "<winLocalAppDataLow>",
        "<winDocuments>",
        "<winSavedGames>",
        "<winPublic>",
        "<winPublic>/Documents",
        "<winProgramData>",
        "<winLocalAppData>/Programs",
        "<winLocalAppData>/Packages",
        "<winLocalAppData>/User Data",
        "<xdgData>",
        "<xdgConfig>",
        "<xdgState>",
        // Raíces de motor compartidas.
        "<winAppData>/RenPy",
        "<winAppData>/Godot",
        "<winAppData>/Godot/app_userdata",
        "<winAppData>/LOVE",
        "<winLocalAppDataLow>/DefaultCompany",
        "<xdgData>/renpy",
        "<xdgData>/godot",
        "<xdgData>/love",
    ] {
        add(tmpl);
    }
    out
}

/// A readable reason when `path` points at a profile or system folder that can
/// never be a game's save root; `None` when it is acceptable.
///
/// It complements [`blocked_roots`], which works on paths already resolved on THIS
/// machine during detection. This one is structural: it looks at the shape of the
/// path, so it also protects what the user types by hand, what arrives from another
/// machine, and what was left poisoned in a `state.json` from before these guards
/// existed.
///
/// Tracking a root like that is not merely untidy: it hashes and uploads the whole
/// profile, and on Windows it blows up on the first legacy junction
/// (`AppData\Local\Application Data`, which points at its own parent).
pub fn dangerous_sync_root(path: &Path) -> Option<String> {
    let raw = path.to_string_lossy();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some("the save path is empty".into());
    }
    // Normalise to `/` with no trailing slash, to compare by segment.
    let p = trimmed.replace('\\', "/");
    let p = p.trim_end_matches('/');
    if p.is_empty() {
        return Some("it is the filesystem root".into());
    }
    let lower = p.to_lowercase();
    let segs: Vec<&str> = lower.split('/').filter(|s| !s.is_empty()).collect();

    // Wine and Proton prefixes are checked first and by their TAIL: one can sit
    // under `~/.local/share/Steam`, under a library on another disk, or wherever
    // Lutris or Bottles put it. What gives it away is how the path ends, not where
    // it begins.
    if let Some(reason) = dangerous_wine_prefix(&segs) {
        return Some(reason);
    }

    // Windows: `C:` como primer segmento.
    if let Some(first) = segs.first() {
        if first.len() == 2 && first.ends_with(':') {
            return dangerous_windows_root(&segs[1..]);
        }
    }
    dangerous_unix_root(&segs)
}

/// The roots of a Wine or Proton prefix. A prefix IS an entire emulated Windows:
/// its `drive_c` with the profile, `ProgramData`, the registry and everything the
/// game installed. Tracking it uploads hundreds of MB of which the save is a few
/// KB, and the rest rebuilds itself on any machine.
///
/// Reported in aug-2026: a Steam Deck ended up monitoring
/// `.../steamapps/compatdata/423230/pfx`, 308.6 MB, for a save that lives in
/// `pfx/drive_c/users/steamuser/AppData/LocalLow/TheGameBakers/Furi`.
fn dangerous_wine_prefix(segs: &[&str]) -> Option<String> {
    let say = |s: &str| Some(s.to_string());

    // Inside the prefix, `drive_c` IS a Windows root: the Windows rules already
    // know what a whole profile or a whole AppData is, so they get reused as-is
    // rather than written twice. `rposition` in case somebody nests prefixes, which
    // Bottles does.
    if let Some(i) = segs.iter().rposition(|s| *s == "drive_c") {
        if let Some(reason) = dangerous_windows_root(&segs[i + 1..]) {
            return Some(reason);
        }
    }

    match segs {
        [.., "pfx"] => say("it is a whole Wine/Proton prefix"),
        [.., "compatdata"] => say("it is Steam's whole compatibility-data folder"),
        // `compatdata/<appid>`: the container of ONE game's prefix.
        [.., "compatdata", _] => say("it is a game's whole Proton prefix folder"),
        _ => None,
    }
}

fn dangerous_windows_root(rest: &[&str]) -> Option<String> {
    let say = |s: &str| Some(s.to_string());
    match rest {
        [] => say("it is a whole drive"),
        ["windows", ..] => say("it is inside the Windows system folder"),
        ["users"] => say("it is the Users folder"),
        ["users", _] => say("it is a whole user profile folder"),
        ["users", _, "appdata"] => say("it is the whole AppData folder"),
        ["users", _, "appdata", tier] if matches!(*tier, "local" | "roaming" | "locallow") => {
            say("it is a whole application-data folder")
        }
        ["users", _, folder]
            if matches!(
                *folder,
                "documents"
                    | "desktop"
                    | "downloads"
                    | "pictures"
                    | "music"
                    | "videos"
                    | "saved games"
                    | "onedrive"
            ) =>
        {
            Some(format!("it is a whole {folder} folder"))
        }
        [only]
            if matches!(
                *only,
                "program files" | "program files (x86)" | "programdata"
            ) =>
        {
            Some(format!("it is the whole {only} folder"))
        }
        _ => None,
    }
}

fn dangerous_unix_root(segs: &[&str]) -> Option<String> {
    let say = |s: &str| Some(s.to_string());
    match segs {
        [] => say("it is the filesystem root"),
        [only]
            if matches!(
                *only,
                "home" | "root" | "etc" | "usr" | "var" | "tmp" | "opt"
            ) =>
        {
            say("it is a system folder")
        }
        ["home", _] => say("it is a whole home folder"),
        ["home", _, dir] if matches!(*dir, ".config" | ".local" | ".steam" | ".var") => {
            Some(format!("it is a whole {dir} folder"))
        }
        ["home", _, ".local", "share"] => say("it is a whole .local/share folder"),
        ["home", _, dir]
            if matches!(
                *dir,
                "documents" | "desktop" | "downloads" | "pictures" | "music" | "videos"
            ) =>
        {
            Some(format!("it is a whole {dir} folder"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_roots_are_refused_on_both_platforms() {
        for (p, hint) in [
            ("C:\\", "drive"),
            ("C:\\Windows\\System32", "windows"),
            ("C:\\Users", "users"),
            ("C:\\Users\\jacka", "profile"),
            ("C:\\Users\\jacka\\AppData", "appdata"),
            ("C:\\Users\\jacka\\AppData\\Roaming", "application-data"),
            ("C:\\Users\\jacka\\Documents", "documents"),
            ("C:\\Users\\jacka\\Saved Games", "saved games"),
            ("C:\\Program Files (x86)", "program files"),
            ("/", "filesystem root"),
            ("/home", "system folder"),
            ("/usr", "system folder"),
            ("/home/insider", "home folder"),
            ("/home/insider/.local/share", ".local/share"),
            ("/home/insider/.config", ".config"),
            ("/home/insider/Documents", "documents"),
            // Wine and Proton prefixes, the Steam Deck case (aug-2026). They are
            // recognised by their tail, so they hold in any library.
            (
                "/home/clock/.local/share/Steam/steamapps/compatdata/423230/pfx",
                "prefix",
            ),
            ("/mnt/juegos/SteamLibrary/steamapps/compatdata/620/pfx", "prefix"),
            ("/home/clock/.local/share/Steam/steamapps/compatdata/423230", "prefix"),
            ("/home/clock/.local/share/Steam/steamapps/compatdata", "compatibility-data"),
            // And inside the prefix the Windows rules take over: `drive_c` is a
            // whole drive, and its profile a whole profile.
            (
                "/home/clock/.local/share/Steam/steamapps/compatdata/423230/pfx/drive_c",
                "drive",
            ),
            (
                "/home/clock/.local/share/Steam/steamapps/compatdata/423230/pfx/drive_c/users/steamuser",
                "profile",
            ),
            (
                "/home/clock/.local/share/Steam/steamapps/compatdata/423230/pfx/drive_c/users/steamuser/AppData/LocalLow",
                "application-data",
            ),
        ] {
            let reason = dangerous_sync_root(Path::new(p));
            assert!(reason.is_some(), "{p} debería rechazarse");
            assert!(
                reason.as_deref().unwrap().to_lowercase().contains(hint),
                "{p}: motivo poco claro → {reason:?}"
            );
        }
    }

    #[test]
    fn a_real_save_folder_passes() {
        for p in [
            "C:\\Users\\jacka\\AppData\\Roaming\\GSE Saves\\413150\\remote",
            "C:\\Users\\jacka\\Documents\\My Games\\Skyrim\\Saves",
            "C:\\Users\\jacka\\Saved Games\\Planet S",
            "/home/insider/.local/share/Steam/userdata/1/413150/remote",
            "/home/insider/.config/unity3d/Studio/Game",
            "/home/insider/Documents/My Games/EU5/save games",
            "/mnt/ssd/Games/Factorio/saves",
            // The good folder INSIDE the prefix: it is the destination the "pick
            // the game's own save folder inside it" message points at, so
            // rejecting it would turn the guard into a dead end.
            "/home/clock/.local/share/Steam/steamapps/compatdata/423230/pfx/drive_c/users/steamuser/AppData/LocalLow/TheGameBakers/Furi",
            "/home/clock/.local/share/Steam/steamapps/compatdata/620/pfx/drive_c/users/steamuser/Saved Games/Portal2",
        ] {
            assert!(
                dangerous_sync_root(Path::new(p)).is_none(),
                "{p} debería aceptarse: {:?}",
                dangerous_sync_root(Path::new(p))
            );
        }
    }

    #[test]
    fn a_trailing_slash_or_mixed_separators_dont_sneak_past() {
        assert!(dangerous_sync_root(Path::new("C:\\Users\\jacka\\")).is_some());
        assert!(dangerous_sync_root(Path::new("C:/Users/jacka")).is_some());
        assert!(dangerous_sync_root(Path::new("/home/insider/")).is_some());
        assert!(dangerous_sync_root(Path::new("")).is_some());
    }

    #[test]
    fn cache_matches_every_spelling_and_the_prefixed_variants() {
        for n in [
            "cache",
            "Cache",
            "shadercache",
            "Shader Cache",
            "shader_cache",
            "ShaderCache",
            "DX12Cache",
            "AnvilDX12Cache",
            "FortniteShaderCache",
            "DerivedDataCache",
            "crashdumps",
            "Logs",
            "temp",
        ] {
            assert!(is_cache_dir_name(n), "{n} debería ser caché");
        }
        for n in ["saves", "SaveGames", "profiles", "slot1", "Documents", ""] {
            assert!(!is_cache_dir_name(n), "{n} NO debería ser caché");
        }
    }

    #[test]
    fn a_cache_named_save_is_still_a_cache() {
        // Order matters: cache is checked before save.
        assert!(is_cache_dir_name("SaveCache"));
        assert!(!looks_like_save_dir_name("SaveCache"));
    }

    #[test]
    fn save_names_ignore_case_and_separators() {
        for n in [
            "saves",
            "SAVE",
            "Save Games",
            "save_data",
            "SaveData",
            "autosave",
        ] {
            assert!(looks_like_save_dir_name(n), "{n} debería parecer save");
        }
        for n in ["config", "binaries", "shaders"] {
            assert!(!looks_like_save_dir_name(n));
        }
    }

    #[test]
    fn backup_suffix_matches_only_at_the_end() {
        // Suffix: yes, whatever the separator or case, the bare
        // word `Backup`, which IS the suffix.
        for n in [
            "SaveGamesBackup",
            "saves_backup",
            "Saves-Backup",
            "NobodyT-bak",
            "slot.bak",
            "SavesOld",
            "backups",
            "Backup",
        ] {
            assert!(ends_with_backup_suffix(n), "{n} ends in a copy suffix");
        }
        // Prefix or unrelated word: NO. `BackupSaves` is a save folder.
        for n in ["BackupSaves", "saves", "SaveGames", "autosave"] {
            assert!(!ends_with_backup_suffix(n), "{n} is not a copy by name");
        }
    }

    /// The names below are not invented: every one is a real save-folder leaf
    /// from the Ludusavi catalog whose letters happen to end in "old". A first
    /// cut of this rule compared the separator-stripped name and condemned all
    /// of them. `savegold` is the one that proves the cost: it is
    /// `wildlife-park-gold-remastered`'s ONLY save path, a folder of `.sav`
    /// files that clears the rotating-content gate, so the penalty would have
    /// applied with nothing to back it up.
    ///
    /// A regression here is invisible to the name-recall benchmark (that one
    /// measures the positive vocabulary, which this rule never touches), so
    /// the corpus has to live as its own test.
    #[test]
    fn real_catalog_names_ending_in_old_are_not_copies() {
        for n in [
            "Sunday Gold",
            "Making History Gold",
            "Trolley_Gold",
            "Hegemony Gold",
            "savegold",
            "rescuequestgold",
            "Stranglehold",
            "Stikbold",
            "Faerie Solitaire Harvest Defold",
            "Castle of Heart_ Retold",
            "jp.konami.mac.FroggerTTGold",
            "Blake Stone - Aliens of Gold",
        ] {
            assert!(
                !ends_with_backup_suffix(n),
                "{n} is a real game's save folder, not a backup copy"
            );
        }
        // The boundary is what separates them from the genuine articles.
        for n in ["Saves_Old", "SavesOld", "saves old", "old"] {
            assert!(ends_with_backup_suffix(n), "{n} really is a copy marker");
        }
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn finds_a_save_dir_next_to_the_game_and_prefers_the_specific_child() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Game");
        // El caso Ubisoft: <install>/savegames/<id numérico>.
        touch(&install.join("savegames/1234567/save.dat"));
        // Y el de Unreal, un nivel más abajo.
        touch(&install.join("Binaries/Saved/SaveGames/slot.sav"));
        // Noise that must not come out.
        touch(&install.join("ShaderCache/x.bin"));
        touch(&install.join("Content/audio/track.ogg"));

        let mut found = save_dirs_under(&install);
        found.sort();
        assert_eq!(
            found,
            vec![
                install.join("Binaries/Saved/SaveGames"),
                install.join("savegames"),
            ],
            "esperado el save de Ubisoft y el de Unreal, sin caché"
        );
    }

    #[test]
    fn an_empty_save_dir_is_not_offered() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("Game/saves")).unwrap();
        assert!(save_dirs_under(&tmp.path().join("Game")).is_empty());
    }

    #[test]
    fn the_walk_is_bounded_by_depth() {
        let tmp = tempfile::tempdir().unwrap();
        // Four levels below the root: out of reach.
        touch(&tmp.path().join("a/b/c/d/saves/x.sav"));
        assert!(save_dirs_under(tmp.path()).is_empty());
    }

    #[test]
    fn blocked_roots_cover_the_profile_but_not_a_game_inside_it() {
        let roots = blocked_roots(Os::current());
        if let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
            assert!(roots.contains(&home), "el home debe estar bloqueado");
            assert!(
                !roots.contains(&home.join("Documents/My Games/Skyrim")),
                "la carpeta de UN juego dentro de una raíz bloqueada es válida"
            );
        }
    }
}
