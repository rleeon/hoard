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

    /// "Sync global" — distinct from both [`Self::auto_restore`] and
    /// [`Self::automatic_mode`]. When `true`, the agent downloads a newer
    /// cloud version the moment it detects the device is outdated, **even
    /// while a game is running or the save was just written**: the sweep
    /// bypasses the "user is mid-session" guards. Stays bandwidth-safe via
    /// the version-gate (never re-pulls a version the device already has) and
    /// non-destructive via conflict backups under `<state_dir>/conflicts/`.
    /// Backup-only saves (per-save preset) still opt out. Defaults to `false`.
    #[serde(default)]
    pub global_sync: bool,

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
    /// app keeps two background schedulers alive: a cheap detection scan
    /// (every `automatic_scan_interval_secs`) that tracks newly installed
    /// games, and an expensive hash sweep (every
    /// `automatic_backup_interval_secs`) that catches save changes the
    /// fs-watcher missed. Activating the toggle also cascades
    /// `auto_restore = true`. Defaults to `false` — the schedulers are
    /// fully opt-in, just like `auto_restore`.
    #[serde(default)]
    pub automatic_mode: bool,

    /// Interval, in seconds, between background detection scans when
    /// `automatic_mode` is on. The scan is the cheap half — a metadata-only
    /// disk walk that cross-references the Ludusavi catalog + Steam against
    /// the filesystem, reading no file bytes. Default `600` (10 min): the
    /// periodic walk is now a slow backstop because the agent also fires an
    /// *immediate* scan the moment it spots a heavy CPU process that looks
    /// like a just-launched game (see `agent::process_poll`), so a new game
    /// is picked up in seconds rather than waiting out the timer.
    ///
    /// Replaces the pre-1.9.14 `automatic_scan_interval_hours`. That single
    /// knob conflated the cheap scan with the expensive hash sweep, forcing
    /// a 6h compromise. The old field is intentionally *not* migrated — its
    /// value encoded the conflated cadence we're splitting apart — so older
    /// `prefs.json` files simply pick up the new defaults (serde ignores the
    /// now-unknown key).
    #[serde(default = "default_scan_interval_secs")]
    pub automatic_scan_interval_secs: u64,

    /// Interval, in seconds, between background backup (hash) sweeps when
    /// `automatic_mode` is on. The sweep re-hashes each tracked save to
    /// catch changes the fs-watcher missed; it reads file bytes, so it's the
    /// expensive half and runs rarely (default 3600s = 1h). The agent
    /// staggers the per-save work across an effective window — which grows
    /// past this interval when there are tens of GB of saves — so sustained
    /// disk use stays spread out instead of bursting all saves at once.
    #[serde(default = "default_backup_interval_secs")]
    pub automatic_backup_interval_secs: u64,

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
    /// against the bandwidth quota). Since 2.4.0 this is the relaxed *backup*
    /// ("airbag") cadence: the Supabase Realtime push (`cloud_realtime`) is the
    /// primary near-instant trigger, so the poll only has to catch the rare
    /// missed push, hence a 60 s default instead of the old 10 s. Range
    /// 5..=300 s; the Settings slider in Cloud section persists this. Decoupled
    /// from the `automatic_backup_interval_secs` sweep because that one
    /// re-hashes save bytes and only makes sense at the hourly scale.
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

fn default_scan_interval_secs() -> u64 {
    600
}

fn default_backup_interval_secs() -> u64 {
    3600
}

fn default_conflict_retention_days() -> u32 {
    14
}

fn default_cloud_poll_interval_secs() -> u32 {
    60
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
            global_sync: false,
            last_update_notified_version: None,
            automatic_mode: false,
            automatic_scan_interval_secs: default_scan_interval_secs(),
            automatic_backup_interval_secs: default_backup_interval_secs(),
            conflict_retention_days: default_conflict_retention_days(),
            cloud_savings_mode: false,
            cloud_poll_interval_secs: default_cloud_poll_interval_secs(),
            live_activity_visible: true,
            data_saving: default_data_saving(),
        }
    }
}

/// The two user-facing operating modes. This is a *derived view* over the
/// internal `auto_restore` / `global_sync` flags — the source of truth stays
/// those two booleans so per-save presets and the existing agent plumbing keep
/// working unchanged. The UI only ever shows / sets this binary choice; it
/// never exposes the two internal toggles directly.
///
/// * `BackupOnly` → `global_sync = false`, `auto_restore = false`: uploads
///   local changes but never downloads automatically. Restoring is always a
///   manual action.
/// * `FullSync` → `global_sync = true`: automatic upload **and** download with
///   the version-gate and mid-session guards. Cross-device adoption downloads
///   on link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    BackupOnly,
    FullSync,
}

