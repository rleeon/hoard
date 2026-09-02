import type { ParamMatcher } from '@sveltejs/kit';
import { LOCALES } from '$lib/i18n/locales';

/**
 * Matches the optional `[[lang]]` segment when it is any known locale,
 * including `en`. English is also served prefix-less (`/guides`), so `/en/...`
 * is an explicit alias of the bare path with identical content. Any non-locale
 * segment (e.g. `/login`) falls through to its own route instead of being
 * swallowed by `[[lang]]`.
 */
export const match: ParamMatcher = (param) => (LOCALES as readonly string[]).includes(param);
