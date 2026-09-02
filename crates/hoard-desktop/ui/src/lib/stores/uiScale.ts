/**
 * Interface scale.
 *
 * This is the engine's own zoom (`webview.setZoom`), not a CSS trick. That
 * matters: the app has ~130 hard-coded pixel sizes (`text-[11px]` and friends)
 * and every lucide icon takes its size as a number, so scaling by moving
 * `--spacing` or the root font-size would shrink the boxes and leave the text
 * and icons where they were. Real zoom scales all of it, including the grain
 * tile and the covers.
 *
 * Reachable two ways, because people reach for both: the slider in Settings,
 * and Ctrl + wheel / Ctrl +/- / Ctrl+0 anywhere in the app.
 *
 * Per-machine UI state, so `localStorage`, never the Rust `Prefs`, same as
 * the theme, the accent, the relief and the atmosphere. The zoom itself does
 * not survive a restart on the engine's side, so it is re-applied at boot.
 */
import { writable } from "svelte/store";
import { getCurrentWebview } from "@tauri-apps/api/webview";

const STORAGE_KEY = "hoard-ui-scale";

export const MIN_SCALE = 0.7;
export const MAX_SCALE = 1.7;
const DEFAULT_SCALE = 1;

/** One notch of the wheel, or one press of Ctrl+plus. */
const STEP = 0.1;

function clamp(n: number): number {
  // Rounded to whole percent so the slider, the wheel and the readout can
  // never disagree about what "105%" means.
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, Math.round(n * 100) / 100));
}

function readInitial(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const n = raw == null ? NaN : Number(raw);
    if (Number.isFinite(n)) return clamp(n);
  } catch {
    /* storage disabled, 100% for this session */
  }
  return DEFAULT_SCALE;
}

let current = readInitial();

export const uiScale = writable<number>(current);

/**
 * Hand the factor to the engine.
 *
 * Swallows failures on purpose. `setZoom` needs the
 * `core:webview:allow-set-webview-zoom` capability, and outside Tauri,
 * `pnpm dev` in a plain browser, `getCurrentWebview()` throws outright. In
 * both cases the honest outcome is an app at 100%, not an error the user can
 * do nothing about.
 *
 * Coalesced: a trackpad pinch fires wheel events faster than the IPC round
 * trip completes, and without this every one of them would queue up behind
 * the last. Only the newest value matters, so a call while one is in flight
 * just replaces the pending target.
 */
let inFlight = false;
let pending: number | null = null;

async function paint(scale: number): Promise<void> {
  if (inFlight) {
    pending = scale;
    return;
  }
  inFlight = true;
  let next: number | null = scale;
  while (next != null) {
    try {
      await getCurrentWebview().setZoom(next);
    } catch {
      /* zoom unavailable here, stay at whatever the engine has */
      pending = null;
    }
    next = pending;
    pending = null;
  }
  inFlight = false;
}

/** Set the scale, remember it, apply it. */
export function setUiScale(scale: number): void {
  const v = clamp(scale);
  current = v;
  uiScale.set(v);
  try {
    localStorage.setItem(STORAGE_KEY, String(v));
  } catch {
    /* best-effort */
  }
  void paint(v);
}

/** Nudge by a delta, for the keyboard and the wheel. */
export function stepUiScale(delta: number): void {
  setUiScale(current + delta);
}

export function resetUiScale(): void {
  setUiScale(DEFAULT_SCALE);
}

/**
 * Ctrl + wheel, app-wide.
 *
 * Three things this has to get right:
 *
 * - `defaultPrevented`, the Map draws its constellation on a canvas with its
 *   own Ctrl+wheel zoom, attached to that canvas and calling `preventDefault()`
 *   unconditionally. Its listener runs first (it's on the element, we're on the
 *   window, both bubbling), so bailing here leaves the map in charge of the
 *   pointer that's over it.
 * - `passive: false`, wheel listeners on `window` default to passive, where
 *   `preventDefault()` is a silent no-op and the page scrolls underneath the
 *   zoom. (No engine here zooms on its own: WebView2 has zoom hotkeys and
 *   pinch off unless `zoomHotkeysEnabled` is set, and neither WebKitGTK nor
 *   WKWebView implements the shortcut at all.)
 * - `deltaMode`, WebKitGTK reports scroll in lines, where one notch is a
 *   deltaY of about 3, not the ~100 pixels the other engines send. Reading it
 *   raw would file every notch under "trackpad pinch".
 */

/** Sub-percent movement carried between events. A pinch delivers deltas far
 *  too small to shift a value rounded to whole percent, so without this the
 *  gesture would round itself back to a standstill. */
let residue = 0;

function onWheel(e: WheelEvent): void {
  if (!(e.ctrlKey || e.metaKey) || e.defaultPrevented) return;
  e.preventDefault();
  // Lines and pages to pixels, so the notch test below means the same thing on
  // every engine.
  const dy =
    e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * 400 : e.deltaY;
  // A notched wheel sends one big delta per click; a trackpad pinch sends a
  // stream of small ones. Scaling by the delta keeps the pinch smooth without
  // making the wheel crawl.
  const magnitude = Math.min(1, Math.abs(dy) / 40);
  residue += (dy < 0 ? 1 : -1) * STEP * magnitude;
  // Nothing worth a whole percent yet, keep carrying.
  if (Math.abs(residue) < 0.01) return;
  const before = current;
  setUiScale(current + residue);
  // Keep only what didn't land. Clamped, or the carry would grow without
  // bound while someone keeps pinching against an end of the range and then
  // have to be unwound before the first notch back registered.
  residue = Math.max(-STEP, Math.min(STEP, residue - (current - before)));
}

/** Ctrl/Cmd +, - and 0. */
function onKeydown(e: KeyboardEvent): void {
  // Cmd on macOS, Ctrl everywhere else, Ctrl+plus is not the Mac convention,
  // and a Mac user pressing Cmd+0 means it.
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
  switch (e.key) {
    case "+":
    case "=": // the unshifted key on most layouts
      e.preventDefault();
      stepUiScale(STEP);
      break;
    case "-":
    case "_":
      e.preventDefault();
      stepUiScale(-STEP);
      break;
    case "0":
      e.preventDefault();
      resetUiScale();
      break;
  }
}

let wired = false;

/**
 * Wire the shortcuts. Synchronous and cheap, so it runs at boot next to
 * `initTheme()`, otherwise Ctrl+wheel would be dead until the locale
 * dictionary finished loading. The guard is for Vite's HMR in dev, which
 * re-executes the module and would otherwise stack a second listener and
 * double every notch.
 */
export function initUiScaleShortcuts(): void {
  if (wired) return;
  wired = true;
  window.addEventListener("wheel", onWheel, { passive: false });
  window.addEventListener("keydown", onKeydown);
}

/**
 * Apply the stored scale. Called from `main.ts` for the main window only,
 * the HUD is a separate webview sized against the game underneath it, and
 * zooming that would just misalign it.
 *
 * Awaited before mount so the first frame is already at the right scale
 * instead of snapping a moment later. Costs nothing visible: the window is
 * created with `visible: false` and Rust only shows it once the app is up.
 */
export async function initUiScale(): Promise<void> {
  initUiScaleShortcuts();
  await paint(current);
}
