/**
 * Live status + activity feed store.
 *
 * Subscribes to the `agent://*` event firehose (the same one `agent.ts`
 * listens to, but for a different purpose):
 *
 *  - {@link liveStatus}, coarse derived state for the LiveStatus header
 *    widget. Two dots: "watcher" (local fs watcher armed?) and "cloud"
 *    (manifest poller getting 2xx?). Pure UI.
 *  - {@link activityFeed}, bounded circular buffer of recent events for
 *    the ActivityFeed panel. Capped at {@link MAX_FEED_ENTRIES} so a
 *    long-running session doesn't grow memory unbounded.
 *
 * The store is intentionally decoupled from `agent.ts`: that one drives
 * per-save state + tray + notifications. This one is the "honest status"
 * surface for the new 1.7.0 UX, keeping them separate lets each
 * subscriber own its concern without one's payload shape leaking into
 * the other.
 */
import { derived, writable, type Writable, type Readable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AgentEvent, CloudPulse, JournalRow } from "../api";

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
    | "bandwidth_throttled"
    | "auto_restored"
    | "cloud_pull"
    | "quota_reached"
    | "offline"
    | "online"
    // Per-save plan-limit outcomes, surfaced here (not as toasts) so the
    // reconciliation sweep doesn't spam a popup per save on every launch.
    | "backup_too_large"
    // The account ran out of storage. Account-wide, not per-save: the engine
    // collapses every save's report into one row (see `journal::collapse_key`)
    // and the row carries the action that opens "liberar espacio".
    | "backup_quota_full"
    | "backup_trimmed"
    // La copia subió sin ficheros que no se dejaron leer (o no subió nada
    // porque no se dejó leer ninguno). Parcial, dicho en voz alta.
    | "backup_files_unreadable"
    | "auto_restore_failed"
    // Auto-restore has failed repeatedly on the same cloud version. Distinct
    // from `auto_restore_failed` (one row per attempt): this is the "it's been
    // failing and it won't fix itself" row, pushed once per (save, version).
    | "auto_restore_stuck"
    | "auto_restore_recovered"
    // La subida se rindió ante un conflicto que no sabe resolver: ya no hay
    // reintento en camino, hace falta el usuario.
    | "backup_blocked"
    | "backup_unblocked"
    // Account-wide storage pressure, driven off the cloud account's
    // `storage_status` (`purging` → amber, `full` → red).
    | "storage_purging"
    | "storage_full"
    // A storage downgrade is scheduled: the account keeps its old limit and
    // nothing is deleted until the date. The row exists so the countdown is
    // seen *before* the shrink, which is the whole point of the window.
    | "storage_grace"
    // Pro-gate (Hoard-Screen) transitions, pushed from the entitlements store
    // when the gate visibly flips between locked and unlocked, with the cause
    // in `reason_key` (an i18n key). Lets the user see WHY the candado
    // changed without opening a log file.
    | "gate_locked"
    | "gate_unlocked";
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
  /** Per-save cap in bytes, for the `backup_too_large` row. */
  limit_bytes?: number;
  /** Who refused the upload, for the `backup_too_large` row. The row's sentence
   *  follows this: a plan cap, the user's own server's `max_snapshot_size_mb`,
   *  or a proxy in front of it, three different fixes. Absent on rows written
   *  before this existed, which render as the plan cap they used to assume. */
  too_large_kind?: "plan_cap" | "server_limit" | "proxy";
  /** Consecutive failures, for the `auto_restore_stuck` row. */
  failures?: number;
  /** i18n key for the cause of a `gate_locked`/`gate_unlocked` row. */
  reason_key?: string;
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

/** "The account is out of storage and uploads are parked", the same fact the
 *  `backup_quota_full` feed row reports, kept as *state* instead of a scrolling
 *  event so a surface that must be visible at all times can read it without the
 *  activity panel being open (see `StorageFullBanner.svelte`).
 *
 *  `null` = uploads are flowing. Non-null carries the figures for the message.
 *  Two writers, on purpose:
 *   - the engine's 402 (`agent://backup-quota-full`), instant, fires the moment
 *     an upload actually bounces;
 *   - the account refresh (`noteStorageStatus`), authoritative, and the only
 *     thing that ever *clears* the flag, so a stale latch can't outlive the
 *     problem it describes. */
export type StorageBlock = { used: number; limit: number };
export const storageBlock: Writable<StorageBlock | null> = writable(null);

/** Compact "everything green / something off" colour for the header dot.
 *  - `ok`     watcher armed + cloud online (or no cloud session).
 *  - `warn`   throttled or only one half off.
 *  - `error`  cloud offline or watcher offline. */
