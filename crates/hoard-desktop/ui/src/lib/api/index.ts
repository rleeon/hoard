/**
 * Typed wrappers around Tauri's `invoke()`.
 *
 * Keeping all `invoke` calls here means commands have one source of truth for
 * their argument and return shapes — components just import a function and
 * the compiler enforces the contract.
 */

import { invoke } from "@tauri-apps/api/core";

export type HealthInfo = {
  status: string;
  version: string;
  uptime_secs: number;
};

export type UserInfo = {
  user_id: string;
  username: string;
  is_admin: boolean;
  server_url: string;
  /** Bytes the user is currently using on the server. Returned by `whoami`. */
  storage_used_bytes: number;
  /** Quota cap in bytes (0 = unlimited). */
  storage_quota_bytes: number;
  /** True when the URL points at a self-hosted server (localhost / RFC1918 /
   *  `.local`). The dashboard uses this to show MB ("23 MB used") instead of
   *  "% of quota" — for a server you own at home a quota bar is meaningless. */
  is_local_server: boolean;
  /** True when the URL points at the managed Hoard Cloud backend
   *  (`*.hoard.services` / `*.fly.dev`). The cloud upgrades itself and has no
   *  `/v1/admin/upgrade` route, so the UI hides the self-hosted server-upgrade
   *  panel for these connections. */
  is_cloud_server: boolean;
};

/** Anonymous probe — used by the wizard to validate the server URL. */
export function healthCheck(url: string): Promise<HealthInfo> {
  return invoke<HealthInfo>("health_check", { url });
}

/** Verify a (URL, token) pair against the server and persist it. */
export function login(url: string, token: string): Promise<UserInfo> {
  return invoke<UserInfo>("login", { url, token });
}

/** Cheap, sync check — does the app have a saved session? */
export function isLoggedIn(): Promise<boolean> {
  return invoke<boolean>("is_logged_in");
}

/** Cached user info from the saved session, or null. */
export function currentUser(): Promise<UserInfo | null> {
  return invoke<UserInfo | null>("current_user");
}

/** Wipe stored credentials and the in-memory cache. */
export function logout(): Promise<void> {
  return invoke<void>("logout");
}

/** Re-fetch quota from the server. Cheap (one round-trip, no body) — call
 *  this on dashboard mount and every ~30s while it's open. Returns the
 *  updated `UserInfo` for stores to swap in. */
export function refreshQuota(): Promise<UserInfo> {
  return invoke<UserInfo>("refresh_quota");
}

/** Phase 0 sanity-check command, kept for the dev "ping Rust" widget. */
export function greet(name: string): Promise<string> {
  return invoke<string>("greet", { name });
}

// ---------------------------------------------------------------------------
// Library / detection
// ---------------------------------------------------------------------------

export type Confidence = "low" | "medium" | "high";
export type DetectionSource =
  | "filesystem_heuristic"
  | "steam_library"
  | "both"
  | "manual_override";

export type DetectedGame = {
  slug: string;
  display_name: string;
  /**
   * Save-path candidates that exist on disk. Never contains the game's
   * install directory — that's in `install_dir`. Empty for Steam-only
   * matches with no save folder yet, in which case `track()` should fall
   * back to the folder picker.
   */
  found_paths: string[];
  confidence: Confidence;
  /**
   * Per-path confidence, aligned 1:1 with `found_paths` and sorted
   * strongest-first alongside it. Lets the card grade each save folder on its
   * own (the real `~/Saved Games/.../saves` as `high` vs an almost-empty
   * Steam-Cloud stub as `low`). May be empty on reports from older builds.
   */
  path_confidences: Confidence[];
  source: DetectionSource;
  steam_app_id: number | null;
  /**
   * Steam install directory (e.g. `…/steamapps/common/Stellaris`). Show
   * as a hint near the folder picker; never use as a backup path.
   */
  install_dir?: string | null;
};

