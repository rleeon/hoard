import { readable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const REPO = 'rleeon/hoard';

/** Build-time seeds (vite `define`): workspace Cargo.toml version + top
 *  dated CHANGELOG entry. Only shown until the live GitHub lookup lands. */
const SEED_VERSION: string = __HOARD_VERSION__;
const SEED_DATE: string = __HOARD_RELEASE_DATE__;

/** The "latest release" page — always points at the current version. */
export const RELEASES_LATEST = `https://github.com/${REPO}/releases/latest`;
export const ALL_RELEASES = `https://github.com/${REPO}/releases`;
export const CHANGELOG_URL = `https://github.com/${REPO}/blob/main/CHANGELOG.md`;

/** Direct-download URL per installer, so clicking starts the download
 *  instead of landing on the release page. */
export type ReleaseAssets = {
  windowsSetup: string;
  windowsMsi: string;
  macosDmg: string;
  linuxDeb: string;
  linuxAppImage: string;
  linuxRpm: string;
};

export type ReleaseInfo = { v: string; date: string; assets: ReleaseAssets };

/** CI publishes assets with these exact names (see the release workflow);
 *  used as the build-time seed and as fallback if one is missing upstream. */
function assetsFor(v: string): ReleaseAssets {
  const base = `https://github.com/${REPO}/releases/download/v${v}`;
  return {
    windowsSetup: `${base}/Hoard_${v}_x64-setup.exe`,
    windowsMsi: `${base}/Hoard_${v}_x64_en-US.msi`,
    macosDmg: `${base}/Hoard_${v}_aarch64.dmg`,
    linuxDeb: `${base}/Hoard_${v}_amd64.deb`,
    linuxAppImage: `${base}/Hoard_${v}_amd64.AppImage`,
    linuxRpm: `${base}/Hoard-${v}-1.x86_64.rpm`
  };
}

/** Prefer the URLs GitHub actually reports over the naming convention. */
function pickAssets(urls: string[], v: string): ReleaseAssets {
  const find = (re: RegExp) => urls.find((u) => re.test(u));
  const guess = assetsFor(v);
  return {
    windowsSetup: find(/-setup\.exe$/) ?? guess.windowsSetup,
    windowsMsi: find(/\.msi$/) ?? guess.windowsMsi,
    macosDmg: find(/\.dmg$/) ?? guess.macosDmg,
    linuxDeb: find(/\.deb$/) ?? guess.linuxDeb,
    linuxAppImage: find(/\.appimage$/i) ?? guess.linuxAppImage,
    linuxRpm: find(/\.rpm$/) ?? guess.linuxRpm
  };
}

const CACHE_KEY = 'hoard:release-latest';
const CACHE_TTL_MS = 60 * 60 * 1000; // GitHub allows 60 unauthenticated req/h

function readCache(): (ReleaseInfo & { at: number }) | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const c = JSON.parse(raw) as Partial<ReleaseInfo> & { at: number };
    if (typeof c?.v !== 'string' || typeof c?.at !== 'number') return null;
    // Older cache entries predate `assets` — rebuild them from the version.
    return { v: c.v, date: c.date ?? SEED_DATE, assets: c.assets ?? assetsFor(c.v), at: c.at };
  } catch {
    return null;
  }
}

/**
 * Latest published GitHub release, shown everywhere a version appears
 * (hero, footer, download). Seeds with the build-time values and, in the
 * browser, upgrades to the live `releases/latest` tag + publish date —
 * the displayed number therefore tracks GitHub automatically, with no
 * hand-edited string anywhere. The result is cached in localStorage for
 * an hour to stay clear of the unauthenticated API rate limit; on error
 * (offline / rate-limited) it stays on the last known value.
 */
export const release = readable<ReleaseInfo>(
  { v: SEED_VERSION, date: SEED_DATE, assets: assetsFor(SEED_VERSION) },
  (set) => {
    if (!browser) return;

    const cached = readCache();
    if (cached) {
      set({ v: cached.v, date: cached.date, assets: cached.assets });
      if (Date.now() - cached.at < CACHE_TTL_MS) return;
    }

    fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: 'application/vnd.github+json' }
    })
      .then((r) => (r.ok ? r.json() : null))
      .then((j) => {
        const tag = (j?.tag_name as string | undefined)?.replace(/^v/, '');
        if (!tag) return;
        const date = (j?.published_at as string | undefined)?.slice(0, 10) ?? SEED_DATE;
        const urls = ((j?.assets ?? []) as { browser_download_url?: string }[])
          .map((a) => a.browser_download_url)
          .filter((u): u is string => typeof u === 'string');
        const assets = pickAssets(urls, tag);
        set({ v: tag, date, assets });
        try {
          localStorage.setItem(CACHE_KEY, JSON.stringify({ v: tag, date, assets, at: Date.now() }));
        } catch {
          /* storage disabled — fine, next load refetches */
        }
      })
      .catch(() => {});
  }
);

/** Just the version string — most call sites only need this. */
export const version = derived(release, (r) => r.v);