export const liveStatus: Readable<"ok" | "warn" | "error" | "unknown"> =
  derived([watcher, cloudLoop], ([$w, $c]) => {
    if (!$w.armed && $c.status === "unknown") return "unknown";
    if ($c.status === "offline")
      return "armed" in $w && $w.armed ? "warn" : "error";
    if ($c.status === "throttled") return "warn";
    if (!$w.armed) return "warn";
    return "ok";
  });

/** Seed the watcher state straight from the agent's boot status.
 *
 *  The `agent://watcher-armed` events are emitted by `start_agent` the instant
 *  it spawns, which happens during `hydrateAuth()`/`bootAgent()`, *before*
 *  `subscribeLive()` has registered its listener. Those events are therefore
 *  routinely missed, leaving the header stuck on "watcher off" even though the
 *  agent is happily watching. `bootAgent` calls this with the watched-save
 *  count so the dot reflects reality without depending on the event race. */
export function seedWatcher(count: number) {
  if (count <= 0) return;
  watcher.set({ armed: true, count });
}

let nextId = 1;
/** `at` defaults to now; the journal replay passes the real timestamp so a
 *  recovered row doesn't claim to have just happened. */
function pushEntry(
  entry: Omit<FeedEntry, "id" | "at">,
  at: number = Date.now(),
) {
  activityFeed.update((rows) => {
    const next: FeedEntry[] = [{ id: nextId++, at, ...entry }, ...rows];
    if (next.length > MAX_FEED_ENTRIES) next.length = MAX_FEED_ENTRIES;
    return next;
  });
}

/** One row of the sync service's journal (ADR 0021 D.14.2), as relayed by the
 *  Rust side on `agent://backlog`. */
type BacklogRow = { at: number; event: AgentEvent };

/** The feed row an engine event maps to, or `null` when it isn't feed material.
 *
 *  One mapping, two callers: the backlog replay below and {@link adoptJournal}.
 *  They used to be one `switch` each, which is how two surfaces drift into
 *  telling slightly different stories about the same event. The pair that only
 *  exists as a *live* alias (`agent://throttled`) is deliberately absent,
 *  "queued, waiting" is a momentary state, not history worth resurrecting. */
function feedRowFor(p: AgentEvent): Omit<FeedEntry, "id" | "at"> | null {
  switch (p.type) {
    case "game_started":
      return { kind: "game_started", save_id: p.save_id, game_slug: p.game_slug };
    case "game_stopped":
      return { kind: "game_stopped", save_id: p.save_id, game_slug: p.game_slug };
    case "backup_started":
      return {
        kind: "upload_started",
        save_id: p.save_id,
        game_slug: p.game_slug,
      };
    case "backup_success":
      return {
        kind: "upload_completed",
        save_id: p.save_id,
        version: p.version_num,
        bytes: p.total_bytes,
      };
    case "backup_failed":
      return {
        kind: "upload_failed",
        save_id: p.save_id,
        game_slug: p.game_slug,
        error: p.error,
      };
    case "backup_throttled":
      return {
        kind: "bandwidth_throttled",
        save_id: p.save_id,
        game_slug: p.game_slug,
        retry_in: p.retry_after_secs,
      };
    case "save_auto_restored":
      return {
        kind: "auto_restored",
        save_id: p.save_id,
        game_slug: p.game_slug,
        version: p.version_num,
        bytes: p.bytes_extracted,
      };
    case "backup_too_large":
      return {
        kind: "backup_too_large",
        save_id: p.save_id,
        game_slug: p.game_slug,
        bytes: p.actual_bytes,
        limit_bytes: p.limit_bytes,
        too_large_kind: p.kind,
      };
    case "backup_quota_full":
      return {
        kind: "backup_quota_full",
        save_id: p.save_id,
        game_slug: p.game_slug,
        plan: p.plan,
        bytes: p.used_bytes,
        limit_bytes: p.limit_bytes,
      };
    case "backup_trimmed":
      return {
        kind: "backup_trimmed",
        save_id: p.save_id,
        game_slug: p.game_slug,
        count: p.omitted_files,
        bytes: p.omitted_bytes,
      };
    case "backup_files_unreadable":
      return {
        kind: "backup_files_unreadable",
        save_id: p.save_id,
        game_slug: p.game_slug,
        count: p.count,
        error: p.sample_error,
      };
    case "save_auto_restore_failed":
      return {
        kind: "auto_restore_failed",
        save_id: p.save_id,
        game_slug: p.game_slug,
        error: p.error,
      };
    case "save_auto_restore_stuck":
      return {
        kind: "auto_restore_stuck",
        save_id: p.save_id,
        game_slug: p.game_slug,
        failures: p.failures,
        error: p.error,
      };
    case "save_auto_restore_recovered":
      return {
        kind: "auto_restore_recovered",
        save_id: p.save_id,
        game_slug: p.game_slug,
      };
    case "backup_needs_attention":
      return {
        kind: "backup_blocked",
        save_id: p.save_id,
        game_slug: p.game_slug,
        failures: p.conflicts,
        error: p.error,
      };
    case "backup_attention_cleared":
      return {
        kind: "backup_unblocked",
        save_id: p.save_id,
        game_slug: p.game_slug,
      };
    default:
      return null;
  }
}

