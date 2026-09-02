/**
 * The tilt's intensity (`use:tilt`'s 3D lean and its glow).
 *
 * It is a continuous percentage, not three buttons: 0 turns the effect off, 100 is
 * the historic 8 degrees, and the default, 50, is half. A slider lets you pick the
 * exact point rather than forcing a choice between three jumps.
 *
 * It is purely an interface matter, so it lives in `localStorage` and never touches
 * Rust's `Prefs`, the same call as the theme and the accent hue.
 *
 * **`prefers-reduced-motion` is not consulted.** It was for a while, to pick the
 * initial value, and in practice it left the application with no tilt out of the
 * box: on WebKitGTK that media query comes from `gtk-enable-animations`, a setting
 * plenty of people have off without meaning "no hover effects at all". The
 * accessibility answer is this slider, which goes to 0 in one gesture, not a default
 * guessed from a signal that in this environment does not mean what it looks
 * like.
 */
import { writable } from "svelte/store";

const STORAGE_KEY = "hoard-motion";

/** The default percentage: half the historic effect. */
const DEFAULT_PCT = 50;

/** The previous version's values, from when this was three named levels. They are
 *  translated on read so nobody loses their choice on updating. */
const LEGACY: Record<string, number> = { off: 0, subtle: 50, full: 100 };

function clamp(n: number): number {
  return Math.max(0, Math.min(100, Math.round(n)));
}

function readInitial(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw == null) return DEFAULT_PCT;
    if (raw in LEGACY) return LEGACY[raw];
    const n = Number(raw);
    if (Number.isFinite(n)) return clamp(n);
  } catch {
    /* almacenamiento deshabilitado, al defecto */
  }
  return DEFAULT_PCT;
}

/** A synchronous mirror of the store. `tiltScale()` is called from a DOM action,
 *  not from a component, so it cannot subscribe: it reads this variable, which the
 *  subscription below keeps current. */
let current = readInitial();

export const motionIntensity = writable<number>(current);

/** A 0 to 1 factor multiplying the base degrees. `lib/actions/tilt.ts` reads it on
 *  every `pointerenter`, so moving the slider shows on the next element you point
 *  at, with nothing remounted. */
export function tiltScale(): number {
  return current / 100;
}

motionIntensity.subscribe((v) => {
  current = clamp(v);
  paint(current);
});

/**
 * The glow that follows the cursor comes from `app.css` and not from the action,
 * and with a continuous value it can no longer be resolved with per-level rules, so
 * the variables are written straight onto `<html>`.
 *
 * `data-motion-on` marks "the user wants motion", and `app.css` uses it to give
 * `.tilt` its easing back under the system's reduced-motion setting: flattening it
 * for somebody who raised the slider by hand does not give them less motion (the
 * degrees are the ones they asked for), it only gives it to them in jerks chasing
 * the mouse.
 */
function paint(pct: number): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  const f = pct / 100;
  root.style.setProperty("--tilt-glow-strength", String(0.18 * f));
  root.style.setProperty("--glow-strength", String(f));
  if (pct > 0) root.dataset.motionOn = "";
  else delete root.dataset.motionOn;
}

/** Sets the intensity and remembers it for the next start. */
export function setMotionIntensity(pct: number): void {
  const v = clamp(pct);
  try {
    localStorage.setItem(STORAGE_KEY, String(v));
  } catch {
    /* best-effort */
  }
  motionIntensity.set(v); // dispara `paint`
}

/** Pinta la intensidad guardada al arrancar (lo llama `main.ts`, junto al tema). */
export function initMotion(): void {
  paint(readInitial());
}
