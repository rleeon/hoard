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
export const ACCENT_STOPS: [string, number, number][] = [
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
  for (const [k, v] of gemRampVars(deg)) root.style.setProperty(k, v);
  for (const [k, l, c, offset] of GEM_STOPS) {
    const gh = ((deg + offset) % 360).toFixed(1);
    root.style.setProperty(k, `oklch(${l} ${c} ${gh})`);
  }
  root.style.setProperty("--tint-hue", deg.toFixed(1));
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

/** The default gem's hue, for drawing it when the choice is `null`. Matches
 *  the `--logo-gem-*` defaults in `app.css`, i.e. what the default gem paints. */
export const DEFAULT_GEM_HUE = 161;

/** Which gem a stored hue corresponds to, or `null` when it's a custom hue.
 *  Drives the selected ring. */
export function gemFor(hue: number | null): Gem | null {
  return gems.find((g) => g.hue === hue) ?? null;
}

/** Persist a custom hue (or clear it) and apply immediately. While a special
 *  mode is on the pick is kept and shows the day the mode ends. */
export function setAccentHue(hue: number | null): void {
  accentHue.set(hue);
  try {
    if (hue == null) localStorage.removeItem(ACCENT_KEY);
    else localStorage.setItem(ACCENT_KEY, String(hue));
  } catch {
    /* best-effort */
  }
  refreshAccent();
}

/** Paint the stored gem before mount, so the first frame already wears it. */
export function initAccent(): void {
  refreshAccent();
  // The window lives in the tray for days: a check at startup alone would
  // keep the pumpkin until a restart in November, or miss the 24th entirely.
  setInterval(refreshAccent, 60 * 60 * 1000);
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) refreshAccent();
  });
}

// ---- special modes

/**
 * A seasonal look worn over the gem: the accent, the edges and whatever the
 * mode adds (Halloween's spider and green title bar). Settings has a switch
 * and a picker for them; there is only Halloween so far.
 *
 * Left alone, a mode turns itself on during its season. Switching it off is
 * remembered for good. Switching it on outside the season is remembered too,
 * but on during the season is just the default again, so nobody who flicks it
 * off and back on in October ends up with a pumpkin in March.
 */
export type SpecialModeId = "halloween";
export type SpecialChoice = "auto" | "on" | "off";

type SpecialMode = {
  id: SpecialModeId;
  labelKey: string;
  hue: number;
  inSeason: (d: Date) => boolean;
};

/** 24 October to 1 November, by the computer's own clock and time zone, so it
 *  is Halloween when it is Halloween where the user lives. */
export function isHalloweenSeason(d: Date = new Date()): boolean {
  const day = d.getDate();
  return (d.getMonth() === 9 && day >= 24) || (d.getMonth() === 10 && day === 1);
}

export const specialModes: SpecialMode[] = [
  // Pumpkin: the same lightness and chroma as every gem, only the hue moves.
  { id: "halloween", labelKey: "settings.special_halloween", hue: 50, inSeason: isHalloweenSeason },
];

const SPECIAL_CHOICE_KEY = "hoard-special-mode";
const SPECIAL_ID_KEY = "hoard-special-mode-id";

export const specialChoice = writable<SpecialChoice>(readSpecialChoice());
export const specialModeId = writable<SpecialModeId>(readSpecialModeId());
/** Whether a special mode is being worn right now. */
export const specialActive = writable(false);
/** Whether that mode is Halloween. The spider and the title bar read this. */
export const halloween = writable(false);

function readSpecialChoice(): SpecialChoice {
  try {
    const v = localStorage.getItem(SPECIAL_CHOICE_KEY);
    return v === "on" || v === "off" ? v : "auto";
  } catch {
    return "auto";
  }
}

function readSpecialModeId(): SpecialModeId {
  try {
    const v = localStorage.getItem(SPECIAL_ID_KEY);
    return specialModes.find((m) => m.id === v)?.id ?? specialModes[0].id;
  } catch {
    return specialModes[0].id;
  }
}

function currentMode(): SpecialMode {
  const id = readSpecialModeId();
  return specialModes.find((m) => m.id === id) ?? specialModes[0];
}

function refreshAccent(): void {
  const choice = readSpecialChoice();
  const mode = currentMode();
  const on = choice === "on" || (choice === "auto" && mode.inSeason(new Date()));
  applyAccentHue(on ? mode.hue : readAccent());
  document.documentElement.toggleAttribute("data-halloween", on && mode.id === "halloween");
  specialChoice.set(choice);
  specialModeId.set(mode.id);
  specialActive.set(on);
  halloween.set(on && mode.id === "halloween");
}

/** The Settings switch (and the debug panel, which can also hand it back to
 *  the calendar with "auto"). */
export function setSpecialChoice(choice: SpecialChoice): void {
  const inSeason = currentMode().inSeason(new Date());
  const stored = choice === "on" && inSeason ? "auto" : choice;
  try {
    if (stored === "auto") localStorage.removeItem(SPECIAL_CHOICE_KEY);
    else localStorage.setItem(SPECIAL_CHOICE_KEY, stored);
  } catch {
    /* best-effort */
  }
  refreshAccent();
}

export function setSpecialModeId(id: SpecialModeId): void {
  try {
    localStorage.setItem(SPECIAL_ID_KEY, id);
  } catch {
    /* best-effort */
  }
  refreshAccent();
}

/** The accent ramp a hue paints on `<html>`. */
function gemRampVars(hue: number): [string, string][] {
  const h = (((hue % 360) + 360) % 360).toFixed(1);
  return [
    ...ACCENT_STOPS.map(([k, l, c]): [string, string] => [k, `oklch(${l} ${c} ${h})`]),
    ["--color-accent", `oklch(0.62 0.15 ${h})`],
    ["--color-accent-hover", `oklch(0.72 0.16 ${h})`],
  ];
}
