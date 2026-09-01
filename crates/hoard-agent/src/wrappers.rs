//! Detection: container folders, meaning Steam emulators and repacks.
//!
//! A "wrapper" is a folder that groups one subdirectory per game, usually named
//! after the Steam AppID:
//!
//! ```text
//! %APPDATA%/Goldberg SteamEmu Saves/413150/remote/...
//! %PUBLIC%/Documents/Steam/CODEX/1091500/remote/...
//! ```
//!
//! Without this stage those folders were only ever reached by the generic phase-4
//! walk, which does not know the subdirectory is an AppID or that the real save is
//! in `remote/`. Two real bugs came out of that: `GSE Saves` ended up tracked
//! under a slug made from the Windows account name, and a save was labelled with
//! an installer's name. Here the AppID is resolved against the catalogue, so the
//! game comes out with its own name and cover art, and the folder offered is the
//! one with the saves rather than the container.
//!
//! The container also matters for a sync reason: alongside `remote/` there are
//! `remotecache.vdf`, achievements, statistics and playtime counters that change
//! every session and differ on every machine. Tracking the parent turns that into
//! a permanent conflict between devices without a single save having moved.

use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::{expand_path, expand_path_in_prefix_as_user};

/// A known wrapper: where it lives and what it is called in the UI.
struct Wrapper {
    /// Plantilla estilo Ludusavi, resuelta igual en el host y dentro de un
    /// prefijo Wine.
    template: &'static str,
    /// Label for the log and for naming a find with no AppID.
    label: &'static str,
}

/// The Steam emulators and repacks that group saves by AppID.
///
/// All of them are Windows conventions; on Linux the same games run under Proton
/// and these paths live inside the prefix, which is exactly why
/// [`discover_wrappers_in_prefix`] exists.
const WRAPPERS: &[Wrapper] = &[
    Wrapper {
        template: "<winAppData>/Goldberg SteamEmu Saves",
        label: "Goldberg",
    },
    Wrapper {
        template: "<winAppData>/GSE Saves",
        label: "Goldberg (GSE)",
    },
    Wrapper {
        template: "<winPublic>/Documents/Steam/CODEX",
        label: "CODEX",
    },
    Wrapper {
        template: "<winPublic>/Documents/Steam/RUNE",
        label: "RUNE",
    },
    Wrapper {
        template: "<winDocuments>/Steam/TENOKE",
        label: "TENOKE",
    },
    Wrapper {
        template: "<winPublic>/Documents/EMPRESS",
        label: "EMPRESS",
    },
    Wrapper {
        template: "<winPublic>/Documents/OnlineFix",
        label: "Online-Fix",
    },
    Wrapper {
        template: "<winPublic>/Documents/CPY_SAVES",
        label: "CPY",
    },
    Wrapper {
        template: "<winAppData>/SmartSteamEmu",
        label: "SmartSteamEmu",
    },
    Wrapper {
        template: "<winAppData>/SKIDROW",
        label: "SKIDROW",
    },
    Wrapper {
        template: "<winLocalAppData>/SKIDROW",
        label: "SKIDROW",
    },
    Wrapper {
        template: "<winPublic>/Documents/3DMGAME",
        label: "3DM",
    },
    Wrapper {
        template: "<winAppData>/FLT",
        label: "Fairlight",
    },
    Wrapper {
        template: "<winAppData>/ALi",
        label: "ALi",
    },
    Wrapper {
        template: "<winProgramData>/Steam/RLD!",
        label: "RELOADED",
    },
    // The generic one goes last: `%PUBLIC%/Documents/Steam` contains CODEX and
    // RUNE as subfolders, and whoever looks first wins (see `is_app_id`, which
    // discards those names for not being numeric).
    Wrapper {
        template: "<winPublic>/Documents/Steam",
        label: "Steam emu",
    },
];

/// Subfolders of a wrapper that are the emulator's own configuration rather than
/// a game. `saves` and `remote` show up when the emulator stores flat instead of
/// by AppID; there the whole container IS the save and the ordinary walk handles
/// it, not this stage.
const WRAPPER_SYSTEM_DIRS: &[&str] = &["settings", "remote", "saves", "stats", "storage"];

