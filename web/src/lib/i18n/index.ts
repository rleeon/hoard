import { init, register, locale, waitLocale } from 'svelte-i18n';
import { browser } from '$app/environment';
import { DEFAULT_LOCALE, LOCALES, isLocale } from './locales';

// Where the chosen language is remembered. Locale-prefixed routes stay
// URL-driven (the prefix wins via the `[[lang]]` layout), but functional routes
// (/account, /login, /checkout, /auth) live outside that tree and carry no
// prefix, on a direct load or refresh nothing would set their language, so
// they'd fall back to English. Persisting the last locale lets them restore it.
const STORAGE_KEY = 'hoard:locale';

// Lazy-register every locale so a page only ships the JSON it actually renders
// (8 locales × ~15 KB would otherwise land in every bundle). svelte-i18n loads
// the matching file on `locale.set` / `waitLocale`.
const loaders = new Map<string, () => Promise<unknown>>();
for (const l of LOCALES) {
  const load = async () => (await import(`./locales/${l}.json`)).default;
  loaders.set(l, load);
  register(l, load);
}

/**
 * Fetch a locale's messages ahead of the navigation that will need them.
 *
 * Switching language blocks on `waitLocale` in the `[[lang]]` layout, so the
 * new page cannot paint until its JSON arrives and the switch feels like it
 * stutters. Calling this on hover or pointerdown starts that same request a few
 * hundred milliseconds earlier, while the finger is still on the way down. It
 * shares the loader `register` uses, so it hits the same chunk and downloads
 * nothing the switch was not going to download anyway; a second call is free
 * because the module is already resolved.
 */
export function preloadLocale(l: string) {
  loaders.get(l)?.();
}

let started = false;

/**
 * Initialise svelte-i18n once. The real locale for a page is set by the
 * `[[lang=locale]]` layout load (route-driven), not by the navigator, that is
 * what lets the prerendered HTML come out in the right language instead of
 * always English. On the client we seed the initial locale from the persisted
 * choice (above) so prefix-less functional pages don't flip back to English.
 */
export function setupI18n(initialLocale: string = DEFAULT_LOCALE) {
  if (started) return;
  started = true;
  if (browser) {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLocale(saved)) initialLocale = saved;
    // Remember every subsequent change (switcher, route-driven prefix) so the
    // next prefix-less page load reflects the language the user is actually in.
    locale.subscribe((l) => {
      if (isLocale(l)) localStorage.setItem(STORAGE_KEY, l);
    });
  }
  init({ fallbackLocale: DEFAULT_LOCALE, initialLocale });
}

export { locale, waitLocale };