/** Rebuild feed rows from what the service journalled while nobody was
 *  listening, the app was closed, or we reconnected.
 *
 *  The feed is **not** cleared on `resync`: rows only ever arrive newer than
 *  our cursor, and it also holds rows from other sources (cloud pulls, gate
 *  flips) that a wipe would throw away. */
function applyBacklogRow({ at, event }: BacklogRow) {
  const row = feedRowFor(event);
  if (row) pushEntry(row, at);
}

/** Replace the feed with a journal snapshot, wholesale.
 *
 *  This is the read-only path, for a surface that doesn't subscribe at all: it
 *  asks Rust what the journal says and paints that. Replacing rather than
 *  merging is the whole point, there is no cursor to keep, no gap to reason
 *  about and no way to double-count a row, because the snapshot *is* the state.
 *
 *  `seq` becomes the row id, so re-reading the same journal yields the same
 *  keys and Svelte reuses the DOM instead of rebuilding the list under the
 *  user's eyes. */
export function adoptJournal(rows: JournalRow[]): void {
  const feed: FeedEntry[] = [];
  // Newest first, like the live path prepends.
  for (let i = rows.length - 1; i >= 0; i--) {
    const { seq, at, event } = rows[i];
    const row = feedRowFor(event);
    if (row) feed.push({ id: seq, at, ...row });
  }
  if (feed.length > MAX_FEED_ENTRIES) feed.length = MAX_FEED_ENTRIES;
  activityFeed.set(feed);
}

/** Adopt the cloud-loop state from a snapshot. Same reason as
 *  {@link adoptJournal}: `agent://cloud-pull-completed` and friends are
 *  momentary, so a window that wasn't listening when the last poll landed can
 *  only learn the answer by asking. */
