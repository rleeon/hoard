/**
 * Typed wrappers around Tauri's `invoke()`.
 *
 * Keeping all `invoke` calls here means commands have one source of truth for
 * their argument and return shapes, components just import a function and
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
   *  "% of quota", for a server you own at home a quota bar is meaningless. */
  is_local_server: boolean;
  /** True when the URL points at the managed Hoard Cloud backend
   *  (`*.hoard.services` / `*.fly.dev`). The cloud upgrades itself and has no
   *  `/v1/admin/upgrade` route, so the UI hides the self-hosted server-upgrade
   *  panel for these connections. */
  is_cloud_server: boolean;
  /** The server's per-snapshot ceiling (`storage.max_snapshot_size_mb`, in
   *  bytes). `null` before the first whoami of the session, and on a server too
   *  old to report it, the account page shows a dash rather than a zero. */
  max_snapshot_size_bytes: number | null;
  /** Stored-version caps, `null` meaning unlimited. Automatic snapshots and
   *  deliberate copies count against separate budgets. */
  max_versions: number | null;
  max_manual_versions: number | null;
};

/** Anonymous probe, used by the wizard to validate the server URL. */
export function healthCheck(url: string): Promise<HealthInfo> {
  return invoke<HealthInfo>("health_check", { url });
}

/** Verify a (URL, token) pair against the server and persist it. */
export function login(url: string, token: string): Promise<UserInfo> {
  return invoke<UserInfo>("login", { url, token });
}

/** Cheap, sync check, does the app have a saved session? */
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

/** Re-fetch quota from the server. Cheap (one round-trip, no body), call
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
   * install directory, that's in `install_dir`. Empty for Steam-only
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
  /**
   * Detection finished without a save folder for this game: `found_paths` is
   * empty and stays empty until someone picks one. The row is still true, the
   * game is installed, but it is not a detected save, and the card answers it
   * with the folder picker. Absent (not `false`) on reports from older builds,
   * where `!found_paths.length` is the same fact.
   */
  needs_folder?: boolean;
  /**
   * The catalog says this game supports Steam Cloud. Purely a note for the
   * user, it must not reorder, re-rank or gate anything. Steam Cloud only
   * covers the Steam copy, can be turned off per game, and keeps no history
   * to roll back to, so wanting a second copy on top of it is normal.
   */
  steam_cloud?: boolean;
};

export type DetectionReport = {
  games: DetectedGame[];
  catalog_size: number;
  steam_apps_found: number;
  scanned_at_ms: number;
  /**
   * Per-stage counters + wall time for the pass (what each pipeline stage
   * contributed). Not rendered anywhere yet, carried for the scan cache
   * and diagnostics. Optional: cached reports from older builds lack it.
   */
  stats?: Record<string, number>;
  /**
   * Saves already tracked whose folder looks like the game's OWN backup
   * mirror, with what looks like the real save sitting next to it.
   *
   * Detection never revisits a folder once it's tracked (`run_scan` skips
   * tracked slugs), so fixing the scoring doesn't repoint anybody: this is
   * the only thing that can tell an affected user their backups are the only
   * thing syncing. Optional, reports cached by older builds lack it.
   */
  mirror_warnings?: MirrorWarning[];
};

/** A tracked folder that looks like a backup mirror, plus the sibling that
 *  looks like the real save. Purely advisory: repointing is a user act. */
export type MirrorWarning = {
  save_id: string;
  game_slug: string;
  label: string;
  tracked_path: string;
  suggested_path: string;
  /** Which evidence fired, the full structural twin, or only the name
   *  relation. Shown so the user can weigh a weaker match. */
  reason: string;
};

export type ScanProgress = {
  done: number;
  total: number;
};

/** One save folder detection found on THIS machine, with that path's own
 *  confidence (not the game's rolled-up grade). */
export type DetectedPath = {
  path: string;
  confidence: Confidence;
};

/** A game detected on THIS machine offered as a link target, so a cloud save
 *  can be bound by picking the *game* instead of hunting for its folder. */
export type LinkCandidate = {
  game_slug: string;
  display_name: string;
  /** Save folders, strongest-first. Never empty. */
  paths: DetectedPath[];
  /** 2 = same normalised name as the cloud slug, 1 = one contains the other,
   *  0 = unrelated. Already sorted on; the UI badges the 2s. */
  affinity: number;
};

/** What local detection knows about one slug: the offer behind the "link to this
 *  machine" dialog. */
