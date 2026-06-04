<script lang="ts">
  import { _ } from 'svelte-i18n';
  import PlanCard from '$lib/components/PlanCard.svelte';
  import { PLANS } from '$lib/plans';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { goto } from '$app/navigation';
  import { reveal } from '$lib/actions/reveal';
  import { Check, Minus, Github } from 'lucide-svelte';

  let cycle = $state<BillingCycle>('monthly');
  let monthlyBtn = $state<HTMLButtonElement | null>(null);
  let yearlyBtn = $state<HTMLButtonElement | null>(null);
  let trackEl = $state<HTMLDivElement | null>(null);

  function choose(plan: PlanId, c: BillingCycle) {
    if (plan === 'free') {
      goto('/download');
      return;
    }
    // Hand off to the confirmation flow. It checks for a session (bouncing to
    // login first if needed), confirms the account, then creates the Polar
    // checkout server-side. No checkout URL is built client-side anymore.
    goto(`/checkout?plan=${plan}&cycle=${c}`);
  }

  // Compute thumb geometry from real button rects so the slider lands
  // pixel-perfect on whichever tab is active. Uses an effect so the
  // measurement re-runs when refs land or `cycle` flips.
  let thumbStyle = $state('opacity:0;');

  function measure() {
    if (!trackEl || !monthlyBtn || !yearlyBtn) return;
    const trackRect = trackEl.getBoundingClientRect();
    const target = cycle === 'monthly' ? monthlyBtn : yearlyBtn;
    const targetRect = target.getBoundingClientRect();
    const left = targetRect.left - trackRect.left;
    const width = targetRect.width;
    thumbStyle = `left:${left}px; width:${width}px;`;
  }

  $effect(() => {
    // touch reactive deps
    void cycle;
    void trackEl;
    void monthlyBtn;
    void yearlyBtn;
    measure();
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    const onResize = () => measure();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  type Row = { label: string; free: string; pro: string };
  const compareRows: Row[] = $derived([
    {
      label: $_('pricing.compare_storage'),
      free: '1 GB',
      pro: '50 GB'
    },
    {
      label: $_('pricing.compare_devices'),
      free: '3',
      pro: $_('pricing.compare_unlimited')
    },
    {
      label: $_('pricing.compare_saves'),
      free: $_('pricing.compare_unlimited'),
      pro: $_('pricing.compare_unlimited')
    },
    {
      label: $_('pricing.compare_history'),
      free: $_('pricing.compare_forever'),
      pro: $_('pricing.compare_forever')
    },
    {
      label: $_('pricing.compare_save_size'),
      free: '200 MB',
      pro: '2 GB'
    },
    {
      label: $_('pricing.compare_bandwidth'),
      free: '500 MB',
      pro: '1 GB'
    },
    {
      label: $_('pricing.compare_export'),
      free: $_('pricing.compare_yes'),
      pro: $_('pricing.compare_yes')
    },
    {
      label: $_('pricing.compare_support'),
      free: $_('pricing.compare_email_basic'),
      pro: $_('pricing.compare_email')
    },
    {
      label: $_('pricing.compare_selfhost'),
      free: $_('pricing.compare_anytime'),
      pro: $_('pricing.compare_anytime')
    }
  ]);
</script>

<svelte:head>
  <title>{`${$_('pricing.title')} — Hoard`}</title>
  <link rel="canonical" href="https://hoard.services/pricing" />
</svelte:head>

<section class="relative mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
  <div
    class="pointer-events-none absolute -top-24 left-1/2 -z-10 h-80 w-[40rem] -translate-x-1/2 rounded-full bg-emerald-500/[0.10] blur-3xl"
  ></div>

  <div class="text-center">
    <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
      Pricing
    </div>
    <h1 class="mt-3 text-balance text-4xl font-extrabold tracking-tight text-white sm:text-6xl">
      {$_('pricing.title')}
    </h1>
    <p class="mx-auto mt-5 max-w-xl text-pretty text-lg text-zinc-400">
      {$_('pricing.subtitle')}
    </p>
  </div>

  <div class="mt-10 flex flex-col items-center gap-3">
    <div
      bind:this={trackEl}
      class="relative inline-flex items-center rounded-full border border-white/10 bg-white/[0.03] p-1 backdrop-blur"
      role="tablist"
      aria-label="Billing cycle"
    >
      <span
        class="absolute inset-y-1 rounded-full bg-gradient-to-b from-emerald-500 to-emerald-600 shadow-[0_6px_20px_-6px_rgba(16,185,129,0.7),inset_0_1px_0_rgba(255,255,255,0.18)] transition-[left,width] duration-300 ease-out"
        aria-hidden="true"
        style={thumbStyle}
      ></span>
      <button
        bind:this={monthlyBtn}
        role="tab"
        aria-selected={cycle === 'monthly'}
        class="relative z-10 rounded-full px-6 py-1.5 text-sm font-medium ring-focus transition-colors duration-300 {cycle ===
        'monthly'
          ? 'text-white'
          : 'text-zinc-400 hover:text-white'}"
        onclick={() => (cycle = 'monthly')}
      >
        {$_('pricing.toggle_monthly')}
      </button>
      <button
        bind:this={yearlyBtn}
        role="tab"
        aria-selected={cycle === 'yearly'}
        class="relative z-10 rounded-full px-6 py-1.5 text-sm font-medium ring-focus transition-colors duration-300 {cycle ===
        'yearly'
          ? 'text-white'
          : 'text-zinc-400 hover:text-white'}"
        onclick={() => (cycle = 'yearly')}
      >
        {$_('pricing.toggle_yearly')}
      </button>
    </div>
    <span
      class="inline-flex items-center gap-1.5 rounded-full border border-emerald-400/25 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-semibold tracking-wide text-emerald-200 transition-opacity duration-300"
      style="opacity: {cycle === 'yearly' ? '1' : '0.55'};"
    >
      <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
      {$_('pricing.save_badge')} {$_('pricing.toggle_yearly').toLowerCase()}
    </span>
  </div>

  <div class="mx-auto mt-12 grid max-w-3xl gap-6 sm:grid-cols-2">
    <div class="reveal" use:reveal={{ delay: 0 }}>
      <PlanCard
        plan={PLANS.free}
        {cycle}
        onChoose={choose}
        ctaLabel={$_('pricing.cta_download_free')}
      />
    </div>
    <div class="reveal" use:reveal={{ delay: 100 }}>
      <PlanCard
        plan={PLANS.pro}
        {cycle}
        featured
        onChoose={choose}
        ctaLabel={$_('pricing.cta_buy_pro')}
      />
    </div>
  </div>

  <!-- Comparison table -->
  <div class="reveal mt-24" use:reveal>
    <div class="mx-auto max-w-2xl text-center">
      <h2 class="text-balance text-2xl font-bold tracking-tight text-white sm:text-3xl">
        {$_('pricing.compare_title')}
      </h2>
      <p class="mt-2 text-sm text-zinc-400">{$_('pricing.compare_subtitle')}</p>
    </div>

    <div class="mx-auto mt-8 max-w-3xl overflow-hidden rounded-2xl border border-white/[0.06] bg-white/[0.02] card-edge">
      <div class="grid grid-cols-[1.6fr_1fr_1fr] items-center gap-0 border-b border-white/[0.06] bg-zinc-950/40 px-5 py-3 text-[11px] font-semibold uppercase tracking-wider text-zinc-400">
        <div>Feature</div>
        <div class="text-center">{$_('plan.free')}</div>
        <div class="text-center text-emerald-300">{$_('plan.pro')}</div>
      </div>
      {#each compareRows as row, i (row.label)}
        <div
          class="grid grid-cols-[1.6fr_1fr_1fr] items-center gap-0 border-b border-white/[0.04] px-5 py-3 text-sm last:border-b-0 {i %
            2 ===
          0
            ? ''
            : 'bg-white/[0.012]'}"
        >
          <div class="text-zinc-200">{row.label}</div>
          <div class="text-center font-mono text-zinc-400">{row.free}</div>
          <div class="text-center font-mono text-emerald-200">{row.pro}</div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Notes with icons -->
  <div class="mt-16 grid gap-5 sm:grid-cols-3 text-sm">
    <div
      class="reveal card-edge group rounded-xl border border-white/[0.06] bg-white/[0.02] p-5 transition-colors hover:border-emerald-500/30"
      use:reveal
    >
      <div class="flex items-center gap-2.5">
        <span
          class="grid h-8 w-8 place-items-center rounded-lg bg-emerald-500/10 text-emerald-300 ring-1 ring-inset ring-emerald-400/15 transition-transform duration-500 group-hover:rotate-[-6deg]"
        >
          <Check class="h-4 w-4" />
        </span>
        <h3 class="font-semibold text-zinc-100">{$_('pricing.note_cancel_title')}</h3>
      </div>
      <p class="mt-3 leading-relaxed text-zinc-400">{$_('pricing.note_cancel_body')}</p>
    </div>
    <div
      class="reveal card-edge group rounded-xl border border-white/[0.06] bg-white/[0.02] p-5 transition-colors hover:border-emerald-500/30"
      use:reveal={{ delay: 80 }}
    >
      <div class="flex items-center gap-2.5">
        <span
          class="grid h-8 w-8 place-items-center rounded-lg bg-emerald-500/10 text-emerald-300 ring-1 ring-inset ring-emerald-400/15 transition-transform duration-500 group-hover:rotate-[-6deg]"
        >
          <Minus class="h-4 w-4 rotate-90" />
        </span>
        <h3 class="font-semibold text-zinc-100">{$_('pricing.note_mor_title')}</h3>
      </div>
      <p class="mt-3 leading-relaxed text-zinc-400">{$_('pricing.note_mor_body')}</p>
    </div>
    <div
      class="reveal card-edge group rounded-xl border border-white/[0.06] bg-white/[0.02] p-5 transition-colors hover:border-emerald-500/30"
      use:reveal={{ delay: 160 }}
    >
      <div class="flex items-center gap-2.5">
        <span
          class="grid h-8 w-8 place-items-center rounded-lg bg-emerald-500/10 text-emerald-300 ring-1 ring-inset ring-emerald-400/15 transition-transform duration-500 group-hover:rotate-[-6deg]"
        >
          <Github class="h-4 w-4" />
        </span>
        <h3 class="font-semibold text-zinc-100">{$_('pricing.note_lockin_title')}</h3>
      </div>
      <p class="mt-3 leading-relaxed text-zinc-400">{$_('pricing.note_lockin_body')}</p>
    </div>
  </div>
</section>
