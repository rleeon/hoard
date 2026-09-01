//! Detect Steam libraries and the games installed inside them.
//!
//! Steam stores its library list in `<steam>/steamapps/libraryfolders.vdf`.
//! Each library directory in turn contains `appmanifest_<appid>.acf` files
//! describing installed games. Both formats are Valve's KeyValues syntax,
//! which is conceptually a JSON-ish nested map; we parse it with a
//! minimal tokenizer rather than pulling in a full VDF crate.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::Os;

/// One installed Steam game with the bits we care about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamApp {
    pub app_id: u64,
    pub name: String,
    pub install_dir: PathBuf,
}

/// A Proton/Wine prefix that Steam created for one Windows-only game.
///
/// Steam stores per-game Proton prefixes under
/// `<library>/steamapps/compatdata/<appid>/pfx/`. The contents mirror a
/// Windows `C:\` drive (under `drive_c/`), so the Windows save-path
/// templates from Ludusavi can be expanded against this root to find the
/// game's saves on Linux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonPrefix {
    pub app_id: u64,
    /// Absolute path to the `pfx/` directory itself.
    pub prefix_root: PathBuf,
}

/// Probe the host for Steam libraries. Returns the directories that contain
/// a `steamapps` subfolder; later passes scan those.
///
/// On Linux the lookup considers the native install (`~/.steam/steam`), the
/// Flatpak path (`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`) and
/// the Snap path (`~/snap/steam/common/.local/share/Steam`).
/// On Windows we read Steam's own install path from the registry (so a
/// non-default drive is covered) and fall back to `%PROGRAMFILES(X86)%/Steam`
/// and `%PROGRAMFILES%/Steam`.
/// On macOS we check `~/Library/Application Support/Steam`.
pub fn detect_steam_libraries(os: Os) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = match os {
        Os::Linux => linux_roots(),
        Os::Windows => windows_roots(),
        Os::Mac => mac_roots(),
    };
    // Beyond the well-known locations, sweep the disks for second Steam
    // installs / custom-named libraries the registry and home paths miss.
    roots.extend(scan_steam_roots(os));

    // The root itself always doubles as a library: Steam ships its own
    // `steamapps` there. Then any extra libraries are listed inside
    // libraryfolders.vdf. `seen` de-dups by a normalised key so the same
    // directory reached two ways (e.g. registry root vs `%ProgramFiles%`
    // guess on Windows) is only counted once.
    let mut libraries: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for root in &roots {
        if root.join("steamapps").is_dir() && seen.insert(lib_key(root)) {
            libraries.push(root.clone());
        }
        let vdf = root.join("steamapps/libraryfolders.vdf");
        let text = match std::fs::read_to_string(&vdf) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for extra in parse_library_folders(&text) {
            if !extra.join("steamapps").is_dir() {
                // A library that libraryfolders.vdf lists but whose steamapps
                // dir isn't readable right now: almost always an unmounted
                // drive, an offline NAS/removable path, or a permissions
                // issue. This is the usual reason "only the main library
                // shows up", so log it at info so it's visible in the app.
                tracing::info!(
                    library = %extra.display(),
                    "Steam library listed in libraryfolders.vdf skipped: no readable steamapps dir (drive offline / not mounted / no permission?)"
                );
                continue;
            }
            if seen.insert(lib_key(&extra)) {
                libraries.push(extra);
            }
        }
    }

    libraries.sort();
    libraries.dedup();
    tracing::debug!(
        ?os,
        roots = roots.len(),
        libraries = libraries.len(),
        detected = %libraries
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        "Steam library scan"
    );
    libraries
}

