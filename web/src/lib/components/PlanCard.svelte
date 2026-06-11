<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { Check } from 'lucide-svelte';
  import Button from './Button.svelte';
  import type { PlanId, BillingCycle, PlanLimits } from '$lib/types';
  import { formatPlanQuota, formatMaxSaveSize, formatBandwidthQuota } from '$lib/plans';

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
  let priceLabel = $derived(
    price === 0 ? '0 €' : `${price.toLocaleString('es-ES', { minimumFractionDigits: 2 })} €`
  );
  let suffix = $derived(cycle === 'monthly' ? $_('pricing.per_month') : $_('pricing.per_year'));

  // What 12 months would cost without the yearly discount, struck through
  // next to the real price. Both figures come from plans.ts — nothing typed
  // by hand.
  let showSavings = $derived(cycle === 'yearly' && price > 0);
  let fullYearly = $derived(plan.priceMonthly * 12);
  let fullYearlyLabel = $derived(
    `${fullYearly.toLocaleString('es-ES', { minimumFractionDigits: 2 })} €`
  );
  let savingsPct = $derived(Math.round((1 - plan.priceYearly / fullYearly) * 100));
</script>

<div
  class="relative flex h-full flex-col rounded-2xl border bg-surface p-7 transition-colors
    {featured ? 'border-accent ring-1 ring-accent' : 'border-line hover:border-line-strong'}"
>
  {#if featured}
    <span
      class="absolute -top-3 left-1/2 -translate-x-1/2 rounded-full bg-accent-deep px-3 py-1 font-mono text-[10px] font-medium uppercase tracking-wider text-white"
    >
      {$_('pricing.popular_badge')}
    </span>
  {/if}

  <div class="mb-6">
    <h3 class="font-display text-2xl font-semibold tracking-tight text-ink">
      {$_(`plan.${plan.id}`)}
    </h3>
    <p class="mt-1 text-sm text-ink-soft">{$_(`plan.tagline.${plan.id}`)}</p>
  </div>

  <div class="mb-7">
    <div class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
      <span class="font-display text-4xl font-bold tabular-nums tracking-tight text-ink">
        {priceLabel}
      </span>
      {#if showSavings}
        <s class="text-base font-medium tabular-nums text-ink-faint">{fullYearlyLabel}</s>
      {/if}
      {#if price > 0}<span class="text-sm text-ink-soft">{suffix}</span>{/if}
    </div>
    {#if showSavings}
      <p class="mt-1.5 text-xs font-medium text-accent">
        {$_('pricing.yearly_savings_note', { values: { pct: savingsPct } })}
      </p>
    {/if}
  </div>

  <ul class="mb-7 space-y-2.5 text-sm">
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft"
        >{$_('plan.storage', { values: { amount: formatPlanQuota(plan.id) } })}</span
      >
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">
        {#if plan.devices === null}{$_('plan.devices_unlimited')}{:else if plan.devices === 1}{$_('plan.devices_1')}{:else}{$_('plan.devices_n', { values: { n: plan.devices } })}{/if}
      </span>
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">{$_('plan.sync')}</span>
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">{$_('plan.saves_unlimited')}</span>
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">{$_('plan.history_forever')}</span>
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft"
        >{$_('plan.max_save_size', { values: { amount: formatMaxSaveSize(plan.id) } })}</span
      >
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft"
        >{$_('plan.bandwidth', { values: { amount: formatBandwidthQuota(plan.id) } })}</span
      >
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">{$_('plan.export')}</span>
    </li>
    <li class="flex items-start gap-2.5">
      <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
      <span class="text-ink-soft">{$_('plan.support')}</span>
    </li>
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
