//! The emulator catalogue and where its save folders live.
//!
//! An emulator reuses the host's filesystem, so the engine already knows how to
//! back one up the moment somebody points at the folder. What it cannot do on its
//! own is *find* it: there is no storefront, no `install_dir`, no manifest entry.
//! This module fills that in with a curated catalogue of native save folders and
//! typical process names, plus two probes that stretch it to where people really
//! keep things:
//!
//! 1. [`resolve_save_paths`]: the catalogue's templates expanded and filtered to
//!    what exists on this host (an ordinary install).
//! 2. [`portable_save_paths`]: the same emulator unpacked on another drive, which
//!    saves next to the executable rather than in the user folder.
//! 3. [`split_per_title`]: descending from a console's save root to EACH game's
//!    folder, when the intermediate tree carries an identifier that means nothing
//!    on the other machine.
//!
//! It lives in the agent rather than the desktop because from the drive probe
//! onwards this is detection, and detection is shared by both frontends: the "add
//! emulator" dialog and `hoard scan` ask the same thing.
//!
//! The catalogue points at native saves (memory cards, per-title folders), never
//! at savestates: those depend on the emulator's exact version and do not survive
//! a trip between machines.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::manifest::Os;
use crate::pathexpand::expand_path;

/// The shape of a console's save tree, when offering the whole root is a mistake.
/// See [`split_per_title`] for the reasoning behind each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleLayout {
    /// `.../nand/user/save/<account>/<profile-uuid>/<title-id>/`, the yuzu line
    /// and its forks. The profile uuid is generated on first run, so it differs on
    /// every install.
    SwitchNand,
    /// `.../sdmc/Nintendo 3DS/<id0>/<id1>/title/<hi>/<lo>/data/`, Citra and
    /// Azahar. `id0` and `id1` derive from the emulated console's keys, so they
    /// are per-install too.
    Citra3ds,
}

/// A catalogue entry. Kept separate from the type that goes over the wire so the
/// templates, which are cross-platform and use Ludusavi-style placeholders, live
/// here and expand into real paths at the moment of asking.
pub struct EmulatorDef {
    /// A stable id; the UI builds the game's synthetic slug as `emu-<id>`.
    pub id: &'static str,
    pub display_name: &'static str,
    /// Consola / plataforma, como sublínea en el selector.
    pub system: &'static str,
    /// Executable names that mark the emulator as running. The agent matches any
    /// of them (case-insensitive, exact name), so the variants for every OS and
    /// build are listed.
    pub processes: &'static [&'static str],
    /// Native save-folder templates. They are expanded with [`expand_path`] and
    /// filtered to the ones that exist; empty, or all absent, means the user picks
    /// the folder by hand, which is the norm for portable emulators that save next
    /// to the ROM.
    pub save_templates: &'static [&'static str],
    /// The per-title tree's shape, when offering the whole root breaks sync.
    pub title_layout: Option<TitleLayout>,
}

