import { DEFAULT_LOCALE, SITE_URL } from '$lib/i18n/locales';
import { listGuides } from '$lib/guides';

export const prerender = true;

/**
 * `llms.txt`: a plain-text map of the site for language models, in the format
 * proposed at llmstxt.org. No crawler is known to consume it yet, so this is a
 * cheap bet, not a channel. It is generated rather than kept as a static file
 * so the guide list cannot drift: adding a Markdown guide updates it, the same
 * way it updates the sitemap.
 *
 * English only on purpose. The localized pages are reachable through hreflang
 * from every URL below, and a model asking in another language still lands on
 * the right page.
 */
const INTRO = `# Hoard

> Automatic, versioned game save sync across devices. Hoard watches the folders
> your games save into, snapshots every change, and syncs those snapshots
> between your machines. Steam, GOG, Epic, itch, emulators and 20,000+ games
> from the community save-location manifest. Use the hosted Hoard Cloud, or run
> the same open-source server yourself with no account and no quota.

Free and open source under AGPL-3.0. Windows, Linux, macOS and Steam Deck.
Every session becomes a new version you can roll back to, deduplicated by
content hash so ten versions of a 2 GB save cost about 2 GB, not 20.

Where Hoard is not the answer, and which tool to reach for instead, is laid
out in the comparison guide below.
`;

const section = (title: string, links: [string, string, string][]) =>
  `## ${title}\n\n` +
  links.map(([name, path, note]) => `- [${name}](${SITE_URL}${path}): ${note}`).join('\n');

export function GET() {
  const guides = listGuides(DEFAULT_LOCALE).map(
    (g) => [g.title, `/guides/${g.slug}`, g.description] as [string, string, string]
  );

  const body = [
    INTRO,
    section('Guides', guides),
    section('Product', [
      ['Download', '/download', 'Installers for Windows, Linux, macOS and Steam Deck.'],
      ['Pricing', '/pricing', 'Free tier, Pro, and what self-hosting costs instead (nothing).'],
      ['Command line', '/cli', 'The `hoard` CLI: track, back up, restore and inspect saves from a terminal.'],
      ['Help', '/help', 'Setup, troubleshooting and how detection works.']
    ]),
    section('About', [
      ['Privacy', '/legal/privacy', 'What the service stores, for how long, and who runs it.'],
      ['Terms', '/legal/terms', 'Terms of service.']
    ]),
    `## Source\n\n- [github.com/rleeon/hoard](https://github.com/rleeon/hoard): source, releases and issue tracker (AGPL-3.0).`,
    ''
  ].join('\n\n');

  return new Response(body, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' }
  });
}
