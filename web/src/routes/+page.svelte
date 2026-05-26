<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import HeroTerminal from '$lib/components/HeroTerminal.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { spotlight } from '$lib/actions/spotlight';
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

<svelte:head>
  <title>Hoard — versioned cloud sync for game saves</title>
  <link rel="canonical" href="https://hoard.services/" />
</svelte:head>

<section class="relative overflow-hidden">
  <div class="grid-bg pointer-events-none absolute inset-0 -z-10"></div>
  <div
    class="pointer-events-none absolute -top-40 left-1/2 -z-10 h-[36rem] w-[60rem] -translate-x-1/2 rounded-full bg-emerald-500/[0.14] blur-3xl animate-drift"
  ></div>

  <div class="mx-auto max-w-6xl px-4 pb-24 pt-16 sm:px-6 sm:pt-24">
    <div class="flex flex-col items-center text-center">
      <StatusDot />

      <h1
        class="mt-6 max-w-3xl text-balance text-[2.6rem] font-bold leading-[1.05] tracking-[-0.02em] text-white sm:text-[3.75rem] animate-fade-up"
      >
        {$_('hero.title_1')}
        <span class="relative inline-block">
          <span
            class="bg-gradient-to-r from-emerald-300 via-emerald-400 to-teal-300 bg-clip-text text-transparent"
          >
            {$_('hero.title_2')}
          </span>
          <span
            aria-hidden="true"
            class="pointer-events-none absolute -inset-x-2 -bottom-1 h-2 rounded-full bg-emerald-500/20 blur-md"
          ></span>
        </span>
      </h1>

      <p
        class="mt-5 max-w-2xl text-pretty text-[1.05rem] leading-relaxed text-zinc-400 sm:text-lg animate-fade-up"
        style="animation-delay:0.1s"
      >
        {$_('hero.subtitle')}
      </p>

      <div
        class="mt-9 flex flex-col items-center gap-3 sm:flex-row animate-fade-up"
        style="animation-delay:0.2s"
      >
        <Button href="/pricing" size="lg" variant="primary">
          {$_('hero.cta_start')}
          <ArrowRight class="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
        </Button>
        <Button
          href="https://github.com/rleeon/hoard#self-host"
          target="_blank"
          size="lg"
          variant="ghost"
        >
          {$_('hero.cta_selfhost')}
        </Button>
      </div>

      <div
        class="mt-14 w-full max-w-4xl animate-fade-up"
        style="animation-delay:0.3s"
      >
        <HeroTerminal />
      </div>
    </div>
  </div>
</section>

<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal text-center" use:reveal>
      <h2 class="text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {$_('features.title')}
      </h2>
    </div>

    <div class="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each features as f, i (f.key)}
        <article
          class="reveal spotlight group rounded-2xl border border-white/[0.06] bg-white/[0.025] p-6 transition-all duration-500 hover:-translate-y-1 hover:border-emerald-500/30 hover:bg-white/[0.045]"
          use:reveal={{ delay: i * 70 }}
          use:spotlight
        >
          <div
            class="mb-4 grid h-11 w-11 place-items-center rounded-xl bg-gradient-to-br from-emerald-500/15 to-emerald-500/[0.04] text-emerald-300 ring-1 ring-inset ring-emerald-400/20 transition-transform duration-500 group-hover:scale-110 group-hover:rotate-[-4deg]"
          >
            <f.icon class="h-5 w-5" />
          </div>
          <h3 class="font-semibold text-white">{$_(`features.${f.key}.title`)}</h3>
          <p class="mt-1.5 text-sm leading-relaxed text-zinc-400">
            {$_(`features.${f.key}.body`)}
          </p>
        </article>
      {/each}
    </div>
  </div>
</section>

<section class="border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
    <div
      class="reveal relative overflow-hidden rounded-3xl border border-emerald-500/15 bg-gradient-to-br from-emerald-950/40 via-zinc-950 to-zinc-950 p-10 sm:p-14"
      use:reveal
    >
      <div
        class="pointer-events-none absolute -right-24 -top-24 h-80 w-80 rounded-full bg-emerald-500/20 blur-3xl animate-drift"
      ></div>
      <div
        class="pointer-events-none absolute inset-0 bg-[radial-gradient(60%_60%_at_50%_0%,rgba(16,185,129,0.10),transparent_70%)]"
      ></div>
      <div class="relative max-w-2xl">
        <h2 class="text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
          {$_('cta_section.title')}
        </h2>
        <p class="mt-3 text-zinc-400">{$_('cta_section.body')}</p>
        <div class="mt-7 flex flex-wrap gap-3">
          <Button href="/download" size="lg" variant="primary">
            {$_('cta_section.cta')}
            <ArrowRight class="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
          </Button>
        </div>
      </div>
    </div>
  </div>
</section>
