//! The playtime catalogue: online games, or games with no local save worth
//! copying, whose play time counts toward the recap but of which we store
//! nothing. The agent's process poll matches them by executable name (exact,
//! case-insensitive), so they count as soon as their binary runs, from whichever
//! launcher.
//!
//! This is the time-only counterpart to save detection: these games get enrolled
//! as [`crate::agent::WatchedSave`] with `track_only = true`. The list is curated
//! on purpose, holding recognisable games that make sense in a Wrapped; widening
//! it means adding a row here.

/// A game we track for play time only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaytimeGame {
    /// Stable slug: the key for hour attribution and for the exclusion list.
    pub slug: &'static str,
    /// Human-readable name for the UI and the recap.
    pub display_name: &'static str,
    /// Lowercase executable names the process poll matches. Windows and native
    /// variants are both covered, because the match is exact.
    pub processes: &'static [&'static str],
}

/// The curated catalogue. Process names in lowercase (the match lowercases both
/// sides, but they are stored this way for a direct lookup).
pub const PLAYTIME_CATALOG: &[PlaytimeGame] = &[
    PlaytimeGame {
        slug: "fortnite",
        display_name: "Fortnite",
        processes: &["fortniteclient-win64-shipping.exe"],
    },
    PlaytimeGame {
        slug: "rust",
        display_name: "Rust",
        processes: &["rustclient.exe", "rust.exe", "rustclient"],
    },
    PlaytimeGame {
        slug: "valorant",
        display_name: "VALORANT",
        processes: &["valorant-win64-shipping.exe"],
    },
    PlaytimeGame {
        slug: "league-of-legends",
        display_name: "League of Legends",
        processes: &["league of legends.exe"],
    },
    PlaytimeGame {
        slug: "counter-strike-2",
        display_name: "Counter-Strike 2",
        processes: &["cs2.exe", "cs2"],
    },
    PlaytimeGame {
        slug: "dota-2",
        display_name: "Dota 2",
        processes: &["dota2.exe", "dota2"],
    },
    PlaytimeGame {
        slug: "apex-legends",
        display_name: "Apex Legends",
        processes: &["r5apex.exe", "r5apex_dx12.exe"],
    },
    PlaytimeGame {
        slug: "overwatch-2",
        display_name: "Overwatch 2",
        processes: &["overwatch.exe"],
    },
    PlaytimeGame {
        slug: "grand-theft-auto-v",
        display_name: "Grand Theft Auto V",
        processes: &["gta5.exe", "gta5_enhanced.exe"],
    },
    PlaytimeGame {
        slug: "destiny-2",
        display_name: "Destiny 2",
        processes: &["destiny2.exe"],
    },
    PlaytimeGame {
        slug: "warframe",
        display_name: "Warframe",
        processes: &["warframe.x64.exe", "warframe.exe"],
    },
    PlaytimeGame {
        slug: "rocket-league",
        display_name: "Rocket League",
        processes: &["rocketleague.exe"],
    },
    PlaytimeGame {
        slug: "world-of-warcraft",
        display_name: "World of Warcraft",
        processes: &["wow.exe", "wowclassic.exe"],
    },
    PlaytimeGame {
        slug: "roblox",
        display_name: "Roblox",
        processes: &["robloxplayerbeta.exe"],
    },
    PlaytimeGame {
        slug: "pubg-battlegrounds",
        display_name: "PUBG: BATTLEGROUNDS",
        processes: &["tslgame.exe"],
    },
    PlaytimeGame {
        slug: "genshin-impact",
        display_name: "Genshin Impact",
        processes: &["genshinimpact.exe", "yuanshen.exe"],
    },
];

/// The catalogue entry whose `slug` matches.
pub fn by_slug(slug: &str) -> Option<&'static PlaytimeGame> {
    PLAYTIME_CATALOG.iter().find(|g| g.slug == slug)
}

/// The catalogue entry that declares `proc_name` (compared in lowercase) as one
/// of its executables. Lets an online game be identified by its live process even
/// when the installed-games scan never saw it.
pub fn game_for_process(proc_name: &str) -> Option<&'static PlaytimeGame> {
    let lower = proc_name.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    PLAYTIME_CATALOG
        .iter()
        .find(|g| g.processes.iter().any(|p| *p == lower))
}

/// The catalogue entry whose readable name matches `name` once normalised
/// (lowercase, alphanumerics only). Pairs the name the storefront gives with our
/// row. Returns the first match.
pub fn game_for_store_name(name: &str) -> Option<&'static PlaytimeGame> {
    let norm = normalize(name);
    if norm.is_empty() {
        return None;
    }
    PLAYTIME_CATALOG
        .iter()
        .find(|g| normalize(g.display_name) == norm || g.slug.replace('-', "") == norm)
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_lookup_is_case_insensitive() {
        let g = game_for_process("FortniteClient-Win64-Shipping.exe").expect("fortnite");
        assert_eq!(g.slug, "fortnite");
        assert!(game_for_process("notagame.exe").is_none());
        assert!(game_for_process("").is_none());
    }

    #[test]
    fn store_name_matches_steam_and_epic_names() {
        assert_eq!(game_for_store_name("Rust").map(|g| g.slug), Some("rust"));
        assert_eq!(
            game_for_store_name("Fortnite").map(|g| g.slug),
            Some("fortnite")
        );
        assert_eq!(
            game_for_store_name("Counter-Strike 2").map(|g| g.slug),
            Some("counter-strike-2")
        );
        assert!(game_for_store_name("Some Unlisted Indie").is_none());
    }

    #[test]
    fn slugs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for g in PLAYTIME_CATALOG {
            assert!(seen.insert(g.slug), "duplicate slug {}", g.slug);
            assert!(!g.processes.is_empty(), "{} has no processes", g.slug);
        }
    }
}
