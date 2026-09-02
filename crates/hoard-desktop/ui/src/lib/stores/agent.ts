/**
 * Live-agent store.
 *
 * The sync engine emits events through the `agent://*` Tauri event
 * channels. This store subscribes to them once at app boot, keeps a
 * per-save activity status in memory, and lets pages reactively render
 * "backing up…", "next in 30s", "failed (will retry)", etc.
 *
 * Since ADR 0021 slice 4b the engine no longer lives inside this process: it
 * runs in the `hoardd` service and the Rust side relays its events onto the
 * same channels. **This store's public surface is unchanged on purpose**, the
 * screens must not notice the backend swap. What's new is internal:
 *
 *  - `agent://backlog`, what the service journalled while we weren't
 *    connected (the app was closed, or we reconnected). Applied like any other
 *    event, but *silently*: replaying yesterday's backups must not fire today's
 *    toasts and notifications.
 *  - `agent://daemon-status`, the engine went up or down inside the service.
 *    Engine liveness isn't an event in the journal, so this is how the tray
 *    stops lying when the service starts its engine 20s after we attached.
 *
 * Activity is keyed by `save_id`; pages look up their saves by id.
 *
 * The store is also responsible for two side effects that need to react to
 * the same event stream:
 *
 *  1. **Tray colouring**, we collapse all per-save states into a single
 *     "global state" and push it down to Rust so the tray icon recolours.
 *  2. **Desktop notifications**, a fallback now, not the source. Since ADR
 *     0021 D.14.1 the *service* fires them, which is the only way they arrive
 *     with the app closed; it says so in `AgentStatus.service_notifies` and
 *     then `notify()` here stays quiet. On a platform where the service can't
 *     yet (Windows, macOS) the flag is `false` and this store notifies exactly
 *     as it always did.
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
import { seedWatcher } from "./live";
import { toastSuccess } from "./toasts";

export type SaveActivity = {
  state:
    | "idle"
    | "running" // game running
    | "scheduled"
    | "uploading"
    | "ok"
    | "partial" // backed up, but the plan's per-save cap dropped old files
    | "failed";
  /** When the next backup is expected to fire (epoch ms), if scheduled. */
  next_backup_at?: number;
  /** When the current game session started (epoch ms), if running. Drives
   *  the elapsed-time counter in the Eye panel. */
  running_since?: number;
  /** The game this save belongs to, as it rides on the events. The map is
   *  keyed by `save_id`, a UUID, so anything that wants to *name* what is
   *  running needs the slug kept next to it. */
  game_slug?: string;
  reason?: BackupReason;
  /** The version **this device** now holds, set by an upload that committed
   *  and by an auto-restore that landed. Never the cloud head: the panel keeps
   *  those apart on purpose (ADR 0021 D.10). */
  last_version?: number;
  last_bytes?: number;
  error?: string;
  will_retry?: boolean;
};

type ActivityMap = Record<string, SaveActivity>;

export const activity: Writable<ActivityMap> = writable({});

/** A save whose cloud auto-restore keeps failing on the same version. */
export type StuckRestore = {
  /** Consecutive failures, so the badge can say "3×" and read as a pattern. */
  failures: number;
  /** Last error chain, the badge's tooltip, so the user sees *why*. */
  error: string;
};

/** Saves currently stuck on a failing cloud restore, keyed by `save_id`.
 *
 * Deliberately separate from `activity`: that map tracks the *current* backup
 * lifecycle and gets overwritten by the next event, whereas this is a sticky
 * condition that must outlive unrelated activity. It's the persistent half of
 * the July-2026 fix, the transient toast is what let a save silently fail to
 * sync for eight days. Cleared only by a recovery (success or a new cloud
 * version), never by time passing. */
export const restoreStuck: Writable<Record<string, StuckRestore>> = writable(
  {},
);

/** Saves whose tracked folder looks wrong: empty, and never once backed up.
 *
 * Same shape and reasoning as `restoreStuck`, a sticky condition, not an
 * activity blip. It deliberately does NOT toast: the reconciliation sweep
 * re-checks the folder every cycle, so a toast would fire forever. Instead the
 * Library card carries an amber hint until a backup actually lands, which is
 * also the only thing that can clear it. */
export const wrongPathSuspected: Writable<Record<string, true>> = writable({});

