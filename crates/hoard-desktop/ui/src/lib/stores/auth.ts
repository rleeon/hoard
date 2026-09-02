/**
 * Reactive view of the auth state.
 *
 * The Rust side is the source of truth (it owns the keychain entry and the
 * session file). This store is a cache that the UI subscribes to. Calls that
 * mutate auth state, `signIn`, `signOut`, go through here so the rest of
 * the app sees the change without each component having to refetch.
 */

import { writable, derived, type Readable } from "svelte/store";
import * as api from "../api";
import type { UserInfo } from "../api";
import { bootAgent, shutdownAgent } from "./agent";
import { clearOnboarding } from "./onboarding";

type AuthState = {
  /** `null` while we haven't asked Rust yet; after that it's a concrete value. */
  user: UserInfo | null;
  /** True once the initial `currentUser()` round-trip has completed. */
  hydrated: boolean;
};

const internal = writable<AuthState>({ user: null, hydrated: false });

/** Subscribe-only handle on the auth state. */
export const auth: Readable<AuthState> = { subscribe: internal.subscribe };

/** Convenience selector: `true` once we know there's a logged-in user. */
export const isLoggedIn: Readable<boolean> = derived(
  internal,
  ($s) => $s.hydrated && $s.user !== null,
);

/** Pull the latest cached user from Rust. Call this once at boot. */
export async function hydrateAuth(): Promise<void> {
  try {
    const user = await api.currentUser();
    internal.set({ user, hydrated: true });
    if (user) {
      // Resume background watching for an already-logged-in session.
      // Failures here shouldn't block the UI, the user can hit Settings →
      // "Restart agent" later if it didn't come up.
      bootAgent().catch((e) =>
        console.warn("agent boot failed on hydrate:", e),
      );
      // The cached UserInfo from disk has zero quota (we don't persist it
      //, see commands/auth.rs). Trigger a fresh whoami so the dashboard
      // shows real numbers as soon as it mounts.
      refreshQuota().catch((e) =>
        console.warn("initial refreshQuota failed:", e),
      );
    }
  } catch (e) {
    // currentUser shouldn't throw, but if Tauri is broken we still want the
    // UI to show *something* rather than spin forever.
    console.error("hydrateAuth failed:", e);
    internal.set({ user: null, hydrated: true });
  }
}

/** Re-fetch storage quota and update the cached user in place. The dashboard
 *  polls this so the "% used" / "MB used" indicator tracks reality without
 *  forcing a full re-login. Silently no-ops when not signed in. */
export async function refreshQuota(): Promise<void> {
  try {
    const user = await api.refreshQuota();
    internal.update(($s) => ({ ...$s, user }));
  } catch (e) {
    // Network blips shouldn't blow up the UI, keep the last known numbers.
    console.warn("refreshQuota failed:", e);
    // …but a *dead* self-hosted session (revoked key / reset server) makes
    // Rust clear the credentials. Re-sync from the source of truth: if the
    // session is gone, drop to the wizard instead of looping on a dashboard
    // that can't reach the server. A transient failure leaves the cached
    // user in place, so this is a no-op for ordinary blips.
    try {
      const user = await api.currentUser();
      if (user === null) internal.set({ user: null, hydrated: true });
    } catch {
      /* currentUser shouldn't throw; ignore if it does */
    }
  }
}

/** Validate (url, token) and persist the session. Throws on failure. */
export async function signIn(url: string, token: string): Promise<UserInfo> {
  const user = await api.login(url, token);
  internal.set({ user, hydrated: true });
  bootAgent().catch((e) => console.warn("agent boot failed on signIn:", e));
  return user;
}

/** Wipe credentials and put the wizard back on screen.
 *
 * Also clears the persisted onboarding state (step + saved URL). Without
 * this the wizard re-prefills the old server address on the next launch, so
 * a user who signs out of a dead self-hosted box can never actually "forget"
 * it, it keeps coming back. Clearing here makes sign-out a true reset. */
export async function signOut(): Promise<void> {
  await shutdownAgent();
  await api.logout();
  await clearOnboarding().catch((e) =>
    console.warn("clearOnboarding on signOut failed:", e),
  );
  internal.set({ user: null, hydrated: true });
}
