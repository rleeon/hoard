import { error } from '@sveltejs/kit';
import { LOCALES } from '$lib/i18n/locales';
import { NOTICES, isNoticeSlug } from '$lib/notices';

/**
 * One page per service email, in every locale.
 *
 * The emails themselves go out in English (`profiles` has no language column
 * yet), so each one links here from its footer: same explanation, in the
 * reader's language. The URL carries the notice slug and nothing else, never
 * the numbers from the message, so a forwarded link leaks nothing about the
 * account that received it.
 */
export const entries = () =>
  LOCALES.flatMap((lang) => NOTICES.map((slug) => ({ lang, slug })));

export const load = ({ params }) => {
  if (!isNoticeSlug(params.slug)) error(404, 'Unknown notice');
  return { slug: params.slug };
};
