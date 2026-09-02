/**
 * Notifications store, server + app messages for the bell dropdown.
 *
 * ============================================================================
 *  HOW TO SHOW A NOTIFICATION HERE
 * ============================================================================
 *
 *  1. From TypeScript (app side):
 *
 *     import { pushNotification } from "../lib/stores/notifications";
 *     pushNotification({
 *       title: "Backup failed",
 *       body: "Save **factorio** exceeded the 50 MB per-save cap.",
 *       priority: "high",
 *     });
 *
 *  2. From the server (Rust → Tauri event):
 *
 *     The Rust side emits a `hoard://notification` Tauri event with a JSON
 *     payload matching `ServerNotification` (see below). The agent polls the
 *     server's notification endpoint and forwards each new message as an
 *     event; this store listens and pushes it automatically.
 *
 *     To send a notification to users from the server, add a row to the
 *     server's notification table (or broadcast endpoint) with:
 *       - title: string
 *       - body: string  (supports **bold**, *italic*, `code`, [links](url))
 *       - priority: "high" | "normal" | "low"
 *       - audience: "all" | specific user_id (filtering is server-side)
 *
 *     The Rust agent polls every N seconds; new entries are deduped by `id`
 *     so a user never sees the same message twice.
 *
 *  3. Only IMPORTANT notifications should land here. This is NOT the activity
 *     feed (that's `stores/live.ts` → `activityFeed`, for per-save events like
 *     "upload started" / "watcher armed"). This store is for account-level
 *     messages the user needs to act on or know about:
 *       - Server broadcasts (maintenance, new features, security advisories)
 *       - Plan changes (quota reached, trial expiring, upgrade required)
 *       - Critical errors that need user intervention
 *
 *     Routine agent activity (backups, syncs, watcher events) goes in the
 *     activity feed, NOT here. When in doubt: if it can wait, it's a feed
 *     entry; if the user should act on it, it's a notification.
 *
 * ============================================================================
 *  MARKDOWN SUPPORT
 * ============================================================================
 *
 *  The `body` field supports a small, safe subset of markdown:
 *    **bold**          → <strong>
 *    *italic*          → <em>
 *    `inline code`     → <code>
 *    [text](url)       → <a href="url" target="_blank">
 *    \n (line breaks)  → <br>
 *
 *  No raw HTML is allowed, everything is escaped before formatting, so
 *  server messages can't inject scripts. Keep messages short; the panel is
 *  288px wide.
 *
 * ============================================================================
 *  STORAGE
 * ============================================================================
 *
 *  Notifications persist to localStorage (capped at 20, oldest dropped) so
 *  they survive restarts. High-priority ones can't be auto-dismissed; the
 *  user must clear them. Normal/low auto-expire after 7 days.
 */
import { writable } from "svelte/store";

export type NotificationPriority = "high" | "normal" | "low";

/** One CTA button. `icon` is a NAME the panel maps to a component, never
 *  markup, because everything the server sends is escaped before it renders.
 *  An unknown name draws a plain button, so the server can start sending a new
 *  icon before the app that knows how to draw it ships. */
export type NotificationAction = {
  url: string;
  label: string;
  icon?: string;
};

export type AppNotification = {
  /** Stable id for dedup. Server notifications use the server's id; app-side
   *  ones use a monotonic counter. */
  id: string;
  title: string;
  body: string;
  priority: NotificationPriority;
  /** Epoch ms. */
  at: number;
  /** "server" = came from the server; "app" = pushed locally. */
  source: "server" | "app";
  /** Optional URL for a CTA button ("Open dashboard", "Upgrade", etc.). */
  action_url?: string;
  action_label?: string;
  /** Multi-button form (server migration 0049). When present it replaces the
   *  single `action_url` pair; when absent that pair is still honoured, so
   *  messages sent before 0049 keep their button. */
  actions?: NotificationAction[];
};

/** Shape of the `hoard://notification` Tauri event payload (from Rust). */
export type ServerNotification = {
  id: string;
  title: string;
  body: string;
  priority?: NotificationPriority;
  action_url?: string;
  action_label?: string;
  actions?: NotificationAction[];
};

const STORAGE_KEY = "hoard-notifications";
const MAX_ENTRIES = 20;
const EXPIRY_MS = 7 * 86_400_000; // 7 days for normal/low

