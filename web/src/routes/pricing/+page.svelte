<script lang="ts">
  import { _ } from 'svelte-i18n';
  import PlanCard from '$lib/components/PlanCard.svelte';
  import { PLANS } from '$lib/plans';
  import { billing } from '$lib/billing';
  import { session } from '$lib/stores/session';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { goto } from '$app/navigation';
  import { reveal } from '$lib/actions/reveal';

  let cycle = $state<BillingCycle>('monthly');

  function choose(plan: PlanId, c: BillingCycle) {
    if (plan === 'free') {
      goto('/download');
      return;
    }
    const url = billing.checkoutUrl(plan, c, $session?.email);
    window.location.href = url;
  }
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
    <h1 class="text-balance text-4xl font-bold tracking-tight text-white sm:text-5xl">
      {$_('pricing.title')}
    </h1>
    <p class="mt-4 text-pretty text-lg text-zinc-400">{$_('pricing.subtitle')}</p>
  </div>

  <div class="mt-10 flex flex-col items-center gap-3">
    <div
      class="relative inline-flex items-center rounded-full border border-white/10 bg-white/[0.03] p-1 backdrop-blur"
      role="tablist"
      aria-label="Billing cycle"
    >
      <span
        class="absolute inset-y-1 left-1 rounded-full bg-emerald-600 shadow-[0_6px_20px_-6px_rgba(16,185,129,0.7)] transition-transform duration-300 ease-out"
        aria-hidden="true"
        style="width: calc(50% - 4px); transform: translateX({cycle === 'monthly' ? '0' : '100%'});"
      ></span>
      <button
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

  <div class="mt-16 grid gap-6 sm:grid-cols-3 text-sm text-zinc-400">
    <div class="reveal rounded-xl border border-white/[0.06] bg-white/[0.02] p-5" use:reveal>
      <h3 class="font-semibold text-zinc-100">{$_('pricing.note_cancel_title')}</h3>
      <p class="mt-2 leading-relaxed">{$_('pricing.note_cancel_body')}</p>
    </div>
    <div class="reveal rounded-xl border border-white/[0.06] bg-white/[0.02] p-5" use:reveal={{ delay: 80 }}>
      <h3 class="font-semibold text-zinc-100">{$_('pricing.note_mor_title')}</h3>
      <p class="mt-2 leading-relaxed">{$_('pricing.note_mor_body')}</p>
    </div>
    <div class="reveal rounded-xl border border-white/[0.06] bg-white/[0.02] p-5" use:reveal={{ delay: 160 }}>
      <h3 class="font-semibold text-zinc-100">{$_('pricing.note_lockin_title')}</h3>
      <p class="mt-2 leading-relaxed">{$_('pricing.note_lockin_body')}</p>
    </div>
  </div>
</section>
