/**
 * Game cover-art loader, backed by the on-device cache.
 *
 * The Rust `cover_bytes` command downloads each game's art once, persists it
 * under the app cache dir, and returns the raw JPEG bytes as an `ArrayBuffer`
 * thereafter, no network round-trip after the first sight, no canvas-tainting
 * cross-origin draws. Here we wrap those bytes in an object URL and memoise
 * the result so a given game is fetched/decoded at most once per session and
 * size. A `null` entry marks a permanent miss (no art / offline first run) so
 * callers fall back to the initial-letter placeholder without retrying.
 *
 * Ask for the size the frame is drawn at (see {@link coverSize}). The webview
 * keeps an image decoded at its own resolution, so a 600×900 cover in a 36 px
 * list icon costs 2.2 MB of memory for what a 96 px copy shows just as well.
 *
 * Covers are addressed by a **cover key**, not a Steam app id, see
 * {@link coverKey}. Rust owns the whole resolution chain behind that key
 * (local app id → catalog → our hosted index → Steam's fuzzy search), so the
 * UI never has to know which source a given game's art came from.
 *
 * Users can override any game's cover with a custom image stored locally,
 * saved as `{key}_custom.{ext}` in the same cache dir and taking priority over
 * anything downloaded.
 */
import { invoke } from "@tauri-apps/api/core";

const cache = new Map<string, string | null>();
const inflight = new Map<string, Promise<string | null>>();
const customCoverCache = new Map<string, boolean>();

/** Sniff the image type from its magic bytes. Steam serves JPEG and the hosted
 *  index can point anywhere, Minecraft's poster on Microsoft's CDN is a PNG,
 *  and a Blob is only as honest as the type you hand it. Browsers do sniff for
 *  `<img>`, so mislabelling happens to render, but the canvas paths in Map and
 *  WrappedCard deserve the truth. */
function mimeOf(buf: ArrayBuffer): string {
  const b = new Uint8Array(buf.slice(0, 12));
  if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return "image/png";
  if (b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46) return "image/gif";
  if (b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50) return "image/webp";
  return "image/jpeg";
}

/** Address a game's cover art.
 *
 *  The slug wins over the app id when both are known, and that ordering is
 *  load-bearing: the slug is the one identifier every screen has (SaveGameCard
 *  passes nothing else), so keying on it is what makes a custom cover set in
 *  the Library show up on the Map and in Wrapped too. Games with an app id and
 *  no slug, and there are none today, but the prop allows it, fall back to
 *  the id. Returns `null` when the caller knows neither, i.e. there is nothing
 *  to look up. */
export function coverKey(
  appId: number | null | undefined,
  slug: string | null | undefined,
): string | null {
  if (slug) return `slug-${slug}`;
  if (appId != null) return String(appId);
  return null;
}

/** The steps Rust keeps a scaled copy at (`SIZES` in `commands/covers.rs`),
 *  in device pixels of the frame's longer side. */
const SIZES = [96, 192, 384, 768];

/** The art as stored: frames past the largest step, and the Map. */
export const FULL = 0;

/** The step for a frame whose longer side is `px` device pixels. */
export function coverSize(px: number): number {
  return SIZES.find((s) => s >= px) ?? FULL;
}

/** Whether art fetched at step `have` is sharp enough for step `want`. */
export function coverFits(have: number, want: number): boolean {
  return have === FULL || (want !== FULL && have >= want);
}

/** Resolve a cover key to a usable `<img src>` object URL, or `null` if
 *  there's no cover to show. Safe to call repeatedly, memoised +
 *  de-duplicated per size. */
export function coverUrl(key: string, size: number = FULL): Promise<string | null> {
  const id = `${key}@${size}`;
  const hit = cache.get(id);
  if (hit !== undefined) return Promise.resolve(hit);
  const pending = inflight.get(id);
  if (pending) return pending;

  const p = (async () => {
    try {
      const buf = await invoke<ArrayBuffer>("cover_bytes", { key, size: size || null });
      const url = URL.createObjectURL(new Blob([buf], { type: mimeOf(buf) }));
      cache.set(id, url);
      return url;
    } catch {
      cache.set(id, null);
      return null;
    } finally {
      inflight.delete(id);
    }
  })();
  inflight.set(id, p);
  return p;
}

/** Drop every size of one game's art, so the next `coverUrl()` reloads it. */
function forget(key: string): void {
  const prefix = `${key}@`;
  for (const id of [...cache.keys()]) if (id.startsWith(prefix)) cache.delete(id);
  for (const id of [...inflight.keys()]) if (id.startsWith(prefix)) inflight.delete(id);
}

/** Check if a game has a user-set custom cover on disk. */
export function hasCustomCover(key: string): Promise<boolean> {
  const hit = customCoverCache.get(key);
  if (hit !== undefined) return Promise.resolve(hit);
  return invoke<boolean>("has_custom_cover", { key }).then((v) => {
    customCoverCache.set(key, v);
    return v;
  });
}

/** Set a custom cover for a game from a local file path. Invalidates the
 *  cover cache so the next `coverUrl()` call loads the new image. */
export async function setCustomCover(
  key: string,
  sourcePath: string,
): Promise<void> {
  await invoke("set_custom_cover", { key, sourcePath });
  forget(key);
  customCoverCache.set(key, true);
}

/** Remove a game's custom cover, reverting to the downloaded art. */
export async function removeCustomCover(key: string): Promise<void> {
  await invoke("remove_custom_cover", { key });
  forget(key);
  customCoverCache.set(key, false);
}

/** Synchronous peek for already-resolved covers (used by the canvas loop,
 *  which can't await per frame). Returns `undefined` if not yet loaded. */
export function cachedCoverUrl(key: string): string | null | undefined {
  return cache.get(`${key}@${FULL}`);
}

const slugIdCache = new Map<string, number | null>();
const slugIdInflight = new Map<string, Promise<number | null>>();

/** Resolve a game slug to its Steam app id via the embedded Ludusavi catalog.
 *  No longer used for covers, Rust owns that chain end to end now, but kept
 *  as the JS-side handle on the command for anything that needs the id itself
 *  rather than the art. */
export function steamIdForSlug(slug: string): Promise<number | null> {
  const hit = slugIdCache.get(slug);
  if (hit !== undefined) return Promise.resolve(hit);
  const pending = slugIdInflight.get(slug);
  if (pending) return pending;

  const p = (async () => {
    try {
      const id = await invoke<number | null>("steam_app_id_for_slug", { slug });
      slugIdCache.set(slug, id ?? null);
      return id ?? null;
    } catch {
      slugIdCache.set(slug, null);
      return null;
    } finally {
      slugIdInflight.delete(slug);
    }
  })();
  slugIdInflight.set(slug, p);
  return p;
}
