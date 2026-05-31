<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { spotlight } from '$lib/actions/spotlight';
  import { onMount } from 'svelte';
  import { Apple, Github, Monitor, ArrowRight } from 'lucide-svelte';

  type Platform = 'windows' | 'macos' | 'linux';
  let detected = $state<Platform | null>(null);

  const HOARD_VERSION = __HOARD_VERSION__;
  const RELEASE_DATE = __HOARD_RELEASE_DATE__;
  const RELEASE_BASE = `https://github.com/rleeon/hoard/releases/tag/v${HOARD_VERSION}`;
  const ALL_RELEASES = 'https://github.com/rleeon/hoard/releases';
  const CHANGELOG_URL = 'https://github.com/rleeon/hoard/blob/main/CHANGELOG.md';

  type Asset = {
    label: string;
    sublabel: string;
    size: string;
    href: string;
  };

  const downloads: Record<Platform, { name: string; assets: Asset[] }> = {
    windows: {
      name: 'Windows',
      assets: [
        {
          label: 'Hoard-Setup.msi',
          sublabel: 'Windows 10/11 · x64',
          size: '14.2 MB',
          href: RELEASE_BASE
        }
      ]
    },
    macos: {
      name: 'macOS',
      assets: [
        {
          label: 'Hoard.dmg',
          sublabel: 'macOS 12+ · Apple Silicon / Intel',
          size: '16.8 MB',
          href: RELEASE_BASE
        }
      ]
    },
    linux: {
      name: 'Linux',
      assets: [
        {
          label: 'hoard.deb',
          sublabel: 'Debian / Ubuntu · x64',
          size: '12.4 MB',
          href: RELEASE_BASE
        },
        {
          label: 'hoard.AppImage',
          sublabel: 'Universal · x64',
          size: '18.1 MB',
          href: RELEASE_BASE
        }
      ]
    }
  };

  onMount(() => {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) detected = 'windows';
    else if (ua.includes('mac')) detected = 'macos';
    else if (ua.includes('linux')) detected = 'linux';
  });

  const order: Platform[] = ['windows', 'macos', 'linux'];
</script>

<svelte:head>
  <title>{`${$_('download.title')} — Hoard`}</title>
  <link rel="canonical" href="https://hoard.services/download" />
</svelte:head>

