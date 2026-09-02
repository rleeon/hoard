<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { tilt } from '$lib/actions/tilt';
  import { localeHref } from '$lib/i18n/href';
  import { onMount } from 'svelte';
  import { Apple, Github, Monitor, Terminal, ArrowRight, Server, Cpu, Copy, Check } from 'lucide-svelte';
  import { release, ALL_RELEASES } from '$lib/version';

  type Platform = 'windows' | 'macos' | 'linux';
  let detected = $state<Platform | null>(null);

  // One-liners point at the raw repo copies (web/static/install.*) so they
  // still work when hoard.services itself is unreachable; the "read before
  // running" links below point at the served copies (/install.sh, /install.ps1).
  const SH_CMD = 'curl -fsSL https://raw.githubusercontent.com/rleeon/hoard/main/web/static/install.sh | sh';
  const PS_CMD = 'irm https://raw.githubusercontent.com/rleeon/hoard/main/web/static/install.ps1 | iex';

  // Two install rows; the one matching the visitor's OS gets highlighted.
  let installRows = $derived([
    { id: 'sh', os: 'Linux · macOS', cmd: SH_CMD, url: '/install.sh', match: detected !== 'windows' },
    { id: 'ps', os: 'Windows · PowerShell', cmd: PS_CMD, url: '/install.ps1', match: detected === 'windows' }
  ]);

  const steps = ['cli.does_1', 'cli.does_2', 'cli.does_3', 'cli.does_4'];

  const commands = [
    { cmd: 'hoard login', key: 'cli.cmd_login' },
    { cmd: 'hoard sync start', key: 'cli.cmd_sync' },
    { cmd: 'hoard sync', key: 'cli.cmd_syncstatus' },
    { cmd: 'hoard track <game>', key: 'cli.cmd_track' },
    { cmd: 'hoard snapshots list <id>', key: 'cli.cmd_snapshots' },
    { cmd: 'hoard restore <id>', key: 'cli.cmd_restore' }
  ];

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = id;
      setTimeout(() => {
        if (copied === id) copied = null;
      }, 2000);
    } catch {
      /* clipboard blocked, the command is still visible to select manually */
    }
  }

  type Asset = { label: string; sublabel: string; href: string };
  const fileName = (url: string) => url.split('/').pop() ?? url;

  let downloads = $derived<Record<Platform, { name: string; assets: Asset[] }>>({
    windows: {
      name: 'Windows',
      assets: [
        { label: fileName($release.assets.cliWindowsX64), sublabel: 'x64', href: $release.assets.cliWindowsX64 },
        { label: fileName($release.assets.cliWindowsArm64), sublabel: 'ARM64', href: $release.assets.cliWindowsArm64 }
      ]
    },
    macos: {
      name: 'macOS',
      assets: [
        { label: fileName($release.assets.cliMacosArm64), sublabel: 'Apple Silicon', href: $release.assets.cliMacosArm64 }
      ]
    },
    linux: {
      name: 'Linux',
      assets: [
        { label: fileName($release.assets.cliLinuxX64), sublabel: 'x64', href: $release.assets.cliLinuxX64 },
        { label: fileName($release.assets.cliLinuxArm64), sublabel: 'ARM64 · RPi / ARM server', href: $release.assets.cliLinuxArm64 }
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

<Seo path="/cli" key="cli" />

<section class="mx-auto max-w-5xl px-4 py-16 sm:px-6 sm:py-24">
  <!-- Hero: the install command, front and centre -->
  <div class="mx-auto max-w-2xl text-center">
    <p class="kicker flex items-center justify-center gap-1.5">
      <Terminal class="h-3.5 w-3.5" />
      Hoard CLI
    </p>
    <h1 class="mt-3 text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('cli.title')}
    </h1>
    <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
      {$_('cli.subtitle')}
    </p>
  </div>

  <figure
    class="reveal tilt relative mx-auto mt-10 max-w-2xl rounded-2xl"
    use:reveal
    use:tilt={{ max: 5 }}
  >
    <img
      src="/CLI.png"
      alt={$_('hero.screenshot_cli_alt')}
      width="664"
      height="630"
      decoding="async"
      class="block w-full rounded-2xl border border-line-strong shadow-[0_40px_80px_-40px_rgba(0,0,0,0.9)]"
    />
  </figure>

  <div class="reveal mx-auto mt-12 max-w-2xl" use:reveal>
    <p class="text-center text-sm font-medium text-ink">{$_('cli.run_hint')}</p>
    <div class="mt-4 space-y-3">
      {#each installRows as row (row.id)}
        <div
          class="rounded-2xl border bg-surface p-4 transition-colors {row.match
            ? 'border-accent'
            : 'border-line'}"
        >
          <div class="flex items-center justify-between gap-3">
            <p class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-ink-soft">
              <Terminal class="h-3.5 w-3.5 text-accent" />
              {row.os}
            </p>
            <a
              href={row.url}
              target="_blank"
              rel="noopener noreferrer"
              class="font-mono text-[11px] text-ink-faint underline-offset-2 ring-focus hover:text-accent hover:underline"
            >
              {row.url}
            </a>
          </div>
          <div class="mt-2.5 flex items-stretch gap-2">
            <code
              class="flex-1 overflow-x-auto whitespace-nowrap rounded-lg bg-pine px-4 py-3 font-mono text-[13px] text-white/90"
              >{row.cmd}</code
            >
            <button
              onclick={() => copy(row.cmd, row.id)}
              class="grid w-11 flex-none place-items-center rounded-lg border border-line bg-bg text-ink-soft ring-focus transition-colors hover:border-accent hover:text-accent"
              aria-label={$_('cli.copy')}
            >
              {#if copied === row.id}
                <Check class="h-4 w-4 text-accent" />
              {:else}
                <Copy class="h-4 w-4" />
              {/if}
            </button>
          </div>
        </div>
      {/each}
    </div>
    <p class="mt-3 text-center font-mono text-xs text-ink-faint">
      {$_('download.version', { values: { v: $release.v, date: $release.date } })}
    </p>
  </div>

  <!-- What that command does -->
  <div class="reveal mt-16" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('cli.does_title')}</h2>
    <ol class="mx-auto mt-8 max-w-2xl space-y-3">
      {#each steps as step, i (step)}
        <li class="flex items-start gap-4 rounded-2xl border border-line bg-surface p-5">
          <span
            class="grid h-8 w-8 flex-none place-items-center rounded-full bg-accent-tint font-mono text-sm font-semibold text-accent"
          >
            {i + 1}
          </span>
          <p class="pt-1 text-sm leading-relaxed text-ink-soft">{$_(step)}</p>
        </li>
      {/each}
    </ol>
    <p class="mx-auto mt-4 max-w-2xl text-center text-xs text-ink-faint">{$_('cli.does_note')}</p>
  </div>

  <!-- What is Hoard CLI -->
  <div class="reveal mt-16" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('cli.about_title')}</h2>
    <div class="mt-8 grid gap-4 sm:grid-cols-3">
      <article class="rounded-2xl border border-line bg-surface p-6">
        <div class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent">
          <Terminal class="h-5 w-5" />
        </div>
        <h3 class="mt-4 text-base font-semibold text-ink">{$_('cli.what_title')}</h3>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{$_('cli.what_body')}</p>
      </article>
      <article class="rounded-2xl border border-line bg-surface p-6">
        <div class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent">
          <Server class="h-5 w-5" />
        </div>
        <h3 class="mt-4 text-base font-semibold text-ink">{$_('cli.who_title')}</h3>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{$_('cli.who_body')}</p>
      </article>
      <article class="rounded-2xl border border-line bg-surface p-6">
        <div class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent">
          <Cpu class="h-5 w-5" />
        </div>
        <h3 class="mt-4 text-base font-semibold text-ink">{$_('cli.engine_title')}</h3>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{$_('cli.engine_body')}</p>
      </article>
    </div>
  </div>

  <!-- The commands you'll use -->
  <div class="reveal mt-16" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('cli.commands_title')}</h2>
    <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">{$_('cli.commands_body')}</p>
    <ul class="mx-auto mt-8 max-w-2xl divide-y divide-line overflow-hidden rounded-2xl border border-line bg-surface">
      {#each commands as c (c.cmd)}
        <li class="flex flex-col gap-1.5 p-4 sm:flex-row sm:items-center sm:gap-4">
          <code
            class="w-fit flex-none rounded-md bg-pine px-2.5 py-1 font-mono text-[13px] text-white/90 sm:w-64"
            >{c.cmd}</code
          >
          <span class="text-sm leading-relaxed text-ink-soft">{$_(c.key)}</span>
        </li>
      {/each}
    </ul>
  </div>

  <!-- Manual download (fallback) -->
  <h2 class="reveal mt-16 text-center text-2xl font-semibold text-ink" use:reveal>
    {$_('cli.manual_title')}
  </h2>
  <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">{$_('cli.manual_body')}</p>

  <div class="mt-8 grid gap-5 sm:grid-cols-3">
    {#each order as p, i (p)}
      <article
        class="reveal flex flex-col rounded-2xl border bg-surface p-6 transition-colors {detected === p
          ? 'border-accent'
          : 'border-line hover:border-line-strong'}"
        use:reveal={{ delay: i * 70 }}
      >
        <div class="flex items-center gap-3">
          <div class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent">
            {#if p === 'macos'}
              <Apple class="h-5 w-5" />
            {:else}
              <Monitor class="h-5 w-5" />
            {/if}
          </div>
          <h3 class="text-lg font-semibold text-ink">{downloads[p].name}</h3>
          {#if detected === p}
            <span
              class="ml-auto rounded-full bg-accent-tint px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-accent"
            >
              {$_('download.detected')}
            </span>
          {/if}
        </div>

        <ul class="mt-5 space-y-2.5">
          {#each downloads[p].assets as a (a.label)}
            <li>
              <a
                href={a.href}
                class="block rounded-lg border border-line bg-bg px-4 py-3 ring-focus transition-colors hover:border-accent hover:bg-accent-tint"
              >
                <span class="block break-all font-mono text-sm font-medium text-ink">{a.label}</span>
                <span class="mt-0.5 block text-xs text-ink-faint">{a.sublabel}</span>
              </a>
            </li>
          {/each}
        </ul>

        {#if p === 'linux'}
          <p class="mt-4 flex items-start gap-2 text-xs leading-relaxed text-ink-faint">
            <Server class="mt-0.5 h-3.5 w-3.5 flex-none text-accent" />
            {$_('cli.linux_note')}
          </p>
        {/if}
      </article>
    {/each}
  </div>

  <div class="reveal mt-8 grid gap-4 sm:grid-cols-2" use:reveal>
    <div class="rounded-2xl border border-line bg-surface p-6">
      <p class="flex items-center gap-2 text-sm font-semibold text-ink">
        <Monitor class="h-4 w-4 text-accent" />
        Linux · macOS
      </p>
      <pre class="mt-3 overflow-x-auto rounded-lg bg-pine p-4 font-mono text-[13px] leading-relaxed text-white/90"><code>tar -xzf hoard-*-*.tar.gz
mv hoard-*/hoard ~/.local/bin/
hoard login</code></pre>
    </div>
    <div class="rounded-2xl border border-line bg-surface p-6">
      <p class="flex items-center gap-2 text-sm font-semibold text-ink">
        <Monitor class="h-4 w-4 text-accent" />
        Windows · PowerShell
      </p>
      <pre class="mt-3 overflow-x-auto rounded-lg bg-pine p-4 font-mono text-[13px] leading-relaxed text-white/90"><code>tar -xzf hoard-*-windows-x86_64.tar.gz
# move hoard.exe onto your PATH, then:
.\hoard.exe login</code></pre>
    </div>
  </div>
  <p class="mt-4 text-center text-xs text-ink-faint">{$_('cli.install_hint')}</p>

  <!-- GUI hint -->
  <div
    class="reveal mt-16 flex flex-col items-center gap-4 rounded-2xl border border-line bg-surface p-7 text-center sm:flex-row sm:justify-between sm:text-left"
    use:reveal
  >
    <div>
      <h3 class="text-lg font-semibold text-ink">{$_('cli.gui_title')}</h3>
      <p class="mt-1.5 text-sm text-ink-soft">{$_('cli.gui_body')}</p>
    </div>
    <Button href={$localeHref('/download')} size="lg">
      {$_('cli.gui_cta')}
      <ArrowRight class="h-4 w-4" />
    </Button>
  </div>

  <!-- Self-host -->
  <div
    class="reveal mt-6 flex flex-col items-center gap-4 rounded-2xl border border-pine-line bg-pine p-7 text-center sm:flex-row sm:justify-between sm:text-left"
    use:reveal
  >
    <div>
      <h3 class="text-lg font-semibold text-white">{$_('cli.selfhost_title')}</h3>
      <p class="mt-1.5 text-sm text-white/60">{$_('cli.selfhost_body')}</p>
    </div>
    <Button
      href="https://github.com/rleeon/hoard/blob/main/SELF-HOST_GUIDE.md"
      target="_blank"
      variant="secondary"
      size="lg"
    >
      <Github class="h-4 w-4" />
      {$_('cli.selfhost_cta')}
    </Button>
  </div>

  <p class="mt-10 text-center text-xs text-ink-faint">
    {$_('cli.all_releases_note')}
    <a
      href={ALL_RELEASES}
      target="_blank"
      rel="noopener noreferrer"
      class="link-underline text-accent ring-focus hover:text-emerald-300"
    >
      GitHub Releases
    </a>.
  </p>
</section>
