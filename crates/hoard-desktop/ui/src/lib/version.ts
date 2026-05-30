/**
 * Single source of truth for the app version shown in the UI.
 *
 * The value is injected at build/dev time by Vite (`define` in
 * `vite.config.ts`) straight from `package.json`'s `version` field, so there
 * is exactly one number to bump. Both the sidebar badge and the "About"
 * section read this constant — never hardcode a version string in a component
 * or a locale file again.
 *
 * The `"dev"` fallback only ever shows if the bundle is loaded without Vite's
 * define (which shouldn't happen in a real build); it's a visible signal that
 * the injection broke rather than a silently stale number.
 */
export const APP_VERSION: string =
  import.meta.env.VITE_HOARD_VERSION || "dev";
