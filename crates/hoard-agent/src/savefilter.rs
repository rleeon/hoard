//! Which game this folder belongs to, and therefore which files inside it are
//! save data.
//!
//! The *what* is decided by [`hoard_core::kernel::fileclass`], which is pure.
//! What lives here is the part that needs IO and the catalogue: pulling the
//! game's file patterns out of the Ludusavi manifest to hand them over as
//! shields.
//!
//! ## Where the patterns come from
//!
//! 20,499 of the catalogue's 47,404 templates end in a file pattern
//! (`<base>/Saves/*.sav`, `<base>/SavesDir/*.sav`). Hoard already had them in
//! front of it and threw them away: `pathexpand::expand_path_globbed` collapses
//! the pattern to its parent folder and returns only the directory, because what
//! Hoard tracks is the folder. The pattern was lost there, and with it the only
//! reliable source for "what counts as save data in this particular folder".
//!
//! It is recovered here, by slug and against the templates of all three systems:
//! a Windows game running under Proton lives in a Windows-shaped folder, so
//! looking only at the host OS would drop half of them. Since the patterns only
//! ever rescue and never exclude, the superset is the safe choice.
//!
//! A hand-added save, or one for a game outside the catalogue, gets no shields:
//! the kernel's name rules decide alone, which is why they are conservative.

use hoard_core::kernel::fileclass::is_useful_shield;

/// The filename patterns the manifest declares as save data for `slug`, in
/// lowercase and deduplicated.
///
/// Only the template's last segment counts, and only when it is a wildcard: a
/// literal segment (`.../Fallout4/Saves`) is the name of the tracked folder
/// rather than a file pattern, and taking it for one would shield a name that
/// does not exist inside.
pub fn shields_for_slug(slug: &str) -> Vec<String> {
    let Some(entry) = hoard_manifest::ludusavi::find_by_slug(slug) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let all = entry
        .paths
        .windows
        .iter()
        .chain(entry.paths.linux.iter())
        .chain(entry.paths.mac.iter());
    for p in all {
        let Some(last) = p.path.rsplit('/').next() else {
            continue;
        };
        // A literal is not a file pattern: it is the folder being tracked.
        if !last.contains('*') && !last.contains('?') {
            continue;
        }
        if !is_useful_shield(last) {
            continue;
        }
        let lower = last.to_ascii_lowercase();
        if !out.contains(&lower) {
            out.push(lower);
        }
    }
    out
}

/// Where the manifest says `slug` saves INSIDE its own installation: the tail of
/// every `<base>/...` template, lowercased (`savedata.dat`, `data/hk*slot*`,
/// `snappdata/savedgames`). Empty for a game that saves anywhere else.
///
/// What a backup always keeps when the tracked folder turns out to be the
/// install, whatever it looks like: Galak-Z's catalogue entry is
/// `<base>/SaveData.dat`, detection tracks the folder around it, and the whole
/// game (1 GB) used to go up while the save did not fit under the free plan's
/// cap.
pub fn install_save_patterns(slug: &str) -> Vec<String> {
    let Some(entry) = hoard_manifest::ludusavi::find_by_slug(slug) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let all = entry
        .paths
        .windows
        .iter()
        .chain(entry.paths.linux.iter())
        .chain(entry.paths.mac.iter());
    for p in all {
        let Some(tail) = p.path.strip_prefix("<base>/") else {
            continue;
        };
        let tail = tail.trim_matches('/').to_ascii_lowercase();
        // A tail that is nothing but wildcards would keep the whole install.
        if tail.is_empty()
            || tail
                .split('/')
                .all(|seg| seg.chars().all(|c| c == '*' || c == '?'))
        {
            continue;
        }
        if !out.contains(&tail) {
            out.push(tail);
        }
    }
    out
}

/// Does `rel` (a path inside the tracked folder) fall under one of `patterns`?
/// A pattern may name a file (`savedata.dat`) or a folder (`snappdata/savedgames`,
/// everything below it counts), and it may match at any depth, because the
/// tracked folder is sometimes one level above the install (a repack's folder
/// holding the game next to `_CommonRedist`).
pub fn matches_install_pattern(rel: &str, patterns: &[String]) -> bool {
    let lower = rel.replace('\\', "/").to_lowercase();
    let parts: Vec<&str> = lower.split('/').filter(|p| !p.is_empty()).collect();
    patterns.iter().any(|pattern| {
        let want: Vec<&str> = pattern.split('/').filter(|p| !p.is_empty()).collect();
        if want.is_empty() || want.len() > parts.len() {
            return false;
        }
        (0..=parts.len() - want.len()).any(|start| {
            want.iter()
                .zip(&parts[start..])
                .all(|(w, p)| hoard_core::kernel::fileclass::glob_match(w, p))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_patterns_come_from_base_templates() {
        // Galak-Z: `<base>/SaveData.dat`.
        assert_eq!(
            install_save_patterns("galak-z-the-dimensional"),
            vec!["savedata.dat".to_string()]
        );
        // Fallout 4 saves in Documents: nothing inside the install.
        assert!(install_save_patterns("fallout-4").is_empty());
    }

    #[test]
    fn install_patterns_match_files_and_folders_at_any_depth() {
        let p = vec![
            "savedata.dat".to_string(),
            "data/hk*slot*".to_string(),
            "snappdata/savedgames".to_string(),
        ];
        assert!(matches_install_pattern("SaveData.dat", &p));
        assert!(matches_install_pattern("Sleeping Dogs/Data/HK_Slot1", &p));
        assert!(matches_install_pattern(
            "SNAppData/SavedGames/slot0000/gameinfo.json",
            &p
        ));
        assert!(!matches_install_pattern(
            "Galak-Z_Data/sharedassets4.assets",
            &p
        ));
        assert!(!matches_install_pattern("Data/level1.pak", &p));
    }

    #[test]
    fn a_game_with_file_patterns_yields_them() {
        // Terraria: `<root>/userdata/<storeUserId>/105600/remote/players/*.plr`
        // y `.../worlds/*.wld`.
        let shields = shields_for_slug("terraria");
        assert!(shields.contains(&"*.plr".to_string()), "{shields:?}");
        assert!(shields.contains(&"*.wld".to_string()), "{shields:?}");
    }

    #[test]
    fn a_bare_directory_template_yields_no_shield() {
        // Fallout 4 is `<winDocuments>/My Games/Fallout4/Saves`: the last
        // segment is the folder, not a pattern.
        assert!(shields_for_slug("fallout-4").is_empty());
        // And the game that motivated all of this has none either.
        assert!(shields_for_slug("cell-to-singularity-evolution-never-ends").is_empty());
    }

    #[test]
    fn an_unknown_slug_is_not_an_error() {
        assert!(shields_for_slug("no-existe-este-juego-12345").is_empty());
    }

    /// The Windows patterns count even when we run on Linux: under Proton the
    /// folder is Windows-shaped.
    #[test]
    fn windows_patterns_count_on_every_host() {
        // `<base>/SavesDir/*.sav`, declared only in `paths.windows`.
        let shields = shields_for_slug("singularity-tactics-arena");
        assert!(shields.contains(&"*.sav".to_string()), "{shields:?}");
    }

    /// `*.*` matches everything: it would shield the whole folder and void the
    /// filter.
    #[test]
    fn degenerate_patterns_never_become_shields() {
        for e in hoard_manifest::ludusavi::catalog().iter().take(4000) {
            for s in shields_for_slug(&e.slug) {
                assert!(is_useful_shield(&s), "{} shielded with {s}", e.slug);
            }
        }
    }
}
