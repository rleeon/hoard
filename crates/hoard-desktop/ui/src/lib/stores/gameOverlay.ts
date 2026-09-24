/**
 * The settings for the HUD over the game: whether it is on and which shortcut
 * opens it.
 *
 * It lives in `localStorage`, like the theme and the accent: it is *this* machine's
 * interface preference and has no reason to travel to the service's `prefs.json`.
 *
 * The shortcut itself is registered by Rust (`overlay_bind`), not from this page:
 * the page belongs to the main window, and on Linux that window is dropped while
 * it sits hidden in the tray, which is exactly when the shortcut is needed.
 */
import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";

const KEY_ENABLED = "hoard-overlay-enabled";
const KEY_HOTKEY = "hoard-overlay-hotkey";

/** Alt+H, the h for Hoard. */
export const DEFAULT_HOTKEY = "Alt+H";

function readEnabled(): boolean {
  try {
    // On out of the box: the HUD is what makes the app useful with a full-screen
    // game in front of you.
    return localStorage.getItem(KEY_ENABLED) !== "0";
  } catch {
    return true;
  }
}

function readHotkey(): string {
  try {
    return localStorage.getItem(KEY_HOTKEY) || DEFAULT_HOTKEY;
  } catch {
    return DEFAULT_HOTKEY;
  }
}

export const overlayEnabled = writable<boolean>(readEnabled());
export const overlayHotkey = writable<string>(readHotkey());

/** The shortcut registered right now, so it can be withdrawn when it changes. */
let active: string | null = null;

async function unbind(): Promise<void> {
  if (!active) return;
  try {
    await invoke("overlay_bind", { accel: null });
  } catch (e) {
    console.warn("no se pudo liberar el atajo del overlay:", e);
  }
  active = null;
}

async function bind(accel: string): Promise<void> {
  // Rust drops the previous shortcut itself before taking this one.
  try {
    await invoke("overlay_bind", { accel });
    active = accel;
  } catch (e) {
    // The common case: another application already took that combination. Not
    // fatal, since the user can pick another in Settings.
    console.warn(`no se pudo registrar «${accel}» para el overlay:`, e);
  }
}

/** Applies the current state: registers the shortcut when it is on, releases it when it is not. */
async function apply(): Promise<void> {
  let enabled = false;
  let accel = DEFAULT_HOTKEY;
  overlayEnabled.subscribe((v) => (enabled = v))();
  overlayHotkey.subscribe((v) => (accel = v))();
  if (enabled) await bind(accel);
  else {
    await unbind();
    // If it was open when it was turned off, it closes: leaving a window that can
    // no longer be summoned would be a dead end.
    void invoke("overlay_set_visible", { visible: false }).catch(() => {});
  }
}

export function setOverlayEnabled(on: boolean): void {
  overlayEnabled.set(on);
  try {
    localStorage.setItem(KEY_ENABLED, on ? "1" : "0");
  } catch {
    /* best-effort */
  }
  void apply();
}

export function setOverlayHotkey(accel: string): void {
  overlayHotkey.set(accel);
  try {
    localStorage.setItem(KEY_HOTKEY, accel);
  } catch {
    /* best-effort */
  }
  void apply();
}

/** Registra el atajo al arrancar la ventana principal. */
export function initGameOverlay(): void {
  void apply();
}