impl Prefs {
    /// Derive the user-facing [`SyncMode`] from the internal flags. `FullSync`
    /// the moment `global_sync` is on; `BackupOnly` otherwise. We key off
    /// `global_sync` alone because that's the flag that opens the
    /// download/version-gate path (`auto_restore` is the older, narrower
    /// "restore on add" knob that `global_sync` subsumes).
    pub fn sync_mode(&self) -> SyncMode {
        if self.global_sync {
            SyncMode::FullSync
        } else {
            SyncMode::BackupOnly
        }
    }

    /// Apply a [`SyncMode`] onto the internal flags. `FullSync` turns both
    /// `global_sync` and `auto_restore` on (the latter for older code paths
    /// that still consult it directly); `BackupOnly` turns both off. Per-save
    /// presets (`policy.auto_restore = Some(false)`) still win as exceptions —
    /// that logic lives in the agent, not here.
    pub fn set_sync_mode(&mut self, mode: SyncMode) {
        match mode {
            SyncMode::BackupOnly => {
                self.global_sync = false;
                self.auto_restore = false;
            }
            SyncMode::FullSync => {
                self.global_sync = true;
                self.auto_restore = true;
            }
        }
    }

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
    fn sync_mode_round_trips_through_internal_flags() {
        // Default prefs (both flags false) read as backup-only.
        let mut p = Prefs::default();
        assert_eq!(p.sync_mode(), SyncMode::BackupOnly);

        // FullSync turns both internal flags on.
        p.set_sync_mode(SyncMode::FullSync);
        assert!(p.global_sync);
        assert!(p.auto_restore);
        assert_eq!(p.sync_mode(), SyncMode::FullSync);

        // BackupOnly turns both back off.
        p.set_sync_mode(SyncMode::BackupOnly);
        assert!(!p.global_sync);
        assert!(!p.auto_restore);
        assert_eq!(p.sync_mode(), SyncMode::BackupOnly);

        // global_sync alone (legacy state) still reads as FullSync even if
        // auto_restore happens to be off — global_sync is the deciding flag.
        p.global_sync = true;
        p.auto_restore = false;
        assert_eq!(p.sync_mode(), SyncMode::FullSync);
    }

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
        // 1.5.3: toggle off by default. 1.9.14: the single 6h interval was
        // split into a cheap 5-min scan and an expensive 1h hash sweep.
        assert!(!p.automatic_mode);
        assert_eq!(p.automatic_scan_interval_secs, 300);
        assert_eq!(p.automatic_backup_interval_secs, 3600);
        // 1.5.5: conflict backups retained for 14 days by default.
        assert_eq!(p.conflict_retention_days, 14);
        // 1.7.0: cloud-pull poller on by default; activity feed on. 2.4.0:
        // relaxed 10 s → 60 s now that Realtime push is the primary trigger.
        assert_eq!(p.cloud_poll_interval_secs, 60);
        assert!(p.live_activity_visible);
        // Storage-efficiency: "ahorro de datos" defaults to 0.3 (ADR 0018).
        assert_eq!(p.data_saving, 0.3);
    }

    #[test]
    fn pre_153_json_deserialises_with_new_defaults() {
        // Shape of a prefs.json written by 1.5.2 (no `automatic_mode` or the
        // interval fields). The `#[serde(default)]` and
        // `#[serde(default = "...")]` attributes must fill them in
        // transparently.
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
        assert_eq!(parsed.automatic_scan_interval_secs, 300);
        assert_eq!(parsed.automatic_backup_interval_secs, 3600);
    }

    /// 1.9.14: a `prefs.json` written by 1.9.13 still carries the old
    /// `automatic_scan_interval_hours` key. We deliberately *don't* migrate
    /// it (its value conflated scan + hash), so it's an unknown field serde
    /// must silently drop — the new interval fields take their defaults.
    #[test]
    fn pre_1914_scan_interval_hours_is_dropped_not_migrated() {
        let legacy = r#"{
            "automatic_mode": true,
            "automatic_scan_interval_hours": 12
        }"#;
        let parsed: Prefs =
            serde_json::from_str(legacy).expect("1.9.13 prefs.json should still parse");
        assert!(parsed.automatic_mode);
        // Old value (12h) is gone; new fields are at their defaults, not 12.
        assert_eq!(parsed.automatic_scan_interval_secs, 300);
        assert_eq!(parsed.automatic_backup_interval_secs, 3600);
    }

    #[test]
    fn round_trip_preserves_non_default_automatic_mode() {
        let p = Prefs {
            automatic_mode: true,
            automatic_scan_interval_secs: 120,
            automatic_backup_interval_secs: 7200,
            ..Prefs::default()
        };
        let json = serde_json::to_string(&p).expect("serialising prefs");
        let back: Prefs = serde_json::from_str(&json).expect("round-trip");
        assert!(back.automatic_mode);
        assert_eq!(back.automatic_scan_interval_secs, 120);
        assert_eq!(back.automatic_backup_interval_secs, 7200);
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
