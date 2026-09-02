/**
 * Local, per-device display-name overrides for tracked games.
 *
 * The backend key for a game is its `game_slug`, that never changes; it's
 * what ties detection, cover art and the server row together. But detection
 * sometimes guesses an ugly or wrong-looking name, so we let the user pick a
 * prettier *visible* name. That override lives only on this machine
 * (`tauri-plugin-store`, a JSON file) and is a pure presentation layer: the
 * slug keeps flowing through sync untouched.
 *
 * The map is `slug -> custom name`. An absent entry means "show the slug"
 * (the previous behaviour). Setting an empty name deletes the override, which
 * reverts the card to the auto-detected name.
 */
import { writable } from "svelte/store";

import { LazyStore } from "@tauri-apps/plugin-store";

const STORE_FILE = "game_names.json";
const KEY = "overrides";

const store = new LazyStore(STORE_FILE);

/** Reactive `slug -> custom name` map. Empty until {@link hydrateGameNames}
 *  runs; components read it through `$customNames` so cards re-render the
 *  moment an override changes. */
export const customNames = writable<Record<string, string>>({});

/** Pull overrides from disk. Call once when the Library mounts. */
export async function hydrateGameNames(): Promise<void> {
  try {
    const map = await store.get<Record<string, string>>(KEY);
    if (map) customNames.set(map);
  } catch {
    // Corrupt or missing file: keep the empty map, falling back to slugs.
  }
}

/** Set (or, with an empty name, clear) the display-name override for a slug
 *  and persist it. Resolves once the write hits disk so the caller can toast. */
export async function setGameName(slug: string, name: string): Promise<void> {
  const trimmed = name.trim();
  let next: Record<string, string> = {};
  customNames.update((m) => {
    next = { ...m };
    if (trimmed) next[slug] = trimmed;
    else delete next[slug];
    return next;
  });
  await store.set(KEY, next);
  await store.save();
}
