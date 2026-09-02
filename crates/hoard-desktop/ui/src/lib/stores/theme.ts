/**
 * Theme store.
 *
 * Hoard ships several themes. They are implemented purely in `app.css` by
 * re-pointing Tailwind's palette CSS variables (zinc / white / emerald) under
 * a `[data-theme]` scope on <html>, every component uses raw Tailwind
 * utilities (bg-zinc-950, text-emerald-300, …) that read those variables, so
 * swapping a theme re-skins the whole app with zero markup edits.
 *
 * The "auto" theme follows the OS colour scheme: Quartz by day, Obsidian by
 * night. The choice persists to localStorage so it survives restarts. Theme
 * is a pure UI concern, so it lives client-side and never touches the Rust
 * `Prefs` struct.
 */
import { writable } from "svelte/store";

export type ThemeId = "obsidian" | "quartz" | "auto";

/** Themes offered in the Settings picker. `labelKey` is resolved through $_()
 *  so the names translate with the active locale. */
export const themes: { id: ThemeId; labelKey: string }[] = [
  { id: "obsidian", labelKey: "settings.theme_obsidian" },
  { id: "auto", labelKey: "settings.theme_auto" },
  { id: "quartz", labelKey: "settings.theme_quartz" },
];

const STORAGE_KEY = "hoard-theme";
const ACCENT_KEY = "hoard-accent-hue";
const VALID_IDS: ThemeId[] = ["obsidian", "quartz", "auto"];

export const theme = writable<ThemeId>(readInitial());

/** Custom accent hue (null = use the theme's default gem). 0-360 degrees. */
export const accentHue = writable<number | null>(readAccent());

function readInitial(): ThemeId {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v && (VALID_IDS as string[]).includes(v)) return v as ThemeId;
  } catch {
    /* storage disabled / private mode, fall back to obsidian for the session */
  }
  return "obsidian";
}

function readAccent(): number | null {
  try {
    const v = localStorage.getItem(ACCENT_KEY);
    const n = v == null ? NaN : Number(v);
    return Number.isFinite(n) && n >= 0 && n < 360 ? n : null;
  } catch {
    return null;
  }
}

/** Resolve "auto" to a concrete palette using the OS scheme. Concrete themes
 *  pass through unchanged. */
function resolve(id: ThemeId): Exclude<ThemeId, "auto"> {
  if (id !== "auto") return id;
  const prefersDark =
    window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? true;
  return prefersDark ? "obsidian" : "quartz";
}

/** Paint `<html data-theme="…">`. Called once at boot and again on every
 *  change + OS-scheme flip (when on "auto"). */
export function applyTheme(id: ThemeId): void {
  document.documentElement.dataset.theme = resolve(id);
}

/**
 * Custom accent ("your gem"): repoints the emerald scale + accent tokens to a
 * user-chosen hue, as inline custom properties on <html>. Inline styles win
 * over every theme's [data-theme] block, so the custom accent composes on top
 * of whichever theme is active. Pass null to revert to the theme's own gem.
 *
 * Lightness/chroma track Obsidian's emerald so it reads well on dark themes; on
 * Quartz (light) it's an approximation, the picker is a power-user touch.
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
 * can't borrow the emerald ramp, which Quartz darkens for paper.
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
}

/**
 * Named gems offered in the Settings picker.
 *
 * The hue wheel is still there behind "Custom", but nobody thinks "I want 265
 * degrees", they think "I want it blue". These seven walk the whole wheel
 * without two of them landing on the same colour.
 *
 * Emerald is `null`, not 160: it means "whatever gem this theme ships", which
 * is what the reset button always did. So picking Emerald *is* the reset, and
 * Quartz keeps its own darker emerald instead of being overridden with a hue
 * tuned for Obsidian.
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
 *  `--logo-gem-*` defaults in `app.css`, i.e. what Obsidian actually paints. */
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
 *  Drives both the selected ring and whether the custom slider starts open. */
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

let mql: MediaQueryList | null = null;
let crossfadeTimer: ReturnType<typeof setTimeout> | null = null;

function onSchemeChange(): void {
  // Only "auto" tracks the OS; concrete themes are pinned by the user.
  let current: ThemeId = "auto";
  const unsub = theme.subscribe((v) => (current = v));
  unsub();
  if (current === "auto") applyTheme("auto");
}

/** Add the crossfade class for ~300ms so a palette swap reads as a fade
 *  rather than a hard cut. No-op under reduced motion. */
function withCrossfade(fn: () => void): void {
  const root = document.documentElement;
  const reduce =
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  if (reduce) {
    fn();
    return;
  }
  root.classList.add("theme-anim");
  fn();
  if (crossfadeTimer) clearTimeout(crossfadeTimer);
  crossfadeTimer = setTimeout(() => root.classList.remove("theme-anim"), 320);
}

/** Wire up persistence + DOM application. Call once at boot, before mount,
 *  so the first paint already has the right palette (no flash). The store
 *  subscription lives for the app's lifetime, theme is a singleton. */
export function initTheme(): void {
  applyTheme(readInitial());
  applyAccentHue(readAccent());
  let booted = false;
  theme.subscribe((id) => {
    try {
      localStorage.setItem(STORAGE_KEY, id);
    } catch {
      /* best-effort; the in-memory switch still applies */
    }
    // Skip the crossfade on the boot emission, the palette is already painted
    // and a fade there would flash the whole app on launch.
    if (booted) withCrossfade(() => applyTheme(id));
    else applyTheme(id);
    booted = true;
  });
  mql = window.matchMedia?.("(prefers-color-scheme: dark)") ?? null;
  mql?.addEventListener("change", onSchemeChange);
}
