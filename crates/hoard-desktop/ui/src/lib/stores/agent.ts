/**
 * Live-agent store.
 *
 * The Rust live agent emits events through the `agent://*` Tauri event
 * channels. This store subscribes to them once at app boot, keeps a
 * per-save activity status in memory, and lets pages reactively render
 * "backing up…", "next in 30s", "failed (will retry)", etc.
 *
 * Activity is keyed by `save_id`; pages look up their saves by id.
 *
 * The store is also responsible for two side effects that need to react to
 * the same event stream:
 *
 *  1. **Tray colouring** — we collapse all per-save states into a single
 *     "global state" and push it down to Rust so the tray icon recolours.
 *  2. **Desktop notifications** — backup successes/failures fire a system
 *     notification, gated by the user's prefs.
 *
 * Doing this in the same place avoids a separate listener subscription per
 * concern, and keeps the priority logic (failures beat successes beat
 * uploads…) in one obvious spot.
 */
import { derived, get, writable, type Writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { _ as i18n } from "svelte-i18n";

import type {
  AgentEvent,
  AgentStatus,
  BackupReason,
  TrayStateName,
} from "../api";
import * as api from "../api";
import { prefs } from "./prefs";
import { toastSuccess, toastError, toastInfo } from "./toasts";

export type SaveActivity = {
  state:
    | "idle"
    | "running" // game running
    | "scheduled"
    | "uploading"
    | "ok"
    | "failed";
  /** When the next backup is expected to fire (epoch ms), if scheduled. */
  next_backup_at?: number;
  reason?: BackupReason;
  last_version?: number;
  last_bytes?: number;
  error?: string;
  will_retry?: boolean;
};

type ActivityMap = Record<string, SaveActivity>;

export const activity: Writable<ActivityMap> = writable({});
export const status: Writable<AgentStatus> = writable({
  running: false,
  watched_count: 0,
});

/** Reduce per-save activity into a single tray-coloring state. Priority is
 * "anything broken first": a failure dominates an in-flight upload, which
 * dominates a running game, which dominates idle. Mirrors the precedence
 * users expect from a status indicator. */
export const trayState = derived<
  [Writable<ActivityMap>, Writable<AgentStatus>],
  TrayStateName
>([activity, status], ([$activity, $status]) => {
  if (!$status.running) return "offline";
  const states = Object.values($activity).map((a) => a.state);
  if (states.some((s) => s === "failed")) return "error";
  if (states.some((s) => s === "uploading")) return "uploading";
  if (states.some((s) => s === "scheduled")) return "uploading";
  if (states.some((s) => s === "running")) return "running";
  if (states.some((s) => s === "ok")) return "ok";
  return "idle";
});

let unlisteners: UnlistenFn[] = [];
let lastTrayState: TrayStateName | null = null;
let trayUnsub: (() => void) | null = null;
let notificationsAllowed = false;

async function ensureNotificationPermission() {
  // Ask once on startup. If the user denies, future toasts no-op silently —
  // we don't want to nag, and Tauri's plugin already handles the system
  // dialog gracefully.
  try {
    notificationsAllowed = await isPermissionGranted();
    if (!notificationsAllowed) {
      const decision = await requestPermission();
      notificationsAllowed = decision === "granted";
    }
  } catch (e) {
    console.warn("notification permission probe failed:", e);
  }
}

function notify(title: string, body: string) {
  if (!notificationsAllowed) return;
  try {
    sendNotification({ title, body });
  } catch (e) {
    console.warn("sendNotification failed:", e);
  }
}

function patch(save_id: string, partial: Partial<SaveActivity>) {
  activity.update((m) => {
    const prev = m[save_id] ?? { state: "idle" };
    return { ...m, [save_id]: { ...prev, ...partial } };
  });
}

function applyEvent(ev: AgentEvent) {
  switch (ev.type) {
    case "game_started":
      patch(ev.save_id, { state: "running" });
      break;
    case "game_stopped":
      patch(ev.save_id, { state: "idle" });
      break;
    case "backup_scheduled":
      patch(ev.save_id, {
        state: "scheduled",
        next_backup_at: Date.now() + ev.delay_ms,
        reason: ev.reason,
      });
      break;
    case "backup_started":
      patch(ev.save_id, { state: "uploading", next_backup_at: undefined });
      break;
    case "backup_success": {
      patch(ev.save_id, {
        state: "ok",
        last_version: ev.version_num,
        last_bytes: ev.total_bytes,
        error: undefined,
        will_retry: undefined,
      });
      const $prefs = get(prefs);
      if ($prefs?.notify_on_success) {
        notify(
          "Backup saved",
          `${ev.save_id.slice(0, 8)} · v${ev.version_num} (${formatBytes(
            ev.total_bytes,
          )})`,
        );
      }
      break;
    }
    case "backup_failed": {
      patch(ev.save_id, {
        state: "failed",
        error: ev.error,
        will_retry: ev.will_retry,
      });
      const $prefs = get(prefs);
      if ($prefs?.notify_on_failure) {
        notify(
          ev.will_retry ? "Backup failed (retrying)" : "Backup failed",
          ev.error,
        );
      }
      break;
    }
    case "save_auto_restored": {
      // Auto-restore is a one-shot adoption side-effect — no need to mutate
      // `activity` (no ongoing watch state to update), but we do want to
      // surface a toast so the user notices files appeared under `~`. The
      // `game_slug` is what we have to hand; resolving to display_name would
      // require another store dependency just for cosmetics.
      const t = get(i18n);
      toastSuccess(
        t("library.auto_restored_toast", {
          values: {
            name: ev.game_slug,
            version: ev.version_num,
            count: ev.files_extracted,
          },
        }),
      );
      break;
    }
    case "save_auto_restore_failed": {
      const t = get(i18n);
      toastError(
        t("library.auto_restore_failed_toast", {
          values: { name: ev.game_slug, error: ev.error },
        }),
      );
      break;
    }
    case "backup_skipped_empty": {
      // Saw an fs event that resolved to an empty/missing folder. We did
      // *not* push an empty snapshot to the server (that would silently
      // overwrite the user's last good copy). Toast the situation so the
      // user knows nothing was uploaded and can flip auto-restore on if
      // they wanted the cloud copy pulled back instead.
      patch(ev.save_id, { state: "idle", error: undefined, will_retry: undefined });
      const t = get(i18n);
      toastInfo(
        t("library.backup_skipped_empty_toast", {
          values: { name: ev.game_slug },
        }),
      );
      break;
    }
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/** Subscribe to all `agent://*` channels. Idempotent — safe to call from
 * `onMount` more than once (we tear down previous listeners first). */
export async function subscribeAgent() {
  await unsubscribeAgent();
  const topics = [
    "agent://game-started",
    "agent://game-stopped",
    "agent://backup-scheduled",
    "agent://backup-started",
    "agent://backup-success",
    "agent://backup-failed",
    "agent://save-auto-restored",
    "agent://save-auto-restore-failed",
    "agent://backup-skipped-empty",
  ];
  unlisteners = await Promise.all(
    topics.map((t) =>
      listen<AgentEvent>(t, (event) => applyEvent(event.payload)),
    ),
  );

  // Mirror the derived tray state into Rust whenever it changes. We only
  // push when the value actually flips so the tray doesn't repaint every
  // tick of the FS-event firehose.
  trayUnsub = trayState.subscribe((s) => {
    if (s === lastTrayState) return;
    lastTrayState = s;
    api.setTrayState(s).catch((e) => console.warn("setTrayState failed:", e));
  });
}

export async function unsubscribeAgent() {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
      /* ignore */
    }
  }
  unlisteners = [];
  if (trayUnsub) {
    trayUnsub();
    trayUnsub = null;
  }
  lastTrayState = null;
}

/** Start the agent and subscribe. Called once after login. */
export async function bootAgent() {
  await ensureNotificationPermission();
  const s = await api.startAgent();
  status.set(s);
  await subscribeAgent();
}

/** Stop the agent and clear state. Called on logout. */
export async function shutdownAgent() {
  await unsubscribeAgent();
  try {
    await api.stopAgent();
  } catch {
    /* logout should never fail because the agent didn't */
  }
  activity.set({});
  status.set({ running: false, watched_count: 0 });
  // Drop the tray back to "offline" so the icon doesn't lie.
  api.setTrayState("offline").catch(() => {});
}
