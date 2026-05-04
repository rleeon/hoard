/**
 * Live-agent store.
 *
 * The Rust live agent emits events through the `agent://*` Tauri event
 * channels. This store subscribes to them once at app boot, keeps a
 * per-save activity status in memory, and lets pages reactively render
 * "backing up…", "next in 30s", "failed (will retry)", etc.
 *
 * Activity is keyed by `save_id`; pages look up their saves by id.
 */
import { writable, type Writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AgentEvent, AgentStatus, BackupReason } from "../api";
import * as api from "../api";

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

let unlisteners: UnlistenFn[] = [];

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
    case "backup_success":
      patch(ev.save_id, {
        state: "ok",
        last_version: ev.version_num,
        last_bytes: ev.total_bytes,
        error: undefined,
        will_retry: undefined,
      });
      break;
    case "backup_failed":
      patch(ev.save_id, {
        state: "failed",
        error: ev.error,
        will_retry: ev.will_retry,
      });
      break;
  }
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
  ];
  unlisteners = await Promise.all(
    topics.map((t) =>
      listen<AgentEvent>(t, (event) => applyEvent(event.payload)),
    ),
  );
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
}

/** Start the agent and subscribe. Called once after login. */
export async function bootAgent() {
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
}
