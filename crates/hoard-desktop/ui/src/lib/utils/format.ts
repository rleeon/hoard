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
import { derived } from "svelte/store";
import { _, locale } from "svelte-i18n";

/**
 * The BCP-47 tag `Intl` should format with: the language the user picked in the
 * app, never the machine's.
 *
 * Passing `undefined` to `toLocaleDateString` (which is what every call site used
 * to do) means "whatever the system says", so an English UI on a Spanish desktop
 * printed "domingo, 21 de septiembre" between English sentences. Plain `en` would
 * swing to the other extreme and print month-first American dates, so English maps
 * to `en-GB` and keeps the day-first order the rest of the app shows.
 */
export function intlLocale(tag: string | null | undefined): string {
  const code = (tag ?? "en").slice(0, 2).toLowerCase();
  return code === "en" ? "en-GB" : code;
}

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
 *
 * A store, not a plain function, so the text repaints when the language changes:
 * reading `_` with `get()` froze whatever language was active at first render.
 */
export const fmtRelativeTime = derived(_, (t) => (iso: string, now: number = Date.now()): string => {
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
});

/** `toLocaleDateString` in the app's language. A store so the date repaints when
 *  the language changes, and so no call site has to remember `intlLocale`. */
export const fmtDate = derived(
  locale,
  (l) =>
    (iso: string | number | Date, opts?: Intl.DateTimeFormatOptions): string =>
      new Date(iso).toLocaleDateString(intlLocale(l), opts),
);

/** Absolute companion for the relative time: "21/07/2026 17:47"-style. Seconds are
 *  noise at this granularity. */
export const fmtDateTime = derived(
  locale,
  (l) =>
    (iso: string | number | Date, opts?: Intl.DateTimeFormatOptions): string =>
      new Date(iso).toLocaleString(intlLocale(l), {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        ...opts,
      }),
);

/** Thousands separators in the app's language: the catalogue's "21.926 games"
 *  read as a version number in an otherwise English window. */
export const fmtNumber = derived(
  locale,
  (l) =>
    (n: number, opts?: Intl.NumberFormatOptions): string =>
      n.toLocaleString(intlLocale(l), opts),
);

/** A readable name for a game we only know by its slug ("terraforming-mars" ->
 *  "Terraforming Mars"). For settings lists, where the catalogue's own title is
 *  not at hand and the raw slug reads like an error code. */
export function titleFromSlug(slug: string): string {
  return slug
    .split(/[-_]+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}
