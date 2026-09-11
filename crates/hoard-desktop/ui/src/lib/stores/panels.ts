/**
 * Which of the two top-right panels is open: the bell's notifications or the
 * eye's live devices. At most one at a time.
 *
 * A store rather than component state because the buttons and the panels no
 * longer live in the same Svelte root. On Windows the buttons sit in the custom
 * title bar, which `main.ts` mounts beside `App`, not inside it, while the
 * panels stay in `App`. Elsewhere both are still `App`'s.
 */
import { get, writable } from "svelte/store";

import * as api from "../api";
import { showError } from "./error_dialog";
import { prefs } from "./prefs";

export const eyeOpen = writable(false);
export const notifOpen = writable(false);

export function toggleEye(): void {
  eyeOpen.update((open) => {
    if (!open) notifOpen.set(false);
    return !open;
  });
}

export function toggleNotif(): void {
  notifOpen.update((open) => {
    if (!open) eyeOpen.set(false);
    return !open;
  });
}

/** The update confirmation, opened from the sidebar's alert or, on Windows, from
 *  the title bar's. */
export const updatePromptOpen = writable(false);

/** Show or hide the live activity panel. The sidebar's scroll button and, on
 *  Windows, the title bar's both call this. */
export async function toggleLiveActivity(): Promise<void> {
  const visible = !(get(prefs)?.live_activity_visible ?? true);
  try {
    const updated = await api.setLiveActivityVisible(visible);
    prefs.set(updated);
  } catch (e) {
    showError(e);
  }
}
