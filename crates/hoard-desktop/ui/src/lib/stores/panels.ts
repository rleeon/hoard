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

import { byKeys } from "../actions/spatialNav";
import * as api from "../api";
import { openExternal } from "./cloud";
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

/** The star button beside the activity toggle. Starring needs a GitHub session,
 *  which the app does not have, so it only opens the repo page. */
export async function openRepoPage(): Promise<void> {
  try {
    await openExternal("https://github.com/rleeon/hoard");
  } catch (e) {
    showError(e);
  }
}

/** A request from the keys to go into the live activity panel, and the control
 *  to come back to. `ActivityFeed` takes it, also when it mounts after it. */
export const liveActivityEnter = writable<{ opener: HTMLElement | null } | null>(null);

export async function setLiveActivityShown(visible: boolean): Promise<void> {
  try {
    const updated = await api.setLiveActivityVisible(visible);
    prefs.set(updated);
  } catch (e) {
    showError(e);
  }
}

/** The sidebar's scroll button and, on Windows, the title bar's. The pointer
 *  shows or hides the panel. From the keys the button is the way into it, as
 *  the bell's is, and opens it first if it was hidden: the panel sits over the
 *  page, where no arrow reaches it, and hiding it from there left no way in.
 *  The keys hide it with its own close button. */
export async function toggleLiveActivity(): Promise<void> {
  const visible = get(prefs)?.live_activity_visible ?? true;
  if (byKeys()) {
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (!visible) await setLiveActivityShown(true);
    liveActivityEnter.set({ opener });
    return;
  }
  await setLiveActivityShown(!visible);
}