/// Normalised key for de-duplicating library paths.
///
/// Two spellings of the same directory have to collapse to one key, and there
/// are two ways they diverge:
///
/// * **Symlinks.** Every standard Steam install on Linux ships
///   `~/.steam/steam` as a symlink to `~/.local/share/Steam`, and both are
///   probed by [`linux_roots`]. Left as raw strings they are different keys, so
///   the same library was listed twice and every Steam Cloud save under it was
///   reported twice: the same `userdata/<id>/<appid>/remote` folder spelled
///   two ways, which the UI has no way to tell apart from two real folders.
///   Resolving the path is what makes them one key.
/// * **Case and slashes.** On Windows the registry gives
///   `c:/program files (x86)/steam` while the `%ProgramFiles%` guess gives
///   `C:\Program Files (x86)\Steam`, so fold both there.
///
/// The key is only ever a key: what goes into the library list is the caller's
/// original path, so no display or stored path gains a `\\?\` prefix or loses
/// the spelling the user knows.
fn lib_key(p: &Path) -> String {
    // A path that can't be resolved (it just vanished, or we can't traverse to
    // it) keeps its literal spelling, so the worst case is the old behaviour.
    let resolved = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let s = resolved.to_string_lossy();
    #[cfg(windows)]
    {
        s.replace('\\', "/").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        s.into_owned()
    }
}

/// Is this the storage saying the whole device isn't there, rather than one
/// file being unreadable?
///
/// The difference decides whether it's worth reading the next file in the same
/// folder. A corrupt manifest is one game; a drive that isn't plugged in is
/// every game on it, and asking each one in turn produces a log line per game
/// per sweep: 553 rows in 48 hours from a single user with `e:/steam`
/// unplugged, all of them the same fact.
///
/// Checked by raw OS code, not by `ErrorKind`: the codes that mean "no device"
/// map to `Uncategorized`, which is not matchable, and `NotFound` deliberately
/// isn't here: a missing appmanifest is Steam deleting one mid-scan, which is
/// a race and not a dead drive.
fn device_is_gone(e: &std::io::Error) -> bool {
    let Some(code) = e.raw_os_error() else {
        return false;
    };
    #[cfg(windows)]
    {
        // ERROR_NOT_READY, ERROR_BAD_NETPATH, ERROR_DEV_NOT_EXIST,
        // ERROR_NO_SUCH_DEVICE (the 433 in the reports),
        // ERROR_DEVICE_NOT_CONNECTED.
        matches!(code, 21 | 53 | 55 | 433 | 1167)
    }
    #[cfg(not(windows))]
    {
        // EIO, ENXIO, ENODEV, ESTALE (a stale NFS handle after an unmount).
        matches!(code, 5 | 6 | 19 | 116)
    }
}

/// List Steam apps installed across all detected libraries.
///
/// Errors reading individual library folders / appmanifest files are logged
/// and skipped; we never abort the whole scan because one folder is
/// missing or one manifest file is corrupt. Returning `Ok(vec![])` is a
/// normal outcome (Steam not installed); detection treats it as "no
/// Steam apps found, fall back to filesystem heuristic".
///
/// A library whose drive is absent costs **one** line per sweep, not one per
/// game: the root is probed before anything is opened, and a device-level error
/// mid-loop abandons that library instead of asking it about the next file.
pub fn list_installed_steam_games(os: Os) -> Result<Vec<SteamApp>> {
    let libraries = detect_steam_libraries(os);
    if libraries.is_empty() {
        tracing::debug!(?os, "no Steam libraries found");
        return Ok(Vec::new());
    }

    let mut out: Vec<SteamApp> = Vec::new();
    for lib in &libraries {
        let steamapps = lib.join("steamapps");
        let scan = match scan_library(&steamapps) {
            Ok(scan) => scan,
            Err(e) => {
                tracing::info!(
                    library = %steamapps.display(),
                    error = %e,
                    "skipping Steam library: its folder isn't reachable (drive offline / not mounted / no permission?)"
                );
                continue;
            }
        };
        if scan.unreadable > 0 {
            // One line for the whole library, whatever the count: the per-file
            // version of this is what filled a user's log with the same fact 553
            // times in two days.
            tracing::warn!(
                library = %steamapps.display(),
                unreadable = scan.unreadable,
                device_gone = scan.device_gone,
                error = %scan.first_error.unwrap_or_default(),
                "appmanifests in this Steam library couldn't be read; skipped"
            );
        }
        out.extend(scan.apps);
    }
    out.sort_by_key(|a| a.app_id);
    out.dedup_by(|a, b| a.app_id == b.app_id);
    tracing::info!(
        libraries = libraries.len(),
        apps = out.len(),
        "Steam scan complete"
    );
    Ok(out)
}

