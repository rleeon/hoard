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
    // Graphics API caches.
    "dx12cache",
    "dxcache",
    "d3dcache",
    "d3dscache",
    "dxil",
    "dxbc",
    "pipelinecache",
    "psocache",
    // Engine and vendor caches.
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
    // Generic ones and regenerated state.
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

/// The cache flavours that are transient rather than the game's own weight: what
/// a process writes while it runs and recreates without being asked. A subset of
/// [`CACHE_DIR_NAMES`], split out for [`holds_foreign_subdir`] alone.
///
/// Everywhere else the distinction would be noise: none of these is a save and
/// none should be uploaded, which is what the rest of the pipeline asks. It only
/// matters when the question is whether a folder is the GAME's or its SAVES'. A
/// `ShaderCache\` of 400 MB answers that; a `Logs\` does not, since a folder full
/// of save slots keeps one just as readily as a game's root does.
const TRANSIENT_DIR_NAMES: &[&str] = &[
    "temp",
    "tmp",
    "logs",
    "log",
    "crashes",
    "crashdumps",
    "crashreports",
];

/// `true` when the name is one of the transient flavours of cache, which carry no
/// weight as evidence. See [`TRANSIENT_DIR_NAMES`].
fn is_transient_dir_name(name: &str) -> bool {
    TRANSIENT_DIR_NAMES.contains(&normalize_dir_name(name).as_str())
}

/// `true` when `dir` holds at least one entry of any kind, file or folder.
///
/// An unreadable folder answers `false`: not being able to look is not evidence
/// of anything, and the caller's default is to keep backing things up.
fn dir_is_populated(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// `true` when `dir` has, directly below it, a **populated** folder of added
/// content or heavy cache: the sign that `dir` is the game's folder rather than
/// its saves' folder, and that adopting it whole would drag in hundreds of
/// megabytes nobody asked for.
///
/// Both conditions are there because the two ways of being wrong do not cost the
/// same. Answering `true` does not widen to the parent, it narrows to the single
/// file the catalog named, so every sibling save in that folder silently stops
/// being backed up. Answering `false` uploads some junk, which the user can see
/// and complain about. Noisy when too lax, silent when too strict: that is what
/// sets the bar at
///
/// * a name that is added content ([`FOREIGN_DIR_NAMES`]) or a cache with real
///   weight behind it. The transient ones do not count
///   ([`TRANSIENT_DIR_NAMES`]): a `Logs\` sits next to save slots as readily as
///   next to a game's binaries, so it settles nothing and would cost the
///   siblings;
/// * **and** a folder with something in it. An empty `mods\` is what a game
///   creates on first run, and adopting a parent because of one costs nothing.
///
/// One level only, on both counts: what is being decided is whether `dir` itself
/// gets offered, and a `mods\` buried three levels down does not change that
/// answer. "Populated" is likewise a single `read_dir`, so issue #17's
/// `mods\promo\` of artwork counts through its subfolder without anyone walking
/// it. An IO error answers `false`, since not being able to read a folder is no
/// reason to stop backing it up.
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
        let carries_weight = is_foreign_dir_name(&name)
            || (is_cache_dir_name(&name) && !is_transient_dir_name(&name));
        if carries_weight && dir_is_populated(&entry.path()) {
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
        // Shared engine roots.
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

// ---- what a folder is, when it is not a save folder

/// A folder that holds something other than saves, whole. What detection and the
/// manual add look for before offering or accepting one, and what the backup
/// narrows down when a tracked folder turns out to be one.
///
/// It exists because of what Hoard Cloud held on 2026-09-26: of 391 GB in the
/// latest versions, 312 GB belonged to 90 saves that were really a game's
/// installation (Unity `_Data` trees, Unreal `.pak`s, executables), a repack's
/// installer or a whole Wine prefix, and in most of them the real save was not
/// even there: the 1 GB cap of the free plan had filled up with game files first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderKind {
    /// A game's installation: the executable and its data.
    Install,
    /// An installer or repack waiting to be run (`setup.exe` next to its parts).
    Installer,
    /// A whole Wine/Proton prefix, `drive_c` and all.
    WinePrefix,
}

/// Names that only turn up in a game's installation. Any of them on its own is
/// enough.
const INSTALL_MARKER_FILES: &[&str] = &[
    "unityplayer.dll",
    "unityplayer.so",
    "unitycrashhandler64.exe",
    "unitycrashhandler32.exe",
    "steam_api.dll",
    "steam_api64.dll",
    "libsteam_api.so",
    "unins000.exe",
    "unins000.dat",
];

/// Reads what sits directly in `dir` (one listing, a couple of stats) and says
/// whether it is an installation, an installer or a Wine prefix. `None` for
/// everything else, save folders included.
///
/// Only the folder's own children count. A save folder that keeps a tool in a
/// subfolder, PunkBuster's `pb/*.dll` next to Ghost Recon's saves or Final
/// Fantasy X's `MemorySumChecker/`, is still a save folder.
pub fn folder_kind(dir: &Path) -> Option<FolderKind> {
    let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir).ok()?.flatten().collect();
    if let Some(kind) = kind_here(dir, &entries, true) {
        return Some(kind);
    }
    // A wrapper around one, tracked a level too high: a repack's folder with the
    // game inside next to its readme, links and redistributables
    // (`AnkerGames ... .url`, `Run Me!.bat`, `Teardown/`, or IGG's `game/`). 34 of
    // the cloud's installs looked like that on 2026-09-27 and none had a
    // program of its own at the top. A handful of subfolders at most: this runs
    // on every walk of the folder. Only the unmistakable signs count down there:
    // two DLLs in a subfolder are PunkBuster next to Ghost Recon's saves as
    // often as they are a game.
    entries
        .iter()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .take(16)
        .filter_map(|e| {
            let inner: Vec<std::fs::DirEntry> =
                std::fs::read_dir(e.path()).ok()?.flatten().collect();
            kind_here(&e.path(), &inner, false)
        })
        .find(|k| matches!(k, FolderKind::Install | FolderKind::Installer))
}

