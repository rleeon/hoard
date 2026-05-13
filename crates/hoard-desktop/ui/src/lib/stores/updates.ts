/**
 * Update-checker plumbing.
 *
 * Fetches both the latest GitHub release for the desktop client and the
 * `/v1/health` `version` of the user's current server. The Rust side does
 * the work; this module is a thin TS shim with a result-type re-export.
 *
 * We don't poll on a timer — the Settings page hits the IPC on mount and
 * the sidebar consumes the cached result via the `lastReport` store.
 * That keeps GitHub's rate limit happy (the API allows 60 req/h
 * unauthenticated) without forcing the user to think about it.
 */
import { invoke } from "@tauri-apps/api/core";
import { writable, type Writable } from "svelte/store";

export type ComponentUpdate = {
  current: string;
  latest: string | null;
  available: boolean;
  error: string | null;
};

export type UpdateReport = {
  client: ComponentUpdate;
  server: ComponentUpdate | null;
};

export const lastReport: Writable<UpdateReport | null> = writable(null);

/** Tauri command wrapper. Caches the most recent report in `lastReport`
 *  so other components can subscribe without re-fetching. */
export async function checkForUpdates(): Promise<UpdateReport> {
  const r = await invoke<UpdateReport>("check_for_updates");
  lastReport.set(r);
  return r;
}
