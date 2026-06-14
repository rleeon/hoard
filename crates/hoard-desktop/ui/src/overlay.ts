// Entry point for the transparent always-on-top overlay window (`overlay.html`).
//
// Kept deliberately separate from `main.ts` so the overlay never boots the
// auth/cloud/agent pollers the main app runs — it only mounts the Pro overlay
// surface. `$pro` resolves to the inert stub in the community build, so this
// page renders nothing there; a Pro build links the real editable overlay.
import { i18nReady } from "./lib/i18n";
import { mount } from "svelte";
import "./app.css";
import { Overlay } from "$pro";

async function bootstrap() {
  await i18nReady;
  return mount(Overlay, {
    target: document.getElementById("overlay")!,
  });
}

const overlay = bootstrap();

export default overlay;
