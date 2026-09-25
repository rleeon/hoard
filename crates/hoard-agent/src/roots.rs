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
///   copies on every other drive ([`internal_drive_roots`]).
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

    // EmuDeck / RetroDECK conventional save roots, local and on every other
    // drive. The mount points come from `internal_drive_roots` so that a microSD
    // under `/run/media` is not the only other disk a save can be on.
    push(home.join("Emulation/saves"), &mut out, &mut seen);
    push(home.join("Emulation/storage"), &mut out, &mut seen);
    for vol in internal_drive_roots(os) {
        push(vol.join("Emulation/saves"), &mut out, &mut seen);
        push(vol.join("Emulation/storage"), &mut out, &mut seen);
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
/// `/var/mnt` is where the rpm-ostree distros put it, Bazzite and Silverblue and
/// the rest, `/var` being the writable half of an image-based system;
/// `/Volumes` is macOS's.
#[cfg(not(windows))]
pub fn internal_drive_roots(os: Os) -> Vec<PathBuf> {
    drive_roots_under(mount_containers(os).iter().map(Path::new))
}

#[cfg(not(windows))]
fn mount_containers(os: Os) -> &'static [&'static str] {
    match os {
        Os::Mac => &["/Volumes"],
        _ => &["/media", "/run/media", "/mnt", "/var/mnt"],
    }
}

/// `true` when `path` is missing because the drive it lives on is not connected
/// (a microSD in the other handheld, an external disk unplugged), as opposed to
/// a folder somebody deleted. The sync engine needs the difference: a deleted
/// save is restored, an absent drive is waited for. Treated as deleted, the
/// Deck's `/run/media/deck/<card>` sent a restore every hour that downloaded
/// the whole snapshot and then failed creating the mount point.
///
/// The rule looks at the nearest ancestor that does exist. The drive is gone
/// when that ancestor is a mount container itself (`/run/media`), or a folder
/// right under one (`/run/media/deck`, `/mnt/games`) that is not a mounted
/// filesystem. A drive that is mounted and simply lacks the folder answers
/// `false`, and so does anything outside the containers.
#[cfg(not(windows))]
pub fn volume_offline(path: &Path) -> bool {
    volume_offline_under(path, mount_containers(Os::current()))
}

#[cfg(not(windows))]
fn volume_offline_under(path: &Path, containers: &[&str]) -> bool {
    use std::os::unix::fs::MetadataExt;

    if path.symlink_metadata().is_ok() {
        return false;
    }
    let Some(existing) = path.ancestors().skip(1).find(|a| a.exists()) else {
        return false;
    };
    let is_container = |p: &Path| containers.iter().any(|c| p == Path::new(c));
    if is_container(existing) {
        return true;
    }
    let Some(parent) = existing.parent() else {
        return false;
    };
    if !is_container(parent) {
        return false;
    }
    // Right under a container: a mounted drive has its own device, an empty
    // mount point left behind by an unmounted one shares its parent's.
    match (existing.metadata(), parent.metadata()) {
        (Ok(here), Ok(up)) => here.dev() == up.dev(),
        _ => false,
    }
}

/// Windows: the drive letter is not there (`E:\` with the USB disk unplugged).
/// Network shares are left alone: probing a dead one can block for seconds.
#[cfg(windows)]
pub fn volume_offline(path: &Path) -> bool {
    use std::path::{Component, Prefix};

    if path.exists() {
        return false;
    }
    match path.components().next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                !Path::new(&format!("{}:\\", letter as char)).exists()
            }
            _ => false,
        },
        _ => false,
    }
}

/// The two levels below each container, which is the rule the function above is
/// made of. It takes the containers so a test can hand it a temporary tree
/// instead of the machine's own mount points.
#[cfg(not(windows))]
fn drive_roots_under<'a>(containers: impl Iterator<Item = &'a Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if seen.insert(p.clone()) && p.is_dir() {
            out.push(p);
        }
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
            // on the distro, so both levels are taken. Both, and not whichever
            // of the two has subdirectories: a disk mounted by hand at
            // `/mnt/games` with an `Emulation/` inside it used to come back as
            // `/mnt/games/Emulation` alone, and every caller then looked for
            // `Emulation/saves` one level too deep. A root that leads nowhere
            // costs each caller one `stat`.
            push(entry.clone(), &mut out);
            let Ok(children) = std::fs::read_dir(&entry) else {
                continue;
            };
            for child in children.flatten().map(|e| e.path()) {
                if child.is_dir() {
                    push(child, &mut out);
                }
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

    #[cfg(not(windows))]
    #[test]
    fn volume_offline_tells_a_missing_drive_from_a_missing_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let media = tmp.path().join("media");
        let user = media.join("deck");
        std::fs::create_dir_all(&user).unwrap();
        let containers = [media.to_str().unwrap()];

        // The card is not mounted: nothing under the user's folder.
        let on_card = user.join("41b8a2a9/steamapps/common/Oceanhorn/SaveFiles");
        assert!(volume_offline_under(&on_card, &containers));

        // The container itself is gone (no drive ever mounted this boot).
        std::fs::remove_dir(&user).unwrap();
        assert!(volume_offline_under(&on_card, &containers));
        std::fs::create_dir_all(&user).unwrap();

        // Mounted drive, deleted game folder: restore it, don't wait.
        let steamapps = user.join("41b8a2a9/steamapps");
        std::fs::create_dir_all(&steamapps).unwrap();
        assert!(!volume_offline_under(
            &steamapps.join("common/Oceanhorn/SaveFiles"),
            &containers
        ));

        // Outside every container a missing folder is just missing.
        assert!(!volume_offline_under(
            &tmp.path().join("home/deck/.config/Game/saves"),
            &containers
        ));

        // A folder that exists is never offline.
        assert!(!volume_offline_under(&steamapps, &containers));
    }

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

    /// The disk itself is a root, not only the folders inside it. Both layouts
    /// a distro can mount with are in the same tree here: `<container>/<volume>`
    /// and `<container>/<user>/<volume>`, and a caller that joins
    /// `Emulation/saves` has to reach it in either.
    #[cfg(not(windows))]
    #[test]
    fn a_mounted_disk_is_a_root_even_when_it_has_folders_inside() {
        let tmp = tempfile::tempdir().unwrap();
        let mnt = tmp.path().join("mnt");
        std::fs::create_dir_all(mnt.join("games/Emulation/saves")).unwrap();
        let media = tmp.path().join("media");
        std::fs::create_dir_all(media.join("insider/SD/Emulation/saves")).unwrap();

        let roots = drive_roots_under([mnt.as_path(), media.as_path()].into_iter());

        assert!(
            roots.contains(&mnt.join("games")),
            "the disk mounted by hand is missing, got {roots:?}"
        );
        assert!(
            roots.contains(&media.join("insider/SD")),
            "the volume under the user directory is missing, got {roots:?}"
        );
        // And every one of them exactly once.
        let mut seen = HashSet::new();
        for r in &roots {
            assert!(seen.insert(r.clone()), "duplicate root: {r:?}");
        }
    }
}
