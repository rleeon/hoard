/**
 * Live status + activity feed store.
 *
 * Subscribes to the `agent://*` event firehose (the same one `agent.ts`
 * listens to, but for a different purpose):
 *
 *  - {@link liveStatus} — coarse derived state for the LiveStatus header
 *    widget. Two dots: "watcher" (local fs watcher armed?) and "cloud"
 *    (manifest poller getting 2xx?). Pure UI.
 *  - {@link activityFeed} — bounded circular buffer of recent events for
 *    the ActivityFeed panel. Capped at {@link MAX_FEED_ENTRIES} so a
 *    long-running session doesn't grow memory unbounded.
 *
 * The store is intentionally decoupled from `agent.ts`: that one drives
 * per-save state + tray + notifications. This one is the "honest status"
 * surface for the new 1.7.0 UX — keeping them separate lets each
 * subscriber own its concern without one's payload shape leaking into
 * the other.
 */
import { derived, writable, type Writable, type Readable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AgentEvent } from "../api";

/** Coarse local state. `armed` when at least one watcher slot is alive
 *  (any `watcher-armed` event seen in the session); `offline` until the
 *  agent boots. */
export type WatcherStatus = "armed" | "offline";

/** Coarse cloud-loop state derived from the last `agent://cloud-pull-*`
 *  outcome. `unknown` until the first poll completes (or until login).
 *  `throttled` only when a 429 came back. */
export type CloudStatus = "online" | "offline" | "throttled" | "unknown";

/** One row in the floating ActivityFeed. */
export type FeedEntry = {
  /** Monotonic id so Svelte's keyed each can dedupe. */
  id: number;
  /** Wall-clock timestamp for relative formatting. */
  at: number;
  kind:
    | "watcher_armed"
    | "game_started"
    | "game_stopped"
    | "throttled"
    | "upload_started"
    | "upload_completed"
    | "upload_failed"
    | "auto_restored"
    | "cloud_pull"
    | "quota_reached"
    | "offline"
    | "online";
  /** Optional save_id / game_slug for renderers that want a hint. */
  save_id?: string;
  game_slug?: string;
  /** Per-kind extras (version, bytes, count, retry seconds, plan). */
  version?: number;
  bytes?: number;
  count?: number;
  new_versions?: number;
  retry_in?: number;
  plan?: string;
  error?: string;
};

const MAX_FEED_ENTRIES = 80;

type WatcherState = {
  /** Has any `watcher-armed` event come in this session? */
  armed: boolean;
  /** Number of distinct save_ids the watcher has armed for. */
  count: number;
};

type CloudState = {
  status: CloudStatus;
  /** Epoch ms of the last successful `cloud-pull-completed`. */
  last_ok_at: number | null;
  /** Set when status === 'throttled'; expressed in seconds-until-reset. */
  retry_in: number | null;
};

export const watcher: Writable<WatcherState> = writable({
  armed: false,
  count: 0,
});

export const cloudLoop: Writable<CloudState> = writable({
  status: "unknown",
  last_ok_at: null,
  retry_in: null,
});

export const activityFeed: Writable<FeedEntry[]> = writable([]);

/** Compact "everything green / something off" colour for the header dot.
 *  - `ok`     watcher armed + cloud online (or no cloud session).
 *  - `warn`   throttled or only one half off.
 *  - `error`  cloud offline or watcher offline. */
export const liveStatus: Readable<"ok" | "warn" | "error" | "unknown"> =
  derived([watcher, cloudLoop], ([$w, $c]) => {
    if (!$w.armed && $c.status === "unknown") return "unknown";
    if ($c.status === "offline") return "armed" in $w && $w.armed ? "warn" : "error";
    if ($c.status === "throttled") return "warn";
    if (!$w.armed) return "warn";
    return "ok";
  });

let nextId = 1;
function pushEntry(entry: Omit<FeedEntry, "id" | "at">) {
  activityFeed.update((rows) => {
    const next: FeedEntry[] = [
      { id: nextId++, at: Date.now(), ...entry },
      ...rows,
    ];
    if (next.length > MAX_FEED_ENTRIES) next.length = MAX_FEED_ENTRIES;
    return next;
  });
}