export type LocalDetection = {
  game_slug: string;
  /** Candidates, strongest-first. Exactly one means the match is unambiguous
   *  and the orphan card can offer a direct "Vincular a <ruta>" button. */
  paths: DetectedPath[];
  /** Every OTHER game detected here, best name match first, the way out when
   *  the two machines slug the same game differently. */
  candidates: LinkCandidate[];
  /**
   * When this machine last scanned, or `null` if it never did. `null` with
   * empty `paths` means "unknown", not "nothing here", offer a scan instead
   * of only the folder picker, since users who never enabled automatic mode land
   * here with a cold cache.
   */
  scanned_at: string | null;
};

export type TrackedSave = {
  save_id: string;
  game_slug: string;
  label: string;
  /** What the user calls this folder ("Mods", "Ironman"), or `null` if they
   *  never named it. Travels with the row, so both machines show the same name.
   *  Edited through {@link setSaveSlotName}, never by typing the label whole,
   *  the number lives in the same field and free text would wipe it. */
  name: string | null;
  /** Which numbered folder of the title this is: 1 = saved games, 2+ = the
   *  rest (config, mods…), which Hoard carries but never restores on its own.
   *  `null` for the free-form labels rows had before slots existed, those
   *  render with their text as-is. Derived from `label` by the engine so both
   *  frontends agree on what counts as slot 1. */
  slot: number | null;
  local_path: string;
  /** The **server's** head version: the newest version that exists in the
   *  cloud, whoever uploaded it, usually another machine. Never render this
   *  as "saved": with the cloud at v138 and this device pinned at v120 that
   *  label invites the user to play on top of a stale save and push it as
   *  v139, walking the cloud head backwards (ADR 0021 D.10). Pair it with
   *  {@link local_version_num} and say which is which. */
  cloud_version_num: number | null;
  /** The version **this device** is synced to (its local `CliState` cursor,
   *  the same number the sync kernel uses as `known_version`). `null` = this
   *  machine has never uploaded or downloaded this save. */
  local_version_num: number | null;
  last_backup_at: string | null;
  paused: boolean;
  /** Bytes occupied on the server (sum of non-deleted snapshots). */
  total_size_bytes: number;
  /** `true` when the save exists server-side but this machine has no
   *  matching CliState row (reinstall, PC switch, manual state wipe). The
   *  UI shows a discreet "Sin estado local" badge and disables the local
   *  untrack button, only `deleteSaveCompletely` is meaningful here. */
  orphan: boolean;
  /** Bytes this save occupies on THIS machine (its local folder, recursive).
   *  `null` for orphan rows (no local folder here) and freshly-created rows.
   *  Distinct from {@link total_size_bytes} (server-side footprint). */
  local_size_bytes: number | null;
  /** The user's explicitly chosen sync preset, or `null` for "standard /
   *  inherit". A built-in per-game preset may still apply at the agent
   *  level when this is `null`; this only reflects the manual override. */
  preset: string | null;
  /** Whether restoring this game writes its device-local files (`.ini`,
   *  `.cfg`, settings) instead of skipping them. `null` = undecided: they are
   *  not written and the restore dialog keeps asking each time. */
  allow_device_local: boolean | null;
};

/** Run a full auto-detection sweep. Subscribe to `library://scan-progress`
 * with `listen()` from `@tauri-apps/api/event` to drive a progress bar. */
export function scanLibrary(): Promise<DetectionReport> {
  return invoke<DetectionReport>("scan_library");
}

/** Forced re-scan from the Library "Re-escanear" button. Same wire shape as
 *  `scan_library`, distinct command so the backend can disambiguate UI
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

/** Ludusavi-style "add from folder": scan ONE user-chosen folder and return
 *  the games detected inside it. One-off lookup, never touches the catalog/
 *  Steam and never persists into the library cache. Backs the folder-picker
 *  button next to "Manual track". */
export function scanFolder(path: string): Promise<DetectedGame[]> {
  return invoke<DetectedGame[]>("scan_folder", { path });
}

/** Return the previous scan if one is in memory, else null. */
export function cachedDetection(): Promise<DetectionReport | null> {
  return invoke<DetectionReport | null>("cached_detection");
}

/** Save folders local detection already knows for a slug, plus every other
 *  game detected here as a link target. Read from the scan cache, cheap
 *  (in-memory lookup, no scan), safe to call per orphan row.
 *
 *  `tracked_paths` are the folders this machine already tracks; they're
 *  dropped from the candidates so no two saves end up on one folder. */
export function detectedPathsForGame(
  game_slug: string,
  tracked_paths: string[] = [],
): Promise<LocalDetection> {
  return invoke<LocalDetection>("detected_paths_for_game", {
    gameSlug: game_slug,
    trackedPaths: tracked_paths,
  });
}

/** Begin tracking a detected game at the given path.
 *
 *  `display_name` and `steam_app_id` are forwarded to the server so it can
 *  self-heal its games table when the desktop's catalog is fresher than the
 *  server's seed. Older servers ignore the extra fields. */
