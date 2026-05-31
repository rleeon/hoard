/**
 * Hoard Cloud account state.
 *
 * Independent of `./auth.ts` (which owns the self-hosted bearer-token
 * session). A user can be signed in to:
 *   - cloud only            → the /account route + sidebar plan chip
 *   - self-hosted only      → today's flow, untouched
 *   - both (rare; future)   → routes pick which the user sees
 *
 * The Rust side persists the Supabase JWT (keyring + 0600 file fallback),
 * so this store is just a reactive view of `cloud_current_account()`.
 */

import { writable, derived, type Readable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openExternal } from "@tauri-apps/plugin-shell";

export type CloudAccount = {
  user_id: string;
  email: string;
  display_name: string | null;
  avatar_url: string | null;
  /** "free" | "pro" */
  plan: string;
  storage_used_bytes: number;
  /** `-1` = unlimited. */
  storage_limit_bytes: number;
  devices_used: number;
  /** `-1` = unlimited. */
  devices_limit: number;
  saves_used: number;
  /** `-1` = unlimited. Unlimited on every tier post-1.6.1. */
  saves_limit: number;
  /** Always `true` post-1.6.1. Future tiers with a rolling-window
   *  retention would flip this to `false`. */
  version_history_forever: boolean;
  /** Per-save upload cap. Server returns 413 above this. */
  max_save_size_bytes: number;
  /** Rolling-window bandwidth quota (over `bandwidth_window_secs`). */
  bandwidth_quota_bytes: number;
  bandwidth_window_secs: number;
  subscription_status: string | null;
  /** RFC3339 — when the current billing period renews. */
  renews_at: string | null;
  /** RFC3339 — populated only when the user has scheduled a cancellation. */
  cancel_at: string | null;
};

type CloudState = {
  account: CloudAccount | null;
  hydrated: boolean;
  /** True while a /v1/me round-trip is in flight. */
  loading: boolean;
};

const internal = writable<CloudState>({
  account: null,
  hydrated: false,
  loading: false,
});

export const cloud: Readable<CloudState> = { subscribe: internal.subscribe };

/** True iff the user is signed in to Hoard Cloud. */
export const isCloudLoggedIn: Readable<boolean> = derived(
  internal,
  ($s) => $s.hydrated && $s.account !== null,
);

/** Convenience selector for the current plan key, or null when signed out. */
export const cloudPlan: Readable<string | null> = derived(
  internal,
  ($s) => $s.account?.plan ?? null,
);

/** One-shot load at app boot. Pulls the cached account from Rust. */
export async function hydrateCloud(): Promise<void> {
  try {
    const account = await invoke<CloudAccount | null>("cloud_current_account");
    internal.set({ account, hydrated: true, loading: false });
    // If we have an account, refresh once in the background so the bar
    // tracks reality instead of whatever was on disk at last sign-in.
    if (account) {
      refreshCloud().catch((e) =>
        console.warn("initial cloud refresh failed:", e),
      );
    }
  } catch (e) {
    console.error("hydrateCloud failed:", e);
    internal.set({ account: null, hydrated: true, loading: false });
  }
}

/** Re-fetch `/v1/me` and update the cache. Throws on auth failure so the
 *  caller can route the user back to /account. */
export async function refreshCloud(): Promise<CloudAccount> {
  internal.update(($s) => ({ ...$s, loading: true }));
  try {
    const account = await invoke<CloudAccount>("cloud_refresh_account");
    internal.set({ account, hydrated: true, loading: false });
    return account;
  } catch (e) {
    internal.update(($s) => ({ ...$s, loading: false }));
    throw e;
  }
}

/** Open the OAuth login URL in the system browser. The browser flow ends
 *  with a `hoard://auth/callback#access_token=…&refresh_token=…` deep link
 *  back into the app; the listener in `initCloudDeepLink()` picks it up. */
export async function startCloudLogin(): Promise<void> {
  const url = await invoke<string>("cloud_login_url");
  await openExternal(url);
}

/** Complete a sign-in from a parsed deep-link payload. */
export async function completeCloudLogin(
  accessToken: string,
  refreshToken: string,
): Promise<CloudAccount> {
  internal.update(($s) => ({ ...$s, loading: true }));
  try {
    const account = await invoke<CloudAccount>("cloud_complete_login", {
      accessToken,
      refreshToken,
    });
    internal.set({ account, hydrated: true, loading: false });
    return account;
  } catch (e) {
    internal.update(($s) => ({ ...$s, loading: false }));
    throw e;
  }
}

