<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import {
    ArrowRight,
    GitBranch,
    Shield,
    Search,
    MonitorSmartphone,
    Download,
    ServerCog
  } from 'lucide-svelte';

  const features = [
    { key: 'versioned', icon: GitBranch },
    { key: 'verified', icon: Shield },
    { key: 'detect', icon: Search },
    { key: 'crossplat', icon: MonitorSmartphone },
    { key: 'export', icon: Download },
    { key: 'selfhost', icon: ServerCog }
  ];
</script>

<section class="relative overflow-hidden">
  <div class="grid-bg absolute inset-0 -z-10"></div>
  <div class="pointer-events-none absolute -top-32 left-1/2 -z-10 h-96 w-[60rem] -translate-x-1/2 rounded-full bg-emerald-500/10 blur-3xl"></div>

  <div class="mx-auto max-w-6xl px-4 pb-24 pt-20 sm:px-6 sm:pt-28">
    <div class="flex flex-col items-center text-center">
      <StatusDot />

      <h1
        class="mt-6 max-w-3xl text-balance text-5xl font-bold leading-[1.05] tracking-tight text-white sm:text-6xl animate-fade-up"
      >
        {$_('hero.title_1')}
        <span
          class="bg-gradient-to-r from-emerald-300 via-emerald-400 to-teal-300 bg-clip-text text-transparent"
        >
          {$_('hero.title_2')}
        </span>
      </h1>

      <p
        class="mt-6 max-w-2xl text-pretty text-lg text-zinc-400 animate-fade-up"
        style="animation-delay:0.1s"
      >
        {$_('hero.subtitle')}
      </p>

      <div
        class="mt-9 flex flex-col items-center gap-3 sm:flex-row animate-fade-up"
        style="animation-delay:0.2s"
      >
        <Button href="/login" size="lg" variant="primary">
          {$_('hero.cta_start')}
          <ArrowRight class="h-4 w-4" />
        </Button>
        <Button
          href="https://github.com/rleeon/hoard#set-up-the-server"
          target="_blank"
          size="lg"
          variant="ghost"
        >
          {$_('hero.cta_selfhost')}
        </Button>
      </div>

      <div
        class="mt-16 w-full max-w-4xl animate-fade-up"
        style="animation-delay:0.3s"
      >
        <div
          class="relative overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-900/60 p-1 shadow-2xl shadow-emerald-950/20"
        >
          <div class="rounded-xl bg-zinc-950 p-6 text-left font-mono text-sm">
            <div class="mb-3 flex gap-1.5">
              <span class="h-3 w-3 rounded-full bg-red-500/70"></span>
              <span class="h-3 w-3 rounded-full bg-amber-500/70"></span>
              <span class="h-3 w-3 rounded-full bg-emerald-500/70"></span>
            </div>
            <div class="space-y-1.5 leading-relaxed">
              <p>
                <span class="text-zinc-500">$</span>
                <span class="text-zinc-300">hoard backup stardew-valley</span>
              </p>
              <p class="text-emerald-400">→ detected: ~/.config/StardewValley/Saves</p>
              <p class="text-emerald-400">→ snapshot v47 · 8.2 MB · SHA-256 verified</p>
              <p class="text-zinc-400">→ uploaded to api.hoard.services in 1.4s</p>
              <p class="mt-3">
                <span class="text-zinc-500">$</span>
                <span class="text-zinc-300">hoard restore stardew-valley --version 43</span>
              </p>
              <p class="text-emerald-400">→ restored "before-final-boss" · 12 days ago</p>
              <p class="inline-flex items-center text-zinc-200">
                <span class="ml-0 inline-block h-4 w-2 animate-pulse bg-emerald-400"></span>
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

<section class="border-t border-zinc-900 bg-zinc-950/40">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
    <div class="text-center">
      <h2 class="text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {$_('features.title')}
      </h2>
    </div>

    <div class="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each features as f (f.key)}
        <div
          class="group rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-6 transition-all hover:-translate-y-1 hover:border-emerald-700/60 hover:bg-zinc-900/80"
        >
          <div
            class="mb-4 grid h-11 w-11 place-items-center rounded-xl bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/20 transition-transform group-hover:scale-110"
          >
            <f.icon class="h-5 w-5" />
          </div>
          <h3 class="font-semibold text-white">{$_(`features.${f.key}.title`)}</h3>
          <p class="mt-1.5 text-sm leading-relaxed text-zinc-400">
            {$_(`features.${f.key}.body`)}
          </p>
        </div>
      {/each}
    </div>
  </div>
</section>

<section class="border-t border-zinc-900">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
    <div
      class="relative overflow-hidden rounded-3xl border border-emerald-800/40 bg-gradient-to-br from-emerald-950/60 via-zinc-950 to-zinc-950 p-10 sm:p-14"
    >
      <div
        class="pointer-events-none absolute -right-20 -top-20 h-80 w-80 rounded-full bg-emerald-500/15 blur-3xl"
      ></div>
      <div class="relative max-w-2xl">
        <h2 class="text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
          Free forever for casual play.
        </h2>
        <p class="mt-3 text-zinc-400">
          500 MB, 1 device, no card required. Pro starts at 3,99 €/mo whenever your library outgrows it.
        </p>
        <div class="mt-6 flex flex-wrap gap-3">
          <Button href="/login" size="lg" variant="primary">
            {$_('hero.cta_start')}
            <ArrowRight class="h-4 w-4" />
          </Button>
          <Button href="/pricing" size="lg" variant="secondary">{$_('nav.pricing')}</Button>
        </div>
      </div>
    </div>
  </div>
</section>
