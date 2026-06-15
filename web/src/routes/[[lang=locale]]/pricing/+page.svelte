<script lang="ts">
  import { _ } from 'svelte-i18n';
  import PlanCard from '$lib/components/PlanCard.svelte';
  import { PLANS, formatPlanQuota, formatMaxSaveSize, formatBandwidthQuota } from '$lib/plans';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { goto } from '$app/navigation';
  import { reveal } from '$lib/actions/reveal';
  import { Check, Receipt, Unlock } from 'lucide-svelte';

  let cycle = $state<BillingCycle>('monthly');

  // Yearly discount derived from the real prices in plans.ts — never typed
  // by hand. 12 × 4,49 € = 53,88 € vs 35,99 € ⇒ 33 %.
  const yearlySavingsPct = Math.round(
    (1 - PLANS.pro.priceYearly / (PLANS.pro.priceMonthly * 12)) * 100
  );
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
      free: formatPlanQuota('free'),
      pro: formatPlanQuota('pro')
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
      free: formatMaxSaveSize('free'),
      pro: formatMaxSaveSize('pro')
    },
    {
      label: $_('pricing.compare_bandwidth'),
      free: formatBandwidthQuota('free'),
      pro: formatBandwidthQuota('pro')
    },
    {
      label: $_('pricing.compare_export'),
      free: $_('pricing.compare_yes'),
      pro: $_('pricing.compare_yes')
    },
    {
      label: $_('pricing.compare_support'),
      free: $_('pricing.compare_email'),
      pro: $_('pricing.compare_email')
    },
    {
      label: $_('pricing.compare_selfhost'),
      free: $_('pricing.compare_anytime'),
      pro: $_('pricing.compare_anytime')
    }
  ]);

  const notes = [
    { icon: Check, title: 'pricing.note_cancel_title', body: 'pricing.note_cancel_body' },
    { icon: Receipt, title: 'pricing.note_mor_title', body: 'pricing.note_mor_body' },
    { icon: Unlock, title: 'pricing.note_lockin_title', body: 'pricing.note_lockin_body' }
  ];
</script>

<svelte:head>
  <title>{`${$_('pricing.title')} — Hoard`}</title>
  <link rel="canonical" href="https://hoard.services/pricing" />
</svelte:head>

<section class="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24">
  <div class="mx-auto max-w-2xl text-center">
    <p class="kicker justify-center">Pricing</p>
    <h1 class="mt-3 text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('pricing.title')}
    </h1>
    <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
      {$_('pricing.subtitle')}
    </p>
  </div>

  <div class="mt-10 flex flex-col items-center gap-3">
    <div
      bind:this={trackEl}
      class="relative inline-flex items-center rounded-full border border-line bg-surface p-1"
      role="tablist"
      aria-label="Billing cycle"
    >
      <span
        class="absolute inset-y-1 rounded-full bg-accent-deep transition-[left,width] duration-300 ease-out"
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
          : 'text-ink-soft hover:text-ink'}"
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
          : 'text-ink-soft hover:text-ink'}"
        onclick={() => (cycle = 'yearly')}
      >
        {$_('pricing.toggle_yearly')}
      </button>
    </div>
    <span
      class="inline-flex items-center gap-1.5 rounded-full border border-accent/30 bg-accent-tint px-2.5 py-0.5 font-mono text-[11px] font-medium text-accent transition-opacity duration-300"
      style="opacity: {cycle === 'yearly' ? '1' : '0.55'};"
    >
      {$_('pricing.save_badge', { values: { pct: yearlySavingsPct } })}
      {$_('pricing.toggle_yearly').toLowerCase()}
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
      <h2 class="text-balance text-2xl font-semibold text-ink sm:text-3xl">
        {$_('pricing.compare_title')}
      </h2>
      <p class="mt-2 text-sm text-ink-soft">{$_('pricing.compare_subtitle')}</p>
    </div>

    <div class="mx-auto mt-8 max-w-3xl overflow-hidden rounded-2xl border border-line bg-surface">
      <div
        class="grid grid-cols-[1.6fr_1fr_1fr] items-center gap-0 border-b border-line bg-bg px-5 py-3 font-mono text-[11px] font-medium uppercase tracking-wider text-ink-faint"
      >
        <div>Feature</div>
        <div class="text-center">{$_('plan.free')}</div>
        <div class="text-center text-accent">{$_('plan.pro')}</div>
      </div>
      {#each compareRows as row (row.label)}
        <div
          class="grid grid-cols-[1.6fr_1fr_1fr] items-center gap-0 border-b border-line px-5 py-3 text-sm last:border-b-0"
        >
          <div class="text-ink">{row.label}</div>
          <div class="text-center font-mono text-xs text-ink-soft">{row.free}</div>
          <div class="text-center font-mono text-xs font-medium text-accent">{row.pro}</div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Notes -->
  <div class="mt-16 grid gap-5 text-sm sm:grid-cols-3">
    {#each notes as n, i (n.title)}
      <div
        class="reveal rounded-2xl border border-line bg-surface p-6"
        use:reveal={{ delay: i * 80 }}
      >
        <div class="flex items-center gap-2.5">
          <span
            class="grid h-8 w-8 place-items-center rounded-lg bg-accent-tint text-accent"
          >
            <n.icon class="h-4 w-4" />
          </span>
          <h3 class="font-semibold text-ink">{$_(n.title)}</h3>
        </div>
        <p class="mt-3 leading-relaxed text-ink-soft">{$_(n.body)}</p>
      </div>
    {/each}
  </div>
</section>
