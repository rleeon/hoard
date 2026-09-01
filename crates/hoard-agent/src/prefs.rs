//! User preferences for the desktop app.
//!
//! These are settings the user chooses through the Settings page: things like
//! "minimise to tray when I close the window" or "show me a notification when
//! a backup finishes". They live in their own JSON file next to the rest of
//! Hoard's state so the user can wipe them independently of credentials and
//! tracked-save metadata.
//!
//! Defaults are picked to be safe and unsurprising for first-run users:
//! native notifications off (the in-app feed carries the news), close-to-tray
//! on, and since the silent-start change, autostart plus start-minimised
//! on, so Hoard runs at login as a background tray app. The desktop only hides
//! the window when launched via the autostart `--silent` flag, so a manual
//! first launch still shows the UI. Diagnostic log shipping is on by default
//! (opt-out in Settings).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Fixed cadence for the `/v1/cloud/sync` airbag poll (desktop poller and
/// CLI daemon). Deliberately **not** a pref: Realtime push is the primary
/// trigger and the poll only catches the rare missed push, so there's no
/// user-visible gain in going faster, but a hand-edited `prefs.json`
/// could hammer the server (2 s ≈ 43k req/day per client). Server cost is
/// not a user knob. Was `cloud_poll_interval_secs` in prefs; old files
/// keep loading because serde ignores unknown keys.
///
/// The number itself lives in the kernel
/// ([`hoard_core::kernel::reconcile::CLOUD_POLL_INTERVAL_SECS`]) and is
/// re-exported here so call sites keep their old path. The kernel needs it to
/// derive how long a cloud-version cache may age before it stops counting as
/// convergence (ADR 0021 D.10), and two literals in two crates is exactly the
/// drift that would make that threshold lie.
pub const CLOUD_POLL_INTERVAL_SECS: u32 =
    hoard_core::kernel::reconcile::CLOUD_POLL_INTERVAL_SECS as u32;

