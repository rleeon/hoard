//! Games other programs tell Hoard about.
//!
//! Hoard finds installed games through the launchers' own records (Steam, Epic,
//! GOG, Microsoft Store). Games that no launcher installed — copied into
//! `C:\Games`, unpacked from an archive, installed by a library manager such as a
//! Playnite plugin, or run straight off a NAS share — are only noticed once their
//! save folder exists, i.e. after the first play, and saves kept inside the game's
//! own folder are never found because Hoard doesn't know where that folder is.
//!
//! A program that does know can drop a JSON file in
//! `<config dir>/libraries/` (on Windows `%APPDATA%\hoard\hoard\config\libraries\`)
//! listing those games. Hoard reads every `*.json` there during detection, the
//! same way it reads Epic's manifests:
//!
//! ```json
//! {
//!   "version": 1,
//!   "source": "Playnite",
//!   "games": [
//!     { "name": "Hades", "installDir": "C:\\Games\\Hades", "exe": "x64\\Hades.exe" },
//!     { "name": "Celeste", "installDir": "\\\\server\\games\\Celeste", "steamAppId": 504230 }
//!   ]
//! }
//! ```
//!
//! * `name` (required): the title as the program knows it.
//! * `installDir`: the game's folder on this machine. A folder on a network share
//!   still identifies the game, but is never used to look for saves (files inside
//!   a share are shared already, and sweeping one is slow).
//! * `exe`: the executable, absolute or relative to `installDir`. Its file name is
//!   looked up in the catalogue's launch executables, which identifies the game
//!   even when the folder and title are named differently. Names shared by many
//!   games (`game.exe`) resolve to nothing, as everywhere else in Hoard.
//! * `steamAppId`: when known, the strongest match of all.
//!
//! Unknown fields are ignored and a malformed file is skipped with a warning, so a
//! writer can add fields without breaking older Hoards.

use std::path::{Path, PathBuf};

use hoard_manifest::ludusavi;
use serde::Deserialize;

/// One game listed by another program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalApp {
    /// The file it came from (for logs).
    pub source: String,
    pub name: String,
    /// Local install folder; `None` when absent or on a network share.
    pub install_dir: Option<PathBuf>,
    /// The listed install folder, local or not.
    pub listed_dir: Option<PathBuf>,
    /// Absolute path or bare file name of the executable.
    pub exe: Option<String>,
    pub steam_app_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct LibraryFile {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    games: Vec<LibraryGame>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryGame {
    name: String,
    #[serde(default)]
    install_dir: Option<String>,
    #[serde(default)]
    exe: Option<String>,
    #[serde(default)]
    steam_app_id: Option<u64>,
}

/// Where library files are read from, or `None` if the config dir is unknown.
pub fn libraries_dir() -> Option<PathBuf> {
    crate::config::CliConfig::project_dirs()
        .ok()
        .map(|pd| pd.config_dir().join("libraries"))
}

/// Every game listed in `dir/*.json`. Missing folder = empty list.
pub fn read_libraries(dir: &Path) -> Vec<ExternalApp> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();

    let mut out = Vec::new();
    for file in files {
        let text = match std::fs::read_to_string(&file) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(file = %file.display(), error = %e, "external library: unreadable, skipped");
                continue;
            }
        };
        // Writers on Windows sometimes emit a BOM.
        let text = text.trim_start_matches('\u{feff}');
        let parsed: LibraryFile = match serde_json::from_str(text) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(file = %file.display(), error = %e, "external library: malformed, skipped");
                continue;
            }
        };
        let source = parsed.source.unwrap_or_else(|| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("external")
                .to_string()
        });
        for g in parsed.games {
            if g.name.trim().is_empty() {
                continue;
            }
            let listed_dir = g
                .install_dir
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from);
            let install_dir = listed_dir
                .as_deref()
                .filter(|d| !is_network_path(d) && d.is_dir())
                .map(Path::to_path_buf);
            let exe = g.exe.filter(|s| !s.trim().is_empty()).map(|e| {
                // Relative to the listed folder when it isn't absolute.
                let p = PathBuf::from(&e);
                match (&listed_dir, p.is_absolute()) {
                    (Some(dir), false) => dir.join(p).to_string_lossy().into_owned(),
                    _ => e,
                }
            });
            out.push(ExternalApp {
                source: source.clone(),
                name: g.name.trim().to_string(),
                install_dir,
                listed_dir,
                exe,
                steam_app_id: g.steam_app_id,
            });
        }
    }
    out
}