export function addGameToTracking(args: {
  game_slug: string;
  label?: string;
  /** Which numbered folder of the title this is (1 = saved games). Wins over
   *  `label`, which only survives for the free-form labels of older rows. */
  slot?: number;
  local_path: string;
  display_name?: string;
  steam_app_id?: number | null;
  /** Pin a sync preset (manual emulator adds pass `"backup_only"`). */
  preset?: string;
  /** Pin the process exe names that mark this save as "playing". */
  processes?: string[];
  /** Those exe names are shared with other tracked saves, so seeing one run
   *  doesn't say which of them is being played, one entry per game of an
   *  emulated console. The engine then needs a write in this save's own folder
   *  before it counts as running, instead of marking all of them at once. */
  shared_processes?: boolean;
  /** The user already said yes to *moving* this slot to another folder. Without
   *  it, a slot already pointing somewhere else is an error instead of a silent
   *  overwrite, see `slotOccupied`. */
  repoint?: boolean;
}): Promise<TrackedSave> {
  return invoke<TrackedSave>("add_game_to_tracking", { args });
}

/** What a folder is currently in, when an add lands on an occupied slot.
 *  Parsed out of the engine's `slot_occupied:<label>:<free>:<path>` error. */
export interface SlotOccupied {
  label: string;
  /** Lowest number this title has free, what to offer as "add it as N". */
  free_slot: number;
  /** The folder the slot points at right now. */
  current_path: string;
}

/** Recognise the "that slot already holds another folder" error so the caller
 *  can ask whether to move it or add the folder as a new slot. Returns `null`
 *  for every other failure. */
export function slotOccupied(e: unknown): SlotOccupied | null {
  const msg = typeof e === "string" ? e : ((e as Error)?.message ?? "");
  const m = /^slot_occupied:([^:]*):(\d+):([\s\S]*)$/.exec(msg);
  return m
    ? { label: m[1], free_slot: Number(m[2]), current_path: m[3] }
    : null;
}

/** One curated emulator, with native-save folders resolved for this host. */
export interface EmulatorPreset {
  id: string;
  display_name: string;
  system: string;
  processes: string[];
  /** Existing save folders found on this machine; first is the best default.
   *  May be empty, then the user must pick the folder by hand. */
  save_paths: string[];
  /** True when this emulator's save root can be split into one folder per
   *  game. The dialog then offers picking titles instead of adding the whole
   *  tree, since the tree carries a profile id generated per install, so copying
   *  the whole thing leaves the save hanging off a profile the other machine's
   *  emulator has never heard of. */
  splits_per_title: boolean;
}

/** One game found inside an emulator's save tree. */
export interface EmulatorTitle {
  /** Title id as the folder names it: the one thing both installs call the
   *  same. */
  title_id: string;
  path: string;
}

/** One live process candidate for the emulator picker. */
export interface RunningProcess {
  /** Executable name as the OS reports it (matches `processes` verbatim). */
  name: string;
  /** Peak CPU usage; the list is sorted by it so the active emulator is first. */
  cpu: number;
}

/** Curated emulator catalog (suggested folders + exes) for the "Add emulator"
 *  dialog. Hoard free: the resulting save is tracked via `addGameToTracking`
 *  with `processes`/`preset` pinned, no detection-pipeline changes. */
export function listEmulatorPresets(): Promise<EmulatorPreset[]> {
  return invoke<EmulatorPreset[]>("list_emulator_presets");
}

/** The games inside an emulator's save folder. Empty is not an error: it means
 *  the tree doesn't have the expected shape and the caller should keep
 *  offering the root as it always did. */
export function listEmulatorTitles(
  emulatorId: string,
  root: string,
): Promise<EmulatorTitle[]> {
  return invoke<EmulatorTitle[]>("list_emulator_titles", { emulatorId, root });
}

/** Live snapshot of game-like processes, sorted by CPU, for the process
 *  picker: open the emulator, refresh, pick the top entry. */
