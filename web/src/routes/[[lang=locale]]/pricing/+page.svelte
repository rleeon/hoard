<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import Button from '$lib/components/Button.svelte';
  import { PLANS, formatPlanQuota, formatMaxSaveSize } from '$lib/plans';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { goto } from '$app/navigation';
  import { reveal } from '$lib/actions/reveal';
  import { localeHref } from '$lib/i18n/href';
  import { Check, HelpCircle, Minus, Receipt, Unlock } from 'lucide-svelte';

  let cycle = $state<BillingCycle>('monthly');

  function choose(plan: PlanId) {
    if (plan === 'free') {
      goto($localeHref('/download'));
      return;
    }
    goto(`/checkout?plan=${plan}&cycle=${cycle}`);
  }

  const proPrice = $derived(cycle === 'monthly' ? PLANS.pro.priceMonthly : PLANS.pro.priceYearly);
  const proPriceLabel = $derived(
    `${proPrice.toLocaleString($locale ?? undefined, { minimumFractionDigits: 2 })} €`
  );
  const proSuffix = $derived(cycle === 'monthly' ? $_('pricing.per_month') : $_('pricing.per_year'));

  // The bands answer the only question a visitor really has: how many of my
  // games fit for free. They are stated in save sizes, never in versions, since
  // only a save's first copy is stored whole and the deltas after it are small.
  // The counts sit under the plain arithmetic (2 GB / 50 MB is 40) to leave room
  // for the history those games accumulate.
  const bands = $derived([
    {
      label: $_('pricing.band_small_label'),
      note: $_('pricing.band_small_note'),
      verdict: $_('pricing.band_small_verdict'),
      pro: false
    },
    {
      label: $_('pricing.band_mid_label'),
      note: $_('pricing.band_mid_note'),
      verdict: $_('pricing.band_mid_verdict'),
      pro: false
    },
    {
      label: $_('pricing.band_big_label'),
      note: $_('pricing.band_big_note'),
      verdict: $_('pricing.band_big_verdict'),
      pro: true
    }
  ]);

  type Cell = string | boolean;
  const rows: { label: string; proTip?: string; free: Cell; pro: Cell }[] = $derived([
    { label: $_('pricing.row_storage'), free: formatPlanQuota('free'), pro: formatPlanQuota('pro') },
    {
      label: $_('pricing.row_save_size'),
      free: formatMaxSaveSize('free'),
      pro: formatMaxSaveSize('pro')
    },
    {
      label: $_('pricing.row_devices'),
      proTip: $_('pricing.devices_tip'),
      free: '3',
      pro: $_('pricing.val_unlimited')
    },
    { label: $_('pricing.row_history'), free: true, pro: true },
    { label: $_('pricing.row_sync'), free: true, pro: true },
    { label: $_('pricing.row_export'), free: true, pro: true },
    { label: $_('pricing.compare_selfhost'), free: true, pro: true },
    { label: 'Hoard Screen', free: false, pro: true }
  ]);

  const notes = [
    { icon: Unlock, title: 'pricing.note_cancel_title', body: 'pricing.note_cancel_body' },
    { icon: Receipt, title: 'pricing.note_mor_title', body: 'pricing.note_mor_body' },
    { icon: Check, title: 'pricing.note_lockin_title', body: 'pricing.note_lockin_body' }
  ];
</script>

<Seo path="/pricing" key="pricing" />

