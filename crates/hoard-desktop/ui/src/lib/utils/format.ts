/**
 * Human-friendly byte formatting for the sidebar plan-usage indicator and
 * any other UI surface that surfaces storage numbers.
 *
 * Uses a 1024-base (binary) divisor but keeps the consumer-friendly
 * decimal labels (`kB`, `MB`, `GB`, `TB`) the rest of the dashboard already
 * uses. A perfect KiB/MiB pedant would object; in practice every consumer
 * storage UI on the market (Dropbox, iCloud, Drive) shows the same
 * compromise, so users read "1.0 GB" the way they expect.
 *
 * `0` short-circuits to `"0 B"`. The `B` unit suppresses decimals because
 * "512.0 B" looks silly; everything else honors `decimals` (default 1).
 */
import { get } from "svelte/store";
import { _ } from "svelte-i18n";

export function formatBytes(bytes: number, decimals = 1): string {
  if (!bytes || bytes <= 0) return "0 B";

  const units = ["B", "kB", "MB", "GB", "TB", "PB"];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const value = bytes / Math.pow(k, i);
  const places = i === 0 ? 0 : decimals;
  return `${value.toFixed(places)} ${units[i]}`;
}

/**
 * "elden-ring" -> "Elden Ring". Purely cosmetic fallback for the visible
 * game name while the user hasn't set a per-device override (gameNames
 * store). The slug itself, the sync key, is never touched.
 */
export function prettifySlug(slug: string): string {
  const pretty = slug
    .split(/[-_]+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
  return pretty || slug;
}

/**
 * Localised "x ago" for an ISO timestamp, relative to `now` (epoch ms).
 * Reuses the History route's relative strings so the two panels read the
 * same; `dashboard.time_yesterday` covers the 1-day case the mockup shows.
 * Future dates (clock skew) collapse to "just now".
 */
export function formatRelativeTime(iso: string, now: number = Date.now()): string {
  const t = get(_);
  const diff = Math.max(0, Math.floor((now - new Date(iso).getTime()) / 1000));
  if (diff < 60) return t("history.relative_just_now");
  if (diff < 3600) {
    return t("history.relative_minutes", {
      values: { count: Math.floor(diff / 60) },
    });
  }
  if (diff < 86400) {
    return t("history.relative_hours", {
      values: { count: Math.floor(diff / 3600) },
    });
  }
  if (diff < 172800) return t("dashboard.time_yesterday");
  return t("history.relative_days", {
    values: { count: Math.floor(diff / 86400) },
  });
}

/** Absolute companion for the relative time: "21/07/2026 17:47"-style,
 *  locale-aware via `toLocaleString`. Seconds are noise at this granularity. */
export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