export function adoptCloud(status: CloudPulse, retryIn: number | null): void {
  cloudLoop.set({
    status,
    // El snapshot trae el estado, no *cuándo* fue: poner `Date.now()` aquí
    // diría "acabo de comprobarlo" cada vez que alguien abre el HUD, que es
    // justo la clase de mentira que este panel existe para no contar.
    last_ok_at: null,
    retry_in: retryIn,
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

  // Catch-up first: whatever the service journalled while nobody was listening.
  unlisteners.push(
    await listen<{ rows: BacklogRow[]; resync: boolean }>(
      "agent://backlog",
      (e) => {
        // Oldest first, so prepending leaves the newest on top.
        for (const row of e.payload.rows) applyBacklogRow(row);
      },
    ),
  );

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
      pushEntry({
        kind: "upload_started",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
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
        game_slug: p.game_slug,
        error: p.error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-throttled", (e) => {
      const p = e.payload;
      if (p.type !== "backup_throttled") return;
      pushEntry({
        kind: "bandwidth_throttled",
        save_id: p.save_id,
        game_slug: p.game_slug,
        retry_in: p.retry_after_secs,
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
        // Only push to the feed when something actually changed, a
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

  // Plan-limit outcomes. These used to fire a toast per save from `agent.ts`,
  // which meant a burst of popups every launch as the reconciliation sweep
  // re-hit the same over-cap saves. They belong in the feed like any other
  // per-save event; `agent.ts` still updates the Library row state.
  unlisteners.push(
    await listen<AgentEvent>("agent://backup-too-large", (e) => {
      const p = e.payload;
      if (p.type !== "backup_too_large") return;
      pushEntry({
        kind: "backup_too_large",
        save_id: p.save_id,
        game_slug: p.game_slug,
        bytes: p.actual_bytes,
        limit_bytes: p.limit_bytes,
        too_large_kind: p.kind,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-quota-full", (e) => {
      const p = e.payload;
      if (p.type !== "backup_quota_full") return;
      // Don't wait for the next /v1/me poll to raise the banner: the upload
      // already bounced, so the account is full *now*.
      storageBlock.set({ used: p.used_bytes, limit: p.limit_bytes });
      pushEntry({
        kind: "backup_quota_full",
        save_id: p.save_id,
        game_slug: p.game_slug,
        plan: p.plan,
        bytes: p.used_bytes,
        limit_bytes: p.limit_bytes,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-trimmed", (e) => {
      const p = e.payload;
      if (p.type !== "backup_trimmed") return;
      pushEntry({
        kind: "backup_trimmed",
        save_id: p.save_id,
        game_slug: p.game_slug,
        count: p.omitted_files,
        bytes: p.omitted_bytes,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-files-unreadable", (e) => {
      const p = e.payload;
      if (p.type !== "backup_files_unreadable") return;
      pushEntry({
        kind: "backup_files_unreadable",
        save_id: p.save_id,
        game_slug: p.game_slug,
        count: p.count,
        error: p.sample_error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://save-auto-restore-failed", (e) => {
      const p = e.payload;
      if (p.type !== "save_auto_restore_failed") return;
      pushEntry({
        kind: "auto_restore_failed",
        save_id: p.save_id,
        game_slug: p.game_slug,
        error: p.error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://save-auto-restore-stuck", (e) => {
      const p = e.payload;
      if (p.type !== "save_auto_restore_stuck") return;
      pushEntry({
        kind: "auto_restore_stuck",
        save_id: p.save_id,
        game_slug: p.game_slug,
        failures: p.failures,
        error: p.error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-needs-attention", (e) => {
      const p = e.payload;
      if (p.type !== "backup_needs_attention") return;
      pushEntry({
        kind: "backup_blocked",
        save_id: p.save_id,
        game_slug: p.game_slug,
        failures: p.conflicts,
        error: p.error,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://backup-attention-cleared", (e) => {
      const p = e.payload;
      if (p.type !== "backup_attention_cleared") return;
      pushEntry({
        kind: "backup_unblocked",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
    }),
  );

  unlisteners.push(
    await listen<AgentEvent>("agent://save-auto-restore-recovered", (e) => {
      const p = e.payload;
      if (p.type !== "save_auto_restore_recovered") return;
      pushEntry({
        kind: "auto_restore_recovered",
        save_id: p.save_id,
        game_slug: p.game_slug,
      });
    }),
  );
}

/** Last account-wide storage pressure we pushed a feed row for, so a 30s
 *  quota refresh that re-reports the same state doesn't spam the panel. */
let lastStorageStatus: string | null = null;

/** Called by the cloud store whenever the account is (re)loaded. Pushes a
 *  single amber (`purging`) / red (`full`) feed row on entering that state,
 *  and resets silently when it recovers to `ok`. */
export function noteStorageStatus(
  status: string | undefined | null,
  usedBytes?: number,
  limitBytes?: number,
) {
  const s = status ?? "ok";
  // Banner state first, and *outside* the dedupe below: a re-poll that still
  // says "full" must keep refreshing the figures (the used total moves as the
  // engine keeps trying), even though it must not push a second feed row.
  // `used >= limit` is the fallback for older servers that send no status. A
  // "full" with no usable figures (limit <= 0, which the server never sends)
  // leaves the store alone rather than overwriting a real latch with zeroes.
  const used = usedBytes ?? 0;
  const limit = limitBytes ?? 0;
  if (limit > 0 && (s === "full" || used >= limit)) {
    storageBlock.set({ used, limit });
  } else if (s !== "full") {
    storageBlock.set(null);
  }
  if (s === lastStorageStatus) return;
  lastStorageStatus = s;
  if (s === "purging") pushEntry({ kind: "storage_purging" });
  else if (s === "full") pushEntry({ kind: "storage_full" });
  else if (s === "grace") pushEntry({ kind: "storage_grace" });
}

/** Last gate state we pushed a feed row for, so a re-pull that reports the
 *  same locked/unlocked state doesn't duplicate the row. */
let lastGateState: "locked" | "unlocked" | null = null;

/** Called by the entitlements store when the Hoard-Screen gate visibly flips
 *  between locked and unlocked. `reasonKey` is an i18n key naming the cause
 *  (fetch failed, Free plan, trial ended, Pro plan, …). Deduped on state so a
 *  healthy re-poll never spams the panel. */
export function noteGateTransition(locked: boolean, reasonKey: string): void {
  const state = locked ? "locked" : "unlocked";
  if (state === lastGateState) return;
  lastGateState = state;
  pushEntry({
    kind: locked ? "gate_locked" : "gate_unlocked",
    reason_key: reasonKey,
  });
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
  storageBlock.set(null);
  lastStorageStatus = null;
  lastGateState = null;
}

/** Reset only the cloud-loop dot to its neutral baseline, leaving the local
 *  watcher state and the activity feed intact. Used on terminal session expiry,
 *  where the cloud half is signed out but the local agent keeps watching, so a
 *  full `resetLive()` would wrongly blank the watcher dot. */
export function resetCloudLoop() {
  cloudLoop.set({ status: "unknown", last_ok_at: null, retry_in: null });
}
