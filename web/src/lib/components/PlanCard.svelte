<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { Check } from 'lucide-svelte';
  import Button from './Button.svelte';
  import type { PlanId, BillingCycle, PlanLimits } from '$lib/types';
  import { formatPlanQuota } from '$lib/plans';

  interface Props {
    plan: PlanLimits;
    cycle: BillingCycle;
    current?: PlanId | null;
    onChoose?: (plan: PlanId, cycle: BillingCycle) => void;
    featured?: boolean;
    ctaLabel?: string;
  }
  let { plan, cycle, current, onChoose, featured = false, ctaLabel }: Props = $props();

  let isCurrent = $derived(current === plan.id);
  let price = $derived(cycle === 'monthly' ? plan.priceMonthly : plan.priceYearly);
  let priceLabel = $derived(price === 0 ? '0 €' : `${price.toLocaleString('es-ES')} €`);
  let suffix = $derived(cycle === 'monthly' ? $_('pricing.per_month') : $_('pricing.per_year'));
</script>

<div
  class="relative flex h-full flex-col rounded-2xl border p-7 transition-all
    {featured
      ? 'border-emerald-500/50 bg-gradient-to-b from-emerald-950/40 to-zinc-900/60 shadow-2xl shadow-emerald-950/40 scale-[1.02]'
      : 'border-zinc-800 bg-zinc-900/40 hover:border-zinc-700'}"
>
  {#if featured}
    <span
      class="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-emerald-500 px-3 py-1 text-xs font-semibold tracking-wide text-emerald-950 shadow"
    >
      MOST POPULAR
    </span>
  {/if}

  <div class="mb-6">
    <h3 class="text-2xl font-semibold text-white">{$_(`plan.${plan.id}`)}</h3>
    <p class="mt-1 text-sm text-zinc-400">{$_(`plan.tagline.${plan.id}`)}</p>
  </div>

  <div class="mb-6 flex items-baseline gap-1">
    <span class="text-4xl font-bold tabular-nums text-white">{priceLabel}</span>
    {#if price > 0}<span class="text-sm text-zinc-400">{suffix}</span>{/if}
  </div>

  <ul class="mb-7 space-y-2.5 text-sm">
    <li class="flex items-start gap-2">
      <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
      <span class="text-zinc-300"
        >{$_('plan.storage', { values: { amount: formatPlanQuota(plan.id) } })}</span
      >
    </li>
    <li class="flex items-start gap-2">
      <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
      <span class="text-zinc-300">
        {#if plan.devices === null}{$_('plan.devices_unlimited')}{:else if plan.devices === 1}{$_('plan.devices_1')}{:else}{$_('plan.devices_n', { values: { n: plan.devices } })}{/if}
      </span>
    </li>
    <li class="flex items-start gap-2">
      <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
      <span class="text-zinc-300">
        {#if plan.saves === null}{$_('plan.saves_unlimited')}{:else}{$_('plan.saves_n', { values: { n: plan.saves } })}{/if}
      </span>
    </li>
    <li class="flex items-start gap-2">
      <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
      <span class="text-zinc-300"
        >{$_('plan.retention', { values: { days: plan.retentionDays } })}</span
      >
    </li>
    <li class="flex items-start gap-2">
      <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
      <span class="text-zinc-300">{$_('plan.export')}</span>
    </li>
    {#if plan.id !== 'free'}
      <li class="flex items-start gap-2">
        <Check class="mt-0.5 h-4 w-4 flex-none text-emerald-400" />
        <span class="text-zinc-300">
          {#if plan.id === 'proplus'}{$_('plan.priority_support')}{:else}{$_('plan.email_support')}{/if}
        </span>
      </li>
    {/if}
  </ul>

  <div class="mt-auto">
    {#if isCurrent}
      <Button variant="secondary" disabled full>{$_('pricing.cta_current')}</Button>
    {:else}
      <Button
        variant={featured ? 'primary' : 'secondary'}
        full
        onclick={() => onChoose?.(plan.id, cycle)}
      >
        {ctaLabel ?? $_('pricing.cta_choose', { values: { plan: $_(`plan.${plan.id}`) } })}
      </Button>
    {/if}
  </div>
</div>