export type DetectionReport = {
  games: DetectedGame[];
  catalog_size: number;
  steam_apps_found: number;
  scanned_at_ms: number;
};

export type ScanProgress = {
  done: number;
  total: number;
};

export type TrackedSave = {
  save_id: string;
  game_slug: string;
  label: string;
  local_path: string;
  last_version_num: number | null;
  last_backup_at: string | null;
  paused: boolean;
  /** Bytes occupied on the server (sum of non-deleted snapshots). */
  total_size_bytes: number;
  /** `true` when the save exists server-side but this machine has no
   *  matching CliState row (reinstall, PC switch, manual state wipe). The
   *  UI shows a discreet "Sin estado local" badge and disables the local
   *  untrack button — only `deleteSaveCompletely` is meaningful here. */
  orphan: boolean;
  /** Bytes this save occupies on THIS machine (its local folder, recursive).
   *  `null` for orphan rows (no local folder here) and freshly-created rows.
   *  Distinct from {@link total_size_bytes} (server-side footprint). */
  local_size_bytes: number | null;
  /** The user's explicitly chosen sync preset, or `null` for "standard /
   *  inherit". A built-in per-game preset may still apply at the agent
   *  level when this is `null`; this only reflects the manual override. */
  preset: string | null;
};

/** Run a full auto-detection sweep. Subscribe to `library://scan-progress`
 * with `listen()` from `@tauri-apps/api/event` to drive a progress bar. */
export function scanLibrary(): Promise<DetectionReport> {
  return invoke<DetectionReport>("scan_library");
}

/** Forced re-scan from the Library "Re-escanear" button. Same wire shape as
 *  `scan_library` — distinct command so the backend can disambiguate UI
 *  intent from page-mount auto-scans in metrics/logs. */
export function rescanLibrary(): Promise<DetectionReport> {
  return invoke<DetectionReport>("rescan_library");
}

/** Exhaustive deep scan from the Library deep-scan tile: looks at arbitrary
 *  Wine prefixes (Heroic/CrossOver/Flatpak/mounted media), Flatpak/Snap/
 *  EmuDeck roots and deeper directory walks. Slower; same wire shape and
 *  `library://scan-progress` events as `scan_library`. */
export function deepScanLibrary(): Promise<DetectionReport> {
  return invoke<DetectionReport>("deep_scan_library");
}

/** Return the previous scan if one is in memory, else null. */
export function cachedDetection(): Promise<DetectionReport | null> {
  return invoke<DetectionReport | null>("cached_detection");
}

/** Begin tracking a detected game at the given path.
 *
 *  `display_name` and `steam_app_id` are forwarded to the server so it can
 *  self-heal its games table when the desktop's catalog is fresher than the
 *  server's seed. Older servers ignore the extra fields. */
export function addGameToTracking(args: {
  game_slug: string;
  label?: string;
  local_path: string;
  display_name?: string;
  steam_app_id?: number | null;
}): Promise<TrackedSave> {
  return invoke<TrackedSave>("add_game_to_tracking", { args });
}

/** Adopt (vincular) a cloud save from another machine: bind a local folder on
 *  THIS machine to the existing `save_id` instead of creating a new save. In
 *  sync mode the agent restores the latest snapshot on attach; in backup-only
 *  mode it just watches the folder. Core of cross-device sync. */
export function adoptSave(args: {
  save_id: string;
  game_slug: string;
  label: string;
  local_path: string;
}): Promise<TrackedSave> {
  return invoke<TrackedSave>("adopt_save", { args });
}

/** List the saves currently tracked for the logged-in user. */
export function listTrackedSaves(): Promise<TrackedSave[]> {
  return invoke<TrackedSave[]>("list_tracked_saves");
}

/** Stop tracking a save locally. Server-side data is left alone. */
export function untrackSave(save_id: string): Promise<void> {
  return invoke<void>("untrack_save", { saveId: save_id });
}

