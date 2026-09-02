import { locale, waitLocale } from 'svelte-i18n';
import { DEFAULT_LOCALE } from '$lib/i18n/locales';

/**
 * Route-driven locale: the `[[lang]]` segment decides the language, and
 * `waitLocale` blocks the render until its messages are loaded, this is what
 * makes the prerendered HTML come out in the right language (English when the
 * segment is absent). The default `*` prerender entry handles the prefix-less
 * English pages; `localeEntries` in each `+page.ts` adds the prefixed ones.
 */
export const load = async ({ params }) => {
  const lang = params.lang ?? DEFAULT_LOCALE;
  locale.set(lang);
  await waitLocale(lang);
  return { lang };
};
