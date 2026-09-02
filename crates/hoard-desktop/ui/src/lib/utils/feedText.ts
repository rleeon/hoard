/**
 * The feed events' text (`agent://*`), shared.
 *
 * It used to live inside `ActivityFeed.svelte`. Once a second surface showing the
 * same events appeared, the HUD over the game, duplicating it guaranteed the two
 * texts would drift apart on the first line anybody touched, so it lives here.
 *
 * It takes the translator as a parameter rather than importing `$_`: in a `.ts`
 * module the store would have to be read with `get()`, which freezes the language
 * at the moment of the call. Passed in from the component (`$_`), the text
 * recomputes on its own when the user changes language.
 */
import type { Readable } from "svelte/store";
import { _ } from "svelte-i18n";

import type { FeedEntry } from "../stores/live";
import { formatBytes } from "./format";

/** `$_`'s exact type.
 *
 *  svelte-i18n declares `MessageFormatter` but does not export it, so it is taken
 *  off the `_` store, which is exported. Writing the signature by hand looked
 *  cleaner and is not: the library constrains the interpolation values and a laxer
 *  signature does not fit what the components pass. */
type Unwrap<T> = T extends Readable<infer U> ? U : never;
export type Translate = Unwrap<typeof _>;

export function feedRelativeTime(at: number, $_: Translate): string {
  const seconds = Math.round((Date.now() - at) / 1000);
  if (seconds < 5) return $_("activity.time_just_now");
  if (seconds < 60)
    return $_("activity.time_seconds_ago", { values: { count: seconds } });
  const minutes = Math.round(seconds / 60);
  if (minutes < 60)
    return $_("activity.time_minutes_ago", { values: { count: minutes } });
  const hours = Math.round(minutes / 60);
  return $_("activity.time_hours_ago", { values: { count: hours } });
}

export function feedSummary(e: FeedEntry, $_: Translate): string {
  const name = e.game_slug ?? e.save_id?.slice(0, 8) ?? "—";
  switch (e.kind) {
    case "watcher_armed":
      return $_("activity.watcher_armed", { values: { name } });
    case "game_started":
      return $_("activity.game_started", { values: { name } });
    case "game_stopped":
      return $_("activity.game_stopped", { values: { name } });
    case "throttled":
      return $_("activity.throttled", { values: { name } });
    case "upload_started":
      return $_("activity.upload_started", { values: { name } });
    case "upload_completed":
      return $_("activity.upload_completed", {
        values: {
          name,
          version: e.version ?? 0,
          size: formatBytes(e.bytes ?? 0),
        },
      });
    case "upload_failed":
      return $_("activity.upload_failed", {
        values: { name, error: e.error ?? "" },
      });
    case "bandwidth_throttled":
      return $_("activity.bandwidth_throttled", {
        values: { name, seconds: e.retry_in ?? 60 },
      });
    case "auto_restored":
      return $_("activity.auto_restored", {
        values: { name, version: e.version ?? 0 },
      });
    case "cloud_pull":
      return $_("activity.cloud_pull", {
        values: {
          count: e.new_versions ?? 0,
          size: formatBytes(e.bytes ?? 0),
        },
      });
    case "quota_reached":
      return $_("activity.quota_reached", {
        values: { plan: e.plan ?? "free", seconds: e.retry_in ?? 60 },
      });
    case "offline":
      return $_("activity.offline");
    case "online":
      return $_("activity.online");
    case "backup_too_large":
      // Which sentence depends on who refused it, because the fix is in a
      // different place each time. The feed used to say "Upgrade to Pro" no
      // matter what, including to self-hosters, whose server had simply hit
      // `max_snapshot_size_mb`, and with `{size}` rendered as "0 B" because
      // only Cloud knows the save's real size up front.
      if (e.too_large_kind === "server_limit" && (e.limit_bytes ?? 0) > 0) {
        return $_("library.backup_too_large_server_toast", {
          values: { name, limit: formatBytes(e.limit_bytes ?? 0) },
        });
      }
      if (e.too_large_kind === "proxy") {
        return $_("library.backup_too_large_proxy_toast", { values: { name } });
      }
      if ((e.limit_bytes ?? 0) === 0 || (e.bytes ?? 0) === 0) {
        return $_("library.backup_too_large_generic_toast", {
          values: { name },
        });
      }
      return $_("activity.backup_too_large", {
        values: {
          name,
          size: formatBytes(e.bytes ?? 0),
          limit: formatBytes(e.limit_bytes ?? 0),
        },
      });
    case "backup_quota_full":
      return $_("activity.backup_quota_full", {
        values: {
          used: formatBytes(e.bytes ?? 0),
          limit: formatBytes(e.limit_bytes ?? 0),
        },
      });
    case "backup_trimmed":
      return $_("activity.backup_trimmed", {
        values: {
          name,
          count: e.count ?? 0,
          size: formatBytes(e.bytes ?? 0),
        },
      });
    case "backup_files_unreadable":
      return $_("activity.backup_files_unreadable", {
        values: { name, count: e.count ?? 0, error: e.error ?? "" },
      });
    case "auto_restore_failed":
      return $_("activity.auto_restore_failed", {
        values: { name, error: e.error ?? "" },
      });
    case "auto_restore_stuck":
      return $_("activity.auto_restore_stuck", {
        values: { name, count: e.failures ?? 0, error: e.error ?? "" },
      });
    case "auto_restore_recovered":
      return $_("activity.auto_restore_recovered", { values: { name } });
    case "backup_blocked":
      return $_("activity.backup_blocked", {
        values: { name, count: e.failures ?? 0, error: e.error ?? "" },
      });
    case "backup_unblocked":
      return $_("activity.backup_unblocked", { values: { name } });
    case "storage_purging":
      return $_("activity.storage_purging");
    case "storage_full":
      return $_("activity.storage_full");
    case "storage_grace":
      return $_("activity.storage_grace");
    case "gate_locked":
      return $_("activity.gate_locked", {
        values: {
          reason: $_(e.reason_key ?? "activity.gate_reason_fetch_failed"),
        },
      });
    case "gate_unlocked":
      return $_("activity.gate_unlocked", {
        values: {
          reason: $_(e.reason_key ?? "activity.gate_reason_pro"),
        },
      });
  }
}