/** A save whose last backup left files out because it could not read them. */
export type UnreadableFiles = {
  /** How many were left out, so the notice can say "3 files". */
  count: number;
  /** One example path and its system error: the tooltip, and the only thing that
   *  tells "start OneDrive" apart from "check the permissions". */
  path: string;
  error: string;
  /** `false` means nothing at all was uploaded (not one readable file). */
  uploaded: boolean;
};

/** Saves whose last backup came out incomplete (or did not come out at all), by
 * `save_id`.
 *
 * The same sticky treatment as `restoreStuck`: this is not a flicker of activity
 * but a condition that lasts as long as its cause (an un-hydrated OneDrive
 * placeholder can sit like that for weeks), and a version missing a file with
 * nobody saying so is exactly the failure you only discover when restoring. No
 * toast, on purpose: the condition repeats on every backup and the notice would
 * never stop. The first complete backup clears it (`backup_success` arrives
 * **before** this event, so a backup that is still partial puts it back in the
 * same tick). */
export const filesUnreadable: Writable<Record<string, UnreadableFiles>> =
  writable({});

/** A save whose upload gave up against a conflict it cannot resolve. */
export type BlockedBackup = {
  /** Consecutive conflicts on giving up, so the notice reads as a pattern. */
  conflicts: number;
  /** The last error string: the tooltip. */
  error: string;
};

/** Saves whose upload **has stopped retrying** and is waiting on the user, by
 * `save_id`.
 *
 * Sticky like `restoreStuck`, and for exactly the same reason: the case that
 * motivated it had been retrying every 13 minutes for 14 days with nothing saying
 * so. The difference from an ordinary failure is that there is no retry left to
 * wait for; somebody has to press "back up now", or another machine has to publish
 * a new version. `backup_attention_cleared` clears it. */
export const backupBlocked: Writable<Record<string, BlockedBackup>> = writable(
  {},
);
export const status: Writable<AgentStatus> = writable({
  running: false,
  watched_count: 0,
  // Nothing has told us anything yet. Every screen that paints a "the service
  // is down" state has to wait for this to flip, or it paints it during the
  // second the window takes to boot, which is what the dashboard did.
  known: false,
});

/** Adopt a status that actually came from the service. The one place `known`
 *  gets set, so no caller can forget it and reintroduce the blank-reads-as-down
 *  banner. */
function adoptStatus(s: AgentStatus) {
  status.set({ ...s, known: true });
}

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
  // "partial" still means the last sync succeeded, the tray stays green so we
  // don't cry wolf; the plan-limit warning lives on the save row + toast.
  if (states.some((s) => s === "ok" || s === "partial")) return "ok";
  return "idle";
});

let unlisteners: UnlistenFn[] = [];
let lastTrayState: TrayStateName | null = null;
let trayUnsub: (() => void) | null = null;
let notificationsAllowed = false;