/** Hard-delete a save server-side: removes the row plus every snapshot, then
 *  purges the local CliState entry and any matching `manual_paths` override
 *  so a subsequent `add_game_to_tracking` for the same slug starts clean
 *  instead of bouncing off the 409-recovery path. Destructive — the UI must
 *  gate this behind a confirmation modal. */
export function deleteSaveCompletely(saveId: string): Promise<void> {
  return invoke<void>("delete_save_completely", { saveId });
}

/** Persist a user-picked save-folder override for a slug. Wins over every
 *  heuristic on subsequent scans (source becomes `manual_override`). The
 *  backend validates that the path exists and is a directory before saving;
 *  it then refreshes the detection cache so the next render sees the
 *  override without forcing a Rescan click. */
export function setManualPath(slug: string, path: string): Promise<void> {
  return invoke<void>("set_manual_path", { slug, path });
}

/** Drop a manual override and fall back to whatever the heuristics find. */
export function clearManualPath(slug: string): Promise<void> {
  return invoke<void>("clear_manual_path", { slug });
}

/** Persistently blacklist a detected slug so it stops appearing in the
 *  Library grid. Reversible via {@link unignoreDetectedGame}; the matching
 *  Settings list renders every currently-ignored slug. */
export async function ignoreDetectedGame(slug: string): Promise<void> {
  await invoke("ignore_detected_game", { slug });
}

/** Reactivate a previously-blacklisted slug; the next scan re-surfaces it
 *  in the Library. Counterpart of {@link ignoreDetectedGame}. */
export async function unignoreDetectedGame(slug: string): Promise<void> {
  await invoke("unignore_detected_game", { slug });
}

/** Every slug the user has currently blacklisted, sorted alphabetically so
 *  the Settings page renders a stable order. */
export async function listIgnoredSlugs(): Promise<string[]> {
  return await invoke<string[]>("list_ignored_slugs");
}

export type DroppedPath = {
  path: string;
  reason: string;
};

export type TraceStep = {
  kind: string;
  template?: string | null;
  expanded?: string[];
  kept?: string[];
  dropped?: DroppedPath[];
};

export type DetectionTrace = {
  slug: string;
  attempts: TraceStep[];
};

/** Replay the detection pipeline for a single slug and return a trace of
 *  every step. Backs the hidden `/diagnostics` panel that's unlocked via
 *  5 clicks on the sidebar version. Read-only — never writes to the
 *  detection cache or `state.json`. */
export function detectionDiagnostics(slug: string): Promise<DetectionTrace> {
  return invoke<DetectionTrace>("detection_diagnostics", { slug });
}

/** Sentinel error string raised by `renameSaveLabel` when another save under
 *  the same user+game already owns the requested label. The UI matches this
 *  exact string to show a localized message instead of the raw server text. */
export const LABEL_COLLISION = "conflict:label_collision";

/** Rename a tracked save's label. The server PATCHes the row and atomically
 *  renames the on-disk snapshot directory; the local agent re-attaches with
 *  the new label so subsequent backups land in the right place. Rejects with
 *  `LABEL_COLLISION` on 409. */
export function renameSaveLabel(
  save_id: string,
  new_label: string,
): Promise<TrackedSave> {
  return invoke<TrackedSave>("rename_save_label", {
    saveId: save_id,
    newLabel: new_label,
  });
}

// ---------------------------------------------------------------------------
// Live agent (process + filesystem watcher)
// ---------------------------------------------------------------------------

export type AgentStatus = {
  running: boolean;
  watched_count: number;
};

/** Per-slot diagnostic snapshot. Mirrors
 * `hoard_agent::agent::AgentSlotStatus`. Empty array = agent not running. */
export type AgentSlotStatus = {
  save_id: string;
  display_name: string;
  path: string;
  watcher_armed: boolean;
  process_running: boolean;
  /** RFC3339 UTC or null if no event seen yet. */
  last_fs_event_at: string | null;
  /** RFC3339 UTC or null if no backup pending. */
  next_scheduled_backup_at: string | null;
};

