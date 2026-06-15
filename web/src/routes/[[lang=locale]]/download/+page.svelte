<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { onMount } from 'svelte';
  import { Apple, Github, Monitor, ArrowRight } from 'lucide-svelte';
  import { version, release, ALL_RELEASES, CHANGELOG_URL } from '$lib/version';

  type Platform = 'windows' | 'macos' | 'linux';
  let detected = $state<Platform | null>(null);

  type Asset = {
    label: string;
    sublabel: string;
    href: string;
  };

  const fileName = (url: string) => url.split('/').pop() ?? url;

  // Direct links to the installers of the latest release — clicking starts
  // the download. URLs come from the `release` store, so they follow GitHub
  // automatically when a new version ships.
  let downloads = $derived<Record<Platform, { name: string; assets: Asset[] }>>({
    windows: {
      name: 'Windows',
      assets: [
        {
          label: fileName($release.assets.windowsSetup),
          sublabel: 'Windows 10/11 · x64',
          href: $release.assets.windowsSetup
        },
        {
          label: fileName($release.assets.windowsMsi),
          sublabel: 'Windows 10/11 · x64 · MSI',
          href: $release.assets.windowsMsi
        }
      ]
    },
    macos: {
      name: 'macOS',
      assets: [
        {
          label: fileName($release.assets.macosDmg),
          sublabel: 'macOS 12+ · Apple Silicon',
          href: $release.assets.macosDmg
        }
      ]
    },
    linux: {
      name: 'Linux',
      assets: [
        {
          label: fileName($release.assets.linuxDeb),
          sublabel: 'Debian / Ubuntu · x64',
          href: $release.assets.linuxDeb
        },
        {
          label: fileName($release.assets.linuxAppImage),
          sublabel: 'Universal · x64',
          href: $release.assets.linuxAppImage
        },
        {
          label: fileName($release.assets.linuxRpm),
          sublabel: 'Fedora / openSUSE · x64',
          href: $release.assets.linuxRpm
        }
      ]
    }
  });

  onMount(() => {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) detected = 'windows';
    else if (ua.includes('mac')) detected = 'macos';
    else if (ua.includes('linux')) detected = 'linux';
  });

  const order: Platform[] = ['windows', 'macos', 'linux'];
</script>

<Seo path="/download" key="download" />

<section class="mx-auto max-w-5xl px-4 py-16 sm:px-6 sm:py-24">
  <div class="mx-auto max-w-2xl text-center">
    <p class="kicker justify-center">{$_('nav.download')}</p>
    <h1 class="mt-3 text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('download.title')}
    </h1>
    <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
      {$_('download.subtitle')}
    </p>
    <p class="mt-3 font-mono text-xs text-ink-faint">
      {$_('download.version', { values: { v: $release.v, date: $release.date } })}
    </p>
  </div>

  {#if detected}
    <div class="reveal mt-10 flex justify-center" use:reveal>
      <a
        href={downloads[detected].assets[0].href}
        class="group inline-flex items-center gap-4 rounded-2xl border border-accent bg-accent-tint px-7 py-5 text-left ring-focus transition-colors hover:bg-accent/15"
      >
        <span
          class="grid h-14 w-14 place-items-center rounded-xl bg-accent-deep text-white"
        >
          {#if detected === 'macos'}
            <Apple class="h-7 w-7" />
          {:else}
            <Monitor class="h-7 w-7" />
          {/if}
        </span>
        <span class="flex flex-col">
          <span class="font-mono text-[11px] uppercase tracking-wider text-accent">
            {$_('download.detected')}
          </span>
          <span class="text-xl font-semibold text-ink">
            {$_('download.cta_for', { values: { platform: downloads[detected].name } })}
          </span>
          <span class="font-mono text-xs text-ink-soft">
            {downloads[detected].assets[0].sublabel}
          </span>
        </span>
        <ArrowRight
          class="h-5 w-5 flex-none text-accent transition-transform group-hover:translate-x-1"
        />
      </a>
    </div>
  {/if}

  <div class="mt-14 grid gap-5 sm:grid-cols-3">
    {#each order as p, i (p)}
      <article
        class="reveal flex flex-col rounded-2xl border border-line bg-surface p-6 transition-colors hover:border-line-strong"
        use:reveal={{ delay: i * 70 }}
      >
        <div class="flex items-center gap-3">
          <div
            class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent"
          >
            {#if p === 'macos'}
              <Apple class="h-5 w-5" />
            {:else}
              <Monitor class="h-5 w-5" />
            {/if}
          </div>
          <h2 class="text-lg font-semibold text-ink">{downloads[p].name}</h2>
        </div>

        <ul class="mt-5 space-y-2.5">
          {#each downloads[p].assets as a (a.label)}
            <li>
              <a
                href={a.href}
                class="block rounded-lg border border-line bg-bg px-4 py-3 ring-focus transition-colors hover:border-accent hover:bg-accent-tint"
              >
                <span class="font-mono text-sm font-medium text-ink">{a.label}</span>
                <span class="mt-0.5 block text-xs text-ink-faint">{a.sublabel}</span>
              </a>
            </li>
          {/each}
        </ul>
      </article>
    {/each}
  </div>

  <!-- changelog -->
  <div
    class="reveal mt-14 overflow-hidden rounded-2xl border border-pine-line bg-pine"
    use:reveal
  >
    <div class="grid items-center gap-6 p-8 sm:grid-cols-[1fr_auto]">
      <div>
        <p class="kicker">v{$version}</p>
        <h3 class="mt-2 text-xl font-semibold text-white">{$_('download.changelog_title')}</h3>
        <p class="mt-2 text-sm text-white/60">{$_('download.changelog_body')}</p>
      </div>
      <a
        href={CHANGELOG_URL}
        target="_blank"
        rel="noopener noreferrer"
        class="inline-flex items-center justify-center gap-2 rounded-lg border border-pine-line bg-white/5 px-5 py-2.5 text-sm font-medium text-white ring-focus transition-colors hover:bg-white/10"
      >
        <Github class="h-4 w-4" />
        {$_('download.changelog_cta')}
      </a>
    </div>
  </div>

  <div
    class="reveal mt-6 flex flex-col items-center gap-4 rounded-2xl border border-line bg-surface p-7 text-center sm:flex-row sm:justify-between sm:text-left"
    use:reveal
  >
    <div>
      <h3 class="text-lg font-semibold text-ink">{$_('download.all_releases_title')}</h3>
      <p class="mt-1.5 text-sm text-ink-soft">{$_('download.all_releases_body')}</p>
    </div>
    <Button href={ALL_RELEASES} target="_blank" variant="secondary" size="lg">
      <Github class="h-4 w-4" />
      {$_('download.all_releases_cta')}
    </Button>
  </div>

  <p class="mt-10 text-center text-xs text-ink-faint">
    {$_('download.selfhost_note')}
    <a
      href="https://github.com/rleeon/hoard#self-host"
      class="link-underline text-accent ring-focus hover:text-emerald-300"
    >
      GitHub
    </a>.
  </p>
</section>