/// Persisted user preferences. New fields default to safe values so older
/// `prefs.json` files keep loading after an upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    /// When `true`, closing the window hides it to the tray instead of
    /// quitting the process. The agent keeps running in the background.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,

    /// When `true`, show a desktop notification after every successful backup.
    /// Off by default (1.0.0): the activity feed already narrates uploads,
    /// so the native banner is opt-in for users who want the extra nudge.
    #[serde(default)]
    pub notify_on_success: bool,

    /// When `true`, show a desktop notification after a failed backup. Off by
    /// default (1.0.0), same rationale: failures already surface in-app as a
    /// long-lived toast plus the red state on the Library row, so the native
    /// banner is an opt-in extra channel, not the only alarm.
    #[serde(default)]
    pub notify_on_failure: bool,

    /// When `true`, the launcher integration registers Hoard to start on
    /// login. We don't read this directly (the autostart plugin owns the truth)
    /// but we mirror it so the Settings page can render without an
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

    /// Consent to share diagnostic logs with the connected server. With `true`,
    /// the shipper (`logship.rs`) sends events to `/v1/cloud/logs` (cloud) or
    /// `/v1/logs` (self-hosted) at the level the server advertises, tagged with
    /// hostname, OS, app version and a `SHA256(machine-id|hostname)` fingerprint.
    /// With `false` nothing goes out. The flag is re-read every cycle, so turning
    /// it off stops the shipping within seconds and needs no restart.
    ///
    /// It is opt-out: the default is `true` (see [`Prefs::default`]). The field
    /// name says "anonymous" and the payload carries a device fingerprint, so it
    /// is not. What the shipper does guarantee is that paths go out without the
    /// profile segment (`logship::redact`), and that is what the Settings label
    /// now says, in those words.
    #[serde(default)]
    pub anonymous_telemetry: bool,

    /// When `true`, this machine ships its playtime breakdown (day, game,
    /// seconds) to the account's server so Wrapple can show real hours merged
    /// across devices. When `false` nothing leaves the machine: the local
    /// store keeps accruing on disk, but no push happens and Wrapple has
    /// nothing to read, so the recap is empty by design.
    ///
    /// **Separate from [`Self::anonymous_telemetry`] on purpose.** That flag
    /// covers diagnostic log shipping and its consent copy promises never to
    /// send game names; playtime is game names by construction, and it is a
    /// feature the user consumes rather than a measurement we take. One switch
    /// could not honestly describe both, so there are two, and turning this one
    /// off sends nothing at all, not even a note saying it is off.
    ///
    /// Opt-out (`true` by default) so Wrapple works out of the box; the copy in
    /// Settings states that turning it off disables the recap.
    #[serde(default = "default_true")]
    pub wrapple_telemetry: bool,

    /// ISO-639 code for the desktop UI's display language (e.g. "en", "fr",
    /// "ja"). `None` means "follow the browser or OS locale"; the desktop
    /// frontend falls back to that on first run. The agent itself doesn't
    /// look at this field; it only exists so the Settings page can persist
    /// the user's language choice across restarts.
    #[serde(default)]
    pub language: Option<String>,

    /// When `true`, the agent restores the latest server snapshot into a
    /// tracked save's local path whenever that path is missing or empty on
    /// add (typical scenarios: fresh install of the game, new machine,
    /// user accidentally wiped the save folder). Defaults to `false`,
    /// silently writing files under the user's `~` is exactly the kind
    /// of thing that earns trust slowly, so we make it opt-in.
    #[serde(default)]
    pub auto_restore: bool,

    /// "Sync global", distinct from both [`Self::auto_restore`] and
    /// [`Self::automatic_mode`]. When `true`, the agent downloads a newer
    /// cloud version as soon as it detects the device is outdated, unless a
    /// game session is live (`is_running`, un-flushed changes, recent write):
    /// then the pull defers until the save settles, so it can never overwrite
    /// progress the backup hasn't captured yet (that race erased a live
    /// session once; see `AgentConfig::global_sync` for the incident).
    /// Stays bandwidth-safe via the version-gate (never re-pulls a version
    /// the device already has) and non-destructive via conflict backups under
    /// `<state_dir>/conflicts/`. Backup-only saves (per-save preset) still
    /// opt out. Defaults to `false`.
    #[serde(default)]
    pub global_sync: bool,

    /// Last desktop-client version we already nudged the user about via a
    /// native OS notification. The update poller checks this before firing
    /// `sendNotification` so the user doesn't get banner-spammed every 30
    /// minutes that 1.5.0 is out. Once they've seen the notification for a
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
    /// `auto_restore = true`. Defaults to `false`, because the schedulers are
    /// fully opt-in, just like `auto_restore`.
    #[serde(default)]
    pub automatic_mode: bool,

    /// Interval, in seconds, between background detection scans when
    /// `automatic_mode` is on. The scan is the cheap half, a metadata-only
    /// disk walk that cross-references the Ludusavi catalog + Steam against
    /// the filesystem, reading no file bytes. Default `600` (10 min): the
    /// periodic walk is now a slow backstop because the agent also fires an
    /// *immediate* scan the moment it spots a heavy CPU process that looks
    /// like a just-launched game (see `agent::process_poll`), so a new game
    /// is picked up in seconds rather than waiting out the timer.
    ///
    /// Replaces the pre-1.9.14 `automatic_scan_interval_hours`. That single
    /// knob conflated the cheap scan with the expensive hash sweep, forcing
    /// a 6h compromise. The old field is intentionally *not* migrated, because
    /// its value encoded the conflated cadence we're splitting apart, so older
    /// `prefs.json` files simply pick up the new defaults (serde ignores the
    /// now-unknown key).
    #[serde(default = "default_scan_interval_secs")]
    pub automatic_scan_interval_secs: u64,

    /// Interval, in seconds, between background backup (hash) sweeps when
    /// `automatic_mode` is on. The sweep re-hashes each tracked save to
    /// catch changes the fs-watcher missed; it reads file bytes, so it's the
    /// expensive half and runs rarely (default 3600s = 1h). The agent
    /// staggers the per-save work across an effective window, which grows past
    /// this interval when there are tens of GB of saves, so sustained
    /// disk use stays spread out instead of bursting all saves at once.
    #[serde(default = "default_backup_interval_secs")]
    pub automatic_backup_interval_secs: u64,

    /// Days to keep per-save conflict backups under
    /// `<state_dir>/conflicts/<save_id>/<rfc3339>/`. The agent sweeps and
    /// removes older subdirs at the start of every auto-restore tick.
    /// Defaults to 14, long enough for the user to notice and recover a
    /// lost local edit, short enough not to balloon disk usage.
    #[serde(default = "default_conflict_retention_days")]
    pub conflict_retention_days: u32,

    /// DEAD CODE, reserved for possible future use (2026-07-04).
    /// Was the global "Modo ahorro (solo subida)" toggle: `true` would flag
    /// every new cloud upload `backup_only` (uploads but hidden from other
    /// devices' manifest pull). The toggle was removed from the desktop UI
    /// (confusing) and the flag was never actually read by the backup path,
    /// so it has no effect today. Kept + `#[serde(default)]` so existing
    /// prefs files keep deserializing; do not resurface without wiring it up.
    #[serde(default)]
    pub cloud_savings_mode: bool,

    /// When `true`, the floating ActivityFeed panel is rendered next to
    /// the sidebar so the user sees a live stream of upload / pull /
    /// throttle events. Default `true`, since it's the most useful first
    /// impression of Modo Automático working. Users who find it noisy
    /// can hide it from Settings → Cloud.
    #[serde(default = "default_true")]
    pub live_activity_visible: bool,

    /// The data-saving knob `k` in `[0,1]` (ADR 0018, decision 4). `0` is "keep
    /// everything" (aggressive cadence, long retention); `1` is maximum saving (a
    /// minimum interval of up to 10 minutes between snapshots, aggressive
    /// retention). It scales two axes: the client's `min_snapshot_interval`
    /// (axis A, via `agent::min_snapshot_interval_for`) and the server's
    /// `RetentionPolicy` (axis B). Default `0.3`, a little saving out of the box,
    /// because "keep everything" surprises users badly (the OpenTTD case).
    ///
    /// Axis A is no longer applied: `hoardd::engine` stopped deriving the floor
    /// from here once it was clear what a setting with no interface costs. The
    /// slider left Settings on 2026-06-14 and whatever value each person had
    /// written kept ruling forever; on one machine that was 1.0, ten minutes of
    /// waiting between uploads that nothing could show or change. If the control
    /// comes back, axis A comes back with it, and not before.
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

