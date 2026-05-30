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

    /// When `true`, the agent restores the latest server snapshot into a
    /// tracked save's local path whenever that path is missing or empty on
    /// add (typical scenarios: fresh install of the game, new machine,
    /// user accidentally wiped the save folder). Defaults to `false` —
    /// silently writing files under the user's `~` is exactly the kind
    /// of thing that earns trust slowly, so we make it opt-in.
    #[serde(default)]
    pub auto_restore: bool,

    /// Last desktop-client version we already nudged the user about via a
    /// native OS notification. The update poller checks this before firing
    /// `sendNotification` so the user doesn't get banner-spammed every 30
    /// minutes that 1.5.0 is out — once they've seen the notification for a
    /// given version we leave them alone (the amber sidebar badge still
    /// shows). Reset to `None` after the user installs an update (the new
    /// client doesn't match this string anymore, so the next *newer* release
    /// will notify again).
    #[serde(default)]
    pub last_update_notified_version: Option<String>,

    /// When `true`, the sidebar "Modo Automático" toggle is on. The desktop
    /// app keeps a background scheduler alive that re-runs the full magic
    /// flow (scan → track high-confidence detections → boot the agent)
    /// every `automatic_scan_interval_hours`, and the toggle also cascades
    /// `auto_restore = true` on activation. Defaults to `false` — the
    /// scheduler is fully opt-in, just like `auto_restore`.
    #[serde(default)]
    pub automatic_mode: bool,

    /// Interval, in hours, between background scans when `automatic_mode`
    /// is enabled. Defaults to 6h — a balance between freshness and not
    /// hammering disks. The frontend has no UI to change this yet; it's
    /// exposed as a field so power users can edit `prefs.json` by hand and
    /// so future Settings pages can surface a slider without a migration.
    #[serde(default = "default_scan_interval_hours")]
    pub automatic_scan_interval_hours: u32,

    /// Days to keep per-save conflict backups under
    /// `<state_dir>/conflicts/<save_id>/<rfc3339>/`. The agent sweeps and
    /// removes older subdirs at the start of every auto-restore tick.
    /// Defaults to 14 — long enough for the user to notice and recover a
    /// lost local edit, short enough not to balloon disk usage.
    #[serde(default = "default_conflict_retention_days")]
    pub conflict_retention_days: u32,

    /// Global "modo ahorro" — when `true`, every new cloud upload is
    /// flagged `backup_only` so the save uploads (and stays version-able
    /// from this device) but no *other* device pulls it in via the
    /// manifest. Pairs with the per-save toggle on the Library card; the
    /// global setting just changes the default for new uploads. Defaults
    /// to `false` so the multi-device flow keeps working out of the box.
    #[serde(default)]
    pub cloud_savings_mode: bool,

    /// How often the desktop pulls `/v1/cloud/sync` to learn what other
    /// devices have uploaded. Cheap call (<5 KB manifest, not counted
    /// against the bandwidth quota) so 10 s default reads as "instant"
    /// across devices without flooding the server. Range 5..=300 s; the
    /// Settings slider in Cloud section persists this. Decoupled from
    /// `automatic_scan_interval_hours` because that one runs the much
    /// heavier scan-library + backup-stale sweep and only makes sense at
    /// the hourly scale.
    #[serde(default = "default_cloud_poll_interval_secs")]
    pub cloud_poll_interval_secs: u32,

    /// When `true`, the floating ActivityFeed panel is rendered next to
    /// the sidebar so the user sees a live stream of upload / pull /
    /// throttle events. Default `true` — it's the most useful first
    /// impression of Modo Automático working. Users who find it noisy
    /// can hide it from Settings → Cloud.
    #[serde(default = "default_true")]
    pub live_activity_visible: bool,

    /// "Ahorro de datos" knob `k ∈ [0,1]` (ADR 0018 Decisión 4). `0` =
    /// "guardar todo" (cadencia agresiva, retención larga); `1` = "máximo
    /// ahorro" (intervalo mínimo de hasta 10 min entre snapshots, retención
    /// agresiva). Escala dos ejes: el `min_snapshot_interval` del cliente
    /// (eje A, vía `agent::min_snapshot_interval_for`) y la `RetentionPolicy`
    /// del server (eje B). Default `0.3` — un poco de ahorro de fábrica
    /// porque "guardar todo" sorprende mal al usuario (caso OpenTTD).
    #[serde(default = "default_data_saving")]
    pub data_saving: f64,
}

fn default_true() -> bool {
    true
}

fn default_scan_interval_hours() -> u32 {
    6
}

fn default_conflict_retention_days() -> u32 {
    14
}

fn default_cloud_poll_interval_secs() -> u32 {
    10
}

