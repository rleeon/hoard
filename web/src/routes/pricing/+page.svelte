<script lang="ts">
  import { _ } from 'svelte-i18n';
  import PlanCard from '$lib/components/PlanCard.svelte';
  import { PLANS } from '$lib/plans';
  import { billing } from '$lib/billing';
  import { session } from '$lib/stores/session';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { goto } from '$app/navigation';

  let cycle = $state<BillingCycle>('monthly');

  function choose(plan: PlanId, c: BillingCycle) {
    if (plan === 'free') {
      goto('/login');
      return;
    }
    if (!$session) {
      goto('/login?next=/pricing');
      return;
    }
    const url = billing.checkoutUrl(plan, c, $session.email);
    window.location.href = url;
  }
</script>

<section class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
  <div class="text-center">
    <h1 class="text-balance text-4xl font-bold tracking-tight text-white sm:text-5xl">
      {$_('pricing.title')}
    </h1>
    <p class="mt-4 text-pretty text-lg text-zinc-400">{$_('pricing.subtitle')}</p>
  </div>

  <div class="mt-10 flex justify-center">
    <div
      class="inline-flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900/60 p-1 backdrop-blur"
    >
      <button
        class="rounded-full px-4 py-1.5 text-sm font-medium transition-all
          {cycle === 'monthly' ? 'bg-emerald-600 text-white shadow' : 'text-zinc-400 hover:text-white'}"
        onclick={() => (cycle = 'monthly')}
      >
        {$_('pricing.toggle_monthly')}
      </button>
      <button
        class="relative rounded-full px-4 py-1.5 text-sm font-medium transition-all
          {cycle === 'yearly' ? 'bg-emerald-600 text-white shadow' : 'text-zinc-400 hover:text-white'}"
        onclick={() => (cycle = 'yearly')}
      >
        {$_('pricing.toggle_yearly')}
        <span
          class="ml-2 rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-semibold text-emerald-300"
        >
          {$_('pricing.save_badge')}
        </span>
      </button>
    </div>
  </div>

  <div class="mx-auto mt-12 grid max-w-3xl gap-6 sm:grid-cols-2">
    <PlanCard plan={PLANS.free} {cycle} onChoose={choose} />
    <PlanCard plan={PLANS.pro} {cycle} featured onChoose={choose} />
  </div>

  <div class="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-3 text-sm text-zinc-400">
    <div>
      <h3 class="font-semibold text-zinc-200">{$_('pricing.note_cancel_title')}</h3>
      <p class="mt-1">{$_('pricing.note_cancel_body')}</p>
    </div>
    <div>
      <h3 class="font-semibold text-zinc-200">{$_('pricing.note_mor_title')}</h3>
      <p class="mt-1">{$_('pricing.note_mor_body')}</p>
    </div>
    <div>
      <h3 class="font-semibold text-zinc-200">{$_('pricing.note_lockin_title')}</h3>
      <p class="mt-1">{$_('pricing.note_lockin_body')}</p>
    </div>
  </div>
</section>
