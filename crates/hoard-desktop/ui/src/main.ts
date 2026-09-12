// i18n must be imported first so registrations and `init()` happen before any
// component subscribes to `$_`. The module has top-level side effects.
import { i18nReady } from "./lib/i18n";
import { initAccent } from "./lib/stores/theme";
import { initUiScale, initUiScaleShortcuts } from "./lib/stores/uiScale";
import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import Overlay from "./lib/overlay/Overlay.svelte";

// Paint the stored accent before mount so the first frame already wears the
// chosen gem instead of flashing emerald first. Pure DOM, no i18n, so it is
// safe to run synchronously here.
initAccent();
if (!isOverlayWindow()) {
  // Ctrl+wheel and Ctrl +/-/0. Wiring two listeners costs nothing and belongs
  // here rather than in the awaited scale below, where the shortcuts would sit
  // dead until the locale dictionary finished loading.
  initUiScaleShortcuts();
}

// Wait for svelte-i18n to finish loading the active locale's dictionary
// before mounting. If we mount eagerly, the first render hits `$_(...)`
// while no messages are loaded, svelte-i18n throws, and Svelte unwinds,
// leaving the user with a blank, body-coloured window. (See v1.2.1 bug.)
//
// It is the *only* thing that blocks the mount: loading an already-registered
// dictionary. The language preference stored on disk runs in parallel and has its
// own deadline inside `i18nReady`; see the i18n module.
/**
 * The HUD over the game is **another window** running the same bundle, told apart
 * by its label (`commands/overlay.rs` sets it). The routing happens here, before
 * mounting, rather than through a route: `App.svelte`'s router decides its
 * destination in `onMount` based on the session and would drag the HUD to the
 * welcome screen. Reading the label off the URL also avoids loading Tauri's window
 * bridge just for this.
 */
function isOverlayWindow(): boolean {
  try {
    // Tauri publica la etiqueta en la query de la ventana.
    const q = new URLSearchParams(window.location.search);
    if (q.get("label") === "overlay") return true;
  } catch {
    /* sin URL utilizable, se asume ventana principal */
  }
  return (window as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } })
    .__TAURI_INTERNALS__?.metadata?.currentWindow?.label === "overlay";
}

/** Paints our own title bar when the window has no system one.
 *
 * Asking the window instead of sniffing the platform is what keeps this honest:
 * Rust drops the decoration on Windows only, and if that ever fails the answer
 * is `true` and the app does not end up wearing two title bars. */
async function mountTitlebar(): Promise<void> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    if (await getCurrentWindow().isDecorated()) return;
    const { default: Titlebar } = await import(
      "./lib/components/Titlebar.svelte"
    );
    document.documentElement.classList.add("has-titlebar");
    mount(Titlebar, { target: document.body });
  } catch {
    /* No Tauri (browser dev) or the call failed: the system bar stays. */
  }
}

async function bootstrap() {
  await i18nReady;
  if (isOverlayWindow()) {
    // The window is transparent: the HUD paints its own background, so `body`'s
    // colour has to go or it would cover the game with a black rectangle.
    document.documentElement.classList.add("is-overlay");
    return mount(Overlay, { target: document.getElementById("app")! });
  }
  // Interface scale is the engine's own zoom, so it travels over IPC and
  // can't be applied synchronously the way the theme is. Awaiting it here costs
  // nothing visible, the main window is created with `visible: false` and Rust
  // only shows it once the app is up, and buys a first frame already at the
  // chosen size instead of one that snaps to it a moment later.
  await initUiScale();
  await mountTitlebar();
  return mount(App, {
    target: document.getElementById("app")!,
  });
}

const app = bootstrap();

export default app;
