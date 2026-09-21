// Small shared helpers for the Pro UI. Kept inside the private layer so Pro
// feature copy never lands in the public locale files.
import { derived } from "svelte/store";
import { locale } from "svelte-i18n";

/** Pick a string for the active locale, falling back to English. The Pro
 *  strings live here (not in the public `i18n/locales/*.json`) on purpose.
 *
 *  A store, not a plain function: reading the language with `get(locale)` made
 *  every `$tr(...)` a one-shot, so a language change mid-session repainted the
 *  `$_` text around it and left these two screens in whatever language they were
 *  mounted with. Subscribing is what makes them follow along. */
export const tr = derived(
  locale,
  (l) =>
    (dict: Record<string, string>): string => {
      const code = (l ?? "en").slice(0, 2);
      return dict[code] ?? dict.en;
    },
);

/** Human-readable byte size (1.5 GB, 320 MB, …). */
export function fmtBytes(n: number): string {
  if (!n || n < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
}
