import { LazyStore } from "@tauri-apps/plugin-store";

type SectionKey =
  | "tracked"
  | "orphans"
  | "playtime"
  | "detected"
  | "dashboard";

const STORE = new LazyStore("card_sizes.json");
const STORE_KEY = "sizes";

const DEFAULTS: Record<SectionKey, number> = {
  tracked: 220,
  orphans: 220,
  playtime: 220,
  detected: 280,
  // The dashboard is large covers, not tiles. 360 is calculated so the elastic grid
  // lands on the same lattice the fixed breakpoints had (lg:3, 2xl:4): full screen
  // at 1080p the content is 1536px wide and with gap-5 that gives 4 columns of
  // 369px, the same as before; in windows around 1280 it still gives 3.
  dashboard: 360,
};

// Per section, because they do not all take the same treatment: a library tile is
// still readable at 140px, but the dashboard's card carries a status pill and a
// backup button on the same row and breaks far sooner.
const BOUNDS: Record<SectionKey, { min: number; max: number }> = {
  tracked: { min: 140, max: 500 },
  orphans: { min: 140, max: 500 },
  playtime: { min: 140, max: 500 },
  detected: { min: 140, max: 500 },
  dashboard: { min: 220, max: 640 },
};

let sizes = $state<Record<SectionKey, number>>({ ...DEFAULTS });

export async function hydrateCardSizes(): Promise<void> {
  try {
    const saved = await STORE.get<Record<SectionKey, number>>(STORE_KEY);
    if (saved) {
      sizes = { ...DEFAULTS, ...saved };
    }
  } catch { /* ignore */ }
}

// Deferred persistence: this is written from a drag, so saving on the spot is a
// trip to disk (an IPC round-trip) on every pointermove. 250 ms after letting go
// goes unnoticed and leaves a single write.
let persistTimer: ReturnType<typeof setTimeout> | null = null;

function schedulePersist(): void {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    void save();
  }, 250);
}

async function save(): Promise<void> {
  try {
    await STORE.set(STORE_KEY, sizes);
    await STORE.save();
  } catch { /* ignore */ }
}

export function cardWidth(key: SectionKey): number {
  return sizes[key];
}

export function setCardWidth(key: SectionKey, w: number): void {
  const { min, max } = BOUNDS[key];
  sizes[key] = Math.round(Math.max(min, Math.min(max, w)));
  schedulePersist();
}

export function resetCardWidths(): void {
  sizes = { ...DEFAULTS };
  schedulePersist();
}

export type { SectionKey };
