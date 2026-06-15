import { init, register, locale, waitLocale } from 'svelte-i18n';
import { DEFAULT_LOCALE, LOCALES } from './locales';

// Lazy-register every locale so a page only ships the JSON it actually renders
// (8 locales × ~15 KB would otherwise land in every bundle). svelte-i18n loads
// the matching file on `locale.set` / `waitLocale`.
for (const l of LOCALES) {
  register(l, async () => (await import(`./locales/${l}.json`)).default);
}

let started = false;

/**
 * Initialise svelte-i18n once. The real locale for a page is set by the
 * `[[lang=locale]]` layout load (route-driven), not by the navigator — that is
 * what lets the prerendered HTML come out in the right language instead of
 * always English.
 */
export function setupI18n(initialLocale: string = DEFAULT_LOCALE) {
  if (started) return;
  started = true;
  init({ fallbackLocale: DEFAULT_LOCALE, initialLocale });
}

export { locale, waitLocale };
