import { readable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const REPO = 'rleeon/hoard';

/** Build-time seeds (vite `define`): workspace Cargo.toml version + top
 *  dated CHANGELOG entry. Only shown until the live GitHub lookup lands. */
const SEED_VERSION: string = __HOARD_VERSION__;
const SEED_DATE: string = __HOARD_RELEASE_DATE__;

/** The "latest release" page, always points at the current version. */
export const RELEASES_LATEST = `https://github.com/${REPO}/releases/latest`;
export const ALL_RELEASES = `https://github.com/${REPO}/releases`;
export const CHANGELOG_URL = `https://github.com/${REPO}/blob/main/CHANGELOG.md`;

/** Direct-download URL per installer, so clicking starts the download
 *  instead of landing on the release page. */
export type ReleaseAssets = {
  // Hoard Setup: the graphical installer, one per platform. It is the normal
  // way in, it fetches the right package for the machine it is on, and the
  // raw bundles below stay published for anyone who would rather not be
  // helped.
  setupWindows: string | null;
  setupWindowsArm64: string | null;
  setupMacos: string | null;
  setupLinux: string | null;
  setupLinuxArm64: string | null;
  windowsSetup: string;
  windowsSetupArm64: string | null;
  windowsMsi: string;
  macosDmg: string;
  linuxDeb: string;
  linuxDebArm64: string | null;
  linuxAppImage: string;
  linuxAppImageArm64: string | null;
  linuxRpm: string;
  linuxRpmArm64: string | null;
  // Headless CLI tarballs (`hoard` binary; Linux tarballs also bundle
  // hoard-server + hoard-admin). See `/cli`.
  cliLinuxX64: string;
  cliLinuxArm64: string;
  cliMacosArm64: string;
  cliWindowsX64: string;
  cliWindowsArm64: string;
};

export type ReleaseInfo = { v: string; date: string; assets: ReleaseAssets };

/** CI publishes assets with these exact names (see the release workflow);
 *  used as the build-time seed and as fallback if one is missing upstream. */
function assetsFor(v: string): ReleaseAssets {
  const base = `https://github.com/${REPO}/releases/download/v${v}`;
  return {
    // Hoard Setup and the ARM bundles are younger than the x86 ones, so a
    // release from before they existed lists none of them, and a URL built
    // from the convention would be a 404 wearing a plausible filename, which
    // is worse than not offering the download at all. No guess for these:
    // either the release names the file or the page does not offer it.
    setupWindows: null,
    setupWindowsArm64: null,
    setupMacos: null,
    setupLinux: null,
    setupLinuxArm64: null,
    windowsSetup: `${base}/Hoard_${v}_x64-setup.exe`,
    windowsSetupArm64: null,
    // x64 only: the ARM desktop bundle is NSIS, no MSI. See release-desktop.yml.
    windowsMsi: `${base}/Hoard_${v}_x64_en-US.msi`,
    macosDmg: `${base}/Hoard_${v}_aarch64.dmg`,
    linuxDeb: `${base}/Hoard_${v}_amd64.deb`,
    linuxDebArm64: null,
    // Not a typo: the AppImage bundler writes `amd64` for x86_64 and
    // `aarch64` for ARM, where the .deb writes `amd64`/`arm64`.
    linuxAppImage: `${base}/Hoard_${v}_amd64.AppImage`,
    linuxAppImageArm64: null,
    linuxRpm: `${base}/Hoard-${v}-1.x86_64.rpm`,
    linuxRpmArm64: null,
    cliLinuxX64: `${base}/hoard-${v}-linux-x86_64.tar.gz`,
    cliLinuxArm64: `${base}/hoard-${v}-linux-aarch64.tar.gz`,
    cliMacosArm64: `${base}/hoard-${v}-macos-aarch64.tar.gz`,
    cliWindowsX64: `${base}/hoard-${v}-windows-x86_64.tar.gz`,
    cliWindowsArm64: `${base}/hoard-${v}-windows-aarch64.tar.gz`
  };
}

/** Prefer the URLs GitHub actually reports over the naming convention. */
function pickAssets(urls: string[], v: string): ReleaseAssets {
  const find = (re: RegExp) => urls.find((u) => re.test(u));
  const guess = assetsFor(v);
  return {
    // Every one of these matches on the architecture too, which it did not
    // have to when a release carried a single bundle per format. Now that ARM
    // bundles ship alongside, a bare `/\.deb$/` would hand whichever GitHub
    // happened to list first, an arm64 .deb to an x86 laptop is not a loud
    // failure, it is dpkg complaining about something that looks unrelated.
    // Version-less names on purpose: the installer resolves the release
    // itself, so the file does not go stale between releases.
    setupWindows: find(/HoardSetup-x86_64\.exe$/) ?? null,
    setupWindowsArm64: find(/HoardSetup-aarch64\.exe$/) ?? null,
    setupMacos: find(/HoardSetup-aarch64\.zip$/) ?? null,
    setupLinux: find(/HoardSetup-x86_64$/) ?? null,
    setupLinuxArm64: find(/HoardSetup-aarch64$/) ?? null,
    windowsSetup: find(/x64-setup\.exe$/) ?? guess.windowsSetup,
    windowsSetupArm64: find(/arm64-setup\.exe$/) ?? null,
    windowsMsi: find(/\.msi$/) ?? guess.windowsMsi,
    macosDmg: find(/\.dmg$/) ?? guess.macosDmg,
    linuxDeb: find(/_amd64\.deb$/) ?? guess.linuxDeb,
    linuxDebArm64: find(/_arm64\.deb$/) ?? null,
    linuxAppImage: find(/_amd64\.appimage$/i) ?? guess.linuxAppImage,
    linuxAppImageArm64: find(/_aarch64\.appimage$/i) ?? null,
    linuxRpm: find(/\.x86_64\.rpm$/) ?? guess.linuxRpm,
    linuxRpmArm64: find(/\.aarch64\.rpm$/) ?? null,
    // Match the CLI tarballs by their exact platform-arch suffix so they never
    // collide with the desktop `Hoard.app.tar.gz` (which also ends in .tar.gz).
    cliLinuxX64: find(/linux-x86_64\.tar\.gz$/) ?? guess.cliLinuxX64,
    cliLinuxArm64: find(/linux-aarch64\.tar\.gz$/) ?? guess.cliLinuxArm64,
    cliMacosArm64: find(/macos-aarch64\.tar\.gz$/) ?? guess.cliMacosArm64,
    cliWindowsX64: find(/windows-x86_64\.tar\.gz$/) ?? guess.cliWindowsX64,
    cliWindowsArm64: find(/windows-aarch64\.tar\.gz$/) ?? guess.cliWindowsArm64
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
    // Older cache entries predate `assets`, rebuild them from the version.
    return {
      v: c.v,
      date: c.date ?? SEED_DATE,
      assets: c.assets ?? assetsFor(c.v),
      at: c.at
    };
  } catch {
    return null;
  }
}

/**
 * Latest published GitHub release, shown everywhere a version appears
 * (hero, footer, download). Seeds with the build-time values and, in the
 * browser, upgrades to the live `releases/latest` tag + publish date,
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
          localStorage.setItem(
            CACHE_KEY,
            JSON.stringify({ v: tag, date, assets, at: Date.now() })
          );
        } catch {
          /* storage disabled, fine, next load refetches */
        }
      })
      .catch(() => {});
  }
);

/** Just the version string, most call sites only need this. */
export const version = derived(release, (r) => r.v);