export function listRunningProcesses(): Promise<RunningProcess[]> {
  return invoke<RunningProcess[]>("list_running_processes");
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
 *  instead of bouncing off the 409-recovery path. Destructive, the UI must
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

/** Persistently blacklist a slug: it stops appearing in the Library grid
 *  **and** any save tracked under it stops being watched on this machine
 *  (snapshots on the server are kept). Resolves to how many tracked saves
 *  that dropped. Reversible via {@link unignoreDetectedGame}; the matching
 *  Settings list renders every currently-ignored slug. */
export async function ignoreDetectedGame(slug: string): Promise<number> {
  return await invoke<number>("ignore_detected_game", { slug });
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
 *  5 clicks on the sidebar version. Read-only, never writes to the
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
  /** Has anything actually told us this? The store starts at `running: false`
   *  before the first status arrives, and a banner keyed on `!running` alone
   *  reads that blank as "the service is stopped" and says so, while the app
   *  is still opening. Set by the store, never by the service. */
  known?: boolean;
  /** The sync service sends the native OS notifications itself (ADR 0021
   *  D.14.1), so this app must not send its own or the user sees each one
   *  twice while the window is open. `false`, including on an older service
   *  that doesn't report the field, and on the platforms whose service-side
   *  backend isn't wired yet (Windows, macOS), means the notification is
   *  still ours to send, exactly as before. */
  service_notifies?: boolean;
  /** Why there's no engine, when there isn't one. Until 1.1.0 the window only
   *  knew *that* the service was down, never why: the reason existed inside the
   *  daemon and was dropped on the way here, which is how two self-hosted users
   *  went days without backups with nothing to report but "it says offline".
   *  Absent (older service) means unknown, the banner falls back to the
   *  generic line. */
  reason?: EngineDownReason;
  /** Raw text of the last start failure, for the detail line and for the user
   *  to paste into a report. The translated sentence comes from `reason`. */
  last_error?: string | null;
  /** Which way the keyring failed, when `reason` is `keyring_unreadable`. One
   *  reason, four next steps: a machine with no secret-service daemon is not a
   *  locked one, and telling that user to unlock their login keyring sends them
   *  after something that isn't installed. Absent on an older service, and then
   *  the general keyring sentence is what shows, exactly as before. */
  keyring?: KeyringFault | null;
};

/** Mirrors `hoard_core::ipc::KeyringFault` (snake_case on the wire). */
export type KeyringFault =
  | "missing"
  | "locked"
  | "refused"
  | "damaged"
  | "unknown";

/** Mirrors `hoard_core::ipc::EngineDownReason` (snake_case on the wire). */
export type EngineDownReason =
  | "unknown"
  | "no_session"
  | "keyring_unreadable"
  | "session_expired"
  /** The status read itself failed, so nothing is known, not even whether the
   *  engine is still up. Filled in by this app, never sent by the service. */
  | "unreachable"
  | "other";

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
      /**
       * Nothing was uploaded: the content was already the server's head (ADR
       * 0021 D.8.3, after the service restarted with an upload in flight that did
       * commit). The fact, "it is saved in version N", is the same, but
       * `total_bytes` is 0 because not one byte travelled. Optional: an older
       * service does not send it.
       */
      already_landed?: boolean;
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
      type: "backup_too_large";
      save_id: string;
      game_slug: string;
      label: string;
      /** Who refused it, and therefore what the user has to change. */
      kind: "plan_cap" | "server_limit" | "proxy";
      plan: string;
      limit_bytes: number;
      actual_bytes: number;
      /** Self-hosted only: bytes sent before the server stopped. A floor. */
      received_bytes: number;
    }
  | {
      /** Account-wide: the plan's storage is full, so no save can upload. */
      type: "backup_quota_full";
      save_id: string;
      game_slug: string;
      label: string;
      plan: string;
      used_bytes: number;
      limit_bytes: number;
    }
  | {
      type: "backup_trimmed";
      save_id: string;
      game_slug: string;
      label: string;
      kept_files: number;
      omitted_files: number;
      omitted_bytes: number;
      plan: string;
      limit_bytes: number;
    }
  | {
      /** The snapshot went up without files whose bytes couldn't be read,
       *  a partial version, said out loud. `uploaded: false` means not one
       *  file was readable, so nothing was backed up at all. */
      type: "backup_files_unreadable";
      save_id: string;
      game_slug: string;
      label: string;
      count: number;
      kept_files: number;
      sample_path: string;
      sample_error: string;
      uploaded: boolean;
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
      /** The save has never produced a snapshot AND its folder is empty,
       *  almost always a wrong tracked path rather than a real state change.
       *  See `AgentEvent::BackupSkippedEmpty` for why the two cases differ. */
      likely_wrong_path?: boolean;
    }
  | {
      /** Auto-restore has failed repeatedly on the same cloud version: the
       *  save is not syncing and won't fix itself. Unlike
       *  `save_auto_restore_failed` (transient, one per attempt), this is a
       *  persistent state the Library card shows until it recovers. */
      type: "save_auto_restore_stuck";
      save_id: string;
      game_slug: string;
      failures: number;
      error: string;
    }
  | {
      /** The stuck save restored successfully (or the cloud moved to a new
       *  version): drop the persistent warning. */
      type: "save_auto_restore_recovered";
      save_id: string;
      game_slug: string;
    }
  | {
      /** The upload keeps hitting a conflict it can't resolve, so it has
       *  STOPPED retrying and needs a person. Persistent state on the save's
       *  card until `backup_attention_cleared`. */
      type: "backup_needs_attention";
      save_id: string;
      game_slug: string;
      label: string;
      conflicts: number;
      error: string;
    }
  | {
      /** The blocked save is uploading again: drop the warning. */
      type: "backup_attention_cleared";
      save_id: string;
      game_slug: string;
    };