/// [`folder_kind`] for `dir` alone. `loose` also accepts the weakest sign, two
/// programs side by side.
fn kind_here(dir: &Path, entries: &[std::fs::DirEntry], loose: bool) -> Option<FolderKind> {
    let names: Vec<(String, bool)> = entries
        .iter()
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (e.file_name().to_string_lossy().to_lowercase(), is_dir)
        })
        .collect();
    let has_dir = |n: &str| names.iter().any(|(name, d)| *d && name == n);
    let has_file = |n: &str| names.iter().any(|(name, d)| !*d && name == n);

    if has_dir("drive_c")
        && (has_dir("dosdevices") || has_file("system.reg") || has_file("user.reg"))
    {
        return Some(FolderKind::WinePrefix);
    }

    let ext = |name: &str| {
        name.rsplit_once('.')
            .map(|(_, e)| e.to_string())
            .unwrap_or_default()
    };
    let is_setup = |name: &str| matches!(name, "setup.exe" | "install.exe" | "installer.exe");
    if names.iter().any(|(n, d)| !*d && is_setup(n))
        && names
            .iter()
            .any(|(n, d)| !*d && matches!(ext(n).as_str(), "bin" | "7z" | "rar" | "msi" | "cab"))
    {
        return Some(FolderKind::Installer);
    }

    // An uninstaller alone is weak: Need for Speed: The Run keeps one in
    // `Documents/NFSTR/Uninstall`, next to its settings.
    if names.iter().any(|(n, d)| {
        !*d && INSTALL_MARKER_FILES.contains(&n.as_str()) && (loose || !n.starts_with("unins"))
    }) {
        return Some(FolderKind::Install);
    }
    // Unity: `<Game>_Data/` with its managed code or its asset index.
    let unity = entries.iter().any(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        name.ends_with("_data")
            && e.path().is_dir()
            && std::fs::read_dir(e.path()).is_ok_and(|inner| {
                inner.flatten().any(|c| {
                    matches!(
                        c.file_name().to_string_lossy().to_lowercase().as_str(),
                        "managed" | "globalgamemanagers" | "resources.assets" | "data.unity3d"
                    )
                })
            })
    });
    // Unreal: `Engine/Binaries`, or a `Content/Paks` of its own.
    let unreal = dir.join("Engine").join("Binaries").is_dir()
        || dir.join("engine").join("binaries").is_dir()
        || dir.join("Content").join("Paks").is_dir();
    if unity || unreal {
        return Some(FolderKind::Install);
    }
    // Two programs or libraries side by side: no save folder looks like that.
    let programs = names
        .iter()
        .filter(|(n, d)| {
            !*d && (matches!(ext(n).as_str(), "exe" | "dll" | "so" | "dylib")
                || n.ends_with(".x86_64")
                || n.ends_with(".x86"))
        })
        .count();
    (loose && programs >= 2).then_some(FolderKind::Install)
}

