import { LazyStore } from "@tauri-apps/plugin-store";

/**
 * Where the user left the live activity panel and how big. Per device, like the
 * card sizes: every machine has its own screen.
 *
 * Docked is how it ships, in the bottom-right stack under the storage banners.
 * Resizing from the top or left edges keeps it docked, because its bottom-right
 * corner does not move. Dragging it, or pulling a bottom or right edge, lets it
 * float, and then it remembers the window corner it was nearest to, so a window
 * that grows or shrinks keeps it beside that corner instead of stranding it in
 * the middle.
 */
export type LiveActivityLayout = {
  docked: boolean;
  anchorX: "left" | "right";
  anchorY: "top" | "bottom";
  /** Gap between the anchored window edges and the panel's own, in px. */
  x: number;
  y: number;
  /** `null` until the user resizes: 22rem wide, as tall as its rows. */
  width: number | null;
  height: number | null;
};

export const MIN_WIDTH = 256;
export const MIN_HEIGHT = 160;
/** The gap the docked stack keeps from the window edges (`bottom-4 right-4`). */
export const EDGE_GAP = 16;

const DEFAULT: LiveActivityLayout = {
  docked: true,
  anchorX: "right",
  anchorY: "bottom",
  x: EDGE_GAP,
  y: EDGE_GAP,
  width: null,
  height: null,
};

const STORE = new LazyStore("live_activity.json");
const STORE_KEY = "layout";

let layout = $state<LiveActivityLayout>({ ...DEFAULT });

export function liveActivityLayout(): LiveActivityLayout {
  return layout;
}

export async function hydrateLiveActivityLayout(): Promise<void> {
  try {
    const saved = await STORE.get<Partial<LiveActivityLayout>>(STORE_KEY);
    if (saved) layout = { ...DEFAULT, ...saved };
  } catch { /* ignore */ }
}

// Written from a drag: one write 250 ms after letting go instead of one per
// pointermove, same as the card sizes.
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
    await STORE.set(STORE_KEY, layout);
    await STORE.save();
  } catch { /* ignore */ }
}

export function setLiveActivityLayout(next: Partial<LiveActivityLayout>): void {
  layout = { ...layout, ...next };
  schedulePersist();
}

/** Back to the corner, keeping the size the user chose. */
export function dockLiveActivity(): void {
  setLiveActivityLayout({ docked: true, anchorX: "right", anchorY: "bottom", x: EDGE_GAP, y: EDGE_GAP });
}