export const notifications = writable<AppNotification[]>(readStored());

function readStored(): AppNotification[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const all = JSON.parse(raw) as AppNotification[];
    // Drop expired normal/low; keep all high.
    const cutoff = Date.now() - EXPIRY_MS;
    const kept = all.filter((n) => n.priority === "high" || n.at >= cutoff);
    if (kept.length !== all.length) persist(kept);
    return kept;
  } catch {
    return [];
  }
}

function persist(list: AppNotification[]): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify(list.slice(0, MAX_ENTRIES)),
    );
  } catch {
    /* storage disabled / quota, the in-memory list still works */
  }
}

// ---------------------------------------------------------------------------
//  Dismissing a broadcast
// ---------------------------------------------------------------------------
//  A broadcast is dismissed on the SERVER (per user, across devices), the
//  local list is only a cache, and `reconcileServer` replaces it wholesale
//  with whatever the server still serves. So dropping the entry locally is
//  not enough: without telling the server, the next snapshot puts it right
//  back. That's exactly what happened with the "Hello World" test broadcast,
//  dismissed, back on restart, forever, while the other one seemed to
//  dismiss fine only because it had expired server-side.
//
//  Two halves, and both are needed:
//    - POST the dismissal so the server stops serving it everywhere.
//    - Keep a local tombstone so the in-flight snapshot (or a failed POST)
//      can't resurrect it in the meantime.

const DISMISSED_KEY = "hoard-notifications-dismissed";

function readDismissed(): Set<string> {
  try {
    const raw = localStorage.getItem(DISMISSED_KEY);
    return new Set(raw ? (JSON.parse(raw) as string[]) : []);
  } catch {
    return new Set();
  }
}

let dismissedIds = readDismissed();
/** Ids with a dismissal POST in flight, so retries don't pile up. */
const dismissing = new Set<string>();

function persistDismissed(): void {
  try {
    localStorage.setItem(DISMISSED_KEY, JSON.stringify([...dismissedIds]));
  } catch {
    /* storage disabled, the server-side dismissal still carries it */
  }
}

/** Record the dismissal server-side. Best-effort: if it fails (offline, token
 *  mid-rotation) the tombstone keeps the entry hidden here, and the next
 *  snapshot that still carries the id retries it. */
async function tellServer(id: string): Promise<void> {
  if (dismissing.has(id)) return;
  dismissing.add(id);
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("notification_dismiss", { id });
  } catch {
    /* retried from reconcileServer while the server keeps serving it */
  } finally {
    dismissing.delete(id);
  }
}

let counter = 0;

/** Push a notification from the app side. Deduped by id if provided. */
export function pushNotification(
  n: Omit<AppNotification, "id" | "at" | "source"> & { id?: string },
): void {
  const id = n.id ?? `app-${Date.now()}-${counter++}`;
  notifications.update((list) => {
    if (list.some((x) => x.id === id)) return list;
    const entry: AppNotification = {
      id,
      title: n.title,
      body: n.body,
      priority: n.priority,
      at: Date.now(),
      source: "app",
      action_url: n.action_url,
      action_label: n.action_label,
      actions: n.actions,
    };
    const next = [entry, ...list].slice(0, MAX_ENTRIES);
    persist(next);
    return next;
  });
}

/** Dismiss a single notification by id. Server broadcasts are dismissed on
 *  the server too, so they stay gone here, on your other machines, and after
 *  a reinstall. */
export function dismissNotification(id: string): void {
  let fromServer = false;
  notifications.update((list) => {
    fromServer = list.some((n) => n.id === id && n.source === "server");
    const next = list.filter((n) => n.id !== id);
    persist(next);
    return next;
  });
  if (fromServer) forgetServerSide(id);
}

/** Clear all. Every server broadcast in the list is dismissed server-side,
 *  same as clearing them one by one. */
export function clearNotifications(): void {
  let serverIds: string[] = [];
  notifications.update((list) => {
    serverIds = list.filter((n) => n.source === "server").map((n) => n.id);
    return [];
  });
  persist([]);
  for (const id of serverIds) forgetServerSide(id);
}

function forgetServerSide(id: string): void {
  dismissedIds.add(id);
  persistDismissed();
  void tellServer(id);
}

