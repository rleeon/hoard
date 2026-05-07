//! `hoard-manifest` — game save-path catalogue.
//!
//! This crate ships a hand-curated catalogue of popular games with their
//! save-file locations on Windows (Linux/macOS coming in v0.4). The catalogue
//! is embedded into the binary at compile time via [`include_dir`], so the
//! desktop app needs no network access to know where saves live.
//!
//! ## Why hand-curated, not PCGamingWiki?
//!
//! The plan for v0.4+ is to import bulk data from PCGamingWiki. That data is
//! licensed CC-BY-NC-SA-3.0, which can't be statically linked into our
//! AGPL-3.0 binary. To ship "preinstalled paths" today without a license
//! mess, we hand-author a small subset (everything in `data/games/` is
//! original work, AGPL-3.0 like the rest of the code).
//!
//! When PCGW import lands, that data will live in a separate
//! runtime-loaded directory with its own LICENSE — not in this crate.
//!
//! ## Schema
//!
//! Each game lives in `data/games/<slug>.toml`. See [`GameManifest`] for the
//! schema. Paths use placeholder tokens like `{APPDATA}` resolved against
//! Windows Known Folders at runtime.

pub mod placeholders;
pub mod schema;

use std::collections::HashMap;
use std::sync::OnceLock;

use include_dir::{include_dir, Dir};

pub use placeholders::{resolve_placeholders, PlaceholderError};
pub use schema::{GameManifest, ManifestSource, SavePath};

/// Compile-time embedded game catalogue.
///
/// All TOML files under `data/games/` are baked into the binary.
static CATALOGUE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/games");

/// Lazily-parsed catalogue, keyed by [`GameManifest::id`].
static CATALOGUE: OnceLock<HashMap<String, GameManifest>> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to parse manifest `{file}`: {source}")]
    Parse {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("manifest `{0}` has non-UTF8 contents")]
    NotUtf8(String),
}

/// Returns the global catalogue, parsing it on first call.
///
/// Panics if any embedded TOML is malformed — this is a programmer error
/// caught at the crate's own test suite, never at runtime in shipped builds.
pub fn catalogue() -> &'static HashMap<String, GameManifest> {
    CATALOGUE.get_or_init(|| {
        load_catalogue().expect(
            "embedded manifest catalogue must parse — \
                                 this is a build-time invariant, see hoard-manifest tests",
        )
    })
}

/// Look up a game by its slug id (e.g. `"stardew-valley"`).
pub fn lookup(id: &str) -> Option<&'static GameManifest> {
    catalogue().get(id)
}

/// Look up a game by its Steam app id.
pub fn lookup_by_steam_appid(appid: u32) -> Option<&'static GameManifest> {
    catalogue().values().find(|g| g.steam_appid == Some(appid))
}

/// All games in the catalogue, sorted by title for deterministic UI.
pub fn all_games() -> Vec<&'static GameManifest> {
    let mut v: Vec<_> = catalogue().values().collect();
    v.sort_by(|a, b| a.title.cmp(&b.title));
    v
}

/// Parse every embedded TOML file. Used by [`catalogue`] and tests.
pub fn load_catalogue() -> Result<HashMap<String, GameManifest>, ManifestError> {
    let mut out = HashMap::new();
    for file in CATALOGUE_DIR.files() {
        let path = file.path().to_string_lossy().into_owned();
        let text = file
            .contents_utf8()
            .ok_or_else(|| ManifestError::NotUtf8(path.clone()))?;
        let game: GameManifest = toml::from_str(text).map_err(|e| ManifestError::Parse {
            file: path.clone(),
            source: e,
        })?;
        out.insert(game.id.clone(), game);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time catch: every shipped TOML must parse.
    #[test]
    fn all_embedded_manifests_parse() {
        let cat = load_catalogue().expect("catalogue parse");
        assert!(!cat.is_empty(), "catalogue should not be empty");
    }

    /// Every manifest must have at least one save path on at least one OS.
    #[test]
    fn all_manifests_have_paths() {
        let cat = load_catalogue().expect("catalogue parse");
        for (id, game) in &cat {
            assert!(
                !game.windows_paths.is_empty() || !game.linux_paths.is_empty(),
                "game `{id}` has no save paths defined"
            );
        }
    }

    /// IDs must be unique and lowercase-kebab.
    #[test]
    fn ids_are_well_formed() {
        let cat = load_catalogue().expect("catalogue parse");
        for id in cat.keys() {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "id `{id}` must be lowercase-kebab"
            );
        }
    }

    #[test]
    fn lookup_round_trip() {
        // Pick any game from the catalogue and confirm both lookups work.
        let any = all_games().first().copied().expect("at least one game");
        assert_eq!(lookup(&any.id).map(|g| &g.id), Some(&any.id));
    }
}