/// What one library's `steamapps` folder yielded, and what it couldn't.
#[derive(Debug)]
struct LibraryScan {
    apps: Vec<SteamApp>,
    /// Manifests that wouldn't read. A count and not a log line each: the caller
    /// says it once.
    unreadable: usize,
    /// The first failure's text, which is the one worth showing; the rest are
    /// the same fact about the same drive.
    first_error: Option<String>,
    /// The device answered "I'm not here" and the rest of the library was
    /// abandoned unread.
    device_gone: bool,
}

/// Read every `appmanifest_*.acf` in one library.
///
/// `Err` means the folder itself isn't reachable, which is the answer for a
/// library on a drive that isn't plugged in. That check happens **before**
/// anything inside is opened, because the alternative is asking the same absent
/// drive about every game it holds and logging each answer.
fn scan_library(steamapps: &Path) -> std::io::Result<LibraryScan> {
    // Probe the root first. A listing that never opens is the cheapest question
    // to ask, and an absent drive usually answers it.
    std::fs::metadata(steamapps)?;
    let entries = std::fs::read_dir(steamapps)?;

    let mut scan = LibraryScan {
        apps: Vec::new(),
        unreadable: 0,
        first_error: None,
        device_gone: false,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                scan.unreadable += 1;
                if scan.first_error.is_none() {
                    scan.first_error = Some(e.to_string());
                }
                // The drive went away under us. A directory listing can be
                // served from cache while the reads behind it reach the
                // hardware, which is how a root that probed fine still fails
                // here, and every remaining file in this library has the same
                // answer waiting, so stop asking for them one at a time.
                if device_is_gone(&e) {
                    scan.device_gone = true;
                    break;
                }
                continue;
            }
        };
        match parse_app_manifest(&text, steamapps) {
            Some(app) => scan.apps.push(app),
            None => {
                tracing::debug!(
                    path = %path.display(),
                    "appmanifest missing required fields (appid/installdir); skipping"
                );
            }
        }
    }
    Ok(scan)
}

/// Enumerate every Proton/Wine prefix Steam has created on this host.
///
/// Steam writes one prefix per Windows-on-Linux game under
/// `<library>/steamapps/compatdata/<appid>/pfx/`. Entries without a `pfx/`
/// subdirectory are skipped (Steam sometimes creates the appid folder
/// before the prefix itself, e.g. mid-install). Returns an empty `Vec` on
/// non-Linux hosts and when no libraries are detected; the caller treats
/// that as "no Proton games to consider".
pub fn list_proton_prefixes(os: Os) -> Vec<ProtonPrefix> {
    let libraries = detect_steam_libraries(os);
    let mut out: Vec<ProtonPrefix> = Vec::new();
    for lib in &libraries {
        let compatdata = lib.join("steamapps/compatdata");
        let entries = match std::fs::read_dir(&compatdata) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(app_id) = name.parse::<u64>() else {
                continue;
            };
            let pfx = path.join("pfx");
            if !pfx.is_dir() {
                continue;
            }
            out.push(ProtonPrefix {
                app_id,
                prefix_root: pfx,
            });
        }
    }
    out.sort_by_key(|p| p.app_id);
    out.dedup_by(|a, b| a.app_id == b.app_id);
    tracing::info!(
        libraries = libraries.len(),
        prefixes = out.len(),
        "Proton prefix scan complete"
    );
    out
}

// ---- Roots --------------------------------------------------------------

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn linux_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = home() {
        // `~/.local/share/Steam` first, `~/.steam/steam` second: on a standard
        // Linux install the second is a symlink to the first, `lib_key` folds
        // the two into one entry, and whichever is listed first is the spelling
        // every save path downstream inherits. The real directory is the better
        // one to inherit, and it survives Steam rebuilding its compatibility
        // symlinks, and it is what the rest of the system shows the user.
        v.push(h.join(".local/share/Steam"));
        v.push(h.join(".steam/steam"));
        v.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        // Snap package (canonical/steam) keeps its own HOME under ~/snap.
        v.push(h.join("snap/steam/common/.local/share/Steam"));
        v.push(h.join("snap/steam/common/.steam/steam"));
    }
    v
}