fn default_data_saving() -> f64 {
    0.3
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            notify_on_success: false,
            notify_on_failure: true,
            // Default on: Hoard registers itself at login and boots silently
            // (only the tray icon). The desktop applies the real OS autostart
            // entry on first run and only hides the window when launched via
            // the autostart `--silent` flag, so a manual first launch still
            // shows the app. See hoard-desktop/src/lib.rs setup().
            autostart: true,
            start_minimised: true,
            seen_tray_hint: false,
            // Default on: diagnostic log shipping is enabled out of the box so
            // crashes/errors reach the server. Read fresh each ship cycle, so
            // a user can turn it off in Settings and the stream stops within
            // seconds. NOTE: the payload carries a device fingerprint, and the consent
            // copy must say so (it's diagnostics, not anonymous counters).
            anonymous_telemetry: true,
            // Default on: Wrapple is the whole reason the playtime store
            // exists, and a recap that is empty until the user hunts for a
            // switch reads as broken. Off means "send nothing", never "tell
            // the server it is off".
            wrapple_telemetry: true,
            language: None,
            auto_restore: false,
            global_sync: false,
            last_update_notified_version: None,
            automatic_mode: false,
            automatic_scan_interval_secs: default_scan_interval_secs(),
            automatic_backup_interval_secs: default_backup_interval_secs(),
            conflict_retention_days: default_conflict_retention_days(),
            cloud_savings_mode: false,
            live_activity_visible: true,
            data_saving: default_data_saving(),
        }
    }
}