export type BackupReason = "filesystem_settled" | "game_stopped" | "manual";

/** Tagged union mirroring `hoard_agent::agent::AgentEvent`. The `type`
 * discriminator is what `serde(tag = "type")` emits on the Rust side. */
export type AgentEvent =
  | { type: "game_started"; save_id: string; game_slug: string }
  | { type: "game_stopped"; save_id: string; game_slug: string }
  | {
      type: "backup_scheduled";
      save_id: string;
      delay_ms: number;
      reason: BackupReason;
    }
  | { type: "backup_started"; save_id: string; game_slug: string; label: string }
  | {
      type: "backup_success";
      save_id: string;
      version_num: number;
      total_bytes: number;
      set_hash: string | null;
    }
  | {
      type: "backup_failed";
      save_id: string;
      game_slug: string;
      error: string;
      will_retry: boolean;
    }
  | {
      type: "backup_throttled";
      save_id: string;
      game_slug: string;
      label: string;
      retry_after_secs: number;
    }
  | {
      type: "save_auto_restored";
      save_id: string;
      game_slug: string;
      version_num: number;
      files_extracted: number;
      bytes_extracted: number;
    }
  | {
      type: "save_auto_restore_failed";
      save_id: string;
      game_slug: string;
      error: string;
    }
  | {
      type: "backup_skipped_empty";
      save_id: string;
      game_slug: string;
    };

/** Boot the live agent and start emitting `agent://*` events. */
export function startAgent(): Promise<AgentStatus> {
  return invoke<AgentStatus>("start_agent");
}

/** Cleanly stop the agent (logout, app exit). */
export function stopAgent(): Promise<void> {
  return invoke<void>("stop_agent");
}

/** Force a backup right now, bypassing debounce. */
export function backupNow(save_id: string): Promise<void> {
  return invoke<void>("backup_now", { saveId: save_id });
}

/** Kick a staggered backup sweep across every tracked save (Modo Automático's
 *  hourly hash pass). The agent spreads each save's re-hash across an
 *  effective window so disk use doesn't burst — see `sweep_backups` /
 *  `AgentCommand::SweepAll` on the Rust side. No-op when the agent isn't
 *  running. */
export function sweepBackups(): Promise<void> {
  return invoke<void>("sweep_backups");
}

/** Per-slot diagnostic snapshot for the hidden Settings panel. */
export function agentStatus(): Promise<AgentSlotStatus[]> {
  return invoke<AgentSlotStatus[]>("agent_status");
}

// ---------------------------------------------------------------------------
// Preferences + tray
// ---------------------------------------------------------------------------

