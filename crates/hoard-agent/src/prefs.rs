//! User preferences for the desktop app.
//!
//! These are settings the user chooses through the Settings page — things like
//! "minimise to tray when I close the window" or "show me a notification when
//! a backup finishes". They live in their own JSON file next to the rest of
//! Hoard's state so the user can wipe them independently of credentials and
//! tracked-save metadata.
//!
//! Defaults are picked to be safe and unsurprising for first-run users:
//! notifications on, close-to-tray on, autostart off (the user has to opt in).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persisted user preferences. New fields default to safe values so older
/// `prefs.json` files keep loading after an upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    /// When `true`, closing the window hides it to the tray instead of
    /// quitting the process. The agent keeps running in the background.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,

    /// When `true`, show a desktop notification after every successful backup.
    /// Many users want this off once they trust the agent — keep it on by
    /// default so the first few backups feel concrete.
    #[serde(default = "default_true")]
    pub notify_on_success: bool,

    /// When `true`, show a desktop notification after a failed backup. We
    /// don't let the user disable this entirely without making them tick a
    /// box — silent failures are a footgun.
    #[serde(default = "default_true")]
    pub notify_on_failure: bool,

    /// When `true`, the launcher integration registers Hoard to start on
    /// login. We don't read this directly — the autostart plugin owns the
    /// truth — but we mirror it so the Settings page can render without an
    /// extra IPC round-trip.
    #[serde(default)]
    pub autostart: bool,

    /// When `true`, Hoard launches with its window hidden (only the tray icon
    /// shows up). Pairs naturally with `autostart = true`.
    #[serde(default)]
    pub start_minimised: bool,

    /// When `true`, the user has acknowledged the "Hoard is still running in
    /// the tray" first-time toast. Suppresses subsequent toasts.
    #[serde(default)]
    pub seen_tray_hint: bool,

    /// When `true`, the desktop app may send anonymous usage pings to the
    /// project's telemetry endpoint (currently: an aggregate counter for
    /// "successful backup" + "restore" + "first-run completed" — never the
    /// game name, save path, file content, or token).
    ///
    /// Defaults to `false`. We only ever look at this flag if the user
    /// explicitly turned it on. The actual sender is not implemented yet
    /// (v0.2 ships with the toggle as a no-op so we can roll it out without
    /// a settings migration); see `docs/privacy.md`.
    #[serde(default)]
    pub anonymous_telemetry: bool,

    /// ISO-639 code for the desktop UI's display language (e.g. "en", "fr",
    /// "ja"). `None` means "follow the browser/OS locale" — the desktop
    /// frontend falls back to that on first run. The agent itself doesn't
    /// look at this field; it only exists so the Settings page can persist
    /// the user's language choice across restarts.
    #[serde(default)]
    pub language: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            notify_on_success: true,
            notify_on_failure: true,
            autostart: false,
            start_minimised: false,
            seen_tray_hint: false,
            anonymous_telemetry: false,
            language: None,
        }
    }
}

impl Prefs {
    /// Where the prefs file lives. We reuse `CliConfig::state_dir` so the
    /// CLI's `--state-dir` override propagates to the desktop app too.
    pub fn default_path() -> Result<PathBuf> {
        let dir = crate::config::CliConfig::state_dir()?;
        Ok(dir.join("prefs.json"))
    }

    /// Load prefs, returning defaults if the file is missing. A malformed
    /// file is logged but doesn't kill the app — a fresh defaults struct is
    /// returned and the next save will overwrite the corrupt version.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        match serde_json::from_str(&text) {
            Ok(p) => Ok(p),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "prefs.json was corrupt; resetting to defaults");
                Ok(Self::default())
            }
        }
    }

    /// Convenience that picks the standard path automatically.
    pub fn load_default() -> Result<(Self, PathBuf)> {
        let path = Self::default_path()?;
        Ok((Self::load(&path)?, path))
    }

    /// Atomically (best-effort) write the prefs file. We `create_dir_all`
    /// the parent so first-run writes succeed before the rest of state has
    /// been touched.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serializing prefs")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}