<section class="relative mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-24">
  <div
    class="pointer-events-none absolute -top-24 left-1/2 -z-10 h-80 w-[40rem] -translate-x-1/2 rounded-full bg-emerald-500/[0.10] blur-3xl"
  ></div>

  <div class="text-center">
    <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
      {$_('nav.download')}
    </div>
    <h1 class="mt-3 text-balance text-4xl font-extrabold tracking-tight text-white sm:text-6xl">
      {$_('download.title')}
    </h1>
    <p class="mx-auto mt-5 max-w-xl text-pretty text-lg text-zinc-400">
      {$_('download.subtitle')}
    </p>
    <p class="mt-3 font-mono text-xs text-zinc-500">
      {$_('download.version', { values: { v: HOARD_VERSION, date: RELEASE_DATE } })}
    </p>
  </div>

  {#if detected}
    <div class="reveal mt-10 flex justify-center" use:reveal>
      <a
        href={downloads[detected].assets[0].href}
        target="_blank"
        rel="noreferrer"
        class="group relative inline-flex items-center gap-4 overflow-hidden rounded-2xl border border-emerald-500/30 bg-gradient-to-br from-emerald-600/15 to-emerald-500/[0.04] px-7 py-5 text-left ring-focus transition-colors hover:border-emerald-400/55 hover:bg-emerald-500/10"
      >
        <span
          class="grid h-14 w-14 place-items-center rounded-xl bg-emerald-500/15 text-emerald-300 ring-1 ring-inset ring-emerald-400/30"
        >
          {#if detected === 'macos'}
            <Apple class="h-7 w-7" />
          {:else}
            <Monitor class="h-7 w-7" />
          {/if}
        </span>
        <span class="flex flex-col">
          <span class="text-xs uppercase tracking-wider text-emerald-300/80">
            {$_('download.detected')}
          </span>
          <span class="text-xl font-semibold text-white">
            {$_('download.cta_for', { values: { platform: downloads[detected].name } })}
          </span>
          <span class="font-mono text-xs text-zinc-400">
            {downloads[detected].assets[0].sublabel} · {downloads[detected].assets[0].size}
          </span>
        </span>
        <ArrowRight
          class="h-5 w-5 flex-none text-emerald-300 transition-transform group-hover:translate-x-1"
        />
        <span
          aria-hidden="true"
          class="pointer-events-none absolute inset-y-0 left-0 w-1/2 -translate-x-full bg-gradient-to-r from-transparent via-white/15 to-transparent transition-transform duration-[1000ms] ease-out group-hover:translate-x-[220%]"
        ></span>
      </a>
    </div>
  {/if}

  <div class="mt-14 grid gap-5 sm:grid-cols-3">
    {#each order as p, i (p)}
      <article
        class="reveal spotlight card-edge group flex flex-col rounded-2xl border border-white/[0.06] bg-white/[0.025] p-6 transition-colors duration-500 hover:border-emerald-500/30 hover:bg-white/[0.045]"
        use:reveal={{ delay: i * 70 }}
        use:spotlight
      >
        <div class="flex items-center gap-3">
          <div
            class="grid h-10 w-10 place-items-center rounded-xl bg-gradient-to-br from-emerald-500/15 to-emerald-500/[0.04] text-emerald-300 ring-1 ring-inset ring-emerald-400/20 transition-transform duration-500 group-hover:scale-110"
          >
            {#if p === 'macos'}
              <Apple class="h-5 w-5" />
            {:else}
              <Monitor class="h-5 w-5" />
            {/if}
          </div>
          <h2 class="text-lg font-semibold text-white">{downloads[p].name}</h2>
        </div>

        <ul class="mt-5 space-y-2.5">
          {#each downloads[p].assets as a (a.label)}
            <li>
              <a
                href={a.href}
                target="_blank"
                rel="noreferrer"
                class="group/asset block rounded-lg border border-white/[0.06] bg-zinc-900/40 px-4 py-3 transition-colors hover:border-emerald-500/30 hover:bg-emerald-500/[0.06]"
              >
                <div class="flex items-center justify-between gap-2">
                  <span class="font-mono text-sm font-medium text-zinc-100">{a.label}</span>
                  <span class="font-mono text-[10px] tabular-nums text-zinc-500">{a.size}</span>
                </div>
                <span class="mt-0.5 block text-xs text-zinc-500">{a.sublabel}</span>
              </a>
            </li>
          {/each}
        </ul>
      </article>
    {/each}
  </div>

  <!-- changelog highlight -->
  <div
    class="reveal mt-14 overflow-hidden rounded-2xl border border-white/[0.06] bg-gradient-to-br from-emerald-950/30 via-zinc-950 to-zinc-950 card-edge"
    use:reveal
  >
    <div class="grid items-center gap-6 p-8 sm:grid-cols-[1fr_auto]">
      <div>
        <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
          <span class="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse-glow"></span>
          v{HOARD_VERSION}
        </div>
        <h3 class="mt-2 text-xl font-bold text-white">{$_('download.changelog_title')}</h3>
        <p class="mt-2 text-sm text-zinc-400">{$_('download.changelog_body')}</p>
      </div>
      <Button href={CHANGELOG_URL} target="_blank" variant="secondary" size="lg">
        <Github class="h-4 w-4" />
        {$_('download.changelog_cta')}
      </Button>
    </div>
  </div>

  <div
    class="reveal mt-6 flex flex-col items-center gap-4 rounded-2xl border border-white/[0.06] bg-white/[0.02] p-7 text-center sm:flex-row sm:justify-between sm:text-left card-edge"
    use:reveal
  >
    <div>
      <h3 class="text-lg font-semibold text-white">{$_('download.all_releases_title')}</h3>
      <p class="mt-1.5 text-sm text-zinc-400">{$_('download.all_releases_body')}</p>
    </div>
    <Button href={ALL_RELEASES} target="_blank" variant="secondary" size="lg">
      <Github class="h-4 w-4" />
      {$_('download.all_releases_cta')}
    </Button>
  </div>

  <p class="mt-10 text-center text-xs text-zinc-500">
    {$_('download.selfhost_note')}
    <a
      href="https://github.com/rleeon/hoard#self-host"
      class="link-underline text-emerald-400 hover:text-emerald-300"
    >
      {$_('hero.cta_selfhost')}
    </a>.
  </p>
</section>