/// The curated set. Conservative on purpose: a wrong suggested path is worse than
/// none (the user would end up backing up an empty folder), so only folders the
/// emulator uses for native saves in a default install are listed.
pub const CATALOG: &[EmulatorDef] = &[
    EmulatorDef {
        id: "pcsx2",
        display_name: "PCSX2",
        system: "PlayStation 2",
        processes: &[
            "pcsx2-qt.exe",
            "pcsx2-qtx64.exe",
            "pcsx2-qtx64-avx2.exe",
            "pcsx2.exe",
            "pcsx2",
        ],
        save_templates: &[
            "<winDocuments>/PCSX2/memcards",
            "<xdgConfig>/PCSX2/memcards",
            "<home>/.var/app/net.pcsx2.PCSX2/config/PCSX2/memcards",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "rpcs3",
        display_name: "RPCS3",
        system: "PlayStation 3",
        processes: &["rpcs3.exe", "rpcs3"],
        save_templates: &[
            "<xdgConfig>/rpcs3/dev_hdd0/home/00000001/savedata",
            "<home>/.config/rpcs3/dev_hdd0/home/00000001/savedata",
            "<home>/.var/app/net.rpcs3.RPCS3/config/rpcs3/dev_hdd0/home/00000001/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "duckstation",
        display_name: "DuckStation",
        system: "PlayStation 1",
        processes: &[
            "duckstation-qt-x64-ReleaseLTCG.exe",
            "duckstation-nogui-x64-ReleaseLTCG.exe",
            "duckstation-qt",
            "duckstation",
        ],
        // Windows moved to Local AppData; the README keeps Documents only for
        // "old installs", so it stays listed but never first — an install that
        // predates the move still has the folder, and existence filtering
        // picks whichever is real. Linux is the data dir, not config: the
        // official migration command moves the Flatpak tree *into*
        // `~/.local/share`, and the Flatpak itself has been seen under both
        // `config/` and `data/`, so both are offered and the one that exists
        // wins.
        save_templates: &[
            "<winLocalAppData>/DuckStation/memcards",
            "<winDocuments>/DuckStation/memcards",
            "<xdgData>/duckstation/memcards",
            "<home>/.var/app/org.duckstation.DuckStation/data/duckstation/memcards",
            "<home>/.var/app/org.duckstation.DuckStation/config/duckstation/memcards",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "shadps4",
        display_name: "shadPS4",
        system: "PlayStation 4",
        processes: &["shadPS4.exe", "shadps4"],
        save_templates: &[
            "<winAppData>/shadPS4/savedata",
            "<xdgData>/shadPS4/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "vita3k",
        display_name: "Vita3K",
        system: "PlayStation Vita",
        processes: &["Vita3K.exe", "Vita3K", "vita3k"],
        save_templates: &[
            "<winAppData>/Vita3K/Vita3K/ux0/user/00/savedata",
            "<xdgConfig>/Vita3K/Vita3K/ux0/user/00/savedata",
            "<home>/.local/share/Vita3K/Vita3K/ux0/user/00/savedata",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "ppsspp",
        display_name: "PPSSPP",
        system: "PSP",
        processes: &[
            "PPSSPPWindows64.exe",
            "PPSSPPWindows.exe",
            "PPSSPPSDL",
            "ppsspp-qt",
            "ppsspp",
        ],
        save_templates: &[
            "<winDocuments>/PPSSPP/PSP/SAVEDATA",
            "<xdgConfig>/ppsspp/PSP/SAVEDATA",
            "<home>/.config/ppsspp/PSP/SAVEDATA",
            "<home>/.var/app/org.ppsspp.PPSSPP/config/ppsspp/PSP/SAVEDATA",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "dolphin",
        display_name: "Dolphin",
        system: "GameCube / Wii",
        processes: &["Dolphin.exe", "dolphin-emu", "dolphin-emu-qt2"],
        save_templates: &[
            "<winDocuments>/Dolphin Emulator/GC",
            "<winDocuments>/Dolphin Emulator/Wii",
            "<xdgData>/dolphin-emu/GC",
            "<xdgData>/dolphin-emu/Wii",
            "<home>/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/GC",
            "<home>/.var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu/Wii",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "cemu",
        display_name: "Cemu",
        system: "Wii U",
        processes: &["Cemu.exe", "cemu"],
        save_templates: &[
            "<winAppData>/Cemu/mlc01/usr/save",
            "<home>/.local/share/Cemu/mlc01/usr/save",
            "<home>/.var/app/info.cemu.Cemu/data/Cemu/mlc01/usr/save",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "ryujinx",
        display_name: "Ryujinx",
        system: "Switch",
        processes: &["Ryujinx.exe", "Ryujinx.Ava.exe", "Ryujinx"],
        // `bis/user/save/<save-data-id>`: the id is assigned by the emulator
        // itself and only its internal database knows which title it belongs to,
        // so it CANNOT be split per title by looking at the folder name. The root
        // is offered, as usual.
        save_templates: &[
            "<winAppData>/Ryujinx/bis/user/save",
            "<xdgConfig>/Ryujinx/bis/user/save",
            "<home>/.local/share/Ryujinx/bis/user/save",
            "<home>/.var/app/org.ryujinx.Ryujinx/config/Ryujinx/bis/user/save",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "yuzu",
        display_name: "yuzu",
        system: "Switch",
        processes: &["yuzu.exe", "yuzu-cmd.exe", "yuzu"],
        save_templates: &[
            "<winAppData>/yuzu/nand/user/save",
            "<home>/.local/share/yuzu/nand/user/save",
            "<home>/.var/app/org.yuzu_emu.yuzu/data/yuzu/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "eden",
        display_name: "Eden",
        system: "Switch",
        processes: &["eden.exe", "eden"],
        save_templates: &[
            "<winAppData>/eden/nand/user/save",
            "<home>/.local/share/eden/nand/user/save",
            "<home>/.var/app/dev.eden_emu.eden/data/eden/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "suyu",
        display_name: "Suyu",
        system: "Switch",
        processes: &["suyu.exe", "suyu"],
        save_templates: &[
            "<winAppData>/suyu/nand/user/save",
            "<home>/.local/share/suyu/nand/user/save",
            "<home>/.var/app/dev.suyu_emu.suyu/data/suyu/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "citron",
        display_name: "Citron",
        system: "Switch",
        processes: &["citron.exe", "citron"],
        save_templates: &[
            "<winAppData>/citron/nand/user/save",
            "<home>/.local/share/citron/nand/user/save",
            "<home>/.var/app/org.citron_emu.citron/data/citron/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "sudachi",
        display_name: "Sudachi",
        system: "Switch",
        processes: &["sudachi.exe", "sudachi"],
        save_templates: &[
            "<winAppData>/sudachi/nand/user/save",
            "<home>/.local/share/sudachi/nand/user/save",
        ],
        title_layout: Some(TitleLayout::SwitchNand),
    },
    EmulatorDef {
        id: "citra",
        display_name: "Citra / Azahar",
        system: "Nintendo 3DS",
        processes: &[
            "citra-qt.exe",
            "azahar.exe",
            "lime3ds.exe",
            "citra.exe",
            "citra-qt",
            "lime3ds",
            "citra",
        ],
        save_templates: &[
            "<winAppData>/Citra/sdmc",
            "<winAppData>/Azahar/sdmc",
            "<winAppData>/Lime3DS/sdmc",
            "<xdgData>/citra-emu/sdmc",
            "<xdgData>/azahar-emu/sdmc",
            "<xdgData>/lime3ds-emu/sdmc",
            "<home>/.var/app/org.azahar_emu.Azahar/data/azahar-emu/sdmc",
        ],
        title_layout: Some(TitleLayout::Citra3ds),
    },
    EmulatorDef {
        id: "xemu",
        display_name: "xemu",
        system: "Xbox",
        processes: &["xemu.exe", "xemu"],
        save_templates: &[
            "<winAppData>/xemu/xemu/eeprom.bin",
            "<xdgData>/xemu/xemu",
            "<home>/.var/app/app.xemu.xemu/data/xemu/xemu",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "flycast",
        display_name: "Flycast",
        system: "Dreamcast",
        processes: &["flycast.exe", "flycast"],
        // No Windows template on purpose: the standalone build ships as a zip
        // with no installer and locates its own folder from the executable
        // path, so nothing ever lands in `%APPDATA%\flycast`. Offering it made
        // the dialog point at a folder that cannot exist. Windows installs are
        // found by `portable_save_paths`, which reuses the `flycast`/`data`
        // pair from the Linux template below — that row is load-bearing for
        // Windows detection even though it never expands there.
        save_templates: &[
            "<xdgData>/flycast/data",
            "<home>/.var/app/org.flycast.Flycast/data/flycast/data",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "retroarch",
        display_name: "RetroArch",
        system: "Multi-system",
        processes: &["retroarch.exe", "retroarch"],
        save_templates: &[
            "<winAppData>/RetroArch/saves",
            "<xdgConfig>/retroarch/saves",
            "<home>/.config/retroarch/saves",
            "<home>/.var/app/org.libretro.RetroArch/config/retroarch/saves",
        ],
        title_layout: None,
    },
    EmulatorDef {
        id: "mgba",
        display_name: "mGBA",
        system: "Game Boy Advance",
        // It saves next to the ROM by default, so there is no reliable template.
        processes: &["mGBA.exe", "mgba-qt", "mgba"],
        save_templates: &[],
        title_layout: None,
    },
    EmulatorDef {
        id: "melonds",
        display_name: "melonDS",
        system: "Nintendo DS",
        processes: &["melonDS.exe", "melonDS", "melonds"],
        save_templates: &[],
        title_layout: None,
    },
    EmulatorDef {
        id: "project64",
        display_name: "Project64",
        system: "Nintendo 64",
        processes: &["Project64.exe"],
        // Project64 is portable by design: the manual puts auto saves in the
        // `Save` subfolder of the program folder, and nothing is written to
        // `%APPDATA%`. The template is kept anyway because it is the only
        // source of the `Project64`/`Save` pair that `portable_save_paths`
        // reanchors onto a real install — delete it and Windows detection goes
        // to zero. What is still wrong is the fallback: with no folder found,
        // `resolve_save_paths` offers this path, which will never exist. That
        // needs the entry to be able to say "portable only", not a different
        // template.
        save_templates: &["<winAppData>/Project64/Save"],
        title_layout: None,
    },
];

/// Looks up a catalogue entry by its id.
pub fn find(id: &str) -> Option<&'static EmulatorDef> {
    CATALOG.iter().find(|d| d.id == id)
}

/// Expands an entry's templates against this OS and keeps the folders that exist,
/// deduplicated and in order. If none exists but some template expands to a
/// concrete path, it returns that single best guess so the dialog has something to
/// show (the user can correct it before adding).
pub fn resolve_save_paths(def: &EmulatorDef) -> Vec<String> {
    let os = Os::current();
    let mut existing: Vec<String> = Vec::new();
    let mut first_guess: Option<String> = None;
    for tmpl in def.save_templates {
        for path in expand_path(tmpl, os) {
            let s = path.to_string_lossy().into_owned();
            if first_guess.is_none() {
                first_guess = Some(s.clone());
            }
            if path.is_dir() && !existing.contains(&s) {
                existing.push(s);
            }
        }
    }
    if existing.is_empty() {
        first_guess.into_iter().collect()
    } else {
        existing
    }
}

// ─── Instalaciones portables ────────────────────────────────────────────────

/// Folders where a portable install keeps what an installed one would put in the
/// user folder. `""` is the install's own root.
const PORTABLE_USER_DIRS: &[&str] = &["", "user"];

/// Splits a template anchored in the user folder into (the app's folder, the tail
/// below it). Returns `None` for templates with no tail, where the app's folder IS
/// the save folder and there is nothing to re-anchor, and for those that do not
/// come off a re-anchorable root (Documents, Saved Games, a wrapper root).
fn app_dir_and_tail(template: &str) -> Option<(&str, &str)> {
    // Only the "application" roots: a Documents template has no portable
    // equivalent, and `<home>` is too wide to infer anything from.
    const REANCHORABLE: &[&str] = &["<winAppData>/", "<xdgData>/", "<xdgConfig>/"];
    let rest = REANCHORABLE.iter().find_map(|p| template.strip_prefix(p))?;
    let (app_dir, tail) = rest.split_once('/')?;
    if app_dir.is_empty() || tail.is_empty() {
        return None;
    }
    Some((app_dir, tail))
}

/// Does this directory's name plausibly name an install of `app_dir`? An exact
/// match, or the name with a suffix: builds that get unpacked arrive as
/// `RetroArch-Win64` or `Azahar-2120` far more often than under the bare name.
fn looks_like_install_of(dir_name: &str, app_dir: &str) -> bool {
    let d = dir_name.to_lowercase();
    let a = app_dir.to_lowercase();
    if d == a {
        return true;
    }
    let Some(rest) = d.strip_prefix(&a) else {
        return false;
    };
    // Demand a separator after the name so "eden" does not match "edenring".
    matches!(
        rest.as_bytes().first(),
        Some(b'-' | b'_' | b' ' | b'.' | b'0'..=b'9')
    )
}

/// This emulator's save folders when it has been unpacked somewhere rather than
/// installed.
///
/// The catalogue locates each emulator by its per-user data folder
/// (`%APPDATA%\RetroArch` and friends), which is where an installer leaves it, and
/// that is on C: wherever the executable lives. Except that a great many people do
/// not install: RetroArch, the Citra line and the yuzu line all ship as a folder
/// you unpack wherever you like, and in that mode they keep their data next to the
/// executable. Somebody with their emulators in `D:\Emulators` has no
/// `%APPDATA%\RetroArch` at all, the scan looks in the one place they are not, and
/// the app looks broken for exactly the audience with the most emulators.
///
/// A portable install's internal layout is the same as the data folder's, only
/// hanging off somewhere else: either directly under the root (RetroArch's
/// `saves/`) or under a `user/` next to the executable (the Citra and yuzu lines).
/// So each template's tail is reused, and to accept a candidate both things are
/// demanded: that the folder be named after the emulator, and that the tail really
/// exist. The tail alone proves nothing, since there are plenty of game folders
/// with something called `saves` inside.
///
/// Bounded on purpose: one listing per drive plus one per collection folder, with
/// nothing walked. A full sweep of a games disk would read tens of thousands of
/// directories to find a handful of hits.
pub fn portable_save_paths(def: &EmulatorDef) -> Vec<PathBuf> {
    let tails: Vec<(&str, &str)> = def
        .save_templates
        .iter()
        .filter_map(|t| app_dir_and_tail(t))
        .collect();
    if tails.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for root in crate::roots::portable_install_roots(Os::current()) {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let dir = entry.path();
            let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            for (app_dir, tail) in &tails {
                if !looks_like_install_of(name, app_dir) {
                    continue;
                }
                for user_dir in PORTABLE_USER_DIRS {
                    let candidate = if user_dir.is_empty() {
                        dir.join(tail)
                    } else {
                        dir.join(user_dir).join(tail)
                    };
                    if candidate.is_dir() && seen.insert(candidate.clone()) {
                        out.push(candidate);
                    }
                }
            }
        }
    }
    out
}

// ─── Partición por título ───────────────────────────────────────────────────

/// One game's save folder inside a console's tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleSave {
    /// The title's identifier exactly as the folder names it (16 hex on Switch,
    /// `<hi>/<lo>` on 3DS). It is the only thing both installs call the same.
    pub title_id: String,
    pub path: PathBuf,
}

/// Is this name a Switch title id? Sixteen hexadecimal digits. Matching by shape
/// avoids offering as games the backups and working directories the emulator
/// leaves alongside.
fn is_switch_title_id(name: &str) -> bool {
    name.len() == 16 && name.chars().all(|c| c.is_ascii_hexdigit())
}

/// Does this folder hold anything? The emulator creates one for every title ever
/// launched, even when nothing was ever saved.
fn has_any_file(dir: &Path) -> bool {
    let mut stack = vec![dir.to_path_buf()];
    let mut budget = 64; // suficiente para decidir; no es un recorrido completo
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            budget -= 1;
            if budget < 0 {
                return false;
            }
            match entry.file_type() {
                Ok(t) if t.is_file() => return true,
                Ok(t) if t.is_dir() => stack.push(entry.path()),
                _ => {}
            }
        }
    }
    false
}

/// Descends from a console's save root to each game's folder.
///
/// Offering the whole root as a single save folder puts an identifier that is
/// generated per install inside the synced tree: the profile uuid in the yuzu
/// line, the `id0`/`id1` pair derived from console keys in Citra. The copy that
/// reaches the other machine then hangs off a profile its emulator has never seen,
/// and the emulator reports a save with no associated profile. Nothing in that
/// message points at syncing, which is why it reads as the emulator being broken
/// or misconfigured.
///
/// The title's folder is the part both installs do agree on, so that is what gets
/// offered, one per game.
///
/// The fallback is the important part. Forks and versions vary, and a layout guess
/// that misses leaves the user with no detection at all, which is worse than the
/// identifier problem. So a shape that is not recognised falls back to offering
/// the root as-is, and only a tree that fits completely gets split per title.
pub fn split_per_title(root: &Path, layout: TitleLayout) -> Vec<TitleSave> {
    match layout {
        TitleLayout::SwitchNand => split_switch_nand(root),
        TitleLayout::Citra3ds => split_citra_sdmc(root),
    }
}

/// `<root>/<account>/<profile-uuid>/<title-id>/`. Two opaque levels and then the
/// title; trees with only one intermediate level are also accepted, which is how
/// some builds end up.
fn split_switch_nand(root: &Path) -> Vec<TitleSave> {
    let mut out = Vec::new();
    for level1 in read_dirs(root) {
        for level2 in read_dirs(&level1) {
            let Some(name) = level2.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if is_switch_title_id(name) && has_any_file(&level2) {
                out.push(TitleSave {
                    title_id: name.to_string(),
                    path: level2.clone(),
                });
                continue;
            }
            // Un nivel más abajo: <cuenta>/<perfil>/<title-id>.
            for level3 in read_dirs(&level2) {
                let Some(name) = level3.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if is_switch_title_id(name) && has_any_file(&level3) {
                    out.push(TitleSave {
                        title_id: name.to_string(),
                        path: level3,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

/// `<sdmc>/Nintendo 3DS/<id0>/<id1>/title/<hi>/<lo>/data/`. The save lives in
/// `data`; that folder is offered and the title is named `<hi><lo>`.
fn split_citra_sdmc(root: &Path) -> Vec<TitleSave> {
    let mut out = Vec::new();
    let base = if root.join("Nintendo 3DS").is_dir() {
        root.join("Nintendo 3DS")
    } else {
        root.to_path_buf()
    };
    for id0 in read_dirs(&base) {
        for id1 in read_dirs(&id0) {
            let titles = id1.join("title");
            if !titles.is_dir() {
                continue;
            }
            for hi in read_dirs(&titles) {
                for lo in read_dirs(&hi) {
                    let data = lo.join("data");
                    if !data.is_dir() || !has_any_file(&data) {
                        continue;
                    }
                    let hi_name = hi.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    let lo_name = lo.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    out.push(TitleSave {
                        title_id: format!("{hi_name}{lo_name}"),
                        path: data,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

// ---- an emulator's root, as detection sees it

/// The emulator whose save root this path **is**, if any.
///
/// The catalog can't answer this by comparing expanded paths: the roots that
/// reach detection are the ones the templates missed. rpcs3 on macOS lives
/// under `~/Library/Application Support`, and under RetroDECK it lives inside
/// `~/retrodeck` — neither is `<xdgConfig>`, and both are still rpcs3.
///
/// So the match is on the **tail** of the template, which is the part that
/// belongs to the emulator instead of to the host: `rpcs3/dev_hdd0/home/
/// <profile>/savedata` identifies rpcs3 wherever the tree was rooted. A
/// numeric template segment matches any numeric segment of the same width,
/// because those are per-install: `00000001` is only the *first* rpcs3
/// profile, and the account of someone on their second is `00000002`.
pub fn save_root_at(path: &Path) -> Option<&'static EmulatorDef> {
    CATALOG.iter().find(|def| {
        def.save_templates
            .iter()
            .filter_map(|t| template_tail(t))
            .any(|tail| path_ends_with_tail(path, tail))
    })
}

/// The emulator root above `path`, along with the title folder it was entered
/// through.
///
/// The sweep does not always land on the root: it descends to where the files are,
/// so what it brings back is `.../savedata/BLUS30443` or something deeper still.
/// Without this, that folder is attributed on its own and comes back named after
/// the emulator's tree rather than after the emulator.
///
/// `None` when `path` hangs off no known root, or when it IS the root, which is
/// what [`save_root_at`] is for; returning both through the same place would make
/// the caller break the tie.
pub fn save_root_above(path: &Path) -> Option<(&'static EmulatorDef, PathBuf)> {
    let mut title = path;
    while let Some(parent) = title.parent() {
        if let Some(def) = save_root_at(parent) {
            return Some((def, title.to_path_buf()));
        }
        title = parent;
    }
    None
}

/// The part of a template below its root placeholder.
fn template_tail(template: &str) -> Option<&str> {
    let (_, tail) = template.strip_prefix('<')?.split_once(">/")?;
    Some(tail).filter(|t| !t.is_empty())
}

/// Does `path` end in this template tail? Compared by component and
/// case-insensitively (macOS and Windows do not distinguish case, and the
/// templates are written in each emulator project's own casing).
fn path_ends_with_tail(path: &Path, tail: &str) -> bool {
    let want: Vec<&str> = tail.split('/').filter(|s| !s.is_empty()).collect();
    let have: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    if want.is_empty() || have.len() < want.len() {
        return false;
    }
    have[have.len() - want.len()..]
        .iter()
        .zip(&want)
        .all(|(h, w)| segment_matches(h, w))
}

/// One template segment against a real one. Account and profile identifiers are
/// compared by SHAPE rather than by value: see [`save_root_at`].
fn segment_matches(have: &str, want: &str) -> bool {
    if want.chars().all(|c| c.is_ascii_digit()) {
        return have.len() == want.len() && have.chars().all(|c| c.is_ascii_digit());
    }
    have.eq_ignore_ascii_case(want)
}

/// The per-title folders inside `def`'s save root.
///
/// Empty means "this root cannot be split", and that is a legitimate answer: a
/// freshly created root has no titles in it yet. The caller decides what to do
/// with that, but what it may NOT do is offer the whole root as though it were a
/// save; see [`split_per_title`].
pub fn titles_in(def: &EmulatorDef, root: &Path) -> Vec<TitleSave> {
    if let Some(layout) = def.title_layout {
        return split_per_title(root, layout);
    }
    // Sin distribución conocida, la forma genérica: una carpeta por título y
    // nada suelto en la raíz. El `has_direct_file` es la línea que separa un
    // contenedor de un save de verdad — RetroArch deja sus `.srm` sueltos en
    // `saves/`, así que esa carpeta ES el save y no hay nada que partir.
    if has_direct_file(root) {
        return Vec::new();
    }
    let mut out: Vec<TitleSave> = Vec::new();
    for dir in read_dirs(root) {
        if !has_any_file(&dir) {
            continue;
        }
        let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        out.push(TitleSave {
            title_id: name.to_string(),
            path: dir.clone(),
        });
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out
}

/// `true` si la raíz tiene ficheros **suyos**, sin bajar. Distingue el save
/// plano (RetroArch) del contenedor de carpetas por título (rpcs3).
pub fn has_direct_file(dir: &Path) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    read.flatten()
        .any(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
}

/// Subdirectorios inmediatos de `dir`, ordenados. Vacío si no se puede leer:
/// aquí un error sólo significa "no hay nada que ofrecer por debajo".
fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Las raíces que producción rastreó como si fueran un save, cada una con
    /// el emulador que debería haberlas reclamado. Las rutas son las de los
    /// casos vistos: rpcs3 en macOS y en RetroDECK (ninguna de las dos es la
    /// que expande la plantilla), RetroArch, Ryujinx, Dolphin y Yuzu.
    #[test]
    fn an_emulator_save_root_is_recognised_wherever_it_was_installed() {
        let cases: &[(&str, Option<&str>)] = &[
            // rpcs3, el de las 224 líneas de "nothing to back up". El slug que
            // salía de aquí era `dev-hdd0`.
            (
                "/Users/u/Library/Application Support/rpcs3/dev_hdd0/home/00000001/savedata",
                Some("rpcs3"),
            ),
            // El mismo árbol dentro de RetroDECK.
            (
                "/home/u/retrodeck/saves/ps3/rpcs3/dev_hdd0/home/00000001/savedata",
                Some("rpcs3"),
            ),
            // Y con el perfil que NO es el primero: el id es por instalación,
            // así que se compara la forma, no el valor.
            (
                "/home/u/.config/rpcs3/dev_hdd0/home/00000002/savedata",
                Some("rpcs3"),
            ),
            (
                "/home/u/.var/app/org.libretro.RetroArch/config/retroarch/saves",
                Some("retroarch"),
            ),
            ("/home/u/.config/retroarch/saves", Some("retroarch")),
            (
                "/home/u/.local/share/Ryujinx/bis/user/save",
                Some("ryujinx"),
            ),
            ("/home/u/.local/share/dolphin-emu/GC", Some("dolphin")),
            ("/home/u/.local/share/dolphin-emu/Wii", Some("dolphin")),
            ("/home/u/.local/share/yuzu/nand/user/save", Some("yuzu")),
            ("/home/u/.local/share/Cemu/mlc01/usr/save", Some("cemu")),
            // Y lo que NO puede reclamar ningún emulador: la carpeta de UN
            // título dentro de la raíz, y un save cualquiera de un juego.
            (
                "/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443",
                None,
            ),
            (
                "/home/u/.local/share/Steam/steamapps/common/Stellaris",
                None,
            ),
            ("/home/u/Documents/My Games/Skyrim/Saves", None),
        ];
        for (raw, expected) in cases {
            let got = save_root_at(Path::new(raw)).map(|d| d.id);
            assert_eq!(got, *expected, "{raw}");
        }
    }

    /// Y desde dentro se llega a la raíz por arriba, que es como el barrido la
    /// encuentra de verdad: baja hasta donde hay ficheros, no para en la raíz.
    #[test]
    fn a_folder_inside_a_save_root_finds_the_root_above_it() {
        let deep = Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443/sub");
        let (def, title) = save_root_above(deep).expect("cuelga de la raíz de rpcs3");
        assert_eq!(def.id, "rpcs3");
        // La carpeta del TÍTULO, no la hoja donde aterrizó el barrido.
        assert_eq!(
            title,
            Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata/BLUS30443")
        );

        // La raíz misma no cuelga de sí misma: eso lo contesta `save_root_at`.
        let root = Path::new("/home/u/.config/rpcs3/dev_hdd0/home/00000001/savedata");
        assert!(save_root_above(root).is_none());
    }

    /// Partir la raíz: una fila por título cuando hay títulos, y **nada**
    /// cuando la raíz está vacía —que es el caso de las 224 líneas: rpcs3
    /// instalado y el `savedata` del primer perfil sin estrenar.
    #[test]
    fn a_container_root_splits_per_title_and_an_empty_one_offers_nothing() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("savedata");
        fs::create_dir_all(&root).unwrap();
        let rpcs3 = find("rpcs3").unwrap();

        // Vacía: no hay título que ofrecer, y la raíz NO vale como save.
        assert!(titles_in(rpcs3, &root).is_empty());
        assert!(!has_direct_file(&root));

        // Con dos títulos dentro, uno por fila.
        for title in ["BLUS30443-AUTOSAVE", "NPUB30493-SAVEDATA01"] {
            let d = root.join(title);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("PARAM.SFO"), b"x").unwrap();
        }
        // Y una carpeta vacía, que no es un título: no hay nada que respaldar.
        fs::create_dir_all(root.join("EMPTY00000")).unwrap();

        let titles = titles_in(rpcs3, &root);
        assert_eq!(
            titles
                .iter()
                .map(|t| t.title_id.as_str())
                .collect::<Vec<_>>(),
            ["BLUS30443-AUTOSAVE", "NPUB30493-SAVEDATA01"]
        );
    }

    /// La otra mitad: una raíz con los ficheros sueltos dentro NO es un
    /// contenedor. RetroArch deja sus `.srm` en `saves/`, así que esa carpeta
    /// ES el save y partirla la destrozaría.
    #[test]
    fn a_flat_save_root_is_not_a_container() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("saves");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Chrono Trigger.srm"), b"x").unwrap();
        let retroarch = find("retroarch").unwrap();

        assert!(has_direct_file(&root));
        assert!(
            titles_in(retroarch, &root).is_empty(),
            "no hay títulos que partir: los saves son los ficheros"
        );
    }

    #[test]
    fn every_catalog_id_is_unique() {
        let mut seen = HashSet::new();
        for def in CATALOG {
            assert!(seen.insert(def.id), "id duplicado: {}", def.id);
        }
    }

    #[test]
    fn install_names_match_with_a_suffix_but_not_a_longer_word() {
        assert!(looks_like_install_of("RetroArch", "RetroArch"));
        assert!(looks_like_install_of("retroarch-win64", "RetroArch"));
        assert!(looks_like_install_of("Azahar-2120", "Azahar"));
        assert!(looks_like_install_of("eden 0.1", "eden"));
        // El caso que obliga al separador: "edenring" no es una build de Eden.
        assert!(!looks_like_install_of("edenring", "eden"));
        assert!(!looks_like_install_of("Elden Ring", "eden"));
    }

    #[test]
    fn only_app_rooted_templates_have_a_portable_equivalent() {
        assert_eq!(
            app_dir_and_tail("<winAppData>/RetroArch/saves"),
            Some(("RetroArch", "saves"))
        );
        assert_eq!(
            app_dir_and_tail("<xdgData>/citra-emu/sdmc"),
            Some(("citra-emu", "sdmc"))
        );
        // Sin cola: la carpeta de la app ES la de saves.
        assert_eq!(app_dir_and_tail("<winAppData>/RetroArch"), None);
        // Documentos y Saved Games no se reanclan.
        assert_eq!(app_dir_and_tail("<winDocuments>/PCSX2/memcards"), None);
        assert_eq!(app_dir_and_tail("<home>/.config/retroarch/saves"), None);
    }

    #[test]
    fn portable_only_emulators_keep_a_reanchorable_template() {
        // Flycast and Project64 write next to their executable on Windows, so
        // neither has a correct `%APPDATA%` path to offer. Detection there
        // runs entirely through `portable_save_paths`, which needs some
        // template with an app-rooted shape to borrow the folder name and
        // tail from. Drop the last one and Windows detection silently goes to
        // zero, which is why these two rows cannot be trimmed to nothing.
        for id in ["flycast", "project64"] {
            let def = find(id).unwrap();
            assert!(
                def.save_templates
                    .iter()
                    .any(|t| app_dir_and_tail(t).is_some()),
                "{id} lost the template that feeds portable detection"
            );
        }
    }

    #[test]
    fn a_switch_nand_tree_splits_into_one_entry_per_title() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let profile = root.join("0000000000000000/78b4e1c9a0f24d3b8e5f6a7c9d0e1f2a");
        for title in ["0100152000022000", "01007ef00011e000"] {
            let t = profile.join(title);
            fs::create_dir_all(&t).unwrap();
            fs::write(t.join("save.dat"), b"x").unwrap();
        }
        // Vacía: el emulador la crea por cada título lanzado, no es un save.
        fs::create_dir_all(profile.join("0100abcd00099000")).unwrap();
        // Ni carpeta de trabajo ni copia: la forma no es un id de título.
        fs::create_dir_all(profile.join("backup")).unwrap();

        let found = split_per_title(root, TitleLayout::SwitchNand);
        let ids: Vec<&str> = found.iter().map(|t| t.title_id.as_str()).collect();
        assert_eq!(ids, vec!["0100152000022000", "01007ef00011e000"]);
    }

    #[test]
    fn an_unrecognised_shape_yields_nothing_so_the_caller_keeps_the_root() {
        // La forma que obliga al fallback: save/0000/save.bin, un solo nivel.
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("0000")).unwrap();
        fs::write(root.join("0000/save.bin"), b"x").unwrap();

        assert!(split_per_title(root, TitleLayout::SwitchNand).is_empty());
    }

    #[test]
    fn a_citra_sdmc_tree_splits_at_the_data_folder() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let data = root
            .join("Nintendo 3DS")
            .join("00000000000000000000000000000000")
            .join("11111111111111111111111111111111")
            .join("title/00040000/00055d00/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("00000001.sav"), b"x").unwrap();

        let found = split_per_title(root, TitleLayout::Citra3ds);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title_id, "0004000000055d00");
        assert_eq!(found[0].path, data);
    }

    #[test]
    fn an_empty_title_folder_is_not_offered() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let t = root.join("0000000000000000/78b4e1c9a0f24d3b8e5f6a7c9d0e1f2a/0100152000022000");
        fs::create_dir_all(&t).unwrap();

        assert!(split_per_title(root, TitleLayout::SwitchNand).is_empty());
    }
}
