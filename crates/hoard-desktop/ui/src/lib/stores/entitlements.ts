/**
 * Per-feature Pro entitlements, mirrored from `GET /v1/cloud/entitlements`.
 *
 * The server is the source of truth (see `cloud/entitlements.rs`): the trial
 * starts the first time the user *opens* a feature's page (the route calls
 * `activateFeature` on first view) and locks (HTTP 402) once the one-week
 * window elapses, independently per feature. This store is just a reactive
 * cache of that snapshot for painting the gate (badge / lock / countdown).
 *
 * Supersedes the legacy global `created_at + 30d` window that used to live in
 * `./cloud.ts` (one clock shared across both features, ticking from signup).
 * Nav gating and the feature routes now read this store exclusively.
 */

import { invoke } from "@tauri-apps/api/core";
import { get, writable, type Readable } from "svelte/store";

import { noteGateTransition } from "./live";

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
 *  on any failure, signed out, offline, self-hosted, so the gate falls back
 *  to the locked state instead of throwing. */
export async function refreshEntitlements(): Promise<Entitlements | null> {
  try {
    const ent = await invoke<Entitlements>("cloud_entitlements");
    internal.set(ent);
    reevalGate(ent, null);
    return ent;
  } catch (e) {
    console.warn("cloud_entitlements failed:", e);
    internal.set(null);
    reevalGate(null, e);
    return null;
  }
}

// ---- gate transition logging + activity feed ----
//
// The user wants to know WHEN the Hoard-Screen candado flips and WHY. Three
// surfaces, each with a different audience:
//   - the Rust `cloud_entitlements` command logs to tracing (target
//     "entitlements"), shipped by logship, the durable log;
//   - `reevalGate` below logs every per-feature transition to the webview
//     console with the exact cause (server snapshot or the invoke error);
//   - `noteGateTransition` pushes an activity-feed row on a real
//     locked↔unlocked flip so the cause is visible in-app without a log file.
// We only log/print TRANSITIONS, a re-pull that reports the same state is a
// no-op, so the backoff retry storm from App.svelte can't spam any surface.

/** Previous per-feature gate state (true = unlocked). `null` = never
 *  determined yet (before the first successful/failed fetch). */
const prevGate: Record<FeatureKey, boolean | null> = {
  screen: null,
  wrapple: null,
};

/** i18n key naming the cause of a feature's current gate state, for the
 *  activity-feed row. */
function gateReasonKey(feature: FeatureKey, ent: Entitlements | null): string {
  if (!ent) return "activity.gate_reason_fetch_failed";
  const fs = ent.features[feature];
  if (ent.plan === "pro" || fs.state === "entitled") {
    return "activity.gate_reason_pro";
  }
  if (fs.state === "trial") return "activity.gate_reason_trial";
  if (fs.state === "trial_expired") return "activity.gate_reason_trial_expired";
  return "activity.gate_reason_plan_free";
}

/** Re-evaluate the gate for every feature against the latest snapshot (or
 *  `null` on fetch failure) and log/feed only on transitions. `err` is the
 *  exact invoke error when the fetch failed, so the console line carries the
 *  real reason (network, 401, server 5xx, …). */
function reevalGate(ent: Entitlements | null, err: unknown): void {
  for (const f of ["screen", "wrapple"] as FeatureKey[]) {
    const next = ent ? featureUnlocked(ent.features[f]) : false;
    const prev = prevGate[f];
    if (prev === next) continue;
    const reasonKey = gateReasonKey(f, ent);
    const cause = err
      ? `fetch failed: ${err instanceof Error ? err.message : String(err)}`
      : `plan=${ent?.plan ?? "?"}, state=${ent?.features[f].state ?? "?"}`;
    const prevLabel = prev === null ? "unknown" : prev ? "unlocked" : "locked";
    console.info(
      `[entitlements] ${f}: ${prevLabel} → ${next ? "unlocked" : "locked"} (${cause})`,
    );
    // Feed row only on a real locked↔unlocked flip (both states known), the
    // initial boot determination is logged but not fed.
    if (prev !== null && prev !== next) {
      noteGateTransition(!next, reasonKey);
    }
    prevGate[f] = next;
  }
}

/** Open a feature: this is the call that actually *starts* the one-week trial
 *  the first time a Free user looks at the feature, and is what flips
 *  `trial_available` → `trial`. The server is the source of truth and idempotent (the clock can't be
 *  restarted by re-opening). Returns the resulting state and patches it into the
 *  cached snapshot so the gate updates immediately. Returns `null` on failure
 *  (signed out, offline) so the caller keeps the locked fallback. */
export async function activateFeature(
  feature: FeatureKey,
): Promise<FeatureState | null> {
  try {
    const fs = await invoke<FeatureState>("cloud_activate_feature", { feature });
    internal.update((ent) =>
      ent ? { ...ent, features: { ...ent.features, [feature]: fs } } : ent,
    );
    // Starting a trial can flip the gate locked→unlocked, re-evaluate so the
    // transition is logged and the activity feed reflects it.
    reevalGate(get(internal), null);
    return fs;
  } catch (e) {
    console.warn("cloud_activate_feature failed:", e);
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
 *  `trial_available` is intentionally NOT unlocked, the trial clock hasn't
 *  started yet (the route flips it via `activateFeature` on first view),
 *  and `trial_expired` is locked. This is the gate that makes the
 *  public binary safe: the Pro UI ships inside it, but only the server saying
 *  "entitled / trial" lets a user actually open it. */
export function featureUnlocked(fs: FeatureState | null | undefined): boolean {
  return fs?.state === "entitled" || fs?.state === "trial";
}

/** Build-time DEV override. A personal/test build sets `HOARD_PRO_UNLOCK=1`
 *  (see `vite.config.ts`) to open the Pro features without a server entitlement,
 *  so the owner can try them. It defaults to `false` and the public/CI build
 *  MUST never set it, the public binary stays server-gated. */
export const PRO_DEV_UNLOCK: boolean =
  import.meta.env.VITE_HOARD_PRO_UNLOCK === true;
