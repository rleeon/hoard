/**
 * Signing out, once, for whatever session this machine has.
 *
 * There are two independent sessions (Hoard Cloud in `./cloud.ts`, a self-hosted
 * server in `./auth.ts`) and three places that offer a "sign out" button. They
 * used to resolve to different things: the dashboard's and the tray's called
 * `signOut()`, which only closes the self-hosted one, so a Cloud user pressing
 * the button in front of them got the engine stopped, a cheerful toast, and
 * their Cloud session still on disk. One user spent an hour trying to sign out
 * of a stale token that way.
 *
 * So: one door, and it closes both without asking which is open. Both commands
 * are idempotent, and neither makes a network call, which is what makes it safe
 * to fire them blind, and what makes signing out work with no connection at all.
 */

import { get } from "svelte/store";

import { auth, signOut } from "./auth";
import { cloud, logoutCloud } from "./cloud";
import { clearOnboarding, clearTourSeen } from "./onboarding";

/** What was actually closed, so the caller can word its toast. */
export type SignOutResult = {
  cloud: boolean;
  selfHost: boolean;
};

/** Close every session, reset the wizard and the tour, and leave the app ready
 *  for the welcome flow. The caller navigates.
 *
 *  Never throws for a session that wasn't there. It does throw if a store that
 *  *was* signed in refused to clear, because then the user has to know the
 *  session is still on this machine. */
export async function signOutEverything(): Promise<SignOutResult> {
  const had: SignOutResult = {
    cloud: get(cloud).account !== null,
    selfHost: get(auth).user !== null,
  };

  // Blind on purpose: a store can read "signed out" while credentials are still
  // on disk (a hydrate that failed, a snapshot that never loaded), and that is
  // exactly the state someone is trying to escape from when they hit the button.
  const failures: string[] = [];
  try {
    await logoutCloud();
  } catch (e) {
    if (had.cloud) failures.push(asMessage(e));
  }
  try {
    await signOut();
  } catch (e) {
    if (had.selfHost) failures.push(asMessage(e));
  }

  await clearOnboarding().catch((e) =>
    console.warn("clearOnboarding on sign-out failed:", e),
  );
  await clearTourSeen().catch((e) =>
    console.warn("clearTourSeen on sign-out failed:", e),
  );

  if (failures.length > 0) throw new Error(failures.join(" · "));
  return had;
}

function asMessage(e: unknown): string {
  return typeof e === "string" ? e : ((e as Error)?.message ?? String(e));
}