/** Unread count, for a badge on the bell. Simplified: all non-dismissed are
 *  "unread". A read/unread flag can be added later if needed. */
export { notifications as unreadCount };

let serverListener: (() => void) | null = null;

/** Reconcile the store against the server's authoritative broadcast list.
 *
 *  The server (cloud/routes/notifications.rs) returns the FULL set of
 *  broadcasts this user should currently see, already filtered by signup
 *  date, expiry and per-user dismissals. So this is a replace, not a merge,
 *  for server-sourced entries: add rows that are new, and DROP server rows the
 *  server no longer delivers (expired, or dismissed on another device). App-
 *  sourced notifications (`source: "app"`) are untouched. Existing `at`
 *  timestamps are preserved so the ordering doesn't churn on every snapshot. */
function reconcileServer(rows: ServerNotification[]): void {
  // A tombstoned id still in the list means the server never got (or hasn't
  // yet applied) the dismissal, offline when the user clicked, say. Retry it
  // now and keep the row hidden meanwhile.
  const served = new Set(rows.map((r) => r.id));
  for (const id of dismissedIds) {
    if (served.has(id)) void tellServer(id);
  }
  // Tombstones for rows the server no longer serves have done their job (the
  // dismissal landed, or it expired); drop them so the list can't grow
  // without bound.
  const stale = [...dismissedIds].filter((id) => !served.has(id));
  if (stale.length > 0) {
    for (const id of stale) dismissedIds.delete(id);
    persistDismissed();
  }

  notifications.update((list) => {
    const prevAt = new Map(list.map((n) => [n.id, n.at]));
    const app = list.filter((n) => n.source === "app");
    const server: AppNotification[] = rows
      .filter((p) => !dismissedIds.has(p.id))
      .map((p) => ({
        id: p.id,
        title: p.title,
        body: p.body,
        priority: p.priority ?? "normal",
        at: prevAt.get(p.id) ?? Date.now(),
        source: "server",
        action_url: p.action_url,
        action_label: p.action_label,
        actions: p.actions,
      }));
    // Newest first across both sources.
    const next = [...server, ...app]
      .sort((a, b) => b.at - a.at)
      .slice(0, MAX_ENTRIES);
    persist(next);
    return next;
  });
}

/** Subscribe to `hoard://notifications-snapshot` Tauri events (server →
 *  client) and pull the boot-time backlog once the listener is armed. Call
 *  once at boot; the listener lives for the app's lifetime. Idempotent.
 *
 *  Two delivery paths, same reconcile: the Rust `cloud_feed` poller/Realtime
 *  emits `hoard://notifications-snapshot` with the full filtered list, and the
 *  `notifications_backlog` command returns that same list for the events that
 *  fired while the webview was still mounting. */
export async function initServerNotifications(): Promise<void> {
  if (serverListener) return;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{ notifications: ServerNotification[] }>(
      "hoard://notifications-snapshot",
      (e) => reconcileServer(e.payload?.notifications ?? []),
    );
    serverListener = unlisten;
    // Race-free catch-up: any snapshot emitted before this listener attached
    // (broadcast sent while the app was closed, or during webview load) is
    // recovered by pulling the backlog now.
    const { invoke } = await import("@tauri-apps/api/core");
    const rows = await invoke<ServerNotification[]>("notifications_backlog");
    reconcileServer(rows ?? []);
  } catch {
    /* Tauri not available (e.g. dev in browser), no-op */
  }
}

/** Escape HTML, then apply a tiny markdown subset. Returns HTML string safe
 *  to render with {@html}. No raw HTML in the input survives the escape. */
export function renderMarkdown(md: string): string {
  // 1. Escape everything.
  let s = md.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  // 2. Inline code first (so its contents aren't mangled by bold/italic).
  s = s.replace(/`([^`]+)`/g, "<code>$1</code>");
  // 3. Bold.
  s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  // 4. Italic.
  s = s.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  // 5. Links [text](url), only http/https, no javascript:.
  s = s.replace(
    /\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-emerald-400 hover:text-emerald-300 underline">$1</a>',
  );
  // 6. Line breaks.
  s = s.replace(/\n/g, "<br>");
  return s;
}
