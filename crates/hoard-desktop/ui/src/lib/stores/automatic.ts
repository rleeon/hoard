/**
 * "Auto-do-everything" workflow backing the sidebar's "Modo Automático" toggle.
 *
 * Calls scan_library, picks every detected game with confidence ≥ "high"
 * AND a non-empty `found_paths`, tracks each one, then starts the agent.
 * Reports progress through `automaticState` so the sidebar can render
 * "Detecting…", "Tracking 3/12…", "Done" without each component having to
 * wire up its own promise chain.
 *
 * The toggle is intentionally limited to "high" confidence picks. Medium /
 * low matches are common false positives (the heuristic finds save-like
 * directories that aren't actually game saves), and silently committing
 * them would make Hoard upload garbage that the user then has to clean up
 * one save at a time. Keeping the bar high means the magic flow is the
 * "lazy but safe" path; the Library page is where you go to handle the
 * tail.
 *
 * Renamed from the legacy auto-setup store in 1.5.3 — the one-shot button
 * became a persisted on/off toggle ("Modo Automático") with background
 * schedulers. Since 1.9.14 the Rust side runs two independent tickers:
 * `automatic-scan-tick` (cheap detection, ~5 min) drives this flow, and
 * `automatic-backup-tick` (expensive hash sweep, ~1 h) drives the staggered
 * `runBackupSweep()`. The listeners are registered by
 * `initAutomaticListener()`.
 */