/// Extensions of a game's own payload: code, packed assets, video, music, fonts.
/// Nothing a game writes as a save, as long as the folder is an installation;
/// in an ordinary save folder some of them are saves (Telltale keeps its slots
/// as `.bundle`), which is why this is only ever applied to [`FolderKind`]
/// folders.
const PAYLOAD_EXTS: &[&str] = &[
    "exe",
    "dll",
    "so",
    "dylib",
    "x86_64",
    "x86",
    "pdb",
    "msi",
    "cab",
    "pak",
    "pak2",
    "ucas",
    "utoc",
    "ress",
    "resource",
    "assets",
    "bundle",
    "unity3d",
    "forge",
    "rpf",
    "vpk",
    "scs",
    "paz",
    "bk2",
    "bik",
    "usm",
    "bank",
    "wem",
    "pck",
    "arc",
    "big",
    "bsa",
    "ba2",
    "cpk",
    "uasset",
    "umap",
    "ushaderprecache",
    "upk",
    "xnb",
    "fsb",
    "gpk",
    "tfc",
    "ttf",
    "otf",
    "mp4",
    "wmv",
    "webm",
    "avi",
    "mkv",
    "mov",
    "mp3",
    "ogg",
    "flac",
    "wav",
    "m4a",
    "iso",
    "mdf",
    "mds",
    "cache",
    "cache2",
    "tga",
    "dds",
    "fbx",
    "ttc",
    "pdf",
    "psarc",
    "psarc_s",
];

/// Inside an installation, a file this large outside a save folder is the game's
/// data, whatever its extension says: MGSV's `master/*.dat` (3.5 GB each),
/// Enshrouded's `*.dat`, Dragon's Dogma 2's `shader.cache`, a 545 MB art book.
/// Saves that big live in a save folder, which this never touches.
pub const PAYLOAD_MIN_BYTES: u64 = 64 * 1024 * 1024;

/// Folders inside an installation that hold nothing but the game itself.
const PAYLOAD_DIR_SUFFIXES: &[&str] = &["_data"];
const PAYLOAD_DIRS: &[&str] = &[
    "engine",
    "binaries",
    "content",
    "_commonredist",
    "commonredist",
    "redist",
    "_redist",
    "directx",
    "monobleedingedge",
    "__installer",
];

/// Inside a Wine prefix, the parts that are Windows rather than anybody's data.
/// `Program Files` is not one of them: old games keep their saves beside the
/// executable (`<install>/save/`), so a game installed into the prefix is
/// narrowed file by file like any other install instead of skipped whole.
const PREFIX_SYSTEM_DIRS: &[&[&str]] = &[
    &["dosdevices"],
    &["drive_c", "windows"],
    &["drive_c", "programdata", "microsoft"],
];

/// Whether `rel_dir`, a folder inside a Wine prefix, is Windows itself: the one
/// kind of folder a walk may skip without looking inside.
pub fn is_prefix_system_dir(rel_dir: &str) -> bool {
    let lower = rel_dir.replace('\\', "/").to_lowercase();
    let dirs: Vec<&str> = lower.split('/').filter(|p| !p.is_empty()).collect();
    in_prefix_system_dir(&dirs)
}

fn in_prefix_system_dir(dirs: &[&str]) -> bool {
    PREFIX_SYSTEM_DIRS
        .iter()
        .any(|sys| dirs.len() >= sys.len() && dirs[..sys.len()] == **sys)
}