async function ensureNotificationPermission() {
  // Ask once on startup. If the user denies, future toasts no-op silently,
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

/** True while we're replaying the service's journal. A backlog row is
 *  history, not news: it must rebuild the on-screen state without popping a
 *  toast or a system notification for something that happened while the app
 *  was closed. */
let replaying = false;

function notify(title: string, body: string) {
  if (replaying) return;
  // The service notifies by itself where it can (Linux today; ADR 0021 D.14.1).
  // Sending ours on top would show the same backup twice whenever the window
  // happens to be open, and the whole point of moving them to the service is
  // that they also arrive when it isn't. Where the service can't yet (Windows,
  // macOS) the flag is `false` and this stays exactly as it was.
  if (get(status).service_notifies) return;
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

/** Apply one event. `at` is when it actually happened (epoch ms), it defaults
 *  to "now" for live events and carries the journal's timestamp when we're
 *  replaying, so a session that started two hours ago reads as two hours and
 *  not as zero. */
function applyEvent(ev: AgentEvent, at: number = Date.now()) {
  switch (ev.type) {
    case "game_started":
      patch(ev.save_id, {
        state: "running",
        running_since: at,
        game_slug: ev.game_slug,
      });
      break;
    case "game_stopped":
      patch(ev.save_id, { state: "idle", running_since: undefined });
      break;
    case "backup_scheduled":
      patch(ev.save_id, {
        state: "scheduled",
        next_backup_at: at + ev.delay_ms,
        reason: ev.reason,
      });
      break;
    case "backup_started":
      patch(ev.save_id, { state: "uploading", next_backup_at: undefined });
      break;
    case "backup_success": {
      // A real backup is the only proof the folder was the right one.
      wrongPathSuspected.update((m) => {
        if (!(ev.save_id in m)) return m;
        const { [ev.save_id]: _gone, ...rest } = m;
        return rest;
      });
      // And a complete backup is the only proof everything reads now. The engine
      // sends `backup_files_unreadable` right AFTER the success when something is
      // still missing, so clearing here erases no live notice.
      filesUnreadable.update((m) => {
        if (!(ev.save_id in m)) return m;
        const { [ev.save_id]: _gone, ...rest } = m;
        return rest;
      });
      patch(ev.save_id, {
        state: "ok",
        last_version: ev.version_num,
        // With `already_landed` not one byte travelled (the content was already up
        // there), so painting 0 B would erase the size of the last real backup. The
        // version number is new, and it is adopted.
        ...(ev.already_landed ? {} : { last_bytes: ev.total_bytes }),
        error: undefined,
        will_retry: undefined,
      });
      const $prefs = get(prefs);
      // Notifying about a backup that did not happen now is the same class of lie
      // as chiming while replaying the journal: the state is updated, the notice is
      // not sent.
      if ($prefs?.notify_on_success && !ev.already_landed) {
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
    case "backup_files_unreadable": {
      filesUnreadable.update((m) => ({
        ...m,
        [ev.save_id]: {
          count: ev.count,
          path: ev.sample_path,
          error: ev.sample_error,
          uploaded: ev.uploaded,
        },
      }));
      // Nada legible en toda la carpeta: eso no es una copia parcial, es una
      // copia que no existe, y la fila tiene que verse rota aunque el motor
      // vaya a reintentarlo solo.
      if (!ev.uploaded) {
        patch(ev.save_id, {
          state: "failed",
          error: ev.sample_error,
          will_retry: true,
        });
      }
      break;
    }
    case "save_auto_restored": {
      // A restore that landed moves *this device's* version forward, so record
      // it: without this the dashboard keeps flagging the cloud as ahead until
      // the next full `list_tracked_saves`. Deliberately doesn't touch `state`
      //, a restore isn't a backup lifecycle transition.
      patch(ev.save_id, { last_version: ev.version_num });
      // Beyond that it's a one-shot adoption side-effect, but we do want to
      // surface a toast so the user notices files appeared under `~`. The
      // `game_slug` is what we have to hand; resolving to display_name would
      // require another store dependency just for cosmetics.
      // Not while replaying the journal: a restore that landed last night is
      // history, and the Library row already shows its version.
      if (!replaying) {
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
      }
      break;
    }
    case "save_auto_restore_failed": {
      // Surfaced in the activity feed (see live.ts), not as a toast. A stale
      // session token at launch made this fire once per tracked save, a burst
      // of popups every start. The agent now suppresses the transient 401 at
      // source; genuine per-save failures still land in the feed.
      break;
    }
    case "save_auto_restore_stuck": {
      // The agent only sends this once per (save, version), after repeated
      // failures, so it's rare enough to deserve a notification, and the
      // Library card keeps an amber badge until it recovers.
      restoreStuck.update((m) => ({
        ...m,
        [ev.save_id]: { failures: ev.failures, error: ev.error },
      }));
      // Honour the same pref as backup failures: the card badge is the part
      // the user can't miss, so a silenced notification loses nothing.
      const $prefs = get(prefs);
      if ($prefs?.notify_on_failure) {
        const t = get(i18n);
        notify(
          t("library.restore_stuck_notify_title"),
          t("library.restore_stuck_notify_body", {
            values: { name: ev.game_slug, count: ev.failures },
          }),
        );
      }
      break;
    }
    case "backup_needs_attention": {
      backupBlocked.update((m) => ({
        ...m,
        [ev.save_id]: { conflicts: ev.conflicts, error: ev.error },
      }));
      patch(ev.save_id, {
        state: "failed",
        error: ev.error,
        // What separates this from an ordinary failure: there is no retry coming.
        // Saying "retrying" would be exactly the lie that kept the real case
        // invisible for two weeks.
        will_retry: false,
      });
      // Once per jam (the engine emits it on the edge), so it does deserve a
      // notification. The same pref as the other failures.
      const $prefs = get(prefs);
      if ($prefs?.notify_on_failure) {
        const t = get(i18n);
        notify(
          t("library.backup_blocked_notify_title"),
          t("library.backup_blocked_notify_body", {
            values: { name: ev.game_slug, count: ev.conflicts },
          }),
        );
      }
      break;
    }
    case "backup_attention_cleared": {
      backupBlocked.update((m) => {
        if (!(ev.save_id in m)) return m;
        const { [ev.save_id]: _gone, ...rest } = m;
        return rest;
      });
      break;
    }
    case "save_auto_restore_recovered": {
      restoreStuck.update((m) => {
        if (!(ev.save_id in m)) return m;
        const next = { ...m };
        delete next[ev.save_id];
        return next;
      });
      break;
    }
    case "backup_too_large": {
      // The upload was refused as too big, so it can never succeed as-is. Not a
      // transient failure and not "retrying", say it once, actionably, and mark
      // the row failed-without-retry so it stops spamming every folder change.
      //
      // Which sentence depends on WHO refused it, because the fix is somewhere
      // different in each case: the plan (upgrade), the user's own server
      // (`max_snapshot_size_mb`), or a proxy in front of it (nothing in Hoard
      // will help). `kind` comes from the agent; the old code only looked at
      // whether `limit_bytes` was non-zero, which lumped the last two together
      // and sent a self-hoster hunting through proxy configs for days.
      let msg: string;
      switch (ev.kind) {
        case "server_limit":
          msg = get(i18n)("library.backup_too_large_server_toast", {
            values: { name: ev.game_slug, limit: formatBytes(ev.limit_bytes) },
          });
          break;
        case "proxy":
          msg = get(i18n)("library.backup_too_large_proxy_toast", {
            values: { name: ev.game_slug },
          });
          break;
        default:
          msg =
            ev.limit_bytes > 0
              ? get(i18n)("library.backup_too_large_toast", {
                  values: {
                    name: ev.game_slug,
                    limit: formatBytes(ev.limit_bytes),
                    size: formatBytes(ev.actual_bytes),
                  },
                })
              : get(i18n)("library.backup_too_large_generic_toast", {
                  values: { name: ev.game_slug },
                });
      }
      patch(ev.save_id, { state: "failed", error: msg, will_retry: false });
      // No toast: the activity feed carries this now (see live.ts). Native
      // notification stays behind the user's opt-in `notify_on_failure`.
      if (get(prefs)?.notify_on_failure) notify("Backup failed", msg);
      break;
    }
    case "backup_trimmed": {
      // The backup succeeded but the plan's per-save cap forced us to upload
      // only the newest files. Not an error (a backup_success already fired),
      // but the user must know their plan isn't enough for this save: mark the
      // row amber "partial" and warn once.
      const msg = get(i18n)("library.backup_trimmed_toast", {
        values: {
          name: ev.game_slug,
          omitted: ev.omitted_files,
          size: formatBytes(ev.omitted_bytes),
        },
      });
      patch(ev.save_id, { state: "partial", error: msg, will_retry: false });
      // No toast, surfaced in the activity feed (see live.ts).
      break;
    }
    case "backup_skipped_empty": {
      // The folder resolved to empty/missing so we did *not* push an empty
      // snapshot (that would silently overwrite the user's last good copy on
      // the server). This is a benign, recurring condition, the
      // reconciliation sweep re-checks the folder every cycle, and a
      // mis-detected/empty tracked folder (or a game that never wrote a save
      // yet) would fire this on every tick. So don't toast or notify; just
      // reflect the save as idle. Users who want the cloud copy pulled into an
      // empty folder enable "Restore from cloud when a save folder is empty"
      // in Settings.
      //
      // The one case worth surfacing is a save that has *never* backed up and
      // is empty: that's a mis-tracked folder, not a game between saves. It
      // goes to a sticky flag on the card (not a toast, see the store).
      if (ev.likely_wrong_path) {
        wrongPathSuspected.update((m) => ({ ...m, [ev.save_id]: true }));
      }
      patch(ev.save_id, {
        state: "idle",
        error: undefined,
        will_retry: undefined,
      });
      break;
    }
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(0)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** The service's journal, replayed. `resync` means we can't claim continuity
 *  with what we already had, the ring dropped rows, or the service restarted,
 *  so the event-derived state is rebuilt instead of patched. */
type BacklogRow = { at: number; event: AgentEvent };
type BacklogPayload = { rows: BacklogRow[]; resync: boolean };

function applyBacklog(payload: BacklogPayload) {
  if (payload.resync) {
    // Rebuild rather than pretend: a gap means some of what we hold may never
    // get its closing event. Both maps are derived purely from the event
    // stream, so replaying the journal restores them.
    activity.set({});
    restoreStuck.set({});
  }
  replaying = true;
  try {
    // Chronological order, oldest first: the last event per save wins, exactly
    // as if we'd been connected the whole time.
    for (const row of payload.rows) applyEvent(row.event, row.at);
  } finally {
    replaying = false;
  }
}

/** Subscribe to all `agent://*` channels. Idempotent, safe to call from
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
    "agent://backup-too-large",
    "agent://backup-trimmed",
    "agent://backup-files-unreadable",
    "agent://backup-needs-attention",
    "agent://backup-attention-cleared",
    "agent://save-auto-restored",
    "agent://save-auto-restore-failed",
    "agent://save-auto-restore-stuck",
    "agent://save-auto-restore-recovered",
    "agent://backup-skipped-empty",
  ];
  unlisteners = await Promise.all(
    topics.map((t) =>
      listen<AgentEvent>(t, (event) => applyEvent(event.payload)),
    ),
  );

  // Catch-up + engine liveness. Both come from the service through the Rust
  // side; the screens see only the same stores as before.
  unlisteners.push(
    await listen<BacklogPayload>("agent://backlog", (e) =>
      applyBacklog(e.payload),
    ),
  );
  unlisteners.push(
    await listen<AgentStatus>("agent://daemon-status", (e) => {
      adoptStatus(e.payload);
      seedWatcher(e.payload.watched_count);
    }),
  );

  // Mirror the derived tray state into Rust whenever it changes. We only
  // push when the value actually flips so the tray doesn't repaint every
  // tick of the FS-event firehose.
  trayUnsub = trayState.subscribe((s) => {
    if (s === lastTrayState) return;
    lastTrayState = s;
    api.setTrayState(s).catch((e) => console.warn("setTrayState failed:", e));
  });

  // Only now, with every listener registered, ask Rust to relay the service's
  // journal and live events. Do it the other way round and the backlog lands
  // before anyone is listening.
  await api
    .attachAgentEvents()
    .catch((e) => console.warn("attachAgentEvents failed:", e));
}

export async function unsubscribeAgent() {
  await api
    .detachAgentEvents()
    .catch((e) => console.warn("detachAgentEvents failed:", e));
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

/** In-flight `bootAgent` promise. Both `hydrateAuth` (self-hosted) and
 *  `hydrateCloud` (cloud) can race to boot the agent on a cold start where the
 *  user has both sessions, and each would otherwise redo the whole boot.
 *  Deduping here collapses them into one. */
let bootInFlight: Promise<void> | null = null;

/** Subscribe and make sure the sync service is up. Idempotent and safe to call
 *  from every login path (self-hosted or cloud) and from the cold-start
 *  hydrate; concurrent calls share a single boot. */
export async function bootAgent() {
  if (bootInFlight) return bootInFlight;
  bootInFlight = (async () => {
    await ensureNotificationPermission();
    // Subscribing first is deliberate: it registers the listeners *and* starts
    // the relay, which retries on its own. Doing it after `startAgent` would
    // mean that a service that's briefly unreachable (an update restarting it)
    // leaves the UI deaf for the rest of the session, because the throw would
    // skip the subscription entirely.
    await subscribeAgent();
    const s = await api.startAgent();
    adoptStatus(s);
    // Seed the live header's watcher dot from the boot status: `watcher-armed`
    // is emitted per slot by the relay, and this is the number the service
    // reports right now, so the dot is honest from the first paint.
    seedWatcher(s.watched_count);
  })();
  try {
    await bootInFlight;
  } finally {
    bootInFlight = null;
  }
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
  // We stopped it ourselves, so this one *is* known.
  status.set({ running: false, watched_count: 0, known: true });
  // Drop the tray back to "offline" so the icon doesn't lie.
  api.setTrayState("offline").catch(() => {});
}
