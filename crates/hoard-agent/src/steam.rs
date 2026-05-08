//! Detect Steam libraries and the games installed inside them.
//!
//! Steam stores its library list in `<steam>/steamapps/libraryfolders.vdf`.
//! Each library directory in turn contains `appmanifest_<appid>.acf` files
//! describing installed games. Both formats are Valve's KeyValues syntax,
//! which is conceptually a JSON-ish nested map; we parse it with a
//! minimal tokenizer rather than pulling in a full VDF crate.

use std::collections::BTreeMap;
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

/// Probe the host for Steam libraries. Returns the directories that contain
/// a `steamapps` subfolder; later passes scan those.
///
/// On Linux the lookup considers the native install (`~/.steam/steam`) and
/// the Flatpak path (`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`).
/// On Windows we look at `%PROGRAMFILES(X86)%/Steam` and `%PROGRAMFILES%/Steam`.
/// On macOS we check `~/Library/Application Support/Steam`.
pub fn detect_steam_libraries(os: Os) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = match os {
        Os::Linux => linux_roots(),
        Os::Windows => windows_roots(),
        Os::Mac => mac_roots(),
    };

    // The root itself always doubles as a library — Steam ships its own
    // `steamapps` there. Then any extra libraries are listed inside
    // libraryfolders.vdf.
    let mut libraries: Vec<PathBuf> = Vec::new();
    for root in &roots {
        if root.join("steamapps").is_dir() {
            libraries.push(root.clone());
        }
        let vdf = root.join("steamapps/libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(&vdf) {
            for extra in parse_library_folders(&text) {
                if extra.join("steamapps").is_dir() && !libraries.contains(&extra) {
                    libraries.push(extra);
                }
            }
        }
    }

    libraries.sort();
    libraries.dedup();
    // Free the root buffer so callers don't keep a dangling ref.
    roots.clear();
    libraries
}

/// List Steam apps installed across all detected libraries.
///
/// Errors reading individual library folders / appmanifest files are logged
/// and skipped — we never abort the whole scan because one folder is
/// missing or one manifest file is corrupt. Returning `Ok(vec![])` is a
/// normal outcome (Steam not installed); detection treats it as "no
/// Steam apps found, fall back to filesystem heuristic".
pub fn list_installed_steam_games(os: Os) -> Result<Vec<SteamApp>> {
    let libraries = detect_steam_libraries(os);
    if libraries.is_empty() {
        tracing::debug!(?os, "no Steam libraries found");
        return Ok(Vec::new());
    }

    let mut out: Vec<SteamApp> = Vec::new();
    for lib in &libraries {
        let steamapps = lib.join("steamapps");
        let entries = match std::fs::read_dir(&steamapps) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    path = %steamapps.display(),
                    error = %e,
                    "skipping Steam library — read_dir failed"
                );
                continue;
            }
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
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "appmanifest unreadable; skipping"
                    );
                    continue;
                }
            };
            match parse_app_manifest(&text, &steamapps) {
                Some(app) => out.push(app),
                None => {
                    tracing::debug!(
                        path = %path.display(),
                        "appmanifest missing required fields (appid/installdir); skipping"
                    );
                }
            }
        }
    }
    out.sort_by_key(|a| a.app_id);
    out.dedup_by(|a, b| a.app_id == b.app_id);
    tracing::info!(libraries = libraries.len(), apps = out.len(), "Steam scan complete");
    Ok(out)
}

// ---- Roots --------------------------------------------------------------

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn linux_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = home() {
        v.push(h.join(".steam/steam"));
        v.push(h.join(".local/share/Steam"));
        v.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    v
}

fn windows_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles", "PROGRAMFILES"] {
        if let Some(p) = std::env::var_os(var) {
            v.push(PathBuf::from(p).join("Steam"));
        }
    }
    v
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
pub fn steam_user_dirs(libraries: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for lib in libraries {
        let userdata = lib.join("userdata");
        let entries = match std::fs::read_dir(&userdata) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
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
}
