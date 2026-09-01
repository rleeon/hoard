//! The Steam playtime index: a map from install folder to slug for the whole
//! Steam library, for the recap's "only what you play" model.
//!
//! The curated [`crate::playtime_catalog`] covers recognisable online games with
//! known process names. But the user wants ANY Steam game they play to count in
//! the Wrapped, even one with no save to copy and no entry in the Ludusavi
//! catalogue, such as an online game with no local save. Nothing gets enrolled
//! and the "played, not backed up" list is untouched: the process poll
//! ([`crate::agent`]) checks whether the live executable falls under one of this
//! index's `steamapps/common/<game>` folders and, if so, attributes the tick's
//! time to it. The slug comes from [`ludusavi::slugify`], the same one detection
//! uses, so the UI shows the pretty name derived from the slug with no extra
//! wiring.
//!
//! It is rebuilt from disk on a TTL ([`REFRESH_TTL`]): reading the
//! `appmanifest_*.acf` files every 2 s would be a waste and the library rarely
//! changes.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use hoard_manifest::ludusavi;

use crate::manifest::Os;
use crate::steam;

/// How often the index is rebuilt by reading Steam's appmanifests.
pub const REFRESH_TTL: Duration = Duration::from_secs(300);

/// Lowercase markers for Steam "apps" that are really tools rather than games:
/// they live under `steamapps/common` like any game and run CPU-heavy processes
/// when a game launches, so without this filter they would add phantom hours to
/// the Wrapped.
const STEAM_TOOL_MARKERS: &[&str] = &[
    "proton",
    "steam linux runtime",
    "steamworks common redistributables",
    "steamvr",
    "steam controller",
    "steam runtime",
];

/// A map from lowercase install folder to slug for the installed Steam library,
/// with a timestamp for the lazy refresh.
#[derive(Default)]
pub struct SteamPlaytimeIndex {
    entries: Vec<(PathBuf, String)>,
    refreshed_at: Option<Instant>,
}

impl SteamPlaytimeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds when it was never loaded or the TTL expired. Cheap in the steady
    /// state (one instant comparison).
    pub fn refresh_if_stale(&mut self) {
        let stale = self
            .refreshed_at
            .map(|t| t.elapsed() >= REFRESH_TTL)
            .unwrap_or(true);
        if stale {
            let apps = steam::list_installed_steam_games(Os::current()).unwrap_or_default();
            self.entries = build_entries(apps);
            self.refreshed_at = Some(Instant::now());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The slug of the game whose install folder contains `exe`, if any.
    /// Compared by path component rather than by string prefix, and in lowercase,
    /// so ".../common/Portal" does not capture ".../common/Portal 2" and Windows
    /// casing does not break the match.
    pub fn slug_for_exe(&self, exe: &Path) -> Option<&str> {
        let exe_lower = lower_path(exe);
        self.entries
            .iter()
            .find(|(dir, _)| exe_lower.starts_with(dir))
            .map(|(_, slug)| slug.as_str())
    }
}

/// Turns installed games into `(lowercase folder, slug)` pairs, dropping tools
/// and names that slugify to nothing.
fn build_entries(apps: Vec<steam::SteamApp>) -> Vec<(PathBuf, String)> {
    apps.into_iter()
        .filter(|a| !is_steam_tool(&a.name))
        .filter_map(|a| {
            let slug = ludusavi::slugify(&a.name);
            if slug.is_empty() {
                return None;
            }
            Some((lower_path(&a.install_dir), slug))
        })
        .collect()
}

fn is_steam_tool(name: &str) -> bool {
    let lower = name.to_lowercase();
    STEAM_TOOL_MARKERS.iter().any(|m| lower.contains(m))
}

/// The path in lowercase, keeping component separators so `Path::starts_with`
/// still compares component by component.
fn lower_path(p: &Path) -> PathBuf {
    PathBuf::from(p.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(entries: &[(&str, &str)]) -> SteamPlaytimeIndex {
        SteamPlaytimeIndex {
            entries: entries
                .iter()
                .map(|(d, s)| (lower_path(Path::new(d)), s.to_string()))
                .collect(),
            refreshed_at: Some(Instant::now()),
        }
    }

    #[test]
    fn matches_exe_under_install_dir_case_insensitively() {
        let idx = index(&[("C:/Steam/steamapps/common/War Selection", "war-selection")]);
        assert_eq!(
            idx.slug_for_exe(Path::new(
                "c:/steam/steamapps/common/war selection/GlyphEngine.exe"
            )),
            Some("war-selection")
        );
    }

    #[test]
    fn does_not_match_sibling_prefix() {
        // "Portal" must not capture an exe from "Portal 2".
        let idx = index(&[("C:/Steam/steamapps/common/Portal", "portal")]);
        assert_eq!(
            idx.slug_for_exe(Path::new("C:/Steam/steamapps/common/Portal 2/portal2.exe")),
            None
        );
    }

    #[test]
    fn no_match_outside_library() {
        let idx = index(&[("C:/Steam/steamapps/common/Rust", "rust")]);
        assert_eq!(
            idx.slug_for_exe(Path::new("C:/Windows/System32/notepad.exe")),
            None
        );
    }

    #[test]
    fn build_entries_drops_tools_and_slugifies() {
        let apps = vec![
            steam::SteamApp {
                app_id: 1022450,
                name: "War Selection".into(),
                install_dir: "/lib/common/War Selection".into(),
            },
            steam::SteamApp {
                app_id: 1420170,
                name: "Proton 9.0".into(),
                install_dir: "/lib/common/Proton 9.0".into(),
            },
            steam::SteamApp {
                app_id: 228980,
                name: "Steamworks Common Redistributables".into(),
                install_dir: "/lib/common/Steamworks Shared".into(),
            },
        ];
        let entries = build_entries(apps);
        assert_eq!(entries.len(), 1, "solo el juego real sobrevive");
        assert_eq!(entries[0].1, "war-selection");
    }
}
