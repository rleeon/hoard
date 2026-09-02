/**
 * Ajustes del HUD sobre el juego: si está activo y con qué atajo se abre.
 *
 * Ojo con el nombre: **no** es Hoard-Screen. Hoard-Screen es la capa Pro, un
 * proceso aparte que compone paneles nativos y que siempre queda por encima de
 * esto. Este HUD es la app normal enseñando su registro en vivo.
 *
 * Vive en `localStorage`, como el tema y el acento: es preferencia de interfaz
 * de *esta* máquina y no tiene por qué viajar al `prefs.json` del servicio.
 *
 * El atajo se registra desde aquí (la ventana principal), no desde Rust: el
 * plugin de atajos globales ya está montado y la ventana principal sigue viva
 * aunque esté oculta en la bandeja, que es justo el caso en el que hace falta.
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

/** Alt+H, «h» de Hoard. */
export const DEFAULT_HOTKEY = "Alt+H";

function readEnabled(): boolean {
  try {
    // Activo de fábrica: el HUD es el que hace que la app sirva de algo con un
    // juego a pantalla completa delante.
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

/** El atajo que está registrado ahora mismo, para poder retirarlo al cambiarlo. */
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
      // El plugin avisa de pulsación Y de soltado; sin filtrar, un toque
      // alternaba dos veces y el HUD parecía no abrirse.
      if (event.state !== "Pressed") return;
      void invoke("overlay_toggle");
    });
    active = accel;
  } catch (e) {
    // Lo más común: otra aplicación ya se quedó con esa combinación. No es
    // fatal, el usuario puede elegir otra en Ajustes.
    console.warn(`no se pudo registrar «${accel}» para el overlay:`, e);
  }
}

/** Aplica el estado actual: registra el atajo si está activo, lo suelta si no. */
async function apply(): Promise<void> {
  let enabled = false;
  let accel = DEFAULT_HOTKEY;
  overlayEnabled.subscribe((v) => (enabled = v))();
  overlayHotkey.subscribe((v) => (accel = v))();
  if (enabled) await bind(accel);
  else {
    await unbind();
    // Si estaba abierto al desactivarlo, se cierra: dejar una ventana que ya
    // no se puede invocar sería un callejón sin salida.
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
