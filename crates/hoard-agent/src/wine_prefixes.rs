//! Unified enumerator for Wine/Proton-style prefixes — Steam (Proton),
//! Lutris and Bottles.
//!
//! `steam::list_proton_prefixes` keeps its existing shape and remains the
//! sole detector of Steam-managed compatdata prefixes (it is `pub` API
//! consumed by `detection.rs` and the integration tests). This module
//! wraps it and adds two Linux-only sources:
//!
//! - **Lutris**: per-runner prefixes under
//!   `~/.local/share/lutris/runners/wine/<runner>/prefixes/<game>/drive_c/`.
//! - **Bottles**: each bottle is itself a prefix root with `drive_c/`
//!   directly inside, both for the native install
//!   (`~/.local/share/bottles/bottles/<bottle>/`) and the Flatpak install
//!   (`~/.var/app/com.usebottles.bottles/data/bottles/bottles/<bottle>/`).
//!
//! Failures (missing `HOME`, unreadable directories, missing `drive_c/`)
//! collapse to an empty vector for that source — never a panic. On
//! non-Linux hosts only the Proton wrapper contributes.
//!
//! Identifiers: the Proton wrapper passes the Steam appid as a string;
//! Lutris and Bottles use the directory name as the identifier, which is
//! the user-visible bottle / game slug.

use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::steam;

/// Which launcher owns the prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixKind {
    Proton,
    Lutris,
    Bottles,
    /// A plain Wine prefix not managed by any of the launchers above:
    /// `$WINEPREFIX`, the default `~/.wine*`, PlayOnLinux, or any prefix a
    /// `.desktop` launcher references. Not tied to a single game — the whole
    /// `drive_c/` may hold saves for any number of catalog titles.
    Generic,
}

/// One Wine/Proton prefix on disk.
///
/// `prefix_root` is the directory that contains `drive_c/` directly. For
/// Proton that is the `pfx/` directory Steam creates; for Lutris it is the
/// `prefixes/<game>/` directory; for Bottles it is the bottle's own root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinePrefix {
    pub kind: PrefixKind,
    /// Stable id within the launcher: Steam appid stringified for Proton,
    /// directory name for Lutris / Bottles. Used by the aggressive walker
    /// to slugify and cross-reference against the Ludusavi catalog.
    pub identifier: String,
    pub prefix_root: PathBuf,
}

/// Enumerate every Wine-style prefix this host has.
///
/// Returns an empty vector when nothing matches; never panics. On
/// non-Linux hosts only the Proton wrapper has a chance to contribute.
pub fn list_wine_prefixes(os: Os) -> Vec<WinePrefix> {
    list_wine_prefixes_mode(os, false)
}

/// Like [`list_wine_prefixes`] but also runs the expensive parent-directory
/// sweep that finds prefixes in arbitrary locations (Heroic, CrossOver,
/// Flatpak'd Wine, mounted media). For the user-triggered deep scan only.
pub fn list_wine_prefixes_deep(os: Os) -> Vec<WinePrefix> {
    list_wine_prefixes_mode(os, true)
}

fn list_wine_prefixes_mode(os: Os, deep: bool) -> Vec<WinePrefix> {
    let mut out: Vec<WinePrefix> = Vec::new();

    // Proton (Steam) — wrapper over the existing, well-tested API.
    for p in steam::list_proton_prefixes(os) {
        out.push(WinePrefix {
            kind: PrefixKind::Proton,
            identifier: p.app_id.to_string(),
            prefix_root: p.prefix_root,
        });
    }

    // Lutris and Bottles are Linux-only (Lutris/Bottles do not ship for
    // Windows or macOS; we do not pretend to scan their data dirs from a
    // foreign OS).
    if matches!(os, Os::Linux) {
        out.extend(discover_lutris_prefixes());
        out.extend(discover_bottles_prefixes());
        // Generic prefixes come last and are deduplicated against everything
        // already found, so a prefix Steam/Lutris/Bottles already own isn't
        // re-reported as Generic.
        let mut known: std::collections::HashSet<PathBuf> =
            out.iter().map(|p| canonical(&p.prefix_root)).collect();
        for p in discover_generic_prefixes() {
            if known.insert(canonical(&p.prefix_root)) {
                out.push(p);
            }
        }
        if deep {
            for p in discover_deep_prefixes() {
                if known.insert(canonical(&p.prefix_root)) {
                    out.push(p);
                }
            }
        }
    }

    out
}

