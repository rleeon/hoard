import { LOCALES } from './locales';

/**
 * Prerender entries for a page under `[[lang=locale]]`. SvelteKit's default
 * `*` entry generates the prefix-less variant (the optional param absent),
 * which serves English at the bare path (`/guides`). This adds one prefixed
 * entry per locale, *including* `en`, so `/en/...`, `/es/...`, `/de/...`, …
 * are all emitted. `/en/x` is an explicit alias of the bare `/x` (same English
 * content; the page's canonical points back at the bare URL). Re-export as
 * `entries` from each marketing `+page.ts`.
 */
export const localeEntries = () => LOCALES.map((lang) => ({ lang }));