/// Un save encontrado dentro de un wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperHit {
    /// The Steam AppID when the subfolder is numeric, which is the usual case.
    pub app_id: Option<u64>,
    /// The folder that really holds the saves, already narrowed.
    pub path: PathBuf,
    /// The subfolder's name, for naming the find when there is no AppID.
    pub folder: String,
    pub wrapper: &'static str,
}

/// Wrappers on the host's native paths. Empty outside Windows: the templates are
/// `<win*>` and `expand_path` does not resolve them on other systems.
pub fn discover_wrappers(os: Os) -> Vec<WrapperHit> {
    let mut out = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for w in WRAPPERS {
        for root in expand_path(w.template, os) {
            collect(&root, w.label, &mut out, &mut seen);
        }
    }
    out
}

/// The same wrappers inside a Wine or Proton prefix, which is where they land on
/// Linux and on the Steam Deck: the repack runs under Proton and writes into the
/// prefix's `drive_c`, not into the native home.
pub fn discover_wrappers_in_prefix(prefix_root: &Path, user: &str) -> Vec<WrapperHit> {
    let mut out = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for w in WRAPPERS {
        for root in expand_path_in_prefix_as_user(w.template, prefix_root, user) {
            collect(&root, w.label, &mut out, &mut seen);
        }
    }
    out
}

/// Lists a wrapper's games. `seen` stops the generic `.../Documents/Steam`
/// wrapper re-offering what CODEX and RUNE already gave.
fn collect(root: &Path, label: &'static str, out: &mut Vec<WrapperHit>, seen: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(folder) = name.to_str() else {
            continue;
        };
        let lower = folder.to_lowercase();
        if WRAPPER_SYSTEM_DIRS.contains(&lower.as_str())
            || crate::junkdirs::is_cache_dir_name(folder)
        {
            continue;
        }
        let container = entry.path();
        // A subdirectory already covered by a more specific wrapper (CODEX or
        // RUNE inside `Documents/Steam`) is not repeated.
        if seen.iter().any(|s| s == &container) {
            continue;
        }
        let path = resolve_game_container_dir(&container);
        if !dir_non_empty(&path) || !holds_anything_but_bookkeeping(&path) {
            continue;
        }
        seen.push(container);
        out.push(WrapperHit {
            app_id: folder.parse::<u64>().ok().filter(|_| is_app_id(folder)),
            path,
            folder: folder.to_string(),
            wrapper: label,
        });
    }
}

/// `true` when the folder name is a Steam AppID: digits only.
fn is_app_id(name: &str) -> bool {
    !name.is_empty() && name.len() <= 10 && name.bytes().all(|b| b.is_ascii_digit())
}

/// Narrows a CONTAINER folder down to the one that really holds the saves.
///
/// Two shapes cover what is in there:
///
/// * `remote/`, the Steam Cloud layout every emulator copies.
/// * a single subdirectory with a save-like name (`Saves`, `SaveData`, Unreal's
///   `Saved/SaveGames` shape) when the container wraps the game's own tree.
///
/// With neither, the container itself is returned: plenty of games and emulators
/// write straight into it. And when there are several candidates nothing is
/// guessed, because being half right is worse than offering the container, which
/// the user can see and correct.
pub fn resolve_game_container_dir(dir: &Path) -> PathBuf {
    let remote = dir.join("remote");
    if remote.is_dir() {
        return remote;
    }
    let nested = crate::junkdirs::save_dirs_under(dir);
    if nested.len() == 1 && nested[0] != dir {
        return nested[0].clone();
    }
    dir.to_path_buf()
}

fn dir_non_empty(p: &Path) -> bool {
    std::fs::read_dir(p).is_ok_and(|mut r| r.next().is_some())
}

/// Files the emulator writes for itself. None of them is a saved game:
/// achievements, stats, the Steam Cloud cache and the subscribed-groups lists
/// all appear on their own the first time the game runs, saved or not.
const WRAPPER_BOOKKEEPING_FILES: &[&str] = &[
    "achievements.json",
    "remotecache.vdf",
    "stats.txt",
    "stats.bin",
    "leaderboards.json",
    "subscribed_groups.json",
    "subscribed_groups_clans.json",
    "time.txt",
];

