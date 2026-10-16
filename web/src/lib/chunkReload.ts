/**
 * Recovery for a JS chunk that failed to download.
 *
 * Browsers remember a failed dynamic import for the life of the document: once
 * a chunk misses (the wifi still reconnecting as a laptop wakes up, a hover
 * preload fired in that same second), every later import of it rejects with the
 * same error, even with the network back. A tab left open for hours then showed
 * "500 Internal Error" on every link it had preloaded, or the nav's account chip
 * stuck on its loading placeholder, until a manual reload. Only a fresh
 * document clears it, so that is what this does.
 */

// Chrome and Edge: "Failed to fetch dynamically imported module", Firefox:
// "error loading dynamically imported module", Safari: "Importing a module
// script failed".
const CHUNK_ERROR = /dynamically imported module|Importing a module script failed/i;

const KEY = 'hoard:chunk-reload';
// A chunk that is really gone (not a blip) would fail again right after the
// reload; past this window it counts as a new failure and gets its own reload.
const LOOP_WINDOW_MS = 30_000;

export function isChunkError(e: unknown): boolean {
  return e instanceof Error && CHUNK_ERROR.test(e.message);
}

function reloadedMomentsAgo(): boolean {
  try {
    const last = JSON.parse(sessionStorage.getItem(KEY) ?? 'null');
    return last?.path === location.pathname && Date.now() - last.at < LOOP_WINDOW_MS;
  } catch {
    return false;
  }
}

function reload() {
  try {
    sessionStorage.setItem(KEY, JSON.stringify({ path: location.pathname, at: Date.now() }));
  } catch {
    // no sessionStorage: the reload still goes ahead, only the loop guard is lost
  }
  location.reload();
}

/**
 * Reload the page now, or as soon as the browser is back online. Returns false
 * when this same page was already reloaded for a chunk moments ago, so the
 * caller shows the error instead of looping.
 */
export function reloadAfterChunkError(): boolean {
  if (reloadedMomentsAgo()) return false;
  if (navigator.onLine) reload();
  else window.addEventListener('online', reload, { once: true });
  return true;
}
