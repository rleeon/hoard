import { derived } from 'svelte/store';
import { locale } from 'svelte-i18n';
import { DEFAULT_LOCALE, isLocale, withLocale, type Locale } from './locales';

/**
 * Store of a function that localizes an internal path to the active locale:
 * `href={$localeHref('/pricing')}` → `/pricing` in English, `/es/pricing` in
 * Spanish, etc. Use for every internal marketing link so navigation stays
 * within the current language.
 */
export const localeHref = derived(locale, ($l) => {
  const loc: Locale = isLocale($l) ? $l : DEFAULT_LOCALE;
  return (path: string) => withLocale(path, loc);
});
