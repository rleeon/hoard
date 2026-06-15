import { PREFIXED_LOCALES } from './locales';

/**
 * Prerender entries for a page under `[[lang=locale]]`. SvelteKit's default
 * `*` entry already generates the prefix-less English variant (the optional
 * param has no required value); this adds one entry per prefixed locale so
 * `/es/...`, `/de/...`, … are emitted too. Re-export as `entries` from each
 * marketing `+page.ts`.
 */
export const localeEntries = () => PREFIXED_LOCALES.map((lang) => ({ lang }));
