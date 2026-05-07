//! Manifest TOML schema.
//!
//! See `data/games/stardew-valley.toml` for a worked example.

use serde::{Deserialize, Serialize};

/// One game's complete save-path manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameManifest {
    /// Slug id, lowercase-kebab. Stable across versions — used as DB key.
    pub id: String,

    /// Human-readable title shown in UI.
    pub title: String,

    /// Steam application id, if the game is on Steam.
    #[serde(default)]
    pub steam_appid: Option<u32>,

    /// GOG product id, if on GOG.
    #[serde(default)]
    pub gog_id: Option<u64>,

    /// Epic Games Store catalog item id (the long hex string).
    #[serde(default)]
    pub epic_id: Option<String>,

    /// Xbox / Microsoft Store package family name (e.g. `Mojang.MinecraftUWP_8wekyb3d8bbwe`).
    #[serde(default)]
    pub xbox_pfn: Option<String>,

    /// Process executable names to watch for. We trigger backups when any of
    /// these is running. On Windows include the `.exe` suffix.
    ///
    /// Multiple entries cover renamed/launcher variants (e.g. game.exe
    /// vs. game_launcher.exe).
    #[serde(default)]
    pub processes: Vec<String>,

    /// Windows save-file locations. Empty means "not applicable on Windows".
    #[serde(default)]
    pub windows_paths: Vec<SavePath>,

    /// Linux save-file locations (native, NOT Proton — Proton resolution
    /// reuses [`Self::windows_paths`] against the prefix). Reserved for v0.4.
    #[serde(default)]
    pub linux_paths: Vec<SavePath>,

    /// Where the data came from. Determines licensing.
    #[serde(default)]
    pub source: ManifestSource,

    /// Optional human note displayed in advanced UI / diagnostics.
    #[serde(default)]
    pub notes: Option<String>,
}

/// One save location for a game on one OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePath {
    /// Path with placeholder tokens. Examples:
    /// - `{APPDATA}/StardewValley/Saves`
    /// - `{DOCUMENTS}/My Games/Skyrim Special Edition/Saves`
    /// - `{STEAM_USERDATA}/<steamid>/<appid>/remote`
    pub location: String,

    /// Glob patterns to include relative to `location`. Defaults to `["**/*"]`
    /// (everything recursive).
    #[serde(default = "default_patterns")]
    pub patterns: Vec<String>,

    /// Glob patterns to exclude (e.g. lock files, telemetry).
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Optional tag identifying this slot (e.g. `"saves"`, `"settings"`,
    /// `"screenshots"`). Lets the UI group multi-path games.
    #[serde(default)]
    pub tag: Option<String>,
}

fn default_patterns() -> Vec<String> {
    vec!["**/*".to_string()]
}

/// Provenance of a manifest entry. Drives licensing & UI badges.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ManifestSource {
    /// Authored by the Hoard project. AGPL-3.0, free to embed.
    #[default]
    HandCurated,
    /// Imported from PCGamingWiki. CC-BY-NC-SA-3.0 — must be loaded from
    /// an external `data/manifest/` directory, never embedded statically.
    /// Reserved for v0.4+; if you see this on a hand-shipped manifest
    /// today, that's a bug.
    PcGamingWiki,
    /// Reported by a user via the in-app contributor flow (future).
    UserContributed,
}

impl GameManifest {
    /// Returns the save paths appropriate for the current OS. Currently
    /// Windows-only; Linux native paths come back here in v0.4.
    pub fn paths_for_current_os(&self) -> &[SavePath] {
        #[cfg(windows)]
        {
            &self.windows_paths
        }
        #[cfg(not(windows))]
        {
            // On Linux today we still expose Windows paths so dev builds can
            // exercise the resolver against fake Known Folders. Detection
            // pipeline will gate this properly in Phase 2.
            if !self.linux_paths.is_empty() {
                &self.linux_paths
            } else {
                &self.windows_paths
            }
        }
    }
}
