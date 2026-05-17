/**
 * Update-checker plumbing.
 *
 * Fetches both the latest GitHub release for the desktop client and the
 * `/v1/health` `version` of the user's current server. The Rust side does
 * the work; this module is a thin TS shim with a result-type re-export.
 *
 * We don't poll on a timer — the Settings page hits the IPC on mount and
 * the sidebar consumes the cached result via the `lastReport` store.
 * That keeps GitHub's rate limit happy (the API allows 60 req/h
 * unauthenticated) without forcing the user to think about it.
 */
import { invoke } from "@tauri-apps/api/core";
import { writable, type Writable } from "svelte/store";

export type ComponentUpdate = {
  current: string;
  latest: string | null;
  available: boolean;
  error: string | null;
};

export type UpdateReport = {
  client: ComponentUpdate;
  server: ComponentUpdate | null;
};

export const lastReport: Writable<UpdateReport | null> = writable(null);

/** Tauri command wrapper. Caches the most recent report in `lastReport`
 *  so other components can subscribe without re-fetching. */
export async function checkForUpdates(): Promise<UpdateReport> {
  const r = await invoke<UpdateReport>("check_for_updates");
  lastReport.set(r);
  return r;
}

/**
 * Result of `apply_desktop_update`. `installer_launched` means we spawned the
 * platform installer (pkexec / msiexec / open); `downloaded` means we got the
 * file onto disk but couldn't launch — the UI tells the user to run it.
 */
export type ApplyOutcome =
  | { kind: "installer_launched"; path: string; version: string }
  | { kind: "downloaded"; path: string; version: string };

/** Download the latest desktop release asset and trigger the OS installer. */
export async function applyDesktopUpdate(): Promise<ApplyOutcome> {
  return await invoke<ApplyOutcome>("apply_desktop_update");
}

// ---------------------------------------------------------------------------
// Periodic re-check
// ---------------------------------------------------------------------------
//
// Boot-time probes catch the moment the user opens the app, but a session
// that stays open for days would otherwise never re-check. We schedule a
// silent re-check every six hours; on failure (typically GitHub rate-limit
// or network hiccup) we exponentially back off so a downed network doesn't
// hammer the IPC, capping at 24h. App tear-down (logout, app exit) cancels
// the timer via the returned dispose function.

/** Six hours in ms — the steady-state cadence when probes succeed. */
const BASE_INTERVAL_MS = 6 * 60 * 60 * 1000;
/** 24h — the longest we ever wait between attempts. */
const MAX_INTERVAL_MS = 24 * 60 * 60 * 1000;

/**
 * Start a background updater loop that re-runs `checkForUpdates()` every
 * six hours. Returns a `dispose()` function the caller is expected to call
 * on logout / unmount.
 *
 * The first probe is NOT triggered here — the boot path in `App.svelte`
 * already does an immediate probe right after auth settles, and we don't
 * want to fire two requests back-to-back. The first scheduled probe lands
 * `BASE_INTERVAL_MS` after `start()` returns.
 *
 * On a failing probe the next delay doubles (6h → 12h → 24h cap). On the
 * next success it snaps back to 6h. This is the same pattern the catalog
 * auto-refresh uses, kept inline here so updates.ts has no extra deps.
 */
export function startUpdatePoller(): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let delay = BASE_INTERVAL_MS;
  let stopped = false;

  function schedule() {
    if (stopped) return;
    timer = setTimeout(async () => {
      try {
        await checkForUpdates();
        // Probe succeeded — reset the backoff so we resume the steady cadence.
        delay = BASE_INTERVAL_MS;
      } catch (e) {
        // GitHub returns 403 on rate-limit and network blips throw too.
        // We log at debug only — failing to know about a newer version
        // isn't user-facing.
        console.warn("scheduled update check failed:", e);
        delay = Math.min(delay * 2, MAX_INTERVAL_MS);
      } finally {
        schedule();
      }
    }, delay);
  }

  schedule();

  return () => {
    stopped = true;
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  };
}
