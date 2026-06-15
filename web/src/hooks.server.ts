import type { Handle } from '@sveltejs/kit';
import { PREFIXED_LOCALES } from '$lib/i18n/locales';

/**
 * Stamp the right `lang` on <html> per prerendered page. The first path
 * segment is the locale prefix for every non-English page (`/es/...`); English
 * pages have no prefix and fall back to `en`. Runs during prerender with
 * adapter-static, so each emitted HTML file carries its own `lang`.
 */
export const handle: Handle = async ({ event, resolve }) => {
  const seg = event.url.pathname.split('/')[1];
  const lang = (PREFIXED_LOCALES as readonly string[]).includes(seg) ? seg : 'en';
  return resolve(event, {
    transformPageChunk: ({ html }) => html.replace('%lang%', lang)
  });
};
