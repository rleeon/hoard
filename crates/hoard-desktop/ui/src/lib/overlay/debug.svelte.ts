/**
 * The HUD's debug controls. **TEMPORARY.**
 *
 * They exist to tune the look live (background opacity, text opacity, font size)
 * and settle the final values while looking at the HUD over a real game, which is
 * the only way to get a translucent surface right. Once the numbers are picked,
 * this gets deleted wholesale: the store, `OverlayDebug.svelte` and its render in
 * `Overlay.svelte`.
 *
 * That is why it lives in its own directory and does not touch `stores/`: it is a
 * block that comes out in one piece, not something to be untangled.
 *
 * It persists to `localStorage` so it survives closing and reopening the HUD while
 * testing; the HUD is its own window with its own JS context.
 */

/**
 * The switch. **Off**: the controls are not painted and the HUD uses the values
 * below as they are.
 *
 * It is here, in a named `const`, so bringing them back is changing a `false` to a
 * `true` rather than reconstructing from memory which import and which line of
 * markup were needed. The scaffolding stays in the tree on purpose: tuning a
 * translucent surface over a real game has to be redone every time the colours
 * change, and deleting it means writing it all again.
 */
export const OVERLAY_DEBUG_PANEL = false;

const KEY = "hoard-overlay-debug";

export type OverlayDebug = {
  /** Opacidad del fondo del HUD (0 = solo se ve el juego). */
  bgOpacity: number;
  /** The opacity of ALL the content (text, logo, button). */
  textOpacity: number;
  /** The base font size, in px. */
  fontSize: number;
  /** Hide the debug panel itself without closing the HUD. */
  panelOpen: boolean;
};

const DEFAULTS: OverlayDebug = {
  bgOpacity: 0.72,
  textOpacity: 1,
  fontSize: 14,
  panelOpen: true,
};

function read(): OverlayDebug {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULTS, ...(JSON.parse(raw) as Partial<OverlayDebug>) };
  } catch {
    /* storage down or corrupt JSON: fall back to the defaults */
  }
  return { ...DEFAULTS };
}

/** Estado reactivo con runas: lo lee el HUD y lo escriben los deslizadores. */
export const overlayDebug = $state<OverlayDebug>(read());

/** Guarda el estado actual. Lo llaman los controles tras cada cambio. */
export function saveOverlayDebug(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(overlayDebug));
  } catch {
    /* best-effort: losing the debug preference does not matter */
  }
}

/** Vuelve a los valores de partida. */
export function resetOverlayDebug(): void {
  Object.assign(overlayDebug, DEFAULTS);
  saveOverlayDebug();
}