fn windows_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    // Steam's own recorded install path first: this is the only source that
    // survives a non-default install drive (`D:\Steam`, etc.). The
    // `%ProgramFiles%` guesses below are the fallback for when the registry
    // read fails.
    if let Some(reg) = windows_registry_root() {
        v.push(reg);
    }
    for var in ["ProgramFiles(x86)", "ProgramFiles", "PROGRAMFILES"] {
        if let Some(p) = std::env::var_os(var) {
            v.push(PathBuf::from(p).join("Steam"));
        }
    }
    v
}

/// Max directory depth the library scan descends below each base dir.
const SCAN_MAX_DEPTH: usize = 3;

/// Hard cap on directories inspected per base, so a pathological tree (a huge
/// system drive, a deep home) can never turn the scan into a stall. Once hit,
/// that base stops early; the well-known roots still cover the common case.
const SCAN_DIR_BUDGET: usize = 6000;

/// Directory names never worth descending into when hunting for Steam
/// libraries: OS/system trees and known-huge noise. Matched case-insensitively.
const SCAN_SKIP_DIRS: &[&str] = &[
    // Windows.
    "windows",
    "windows.old",
    "$recycle.bin",
    "system volume information",
    "programdata",
    "appdata",
    "msocache",
    "recovery",
    "$winreagent",
    // Unix / macOS system + noise.
    "proc",
    "sys",
    "dev",
    "run",
    "boot",
    "var",
    "usr",
    "lib",
    "lib64",
    "node_modules",
    ".git",
];

/// `true` if a directory name should not be descended into during the scan.
/// Skips dot-directories (unix hidden) and the explicit system/noise list,
/// but never `steamapps` itself, which is the thing we're looking for.
fn is_skippable_scan_dir(name: &str) -> bool {
    if name == "steamapps" {
        return false;
    }
    if name.starts_with('.') {
        return true;
    }
    let lower = name.to_lowercase();
    SCAN_SKIP_DIRS.iter().any(|d| *d == lower)
}

/// Base directories to sweep for Steam libraries the well-known paths miss.
///
/// Cross-platform by design; only the set of bases differs:
/// - **Windows**: every present drive root `C:`–`Z:` (second installs and
///   custom-named libraries live on other drives).
/// - **macOS**: `/Volumes` (external/extra disks) plus `$HOME`.
/// - **Linux/SteamOS**: the usual removable/extra mount points
///   (`/mnt`, `/media`, `/run/media`) plus `$HOME`.
///
/// Only meaningful for the host OS, so it returns empty when `os` isn't the
/// one we're running on (keeps cross-OS unit tests from touching real disks).
fn scan_bases(os: Os) -> Vec<PathBuf> {
    if os != Os::current() {
        return Vec::new();
    }
    #[cfg(windows)]
    {
        return (b'C'..=b'Z')
            .map(|l| PathBuf::from(format!("{}:\\", l as char)))
            .filter(|p| p.is_dir())
            .collect();
    }
    #[cfg(target_os = "macos")]
    {
        let mut v = vec![PathBuf::from("/Volumes")];
        if let Some(h) = home() {
            v.push(h);
        }
        return v;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut v = vec![
            PathBuf::from("/mnt"),
            PathBuf::from("/media"),
            PathBuf::from("/run/media"),
        ];
        if let Some(h) = home() {
            v.push(h);
        }
        return v;
    }
    #[allow(unreachable_code)]
    Vec::new()
}

/// Steam libraries found by sweeping the disks. Catches second and third Steam
/// installs and custom-named library folders on other drives/mounts that
/// neither the registry (Windows) nor the well-known home paths record. A
/// Steam library is any directory that holds a `steamapps` subfolder; we
/// breadth-first walk each base up to [`SCAN_MAX_DEPTH`], record matches, and
/// prune once found (a library never nests inside another). Memoised: the
/// sweep (and its discovery logs) runs once per process, so a disk mounted after
/// launch is picked up on the next run, the same contract the catalog uses.
fn scan_steam_roots(os: Os) -> Vec<PathBuf> {
    use std::sync::OnceLock;
    static SCAN: OnceLock<Vec<PathBuf>> = OnceLock::new();
    SCAN.get_or_init(|| {
        let mut out: Vec<PathBuf> = Vec::new();
        for base in scan_bases(os) {
            find_steam_libraries_under(&base, &mut out);
        }
        out
    })
    .clone()
}

