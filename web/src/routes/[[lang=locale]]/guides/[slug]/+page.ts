import { error } from '@sveltejs/kit';
import { getGuide, guideSlugs } from '$lib/guides';
import { DEFAULT_LOCALE, LOCALES, isLocale, type Locale } from '$lib/i18n/locales';

export const load = ({ params }) => {
  const lang: Locale = isLocale(params.lang) ? (params.lang as Locale) : DEFAULT_LOCALE;
  const guide = getGuide(params.slug, lang);
  if (!guide) throw error(404, 'Guide not found');
  return { guide };
};

/**
 * Enumerate every (locale × slug) combo so each guide is prerendered in all 8
 * languages. Each slug is emitted at the bare path (English, no `lang`) AND
 * under every locale prefix, including `/en/...`, an explicit alias of the
 * bare English guide.
 */
export const entries = () =>
  guideSlugs().flatMap((slug) => [{ slug }, ...LOCALES.map((lang) => ({ lang, slug }))]);