/** Mirrors `hoard_agent::prefs::Prefs`. Persisted to `prefs.json`. */
export type Prefs = {
  close_to_tray: boolean;
  notify_on_success: boolean;
  notify_on_failure: boolean;
  autostart: boolean;
  start_minimised: boolean;
  seen_tray_hint: boolean;
  anonymous_telemetry: boolean;
  /** ISO-639 code for the desktop UI language, e.g. "en", "fr". `null` means
   *  the user hasn't picked one yet — we then fall back to the browser
   *  language at boot. */
  language: string | null;
  /** When `true`, the agent restores the latest server snapshot into a
   *  tracked save's local path whenever that path is missing or empty on
   *  add. Off by default — silent writes under `~` are the kind of thing
   *  that earns trust slowly, so users have to opt in. */
  auto_restore: boolean;
  /** "Sync global" — distinct from both `auto_restore` and `automatic_mode`.
   *  When `true`, the agent downloads a newer cloud version the moment it
   *  detects the device is outdated, even while a game is running or the save
   *  was just written. Version-gated (never re-pulls a version already held)
   *  and non-destructive (local-newer files are parked under conflicts).
   *  Off by default. */
  global_sync: boolean;
  /** Last desktop-client version we already fired a native notification
   *  about. The update poller compares this against the latest report and
   *  only sends a notification the first time it sees a new version — the
   *  sidebar amber badge keeps showing regardless. Persisted so reopening
   *  the app doesn't re-notify for a version the user already saw. */
  last_update_notified_version: string | null;
  /** When `true`, the sidebar "Modo Automático" toggle is on. The Rust
   *  side keeps two background schedulers alive — a cheap detection scan
   *  (`automatic_scan_interval_secs`) and an expensive staggered hash sweep
   *  (`automatic_backup_interval_secs`); activating the toggle also cascades
   *  `auto_restore = true`. Off by default. */
  automatic_mode: boolean;
  /** Seconds between background detection scans while `automatic_mode` is on.
   *  The scan is the cheap, metadata-only half (no file bytes read), so it
   *  runs often — default 300s (5 min). Replaces the pre-1.9.14
   *  `automatic_scan_interval_hours`. */
  automatic_scan_interval_secs: number;
  /** Seconds between background backup (hash) sweeps while `automatic_mode`
   *  is on. The sweep re-hashes save bytes to catch missed changes, so it's
   *  the expensive half and runs rarely — default 3600s (1h). The agent
   *  staggers per-save work across an effective window that grows with the
   *  total footprint, so this is the nominal cadence, not a hard ceiling. */
  automatic_backup_interval_secs: number;
  /** Days to retain per-save conflict backups under
   *  `<state_dir>/conflicts/<save_id>/<rfc3339>/`. Defaults to 14;
   *  validated on the Rust side to 1..=30. */
  conflict_retention_days: number;
  /** Global "modo ahorro" — when `true`, every new cloud upload defaults
   *  to `backup_only`: the save still uploads and is version-able from
   *  this device, but the server hides it from *other* devices'
   *  manifest pull so nothing auto-restores it elsewhere. Pairs with
   *  the per-save toggle on the Library card. Off by default. */
  cloud_savings_mode: boolean;
  /** Seconds between manifest polls on the live cloud-pull loop.
   *  Range 5..=300; default 10. Independent from
   *  `automatic_backup_interval_secs` — that one re-hashes save bytes,
   *  this one only emits `agent://cloud-pull-*` events so the LiveStatus
   *  widget reflects server state. */
  cloud_poll_interval_secs: number;
  /** Whether the floating ActivityFeed panel is visible. Defaults to
   *  true; the user can hide it from the sidebar toggle. */
  live_activity_visible: boolean;
  /** "Ahorro de datos" knob `k ∈ [0,1]` (ADR 0018). 0 = "guardar todo"
   *  (cadencia agresiva, retención larga); 1 = "máximo ahorro" (intervalo
   *  mínimo de hasta 10 min entre snapshots, retención agresiva). Scales
   *  both the client min-snapshot-interval and the server retention
   *  policy. Default 0.3. */
  data_saving: number;
};

/** The single user-facing operating mode. Mirrors `hoard_agent::prefs::SyncMode`.
 *  Derived from / applied onto the internal `global_sync` + `auto_restore`
 *  flags — the UI only ever shows this binary choice, never the two toggles. */
export type SyncMode = "backup_only" | "full_sync";

/** Derive the user-facing mode from a Prefs object, mirroring
 *  `Prefs::sync_mode` on the Rust side: full sync the moment `global_sync` is
 *  on, backup-only otherwise. */
export function syncModeOf(p: Prefs): SyncMode {
  return p.global_sync ? "full_sync" : "backup_only";
}

export type TrayStateName =
  | "idle"
  | "running"
  | "uploading"
  | "ok"
  | "error"
  | "offline";

/** Read the prefs file from disk. */
export function getPrefs(): Promise<Prefs> {
  return invoke<Prefs>("get_prefs");
}

/** Persist prefs. Returns the saved object so the caller can hydrate stores. */
export function savePrefs(prefs: Prefs): Promise<Prefs> {
  return invoke<Prefs>("save_prefs", { prefs });
}

