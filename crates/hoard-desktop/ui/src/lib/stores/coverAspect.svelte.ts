/**
 * Shape of the cover frame in the Dashboard grid, a local, per-device
 * preference, like `cardSizes`, and dragged from the same corner handle:
 * sideways sizes the card, downwards reshapes the cover.
 *
 * The value is the cover's height ÷ width. 1.5 (2:3) is the poster every store
 * ships (Steam's `library_600x900`, GOG, Epic) and stays the default; 1 is the
 * square plenty of curated art comes in, emulated or non-Steam games; below
 * that live the landscape header capsules (Steam's 460×215 ≈ 0.47), which the
 * old two-value toggle had no way to ask for. Those are just points on the
 * range now, and `SaveGameCard` snaps to the two canonical ones while dragging
 * so "exactly 2:3" and "exactly square" stay reachable by hand.
 */
import { LazyStore } from "@tauri-apps/plugin-store";

/** 2:3, the store-standard poster. */
export const POSTER = 1.5;
/** 1:1. */
export const SQUARE = 1;

const STORE = new LazyStore("cover_shape.json");
const STORE_KEY = "shape";
const MIN = 0.45;
const MAX = 1.8;

let aspect = $state<number>(POSTER);

function clamp(n: number): number {
  return Math.max(MIN, Math.min(MAX, n));
}

export async function hydrateCoverAspect(): Promise<void> {
  try {
    const saved = await STORE.get<number | string>(STORE_KEY);
    // This used to be a two-position switch; it is translated into the equivalent
    // number so nobody loses their choice on updating.
    if (saved === "portrait") aspect = POSTER;
    else if (saved === "square") aspect = SQUARE;
    else if (typeof saved === "number" && Number.isFinite(saved)) {
      aspect = clamp(saved);
    }
  } catch {
    /* first run / unreadable store, the default stands */
  }
}

export function coverAspect(): number {
  return aspect;
}

// Persistencia diferida, igual que `cardSizes`: esto lo mueve un arrastre.
let persistTimer: ReturnType<typeof setTimeout> | null = null;

export function setCoverAspect(next: number): void {
  if (!Number.isFinite(next)) return;
  aspect = clamp(next);
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    void persist(aspect);
  }, 250);
}

async function persist(value: number): Promise<void> {
  try {
    await STORE.set(STORE_KEY, value);
    await STORE.save();
  } catch {
    /* the choice still applies this session */
  }
}

/** Inline `aspect-ratio` for the cover box, free-form, so no Tailwind class. */
export function coverAspectStyle(a: number = aspect): string {
  return `aspect-ratio: 1 / ${a.toFixed(3)}`;
}