/** Clear the cloud session locally. The server has no concept of "logout"
 *  for JWT-based auth — tokens just expire. */
export async function logoutCloud(): Promise<void> {
  await invoke<void>("cloud_logout");
  internal.set({ account: null, hydrated: true, loading: false });
}

export type CloudExportJob = {
  job_id: string;
  status: string;
};

/** Kick off a server-side export. The server emails the user when the
 *  archive is ready (24h presigned R2 URL). */
export async function exportAllCloudData(): Promise<CloudExportJob> {
  return await invoke<CloudExportJob>("cloud_export_all");
}

/** Soft-delete the cloud account. The server keeps the data for 30 days
 *  in case the user changes their mind. Local session is wiped here. */
export async function deleteCloudAccount(): Promise<void> {
  await invoke<void>("cloud_delete_account");
  internal.set({ account: null, hydrated: true, loading: false });
}

/** Open the public pricing page in the browser, where the checkout buttons
 *  live. There is no `/upgrade` route on the site (it 404'd); `/pricing` is
 *  the page that links to the Lemon Squeezy hosted checkout. The optional
 *  `plan` is kept for future deep-linking but `/pricing` ignores it today. */
export async function openUpgradePage(plan?: "pro"): Promise<void> {
  const base = "https://hoard.services";
  const url = plan ? `${base}/pricing?plan=${plan}` : `${base}/pricing`;
  await openExternal(url);
}

/** Open the web account page, which exposes the customer portal link
 *  (Lemon Squeezy). The old `/billing` route didn't exist. */
export async function openBillingPortal(): Promise<void> {
  const base = "https://hoard.services";
  await openExternal(`${base}/account`);
}

// ---- deep-link plumbing ----------------------------------------------

let dlUnlisten: UnlistenFn | null = null;

/** Wire the `deep-link://new-url` listener that the Rust side emits for
 *  every `hoard://…` URL. Parses the OAuth callback fragment and calls
 *  `completeCloudLogin`. Idempotent — calling twice replaces the previous
 *  subscription. */
export async function initCloudDeepLink(
  onSignedIn?: (account: CloudAccount) => void,
  onError?: (err: unknown) => void,
): Promise<void> {
  if (dlUnlisten) {
    dlUnlisten();
    dlUnlisten = null;
  }
  dlUnlisten = await listen<string>("deep-link://new-url", async (event) => {
    const raw = event.payload;
    if (typeof raw !== "string") return;
    if (!raw.startsWith("hoard://auth/callback")) return;
    const tokens = parseAuthCallback(raw);
    if (!tokens) return;
    try {
      const account = await completeCloudLogin(
        tokens.accessToken,
        tokens.refreshToken,
      );
      onSignedIn?.(account);
    } catch (e) {
      console.error("cloud deep-link sign-in failed:", e);
      onError?.(e);
    }
  });
}

function parseAuthCallback(
  url: string,
): { accessToken: string; refreshToken: string } | null {
  // Supabase OAuth lands tokens in the URL fragment; we accept either
  // shape so a server-side redirect that copies them to the query string
  // still works.
  const hashIdx = url.indexOf("#");
  const queryIdx = url.indexOf("?");
  let kv: URLSearchParams;
  if (hashIdx >= 0) {
    kv = new URLSearchParams(url.slice(hashIdx + 1));
  } else if (queryIdx >= 0) {
    kv = new URLSearchParams(url.slice(queryIdx + 1));
  } else {
    return null;
  }
  const access = kv.get("access_token");
  const refresh = kv.get("refresh_token") ?? "";
  if (!access) return null;
  return { accessToken: access, refreshToken: refresh };
}

// ---- presentation helpers --------------------------------------------

/** Pretty plan label, matching the marketing copy. Falls back to the raw
 *  key when the server invents a new tier we don't render yet. Legacy
 *  "proplus" rows are surfaced as Pro since the 1.6.1 migration folds
 *  Pro+ subscribers onto Pro. */
export function planLabel(plan: string | null | undefined): string {
  switch (plan) {
    case "free":
      return "Free";
    case "pro":
    case "proplus":
      return "Pro";
    default:
      return plan ?? "—";
  }
}