fn default_data_saving() -> f64 {
    0.3
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
            auto_restore: false,
            last_update_notified_version: None,
            automatic_mode: false,
            automatic_scan_interval_hours: default_scan_interval_hours(),
            conflict_retention_days: default_conflict_retention_days(),
            cloud_savings_mode: false,
            cloud_poll_interval_secs: default_cloud_poll_interval_secs(),
            live_activity_visible: true,
            data_saving: default_data_saving(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documented_values() {
        let p = Prefs::default();
        // Existing defaults stay stable — guards against an accidental flip
        // of a `default_true` when somebody adds a new field.
        assert!(p.close_to_tray);
        assert!(p.notify_on_success);
        assert!(p.notify_on_failure);
        assert!(!p.autostart);
        assert!(!p.auto_restore);
        // New fields introduced in 1.5.3 — toggle is off, interval is 6h.
        assert!(!p.automatic_mode);
        assert_eq!(p.automatic_scan_interval_hours, 6);
        // 1.5.5: conflict backups retained for 14 days by default.
        assert_eq!(p.conflict_retention_days, 14);
        // 1.7.0: cloud-pull poller every 10 s by default; activity feed on.
        assert_eq!(p.cloud_poll_interval_secs, 10);
        assert!(p.live_activity_visible);
        // Storage-efficiency: "ahorro de datos" defaults to 0.3 (ADR 0018).
        assert_eq!(p.data_saving, 0.3);
    }

    #[test]
    fn pre_153_json_deserialises_with_new_defaults() {
        // Shape of a prefs.json written by 1.5.2 (no `automatic_mode` or
        // `automatic_scan_interval_hours`). The `#[serde(default)]` and
        // `#[serde(default = "default_scan_interval_hours")]` attributes
        // must fill them in transparently.
        let legacy = r#"{
            "close_to_tray": true,
            "notify_on_success": true,
            "notify_on_failure": true,
            "autostart": false,
            "start_minimised": false,
            "seen_tray_hint": false,
            "anonymous_telemetry": false,
            "language": null,
            "auto_restore": false,
            "last_update_notified_version": null
        }"#;
        let parsed: Prefs =
            serde_json::from_str(legacy).expect("legacy prefs.json should still parse");
        assert!(!parsed.automatic_mode);
        assert_eq!(parsed.automatic_scan_interval_hours, 6);
    }

    #[test]
    fn round_trip_preserves_non_default_automatic_mode() {
        let mut p = Prefs::default();
        p.automatic_mode = true;
        p.automatic_scan_interval_hours = 12;
        let json = serde_json::to_string(&p).expect("serialising prefs");
        let back: Prefs = serde_json::from_str(&json).expect("round-trip");
        assert!(back.automatic_mode);
        assert_eq!(back.automatic_scan_interval_hours, 12);
    }

    /// 1.5.5 retro-compat: un `prefs.json` escrito por 1.5.4 (sin
    /// `conflict_retention_days`) debe seguir cargando y rellenar el
    /// default 14d sin perder el resto de los campos.
    #[test]
    fn pre_155_json_deserialises_with_conflict_retention_default() {
        let legacy = r#"{
            "close_to_tray": true,
            "notify_on_success": true,
            "notify_on_failure": true,
            "autostart": false,
            "start_minimised": false,
            "seen_tray_hint": false,
            "anonymous_telemetry": false,
            "language": null,
            "auto_restore": true,
            "last_update_notified_version": null,
            "automatic_mode": true,
            "automatic_scan_interval_hours": 6
        }"#;
        let parsed: Prefs =
            serde_json::from_str(legacy).expect("1.5.4 prefs.json should still parse");
        assert_eq!(parsed.conflict_retention_days, 14);
        assert!(parsed.auto_restore);
        assert!(parsed.automatic_mode);
    }

    /// Invariante crítico de 1.5.3: el deserializador NO debe acoplar
    /// `automatic_mode` y `auto_restore`. La cascada "activar Modo Automático
    /// ⇒ encender auto_restore" vive en el comando Tauri `set_automatic_mode`
    /// (`crates/hoard-desktop/src/commands/prefs.rs`), no en `Prefs`. Si un
    /// día alguien intenta "simplificar" derivando una de la otra en el
    /// tipo, este test debe romper para forzar una conversación.
    #[test]
    fn automatic_mode_true_in_json_does_not_force_auto_restore() {
        // Sólo `automatic_mode = true` — todo lo demás default.
        let json = r#"{"automatic_mode": true}"#;
        let parsed: Prefs = serde_json::from_str(json)
            .expect("minimal prefs.json with only automatic_mode should parse");
        assert!(parsed.automatic_mode, "automatic_mode should be true");
        assert!(
            !parsed.auto_restore,
            "auto_restore must remain false; cascade lives in the Tauri command, not the deserialiser",
        );
        // Belt-and-braces: el round-trip también respeta la independencia.
        let json2 = serde_json::to_string(&parsed).unwrap();
        let back: Prefs = serde_json::from_str(&json2).unwrap();
        assert!(back.automatic_mode);
        assert!(!back.auto_restore);
    }
}
