// The games pinned to the top of the Library and the Panel, in the order they
// are shown. Kept on this device like the card sizes: the server never hears
// of it, and a rename does not move anything because the key is the slug.
import { LazyStore } from "@tauri-apps/plugin-store";

const STORE = new LazyStore("pins.json");
const STORE_KEY = "pinned";

let pinned = $state<string[]>([]);

export async function hydratePins(): Promise<void> {
  try {
    const saved = await STORE.get<string[]>(STORE_KEY);
    if (Array.isArray(saved)) pinned = saved.filter((s) => typeof s === "string");
  } catch { /* ignore */ }
}

function persist(): void {
  void STORE.set(STORE_KEY, $state.snapshot(pinned))
    .then(() => STORE.save())
    .catch(() => {});
}

/** Place among the pinned games, from 1. Zero when the game is not pinned. */
export function pinPosition(slug: string): number {
  return pinned.indexOf(slug) + 1;
}

/** A newly pinned game goes last: the numbers follow the order of arrival
 *  until somebody types another one. */
export function togglePin(slug: string): void {
  pinned = pinned.includes(slug) ? pinned.filter((s) => s !== slug) : [...pinned, slug];
  persist();
}

/** Moves a pinned game to `position` and the others make room, so the numbers
 *  stay 1..n with no gap and no repeat. Past the end means last. */
export function setPinPosition(slug: string, position: number): void {
  const from = pinned.indexOf(slug);
  if (from < 0) return;
  const to = Math.min(Math.max(Math.trunc(position), 1), pinned.length) - 1;
  if (to === from) return;
  const next = pinned.filter((s) => s !== slug);
  next.splice(to, 0, slug);
  pinned = next;
  persist();
}

/** For `sort`: pinned before the rest, in their order. Zero between two games
 *  that are not pinned, which leaves them to whatever order the list has. */
export function comparePinned(a: string, b: string): number {
  const pa = pinPosition(a);
  const pb = pinPosition(b);
  if (pa && pb) return pa - pb;
  if (pa) return -1;
  if (pb) return 1;
  return 0;
}