/// Breadth-first walk `base` up to [`SCAN_MAX_DEPTH`], pushing every directory
/// that holds a `steamapps` subfolder into `out`. Prunes once a library is
/// found (they don't nest), skips system/hidden dirs, doesn't follow symlinks
/// (no cycles), and stops a runaway tree at [`SCAN_DIR_BUDGET`]. Pure w.r.t.
/// the filesystem it's handed, so it's unit-testable against a temp tree.
fn find_steam_libraries_under(base: &Path, out: &mut Vec<PathBuf>) {
    use std::collections::VecDeque;
    let mut budget = SCAN_DIR_BUDGET;
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((base.to_path_buf(), 0));
    while let Some((dir, depth)) = queue.pop_front() {
        if budget == 0 {
            tracing::debug!("Steam disk scan hit its budget; stopping this base early");
            break;
        }
        budget -= 1;
        // Is this dir itself a Steam library?
        if dir.join("steamapps").is_dir() {
            tracing::info!(root = %dir.display(), "Steam library found by disk scan (outside registry / known paths)");
            out.push(dir);
            continue; // libraries don't nest, so prune here
        }
        if depth >= SCAN_MAX_DEPTH {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            // Only recurse into real directories; `file_type` doesn't follow
            // symlinks, so we never chase a link into a cycle.
            match entry.file_type() {
                Ok(t) if t.is_dir() => {}
                _ => continue,
            }
            let name = entry.file_name();
            if is_skippable_scan_dir(&name.to_string_lossy()) {
                continue;
            }
            queue.push_back((entry.path(), depth + 1));
        }
    }
}

