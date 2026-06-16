import { LOCALES, DEFAULT_LOCALE, HREFLANG, SITE_URL, withLocale } from '$lib/i18n/locales';
import { guideSlugs } from '$lib/guides';

export const prerender = true;

// Indexable marketing pages (paths without locale prefix). Functional routes
// (login, checkout, account, auth) are noindex and stay out of the sitemap.
const PATHS = [
  '/',
  '/pricing',
  '/help',
  '/download',
  '/guides',
  ...guideSlugs().map((slug) => `/guides/${slug}`),
  '/legal/terms',
  '/legal/privacy'
];

const loc = (path: string, lang: (typeof LOCALES)[number]) =>
  `${SITE_URL}${withLocale(path, lang)}`;

export function GET() {
  const urls = PATHS.flatMap((path) =>
    LOCALES.map((lang) => {
      const alternates = [
        ...LOCALES.map(
          (l) => `    <xhtml:link rel="alternate" hreflang="${HREFLANG[l]}" href="${loc(path, l)}" />`
        ),
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${loc(path, DEFAULT_LOCALE)}" />`
      ].join('\n');
      return `  <url>\n    <loc>${loc(path, lang)}</loc>\n${alternates}\n  </url>`;
    })
  ).join('\n');

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls}
</urlset>
`;

  return new Response(body, {
    headers: { 'Content-Type': 'application/xml' }
  });
}