/** Ensure the sync service is up and report its engine status. */
export function startAgent(): Promise<AgentStatus> {
  return invoke<AgentStatus>("start_agent");
}

/** Cleanly stop the agent (logout, app exit). */
export function stopAgent(): Promise<void> {
  return invoke<void>("stop_agent");
}

/** Start relaying the sync service's events onto the `agent://*` channels.
 *
 *  Called by the agent store once its `listen()`s are registered, deliberately
 *  *not* folded into `startAgent`, which Rust background work also calls and
 *  which can therefore run before the webview has mounted. A journal replayed
 *  into a page with no listeners would be a history lost in silence. */
export function attachAgentEvents(): Promise<void> {
  return invoke<void>("attach_agent_events");
}

/** Stop the relay (the service keeps running, it owns the sync engine). */
export function detachAgentEvents(): Promise<void> {
  return invoke<void>("detach_agent_events");
}

/** One row of the sync service's journal, as this process relayed it.
 *  `seq` identifies the row within a run of the daemon, a stable key for any
 *  surface that re-reads the whole snapshot instead of stitching events. */
export type JournalRow = { seq: number; at: number; event: AgentEvent };

/** Cloud-loop state. Same vocabulary as `CloudStatus` in `stores/live.ts`. */
export type CloudPulse = "unknown" | "online" | "offline" | "throttled";

/** Everything this process already knows, copied. See {@link agentSnapshot}. */
export type UiSnapshot = {
  status: AgentStatus;
  /** What the service says about each watched save: is the game running, and
   *  when is its next backup due. Kept apart from the journal on purpose,
   *  those are *state*, and rebuilding state by replaying events means keeping
   *  the `game_started` row forever or lying about who's playing. */
  slots: AgentSlotStatus[];
  /** Oldest first, like the backlog. */
  rows: JournalRow[];
  cloud: CloudPulse;
  cloud_retry_in: number | null;
};

/** Read the current state instead of subscribing to it.
 *
 *  Subscribing only works for whoever was there at boot: the backlog is emitted
 *  once, `attachAgentEvents` is idempotent, and the daemon status is only
 *  re-emitted when it changes. A window created later, the in-game HUD, can
 *  have every listener correctly registered and still never receive a line.
 *
 *  So it reads. The call touches nothing but three in-memory mutexes on the Rust
 *  side: no I/O, no network, no request to the service, and above all nothing
 *  that could *start* anything. Opening a window to look at the state must not
 *  change it. */
export function agentSnapshot(): Promise<UiSnapshot> {
  return invoke<UiSnapshot>("agent_snapshot");
}

/** Force a backup right now, bypassing debounce. */
export function backupNow(save_id: string): Promise<void> {
  return invoke<void>("backup_now", { saveId: save_id });
}

