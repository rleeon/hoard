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
 * became a persisted on/off toggle ("Modo Automático") with a background
 * scheduler. The Rust side emits an `automatic-tick` event every interval
 * and the listener registered by `initAutomaticListener()` runs this flow
 * in response.
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
 * Catch-up backup pass: for every tracked save, ask the agent for an
 * explicit backup (bypassing the fs-watcher debounce). This is the fix
 * for the user-reported "noto que las cosas no se copian solas" bug —
 * the watcher can miss events while the desktop app is closed or the
 * agent isn't booted yet, leaving the local save newer than the last
 * remote snapshot. Running this on every `automatic-tick` guarantees a
 * periodic upload regardless of watcher state.
 *
 * `backupNow` is the same Tauri command the dashboard's "Subir" button
 * uses; the agent dedupes if a backup is already pending for the slot
 * (`schedule_backup` aborts the previous pending task), so a sweep
 * across N saves doesn't fan out into N concurrent backups for the
 * same save.
 */
async function runBackupStaleSweep(
  saves: { save_id: string }[],
): Promise<number> {
  if (saves.length === 0) return 0;
  automaticState.set({ kind: "syncing" });
  let synced = 0;
  for (const save of saves) {
    try {
      await api.backupNow(save.save_id);
      synced += 1;
    } catch (e) {
      console.warn(
        `automatic-tick: backup-stale failed for ${save.save_id}:`,
        e,
      );
    }
  }
  return synced;
}

/** Run the full auto-setup flow. Resolves to the count of newly-tracked
 *  games. Surfaces user-friendly toasts; failures fall through to the
 *  toast and resolve to 0 instead of throwing — the sidebar's toggle is
 *  not a place where we want unhandled rejections crashing the UI. */
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
      // from a previous session that aren't running yet.
      await bootAgent().catch(() => {});
      await runBackupStaleSweep(tracked);
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

    // Catch-up sweep: every tracked save (including the ones we just
    // added) gets an explicit backup request. The agent's debounce/abort
    // logic in `schedule_backup` dedupes if a backup is already pending,
    // so we don't spam concurrent uploads for the same save.
    const refreshed = await api.listTrackedSaves().catch(() => tracked);
    await runBackupStaleSweep(refreshed);

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
let unlisten: UnlistenFn | null = null;

/**
 * Subscribe to the `automatic-tick` Tauri event so the Rust-side scheduler
 * can drive periodic re-runs of `runAutomaticSetup()`. Idempotent — calling
 * twice does nothing on the second call. Designed to be invoked once from
 * `App.svelte::onMount` and never torn down (the listener costs nothing
 * when no events fire).
 */
export function initAutomaticListener(): void {
  if (listenerInstalled) return;
  listenerInstalled = true;
  listen("automatic-tick", () => {
    // Show a discreet info toast so the user notices something happened
    // when the scheduler fires on its own. `runAutomaticSetup()` may also
    // emit its own toasts; that's fine — the two read as a small "scan
    // started, here's the result" pair.
    toastInfo(tr("automatic.scanning"));
    void runAutomaticSetup();
  })
    .then((u) => {
      unlisten = u;
    })
    .catch((e) => {
      console.warn("automatic-tick listener install failed:", e);
      listenerInstalled = false;
    });
}

/** Test-only helper to undo `initAutomaticListener`. Not used in app code. */
export function disposeAutomaticListener(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  listenerInstalled = false;
}
