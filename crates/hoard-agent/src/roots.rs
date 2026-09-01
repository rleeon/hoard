//! Detection: enumerating the user roots (phase 0, ADR 0020).
//!
//! Lists the root directories where games keep saves, per OS, derived from the
//! placeholders `pathexpand` already knows how to expand (`<winAppData>`,
//! `<winLocalAppDataLow>`, `<xdgData>` and the rest). It is the base of the
//! catalogue-free automatic scan: the signal-driven walk (phase 1 onwards) has to
//! cover THESE roots, not just `install_dir` plus `drive_c/users/steamuser`.
//!
//! Integration note: this module is phase 0's foundation and is not yet wired
//! into `detection::detect_all`. Walking the whole HOME for every unresolved slug
//! would be explosive IO, so the real wiring waits for phase 4 (attribution),
//! which ties loose candidates to games. All that is provided here is the list of
//! roots, deduplicated and filtered down to the ones that exist on the host.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::expand_path;

/// User-root templates per OS, using Ludusavi-style placeholders.
fn root_templates(os: Os) -> &'static [&'static str] {
    match os {
        Os::Windows => &[
            "<winAppData>",         // Roaming
            "<winLocalAppData>",    // Local
            "<winLocalAppDataLow>", // LocalLow: Unity Application.persistentDataPath
            "<winSavedGames>",
            "<home>/Documents",
            "<home>/Documents/My Games",
        ],
        Os::Linux => &[
            "<xdgData>",   // ~/.local/share
            "<xdgConfig>", // ~/.config
            "<home>/.local/state",
            "<home>/Documents",
            // Native, non-Proton games that write into a Windows-style "Saved
            // Games" inside HOME (cross-platform Unity and Unreal, several
            // indies). Without this, only Wine prefixes were looked at.
            "<home>/Saved Games",
        ],
        Os::Mac => &["<macAppSupport>", "<macPreferences>", "<home>/Documents"],
    }
}

/// The native user roots that exist on this host, deduplicated.
pub fn user_save_roots(os: Os) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for tmpl in root_templates(os) {
        for p in expand_path(tmpl, os) {
            if seen.insert(p.clone()) && p.is_dir() {
                out.push(p);
            }
        }
    }
    out
}

/// Extra roots only the deep scan walks (Linux): sandboxed gaming and emulators,
/// which the periodic tick skips because of the cost. Covers:
///
/// - Flatpak: per-app data in `~/.var/app/<id>/{config,data,.local/share,
///   .config}`, so Steam Deck, Flatpak Heroic, Lutris and Bottles, and the
///   EmuDeck and RetroDECK emulators.
/// - Snap: `~/snap/<app>/{common,current}/.local/share` and `/.config`.
/// - EmuDeck and RetroDECK: `~/Emulation/saves`, `~/Emulation/storage`, and the
///   microSD copies at `/run/media/<user>/<label>/Emulation/saves`.
///
/// All filtered down to the ones that exist; empty on anything but Linux.
pub fn deep_save_roots(os: Os) -> Vec<PathBuf> {
    if !matches!(os, Os::Linux) {
        return Vec::new();
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>| {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
    };

    // Flatpak: one entry per installed app id under ~/.var/app.
    if let Ok(entries) = std::fs::read_dir(home.join(".var/app")) {
        for app in entries.flatten().map(|e| e.path()) {
            for sub in ["config", "data", ".local/share", ".config"] {
                push(app.join(sub), &mut out, &mut seen);
            }
        }
    }

    // Snap: per-app data lives under ~/snap/<app>/{common,current}.
    if let Ok(entries) = std::fs::read_dir(home.join("snap")) {
        for app in entries.flatten().map(|e| e.path()) {
            for rev in ["common", "current"] {
                push(app.join(rev).join(".local/share"), &mut out, &mut seen);
                push(app.join(rev).join(".config"), &mut out, &mut seen);
            }
        }
    }

    // EmuDeck / RetroDECK conventional save roots, local and on microSD.
    push(home.join("Emulation/saves"), &mut out, &mut seen);
    push(home.join("Emulation/storage"), &mut out, &mut seen);
    if let Ok(mounts) = std::fs::read_dir("/run/media") {
        for user in mounts.flatten().map(|e| e.path()) {
            if let Ok(vols) = std::fs::read_dir(&user) {
                for vol in vols.flatten().map(|e| e.path()) {
                    push(vol.join("Emulation/saves"), &mut out, &mut seen);
                    push(vol.join("Emulation/storage"), &mut out, &mut seen);
                }
            }
        }
    }

    out
}

/// The storefront roots that aren't Steam's, for the `<root>` placeholder: one
/// entry per row of [`pathexpand::NON_STEAM_STORE_ROOTS`], filtered to the ones
/// installed here.
///
/// Native only, and Windows-only in practice: no such launcher ships a Linux
/// or macOS build, and under Proton the same roots live inside the prefix,
/// where `pathexpand::expand_path_in_prefix_as_user` resolves them from the
/// same table.
pub fn other_store_roots(os: Os) -> Vec<PathBuf> {
    if !matches!(os, Os::Windows) {
        return Vec::new();
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    for store in crate::pathexpand::NON_STEAM_STORE_ROOTS {
        // The env vars are the only way to `Program Files`, since `pathexpand`
        // carries no placeholder for it, because no save template needs one.
        for key in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(base) = std::env::var_os(key) {
                candidates.push(PathBuf::from(base).join(store.program_files));
            }
        }
        if let Some(local) = store.local_appdata {
            for p in expand_path("<winLocalAppData>", os) {
                candidates.push(p.join(local));
            }
        }
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for p in candidates {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
    }
    out
}

/// Folders where people group unpacked programs. Both one level in and the
/// drive root itself get looked at.
const COLLECTION_DIRS: &[&str] = &["Emulators", "Emulation", "Emus", "Games", "Juegos", "ROMs"];

/// Where to look for programs installed by unpacking a folder rather than by
/// running an installer.
///
/// There are two: the root of each internal drive (`D:\RetroArch`) and one level
/// inside a collection folder (`D:\Emulators\RetroArch`). It returns the
/// directories to list, not the candidates: the caller decides which names count.
///
/// Deliberately bounded, and that is half the design: one listing per drive plus
/// one per collection, with nothing walked below. Sweeping a games disk would
/// read tens of thousands of directories to find a handful of hits, and every
/// scan's startup would pay for it.
///
/// Removable, optical and network drives are skipped: a disconnected share blocks
/// for seconds on every call, and the whole scan would feel that cost.
pub fn portable_install_roots(os: Os) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
    };

    for drive in internal_drive_roots(os) {
        for dir in COLLECTION_DIRS {
            push(drive.join(dir), &mut out);
        }
        push(drive, &mut out);
    }
    out
}