/** Toggle the sidebar's "Modo Automático" persisted flag. Returns the
 *  full updated prefs so the caller can hydrate every dependent store with
 *  the cascaded value (activation also flips `auto_restore` to true). The
 *  Rust side also starts or stops the background scheduler as part of the
 *  call. */
export function setAutomaticMode(enabled: boolean): Promise<Prefs> {
  return invoke<Prefs>("set_automatic_mode", { enabled });
}

/** Flip "Sync" (sync global). Distinct from Modo Automático: it doesn't start
 *  any scheduler and doesn't cascade `auto_restore`. When on, the agent
 *  downloads a newer cloud version the moment it detects the device is
 *  outdated — even while a game is running. Returns the updated prefs. */
export function setGlobalSync(enabled: boolean): Promise<Prefs> {
  return invoke<Prefs>("set_global_sync", { enabled });
}

/** Set the single user-facing operating mode (onboarding + Settings radio).
 *  Maps the chosen `SyncMode` onto the internal `global_sync` / `auto_restore`
 *  flags, persists prefs, and hot-reconfigures the live agent. Returns the
 *  updated prefs so the caller can hydrate dependent stores. */
export function setSyncMode(mode: SyncMode): Promise<Prefs> {
  return invoke<Prefs>("set_sync_mode", { mode });
}

/** Persist a new detection-scan interval (seconds, 60..=3600) for Modo
 *  Automático. If the toggle is on, the schedulers restart so the new
 *  cadence applies immediately and a scan fires right away. */
export function setScanInterval(secs: number): Promise<Prefs> {
  return invoke<Prefs>("set_scan_interval", { secs });
}

/** Persist a new backup-sweep interval (seconds, 300..=86400) for Modo
 *  Automático. The agent staggers per-save work across an effective window
 *  that grows with the total save footprint, so this is the nominal cadence,
 *  not a hard ceiling. Restarts the schedulers if the toggle is on. */
export function setBackupInterval(secs: number): Promise<Prefs> {
  return invoke<Prefs>("set_backup_interval", { secs });
}

/** Persist a new retention window (days, 1..=30) for per-save conflict
 *  backups. Picked up by the agent on its next auto-restore sweep. */
export function setConflictRetention(days: number): Promise<Prefs> {
  return invoke<Prefs>("set_conflict_retention", { days });
}

/** Persist a new cloud-pull interval (seconds, 5..=300). If a cloud
 *  session is active the poller restarts so the new cadence kicks in
 *  immediately. */
export function setCloudPollInterval(secs: number): Promise<Prefs> {
  return invoke<Prefs>("set_cloud_poll_interval", { secs });
}

/** Toggle whether the floating ActivityFeed panel renders. */
export function setLiveActivityVisible(visible: boolean): Promise<Prefs> {
  return invoke<Prefs>("set_live_activity_visible", { visible });
}

/** Persist the "ahorro de datos" knob `k ∈ [0,1]` (ADR 0018). Clamped on
 *  the Rust side. Takes effect on the agent's next boot. */
export function setDataSaving(saving: number): Promise<Prefs> {
  return invoke<Prefs>("set_data_saving", { saving });
}

/** Toggle the launcher autostart entry. Returns the resulting state. */
export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_autostart", { enabled });
}

/** Read the autostart entry's current state from the OS. */
export function isAutostartEnabled(): Promise<boolean> {
  return invoke<boolean>("is_autostart_enabled");
}

/** Recolour the tray icon. The frontend derives the global state from the
 * activity store and pushes it here. */
export function setTrayState(state: TrayStateName): Promise<void> {
  return invoke<void>("set_tray_state", { state });
}

// ---------------------------------------------------------------------------
// Snapshot history, restore, manual override, logs
// ---------------------------------------------------------------------------

export type SnapshotEntry = {
  version_num: number;
  file_count: number;
  total_size_bytes: number;
  is_pinned: boolean;
  created_at: string;
  deleted_at: string | null;
};

