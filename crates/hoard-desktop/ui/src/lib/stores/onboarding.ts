/**
 * Persisted wizard state.
 *
 * If the user closes the app halfway through onboarding we bring them back
 * to the same step on the next launch. Storage lives in
 * `tauri-plugin-store` (a JSON file managed by Tauri), not in the browser's
 * localStorage, we want it to survive a webview cache wipe.
 */

import { LazyStore } from "@tauri-apps/plugin-store";

const STORE_FILE = "onboarding.json";
const KEY_STEP = "step";
const KEY_URL = "url";
/** Identity (account / server) the post-onboarding tour was last shown for.
 *  We key the tour on *who* you signed in as, not a global "seen once" flag,
 *  so it replays when you switch to a different account or self-host a
 *  different server, but stays quiet on ordinary relaunches of the same
 *  session. Cleared on forget/logout/delete so reconnecting shows it again. */
const KEY_TOUR_SEEN = "tour_seen_for";

/** Routes that make up the wizard, in order.
 *
 * The path forks at `choose`: the Cloud branch is `language → choose → terms`
 * (3 steps), the self-hosted branch is `language → choose → server → token →
 * done` (5 steps). `WizardShell` sizes the progress dots per branch. */
export type OnboardingStep =
  | "language"
  | "choose"
  | "terms"
  | "server"
  | "token"
  | "done";

const STEPS: OnboardingStep[] = [
  "language",
  "choose",
  "terms",
  "server",
  "token",
  "done",
];

const store = new LazyStore(STORE_FILE);

export async function loadStep(): Promise<OnboardingStep> {
  const raw = await store.get<string>(KEY_STEP);
  if (raw && STEPS.includes(raw as OnboardingStep)) {
    return raw as OnboardingStep;
  }
  return "language";
}

export async function saveStep(step: OnboardingStep): Promise<void> {
  await store.set(KEY_STEP, step);
  await store.save();
}

export async function loadUrl(): Promise<string> {
  return (await store.get<string>(KEY_URL)) ?? "";
}

export async function saveUrl(url: string): Promise<void> {
  await store.set(KEY_URL, url);
  await store.save();
}

/** Wipe wizard state, call this after a successful login. Does NOT clear the
 *  tour flag: logging out and back in shouldn't replay the tour. */
export async function clearOnboarding(): Promise<void> {
  await store.delete(KEY_STEP);
  await store.delete(KEY_URL);
  await store.save();
}

/** The session identity (see {@link KEY_TOUR_SEEN}) the tour was last shown
 *  for, or `null` if never. Compare against the current session to decide
 *  whether to replay it. */
export async function loadTourSeen(): Promise<string | null> {
  return (await store.get<string>(KEY_TOUR_SEEN)) ?? null;
}

/** Remember that the tour was shown for `sig` so it doesn't reopen for the
 *  same account/server on the next launch. */
export async function markTourSeen(sig: string): Promise<void> {
  await store.set(KEY_TOUR_SEEN, sig);
  await store.save();
}

/** Forget which session saw the tour, so the next sign-in, even to the same
 *  account/server, replays it. Called from forget-server / logout / delete. */
export async function clearTourSeen(): Promise<void> {
  await store.delete(KEY_TOUR_SEEN);
  await store.save();
}

export function routeForStep(step: OnboardingStep): string {
  switch (step) {
    case "language":
      return "/onboarding/language";
    case "choose":
      return "/onboarding/choose";
    case "terms":
      return "/onboarding/terms";
    case "server":
      return "/onboarding/server";
    case "token":
      return "/onboarding/token";
    case "done":
      return "/onboarding/done";
  }
}
