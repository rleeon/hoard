/**
 * Background atmosphere, how the empty space behind the app is painted.
 *
 * Obsidian's canvas is pure black with a faint grain tile over it. That was a
 * deliberate call for WOLED panels (see the note in `app.css`): no top glow,
 * so the black stays genuinely off and never blinks. It is a good default and
 * a bad decree, some people run an IPS panel and want the glow back, others
 * want the grain gone. So the call becomes a choice, with today's look as the
 * one that stays selected for everyone who never opens this setting.
 *
 * Pure UI, so it lives in `localStorage` and never touches the Rust `Prefs`,
 * same reasoning as the theme, the accent and the relief intensity.
 *
 * The value lands on `<html data-atmos>` and everything else is `app.css`.
 */
import { writable } from "svelte/store";

export type AtmosphereId = "grain" | "flat" | "glow" | "vignette";

/** Offered in the Settings picker. `labelKey` resolves through `$_()`. */
export const atmospheres: { id: AtmosphereId; labelKey: string }[] = [
  { id: "grain", labelKey: "settings.atmos_grain" },
  { id: "flat", labelKey: "settings.atmos_flat" },
  { id: "glow", labelKey: "settings.atmos_glow" },
  { id: "vignette", labelKey: "settings.atmos_vignette" },
];

const STORAGE_KEY = "hoard-atmos";
const VALID: AtmosphereId[] = ["grain", "flat", "glow", "vignette"];
const DEFAULT: AtmosphereId = "grain";

function readInitial(): AtmosphereId {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v && (VALID as string[]).includes(v)) return v as AtmosphereId;
  } catch {
    /* storage disabled / private mode, the default look for this session */
  }
  return DEFAULT;
}

export const atmosphere = writable<AtmosphereId>(readInitial());

/** Paint `<html data-atmos="…">`. */
export function applyAtmosphere(id: AtmosphereId): void {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.atmos = id;
}

/** Persist a choice and apply it immediately. */
export function setAtmosphere(id: AtmosphereId): void {
  atmosphere.set(id);
  try {
    localStorage.setItem(STORAGE_KEY, id);
  } catch {
    /* best-effort; the in-memory switch still applies */
  }
  applyAtmosphere(id);
}

/** Paint the stored choice at boot (called by `main.ts`, next to the theme).
 *  Runs before mount so the first frame is already the right background. */
export function initAtmosphere(): void {
  applyAtmosphere(readInitial());
}
