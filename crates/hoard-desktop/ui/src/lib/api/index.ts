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
  | "both";

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
};

/** Run a full auto-detection sweep. Subscribe to `library://scan-progress`
 * with `listen()` from `@tauri-apps/api/event` to drive a progress bar. */
export function scanLibrary(): Promise<DetectionReport> {
  return invoke<DetectionReport>("scan_library");
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

/** List the saves currently tracked for the logged-in user. */
export function listTrackedSaves(): Promise<TrackedSave[]> {
  return invoke<TrackedSave[]>("list_tracked_saves");
}

/** Stop tracking a save locally. Server-side data is left alone. */
export function untrackSave(save_id: string): Promise<void> {
  return invoke<void>("untrack_save", { saveId: save_id });
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
  | { type: "backup_started"; save_id: string }
  | {
      type: "backup_success";
      save_id: string;
      version_num: number;
      total_bytes: number;
    }
  | {
      type: "backup_failed";
      save_id: string;
      error: string;
      will_retry: boolean;
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
};

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

export function catalogStatus(): Promise<CatalogStatus> {
  return invoke<CatalogStatus>("catalog_status");
}

export function updateCatalog(): Promise<CatalogUpdateResult> {
  return invoke<CatalogUpdateResult>("update_catalog");
}
