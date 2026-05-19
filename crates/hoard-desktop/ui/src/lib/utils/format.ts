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
export function formatBytes(bytes: number, decimals = 1): string {
  if (!bytes || bytes <= 0) return "0 B";

  const units = ["B", "kB", "MB", "GB", "TB", "PB"];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const value = bytes / Math.pow(k, i);
  const places = i === 0 ? 0 : decimals;
  return `${value.toFixed(places)} ${units[i]}`;
}
