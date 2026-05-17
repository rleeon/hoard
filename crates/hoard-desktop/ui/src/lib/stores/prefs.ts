/**
 * User preferences store.
 *
 * Mirrors the Rust `Prefs` struct. The store holds `null` until we've
 * round-tripped Rust on first load — components that depend on prefs should
 * check for `null` and render a sensible "loading" placeholder, not a
 * defaults-baked UI that would flicker.
 *
 * `update()` writes through to disk before resolving so the rest of the app
 * (and the tray's close-to-tray check, which reads on every window-close)
 * sees the new value immediately.
 */
import { writable, type Writable } from "svelte/store";

import * as api from "../api";
import type { Prefs } from "../api";

export const prefs: Writable<Prefs | null> = writable(null);

/** Pull prefs from disk. Call once at boot. Failures fall back to defaults
 * so the Settings page can still render — the user can hit "Save" to repair
 * the file if it was corrupt. */
export async function hydratePrefs(): Promise<void> {
  try {
    const p = await api.getPrefs();
    prefs.set(p);
  } catch (e) {
    console.error("getPrefs failed:", e);
    // Pessimistic defaults so the rest of the UI keeps functioning.
    prefs.set({
      close_to_tray: true,
      notify_on_success: true,
      notify_on_failure: true,
      autostart: false,
      start_minimised: false,
      seen_tray_hint: false,
      anonymous_telemetry: false,
      language: null,
      auto_restore: false,
    });
  }
}

/** Persist a partial update and fan it out to subscribers. Returns the new
 * full Prefs object. Throws on disk-write failure so the Settings page can
 * surface a toast. */
export async function updatePrefs(patch: Partial<Prefs>): Promise<Prefs> {
  // Read the current value synchronously so we always send the full struct
  // to Rust (the command replaces wholesale; partials are a footgun).
  let current: Prefs | null = null;
  prefs.subscribe((p) => (current = p))();
  if (!current) {
    current = await api.getPrefs();
  }
  const next: Prefs = { ...current, ...patch };
  const saved = await api.savePrefs(next);
  prefs.set(saved);
  return saved;
}
