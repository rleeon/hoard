/**
 * Steam cover-art loader, backed by the on-device cache.
 *
 * The Rust `cover_bytes` command downloads each game's Steam capsule once,
 * persists it under the app cache dir, and returns the raw JPEG bytes as an
 * `ArrayBuffer` thereafter — no network round-trip after the first sight, no
 * canvas-tainting cross-origin draws. Here we wrap those bytes in an object
 * URL and memoise the result so a given app id is fetched/decoded at most once
 * per session. A `null` entry marks a permanent miss (no art / offline first
 * run) so callers fall back to the initial-letter placeholder without retrying.
 */
import { invoke } from "@tauri-apps/api/core";

const cache = new Map<number, string | null>();
const inflight = new Map<number, Promise<string | null>>();

/** Resolve an app id to a usable `<img src>` object URL, or `null` if there's
 *  no cover to show. Safe to call repeatedly — memoised + de-duplicated. */
export function coverUrl(appId: number): Promise<string | null> {
  const hit = cache.get(appId);
  if (hit !== undefined) return Promise.resolve(hit);
  const pending = inflight.get(appId);
  if (pending) return pending;

  const p = (async () => {
    try {
      const buf = await invoke<ArrayBuffer>("cover_bytes", { appId });
      const url = URL.createObjectURL(new Blob([buf], { type: "image/jpeg" }));
      cache.set(appId, url);
      return url;
    } catch {
      cache.set(appId, null);
      return null;
    } finally {
      inflight.delete(appId);
    }
  })();
  inflight.set(appId, p);
  return p;
}

/** Synchronous peek for already-resolved covers (used by the canvas loop,
 *  which can't await per frame). Returns `undefined` if not yet loaded. */
export function cachedCoverUrl(appId: number): string | null | undefined {
  return cache.get(appId);
}

const slugIdCache = new Map<string, number | null>();
const slugIdInflight = new Map<string, Promise<number | null>>();

/** Resolve a game slug to its Steam app id via the embedded Ludusavi catalog.
 *  Lets a cover load for a save tracked on another device — one this machine
 *  never detected, so it has no local app id. Memoised + de-duplicated; a
 *  `null` marks a slug the catalog doesn't list so we don't ask twice. */
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
