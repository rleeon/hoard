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
// The sections the user sized by dragging. The others follow the screen through
// `fitCardWidth`: a default calibrated on 1080p made the Deck's cards taller than
// its 800 px, cover on screen and the details below it out of reach, and a
// controller has no corner to drag.
let chosen = $state<Partial<Record<SectionKey, boolean>>>({});
let fitted = $state<Partial<Record<SectionKey, number>>>({});

export async function hydrateCardSizes(): Promise<void> {
  try {
    const saved = await STORE.get<Record<SectionKey, number>>(STORE_KEY);
    if (saved) {
      sizes = { ...DEFAULTS, ...saved };
      // Files written before only the dragged sections were saved hold every
      // section; one still at its default was never touched.
      chosen = Object.fromEntries(
        (Object.keys(saved) as SectionKey[])
          .filter((k) => k in DEFAULTS && saved[k] !== DEFAULTS[k])
          .map((k) => [k, true]),
      );
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
    await STORE.set(
      STORE_KEY,
      Object.fromEntries(Object.entries(sizes).filter(([k]) => chosen[k as SectionKey])),
    );
    await STORE.save();
  } catch { /* ignore */ }
}

function clampWidth(key: SectionKey, w: number): number {
  const { min, max } = BOUNDS[key];
  return Math.round(Math.max(min, Math.min(max, w)));
}

export function cardWidth(key: SectionKey): number {
  return chosen[key] ? sizes[key] : (fitted[key] ?? sizes[key]);
}

export function setCardWidth(key: SectionKey, w: number): void {
  sizes[key] = clampWidth(key, w);
  chosen[key] = true;
  schedulePersist();
}

/** The width a view worked out for this screen, used until the user drags. */
export function fitCardWidth(key: SectionKey, w: number): void {
  fitted[key] = clampWidth(key, w);
}

export function resetCardWidths(): void {
  sizes = { ...DEFAULTS };
  chosen = {};
  schedulePersist();
}

/** Room left above and below a dashboard card that fits the screen. */
const FIT_MARGIN = 48;

/**
 * The widest dashboard card whose cover and details fit on screen together.
 *
 * The grid stretches its columns (`minmax(width, 1fr)`), so the width that
 * matters is the column's, not the one asked for: on the Deck's 976 px of content
 * a 360 asked for two columns of 478, and a 2:3 cover of 717. Hence one column
 * count after another until the column is short enough. Never above the
 * calibrated default, so a screen where that already fits looks the same.
 */
export function fitDashboardWidth(
  content: number,
  viewport: number,
  aspect: number,
  info: number,
  gap: number,
): number {
  const { min } = BOUNDS.dashboard;
  const room = viewport - info - FIT_MARGIN;
  for (let n = 1; ; n++) {
    const column = (content - gap * (n - 1)) / n;
    if (column <= min) return min;
    if (column * aspect <= room) return Math.min(DEFAULTS.dashboard, Math.floor(column));
  }
}

export type { SectionKey };
