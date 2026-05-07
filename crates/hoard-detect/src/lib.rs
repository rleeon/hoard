//! `hoard-detect` — when does the user start a game, and which one?
//!
//! Two independent capabilities:
//!
//! 1. [`process::ProcessWatcher`] — poll running processes and emit
//!    [`process::ProcessEvent::GameStarted`] / `GameStopped` for any
//!    process matching a manifest's `processes` list. **This is the
//!    primary detection path** in v0.3 — we never scan a save tree
//!    speculatively, only react to the game the user is playing.
//!
//! 2. [`store::scan_steam`] — walk Steam's `appmanifest_*.acf` files to
//!    populate the Library UI with what's *installed*. Runs once on
//!    first launch so the user sees their library populated even before
//!    they play; thereafter the process watcher does all the work.
//!
//! Other stores (GOG / Epic / Xbox) are stubbed for v0.3 and get filled
//! in over the v0.3.x patch line.

pub mod process;
pub mod store;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where Hoard discovered an installed game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GameSource {
    Steam,
    Gog,
    Epic,
    Xbox,
    /// User added it manually via the UI.
    Manual,
}

/// One installed game discovered by [`store::scan_all`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedGame {
    /// Slug from the manifest catalogue.
    pub manifest_id: String,
    /// Pretty title (denormalised from manifest for UI convenience).
    pub title: String,
    /// Where the game's binary lives, if known.
    pub install_dir: Option<PathBuf>,
    /// Which storefront we found it through.
    pub source: GameSource,
}

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("Steam library not found: {0}")]
    SteamMissing(String),
    #[error("I/O error while scanning {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("malformed Steam manifest at {path}: {message}")]
    SteamParse { path: String, message: String },
}