/** Kick a staggered backup sweep across every tracked save (automatic mode's
 *  hourly hash pass). The agent spreads each save's re-hash across an
 *  effective window so disk use doesn't burst, see `sweep_backups` /
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
  /** When `true`, this machine ships its playtime breakdown so Wrapple can
   *  show real hours merged across the account's devices. Off means nothing
   *  leaves the machine and the recap has nothing to read, the local store
   *  keeps accruing, so turning it back on restores the history. Deliberately
   *  independent of `anonymous_telemetry`, whose consent copy promises never
   *  to send game names. */
  wrapple_telemetry: boolean;
  /** ISO-639 code for the desktop UI language, e.g. "en", "fr". `null` means
   *  the user hasn't picked one yet, we then fall back to the browser
   *  language at boot. */
  language: string | null;
  /** When `true`, the agent restores the latest server snapshot into a
   *  tracked save's local path whenever that path is missing or empty on
   *  add. Off by default, silent writes under `~` are the kind of thing
   *  that earns trust slowly, so users have to opt in. */
  auto_restore: boolean;
  /** "Sync global", distinct from both `auto_restore` and `automatic_mode`.
   *  When `true`, the agent downloads a newer cloud version the moment it
   *  detects the device is outdated, even while a game is running or the save
   *  was just written. Version-gated (never re-pulls a version already held)
   *  and non-destructive (local-newer files are parked under conflicts).
   *  Off by default. */
  global_sync: boolean;
  /** Last desktop-client version we already fired a native notification
   *  about. The update poller compares this against the latest report and
   *  only sends a notification the first time it sees a new version, the
   *  sidebar amber badge keeps showing regardless. Persisted so reopening
   *  the app doesn't re-notify for a version the user already saw. */
  last_update_notified_version: string | null;
  /** When `true`, the sidebar's automatic-mode toggle is on. The Rust
   *  side keeps two background schedulers alive, a cheap detection scan
   *  (`automatic_scan_interval_secs`) and an expensive staggered hash sweep
   *  (`automatic_backup_interval_secs`); activating the toggle also cascades
   *  `auto_restore = true`. Off by default. */
  automatic_mode: boolean;
  /** Seconds between background detection scans while `automatic_mode` is on.
   *  The scan is the cheap, metadata-only half (no file bytes read), so it
   *  runs often, default 300s (5 min). Replaces the pre-1.9.14
   *  `automatic_scan_interval_hours`. */
  automatic_scan_interval_secs: number;
  /** Seconds between background backup (hash) sweeps while `automatic_mode`
   *  is on. The sweep re-hashes save bytes to catch missed changes, so it's
   *  the expensive half and runs rarely, default 3600s (1h). The agent
   *  staggers per-save work across an effective window that grows with the
   *  total footprint, so this is the nominal cadence, not a hard ceiling. */
  automatic_backup_interval_secs: number;
  /** Days to retain per-save conflict backups under
   *  `<state_dir>/conflicts/<save_id>/<rfc3339>/`. Defaults to 14;
   *  validated on the Rust side to 1..=30. */
  conflict_retention_days: number;
  /** DEAD CODE, reserved for possible future use (2026-07-04).
   *  Was the global "Modo ahorro (solo subida)" toggle: `true` would default
   *  every new cloud upload to `backup_only` (uploads but hidden from other
   *  devices' manifest pull). The toggle was removed from the UI because it
   *  confused users, and the flag was never actually consumed by the agent.
   *  Kept in the struct so the pref file stays stable; do not surface it. */
  cloud_savings_mode: boolean;
  /** Whether the floating ActivityFeed panel is visible. Defaults to
   *  true; the user can hide it from the sidebar toggle. */
  live_activity_visible: boolean;
  /** The data-saving knob `k` in `[0,1]` (ADR 0018). 0 is "keep everything" (an
   *  aggressive cadence, long retention); 1 is "maximum saving" (a minimum
   *  interval of up to 10 min between snapshots, aggressive retention). It scales
   *  both the client's min-snapshot-interval and the server's retention policy.
   *  Default 0.3. */
  data_saving: number;
};

/** The single user-facing operating mode. Mirrors `hoard_agent::prefs::SyncMode`.
 *  Derived from / applied onto the internal `global_sync` + `auto_restore`
 *  flags, the UI only ever shows this binary choice, never the two toggles. */
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

/** Tells the backend the UI has painted its first frame so it can show the window
 *  (it is born hidden; see `commands/window.rs`). Idempotent, with a safety net on
 *  the Rust side, so nothing breaks if it is late or if the start is silent, in
 *  which case the backend ignores it. */
export function uiReady(): Promise<void> {
  return invoke<void>("ui_ready");
}

/** Persist prefs. Returns the saved object so the caller can hydrate stores. */
export function savePrefs(prefs: Prefs): Promise<Prefs> {
  return invoke<Prefs>("save_prefs", { prefs });
}

/** Toggle the sidebar's persisted automatic-mode flag. Returns the
 *  full updated prefs so the caller can hydrate every dependent store with
 *  the cascaded value (activation also flips `auto_restore` to true). The
 *  Rust side also starts or stops the background scheduler as part of the
 *  call. */
export function setAutomaticMode(enabled: boolean): Promise<Prefs> {
  return invoke<Prefs>("set_automatic_mode", { enabled });
}

/** Flip "Sync" (global sync). Distinct from automatic mode: it doesn't start
 *  any scheduler and doesn't cascade `auto_restore`. When on, the agent
 *  downloads a newer cloud version the moment it detects the device is
 *  outdated, even while a game is running. Returns the updated prefs. */
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

/** Persist a new detection-scan interval (seconds, 60..=3600) for automatic
 *  mode. If the toggle is on, the schedulers restart so the new
 *  cadence applies immediately and a scan fires right away. */
export function setScanInterval(secs: number): Promise<Prefs> {
  return invoke<Prefs>("set_scan_interval", { secs });
}