/// The catalogue slug a listed game resolves to, and whether only a fuzzy name
/// match found it. Strongest evidence first: Steam appid, executable, exact name,
/// fuzzy name.
pub fn resolve_slug(
    app: &ExternalApp,
    catalog_has: impl Fn(&str) -> bool,
    fuzzy_threshold: f32,
) -> Option<(String, bool)> {
    if let Some(id) = app.steam_app_id {
        if let Some(e) = ludusavi::find_by_steam_app_id(id) {
            if catalog_has(&e.slug) {
                return Some((e.slug.clone(), false));
            }
        }
    }
    if let Some(exe) = &app.exe {
        if let Some(e) = ludusavi::find_by_exe(exe) {
            if catalog_has(&e.slug) {
                return Some((e.slug.clone(), false));
            }
        }
    }
    let slug = ludusavi::slugify(&app.name);
    if catalog_has(&slug) {
        return Some((slug, false));
    }
    ludusavi::find_by_fuzzy_name(&app.name, fuzzy_threshold)
        .filter(|e| catalog_has(&e.slug))
        .map(|e| (e.slug.clone(), true))
}

/// A UNC path, or (on Windows) a drive letter mapped to a network share.
pub fn is_network_path(p: &Path) -> bool {
    let s = p.to_string_lossy();
    if s.starts_with(r"\\") || s.starts_with("//") {
        // `\\?\C:\...` is a local verbatim path; `\\?\UNC\...` is not.
        return !(s.starts_with(r"\\?\") && !s[4..].to_ascii_uppercase().starts_with("UNC"));
    }
    is_remote_drive(&s)
}

#[cfg(windows)]
fn is_remote_drive(s: &str) -> bool {
    use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
    const DRIVE_REMOTE: u32 = 4;
    let bytes = s.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    let root: Vec<u16> = format!("{}:\\", bytes[0] as char)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_REMOTE }
}

#[cfg(not(windows))]
fn is_remote_drive(_s: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn missing_folder_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_libraries(&tmp.path().join("nope")).is_empty());
    }

    #[test]
    fn reads_games_and_resolves_relative_exe() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("Hades");
        std::fs::create_dir_all(&game_dir).unwrap();
        let body = serde_json::json!({
            "version": 1,
            "source": "Playnite",
            "someFutureField": true,
            "games": [
                { "name": "Hades", "installDir": game_dir.to_string_lossy(), "exe": "x64/Hades.exe" },
                { "name": "  ", "installDir": "ignored" },
                { "name": "Celeste", "installDir": "\\\\server\\games\\Celeste", "steamAppId": 504230 }
            ]
        });
        write(tmp.path(), "nas.json", &format!("\u{feff}{body}"));
        write(tmp.path(), "notes.txt", "not a library");

        let apps = read_libraries(tmp.path());
        assert_eq!(apps.len(), 2);

        let hades = &apps[0];
        assert_eq!(hades.source, "Playnite");
        assert_eq!(hades.install_dir.as_deref(), Some(game_dir.as_path()));
        assert!(hades.exe.as_deref().unwrap().ends_with("Hades.exe"));
        assert!(Path::new(hades.exe.as_deref().unwrap()).starts_with(&game_dir));

        let celeste = &apps[1];
        assert_eq!(
            celeste.install_dir, None,
            "a network folder never becomes an install dir"
        );
        assert_eq!(
            celeste.listed_dir.as_deref(),
            Some(Path::new("\\\\server\\games\\Celeste"))
        );
        assert_eq!(celeste.steam_app_id, Some(504230));
    }

    #[test]
    fn malformed_file_is_skipped_not_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "a.json", "{ not json");
        write(tmp.path(), "b.json", r#"{"games":[{"name":"Celeste"}]}"#);
        let apps = read_libraries(tmp.path());
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].source, "b");
        assert_eq!(apps[0].install_dir, None);
    }

    #[test]
    fn network_paths() {
        assert!(is_network_path(Path::new(r"\\192.168.1.2\games\Hades")));
        assert!(is_network_path(Path::new(r"\\?\UNC\server\games")));
        assert!(!is_network_path(Path::new(r"\\?\C:\Games\Hades")));
        assert!(!is_network_path(Path::new("/home/user/games")));
    }

    #[test]
    fn resolves_by_steam_id_then_exe_then_name() {
        let everything = |s: &str| ludusavi::catalog().iter().any(|e| e.slug == s);
        // Pick a real catalogue entry that has a Steam id, to exercise each step.
        let entry = ludusavi::catalog()
            .iter()
            .find(|e| e.steam_app_id.is_some())
            .expect("catalogue has Steam games");

        let by_id = ExternalApp {
            source: "t".into(),
            name: "a name that matches nothing at all zzz".into(),
            install_dir: None,
            listed_dir: None,
            exe: None,
            steam_app_id: entry.steam_app_id,
        };
        assert_eq!(
            resolve_slug(&by_id, everything, 0.15).map(|r| r.0),
            Some(entry.slug.clone())
        );

        let by_name = ExternalApp {
            steam_app_id: None,
            name: entry.display_name.clone(),
            ..by_id.clone()
        };
        assert_eq!(
            resolve_slug(&by_name, everything, 0.15),
            Some((entry.slug.clone(), false))
        );

        let unknown = ExternalApp {
            steam_app_id: None,
            ..by_id.clone()
        };
        assert_eq!(resolve_slug(&unknown, everything, 0.15), None);

        // Not in the catalogue being scanned: no match, even with a valid id.
        assert_eq!(resolve_slug(&by_id, |_| false, 0.15), None);
    }
}
