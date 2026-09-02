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

import { writable, derived, get, type Readable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { bootAgent, shutdownAgent } from "./agent";
import { auth } from "./auth";
import { noteStorageStatus } from "./live";
import { notePlanSnapshot } from "./planEvents";

export type CloudAccount = {
  user_id: string;
  email: string;
  display_name: string | null;
  avatar_url: string | null;
  /** "free" | "pro" */
  plan: string;
  /** RFC3339, account creation time. Informational only: the Hoard-Screen /
   *  Hoard-Wrapped trials are per-feature and start at first look (see
   *  `./entitlements.ts`), never from this. `null` on older servers. */
  created_at: string | null;
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
  /** RFC3339, when the current billing period renews. */
  renews_at: string | null;
  /** RFC3339, populated only when the user has scheduled a cancellation. */
  cancel_at: string | null;
  /** RFC3339, the first time this account was ever Pro, `null` if never.
   *  One-way: a downgrade doesn't clear it. It's why an account on Free can
   *  legitimately report an unlimited `devices_limit`, and what lets the
   *  farewell dialog say which of the Pro perks survive the cancellation. */
  first_pro_at?: string | null;
  /** Storage pressure: `"ok"` (green), `"purging"` (orange, old versions are
   *  being auto-deleted to free room), `"full"` (red, at the hard limit, sync
   *  stopped) or `"grace"` (blue, a downgrade is scheduled; the account still
   *  has its old, larger limit and **nothing is being deleted** until
   *  `storage_limit_change_at`). Absent on older servers → treat as `"ok"`. */
  storage_status?: "ok" | "purging" | "full" | "grace";
  /** Set while a storage downgrade is scheduled but not yet applied: the limit
   *  the account will drop to (bytes). During this grace window the user keeps
   *  the larger limit and nothing is purged. `null`/absent = no pending change. */
  pending_storage_limit_bytes?: number | null;
  /** RFC3339, when the pending downgrade takes effect (end of grace window).
   *  `null`/absent = no pending change. */
  storage_limit_change_at?: string | null;
  /** RFC3339, set while the account is soft-deleted and inside its 30-day
   *  grace. When present the app is frozen server-side and the desktop shows the
   *  reactivation screen instead of the normal UI. `null`/absent = live. */
  deleted_at?: string | null;
  /** RFC3339, when the account is hard-purged if not reactivated
   *  (`deleted_at` + 30 days). `null`/absent = live. */
  purges_at?: string | null;
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
    noteStorageStatus(
      account?.storage_status,
      account?.storage_used_bytes,
      account?.storage_limit_bytes,
    );
    // With the cached snapshot, which can be weeks old: if the plan changed while
    // the application was closed, the `refreshCloud` two lines below is what
    // notices. This pass only keeps the marker current so that one has something to
    // compare against.
    void notePlanSnapshot(account);
    // If we have an account, refresh once in the background so the bar
    // tracks reality instead of whatever was on disk at last sign-in.
    if (account) {
      refreshCloud().catch((e) =>
        console.warn("initial cloud refresh failed:", e),
      );
      // Resume background watching for a restored cloud session. Self-hosted
      // does this in `hydrateAuth`, but a cloud-only user has no `$auth.user`,
      // so without this the agent never boots on launch: tracked saves aren't
      // watched, running games aren't detected, and the watcher stays off
      // until the user toggles automatic mode or restarts. `bootAgent` is
      // idempotent and dedups against a concurrent self-hosted boot.
      bootAgent().catch((e) =>
        console.warn("agent boot failed on cloud hydrate:", e),
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
    noteStorageStatus(
      account.storage_status,
      account.storage_used_bytes,
      account.storage_limit_bytes,
    );
    // This is where a payment or a cancellation made elsewhere is noticed: it is
    // the only point that really talks to the server.
    void notePlanSnapshot(account);
    return account;
  } catch (e) {
    internal.update(($s) => ({ ...$s, loading: false }));
    throw e;
  }
}

/** Open a web URL in the system browser through the Rust `open_external`
 *  command (not `@tauri-apps/plugin-shell`), which strips the AppImage's
 *  injected loader env so the spawned browser uses the host's libraries. */
export async function openExternal(url: string): Promise<void> {
  await invoke("open_external", { url });
}

/** Open the OAuth login URL in the system browser. The browser flow ends
 *  with a `hoard://auth/callback#access_token=…&refresh_token=…` deep link
 *  back into the app; the listener in `initCloudDeepLink()` picks it up. */
export async function startCloudLogin(): Promise<void> {
  const url = await invoke<string>("cloud_login_url");
  await openExternal(url);
}

/** Complete a sign-in from a parsed deep-link payload. The `callbackState` is
 *  the CSRF nonce echoed back by the OAuth handoff; Rust rejects the login
 *  unless it matches the one minted at `startCloudLogin`. */
export async function completeCloudLogin(
  accessToken: string,
  refreshToken: string,
  callbackState: string,
): Promise<CloudAccount> {
  internal.update(($s) => ({ ...$s, loading: true }));
  try {
    const account = await invoke<CloudAccount>("cloud_complete_login", {
      accessToken,
      refreshToken,
      callbackState,
    });
    internal.set({ account, hydrated: true, loading: false });
    // First contact with this account's plan. If the machine had never seen it, the
    // marker is seeded and stays quiet: whoever just signed in already knows which
    // plan they did it on. If it had seen it, the difference counts anyway, since
    // paying on the web and coming back here to sign in is a legitimate way to
    // arrive.
    void notePlanSnapshot(account);
    // Leave a record of the acceptance the user gave on the onboarding screen.
    // It has to happen here and not there: the checkbox is ticked before the
    // OAuth round-trip, when there is no account yet to attach it to.
    // Best-effort on purpose, a signed-in user must not be bounced back out
    // because a bookkeeping call failed, and the server is idempotent, so the
    // next launch writes it.
    invoke("cloud_accept_terms").catch((e) =>
      console.warn("terms acceptance not recorded:", e),
    );
    // Boot the live agent for the freshly signed-in account so watching starts
    // immediately, same as `signIn` does for self-hosted. Rust already pointed
    // `CliState` at this account's context inside `cloud_complete_login`, so the
    // watch list hydrates from the right file. Idempotent.
    bootAgent().catch((e) =>
      console.warn("agent boot failed on cloud sign-in:", e),
    );
    return account;
  } catch (e) {
    internal.update(($s) => ({ ...$s, loading: false }));
    throw e;
  }
}

/** Clear the cloud session locally. The server has no concept of "logout"
 *  for JWT-based auth, tokens just expire. */
export async function logoutCloud(): Promise<void> {
  await invoke<void>("cloud_logout");
  internal.set({ account: null, hydrated: true, loading: false });
  // Stop watching once the cloud session is gone, unless a self-hosted
  // session is still active, which keeps its own agent. Leaving it running
  // would have it hammer a cleared token and 401 in a loop.
  if (!get(auth).user) {
    await shutdownAgent();
  }
}

export type CloudExportJob = {
  job_id: string;
  status: string;
};

/** Latest export job's state. `download_url` is a fresh presigned R2 link,
 *  present only when `status === "done"` and the object hasn't expired. All
 *  fields are `null` when the user has never requested an export. */
export type CloudExportStatus = {
  job_id: string | null;
  status: "pending" | "running" | "done" | "failed" | "expired" | null;
  requested_at: string | null;
  size_bytes: number | null;
  expires_at: string | null;
  download_url: string | null;
  error: string | null;
};

/** Kick off a server-side export. A background worker builds the ZIP; poll
 *  {@link exportStatusCloud} for the download link (the server also emails it
 *  when email delivery is configured). */
export async function exportAllCloudData(): Promise<CloudExportJob> {
  return await invoke<CloudExportJob>("cloud_export_all");
}

/** Poll the latest export job's status + download link. */
export async function exportStatusCloud(): Promise<CloudExportStatus> {
  return await invoke<CloudExportStatus>("cloud_export_status");
}

/** Open a ready export's presigned download URL in the system browser, which
 *  saves the ZIP. */
export async function downloadCloudExport(url: string): Promise<void> {
  await openExternal(url);
}

// ---- Caja negra: archived games ----

/** One game's freeable footprint, mirrors the server's `GameFootprint`. */
export type StorageGame = {
  save_id: string;
  game_slug: string;
  label: string;
  /** Bytes the quota drops by if this game is archived (deduped exclusive
   *  blobs). The dialog ranks the heaviest by this. */
  freeable_bytes: number;
  archived: boolean;
  /** RFC3339 hard-delete instant, present only while archived. */
  purge_after: string | null;
};

/** Blobs shared by two or more live saves, keyed by the exact set sharing
 *  them. Their bytes belong to no single game's `freeable_bytes`, so they only
 *  come back once every save in `save_ids` is archived, the "same folder
 *  tracked twice" case. */
export type SharedGroup = {
  save_ids: string[];
  bytes: number;
};

/** Per-game freeable footprint + quota figures for the "free space" dialog. */
export type StorageGames = {
  plan: string;
  used_bytes: number;
  limit_bytes: number;
  /** Bytes the live footprint is over the limit (0 if within). */
  over_bytes: number;
  games: StorageGame[];
  shared_groups?: SharedGroup[];
};

export type ArchiveResult = {
  save_id: string;
  archived: boolean;
  /** RFC3339, when the frozen copy is hard-deleted (archive instant + 7d). */
  purge_after: string;
  freed_bytes: number;
};

/** Per-game freeable footprint + quota, to drive the archive dialog. */
export async function storageGamesCloud(): Promise<StorageGames> {
  return await invoke<StorageGames>("cloud_storage_games");
}

/** Archive a game into the black box: frees quota now, keeps it downloadable
 *  for 7 days, then it's purged. The local save is never touched. */
export async function archiveSaveCloud(saveId: string): Promise<ArchiveResult> {
  return await invoke<ArchiveResult>("cloud_archive_save", { saveId });
}

/** Bring an archived game back (after upgrading / freeing space). */
export async function reactivateSaveCloud(saveId: string): Promise<void> {
  await invoke<void>("cloud_reactivate_save", { saveId });
}

// ---- archived-save lookup (drives badges + the reactivate button) ----

/** `save_id → RFC3339` hard-delete instant, for every archived game. Lets the
 *  Library and History views badge a frozen game and offer "Reactivar" without
 *  each re-deriving the archive state. Empty when signed out / self-hosted
 *  (there's no black box there). Refresh via {@link refreshArchivedSaves}. */
export const archivedSaves = writable<Record<string, string>>({});

/** Repopulate {@link archivedSaves} from the server. No-op that clears the map
 *  when the user isn't signed in to cloud. */
export async function refreshArchivedSaves(): Promise<void> {
  if (!get(internal).account) {
    archivedSaves.set({});
    return;
  }
  try {
    const data = await storageGamesCloud();
    const map: Record<string, string> = {};
    for (const g of data.games) {
      if (g.archived && g.purge_after) map[g.save_id] = g.purge_after;
    }
    archivedSaves.set(map);
  } catch (e) {
    console.warn("refreshArchivedSaves failed:", e);
  }
}

/** Reactivate an archived game and refresh the map so its badge clears. */
export async function reactivateAndRefresh(saveId: string): Promise<void> {
  await reactivateSaveCloud(saveId);
  await refreshArchivedSaves();
}

/** Soft-delete the cloud account. The server freezes it and keeps the data for
 *  a 30-day grace in case the user changes their mind (they can sign back in and
 *  reactivate). Local session is wiped here. */
export async function deleteCloudAccount(): Promise<void> {
  await invoke<void>("cloud_delete_account");
  internal.set({ account: null, hydrated: true, loading: false });
}

/** Cancel a pending soft-delete for the signed-in account. Returns the fresh,
 *  live account (no `deleted_at`) and updates the store so the reactivation
 *  screen dismisses. */
export async function reactivateCloudAccount(): Promise<CloudAccount> {
  const account = await invoke<CloudAccount>("cloud_reactivate_account");
  internal.set({ account, hydrated: true, loading: false });
  noteStorageStatus(
    account.storage_status,
    account.storage_used_bytes,
    account.storage_limit_bytes,
  );
  return account;
}

/** Open the public pricing page in the browser, where the checkout buttons
 *  live. There is no `/upgrade` route on the site (it 404'd); `/pricing` is
 *  the page that links to the Polar hosted checkout. The optional
 *  `plan` is kept for future deep-linking but `/pricing` ignores it today. */
export async function openUpgradePage(plan?: "pro"): Promise<void> {
  const base = "https://hoard.services";
  const url = plan ? `${base}/pricing?plan=${plan}` : `${base}/pricing`;
  await openExternal(url);
}

/** Open the web account page, which exposes the customer portal link
 *  (Polar). The old `/billing` route didn't exist. */
export async function openBillingPortal(): Promise<void> {
  const base = "https://hoard.services";
  await openExternal(`${base}/account`);
}

// ---- deep-link plumbing ----------------------------------------------

let dlUnlisten: UnlistenFn | null = null;
/** The last callback URL we acted on, so the live event and the on-mount
 *  buffer drain don't both run `completeCloudLogin` for the same tokens. */
let lastHandledUrl: string | null = null;

/** Wire the `deep-link://new-url` listener that the Rust side emits for
 *  every `hoard://…` URL AND drain any URL buffered before this listener
 *  existed (the cold-start case: the OS launches the app with the OAuth
 *  callback as a launch argument, well before the webview mounts). Parses
 *  the callback and calls `completeCloudLogin`. Idempotent, calling twice
 *  replaces the previous subscription. */
export async function initCloudDeepLink(
  onSignedIn?: (account: CloudAccount) => void,
  onError?: (err: unknown) => void,
): Promise<void> {
  const handle = async (raw: unknown): Promise<void> => {
    if (typeof raw !== "string") return;
    if (!raw.startsWith("hoard://auth/callback")) return;
    if (raw === lastHandledUrl) return; // dedup event vs. buffer drain
    lastHandledUrl = raw;
    const tokens = parseAuthCallback(raw);
    if (!tokens) return;
    try {
      const account = await completeCloudLogin(
        tokens.accessToken,
        tokens.refreshToken,
        tokens.state,
      );
      onSignedIn?.(account);
    } catch (e) {
      console.error("cloud deep-link sign-in failed:", e);
      // Allow a retry of the same URL after a failure (e.g. transient
      // network error verifying the token against /v1/me).
      lastHandledUrl = null;
      onError?.(e);
    }
  };

  if (dlUnlisten) {
    dlUnlisten();
    dlUnlisten = null;
  }
  // Subscribe first so nothing emitted between the drain and the listen is
  // lost, then drain whatever was buffered before we got here.
  dlUnlisten = await listen<string>("deep-link://new-url", (event) =>
    handle(event.payload),
  );
  try {
    const pending = await invoke<string | null>("cloud_take_pending_deep_link");
    if (pending) await handle(pending);
  } catch (e) {
    console.warn("draining pending deep link failed:", e);
  }
}

let seUnlisten: UnlistenFn | null = null;

/** Listen for `agent://session-expired`, emitted by the Rust side when the
 *  Supabase refresh-token family is revoked (terminal expiry). The backend has
 *  already cleared creds + stopped the pollers; we mirror that here so the
 *  signed-in shell collapses and the LiveStatus dot stops looping on "server
 *  unavailable". `onExpired` lets the caller toast + reset the cloud dot +
 *  route. Idempotent, calling twice replaces the previous subscription. */
export async function initCloudSessionWatch(
  onExpired?: () => void,
): Promise<void> {
  if (seUnlisten) {
    seUnlisten();
    seUnlisten = null;
  }
  seUnlisten = await listen<void>("agent://session-expired", () => {
    internal.set({ account: null, hydrated: true, loading: false });
    onExpired?.();
  });
}

function parseAuthCallback(
  url: string,
): { accessToken: string; refreshToken: string; state: string } | null {
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
  const state = kv.get("state") ?? "";
  if (!access) return null;
  return { accessToken: access, refreshToken: refresh, state };
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