/** Persist a new backup-sweep interval (seconds, 300..=86400) for automatic
 *  mode. The agent staggers per-save work across an effective window
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

/** Why the sync *service* can't start at login, when it can't. The app's own
 *  launcher entry and the service are two separate registrations since the
 *  engine moved out of the window (ADR 0021, Slice 4), and only the first one
 *  works everywhere: an AppImage runs from a mount that's gone by the next
 *  login, and a machine without systemd has nothing to declare a unit to.
 *  Mirrors `commands::prefs::ServiceAutostart`. */
export type ServiceAutostart = {
  enabled: boolean;
  /** Which manager took it: "systemd --user", "Task Scheduler", "Startup entry
   *  (HKCU Run)". On Windows those last two are genuinely different outcomes,
   *  the Run entry is the fallback when the task needs an elevated console. */
  manager?: string | null;
  unit?: string | null;
  /** Typed reason, or absent when login start is registered (or off on
   *  purpose). The sentence shown comes from i18n keyed on this. */
  unsupported?: ServiceAutostartBlock | null;
  /** Raw failure text for the detail line and for a bug report. */
  detail?: string | null;
};

export type ServiceAutostartBlock = "no_stable_path" | "no_service_manager";

/** Whether the sync service starts at login, plus the last attempt's reason if
 *  it can't. `enabled` is probed from the service manager, not remembered:
 *  `hoard sync autostart` moves the same switch from a terminal. */
export function serviceAutostartState(): Promise<ServiceAutostart> {
  return invoke<ServiceAutostart>("service_autostart_state");
}

/** Register the sync service for login start (and start it now), or take it out
 *  of login start. Turning it off leaves a running sync alone; stopping it now
 *  is `hoard sync stop`. The twin of `hoard sync autostart on|off`. */
export function setServiceAutostart(
  enabled: boolean,
): Promise<ServiceAutostart> {
  return invoke<ServiceAutostart>("set_service_autostart", { enabled });
}

/** Recolour the tray icon. The frontend derives the global state from the
 * activity store and pushes it here. */
export function setTrayState(state: TrayStateName): Promise<void> {
  return invoke<void>("set_tray_state", { state });
}

// ---------------------------------------------------------------------------
// Snapshot history, restore, manual override, logs
// ---------------------------------------------------------------------------

/** One labelled fact about a version. `kind` picks the formatting so a single
 *  renderer can draw every game, a per-game probe adds fields, never a
 *  component. */
export type InsightField = {
  kind: "text" | "number" | "duration" | "date" | "money" | "badge";
  label: string;
  value: string;
};

/** What a version is *about*, derived by the server from the version's own file
 *  manifest: which save moved, how much changed, how many saves the folder
 *  holds. Absent on versions uploaded before the server derived any of this,
 *  and on legacy whole-archive versions with no per-file manifest, those rows
 *  render exactly as they always did.
 *
 *  Field names are one or two letters because this is stored once per version;
 *  the shape is `hoard_core::kernel::insight::VersionInsight`. */
export type VersionInsight = {
  /** Schema version. Unknown values render what we recognise and ignore the
   *  rest. */
  v: number;
  /** The save's display name. */
  t?: string;
  /** Free line under the title; only a per-game probe sets it. */
  s?: string;
  /** Manifest path of the file the row is about. */
  p?: string;
  /** Distinct saves in the folder, worlds, characters, slots. */
  n?: number;
  /** Files added or rewritten since the previous version. */
  c?: number;
  /** Files the previous version had and this one doesn't. */
  r?: number;
  /** Signed size delta against the previous version, in bytes. */
  d?: number;
  /** sha256 of the thumbnail blob, once there are thumbnails. */
  th?: string;
  f?: InsightField[];
  /** `generic`, or the name of the probe that filled this in. */
  src: string;
};

