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
  found_paths: string[];
  confidence: Confidence;
  source: DetectionSource;
  steam_app_id: number | null;
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

/** Begin tracking a detected game at the given path. */
export function addGameToTracking(args: {
  game_slug: string;
  label?: string;
  local_path: string;
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
