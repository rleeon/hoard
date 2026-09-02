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

// Lazy registration, Vite splits each JSON into its own chunk, so users only
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

/** How long the persisted language may delay the first render.
 *
 *  Reading `prefs.json` is an `invoke` plus a disk read: usually a couple of
 *  milliseconds, but it is I/O and nothing guarantees it stays that way (a sleeping
 *  disk, a profile on the network, an antivirus watching the file). Under this
 *  deadline we wait and the start comes up in the right language; over it we mount
 *  with the system's and correct once it answers. */
const PREFS_LOCALE_BUDGET_MS = 250;

/** The language stored in prefs, or `null` when there is none or it could not be
 *  read. It starts on module import so it runs in parallel with the rest of the
 *  bootstrap. */
const persistedLocale: Promise<string | null> = getPrefs()
  .then((prefs) => pickSupported(prefs.language ?? null))
  // Keeping whatever `init` picked is acceptable, since the Settings page can
  // repair a corrupt prefs.json.
  .catch(() => null);

/** Resolves to `null` when `promise` takes longer than `ms`, without cancelling it. */
function withBudget<T>(promise: Promise<T>, ms: number): Promise<T | null> {
  return Promise.race([
    promise,
    new Promise<null>((resolve) => setTimeout(() => resolve(null), ms)),
  ]);
}

/** Aplica el idioma persistido si difiere del activo. */
async function applyPersisted(code: string | null): Promise<void> {
  if (code && code !== get(locale)) {
    await locale.set(code);
  }
}

/** Promise the bootstrap (`main.ts`) awaits before calling `mount()`.
 *
 *  svelte-i18n's `$_` throws "Cannot format a message without first setting
 *  the initial locale" if a component renders before the locale loader has
 *  resolved. `init()` only *queues* the load, we must explicitly wait for
 *  it.
 *
 *  What this promise does **not** do is wait on the disk without a bound. It used
 *  to chain `getPrefs()` and then `waitLocale()`, so `mount()`, and with it the
 *  app's first pixel, sat behind an IPC round-trip. The prefs read now runs in
 *  parallel and is only waited on for `PREFS_LOCALE_BUDGET_MS`; if it arrives late,
 *  we mount with the system's language and [`i18nSettled`] applies it afterwards
 *  (svelte-i18n is reactive: the text changes on its own, with nothing
 *  remounted). */
export const i18nReady: Promise<void> = (async () => {
  await applyPersisted(await withBudget(persistedLocale, PREFS_LOCALE_BUDGET_MS));
  // Block until the active locale's dictionary is loaded. Without this the
  // very first render sees `$locale = null` and `$_` throws, which silently
  // unwinds Svelte's mount() and leaves the user with a blank window.
  await waitLocale();
})();

/** El idioma definitivo, ya con la preferencia del disco aplicada llegase
 *  cuando llegase. Nadie tiene que esperarlo para pintar; existe para que el
 *  arranque lento acabe igualmente en el idioma elegido (y para que un test
 *  pueda esperar a que todo se asiente). */
export const i18nSettled: Promise<void> = (async () => {
  try {
    await i18nReady;
    await applyPersisted(await persistedLocale);
  } catch {
    // A dictionary that fails to load leaves the text in the previous language,
    // which beats an unowned rejected promise in the console.
  }
})();

/** The read in flight, so a burst of events collapses into one. */
let syncing: Promise<void> | null = null;

/**
 * Re-reads the stored language and applies it when it has fallen behind.
 *
 * It is needed because [`setLocale`] changes the language **of the window that
 * calls it** and persists to prefs, but tells nobody: every window has its own JS
 * context and therefore its own svelte-i18n store. The main window is the only one
 * that goes through Settings, so a second window that survives the change keeps the
 * language it mounted with, and the game HUD always survives, because closing it
 * **hides** it rather than destroying it.
 *
 * Calling it when it is shown again is the cheap half of the fix: one prefs read per
 * opening, with no new events to emit or hear, and nothing it can trigger. If prefs
 * cannot be read, or no language was chosen, whatever was there stays: there is
 * nothing better than guessing.
 */
export function syncPersistedLocale(): Promise<void> {
  if (syncing) return syncing;
  syncing = (async () => {
    try {
      const prefs = await getPrefs();
      await applyPersisted(pickSupported(prefs.language ?? null));
    } catch {
      /* best-effort */
    } finally {
      syncing = null;
    }
  })();
  return syncing;
}

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
