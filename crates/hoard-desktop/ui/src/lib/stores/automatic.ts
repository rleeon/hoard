/**
 * UI-side observer for the sidebar's "Modo Automático" toggle.
 *
 * The detect + track + backup-sweep work itself now lives **entirely in Rust**
 * (`commands/automatic.rs`): two Tokio tickers drive it so it keeps running
 * while the window is minimized to the tray during a gaming session — back
 * when this logic lived in the frontend it silently stalled whenever WebView2
 * suspended the background window ("nothing got monitored no matter how long
 * passed").
 *
 * This module is now a pure observer. It subscribes to the events the Rust
 * side emits and reflects them in the UI:
 *
 * * `automatic-phase` → drives `automaticState` so the sidebar animates
 *   (detecting / tracking N/M / starting agent / idle).
 * * `automatic-scan-complete` → toasts when new games were tracked (silent on
 *   "nothing new", since the scan fires every few minutes and a recurring
 *   "nothing new" toast would be noise).
 * * `agent://save-conflicts-backed-up` → points the user at the conflict
 *   backup folder.
 */
import { writable, type Writable } from "svelte/store";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { get } from "svelte/store";
import { _ } from "svelte-i18n";
import { toastInfo, toastSuccess } from "./toasts";

export type AutomaticPhase =
  | { kind: "idle" }
  | { kind: "detecting" }
  | { kind: "tracking"; done: number; total: number }
  | { kind: "starting_agent" }
  | { kind: "syncing" };

export const automaticState: Writable<AutomaticPhase> = writable({
  kind: "idle",
});

/**
 * Translation helper local to this module so we don't have to repeat the
 * `get(_)` ceremony inline. `svelte-i18n`'s `_` is itself a store; calling
 * it like a function requires unwrapping the value first.
 */
type InterpolationValue = string | number | boolean | Date | null | undefined;
function tr(
  key: string,
  values?: Record<string, InterpolationValue>,
): string {
  const fn = get(_);
  return fn(key, values ? { values } : undefined);
}

// ---------------------------------------------------------------------------
// Tauri event listener
// ---------------------------------------------------------------------------

let listenerInstalled = false;
let unlisteners: UnlistenFn[] = [];

/** Payload of `automatic-scan-complete`. */
type ScanComplete = { tracked: number };

/** Payload of `agent://save-conflicts-backed-up`. Emitted when the agent's
 *  diff-based restore detects that some local files were older than the
 *  cloud copy and stashed them under `<state_dir>/conflicts/<save_id>/<ts>/`
 *  before overwriting them. The toast points the user at the folder so they
 *  can recover the local version if it turns out to be the canonical one. */
type SaveConflictsBackedUp = {
  save_id: string;
  game_slug: string;
  count: number;
  conflict_dir: string;
};

/**
 * Subscribe to the Modo Automático events the Rust scheduler emits. Idempotent
 * — calling twice does nothing on the second call. Designed to be invoked once
 * from `App.svelte::onMount` and never torn down (the listeners cost nothing
 * when no events fire).
 *
 * * `automatic-phase` → mirror into `automaticState` for the sidebar pulse.
 * * `automatic-scan-complete` → toast only when something new was tracked.
 * * `agent://save-conflicts-backed-up` → conflict-backup notice.
 */
export function initAutomaticListener(): void {
  if (listenerInstalled) return;
  listenerInstalled = true;
  Promise.all([
    listen<AutomaticPhase>("automatic-phase", (event) => {
      automaticState.set(event.payload);
    }),
    listen<ScanComplete>("automatic-scan-complete", (event) => {
      const { tracked } = event.payload;
      // Stay silent when nothing new turned up — this fires on every periodic
      // scan (minutes apart), so a recurring "nothing new" toast is just noise.
      if (tracked > 0) {
        toastSuccess(tr("automatic.tracked_summary", { count: tracked }));
      }
    }),
    listen<SaveConflictsBackedUp>(
      "agent://save-conflicts-backed-up",
      (event) => {
        const { count, conflict_dir } = event.payload;
        // No warning level in the toast store, so this surfaces as info.
        // The retention slider in Settings explains the lifetime; the
        // toast body cites the absolute path so the user can paste it
        // into their file manager.
        toastInfo(
          tr("automatic.conflicts_backed_up", {
            count,
            dir: conflict_dir,
          }),
        );
      },
    ),
  ])
    .then((subs) => {
      unlisteners = subs;
    })
    .catch((e) => {
      console.warn("automatic listener install failed:", e);
      listenerInstalled = false;
    });
}

/** Test-only helper to undo `initAutomaticListener`. Not used in app code. */
export function disposeAutomaticListener(): void {
  for (const u of unlisteners) {
    try {
      u();
    } catch {
      /* ignore */
    }
  }
  unlisteners = [];
  listenerInstalled = false;
}