/// Steam's install directory as recorded in the registry.
///
/// HKCU `Software\Valve\Steam\SteamPath` is the per-user value Steam keeps
/// current (forward-slashed, e.g. `d:/steam`); HKLM
/// `…\Valve\Steam\InstallPath` is the machine-wide fallback (back-slashed).
/// Reading this is what lets `libraryfolders.vdf` be found, and therefore every
/// extra library be enumerated, even when Steam lives on another drive.
#[cfg(windows)]
fn windows_registry_root() -> Option<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("SteamPath") {
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [r"SOFTWARE\Wow6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"] {
        if let Ok(key) = hklm.open_subkey(subkey) {
            if let Ok(p) = key.get_value::<String, _>("InstallPath") {
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn windows_registry_root() -> Option<PathBuf> {
    None
}

fn mac_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = home() {
        v.push(h.join("Library/Application Support/Steam"));
    }
    v
}

// ---- VDF / ACF parser ---------------------------------------------------

/// Parse `libraryfolders.vdf` and return the `path` of each entry.
///
/// The VDF format Valve uses is a nested map with quoted keys/values:
///
/// ```text
/// "libraryfolders"
/// {
///     "0" { "path" "/home/x/.steam/steam"  ... }
///     "1" { "path" "/mnt/games/steamlib"   ... }
/// }
/// ```
///
/// We only need the `"path"` fields; full key/value tree extraction would
/// be overkill.
pub fn parse_library_folders(text: &str) -> Vec<PathBuf> {
    let tokens = tokenize_vdf(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < tokens.len() {
        if tokens[i].as_str() == "path" {
            // Path values can contain backslashes (Windows) or unix slashes.
            let raw = &tokens[i + 1];
            let normalised = raw.replace("\\\\", "\\");
            out.push(PathBuf::from(normalised));
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// Parse one `appmanifest_<id>.acf` and return a `SteamApp` if it has the
/// fields we need.
pub fn parse_app_manifest(text: &str, steamapps_dir: &Path) -> Option<SteamApp> {
    let kvs = collect_top_level_kvs(text);
    let app_id: u64 = kvs.get("appid")?.parse().ok()?;
    let name = kvs
        .get("name")
        .cloned()
        .unwrap_or_else(|| format!("App {app_id}"));
    let install_subdir = kvs.get("installdir").cloned()?;
    let install_dir = steamapps_dir.join("common").join(install_subdir);
    Some(SteamApp {
        app_id,
        name,
        install_dir,
    })
}

/// Collect the top-level scalar key/value pairs of a Valve KV blob.
///
/// We deliberately ignore nested objects (`UserConfig`, `MountedDepots`, …)
/// because our needs are flat: appid / name / installdir.
fn collect_top_level_kvs(text: &str) -> BTreeMap<String, String> {
    let tokens = tokenize_vdf(text);
    let mut map = BTreeMap::new();
    let mut depth = 0i32;
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        match t {
            "{" => depth += 1,
            "}" => depth -= 1,
            _ => {
                // We're "inside" the outer "AppState" block at depth=1, so
                // grab its scalar key/value pairs there.
                if depth == 1 && i + 1 < tokens.len() {
                    let next = tokens[i + 1].as_str();
                    if next != "{" && next != "}" {
                        map.insert(tokens[i].clone(), tokens[i + 1].clone());
                        i += 2;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    map
}

/// Quick-and-dirty tokenizer for Valve KeyValues. Emits one token per quoted
/// string and one token for each `{` / `}` brace. Comments and unquoted
/// barewords are not part of the dialect we care about.
fn tokenize_vdf(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'"' => {
                // Quoted string. Read until the matching unescaped quote.
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
                let raw = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                out.push(raw.replace("\\\"", "\""));
                if i < bytes.len() {
                    i += 1; // consume closing quote
                }
            }
            b'{' => {
                out.push("{".into());
                i += 1;
            }
            b'}' => {
                out.push("}".into());
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // Line comment.
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Best-effort: read `<root>/userdata` and return one user folder path per
/// account. Steam writes per-user save data under
/// `<root>/userdata/<storeUserId>/<appid>/remote/`. Detection consumers can
/// fan a `<storeUserId>` placeholder out across these.
///
/// De-duplicated by [`lib_key`], the same way the library list is: a user
/// folder reached through two spellings of one library is one account, and
/// returning it twice makes every Steam Cloud save of every game show up twice.
pub fn steam_user_dirs(libraries: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for lib in libraries {
        let userdata = lib.join("userdata");
        let entries = match std::fs::read_dir(&userdata) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && seen.insert(lib_key(&path)) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// Convenience for callers wanting all library paths plus the user data dir
/// for the first library.
pub fn primary_user_dir(os: Os) -> Result<Option<PathBuf>> {
    let libs = detect_steam_libraries(os);
    if libs.is_empty() {
        return Ok(None);
    }
    let dirs = steam_user_dirs(&libs).context("scanning userdata")?;
    Ok(dirs.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library on a drive that isn't there is one answer, not one per game.
    /// Asking each file in turn is what put 553 identical rows in a user's log
    /// over two days with `e:/steam` unplugged.
    #[test]
    fn an_unreachable_library_answers_once_at_the_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = scan_library(&dir.path().join("nope/steamapps"))
            .expect_err("an absent folder can't be scanned");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    /// The codes that mean "the device isn't there" have to be told apart from
    /// an ordinary unreadable file, because they decide whether it's worth
    /// reading the next one. `NotFound` is deliberately not one of them: a
    /// missing appmanifest is Steam deleting one mid-scan.
    #[test]
    fn a_dead_device_is_told_apart_from_one_bad_file() {
        use std::io::{Error, ErrorKind};

        #[cfg(windows)]
        let gone = Error::from_raw_os_error(433); // ERROR_NO_SUCH_DEVICE
        #[cfg(not(windows))]
        let gone = Error::from_raw_os_error(19); // ENODEV
        assert!(device_is_gone(&gone));

        assert!(!device_is_gone(&Error::from_raw_os_error(13))); // EACCES / access denied
        assert!(!device_is_gone(&Error::new(
            ErrorKind::NotFound,
            "gone mid-scan"
        )));
        // No OS code at all (a synthesised error) is never a dead device.
        assert!(!device_is_gone(&Error::other("made up")));
    }

    /// One bad manifest doesn't cost the library: the rest still parse, and what
    /// failed comes back as a count for the caller to say once.
    #[test]
    fn one_unreadable_manifest_doesnt_cost_the_library() {
        let dir = tempfile::tempdir().expect("tempdir");
        let steamapps = dir.path().join("steamapps");
        std::fs::create_dir_all(steamapps.join("common")).unwrap();
        std::fs::write(
            steamapps.join("appmanifest_220.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\"220\"\n\t\"name\"\t\"Half-Life 2\"\n\t\"installdir\"\t\"Half-Life 2\"\n}\n",
        )
        .unwrap();
        // A directory where a manifest should be: reading it fails the same way
        // a broken file does, without needing to be root to arrange it.
        std::fs::create_dir(steamapps.join("appmanifest_440.acf")).unwrap();

        let scan = scan_library(&steamapps).expect("the folder is reachable");
        assert_eq!(scan.apps.len(), 1, "the good manifest still parses");
        assert_eq!(scan.apps[0].app_id, 220);
        assert_eq!(scan.unreadable, 1);
        assert!(
            scan.first_error.is_some(),
            "the caller needs something to show"
        );
        assert!(!scan.device_gone, "a bad file is not a missing drive");
    }

    #[test]
    fn parses_library_folders_v2() {
        let sample = r#"
"libraryfolders"
{
    "0"
    {
        "path"        "/home/test/.steam/steam"
        "label"       ""
        "contentid"   "12345"
    }
    "1"
    {
        "path"        "/mnt/games/SteamLibrary"
    }
}
"#;
        let paths = parse_library_folders(sample);
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/home/test/.steam/steam"),
                PathBuf::from("/mnt/games/SteamLibrary"),
            ]
        );
    }

    #[test]
    fn parses_app_manifest() {
        let sample = r#"
"AppState"
{
    "appid"             "413150"
    "name"              "Stardew Valley"
    "installdir"        "Stardew Valley"
    "UserConfig"
    {
        "language"      "english"
    }
}
"#;
        let app = parse_app_manifest(sample, Path::new("/lib/steamapps")).expect("parsed");
        assert_eq!(app.app_id, 413150);
        assert_eq!(app.name, "Stardew Valley");
        assert_eq!(
            app.install_dir,
            PathBuf::from("/lib/steamapps/common/Stardew Valley")
        );
    }

    #[test]
    fn ignores_nested_keys() {
        let sample = r#"
"AppState"
{
    "appid"  "1"
    "name"   "Test"
    "installdir" "Test"
    "MountedDepots"
    {
        "appid"  "999"
        "name"   "Wrong"
    }
}
"#;
        let app = parse_app_manifest(sample, Path::new("/x")).unwrap();
        // Top-level appid wins, not the nested MountedDepots one.
        assert_eq!(app.app_id, 1);
        assert_eq!(app.name, "Test");
    }

    #[test]
    fn handles_missing_installdir() {
        let sample = r#"
"AppState"
{
    "appid" "1"
    "name"  "Test"
}
"#;
        assert!(parse_app_manifest(sample, Path::new("/x")).is_none());
    }

    fn with_home<F: FnOnce()>(home: &Path, f: F) {
        let _guard = crate::test_lock::ENV
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", home);
        f();
        match prev {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn list_proton_prefixes_detects_appids_with_pfx() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let compatdata = home.join(".steam/steam/steamapps/compatdata");
        // 413150 (Stardew) has a real pfx, so it should appear.
        std::fs::create_dir_all(compatdata.join("413150/pfx/drive_c")).unwrap();
        // 999999 has the appid dir but no pfx, so it should be skipped.
        std::fs::create_dir_all(compatdata.join("999999")).unwrap();
        // "shader_cache" is not numeric, so it should be skipped.
        std::fs::create_dir_all(compatdata.join("shader_cache/pfx")).unwrap();

        with_home(home, || {
            let prefixes = list_proton_prefixes(Os::Linux);
            let ids: Vec<u64> = prefixes.iter().map(|p| p.app_id).collect();
            assert_eq!(ids, vec![413150]);
            assert!(prefixes[0].prefix_root.ends_with("compatdata/413150/pfx"));
        });
    }

    #[test]
    fn detect_libraries_skips_absent_and_dedups() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let root = home.join(".steam/steam");
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        // An extra library that exists on disk.
        let extra = home.join("games/SteamLibrary");
        std::fs::create_dir_all(extra.join("steamapps")).unwrap();
        // A ghost library listed in the vdf but with no steamapps dir
        // (unmounted drive, offline NAS): must be skipped.
        let ghost = home.join("mnt/nas/SteamLibrary");

        std::fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{}\" }}\n  \"1\" {{ \"path\" \"{}\" }}\n  \"2\" {{ \"path\" \"{}\" }}\n}}\n",
                root.display(),
                extra.display(),
                ghost.display(),
            ),
        )
        .unwrap();

        with_home(home, || {
            let libs = detect_steam_libraries(Os::Linux);
            assert!(libs.contains(&root), "root library present");
            assert!(libs.contains(&extra), "extra readable library present");
            assert!(!libs.contains(&ghost), "ghost library skipped");
            // The root is listed both as a linux_root and inside the vdf; it
            // must appear exactly once.
            assert_eq!(libs.iter().filter(|p| **p == root).count(), 1);
        });
    }

    #[test]
    fn scan_finds_custom_named_library_deep_and_prunes_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // A second Steam install, custom folder name, two levels down:
        //   <base>/Juegos/SteamLua/steamapps        (depth 2 -> found)
        let lua = base.join("Juegos/SteamLua");
        std::fs::create_dir_all(lua.join("steamapps/common")).unwrap();
        // A nested library inside the first must be pruned (not reported).
        std::fs::create_dir_all(lua.join("steamapps/common/inner/steamapps")).unwrap();
        // A library at depth 3:
        //   <base>/a/b/Lib3/steamapps
        let lib3 = base.join("a/b/Lib3");
        std::fs::create_dir_all(lib3.join("steamapps")).unwrap();
        // Too deep (depth 4): must NOT be found.
        let deep = base.join("a/b/c/TooDeep");
        std::fs::create_dir_all(deep.join("steamapps")).unwrap();
        // Hidden + system-named dirs holding a library must be skipped.
        std::fs::create_dir_all(base.join(".hidden/steamapps")).unwrap();
        std::fs::create_dir_all(base.join("Windows/steamapps")).unwrap();

        let mut out = Vec::new();
        find_steam_libraries_under(base, &mut out);

        assert!(out.contains(&lua), "custom-named library at depth 2 found");
        assert!(out.contains(&lib3), "library at depth 3 found");
        assert!(!out.contains(&deep), "depth-4 library not reached");
        assert!(
            !out.iter().any(|p| p.ends_with("inner")),
            "nested library inside a found one is pruned"
        );
        assert!(
            !out.iter().any(|p| p.starts_with(base.join(".hidden"))),
            "hidden dir skipped"
        );
        assert!(
            !out.iter().any(|p| p.starts_with(base.join("Windows"))),
            "system-named dir skipped"
        );
    }

    /// The compatibility symlink every Linux Steam install ships
    /// (`~/.steam/steam` → `~/.local/share/Steam`) must not turn one library
    /// into two. It did, and with it every Steam Cloud save under `userdata`
    /// was reported twice: 34 duplicate paths across 24 games on one machine.
    ///
    /// The symlink is created for real, not simulated with two separate
    /// directories: the whole bug is that two *different* paths name one
    /// directory, and two real directories can't reproduce that.
    #[cfg(unix)]
    #[test]
    fn the_steam_compatibility_symlink_is_one_library_not_two() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        // The real install, and the symlink Steam leaves next to it.
        let real = home.join(".local/share/Steam");
        std::fs::create_dir_all(real.join("steamapps")).unwrap();
        std::fs::create_dir_all(home.join(".steam")).unwrap();
        let link = home.join(".steam/steam");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(link.join("steamapps").is_dir(), "the symlink resolves");

        // One Steam account with one Cloud-saving game under it.
        let user = real.join("userdata/76561198041773665");
        std::fs::create_dir_all(user.join("646270/remote")).unwrap();

        // The vdf names the library by the symlinked spelling, which is the
        // third way the same directory arrives.
        std::fs::write(
            real.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{}\" }}\n}}\n",
                link.display()
            ),
        )
        .unwrap();

        with_home(home, || {
            let libs = detect_steam_libraries(Os::Linux);
            assert_eq!(
                libs.len(),
                1,
                "one directory reached three ways is one library; got {libs:?}"
            );
            assert_eq!(libs[0], real, "the real directory is the one kept");

            let users = steam_user_dirs(&libs).expect("userdata is readable");
            assert_eq!(
                users.len(),
                1,
                "one account, not one per spelling: {users:?}"
            );
            assert_eq!(users[0], user);
        });
    }

    #[test]
    fn list_proton_prefixes_empty_when_no_steam() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(tmp.path(), || {
            let prefixes = list_proton_prefixes(Os::Linux);
            assert!(prefixes.is_empty());
        });
    }
}
