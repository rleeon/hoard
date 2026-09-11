/**
 * The dev panel's overrides (Ctrl+Shift+D, dev builds only).
 *
 * A lot of what the UI has to get right only shows up in states nobody can
 * reproduce on demand: a quota at 80 %, an account at its device limit, a plan
 * that just expired. Waiting to fill 40 GB, or keeping four machines around, is
 * not a way to design a screen.
 *
 * So the account snapshot the UI reads can be shadowed. It shadows *the view*
 * and nothing else: no request is faked, no state is written, and the service
 * keeps doing exactly what it was doing. Wrong numbers on screen are the whole
 * point, which is why this never ships (`import.meta.env.DEV` gates the panel
 * and the shortcut).
 */

import { writable } from "svelte/store";

import type { CloudAccount } from "./cloud";

/** Fields to paint over the real account, or `null` for "show reality". */
export const cloudOverrides = writable<Partial<CloudAccount> | null>(null);

/** Whether the panel itself is on screen. */
export const debugPanelOpen = writable(false);

/** Only ever true in a dev build. Kept as one exported constant so a component
 *  does not have to remember which env var it was. */
export const DEBUG_TOOLS = import.meta.env.DEV;

/** Set from the panel to pop the "this computer wasn't linked" dialog without
 *  needing an account that is actually full. */
export const debugDeviceLimit = writable<{
  used: number;
  limit: number;
  plan: string;
} | null>(null);

export function setOverrides(patch: Partial<CloudAccount>): void {
  cloudOverrides.update(($o) => ({ ...($o ?? {}), ...patch }));
}

export function clearOverrides(): void {
  cloudOverrides.set(null);
}

// ---- surfaces
// The four layers (`--surface-*` in app.css) plus the window's floor, tunable
// live from the panel: how black the ground is, how far each layer lifts off
// it, and the tint they all share. Kept in this browser's storage so a tuning
// survives a reload; the reset puts the stylesheet back in charge.
const SURFACES_KEY = "hoard-debug-surfaces";

export type SurfaceTune = {
  floor: number;
  panel: number;
  raised: number;
  pop: number;
  /** The left rail: how light its fill is, and how opaque. */
  sidebar: number;
  sidebarAlpha: number;
  hue: number;
  chroma: number;
};

export const SURFACE_DEFAULTS: SurfaceTune = {
  floor: 0,
  panel: 0.05,
  raised: 0.06,
  pop: 0.05,
  sidebar: 0,
  sidebarAlpha: 0,
  hue: 165,
  chroma: 0.006,
};

export const surfaceTune = writable<SurfaceTune | null>(null);

const SURFACE_PROPS = [
  "--surface-1",
  "--surface-2",
  "--surface-3",
  "--surface-hover",
  "--sidebar-from",
  "--sidebar-via",
  "--sidebar-to",
];

export function applySurfaces(t: SurfaceTune | null): void {
  const root = document.documentElement;
  surfaceTune.set(t);
  if (!t) {
    for (const k of SURFACE_PROPS) root.style.removeProperty(k);
    document.body.style.removeProperty("background-color");
    try {
      localStorage.removeItem(SURFACES_KEY);
    } catch {
      /* no storage, nothing to forget */
    }
    return;
  }
  const c = (l: number, a?: number) =>
    `oklch(${l.toFixed(3)} ${t.chroma.toFixed(3)} ${t.hue.toFixed(0)}${a === undefined ? "" : ` / ${a}%`})`;
  root.style.setProperty("--surface-1", c(t.panel, 62));
  root.style.setProperty("--surface-2", c(t.raised, 72));
  root.style.setProperty("--surface-3", c(t.pop, 88));
  root.style.setProperty("--surface-hover", c(t.panel + 0.035, 55));
  // The rail keeps the shape of its fade (the stylesheet's 70 / 45 / 30 %, a
  // touch lighter at the bottom) and only moves where it sits.
  const rail = (l: number, share: number) =>
    `oklch(${l.toFixed(3)} ${t.chroma.toFixed(3)} ${t.hue.toFixed(0)} / ${(t.sidebarAlpha * share * 100).toFixed(0)}%)`;
  root.style.setProperty("--sidebar-from", rail(t.sidebar, 1));
  root.style.setProperty("--sidebar-via", rail(t.sidebar, 0.64));
  root.style.setProperty("--sidebar-to", rail(t.sidebar + 0.055, 0.43));
  document.body.style.setProperty("background-color", c(t.floor));
  try {
    localStorage.setItem(SURFACES_KEY, JSON.stringify(t));
  } catch {
    /* no storage: the tuning lasts until the reload */
  }
}

export function restoreSurfaces(): void {
  try {
    const raw = localStorage.getItem(SURFACES_KEY);
    if (raw) applySurfaces({ ...SURFACE_DEFAULTS, ...JSON.parse(raw) });
  } catch {
    /* unreadable: the stylesheet's values stand */
  }
}