/// The roots of this machine's internal drives.
///
/// On Windows those are the fixed drive letters. On Linux and macOS there are no
/// letters, so the usual mount points for secondary disks are taken, which is
/// where a second SSD or a Deck's microSD ends up.
#[cfg(windows)]
pub fn internal_drive_roots(_os: Os) -> Vec<PathBuf> {
    // `DRIVE_FIXED` does not sit next to the two functions that consume it; it
    // lives in `System::WindowsProgramming`. It compiles from either place, so the
    // mistake only shows up when building for Windows.
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;

    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        // `GetDriveTypeW` wants the root with a trailing slash, as null-terminated
        // en nulo: "D:\\\0".
        let root: Vec<u16> = format!("{letter}:\\\0").encode_utf16().collect();
        // SAFETY: `root` is valid null-terminated UTF-16 and lives for the whole
        // call.
        if unsafe { GetDriveTypeW(root.as_ptr()) } == DRIVE_FIXED {
            out.push(PathBuf::from(format!("{letter}:\\")));
        }
    }
    out
}

/// The non-Windows equivalent: the mount points where a secondary disk turns up.
/// `/media/<user>` and `/run/media/<user>` are what Linux desktops use (and the
/// Deck for its microSD); `/mnt` is the by-hand mount of long tradition;
/// `/Volumes` is macOS's.
#[cfg(not(windows))]
pub fn internal_drive_roots(os: Os) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
    };

    let containers: &[&str] = match os {
        Os::Mac => &["/Volumes"],
        _ => &["/media", "/run/media", "/mnt"],
    };
    for container in containers {
        let Ok(entries) = std::fs::read_dir(container) else {
            continue;
        };
        for entry in entries.flatten().map(|e| e.path()) {
            if !entry.is_dir() {
                continue;
            }
            // `/media/<user>/<volume>` and `/media/<volume>` coexist depending
            // on the distro, so both levels are accepted.
            let mut had_child = false;
            if let Ok(children) = std::fs::read_dir(&entry) {
                for child in children.flatten().map(|e| e.path()) {
                    if child.is_dir() {
                        had_child = true;
                        push(child, &mut out);
                    }
                }
            }
            if !had_child {
                push(entry, &mut out);
            }
        }
    }
    out
}

/// Real Windows user names inside a Wine or Proton prefix.
///
/// Lists the directories under `drive_c/users/` that are real users: Proton uses
/// `steamuser`, while generic prefixes (`wine`, PlayOnLinux, `.desktop`
/// launchers) use the host login (`$USER`). Excludes `Public`, which is not a
/// user profile, and non-directory entries. Empty when the prefix does not exist
/// or has no `drive_c/users/`.
pub fn prefix_windows_users(prefix: &Path) -> Vec<String> {
    let users_dir = prefix.join("drive_c/users");
    let entries = match std::fs::read_dir(&users_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.eq_ignore_ascii_case("Public") {
            continue;
        }
        out.push(name);
    }
    out
}

/// The per-user subdirectories inside a Wine or Proton prefix where saves land,
/// for every real user of the prefix. Same Windows naming as
/// `pathexpand::expand_placeholder_in_prefix`. `prefix` points at the directory
/// that directly contains `drive_c/`.
pub fn prefix_user_roots(prefix: &Path) -> Vec<PathBuf> {
    prefix_windows_users(prefix)
        .iter()
        .flat_map(|user| prefix_user_roots_for(prefix, user))
        .collect()
}

/// Subdirectorios de save de un usuario Windows concreto dentro de un prefijo.
pub fn prefix_user_roots_for(prefix: &Path, user: &str) -> Vec<PathBuf> {
    let userhome = prefix.join("drive_c/users").join(user);
    [
        "AppData/Roaming",
        "AppData/Local",
        "AppData/LocalLow",
        "Documents",
        "Saved Games",
    ]
    .iter()
    .map(|sub| userhome.join(sub))
    .filter(|p| p.is_dir())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_non_empty_per_os() {
        for os in [Os::Windows, Os::Linux, Os::Mac] {
            assert!(!root_templates(os).is_empty());
        }
    }

    #[test]
    fn user_save_roots_runs_and_dedups() {
        // No panics; result is deduplicated (existence depends on host).
        let roots = user_save_roots(Os::current());
        let mut seen = HashSet::new();
        for r in &roots {
            assert!(seen.insert(r.clone()), "duplicate root: {r:?}");
        }
    }

    #[test]
    fn prefix_user_roots_filters_missing() {
        // A bogus prefix has none of the steamuser subdirs.
        let roots = prefix_user_roots(Path::new("/nonexistent/prefix/pfx"));
        assert!(roots.is_empty());
    }
}