/// Canonicalize a path for dedup, falling back to the path itself when the
/// target can't be resolved (e.g. a symlink to a missing dir).
fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Discover plain Wine prefixes from every general source we know, regardless
/// of which launcher (if any) created them:
///
/// - `$WINEPREFIX` — the prefix the user's shell currently points at.
/// - Default locations: `~/.wine`, `~/.wine32`, `~/.wine64`.
/// - PlayOnLinux: `~/.PlayOnLinux/wineprefix/<name>/`.
/// - Any prefix referenced by a `WINEPREFIX=…` assignment inside a desktop
///   entry the user can launch (`~/.local/share/applications`, the XDG
///   desktop dir, `/usr/share/applications`). This covers prefixes in fully
///   arbitrary locations.
///
/// A candidate only qualifies when its `drive_c/` exists, mirroring the
/// other discoverers. Results are deduplicated by canonical path; the
/// identifier is the prefix directory name (best-effort, not a game slug).
fn discover_generic_prefixes() -> Vec<WinePrefix> {
    let Some(home) = home() else {
        return Vec::new();
    };

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(wp) = std::env::var_os("WINEPREFIX") {
        candidates.push(PathBuf::from(wp));
    }
    for name in [".wine", ".wine32", ".wine64"] {
        candidates.push(home.join(name));
    }
    if let Ok(entries) = std::fs::read_dir(home.join(".PlayOnLinux/wineprefix")) {
        for e in entries.flatten() {
            candidates.push(e.path());
        }
    }
    candidates.extend(prefixes_from_desktop_files(&home));

    let mut out: Vec<WinePrefix> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for root in candidates {
        if !root.join("drive_c").is_dir() {
            continue;
        }
        if !seen.insert(canonical(&root)) {
            continue;
        }
        let identifier = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("wine")
            .to_string();
        out.push(WinePrefix {
            kind: PrefixKind::Generic,
            identifier,
            prefix_root: root,
        });
    }
    out
}

/// Deep sweep: find Wine prefixes in arbitrary locations by scanning the
/// directories under a set of likely parents for a child holding `drive_c/`.
///
/// Parents (each scanned one level down, so `<parent>/<name>/drive_c`):
/// - Heroic: `~/Games/Heroic/Prefixes` and its `default/` subtree.
/// - CrossOver: `~/.cxoffice`.
/// - Flatpak'd Wine front-ends keep prefixes under their app data dir.
/// - Anything the user dropped under `~/Games`, `~/.local/share`, `/opt`, or a
///   mounted volume (`/run/media/<user>/<label>`).
///
/// Bounded: each parent is read once and only its immediate children are
/// stat'd for `drive_c/`; no recursion. Identifier is the prefix dir name.
fn discover_deep_prefixes() -> Vec<WinePrefix> {
    let Some(home) = home() else {
        return Vec::new();
    };

    let mut parents: Vec<PathBuf> = vec![
        home.join("Games/Heroic/Prefixes"),
        home.join("Games/Heroic/Prefixes/default"),
        home.join(".cxoffice"),
        home.join("Games"),
        home.join(".local/share"),
        PathBuf::from("/opt"),
    ];
    // Flatpak Lutris/Heroic prefixes live under their app data dir.
    for app in [
        "net.lutris.Lutris/data/lutris/runners/wine",
        "com.heroicgameslauncher.hgl/config/heroic/Prefixes/default",
    ] {
        parents.push(home.join(".var/app").join(app));
    }
    // Mounted volumes: one level for the volume, then look inside.
    if let Ok(users) = std::fs::read_dir("/run/media") {
        for user in users.flatten().map(|e| e.path()) {
            if let Ok(vols) = std::fs::read_dir(&user) {
                for vol in vols.flatten().map(|e| e.path()) {
                    parents.push(vol);
                }
            }
        }
    }

    let mut out: Vec<WinePrefix> = Vec::new();
    for parent in parents {
        let entries = match std::fs::read_dir(&parent) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let root = entry.path();
            if !root.join("drive_c").is_dir() {
                continue;
            }
            let identifier = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("wine")
                .to_string();
            out.push(WinePrefix {
                kind: PrefixKind::Generic,
                identifier,
                prefix_root: root,
            });
        }
    }
    out
}

/// Scan desktop-entry directories for `WINEPREFIX=` assignments and return the
/// referenced prefix roots. This is how a manually installed game (the
/// `.desktop` Wine generates, PlayOnLinux/Lutris shortcuts, etc.) advertises
/// the prefix it runs in, so it generalizes to arbitrary prefix locations.
fn prefixes_from_desktop_files(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join(".local/share/applications"),
        PathBuf::from("/usr/share/applications"),
    ];
    // The user's desktop dir (localized — `~/Desktop`, `~/Escritorio`, …).
    if let Some(d) = std::env::var_os("XDG_DESKTOP_DIR") {
        dirs.push(PathBuf::from(d));
    }
    dirs.push(home.join("Desktop"));

    let mut out: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(prefix) = parse_wineprefix_assignment(&content) {
                out.push(prefix);
            }
        }
    }
    out
}

