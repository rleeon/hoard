/**
 * Internationalization bootstrap.
 *
 * Registers every locale lazily so messages aren't bundled into the initial
 * chunk, and seeds the active locale from the persisted user preference (with
 * a fallback to the navigator's language). Components import `$_`, `locale`,
 * etc. from `svelte-i18n` directly.
 */
import {
  init,
  locale,
  register,
  waitLocale,
  getLocaleFromNavigator,
} from "svelte-i18n";
import { get } from "svelte/store";

import { getPrefs, savePrefs } from "../api";

/** Locales the UI ships translations for. Adding a new one means dropping a
 *  JSON in `locales/` and pushing an entry here. */
export const supportedLocales: { code: string; label: string }[] = [
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "pt", label: "Português" },
  { code: "it", label: "Italiano" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "简体中文" },
];

// Lazy registration — Vite splits each JSON into its own chunk, so users only
// pay the cost of the locale they actually pick.
register("en", () => import("./locales/en.json"));
register("es", () => import("./locales/es.json"));
register("fr", () => import("./locales/fr.json"));
register("de", () => import("./locales/de.json"));
register("pt", () => import("./locales/pt.json"));
register("it", () => import("./locales/it.json"));
register("ja", () => import("./locales/ja.json"));
register("zh", () => import("./locales/zh.json"));

/** Pick the best available locale from a navigator string like "fr-FR". We
 *  match on the primary subtag so "pt-BR" still finds our `pt` bundle. */
function pickSupported(candidate: string | null | undefined): string | null {
  if (!candidate) return null;
  const lower = candidate.toLowerCase();
  const exact = supportedLocales.find((l) => l.code === lower);
  if (exact) return exact.code;
  const primary = lower.split(/[-_]/)[0];
  const match = supportedLocales.find((l) => l.code === primary);
  return match ? match.code : null;
}

// Seed `init` synchronously with a best-effort initial locale (navigator),
// then asynchronously override with the persisted preference once disk I/O
// resolves. This keeps the first paint localized correctly for users whose
// browser language matches their pref, which is the common case.
init({
  fallbackLocale: "en",
  initialLocale: pickSupported(getLocaleFromNavigator()) ?? "en",
});

/** Promise the bootstrap (`main.ts`) awaits before calling `mount()`.
 *
 *  svelte-i18n's `$_` throws "Cannot format a message without first setting
 *  the initial locale" if a component renders before the locale loader has
 *  resolved. `init()` only *queues* the load — we must explicitly wait for
 *  it. We also try to read the persisted language preference here so the
 *  first paint already uses the user's chosen locale (no flash from
 *  navigator-language → persisted-language). */
export const i18nReady: Promise<void> = (async () => {
  try {
    const prefs = await getPrefs();
    const persisted = pickSupported(prefs.language ?? null);
    if (persisted && persisted !== get(locale)) {
      await locale.set(persisted);
    }
  } catch {
    // Falling back to whatever `init` picked is fine — the Settings page can
    // repair prefs.json if it was corrupt.
  }
  // Block until the active locale's dictionary is loaded. Without this the
  // very first render sees `$locale = null` and `$_` throws, which silently
  // unwinds Svelte's mount() and leaves the user with a blank window.
  await waitLocale();
})();

/** Update the active locale and persist it to prefs so the next launch
 *  remembers the choice. */
export async function setLocale(code: string): Promise<void> {
  await locale.set(code);
  try {
    const current = await getPrefs();
    await savePrefs({ ...current, language: code });
  } catch {
    // Persisting is best-effort; the in-memory switch already happened.
  }
}