export type SnapshotFile = {
  relative_path: string;
  size_bytes: number;
  sha256: string;
};

export type SnapshotDetail = SnapshotEntry & {
  files: SnapshotFile[];
};

export type RestorePhase = "pre_backup" | "downloading" | "done";

export type RestoreProgress = {
  save_id: string;
  version: number;
  phase: RestorePhase;
  downloaded: number;
  total: number;
};

export type RestoreOutcome = {
  files_extracted: number;
  bytes_extracted: number;
  destination: string;
  /** If `backup_first` was set on the call, this is the version number of
   *  the safety backup the user can restore to undo this restore. */
  safety_version: number | null;
};

export type LogLine = {
  timestamp: string;
  level: string;
  message: string;
};

export function listSaveSnapshots(
  saveId: string,
  includeDeleted: boolean,
): Promise<SnapshotEntry[]> {
  return invoke<SnapshotEntry[]>("list_save_snapshots", {
    saveId,
    includeDeleted,
  });
}

export function saveSnapshotDetail(
  saveId: string,
  version: number,
): Promise<SnapshotDetail> {
  return invoke<SnapshotDetail>("save_snapshot_detail", {
    saveId,
    version,
  });
}

export function deleteSnapshot(saveId: string, version: number): Promise<void> {
  return invoke<void>("delete_snapshot", { saveId, version });
}

export function undeleteSnapshot(
  saveId: string,
  version: number,
): Promise<void> {
  return invoke<void>("undelete_snapshot", { saveId, version });
}

export function restoreSnapshot(args: {
  save_id: string;
  version: number;
  backup_first: boolean;
  /** When the save isn't tracked locally yet, the caller passes the folder
   *  the user picked from a dialog. The backend creates it if missing and
   *  records the (save_id → path) mapping in CliState so subsequent
   *  restores skip the dialog. */
  destination_override?: string | null;
}): Promise<RestoreOutcome> {
  return invoke<RestoreOutcome>("restore_snapshot", {
    saveId: args.save_id,
    version: args.version,
    backupFirst: args.backup_first,
    destinationOverride: args.destination_override ?? null,
  });
}

/** Sentinel error string from the Rust side meaning "we have no local path
 *  for this save — prompt the user to pick one and retry with
 *  `destination_override`". */
export const NEEDS_DESTINATION = "NEEDS_DESTINATION";

export function setSavePaused(saveId: string, paused: boolean): Promise<void> {
  return invoke<void>("set_save_paused", { saveId, paused });
}

export function setSaveLocalPath(
  saveId: string,
  newPath: string,
): Promise<void> {
  return invoke<void>("set_save_local_path", { saveId, newPath });
}

/** The catalog of selectable sync presets (slugs). `"standard"` means
 *  "inherit the global config"; the rest tune auto-restore / snapshot
 *  cadence for games that misbehave under the defaults. */
export function listSavePresets(): Promise<string[]> {
  return invoke<string[]>("list_save_presets");
}

/** Set (or clear, with `null`) the manual sync preset for a save. Passing
 *  `"standard"` or `null` clears the override back to the global config. */
export function setSavePreset(
  saveId: string,
  preset: string | null,
): Promise<void> {
  return invoke<void>("set_save_preset", { saveId, preset });
}

export function tailLogs(maxLines?: number): Promise<LogLine[]> {
  return invoke<LogLine[]>("tail_logs", { maxLines: maxLines ?? null });
}

export function logsPath(): Promise<string> {
  return invoke<string>("logs_path");
}

// ---- Game catalog (Ludusavi) updates --------------------------------------

/**
 * Wire shape for `catalog_status` / `update_catalog`.
 *
 * - `games`               — number of games in the currently-loaded catalog.
 * - `has_runtime_override` — whether a refreshed copy is on disk; `false`
 *                            means we're on the version that shipped with the app.
 * - `updated_at`          — Unix epoch seconds of the last successful refresh.
 */
