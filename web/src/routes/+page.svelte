<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import HeroTerminal from '$lib/components/HeroTerminal.svelte';
  import MetricStrip from '$lib/components/MetricStrip.svelte';
  import PlatformMarquee from '$lib/components/PlatformMarquee.svelte';
  import HowItWorks from '$lib/components/HowItWorks.svelte';
  import SecurityStrip from '$lib/components/SecurityStrip.svelte';
  import ProductMockup from '$lib/components/ProductMockup.svelte';
  import DownloadCTA from '$lib/components/DownloadCTA.svelte';
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
    { key: 'versioned', icon: GitBranch, tint: 'emerald' },
    { key: 'verified', icon: Shield, tint: 'teal' },
    { key: 'detect', icon: Search, tint: 'emerald' },
    { key: 'crossplat', icon: MonitorSmartphone, tint: 'emerald' },
    { key: 'export', icon: Download, tint: 'teal' },
    { key: 'selfhost', icon: ServerCog, tint: 'emerald' }
  ];
</script>

<svelte:head>
  <title>Hoard — versioned cloud sync for game saves</title>
  <link rel="canonical" href="https://hoard.services/" />
</svelte:head>

<!-- ───────── HERO ───────── -->
<section class="relative overflow-hidden">
  <div class="grid-bg pointer-events-none absolute inset-0 -z-10"></div>

  <div class="mx-auto max-w-6xl px-4 pb-20 pt-14 sm:px-6 sm:pt-20">
    <div class="flex flex-col items-center text-center">
      <StatusDot />

      <div class="mt-5 inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.025] px-3 py-1 text-[11px] font-medium uppercase tracking-[0.14em] text-zinc-400 backdrop-blur animate-fade-up">
        <span class="h-1 w-1 rounded-full bg-emerald-400"></span>
        {$_('hero.eyebrow')}
      </div>

      <h1
        class="mt-5 max-w-4xl text-balance text-[2.1rem] font-extrabold leading-[1.04] tracking-[-0.03em] text-white sm:text-[3.4rem] lg:text-[4.25rem] animate-fade-up"
      >
        {$_('hero.title_1')}
        <span class="relative inline-block">
          <span class="hue-pan font-extrabold">
            {$_('hero.title_2')}
          </span>
        </span>
      </h1>

      <p
        class="mt-6 max-w-3xl text-pretty text-[1.02rem] leading-relaxed text-zinc-400 sm:text-lg animate-fade-up"
        style="animation-delay:0.1s"
      >
        {$_('hero.subtitle')}
      </p>

      <div
        class="mt-9 flex flex-col items-center gap-4 sm:flex-row animate-fade-up"
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

      <div
        class="mt-10 w-full animate-fade-up"
        style="animation-delay:0.4s"
      >
        <MetricStrip />
      </div>
    </div>
  </div>
</section>

<!-- ───────── PLATFORM MARQUEE ───────── -->
<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-12 sm:px-6 sm:py-14">
    <PlatformMarquee />
  </div>
</section>

<!-- ───────── FEATURES ───────── -->
<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal mx-auto max-w-2xl text-center" use:reveal>
      <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
        Why Hoard
      </div>
      <h2 class="mt-3 text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {$_('features.title')}
      </h2>
      <p class="mt-3 text-pretty text-zinc-400">{$_('features.subtitle')}</p>
    </div>

    <div class="mt-14 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each features as f, i (f.key)}
        <article
          class="reveal spotlight card-edge group rounded-2xl border border-white/[0.06] bg-white/[0.025] p-6 transition-colors duration-500 hover:border-emerald-500/30 hover:bg-white/[0.045]"
          use:reveal={{ delay: i * 70 }}
          use:spotlight
        >
          <div
            class="mb-4 grid h-11 w-11 place-items-center rounded-xl bg-gradient-to-br {f.tint ===
            'teal'
              ? 'from-teal-500/15 to-teal-500/[0.04] text-teal-200 ring-teal-400/25'
              : 'from-emerald-500/15 to-emerald-500/[0.04] text-emerald-300 ring-emerald-400/20'} ring-1 ring-inset transition-transform duration-500 group-hover:scale-110 group-hover:rotate-[-4deg]"
          >
            <f.icon class="h-5 w-5" />
          </div>
          <h3 class="font-semibold text-white">{$_(`features.${f.key}.title`)}</h3>
          <p class="mt-1.5 text-sm leading-relaxed text-zinc-300">
            {$_(`features.${f.key}.body`)}
          </p>
        </article>
      {/each}
    </div>
  </div>
</section>

<!-- ───────── HOW IT WORKS ───────── -->
<HowItWorks />

<!-- ───────── PRODUCT MOCKUP ───────── -->
<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal mx-auto max-w-2xl text-center" use:reveal>
      <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
        A real desktop app
      </div>
      <h2 class="mt-3 text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        Looks like a tool. Acts like one.
      </h2>
      <p class="mt-3 text-pretty text-zinc-400">
        Tray icon, autostart, four tabs. No tutorials, no upsell modals, no toasts that don't say anything.
      </p>
    </div>

    <div class="reveal mx-auto mt-12 max-w-5xl" use:reveal={{ delay: 80 }}>
      <ProductMockup />
    </div>
  </div>
</section>

<!-- ───────── SECURITY / TRUST ───────── -->
<SecurityStrip />

<!-- ───────── FINAL CTA ───────── -->
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
        <div class="mt-7">
          <DownloadCTA />
        </div>
      </div>
    </div>
  </div>
</section>
