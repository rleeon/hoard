import type { RequestHandler } from './$types';
import { PRESS_ASSETS } from '$lib/press/assets';
import { storedZip } from '$lib/press/zip';
import { SITE_URL } from '$lib/i18n/locales';
import en from '$lib/i18n/locales/en.json';

export const prerender = true;

const CONTACT = 'support@hoard.services';

// The zip carries its own copy of the texts so whoever unpacks it does not
// have to come back to the page. English only, like llms.txt.
function about(): string {
  const press = __HOARD_PRESS__;
  const files = press.assets.map((a) => {
    const size = a.width && a.height ? ` (${a.width} x ${a.height})` : '';
    return `  ${a.zipName}  ${en[a.label as keyof typeof en]}${size}`;
  });
  const release = press.release
    ? [
        `LATEST RELEASE: v${press.release.version} (${press.release.date})`,
        press.release.summary,
        'Full notes: https://github.com/rleeon/hoard/blob/main/CHANGELOG.md',
        ''
      ]
    : [];
  return [
    'Hoard press kit',
    `${SITE_URL}/press`,
    '',
    'ONE LINE',
    en['press.about.line'],
    '',
    'SHORT',
    en['press.about.short'],
    '',
    'LONG',
    en['press.about.long'],
    '',
    ...release,
    'FILES',
    ...files,
    '',
    en['press.usage_body'],
    '',
    `Website:  ${SITE_URL}`,
    `Download: ${SITE_URL}/download`,
    'Source:   https://github.com/rleeon/hoard',
    `Contact:  ${CONTACT}`,
    ''
  ].join('\r\n');
}

export const GET: RequestHandler = async ({ fetch }) => {
  // Served from static/ by SvelteKit's own fetch during prerender, so a file
  // missing from disk fails the build here.
  const files = await Promise.all(
    PRESS_ASSETS.map(async (a) => {
      const res = await fetch(`/${a.file}`);
      if (!res.ok) throw new Error(`press kit: /${a.file} answered ${res.status}`);
      return { name: `hoard-press-kit/${a.zipName}`, data: new Uint8Array(await res.arrayBuffer()) };
    })
  );
  files.unshift({ name: 'hoard-press-kit/ABOUT.txt', data: new TextEncoder().encode(about()) });

  const stamp = new Date(`${__HOARD_PRESS__.release?.date ?? '2026-01-01'}T12:00:00Z`);
  return new Response(storedZip(files, stamp), {
    headers: { 'Content-Type': 'application/zip' }
  });
};
