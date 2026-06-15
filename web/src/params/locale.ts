import type { ParamMatcher } from '@sveltejs/kit';
import { PREFIXED_LOCALES } from '$lib/i18n/locales';

/**
 * Matches the optional `[[lang]]` segment only when it is one of the prefixed
 * locales (es, de, fr, it, ja, pt, zh). English carries no prefix, so it is
 * excluded — and any non-locale segment (e.g. `/login`) falls through to its
 * own route instead of being swallowed by `[[lang]]`.
 */
export const match: ParamMatcher = (param) =>
  (PREFIXED_LOCALES as readonly string[]).includes(param);