export type SnapshotEntry = {
  version_num: number;
  file_count: number;
  total_size_bytes: number;
  is_pinned: boolean;
  /** Which machine this version came from. `null` for anything uploaded before
   *  the server started recording it, the timeline drops the suffix rather
   *  than naming a PC it doesn't know. */
  device_name: string | null;
  created_at: string;
  deleted_at: string | null;
  insight: VersionInsight | null;
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

/** Per-user cap on stored versions per save. `null` = unlimited.
 *
 *  `manual` picks which budget: the copies the user asked for (and the safety
 *  copy taken before a restore) or the automatic ones. They are counted
 *  separately so a game that autosaves every minute can't fill the history and
 *  evict the copy someone made on purpose before a boss. */
export function getMaxVersions(manual = false): Promise<number | null> {
  return invoke<number | null>("get_max_versions", { manual });
}

/** Dry-run: how many stored versions a cap of `maxVersions` would delete
 *  right now. Nothing is written, used for the confirmation dialog. */
export function previewMaxVersions(
  maxVersions: number,
  manual = false,
): Promise<number> {
  return invoke<number>("preview_max_versions", { maxVersions, manual });
}

/** Set (or clear, with `null`) the max-versions cap. The server prunes the
 *  excess immediately, so refresh History / quota afterwards. */
export function setMaxVersions(
  maxVersions: number | null,
  manual = false,
): Promise<void> {
  return invoke<void>("set_max_versions", { maxVersions, manual });
}

/** What restoring this version will do to the folder, before confirming it.
 *  Downloads nothing: crosses the version's manifest with what's on disk.
 *
 *  `comparable: false` means the version publishes no per-file hashes (the
 *  legacy whole-archive ones), so `modified` can't be told from `unchanged`
 *  and the UI must say it can't preview rather than show an empty diff. */
export type RestorePreview = {
  unchanged: number;
  /** Capped at 200 entries, count with `modified_count`, never `.length`. */
  modified: string[];
  added: string[];
  local_only: string[];
  /** Real totals, whatever the lists above had room for. */
  modified_count: number;
  added_count: number;
  local_only_count: number;
  bytes_to_write: number;
  comparable: boolean;
};

export function previewRestore(
  saveId: string,
  version: number,
  destinationOverride?: string | null,
  allowConfig = false,
): Promise<RestorePreview> {
  return invoke<RestorePreview>("preview_restore", {
    saveId,
    version,
    destinationOverride: destinationOverride ?? null,
    allowConfig,
  });
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
  /** Also write the snapshot's config files (.ini, .cfg, settings) over this
   *  machine's. Off by default: they carry the resolution, the GPU and the paths
   *  of the PC that uploaded the copy, and the game breaks on them. */
  allow_config?: boolean;
}): Promise<RestoreOutcome> {
  return invoke<RestoreOutcome>("restore_snapshot", {
    saveId: args.save_id,
    version: args.version,
    backupFirst: args.backup_first,
    destinationOverride: args.destination_override ?? null,
    allowConfig: args.allow_config ?? false,
  });
}

/** Sentinel error string from the Rust side meaning "we have no local path
 *  for this save, prompt the user to pick one and retry with
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

/** Decide whether restoring this game writes its device-local files (`.ini`,
 *  `.cfg`, settings) or skips them. `null` goes back to undecided: not
 *  written, and the restore dialog keeps asking each time. */
export function setSaveAllowConfig(
  saveId: string,
  allow: boolean | null,
): Promise<void> {
  return invoke<void>("set_save_allow_config", { saveId, allow });
}

/** Name a folder without touching its number. `null` clears the name. */
export function setSaveSlotName(
  saveId: string,
  name: string | null,
): Promise<TrackedSave> {
  return invoke<TrackedSave>("set_save_slot_name", { saveId, name });
}

/** Move a folder to another number, keeping its name. Rejects with
 *  `slot_taken:<n>` when the cloud already holds that number, see
 *  {@link slotTaken}. */
export function renumberSaveSlot(
  saveId: string,
  slot: number,
): Promise<TrackedSave> {
  return invoke<TrackedSave>("renumber_save_slot", { saveId, slot });
}

/** The number a renumber asked for, when it is already in use. Linking to that
 *  row is what pairs the machines; renaming into it would only collide. */
export function slotTaken(e: unknown): number | null {
  const msg = typeof e === "string" ? e : ((e as Error)?.message ?? "");
  const m = /^slot_taken:(\d+)$/.exec(msg);
  return m ? Number(m[1]) : null;
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
 * - `games`              , number of games in the currently-loaded catalog.
 * - `has_runtime_override`, whether a refreshed copy is on disk; `false`
 *                            means we're on the version that shipped with the app.
 * - `updated_at`         , Unix epoch seconds of the last successful refresh.
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
 *  day. Empty until a tracked game has been observed running. Local-only,
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
   *  session is stale, callers fall back to the current footprint. */
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

/** Push this device's playtime to the server (Hoard Cloud, or the user's own
 *  server when self-hosted) and read back the device-merged aggregate, the
 *  recap's "multi-equipo" source of truth, read from the server ONLY. Same
 *  shape as {@link listPlaytime}; returns an empty summary (never the local
 *  store) when there's no session or the server is unreachable. */
export function syncPlaytime(): Promise<PlaytimeSummary> {
  return invoke<PlaytimeSummary>("cloud_sync_playtime");
}

export function catalogStatus(): Promise<CatalogStatus> {
  return invoke<CatalogStatus>("catalog_status");
}

export function updateCatalog(): Promise<CatalogUpdateResult> {
  return invoke<CatalogUpdateResult>("update_catalog");
}