export type CatalogStatus = {
  games: number;
  has_runtime_override: boolean;
  updated_at: number | null;
};

export type CatalogUpdateResult = {
  games: number;
  updated_at: number;
  size_bytes: number;
  path: string;
};

/** Real playtime totals, computed locally by the agent from process-running
 *  time. `days` keys are local `YYYY-MM-DD`; values are seconds played that
 *  day. Empty until a tracked game has been observed running. Local-only —
 *  this never hits the network. */
export type PlaytimeSummary = {
  days: Record<string, number>;
  by_game: Record<string, number>;
  /** day (`YYYY-MM-DD`) → game_slug → seconds. Per-day game breakdown for the
   *  recap's day-detail panel. Empty on older builds. */
  daily_by_game?: Record<string, Record<string, number>>;
  total_secs: number;
};

/** Minimal view of the Hoard Cloud account, for surfaces that just need the
 *  identity (name / email / avatar / plan). Full shape lives in
 *  `stores/cloud.ts`; extra fields the command returns are ignored here. */
export type CloudAccountInfo = {
  email: string;
  display_name: string | null;
  avatar_url: string | null;
  plan: string;
  /** Total bytes ever stored on the server (monotonic, never credited back on
   *  delete/purge). `0` when the server predates the counter or the cached
   *  session is stale — callers fall back to the current footprint. */
  lifetime_storage_bytes: number;
};

/** Cached cloud account from the on-disk session, or `null` when signed out.
 *  Cheap; no network. Works from any window (overlay included). */
export function cloudCurrentAccount(): Promise<CloudAccountInfo | null> {
  return invoke<CloudAccountInfo | null>("cloud_current_account");
}

/** Re-fetch `/v1/me` (network), refreshing the cached account. Returns the
 *  fresh account; throws when signed out / offline. Used by the recap to get a
 *  current `lifetime_storage_bytes` rather than a possibly-stale cache. */
export function cloudRefreshAccount(): Promise<CloudAccountInfo> {
  return invoke<CloudAccountInfo>("cloud_refresh_account");
}

export function listPlaytime(): Promise<PlaytimeSummary> {
  return invoke<PlaytimeSummary>("list_playtime");
}

/** A playtime-only game: an always-online title (Fortnite, Rust…) we track
 *  purely for hours played, never for saves. Auto-enrolled from the installed-
 *  game scan; `excluded` is the user's opt-out. */
export type PlaytimeGameInfo = {
  slug: string;
  display_name: string;
  excluded: boolean;
};

/** Installed playtime-only games (Steam + Epic + GOG + MS Store ∩ catalog),
 *  minus those already tracked as real saves, plus any excluded-but-uninstalled
 *  ones so they stay re-enablable. Local-only; no network. */
export function listPlaytimeGames(): Promise<PlaytimeGameInfo[]> {
  return invoke<PlaytimeGameInfo[]>("list_playtime_games");
}

/** Stop counting `slug` toward the recap and detach its live slot. */
export function excludePlaytimeGame(slug: string): Promise<void> {
  return invoke<void>("exclude_playtime_game", { slug });
}

/** Re-allow `slug` for playtime tracking, re-attaching it if installed. */
export function includePlaytimeGame(slug: string): Promise<void> {
  return invoke<void>("include_playtime_game", { slug });
}

/** Push this device's playtime to Hoard Cloud and read back the device-merged
 *  aggregate (the recap's "multi-equipo" source of truth). Same shape as
 *  {@link listPlaytime}; the command falls back to the local summary when
 *  signed out or offline, so this never throws on a missing session. */
export function syncPlaytime(): Promise<PlaytimeSummary> {
  return invoke<PlaytimeSummary>("cloud_sync_playtime");
}

export function catalogStatus(): Promise<CatalogStatus> {
  return invoke<CatalogStatus>("catalog_status");
}

export function updateCatalog(): Promise<CatalogUpdateResult> {
  return invoke<CatalogUpdateResult>("update_catalog");
}