/** Type for the `agent://watcher-armed` payload. Mirrors the Rust struct. */
type WatcherArmedPayload = { save_id: string; game_slug: string };

/** Type for the `agent://cloud-pull-completed` payload. */
type CloudPullCompletedPayload = {
  count: number;
  new_versions: number;
  bytes: number;
};

type QuotaReachedPayload = { reset_in_seconds: number; plan: string };

let unlisteners: UnlistenFn[] = [];
const seenArmed = new Set<string>();

export async function subscribeLive() {
  await unsubscribeLive();

  unlisteners.push(
    await listen<WatcherArmedPayload>("agent://watcher-armed", (e) => {
      const p = e.payload;
      if (seenArmed.has(p.save_id)) return;
      seenArmed.add(p.save_id);
      watcher.update((w) => ({
        armed: true,
        count: seenArmed.size,
      }));
      pushEntry({
        kind: "watcher_armed",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://game-started", (e) => {
      const p = e.payload;
      if (p.type !== "game_started") return;
      pushEntry({
        kind: "game_started",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://game-stopped", (e) => {
      const p = e.payload;
      if (p.type !== "game_stopped") return;
      pushEntry({
        kind: "game_stopped",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://throttled", (e) => {
      const p = e.payload;
      if (p.type !== "backup_scheduled") return;
      pushEntry({ kind: "throttled", save_id: p.save_id });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://upload-started", (e) => {
      const p = e.payload;
      if (p.type !== "backup_started") return;
      pushEntry({ kind: "upload_started", save_id: p.save_id });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://upload-completed", (e) => {
      const p = e.payload;
      if (p.type !== "backup_success") return;
      pushEntry({
        kind: "upload_completed",
        save_id: p.save_id,
        version: p.version_num,
        bytes: p.total_bytes,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-failed", (e) => {
      const p = e.payload;
      if (p.type !== "backup_failed") return;
      pushEntry({
        kind: "upload_failed",
        save_id: p.save_id,
        error: p.error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://save-auto-restored", (e) => {
      const p = e.payload;
      if (p.type !== "save_auto_restored") return;
      pushEntry({
        kind: "auto_restored",
        save_id: p.save_id,
        game_slug: p.game_slug,
        version: p.version_num,
        bytes: p.bytes_extracted,
      });
    }),
  );

  // `cloud-pull-started` is intentionally not pushed to the feed: it would
  // dominate the panel at a 10s cadence. We use it only to keep the dot
  // honest when the very first poll hasn't finished yet.
  unlisteners.push(
    await listen<CloudPullCompletedPayload>(
      "agent://cloud-pull-completed",
      (e) => {
        const p = e.payload;
        cloudLoop.update((c) => ({
          status: "online",
          last_ok_at: Date.now(),
          retry_in: null,
        }));
        // Only push to the feed when something actually changed — a
        // baseline "247 saves, 0 new" poll is uninteresting noise.
        if (p.new_versions > 0) {
          pushEntry({
            kind: "cloud_pull",
            count: p.count,
            new_versions: p.new_versions,
            bytes: p.bytes,
          });
        }
      },
    ),
  );

  unlisteners.push(
    await listen<QuotaReachedPayload>("agent://quota-reached", (e) => {
      const p = e.payload;
      cloudLoop.update((c) => ({
        ...c,
        status: "throttled",
        retry_in: p.reset_in_seconds,
      }));
      pushEntry({
        kind: "quota_reached",
        retry_in: p.reset_in_seconds,
        plan: p.plan,
      });
    }),
  );

  unlisteners.push(
    await listen<void>("agent://offline", () => {
      cloudLoop.update((c) =>
        c.status === "offline" ? c : { ...c, status: "offline" },
      );
      pushEntry({ kind: "offline" });
    }),
  );
}

export async function unsubscribeLive() {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
      /* ignore */
    }
  }
  unlisteners = [];
}

/** Reset the in-memory state. Called on logout so a re-login starts clean. */
export function resetLive() {
  seenArmed.clear();
  watcher.set({ armed: false, count: 0 });
  cloudLoop.set({ status: "unknown", last_ok_at: null, retry_in: null });
  activityFeed.set([]);
}
