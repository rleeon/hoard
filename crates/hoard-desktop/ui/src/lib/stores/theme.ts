/**
 * The accent, "the gem".
 *
 * Hoard has one look, the dark one, and what the user picks is the gem: the hue
 * the accent ramp and the logo take. It is a pure UI concern, so it lives in
 * localStorage and never touches the Rust `Prefs` struct.
 */
import { writable } from "svelte/store";

const ACCENT_KEY = "hoard-accent-hue";

/** Custom accent hue (null = the default emerald gem). 0-360 degrees. */
export const accentHue = writable<number | null>(readAccent());

function readAccent(): number | null {
  try {
    const v = localStorage.getItem(ACCENT_KEY);
    const n = v == null ? NaN : Number(v);
    return Number.isFinite(n) && n >= 0 && n < 360 ? n : null;
  } catch {
    return null;
  }
}

/**
 * Custom accent ("your gem"): repoints the emerald scale + accent tokens to a
 * user-chosen hue, as inline custom properties on <html>, where they win over
 * the stylesheet's defaults. Pass null to go back to emerald. Lightness and
 * chroma track the default emerald, so every gem sits on the black the same way.
 *
 * The scale is covered end to end, not just the shades that carry most of the
 * UI. A shade left out doesn't fall back to something neutral, it keeps
 * Tailwind's own emerald, so it stays green while everything around it turns
 * the chosen hue. That is how the playtime heatmap ended up with a green square
 * in the middle of a purple ramp: its lowest level is `emerald-900`, and only
 * 300-700 were being repointed. Chroma tapers at both ends because the mid-ramp
 * 0.15 is not reachable at those lightnesses for every hue, and a value outside
 * the gamut gets clipped, which shifts the hue, the one thing this must hold.
 *
 * The mark's gem (`--logo-gem-*`, drawn by `Logo.svelte`) follows along, but on
 * its own lightness/chroma: the logo always sits on a near-black tile, so it
 * can't borrow the emerald ramp.
 *
 * `--tint-hue` carries the gem into the neutrals. The grey ramp and the surfaces
 * are black with a whisper of hue in them (chroma under 0.01), and with a ruby or
 * a sapphire gem that whisper kept saying emerald.
 */
const ACCENT_STOPS: [string, number, number][] = [
  ["--color-emerald-50", 0.95, 0.04],
  ["--color-emerald-100", 0.89, 0.09],
  ["--color-emerald-200", 0.82, 0.13],
  ["--color-emerald-300", 0.75, 0.15],
  ["--color-emerald-400", 0.7, 0.16],
  ["--color-emerald-500", 0.64, 0.15],
  ["--color-emerald-600", 0.58, 0.16],
  ["--color-emerald-700", 0.5, 0.15],
  ["--color-emerald-800", 0.44, 0.13],
  ["--color-emerald-900", 0.37, 0.1],
  ["--color-emerald-950", 0.27, 0.06],
];

/** The mark's gem: lightness, chroma and a hue offset from the chosen accent.
 *  The offset keeps the logo's two-tone sweep (the top stop leads the bottom
 *  one, teal→emerald in the default gem) instead of flattening it to one hue. */
const GEM_STOPS: [string, number, number, number][] = [
  ["--logo-gem-from", 0.855, 0.138, 20],
  ["--logo-gem-to", 0.6, 0.13, 2],
  ["--logo-gem-ring", 0.7, 0.149, 0],
];

export function applyAccentHue(hue: number | null): void {
  const root = document.documentElement;
  if (hue == null) {
    for (const [k] of ACCENT_STOPS) root.style.removeProperty(k);
    for (const [k] of GEM_STOPS) root.style.removeProperty(k);
    root.style.removeProperty("--color-accent");
    root.style.removeProperty("--color-accent-hover");
    root.style.removeProperty("--tint-hue");
    return;
  }
  const deg = ((hue % 360) + 360) % 360;
  const h = deg.toFixed(1);
  for (const [k, l, c] of ACCENT_STOPS) {
    root.style.setProperty(k, `oklch(${l} ${c} ${h})`);
  }
  for (const [k, l, c, offset] of GEM_STOPS) {
    const gh = ((deg + offset) % 360).toFixed(1);
    root.style.setProperty(k, `oklch(${l} ${c} ${gh})`);
  }
  root.style.setProperty("--color-accent", `oklch(0.62 0.15 ${h})`);
  root.style.setProperty("--color-accent-hover", `oklch(0.72 0.16 ${h})`);
  root.style.setProperty("--tint-hue", h);
}

/**
 * Named gems offered in the Settings picker.
 *
 * The hue wheel is still there, under them, but nobody thinks "I want 265
 * degrees", they think "I want it blue". These seven walk the whole wheel
 * without two of them landing on the same colour.
 *
 * Emerald is `null`, not 160: it means the default gem, which is what the reset
 * button always did. So picking Emerald *is* the reset.
 */
export type Gem = { id: string; hue: number | null; labelKey: string };

export const gems: Gem[] = [
  { id: "emerald", hue: null, labelKey: "settings.gem_emerald" },
  { id: "citrine", hue: 100, labelKey: "settings.gem_citrine" },
  { id: "amber", hue: 70, labelKey: "settings.gem_amber" },
  { id: "ruby", hue: 20, labelKey: "settings.gem_ruby" },
  { id: "amethyst", hue: 305, labelKey: "settings.gem_amethyst" },
  { id: "sapphire", hue: 265, labelKey: "settings.gem_sapphire" },
  { id: "aquamarine", hue: 205, labelKey: "settings.gem_aquamarine" },
];

/** Default hue for the swatch preview when the choice is `null`. Matches the
 *  `--logo-gem-*` defaults in `app.css`, i.e. what the default gem paints. */
const DEFAULT_GEM_HUE = 161;

/**
 * The two gradient stops a given hue produces, as CSS colours. Settings uses
 * them to draw each swatch with the *same* maths `applyAccentHue` will apply,
 * so a swatch is a real preview of the mark rather than a lookalike dot.
 */
export function gemSwatch(hue: number | null): { from: string; to: string } {
  const deg = hue == null ? DEFAULT_GEM_HUE : (((hue % 360) + 360) % 360);
  const at = (key: string) => {
    const stop = GEM_STOPS.find(([k]) => k === key)!;
    const [, l, c, offset] = stop;
    return `oklch(${l} ${c} ${(deg + offset) % 360})`;
  };
  return { from: at("--logo-gem-from"), to: at("--logo-gem-to") };
}

/** Which gem a stored hue corresponds to, or `null` when it's a custom hue.
 *  Drives the selected ring. */
export function gemFor(hue: number | null): Gem | null {
  return gems.find((g) => g.hue === hue) ?? null;
}

/** Persist a custom hue (or clear it) and apply immediately. */
export function setAccentHue(hue: number | null): void {
  accentHue.set(hue);
  try {
    if (hue == null) localStorage.removeItem(ACCENT_KEY);
    else localStorage.setItem(ACCENT_KEY, String(hue));
  } catch {
    /* best-effort */
  }
  applyAccentHue(hue);
}

/** Paint the stored gem before mount, so the first frame already wears it. */
export function initAccent(): void {
  applyAccentHue(readAccent());
}
