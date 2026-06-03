import { readable } from 'svelte/store';

const REPO = 'rleeon/hoard';

/** Build-time fallback: workspace Cargo.toml version (vite `define`). */
export const FALLBACK_VERSION: string = __HOARD_VERSION__;

/** Date of the latest released version, read from CHANGELOG at build time. */
export const RELEASE_DATE: string = __HOARD_RELEASE_DATE__;

/** The "latest release" page — always points at the current version. */
export const RELEASES_LATEST = `https://github.com/${REPO}/releases/latest`;
export const ALL_RELEASES = `https://github.com/${REPO}/releases`;
export const CHANGELOG_URL = `https://github.com/${REPO}/blob/main/CHANGELOG.md`;

/**
 * Live version string shown across the site. Seeds with the build-time
 * fallback and, in the browser, upgrades to the latest published GitHub
 * release tag. The displayed number therefore tracks
 * `github.com/rleeon/hoard/releases/latest` automatically — no rebuild, no
 * manual bump. On error (offline / rate-limit) it stays on the fallback.
 */
export const version = readable(FALLBACK_VERSION, (set) => {
  if (typeof fetch === 'undefined') return;
  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: 'application/vnd.github+json' }
  })
    .then((r) => (r.ok ? r.json() : null))
    .then((j) => {
      const tag = j?.tag_name as string | undefined;
      if (tag) set(tag.replace(/^v/, ''));
    })
    .catch(() => {});
});
