//! Steam scanner.
//!
//! Steam stores its installed-app metadata in two VDF files:
//!
//! - `<root>/steamapps/libraryfolders.vdf` — list of all library paths
//!   (Steam supports multi-disk libraries via this file).
//! - `<root>/steamapps/appmanifest_<appid>.acf` — one per installed game.
//!
//! We pull just two fields from each `appmanifest`: `appid` and
//! `installdir`. That's enough to cross-reference with the manifest
//! catalogue (via `steam_appid`) and report an install path to the UI.
//!
//! ## VDF format
//!
//! VDF is a brace-delimited key-value text format. We don't need a full
//! parser — every field we care about is on its own line as
//! `"key"\s+"value"`. The tiny [`extract_field`] helper below covers
//! 100% of `appmanifest_*.acf` and the subset of `libraryfolders.vdf`
//! we read.

use std::path::{Path, PathBuf};

use crate::{DetectError, DetectedGame, GameSource};

use super::{from_manifest, read_to_string};

/// Run the Steam scan.
pub fn scan() -> Result<Vec<DetectedGame>, DetectError> {
    let root = locate_steam_root().ok_or_else(|| {
        DetectError::SteamMissing(
            "could not locate Steam installation (registry / common paths empty)".into(),
        )
    })?;
    tracing::debug!(steam_root = %root.display(), "Steam scan");

    let libraries = read_library_folders(&root)?;
    tracing::debug!(count = libraries.len(), "Steam libraries discovered");

    let mut out = Vec::new();
    for lib in &libraries {
        let steamapps = lib.join("steamapps");
        let dir = match std::fs::read_dir(&steamapps) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(path = %steamapps.display(), error = %e, "skip library");
                continue;
            }
        };

        for entry in dir.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            match parse_appmanifest(&entry.path()) {
                Ok(Some((appid, install_subdir))) => {
                    if let Some(m) = hoard_manifest::lookup_by_steam_appid(appid) {
                        let install_dir = steamapps.join("common").join(&install_subdir);
                        out.push(from_manifest(m, Some(install_dir), GameSource::Steam));
                    }
                }
                Ok(None) => {
                    tracing::debug!(path = %entry.path().display(), "appmanifest missing fields");
                }
                Err(e) => {
                    tracing::warn!(path = %entry.path().display(), error = %e, "appmanifest parse failed");
                }
            }
        }
    }

    Ok(out)
}

/// Extract the value of a flat top-level field (`"key" "value"`) from a
/// VDF/ACF text. Returns the first match. Matching is whitespace-tolerant
/// but not full VDF-aware — sufficient for the simple `appmanifest_*.acf`
/// schema.
pub(crate) fn extract_field(text: &str, key: &str) -> Option<String> {
    // Look for: "key"  whitespace  "value"
    let needle = format!("\"{key}\"");
    let mut rest = text;
    while let Some(pos) = rest.find(&needle) {
        let after = &rest[pos + needle.len()..];
        let after = after.trim_start();
        if let Some(stripped) = after.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                return Some(stripped[..end].to_string());
            }
        }
        // No value-string after this occurrence — keep scanning.
        rest = &rest[pos + needle.len()..];
    }
    None
}

fn parse_appmanifest(path: &Path) -> Result<Option<(u32, String)>, DetectError> {
    let text = read_to_string(path)?;
    let appid_str = match extract_field(&text, "appid") {
        Some(s) => s,
        None => return Ok(None),
    };
    let installdir = match extract_field(&text, "installdir") {
        Some(s) => s,
        None => return Ok(None),
    };
    let appid: u32 = appid_str.parse().map_err(|_| DetectError::SteamParse {
        path: path.display().to_string(),
        message: format!("non-numeric appid: {appid_str:?}"),
    })?;
    Ok(Some((appid, installdir)))
}

