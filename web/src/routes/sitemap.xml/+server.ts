import { LOCALES, DEFAULT_LOCALE, HREFLANG, SITE_URL, withLocale } from '$lib/i18n/locales';
import { getGuide, guideSlugs } from '$lib/guides';

export const prerender = true;

// Indexable marketing pages (paths without locale prefix). Functional routes
// (login, checkout, account, auth) are noindex and stay out of the sitemap.
const PATHS = [
  '/',
  '/pricing',
  '/help',
  '/download',
  '/cli',
  '/guides',
  ...guideSlugs().map((slug) => `/guides/${slug}`),
  '/legal/terms',
  '/legal/privacy',
  '/legal/subprocessors',
  '/legal/notice'
];

const loc = (path: string, lang: (typeof LOCALES)[number]) =>
  `${SITE_URL}${withLocale(path, lang)}`;

/**
 * `<lastmod>` for the paths that have a real edit date: the guides, from their
 * `updated` frontmatter, plus the index, which is as fresh as its newest guide.
 * The marketing pages get none on purpose: stamping a build date on every URL
 * every deploy is what teaches a crawler to ignore the field.
 */
const lastmod = (path: string, lang: (typeof LOCALES)[number]) => {
  if (path === '/guides') {
    const dates = guideSlugs()
      .map((slug) => getGuide(slug, lang)?.updated)
      .filter(Boolean) as string[];
    return dates.sort().at(-1) ?? '';
  }
  const slug = path.startsWith('/guides/') ? path.slice('/guides/'.length) : '';
  return (slug && getGuide(slug, lang)?.updated) || '';
};

export function GET() {
  const urls = PATHS.flatMap((path) =>
    LOCALES.map((lang) => {
      const alternates = [
        ...LOCALES.map(
          (l) => `    <xhtml:link rel="alternate" hreflang="${HREFLANG[l]}" href="${loc(path, l)}" />`
        ),
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${loc(path, DEFAULT_LOCALE)}" />`
      ].join('\n');
      const mod = lastmod(path, lang);
      return `  <url>\n    <loc>${loc(path, lang)}</loc>\n${
        mod ? `    <lastmod>${mod}</lastmod>\n` : ''
      }${alternates}\n  </url>`;
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