/// Extract the value of the first `WINEPREFIX=` assignment in a desktop entry.
///
/// Handles the common forms seen in Wine-generated launchers:
///   `Exec=env WINEPREFIX="/home/u/.wine64" wine-stable "C:\\..."`
///   `Exec=env WINEPREFIX=/home/u/prefix wine ...`
/// The value may be double- or single-quoted, or bare up to the next space.
fn parse_wineprefix_assignment(content: &str) -> Option<PathBuf> {
    let idx = content.find("WINEPREFIX=")?;
    let rest = &content[idx + "WINEPREFIX=".len()..];
    let mut chars = rest.chars();
    let first = chars.next()?;
    let value: String = match first {
        '"' => rest[1..].chars().take_while(|c| *c != '"').collect(),
        '\'' => rest[1..].chars().take_while(|c| *c != '\'').collect(),
        _ => rest.chars().take_while(|c| !c.is_whitespace()).collect(),
    };
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Walk `~/.local/share/lutris/runners/wine/<runner>/prefixes/<game>/`.
///
/// Each runner directory (e.g. `lutris-fshack-7.2-x86_64`) carries its own
/// `prefixes/` subtree. A prefix is only reported when its `drive_c/`
/// exists, mirroring `list_proton_prefixes`'s "skip prefixes Steam left
/// half-created" check.
fn discover_lutris_prefixes() -> Vec<WinePrefix> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let runners_root = home.join(".local/share/lutris/runners/wine");
    let runner_entries = match std::fs::read_dir(&runners_root) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<WinePrefix> = Vec::new();
    for runner in runner_entries.flatten() {
        let prefixes_dir = runner.path().join("prefixes");
        let prefix_entries = match std::fs::read_dir(&prefixes_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for prefix in prefix_entries.flatten() {
            let prefix_path = prefix.path();
            if !prefix_path.join("drive_c").is_dir() {
                continue;
            }
            let Some(name) = prefix_path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            out.push(WinePrefix {
                kind: PrefixKind::Lutris,
                identifier: name.to_string(),
                prefix_root: prefix_path,
            });
        }
    }
    out
}

/// Walk Bottles' native and Flatpak data dirs.
///
/// Bottles uses the bottle root itself as a prefix (its `drive_c/` lives
/// directly inside), so the entry's directory name doubles as both the
/// identifier and the prefix root.
fn discover_bottles_prefixes() -> Vec<WinePrefix> {
    let Some(home) = home() else {
        return Vec::new();
    };
    let candidates = [
        home.join(".local/share/bottles/bottles"),
        home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    ];

    let mut out: Vec<WinePrefix> = Vec::new();
    for root in &candidates {
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let bottle_path = entry.path();
            if !bottle_path.join("drive_c").is_dir() {
                continue;
            }
            let Some(name) = bottle_path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            out.push(WinePrefix {
                kind: PrefixKind::Bottles,
                identifier: name.to_string(),
                prefix_root: bottle_path,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Run `f` with `HOME` pointed at `home`, holding the crate's
    /// process-wide env lock so parallel tests don't race.
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
    fn list_wine_prefixes_empty_on_windows() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(tmp.path(), || {
            // Windows: Proton wrapper inspects Windows Steam roots (none
            // exist under the isolated HOME), Lutris/Bottles are skipped
            // because the OS branch only runs on Linux.
            let prefixes = list_wine_prefixes(Os::Windows);
            assert!(
                prefixes.is_empty(),
                "expected no prefixes on Windows with empty HOME, got {prefixes:?}"
            );
        });
    }

    #[test]
    fn list_wine_prefixes_includes_lutris() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let prefix_root = home
            .join(".local/share/lutris/runners/wine")
            .join("lutris-fshack-7.2-x86_64")
            .join("prefixes")
            .join("some-game");
        std::fs::create_dir_all(prefix_root.join("drive_c")).unwrap();

        // A second runner dir without a `drive_c/` must be ignored so we
        // do not surface half-created Lutris installs.
        std::fs::create_dir_all(
            home.join(".local/share/lutris/runners/wine/wine-ge-8-26-x86_64/prefixes/half-baked"),
        )
        .unwrap();

        with_home(home, || {
            let prefixes = list_wine_prefixes(Os::Linux);
            let lutris: Vec<&WinePrefix> = prefixes
                .iter()
                .filter(|p| p.kind == PrefixKind::Lutris)
                .collect();
            assert_eq!(
                lutris.len(),
                1,
                "expected exactly one Lutris prefix, got {prefixes:?}"
            );
            assert_eq!(lutris[0].identifier, "some-game");
            assert_eq!(lutris[0].prefix_root, prefix_root);
        });
    }

    #[test]
    fn list_wine_prefixes_includes_bottles_native() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let bottle_root = home.join(".local/share/bottles/bottles/MyBottle");
        std::fs::create_dir_all(bottle_root.join("drive_c")).unwrap();

        with_home(home, || {
            let prefixes = list_wine_prefixes(Os::Linux);
            let bottles: Vec<&WinePrefix> = prefixes
                .iter()
                .filter(|p| p.kind == PrefixKind::Bottles)
                .collect();
            assert_eq!(bottles.len(), 1, "got {prefixes:?}");
            assert_eq!(bottles[0].identifier, "MyBottle");
            assert_eq!(bottles[0].prefix_root, bottle_root);
        });
    }

    #[test]
    fn parse_wineprefix_handles_quotes_and_bare() {
        let double = "Exec=env WINEPREFIX=\"/home/u/.wine64\" wine-stable \"C:\\\\x\"";
        assert_eq!(
            parse_wineprefix_assignment(double),
            Some(PathBuf::from("/home/u/.wine64"))
        );
        let bare = "Exec=env WINEPREFIX=/home/u/prefix wine foo.exe";
        assert_eq!(
            parse_wineprefix_assignment(bare),
            Some(PathBuf::from("/home/u/prefix"))
        );
        assert_eq!(parse_wineprefix_assignment("Exec=wine foo.exe"), None);
    }

    #[test]
    fn list_wine_prefixes_includes_default_wine_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        // A default `~/.wine64` prefix with a real (non-steamuser) user home.
        let prefix = home.join(".wine64");
        std::fs::create_dir_all(prefix.join("drive_c/users/insider")).unwrap();

        with_home(home, || {
            // Isolate from any ambient WINEPREFIX the test host exports.
            let prev = std::env::var_os("WINEPREFIX");
            std::env::remove_var("WINEPREFIX");
            let prefixes = list_wine_prefixes(Os::Linux);
            if let Some(v) = prev {
                std::env::set_var("WINEPREFIX", v);
            }
            let generic: Vec<&WinePrefix> = prefixes
                .iter()
                .filter(|p| p.kind == PrefixKind::Generic)
                .collect();
            assert_eq!(generic.len(), 1, "got {prefixes:?}");
            assert_eq!(generic[0].identifier, ".wine64");
            assert_eq!(canonical(&generic[0].prefix_root), canonical(&prefix));
        });
    }

    #[test]
    fn list_wine_prefixes_finds_prefix_via_desktop_file() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        // Prefix in an arbitrary location only discoverable through a launcher.
        let prefix = home.join("games/custom-prefix");
        std::fs::create_dir_all(prefix.join("drive_c/users/insider")).unwrap();
        let apps = home.join(".local/share/applications");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(
            apps.join("Game.desktop"),
            format!(
                "[Desktop Entry]\nExec=env WINEPREFIX=\"{}\" wine-stable game.exe\n",
                prefix.display()
            ),
        )
        .unwrap();

        with_home(home, || {
            let prev = std::env::var_os("WINEPREFIX");
            std::env::remove_var("WINEPREFIX");
            let prefixes = list_wine_prefixes(Os::Linux);
            if let Some(v) = prev {
                std::env::set_var("WINEPREFIX", v);
            }
            let found = prefixes.iter().any(|p| {
                p.kind == PrefixKind::Generic && canonical(&p.prefix_root) == canonical(&prefix)
            });
            assert!(found, "desktop-referenced prefix not found: {prefixes:?}");
        });
    }

    #[test]
    fn list_wine_prefixes_includes_bottles_flatpak() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let bottle_root = home
            .join(".var/app/com.usebottles.bottles/data/bottles/bottles")
            .join("Flatpacked");
        std::fs::create_dir_all(bottle_root.join("drive_c")).unwrap();

        with_home(home, || {
            let prefixes = list_wine_prefixes(Os::Linux);
            let bottles: Vec<&WinePrefix> = prefixes
                .iter()
                .filter(|p| p.kind == PrefixKind::Bottles)
                .collect();
            assert_eq!(bottles.len(), 1, "got {prefixes:?}");
            assert_eq!(bottles[0].identifier, "Flatpacked");
            assert_eq!(bottles[0].prefix_root, bottle_root);
        });
    }
}