/// The two user-facing operating modes. This is a *derived view* over the
/// internal `auto_restore` and `global_sync` flags; the source of truth stays
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
    /// presets (`policy.auto_restore = Some(false)`) still win as exceptions,
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
    /// file is logged but doesn't kill the app: a fresh defaults struct is
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

    /// Atomically write the prefs file: temp sibling, fsync, rename over the
    /// target (see [`crate::atomic_write`]). The parent is created on the way
    /// through, so first-run writes succeed before the rest of state has been
    /// touched.
    ///
    /// This used to be a plain `fs::write`, which truncates first: a process
    /// that died mid-write left a 0-byte `prefs.json` and [`Self::load`] then
    /// silently reset every setting the user had chosen.
    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).context("serializing prefs")?;
        crate::atomic_write::write_atomic(path, text.as_bytes())
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
        // auto_restore happens to be off; global_sync is the deciding flag.
        p.global_sync = true;
        p.auto_restore = false;
        assert_eq!(p.sync_mode(), SyncMode::FullSync);
    }

    #[test]
    fn defaults_match_documented_values() {
        let p = Prefs::default();
        // Existing defaults stay stable. Guards against an accidental flip
        // of a `default_true` when somebody adds a new field.
        assert!(p.close_to_tray);
        // 1.0.0: success notifications are opt-in (the in-app feed + toasts are
        // the default channel); failures notify by default so a silent backup
        // error doesn't slip by unnoticed.
        assert!(!p.notify_on_success);
        assert!(p.notify_on_failure);
        // Default on since the "arranque silencioso" change: autostart at login
        // + start hidden (only the tray). The desktop gates the actual hide on
        // the autostart `--silent` flag so a manual launch still shows.
        assert!(p.autostart);
        assert!(p.start_minimised);
        assert!(p.anonymous_telemetry);
        // Opt-out, and independent of `anonymous_telemetry`: Wrapple needs the
        // playtime push to have anything to show, so it ships by default and
        // the Settings copy says what turning it off costs.
        assert!(p.wrapple_telemetry);
        assert!(!p.auto_restore);
        // 1.5.3: toggle off by default. 1.9.14: the single 6h interval was
        // split into a cheap 10-min scan and an expensive 1h hash sweep.
        assert!(!p.automatic_mode);
        assert_eq!(p.automatic_scan_interval_secs, 600);
        assert_eq!(p.automatic_backup_interval_secs, 3600);
        // 1.5.5: conflict backups retained for 14 days by default.
        assert_eq!(p.conflict_retention_days, 14);
        // 1.7.0: cloud-pull poller on by default; activity feed on. The poll
        // cadence stopped being a pref (fixed CLOUD_POLL_INTERVAL_SECS).
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
        assert_eq!(parsed.automatic_scan_interval_secs, 600);
        assert_eq!(parsed.automatic_backup_interval_secs, 3600);
    }

    /// 1.9.14: a `prefs.json` written by 1.9.13 still carries the old
    /// `automatic_scan_interval_hours` key. We deliberately *don't* migrate
    /// it (its value conflated scan + hash), so it's an unknown field serde
    /// must silently drop, and the new interval fields take their defaults.
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
        assert_eq!(parsed.automatic_scan_interval_secs, 600);
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

    /// 1.5.5 backwards compatibility: a `prefs.json` written by 1.5.4, without
    /// the new field, still loads and takes the 14-day default without losing the
    /// rest of its fields.
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
        // A file written before the switch existed must read as "on": every
        // install that predates it was already shipping playtime, and silently
        // flipping it off would empty their recap on upgrade.
        assert!(parsed.wrapple_telemetry);
        // ...and it must NOT inherit the diagnostics flag, which is `false` here.
        assert!(!parsed.anonymous_telemetry);
    }

    /// Invariante crítico de 1.5.3: el deserializador NO debe acoplar
    /// `automatic_mode` y `auto_restore`. La cascada "activar Modo Automático
    /// ⇒ encender auto_restore" vive en el comando Tauri `set_automatic_mode`
    /// (`crates/hoard-desktop/src/commands/prefs.rs`), no en `Prefs`. Si un
    /// day somebody tries to "simplify" by deriving one from the other in the
    /// type, this test has to break and force a conversation.
    #[test]
    fn automatic_mode_true_in_json_does_not_force_auto_restore() {
        // Only `automatic_mode = true`; everything else defaults.
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

    /// A crash inside the old truncate-then-write left exactly this: a file
    /// that exists and is empty. Loading has to survive it (it already did),
    /// and the record of what it costs the user lives here.
    #[test]
    fn a_zero_byte_prefs_file_loads_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        std::fs::write(&path, b"").unwrap();

        let p = Prefs::load(&path).expect("a 0-byte prefs.json must not fail the load");

        assert_eq!(p.sync_mode(), Prefs::default().sync_mode());
        assert_eq!(p.close_to_tray, Prefs::default().close_to_tray);
    }

    /// The other half of a torn write: some bytes made it, the closing brace
    /// didn't.
    #[test]
    fn a_truncated_prefs_file_loads_as_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        std::fs::write(&path, br#"{"close_to_tray": fal"#).unwrap();

        let p = Prefs::load(&path).expect("a truncated prefs.json must not fail the load");

        assert_eq!(p.close_to_tray, Prefs::default().close_to_tray);
    }

    /// The fix proper: saving replaces the file in one step, so the reload sees
    /// the whole thing and no temp file is left in the state dir.
    #[test]
    fn saving_over_a_corrupt_file_writes_it_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prefs.json");
        std::fs::write(&path, b"").unwrap();

        let mut p = Prefs::default();
        p.set_sync_mode(SyncMode::FullSync);
        p.close_to_tray = !Prefs::default().close_to_tray;
        p.save(&path).unwrap();

        let back = Prefs::load(&path).unwrap();
        assert_eq!(back.sync_mode(), SyncMode::FullSync);
        assert_eq!(back.close_to_tray, p.close_to_tray);

        let left: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, vec!["prefs.json".to_string()]);
    }

    /// Saving into a state dir that doesn't exist yet is the first-run path;
    /// the atomic write has to keep doing the `create_dir_all` the old one did.
    #[test]
    fn saving_creates_the_state_dir_on_first_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("prefs.json");

        Prefs::default().save(&path).unwrap();

        assert!(Prefs::load(&path).is_ok());
    }
}
