/**
 * Per-feature Pro entitlements, mirrored from `GET /v1/cloud/entitlements`.
 *
 * The server is the source of truth (see `cloud/entitlements.rs`): the trial
 * starts on the first *use* of a Pro content endpoint and locks (HTTP 402) once
 * the one-month window elapses, independently per feature. This store is just a
 * reactive cache of that snapshot for painting the gate (badge / lock / days).
 *
 * Supersedes the legacy global `created_at + 30d` window in `./cloud.ts`, which
 * shared one clock across both features. That helper still backs other nav
 * gating; this one is per-feature and authoritative.
 */

import { invoke } from "@tauri-apps/api/core";
import { writable, type Readable } from "svelte/store";

/** One feature's resolved access. Tag matches the server enum. */
export type FeatureState =
  | { state: "entitled" }
  | { state: "trial_available"; days: number }
  | { state: "trial"; expires_at: string }
  | { state: "trial_expired" };

export type FeatureKey = "screen" | "wrapple";

export type Entitlements = {
  /** "free" | "pro" */
  plan: string;
  features: Record<FeatureKey, FeatureState>;
};

const internal = writable<Entitlements | null>(null);

/** Reactive view of the last fetched snapshot, or `null` before the first
 *  fetch / when signed out of Hoard Cloud. */
export const entitlements: Readable<Entitlements | null> = {
  subscribe: internal.subscribe,
};

/** Pull a fresh snapshot from the server. Returns `null` (and caches `null`)
 *  on any failure — signed out, offline, self-hosted — so the gate falls back
 *  to the locked state instead of throwing. */
export async function refreshEntitlements(): Promise<Entitlements | null> {
  try {
    const ent = await invoke<Entitlements>("cloud_entitlements");
    internal.set(ent);
    return ent;
  } catch (e) {
    console.warn("cloud_entitlements failed:", e);
    internal.set(null);
    return null;
  }
}

/** Whole days left for a feature (rounded up). `trial_available` reports the
 *  full window; `trial` counts down from `expires_at`; everything else is 0. */
export function featureDaysLeft(fs: FeatureState | null | undefined): number {
  if (!fs) return 0;
  if (fs.state === "trial_available") return fs.days;
  if (fs.state === "trial") {
    const ms = Date.parse(fs.expires_at) - Date.now();
    return ms > 0 ? Math.ceil(ms / (24 * 60 * 60 * 1000)) : 0;
  }
  return 0;
}

/** Whether a feature should render its REAL UI: paid Pro or an active trial.
 *  `trial_available` is intentionally NOT unlocked — the one-month clock hasn't
 *  started yet — and `trial_expired` is locked. This is the gate that makes the
 *  public binary safe: the Pro UI ships inside it, but only the server saying
 *  "entitled / trial" lets a user actually open it. */
export function featureUnlocked(fs: FeatureState | null | undefined): boolean {
  return fs?.state === "entitled" || fs?.state === "trial";
}

/** Build-time DEV override. A personal/test build sets `HOARD_PRO_UNLOCK=1`
 *  (see `vite.config.ts`) to open the Pro features without a server entitlement,
 *  so the owner can try them. It defaults to `false` and the public/CI build
 *  MUST never set it — the public binary stays server-gated. */
export const PRO_DEV_UNLOCK: boolean =
  import.meta.env.VITE_HOARD_PRO_UNLOCK === true;