import { writable, get, type Writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import * as api from "../api";
import { _ } from "svelte-i18n";
import { toastError, toastSuccess, toastInfo } from "./toasts";
import { bootAgent } from "./agent";

export type AutomaticPhase =
  | { kind: "idle" }
  | { kind: "detecting" }
  | { kind: "tracking"; done: number; total: number }
  | { kind: "starting_agent" }
  | { kind: "syncing" };

export const automaticState: Writable<AutomaticPhase> = writable({
  kind: "idle",
});

/**
 * Translation helper local to this module so we don't have to repeat the
 * `get(_)` ceremony inline. `svelte-i18n`'s `_` is itself a store; calling
 * it like a function requires unwrapping the value first.
 */
type InterpolationValue = string | number | boolean | Date | null | undefined;
function tr(
  key: string,
  values?: Record<string, InterpolationValue>,
): string {
  const fn = get(_);
  return fn(key, values ? { values } : undefined);
}

/**
 * Catch-up backup pass, driven by `automatic-backup-tick` (the hourly half
 * of Modo Automático). Hands the whole sweep to the agent in one call: it
 * re-hashes every tracked save to catch changes the fs-watcher missed, but
 * staggers the per-save work across an effective window so disk I/O doesn't
 * burst — see `sweep_backups` / `AgentCommand::SweepAll` on the Rust side.
 *
 * This replaces the old "loop `backupNow` over every save" sweep, which
 * fired all backups near-simultaneously (each `backupNow` only *schedules*
 * a zero-delay task, so awaiting it didn't space them out). The agent owns
 * the timing now, so it keeps spreading the load even while the UI window is
 * closed.
 */
async function runBackupSweep(): Promise<void> {
  try {
    await api.sweepBackups();
  } catch (e) {
    console.warn("automatic-backup-tick: sweep failed:", e);
  }
}

/** Run the detection + tracking flow, driven by `automatic-scan-tick` (the
 *  cheap, frequent half of Modo Automático). Resolves to the count of
 *  newly-tracked games. Surfaces user-friendly toasts; failures fall through
 *  to the toast and resolve to 0 instead of throwing — the sidebar's toggle
 *  is not a place where we want unhandled rejections crashing the UI.
 *
 *  Backups are no longer part of this flow: the staggered sweep runs on its
 *  own `automatic-backup-tick` cadence. We still boot the agent here so any
 *  newly-tracked save starts being watched (and live-backed-up) immediately. */
export async function runAutomaticSetup(): Promise<number> {
  if (get(automaticState).kind !== "idle") return 0;

  try {
    automaticState.set({ kind: "detecting" });
    const report = await api.scanLibrary();

    // Already-tracked games are filtered server-side by `list_tracked_saves`,
    // but a fresh scan returns *every* match — including ones the user has
    // already added. We avoid double-tracking by intersecting with the
    // current tracked list.
    const tracked = await api.listTrackedSaves();
    const trackedSlugs = new Set(tracked.map((t) => t.game_slug));

    const candidates = report.games.filter(
      (g) =>
        g.confidence === "high" &&
        g.found_paths.length > 0 &&
        !trackedSlugs.has(g.slug),
    );

    if (candidates.length === 0) {
      // Still try to start the agent — the user may have tracked games
      // from a previous session that aren't running yet. The backup sweep
      // runs on its own `automatic-backup-tick`, so we don't trigger it here.
      await bootAgent().catch(() => {});
      automaticState.set({ kind: "idle" });
      toastInfo(tr("automatic.nothing_new"));
      return 0;
    }

    let trackedCount = 0;
    for (const [i, game] of candidates.entries()) {
      automaticState.set({
        kind: "tracking",
        done: i,
        total: candidates.length,
      });
      try {
        await api.addGameToTracking({
          game_slug: game.slug,
          local_path: game.found_paths[0],
          display_name: game.display_name,
          steam_app_id: game.steam_app_id,
        });
        trackedCount += 1;
      } catch (e) {
        // Don't abort the whole batch over one game — Pragmata might 422
        // on an old server while the next ten games go through fine.
        console.warn(`automatic-setup: couldn't track ${game.slug}:`, e);
      }
    }

    automaticState.set({ kind: "starting_agent" });
    await bootAgent().catch(() => {});

    automaticState.set({ kind: "idle" });
    toastSuccess(
      tr("automatic.tracked_summary", {
        count: trackedCount,
      }),
    );
    return trackedCount;
  } catch (e) {
    automaticState.set({ kind: "idle" });
    toastError(typeof e === "string" ? e : (e as Error).message);
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Tauri event listener
// ---------------------------------------------------------------------------

let listenerInstalled = false;
let unlisteners: UnlistenFn[] = [];

/** Payload of `agent://save-conflicts-backed-up`. Emitted when the agent's
 *  diff-based restore detects that some local files were older than the
 *  cloud copy and stashed them under `<state_dir>/conflicts/<save_id>/<ts>/`
 *  before overwriting them. The toast points the user at the folder so they
 *  can recover the local version if it turns out to be the canonical one. */
type SaveConflictsBackedUp = {
  save_id: string;
  game_slug: string;
  count: number;
  conflict_dir: string;
};

/**
 * Subscribe to the two Modo Automático scheduler events so the Rust side can
 * drive periodic work. Idempotent — calling twice does nothing on the second
 * call. Designed to be invoked once from `App.svelte::onMount` and never torn
 * down (the listeners cost nothing when no events fire).
 *
 * * `automatic-scan-tick` (cheap, ~5 min) → `runAutomaticSetup()`: detect +
 *   track newly installed games.
 * * `automatic-backup-tick` (expensive, ~1 h) → `runBackupSweep()`: the
 *   agent-staggered hash pass. No toast — it's a quiet background sweep whose
 *   uploads already surface through the LiveStatus / ActivityFeed events.
 *
 * Also installs the `agent://save-conflicts-backed-up` listener — same
 * lifetime, same install-once contract. We keep all subscriptions in the
 * same `unlisteners[]` so `disposeAutomaticListener()` tears them down
 * together.
 */
export function initAutomaticListener(): void {
  if (listenerInstalled) return;
  listenerInstalled = true;
  Promise.all([
    listen("automatic-scan-tick", () => {
      // Show a discreet info toast so the user notices something happened
      // when the scheduler fires on its own. `runAutomaticSetup()` may also
      // emit its own toasts; that's fine — the two read as a small "scan
      // started, here's the result" pair.
      toastInfo(tr("automatic.scanning"));
      void runAutomaticSetup();
    }),
    listen("automatic-backup-tick", () => {
      void runBackupSweep();
    }),
    listen<SaveConflictsBackedUp>(
      "agent://save-conflicts-backed-up",
      (event) => {
        const { count, conflict_dir } = event.payload;
        // No warning level in the toast store, so this surfaces as info.
        // The retention slider in Settings explains the lifetime; the
        // toast body cites the absolute path so the user can paste it
        // into their file manager.
        toastInfo(
          tr("automatic.conflicts_backed_up", {
            count,
            dir: conflict_dir,
          }),
        );
      },
    ),
  ])
    .then((subs) => {
      unlisteners = subs;
    })
    .catch((e) => {
      console.warn("automatic listener install failed:", e);
      listenerInstalled = false;
    });
}

/** Test-only helper to undo `initAutomaticListener`. Not used in app code. */
export function disposeAutomaticListener(): void {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
      /* ignore */
    }
  }
  unlisteners = [];
  listenerInstalled = false;
}
