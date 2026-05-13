/**
 * "Auto-do-everything" workflow used by the sidebar's Magic button.
 *
 * Calls scan_library, picks every detected game with confidence ≥ "high"
 * AND a non-empty `found_paths`, tracks each one, then starts the agent.
 * Reports progress through `magicState` so the sidebar can render
 * "Detecting…", "Tracking 3/12…", "Done" without each component having
 * to wire up its own promise chain.
 *
 * The button is intentionally limited to "high" confidence picks. Medium
 * / low matches are common false positives (the heuristic finds save-like
 * directories that aren't actually game saves), and silently committing
 * them would make Hoard upload garbage that the user then has to clean up
 * one save at a time. Keeping the bar high means the magic button is the
 * "lazy but safe" path; the Library page is where you go to handle the
 * tail.
 */
import { writable, get, type Writable } from "svelte/store";
import * as api from "../api";
import { toastError, toastSuccess, toastInfo } from "./toasts";
import { bootAgent } from "./agent";

export type MagicPhase =
  | { kind: "idle" }
  | { kind: "detecting" }
  | { kind: "tracking"; done: number; total: number }
  | { kind: "starting_agent" };

export const magicState: Writable<MagicPhase> = writable({ kind: "idle" });

/** Run the full auto-setup flow. Resolves to the count of newly-tracked
 *  games. Surfaces user-friendly toasts; failures fall through to the
 *  toast and resolve to 0 instead of throwing — the sidebar's button is
 *  not a place where we want unhandled rejections crashing the UI. */
export async function runMagicSetup(): Promise<number> {
  if (get(magicState).kind !== "idle") return 0;

  try {
    magicState.set({ kind: "detecting" });
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
      magicState.set({ kind: "idle" });
      toastInfo("Nothing new to track. You're all set.");
      // Still try to start the agent — the user may have tracked games
      // from a previous session that aren't running yet.
      await bootAgent().catch(() => {});
      return 0;
    }

    let trackedCount = 0;
    for (const [i, game] of candidates.entries()) {
      magicState.set({
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
        console.warn(`magic-setup: couldn't track ${game.slug}:`, e);
      }
    }

    magicState.set({ kind: "starting_agent" });
    await bootAgent().catch(() => {});

    magicState.set({ kind: "idle" });
    toastSuccess(
      `Tracked ${trackedCount} game${trackedCount === 1 ? "" : "s"}; agent running.`,
    );
    return trackedCount;
  } catch (e) {
    magicState.set({ kind: "idle" });
    toastError(typeof e === "string" ? e : (e as Error).message);
    return 0;
  }
}