/// `true` if the folder holds anything that could be a saved game.
///
/// [`dir_non_empty`] wasn't enough, and Stellaris is the case that showed it: a
/// Goldberg repack leaves `GSE Saves/281990/achievements.json` with no `remote/`,
/// because the real saves live where the game has always put them
/// (`Documents/Paradox Interactive/Stellaris/save games`). That folder isn't
/// empty, so it was offered as the game's save on every sweep: a log line every
/// ten minutes, forever, about a directory with no game in it.
/// known emulator bookkeeping. One unknown file, one subdirectory, anything off
/// the list, and the folder passes: missing a real save is far worse than one
/// spurious offer.
fn holds_anything_but_bookkeeping(p: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(p) else {
        return false;
    };
    for entry in read.flatten() {
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            return true;
        }
        let name = entry.file_name();
        let lower = name.to_string_lossy().to_lowercase();
        if !WRAPPER_BOOKKEEPING_FILES.contains(&lower.as_str()) {
            return true;
        }
    }
    // Nothing worth offering, the empty case included. `dir_non_empty` also
    // rejects that one, and the two agreeing is the point: a caller that ever
    // drops the other check doesn't silently start offering empty folders.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn container_narrows_to_remote() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("413150");
        touch(&app.join("remote/save.dat"));
        // Exactly the litter that makes two machines diverge if the parent is
        // tracked.
        touch(&app.join("remotecache.vdf"));
        touch(&app.join("playtime.txt"));
        assert_eq!(resolve_game_container_dir(&app), app.join("remote"));
    }

    /// The Stellaris case: the repack leaves the achievements file and nothing
    /// else, because the real saves go where the game has always written them.
    #[test]
    fn a_folder_with_only_emulator_bookkeeping_is_not_a_save() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("281990");
        touch(&app.join("achievements.json"));
        assert!(
            dir_non_empty(&app),
            "not empty, which is why it used to pass"
        );
        assert!(
            !holds_anything_but_bookkeeping(&app),
            "emulator bookkeeping only: not a save"
        );

        // One unknown file and the folder passes again: rejecting too much
        // costs a save, rejecting too little costs a log line.
        touch(&app.join("campaign01.sav"));
        assert!(holds_anything_but_bookkeeping(&app));
    }

    #[test]
    fn container_narrows_to_a_single_save_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("1091500");
        touch(&app.join("Saved/SaveGames/slot.sav"));
        // The more specific child wins over `Saved`.
        assert_eq!(
            resolve_game_container_dir(&app),
            app.join("Saved").join("SaveGames")
        );
    }

    #[test]
    fn container_stays_put_when_ambiguous_or_flat() {
        let tmp = tempfile::tempdir().unwrap();
        // Flat: the save hangs directly off the container.
        let flat = tmp.path().join("flat");
        touch(&flat.join("game.sav"));
        assert_eq!(resolve_game_container_dir(&flat), flat);

        // Ambiguous: two candidates, so nothing is guessed.
        let ambiguous = tmp.path().join("ambiguous");
        touch(&ambiguous.join("saves/a.sav"));
        touch(&ambiguous.join("savedata/b.sav"));
        assert_eq!(resolve_game_container_dir(&ambiguous), ambiguous);
    }

    #[test]
    fn collect_reads_appids_and_skips_emulator_plumbing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("GSE Saves");
        touch(&root.join("413150/remote/save.dat"));
        touch(&root.join("settings/user.ini")); // config del emulador
        touch(&root.join("MyGame/saves/x.sav")); // sin AppID, pero es un juego
        std::fs::create_dir_all(root.join("empty")).unwrap(); // empty: ignored

        let mut out = Vec::new();
        let mut seen = Vec::new();
        collect(&root, "GSE", &mut out, &mut seen);
        out.sort_by(|a, b| a.folder.cmp(&b.folder));

        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0].app_id, Some(413150));
        assert_eq!(out[0].path, root.join("413150").join("remote"));
        assert_eq!(out[1].app_id, None);
        assert_eq!(out[1].folder, "MyGame");
        assert_eq!(out[1].path, root.join("MyGame").join("saves"));
    }

    #[test]
    fn a_non_numeric_folder_is_not_an_appid() {
        assert!(is_app_id("413150"));
        assert!(!is_app_id("CODEX"));
        assert!(!is_app_id(""));
        // An absurdly long name is not an AppID even when it is numeric.
        assert!(!is_app_id("12345678901234"));
    }
}