/// Whether `rel`, a path inside a folder of `kind`, is part of what the folder
/// holds besides saves: the game's payload in an installation or an installer,
/// Windows itself in a prefix and the payload of any game installed inside it.
pub fn is_payload(kind: FolderKind, rel: &str, size: u64) -> bool {
    let lower = rel.replace('\\', "/").to_lowercase();
    let parts: Vec<&str> = lower.split('/').filter(|p| !p.is_empty()).collect();
    let Some((file, dirs)) = parts.split_last() else {
        return false;
    };
    if kind == FolderKind::WinePrefix && in_prefix_system_dir(dirs) {
        return true;
    }
    let ext = file.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    if PAYLOAD_EXTS.contains(&ext) {
        return true;
    }
    // Some games do save inside their install, under a folder named for it
    // (`<install>/Binaries/Saves`, `<install>/savegames/<id>`): that folder is
    // theirs, whatever it hangs off.
    if dirs.iter().any(|d| looks_like_save_dir_name(d)) {
        return false;
    }
    // Split archives (`data.041`, `game.000`): three digits and nothing else.
    if ext.len() >= 3 && ext.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    if size >= PAYLOAD_MIN_BYTES {
        return true;
    }
    dirs.iter()
        .any(|d| PAYLOAD_DIRS.contains(d) || PAYLOAD_DIR_SUFFIXES.iter().any(|s| d.ends_with(s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_kind_tells_installs_installers_and_prefixes_from_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let mk = |rel: &str| {
            let p = tmp.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"x").unwrap();
        };
        // Galak-Z as it reached the cloud: a Unity install with the save beside it.
        mk("galak/Galak-Z.x86_64");
        mk("galak/Galak-Z_Data/Managed/Assembly-CSharp.dll");
        mk("galak/SaveData.dat");
        assert_eq!(
            folder_kind(&tmp.path().join("galak")),
            Some(FolderKind::Install)
        );
        // Unreal.
        mk("sifu/Engine/Binaries/ThirdParty/x.dll");
        mk("sifu/Sifu.exe");
        assert_eq!(
            folder_kind(&tmp.path().join("sifu")),
            Some(FolderKind::Install)
        );
        // A repack waiting to be installed.
        mk("repack/setup.exe");
        mk("repack/fg-01.bin");
        assert_eq!(
            folder_kind(&tmp.path().join("repack")),
            Some(FolderKind::Installer)
        );
        // A Heroic prefix.
        mk("prefix/drive_c/users/steamuser/Saved Games/x.sav");
        mk("prefix/system.reg");
        assert_eq!(
            folder_kind(&tmp.path().join("prefix")),
            Some(FolderKind::WinePrefix)
        );
        // Saves stay saves, even with a tool in a subfolder (Ghost Recon's PunkBuster).
        mk("grfs/pb/pbcl.dll");
        mk("grfs/pb/pbags.dll");
        mk("grfs/savegame1.sav");
        assert_eq!(folder_kind(&tmp.path().join("grfs")), None);
        // Telltale saves are `.bundle`, and their folder is not an install.
        mk("wolf/_saveslot1_autosave.bundle");
        assert_eq!(folder_kind(&tmp.path().join("wolf")), None);
        // A repack's wrapper: the install is one level down.
        mk("anker/Read Me.txt");
        mk("anker/Run Me!.bat");
        mk("anker/Teardown/teardown.exe");
        mk("anker/Teardown/steam_api64.dll");
        assert_eq!(
            folder_kind(&tmp.path().join("anker")),
            Some(FolderKind::Install)
        );
        // An uninstaller in a subfolder is not a game.
        mk("nfstr/settings/profile.dat");
        mk("nfstr/Uninstall/unins000.exe");
        assert_eq!(folder_kind(&tmp.path().join("nfstr")), None);
    }

    #[test]
    fn payload_is_the_game_not_the_save() {
        use FolderKind::*;
        assert!(is_payload(
            Install,
            "Galak-Z_Data/StreamingAssets/Bundles/x.unity3d",
            1
        ));
        assert!(is_payload(Install, "Sifu/Binaries/Win64/x.dll", 1));
        assert!(is_payload(
            Install,
            "Hollow Knight - Official Soundtrack/01.mp3",
            1
        ));
        assert!(!is_payload(Install, "SaveData.dat", 1));
        assert!(!is_payload(Install, "SaveFile.gwsave", 1));
        assert!(!is_payload(Install, "savegames/1234567/1.save", 1));
        assert!(!is_payload(Install, "Binaries/Saves/slot1.dat", 1));
        assert!(is_payload(Install, "Binaries/Win64/config.json", 1));
        // Huge files and split archives, but never inside a save folder.
        assert!(is_payload(Install, "master/texture0.dat", 3_500_000_000));
        assert!(is_payload(Install, "Data/data/data.041", 1));
        assert!(!is_payload(Install, "SaveData.dat", 1));
        assert!(!is_payload(Install, "Saves/world.dat", 3_500_000_000));
        assert!(is_payload(
            WinePrefix,
            "drive_c/windows/system32/d3d9.dll",
            1
        ));
        assert!(is_payload(WinePrefix, "dosdevices/c:", 1));
        assert!(!is_payload(
            WinePrefix,
            "drive_c/users/steamuser/Saved Games/Beyond Good & Evil/global.sav",
            1
        ));
        // A game installed into the prefix is an install like any other.
        assert!(!is_payload(
            WinePrefix,
            "drive_c/Program Files (x86)/Old Game/SAVE/slot1.sav",
            1
        ));
        assert!(is_payload(
            WinePrefix,
            "drive_c/Program Files (x86)/Old Game/oldgame.exe",
            1
        ));
        assert!(is_prefix_system_dir("drive_c/windows/system32"));
        assert!(!is_prefix_system_dir("drive_c/Program Files"));
        assert!(!is_prefix_system_dir(
            "drive_c/users/steamuser/AppData/Local/Game/Content"
        ));
    }

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
            assert!(reason.is_some(), "{p} should be rejected");
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
                "{p} should be accepted: {:?}",
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
            assert!(is_cache_dir_name(n), "{n} should be a cache");
        }
        for n in ["saves", "SaveGames", "profiles", "slot1", "Documents", ""] {
            assert!(!is_cache_dir_name(n), "{n} should NOT be a cache");
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
            assert!(looks_like_save_dir_name(n), "{n} should look like a save");
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
        // The Ubisoft case: <install>/savegames/<numeric id>.
        touch(&install.join("savegames/1234567/save.dat"));
        // And the Unreal one, a level further down.
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
            "expected the Ubisoft save and the Unreal one, with no cache"
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
                "the folder of ONE game inside a blocked root is valid"
            );
        }
    }

    /// `TRANSIENT_DIR_NAMES` is a subset of `CACHE_DIR_NAMES`, and subtracting a
    /// name that was never in the other list would quietly subtract nothing.
    #[test]
    fn every_transient_name_is_also_a_cache_name() {
        for n in TRANSIENT_DIR_NAMES {
            assert!(
                CACHE_DIR_NAMES.contains(n),
                "{n} is subtracted from a list it is not in"
            );
        }
    }

    /// Issue #17's shape: the artwork lives one level further down, in
    /// `mods\promo\`, so the check has to count a subfolder as content.
    #[test]
    fn a_populated_mods_folder_marks_the_game_folder() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("mods").join("promo")).unwrap();
        touch(&tmp.path().join("savegame.xml"));
        assert!(holds_foreign_subdir(tmp.path()));
    }

    /// The heavy caches still count, and they usually arrive with content.
    #[test]
    fn a_populated_shader_cache_marks_the_game_folder() {
        let tmp = tempfile::tempdir().unwrap();
        touch(&tmp.path().join("ShaderCache/pipeline.bin"));
        assert!(holds_foreign_subdir(tmp.path()));
    }

    /// An empty one is what a game creates on first run: no weight, no verdict.
    /// Narrowing on it would cost the folder's other saves for nothing.
    #[test]
    fn an_empty_foreign_folder_is_not_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("mods")).unwrap();
        std::fs::create_dir_all(tmp.path().join("ShaderCache")).unwrap();
        touch(&tmp.path().join("savegame.xml"));
        assert!(!holds_foreign_subdir(tmp.path()));
    }

    /// Logs, temp and crash dumps are cache everywhere else in the pipeline, and
    /// rightly so, but they say nothing about whose folder this is: a folder full
    /// of save slots keeps them too.
    #[test]
    fn transient_folders_do_not_decide_whose_folder_this_is() {
        for n in ["Logs", "log", "Temp", "tmp", "CrashDumps", "crash reports"] {
            let tmp = tempfile::tempdir().unwrap();
            touch(&tmp.path().join(n).join("run.txt"));
            touch(&tmp.path().join("slot1.sav"));
            assert!(
                !holds_foreign_subdir(tmp.path()),
                "{n} must not narrow the folder down to one file"
            );
        }
    }

    /// The rest of the pipeline keeps treating them as cache: this split is local
    /// to `holds_foreign_subdir` and must not leak into the walk's skip rule.
    #[test]
    fn transient_folders_are_still_cache_for_everyone_else() {
        for n in ["Logs", "Temp", "CrashDumps"] {
            assert!(is_cache_dir_name(n), "{n} is still cache");
        }
    }

    /// A name that announces saves wins over the foreign list, populated or not.
    #[test]
    fn a_save_named_folder_is_never_foreign_content() {
        let tmp = tempfile::tempdir().unwrap();
        touch(&tmp.path().join("SaveMods/slot1.sav"));
        assert!(!holds_foreign_subdir(tmp.path()));
    }
}