<section class="mx-auto max-w-4xl px-4 pb-16 pt-16 sm:px-6 sm:pb-24 sm:pt-24 2xl:pt-12">
  <div class="reveal text-center" use:reveal>
    <h1 class="text-balance text-4xl font-semibold text-ink sm:text-5xl">{$_('pricing.title')}</h1>
    <p class="mx-auto mt-4 max-w-2xl text-pretty text-lg leading-relaxed text-ink-soft">
      {$_('pricing.subtitle')}
    </p>
  </div>

  <!-- How many of your games fit -->
  <div class="reveal mt-10 overflow-hidden rounded-2xl border border-line" use:reveal>
    {#each bands as b, i (b.label)}
      <div
        class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2 px-6 py-5 {i > 0
          ? 'border-t border-line'
          : ''} {b.pro ? 'bg-accent/10' : 'bg-bg'}"
      >
        <div class="min-w-0">
          <p class="font-mono text-sm {b.pro ? 'text-accent' : 'text-ink'}">{b.label}</p>
          <p class="mt-1 text-xs {b.pro ? 'text-accent/70' : 'text-ink-faint'}">{b.note}</p>
        </div>
        <span class="shrink-0 text-sm {b.pro ? 'font-semibold text-accent' : 'text-ink-soft'}"
          >{b.verdict}</span
        >
      </div>
    {/each}
  </div>
  <p class="mt-4 max-w-2xl text-sm leading-relaxed text-ink-faint">{$_('pricing.bands_note')}</p>

  <!-- Billing cycle, centred on the table as a whole. The "2 months free" note
       hangs off the toggle absolutely so it cannot shove it off centre. -->
  <div class="reveal relative mb-6 mt-14 flex flex-col items-center gap-2 sm:block sm:text-center" use:reveal>
    <div class="relative inline-block">
      <div class="inline-flex rounded-full border border-line bg-surface p-1">
        <button
          class="rounded-full px-4 py-1.5 text-sm transition-colors {cycle === 'monthly'
            ? 'bg-accent text-pine'
            : 'text-ink-soft hover:text-ink'}"
          onclick={() => (cycle = 'monthly')}>{$_('pricing.toggle_monthly')}</button
        >
        <button
          class="rounded-full px-4 py-1.5 text-sm transition-colors {cycle === 'yearly'
            ? 'bg-accent text-pine'
            : 'text-ink-soft hover:text-ink'}"
          onclick={() => (cycle = 'yearly')}>{$_('pricing.toggle_yearly')}</button
        >
      </div>
      <span
        class="pointer-events-none whitespace-nowrap font-mono text-[11px] text-accent transition-opacity max-sm:mt-2 max-sm:block max-sm:text-center sm:absolute sm:left-full sm:top-1/2 sm:ml-3 sm:-translate-y-1/2"
        style="opacity: {cycle === 'yearly' ? 1 : 0.5}">{$_('pricing.yearly_badge')}</span
      >
    </div>
  </div>

  <!-- Comparison. The Pro column is a run of bordered cells, not a background
       panel with cells drawn over it, so nothing can eat its right edge. -->
  <!-- Three columns need room, so below `sm` the plans stack as two cards
       instead. The old page did the same; rewriting it I dropped the phone half
       and the Pro column ran off the screen. -->
  <div class="hidden grid-cols-[1.5fr_1fr_1.2fr] overflow-hidden rounded-2xl border border-line sm:grid">
    <div class="border-b border-line p-5"></div>
    <div class="flex flex-col gap-2 border-b border-l border-line p-5 text-center">
      <h2 class="font-display text-lg font-semibold text-ink">Hoard Free</h2>
      <p class="font-mono text-2xl text-ink">0 €</p>
      <p class="text-xs text-ink-faint">{$_('pricing.free_forever')}</p>
      <div class="mt-auto pt-3">
        <Button variant="outline" full onclick={() => choose('free')}>
          {$_('pricing.cta_download_free')}
        </Button>
      </div>
    </div>
    <div class="flex flex-col gap-2 border-b border-l border-accent/45 bg-accent-tint p-5 text-center">
      <h2 class="font-display text-lg font-semibold text-accent">Hoard Pro</h2>
      <p class="font-mono text-2xl text-ink">
        {proPriceLabel}<span class="text-sm text-ink-faint">{proSuffix}</span>
      </p>
      <p class="text-xs text-ink-faint">
        {cycle === 'monthly' ? $_('pricing.billed_monthly') : $_('pricing.billed_yearly')}
      </p>
      <div class="mt-auto pt-3">
        <Button variant="primary" full onclick={() => choose('pro')}>
          {$_('pricing.cta_buy_pro')}
        </Button>
      </div>
    </div>

    {#each rows as r (r.label)}
      <div class="flex items-center border-t border-line px-5 py-3.5 text-sm text-ink-soft">
        {r.label}
      </div>
      <div
        class="grid place-items-center border-l border-t border-line px-5 py-3.5 font-mono text-sm text-ink-soft"
      >
        {#if typeof r.free === 'boolean'}
          {#if r.free}<Check class="h-4 w-4 text-accent" />{:else}<Minus
              class="h-4 w-4 text-ink-faint"
            />{/if}
        {:else}{r.free}{/if}
      </div>
      <div
        class="relative flex items-center justify-center gap-1.5 border-l border-t border-accent/45 bg-accent-tint px-5 py-3.5 font-mono text-sm text-accent"
      >
        {#if typeof r.pro === 'boolean'}
          {#if r.pro}<Check class="h-4 w-4" />{:else}<Minus class="h-4 w-4 text-ink-faint" />{/if}
        {:else}{r.pro}{/if}
        {#if r.proTip}
          <!-- Anchored right and opening downwards so the table's rounded clip
               does not cut it off, which is what happened on the label side. -->
          <span class="group inline-flex">
            <HelpCircle class="h-3.5 w-3.5 cursor-help text-accent/70" />
            <span
              class="pointer-events-none absolute right-3 top-10 z-20 w-60 rounded-lg border border-line bg-bg p-3 text-left font-sans text-xs leading-relaxed text-ink-soft opacity-0 shadow-lg transition-opacity group-hover:opacity-100"
              >{r.proTip}</span
            >
          </span>
        {/if}
      </div>
    {/each}
  </div>

  <div class="grid gap-5 sm:hidden">
    <div class="rounded-2xl border border-line bg-surface p-6">
      <h2 class="font-display text-lg font-semibold text-ink">Hoard Free</h2>
      <p class="mt-1 font-mono text-2xl text-ink">0 €</p>
      <p class="text-xs text-ink-faint">{$_('pricing.free_forever')}</p>
      <div class="mt-5">
        <Button variant="outline" full onclick={() => choose('free')}>
          {$_('pricing.cta_download_free')}
        </Button>
      </div>
      <dl class="mt-6 space-y-2.5 text-sm">
        {#each rows as r (r.label)}
          <div class="flex items-center justify-between gap-4 border-t border-line pt-2.5">
            <dt class="text-ink-soft">{r.label}</dt>
            <dd class="font-mono text-ink">
              {#if typeof r.free === 'boolean'}
                {#if r.free}<Check class="h-4 w-4 text-accent" />{:else}<Minus
                    class="h-4 w-4 text-ink-faint"
                  />{/if}
              {:else}{r.free}{/if}
            </dd>
          </div>
        {/each}
      </dl>
    </div>

    <div class="rounded-2xl border border-accent/45 bg-accent-tint p-6">
      <h2 class="font-display text-lg font-semibold text-accent">Hoard Pro</h2>
      <p class="mt-1 font-mono text-2xl text-ink">
        {proPriceLabel}<span class="text-sm text-ink-faint">{proSuffix}</span>
      </p>
      <p class="text-xs text-ink-faint">
        {cycle === 'monthly' ? $_('pricing.billed_monthly') : $_('pricing.billed_yearly')}
      </p>
      <div class="mt-5">
        <Button variant="primary" full onclick={() => choose('pro')}>
          {$_('pricing.cta_buy_pro')}
        </Button>
      </div>
      <dl class="mt-6 space-y-2.5 text-sm">
        {#each rows as r (r.label)}
          <div class="flex items-center justify-between gap-4 border-t border-accent/20 pt-2.5">
            <dt class="text-ink-soft">{r.label}</dt>
            <dd class="text-right font-mono text-accent">
              {#if typeof r.pro === 'boolean'}
                {#if r.pro}<Check class="ml-auto h-4 w-4" />{:else}<Minus
                    class="ml-auto h-4 w-4 text-ink-faint"
                  />{/if}
              {:else}{r.pro}{/if}
            </dd>
          </div>
        {/each}
      </dl>
      <p class="mt-4 text-xs leading-relaxed text-accent/70">{$_('pricing.devices_tip')}</p>
    </div>
  </div>

  <p class="reveal mx-auto mt-10 max-w-2xl text-center text-sm leading-relaxed text-ink-soft" use:reveal>
    {$_('pricing.who_pays')}
  </p>

  <div class="mt-14 grid gap-4 sm:grid-cols-3">
    {#each notes as n, i (n.title)}
      <div class="reveal rounded-2xl border border-line bg-surface p-6" use:reveal={{ delay: i * 80 }}>
        <n.icon class="h-4 w-4 text-accent" />
        <h3 class="mt-3 font-semibold text-ink">{$_(n.title)}</h3>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{$_(n.body)}</p>
      </div>
    {/each}
  </div>
</section>
