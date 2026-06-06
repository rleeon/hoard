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
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { writable, type Writable } from "svelte/store";

import { prefs, updatePrefs } from "./prefs";

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
 *  so other components can subscribe without re-fetching. After the report
 *  lands we also fan it through `notifyIfNewClientRelease` so the OS sees
 *  a native banner even when the app is minimised to the tray (which was
 *  the entire reason this lived behind a "user reopens the app" gate
 *  pre-1.4.5). */
export async function checkForUpdates(): Promise<UpdateReport> {
  const r = await invoke<UpdateReport>("check_for_updates");
  lastReport.set(r);
  await notifyIfNewClientRelease(r);
  return r;
}

let notificationsAllowed = false;
let permissionProbed = false;

/** Ask the OS for permission once per session. Same pattern as
 *  `lib/stores/agent.ts`. Silent if the user denied — we just skip the
 *  native banner; the amber sidebar badge is still there for when they
 *  open the window. */
async function ensureNotificationPermission(): Promise<boolean> {
  if (permissionProbed) return notificationsAllowed;
  permissionProbed = true;
  try {
    notificationsAllowed = await isPermissionGranted();
    if (!notificationsAllowed) {
      const decision = await requestPermission();
      notificationsAllowed = decision === "granted";
    }
  } catch (e) {
    console.warn("notification permission probe failed:", e);
    notificationsAllowed = false;
  }
  return notificationsAllowed;
}

/**
 * Fire an OS notification when a new desktop release lands — but only the
 * first time we see a given version. We persist the last notified version
 * in `prefs.last_update_notified_version`, so even across app restarts the
 * user only gets one banner per release.
 *
 * Server updates are deliberately NOT notified: the server lives on a
 * machine the user controls and the upgrade requires a manual shell
 * command. Spamming them every 30 minutes with "your server is behind"
 * isn't actionable on the device they're holding.
 */
async function notifyIfNewClientRelease(report: UpdateReport): Promise<void> {
  const latest = report.client.latest;
  if (!report.client.available || !latest) return;

  const p = get(prefs);
  if (p && p.last_update_notified_version === latest) return;

  const allowed = await ensureNotificationPermission();
  if (!allowed) return;

  const title = get(_)("updates.notification_title");
  const body = get(_)("updates.notification_body", { values: { latest } });

  try {
    sendNotification({ title, body });
  } catch (e) {
    console.warn("update notification failed:", e);
    return;
  }

  // Persist *after* we've actually fired the banner so a failure earlier in
  // the chain leaves the prefs unchanged and the next poll re-tries.
  try {
    await updatePrefs({ last_update_notified_version: latest });
  } catch (e) {
    // Persistence failures are non-fatal — worst case the user gets the
    // banner again next session. That's preferable to silently never
    // notifying them because the prefs file was momentarily locked.
    console.warn("persisting last_update_notified_version failed:", e);
  }
}

/**
 * Result of `apply_desktop_update`. `installer_launched` means we spawned the
 * platform installer (pkexec / msiexec / open); `downloaded` means we got the
 * file onto disk but couldn't launch — the UI tells the user to run it.
 */
export type ApplyOutcome =
  | { kind: "installer_launched"; path: string; version: string }
  | { kind: "downloaded"; path: string; version: string }
  | { kind: "superseded"; latest: string };

/** Download the latest desktop release asset and trigger the OS installer.
 *  Pass the version the modal offered (`expectedVersion`) so the Rust side can
 *  abort with `superseded` if a newer release appeared in the meantime — we
 *  never want to install an older build than GitHub's current "latest". */
export async function applyDesktopUpdate(
  expectedVersion?: string,
): Promise<ApplyOutcome> {
  return await invoke<ApplyOutcome>("apply_desktop_update", {
    expectedVersion: expectedVersion ?? null,
  });
}

/**
 * Result of `apply_server_update`. The two `kind`s let the UI distinguish
 * "all done, server is back on the new version" (`upgraded_and_restarted`)
 * from "binary swapped, but you'll need to restart it yourself" (`upgraded`).
 * In the latter case `restart_error` carries whatever pkexec / systemctl
 * said so we can show it for triage.
 */
export type ServerUpgradeOutcome =
  | { kind: "upgraded_and_restarted"; output: string }
  | { kind: "upgraded"; output: string; restart_error: string };

/**
 * Trigger an in-app upgrade of the user's self-hosted server. Linux only;
 * pops a polkit auth prompt (pkexec). Only works when the server shares the
 * machine with the desktop app — superseded by `triggerServerUpgrade` for
 * the general (cross-machine) case. Kept for the same-box convenience path.
 */
export async function applyServerUpdate(): Promise<ServerUpgradeOutcome> {
  return await invoke<ServerUpgradeOutcome>("apply_server_update");
}

/**
 * Result of `trigger_server_upgrade` (ADR 0017). `confirmed` means the server
 * came back on a new version within the poll window; `scheduled` means the
 * request was accepted but we couldn't confirm the restart before timing out
 * (the server may still be coming back, or was already up to date).
 */
export type RemoteUpgradeOutcome =
  | { kind: "confirmed"; version: string }
  | { kind: "scheduled" };

/**
 * Ask the self-hosted server to upgrade *itself* over HTTP. Works from any
 * OS and whether the server is local or on another box — the server runs a
 * signed self-upgrade and restarts. Requires an admin token; the Settings
 * panel gates the button on `auth.user.is_admin`.
 */
export async function triggerServerUpgrade(): Promise<RemoteUpgradeOutcome> {
  return await invoke<RemoteUpgradeOutcome>("trigger_server_upgrade");
}

// ---------------------------------------------------------------------------
// Periodic re-check
// ---------------------------------------------------------------------------
//
// Boot-time probes catch the moment the user opens the app, but the agent is
// a tray-resident process — sessions stay alive for *days* in real usage and
// the original 6h cadence meant the user effectively had to reopen the
// window to get an update prompt. 1.4.5 dropped the cadence to 30 minutes
// because the GitHub API costs are still negligible (48 req/day worst-case,
// well below the unauthenticated 60-req/h ceiling) and pairs each successful
// detection with a native OS notification — the user no longer has to be
// looking at the window to find out.
//
// On failure (typically GitHub rate-limit or network hiccup) we
// exponentially back off so a downed network doesn't hammer the IPC,
// capping at 6h. App tear-down (logout, app exit) cancels the timer via
// the returned dispose function.

/** 30 min in ms — the steady-state cadence when probes succeed. */
const BASE_INTERVAL_MS = 30 * 60 * 1000;
/** 6h — the longest we ever wait between attempts. */
const MAX_INTERVAL_MS = 6 * 60 * 60 * 1000;

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
