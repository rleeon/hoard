/**
 * Persisted wizard state.
 *
 * If the user closes the app halfway through onboarding we bring them back
 * to the same step on the next launch. Storage lives in
 * `tauri-plugin-store` (a JSON file managed by Tauri), not in the browser's
 * localStorage — we want it to survive a webview cache wipe.
 */

import { LazyStore } from "@tauri-apps/plugin-store";

const STORE_FILE = "onboarding.json";
const KEY_STEP = "step";
const KEY_URL = "url";

/** Routes that make up the wizard, in order. */
export type OnboardingStep =
  | "welcome"
  | "choose"
  | "server"
  | "token"
  | "done";

const STEPS: OnboardingStep[] = ["welcome", "choose", "server", "token", "done"];

const store = new LazyStore(STORE_FILE);

export async function loadStep(): Promise<OnboardingStep> {
  const raw = await store.get<string>(KEY_STEP);
  if (raw && STEPS.includes(raw as OnboardingStep)) {
    return raw as OnboardingStep;
  }
  return "welcome";
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

/** Wipe wizard state — call this after a successful login. */
export async function clearOnboarding(): Promise<void> {
  await store.delete(KEY_STEP);
  await store.delete(KEY_URL);
  await store.save();
}

export function routeForStep(step: OnboardingStep): string {
  switch (step) {
    case "welcome":
      return "/welcome";
    case "choose":
      return "/onboarding/choose";
    case "server":
      return "/onboarding/server";
    case "token":
      return "/onboarding/token";
    case "done":
      return "/onboarding/done";
  }
}