/// Read every library `path` field from `libraryfolders.vdf`. Steam stores
/// the main library + each extra one as a numbered entry; we extract
/// every `"path"` field at any nesting depth.
fn read_library_folders(root: &Path) -> Result<Vec<PathBuf>, DetectError> {
    let mut paths = vec![root.to_path_buf()];

    let vdf = root.join("steamapps").join("libraryfolders.vdf");
    let text = match std::fs::read_to_string(&vdf) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(paths),
        Err(e) => {
            return Err(DetectError::Io {
                path: vdf.display().to_string(),
                source: e,
            })
        }
    };

    // Pull every `"path" "..."` line. `extract_field` only returns the
    // first; iterate manually instead.
    let needle = "\"path\"";
    let mut rest = text.as_str();
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        let after = after.trim_start();
        if let Some(stripped) = after.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                let path = &stripped[..end];
                // VDF doubles backslashes on Windows; normalise.
                let normalised = path.replace("\\\\", "\\");
                let p = PathBuf::from(normalised);
                if !paths.contains(&p) {
                    paths.push(p);
                }
                rest = &stripped[end + 1..];
                continue;
            }
        }
        rest = &rest[pos + needle.len()..];
    }

    Ok(paths)
}

/// Find the Steam root directory.
///
/// On Windows we check the registry first (canonical), then fall back
/// to `%PROGRAMFILES(X86)%\Steam`. On Linux we check the common XDG
/// locations. Returns `None` if nothing is found — Steam may simply
/// not be installed, which is a normal state, not an error.
fn locate_steam_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(p) = registry_steam_path() {
            if p.exists() {
                return Some(p);
            }
        }
        for env_var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Ok(pf) = std::env::var(env_var) {
                let p = PathBuf::from(pf).join("Steam");
                if p.exists() {
                    return Some(p);
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").ok()?;
        let candidates = [
            PathBuf::from(&home).join(".steam/steam"),
            PathBuf::from(&home).join(".local/share/Steam"),
            PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/data/Steam"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_field_basic() {
        let text = r#""AppState"
{
    "appid"        "413150"
    "name"         "Stardew Valley"
    "installdir"   "Stardew Valley"
}"#;
        assert_eq!(extract_field(text, "appid"), Some("413150".to_string()));
        assert_eq!(
            extract_field(text, "installdir"),
            Some("Stardew Valley".to_string())
        );
        assert_eq!(extract_field(text, "missing"), None);
    }

    #[test]
    fn extract_field_handles_extra_whitespace_and_tabs() {
        let text = "\"appid\"\t\t\"413150\"\n\"installdir\"   \"Foo\"";
        assert_eq!(extract_field(text, "appid"), Some("413150".to_string()));
        assert_eq!(extract_field(text, "installdir"), Some("Foo".to_string()));
    }

    #[test]
    fn parse_appmanifest_real_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appmanifest_413150.acf");
        std::fs::write(
            &path,
            r#""AppState"
{
    "appid"        "413150"
    "Universe"     "1"
    "name"         "Stardew Valley"
    "installdir"   "Stardew Valley"
    "LastUpdated"  "1700000000"
}"#,
        )
        .unwrap();
        let parsed = parse_appmanifest(&path).unwrap();
        assert_eq!(parsed, Some((413150, "Stardew Valley".to_string())));
    }

    #[test]
    fn parse_appmanifest_missing_fields_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appmanifest_x.acf");
        std::fs::write(&path, "\"AppState\" { }").unwrap();
        assert!(parse_appmanifest(&path).unwrap().is_none());
    }

    #[test]
    fn parse_appmanifest_non_numeric_appid_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appmanifest_x.acf");
        std::fs::write(
            &path,
            "\"AppState\" { \"appid\" \"oops\" \"installdir\" \"X\" }",
        )
        .unwrap();
        assert!(matches!(
            parse_appmanifest(&path),
            Err(DetectError::SteamParse { .. })
        ));
    }

    #[test]
    fn library_folders_scan_finds_extras() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        std::fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            r#""libraryfolders"
{
    "0"
    {
        "path"  "/home/x/.steam/steam"
    }
    "1"
    {
        "path"  "/mnt/games/SteamLibrary"
    }
}"#,
        )
        .unwrap();
        let libs = read_library_folders(root).unwrap();
        assert!(libs
            .iter()
            .any(|p| p == &PathBuf::from("/home/x/.steam/steam")));
        assert!(libs
            .iter()
            .any(|p| p == &PathBuf::from("/mnt/games/SteamLibrary")));
        // The root itself is always included.
        assert!(libs.iter().any(|p| p == root));
    }

    #[test]
    fn missing_libraryfolders_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        // No libraryfolders.vdf written. Should return just the root.
        let libs = read_library_folders(dir.path()).unwrap();
        assert_eq!(libs, vec![dir.path().to_path_buf()]);
    }
}
