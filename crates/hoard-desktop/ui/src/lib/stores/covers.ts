/**
 * Game cover-art loader, backed by the on-device cache.
 *
 * The Rust `cover_bytes` command downloads each game's art once, persists it
 * under the app cache dir, and returns the raw JPEG bytes as an `ArrayBuffer`
 * thereafter, no network round-trip after the first sight, no canvas-tainting
 * cross-origin draws. Here we wrap those bytes in an object URL and memoise
 * the result so a given game is fetched/decoded at most once per session. A
 * `null` entry marks a permanent miss (no art / offline first run) so callers
 * fall back to the initial-letter placeholder without retrying.
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

/** Resolve a cover key to a usable `<img src>` object URL, or `null` if
 *  there's no cover to show. Safe to call repeatedly, memoised +
 *  de-duplicated. */
export function coverUrl(key: string): Promise<string | null> {
  const hit = cache.get(key);
  if (hit !== undefined) return Promise.resolve(hit);
  const pending = inflight.get(key);
  if (pending) return pending;

  const p = (async () => {
    try {
      const buf = await invoke<ArrayBuffer>("cover_bytes", { key });
      const url = URL.createObjectURL(new Blob([buf], { type: mimeOf(buf) }));
      cache.set(key, url);
      return url;
    } catch {
      cache.set(key, null);
      return null;
    } finally {
      inflight.delete(key);
    }
  })();
  inflight.set(key, p);
  return p;
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
  // Invalidate caches so the cover reloads.
  cache.delete(key);
  inflight.delete(key);
  customCoverCache.set(key, true);
}

/** Remove a game's custom cover, reverting to the downloaded art. */
export async function removeCustomCover(key: string): Promise<void> {
  await invoke("remove_custom_cover", { key });
  cache.delete(key);
  inflight.delete(key);
  customCoverCache.set(key, false);
}

/** Synchronous peek for already-resolved covers (used by the canvas loop,
 *  which can't await per frame). Returns `undefined` if not yet loaded. */
export function cachedCoverUrl(key: string): string | null | undefined {
  return cache.get(key);
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
