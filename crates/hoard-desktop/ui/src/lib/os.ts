/**
 * Which OS the webview is running on, cheaply.
 *
 * `@tauri-apps/plugin-os` isn't installed, and the Tauri webview keeps the host
 * UA on every platform, so the heuristic is reliable enough for what depends on
 * it: the font stack, the resize grips of our own title bar, and the one effect
 * WebKitGTK gets wrong (`backdrop-blur`, which paints a black rectangle on
 * hover).
 */
export type OsTag = "linux" | "macos" | "windows" | "unknown";

let cached: OsTag | null = null;

export function osTag(): OsTag {
  if (cached) return cached;
  const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
  if (/Linux/i.test(ua) && !/Android/i.test(ua)) cached = "linux";
  else if (/Mac/i.test(ua)) cached = "macos";
  else if (/Windows/i.test(ua)) cached = "windows";
  else cached = "unknown";
  return cached;
}

/** Puts `is-<os>` on `<html>` so the global stylesheet can key off it.
 *  Idempotent, `classList` dedupes, and cheap enough to call from wherever
 *  needs the class to be there already. */
export function tagOs(): OsTag {
  const tag = osTag();
  if (typeof document !== "undefined") {
    document.documentElement.classList.add(`is-${tag}`);
  }
  return tag;
}

export function isWindows(): boolean {
  return osTag() === "windows";
}
