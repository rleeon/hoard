/**
 * The settings for the HUD over the game: whether it is on and which shortcut
 * opens it.
 *
 * Mind the name: this is **not** Hoard-Screen. Hoard-Screen is the Pro layer, a
 * separate process that composes native panels and always sits above this. This HUD
 * is the normal app showing its live log.
 *
 * It lives in `localStorage`, like the theme and the accent: it is *this* machine's
 * interface preference and has no reason to travel to the service's `prefs.json`.
 *
 * The shortcut is registered from here (the main window) and not from Rust: the
 * global-shortcut plugin is already mounted and the main window stays alive even
 * hidden in the tray, which is exactly the case where it is needed.
 */
import { invoke } from "@tauri-apps/api/core";
import {
  register,
  unregister,
  isRegistered,
} from "@tauri-apps/plugin-global-shortcut";
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
    if (await isRegistered(active)) await unregister(active);
  } catch (e) {
    console.warn("no se pudo liberar el atajo del overlay:", e);
  }
  active = null;
}

async function bind(accel: string): Promise<void> {
  await unbind();
  try {
    await register(accel, (event) => {
      // The plugin reports both the press AND the release; unfiltered, one tap
      // toggled twice and the HUD looked like it never opened.
      if (event.state !== "Pressed") return;
      void invoke("overlay_toggle");
    });
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
