/**
 * Typed wrappers around Tauri's `invoke()`.
 *
 * Keeping all `invoke` calls here means commands have one source of truth for
 * their argument and return shapes — components just import a function and
 * the compiler enforces the contract.
 */

import { invoke } from "@tauri-apps/api/core";

export type HealthInfo = {
  status: string;
  version: string;
  uptime_secs: number;
};

export type UserInfo = {
  user_id: string;
  username: string;
  is_admin: boolean;
  server_url: string;
};

/** Anonymous probe — used by the wizard to validate the server URL. */
export function healthCheck(url: string): Promise<HealthInfo> {
  return invoke<HealthInfo>("health_check", { url });
}

/** Verify a (URL, token) pair against the server and persist it. */
export function login(url: string, token: string): Promise<UserInfo> {
  return invoke<UserInfo>("login", { url, token });
}

/** Cheap, sync check — does the app have a saved session? */
export function isLoggedIn(): Promise<boolean> {
  return invoke<boolean>("is_logged_in");
}

/** Cached user info from the saved session, or null. */
export function currentUser(): Promise<UserInfo | null> {
  return invoke<UserInfo | null>("current_user");
}

/** Wipe stored credentials and the in-memory cache. */
export function logout(): Promise<void> {
  return invoke<void>("logout");
}

/** Phase 0 sanity-check command, kept for the dev "ping Rust" widget. */
export function greet(name: string): Promise<string> {
  return invoke<string>("greet", { name });
}
