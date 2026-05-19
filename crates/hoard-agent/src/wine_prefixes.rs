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

use std::path::PathBuf;

use crate::manifest::Os;
use crate::steam;

/// Which launcher owns the prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixKind {
    Proton,
    Lutris,
    Bottles,
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
    }

    out
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
