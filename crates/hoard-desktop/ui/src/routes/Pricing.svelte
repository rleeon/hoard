<script lang="ts">
  // Free against Pro, inside the app, with the copy and numbers of the site's
  // /pricing. The Upgrade button used to open the card form straight away, and
  // from 03-10 to 13-10 that made 30 Polar checkouts with nobody paying: not one
  // of them got as far as picking a country. Nine of the thirteen accounts
  // clicked within a day of signing up, most nowhere near 2 GB. They were asking
  // what Pro is, and the answer they got was a payment form.
  import { _, locale } from "svelte-i18n";
  import { pop } from "svelte-spa-router";
  import {
    ArrowLeft,
    ArrowRight,
    Check,
    CircleHelp,
    LockOpen,
    Minus,
    Receipt,
  } from "@lucide/svelte";
  import Button from "../lib/components/Button.svelte";
  import { cloud, openUpgradePage } from "../lib/stores/cloud";
  import { track } from "../lib/stores/telemetry";
  import {
    PLANS,
    formatSaveSizeRange,
    formatStorage,
    type BillingCycle,
  } from "../lib/plans";

  let cycle = $state<BillingCycle>("monthly");
  let opening = $state(false);

  const plan = $derived($cloud.account?.plan ?? null);

  const proPrice = $derived(
    cycle === "monthly" ? PLANS.pro.priceMonthly : PLANS.pro.priceYearly,
  );
  const proPriceLabel = $derived(
    `${proPrice.toLocaleString($locale ?? undefined, { minimumFractionDigits: 2 })} €`,
  );

  // Creating the checkout is a round trip through our server to Polar, a second
  // or two with nothing on screen. Without the lock people clicked again, and
  // every click is another checkout.
  async function continueToPayment() {
    if (opening) return;
    opening = true;
    const interval = cycle === "yearly" ? "year" : "month";
    track("pricing.continue", { interval });
    try {
      await openUpgradePage("pro", interval);
    } finally {
      opening = false;
    }
  }

  const bands = $derived([
    {
      label: $_("pricing.band_small_label"),
      note: $_("pricing.band_small_note"),
      verdict: $_("pricing.band_small_verdict"),
      pro: false,
    },
    {
      label: $_("pricing.band_mid_label"),
      note: $_("pricing.band_mid_note"),
      verdict: $_("pricing.band_mid_verdict"),
      pro: false,
    },
    {
      label: $_("pricing.band_big_label"),
      note: $_("pricing.band_big_note"),
      verdict: $_("pricing.band_big_verdict"),
      pro: true,
    },
  ]);

  type Cell = string | boolean;
  const rows: { label: string; proTip?: string; free: Cell; pro: Cell }[] = $derived([
    { label: $_("pricing.row_storage"), free: formatStorage("free"), pro: formatStorage("pro") },
    {
      label: $_("pricing.row_save_size"),
      free: formatSaveSizeRange("free"),
      pro: formatSaveSizeRange("pro"),
    },
    {
      label: $_("pricing.row_devices"),
      proTip: $_("pricing.devices_tip"),
      free: String(PLANS.free.devices),
      pro: $_("pricing.val_unlimited"),
    },
    { label: $_("pricing.row_history"), free: true, pro: true },
    { label: $_("pricing.row_sync"), free: true, pro: true },
    { label: $_("pricing.row_export"), free: true, pro: true },
    { label: $_("pricing.compare_selfhost"), free: true, pro: true },
  ]);

  const notes = [
    { icon: LockOpen, title: "pricing.note_cancel_title", body: "pricing.note_cancel_body" },
    { icon: Receipt, title: "pricing.note_mor_title", body: "pricing.note_mor_body" },
    { icon: Check, title: "pricing.note_lockin_title", body: "pricing.note_lockin_body" },
  ];
</script>

<div class="mx-auto max-w-5xl px-6 py-8">
  <!-- Back to wherever the button was. The device-limit dialog sends people here
       from the middle of the setup wizard, and without this the only way out
       was the sidebar, which skips the rest of the wizard. -->
  <button
    type="button"
    onclick={() => pop()}
    class="anim-host mb-4 inline-flex items-center gap-2 text-sm text-zinc-400 transition-colors hover:text-zinc-100"
  >
    <ArrowLeft size={14} data-anim="nudge-back" />
    {$_("common.back")}
  </button>

  <div class="mb-6">
    <h1 class="font-display text-2xl font-semibold tracking-tight text-zinc-50">
      {$_("pricing.title")}
    </h1>
    <p class="mt-0.5 max-w-3xl text-sm text-zinc-400">
      {$_("pricing.subtitle")}
    </p>
  </div>

  <!-- how many of your games fit -->
  <div class="overflow-hidden rounded-2xl border border-white/[0.08] bg-layer-1">
    {#each bands as b, i (b.label)}
      <div
        class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 px-5 py-4 {i > 0
          ? 'border-t border-white/[0.08]'
          : ''}"
      >
        <div class="min-w-0">
          <p class="font-mono text-sm {b.pro ? 'text-emerald-400' : 'text-zinc-100'}">
            {b.label}
          </p>
          <p class="mt-1 text-xs text-zinc-500">{b.note}</p>
        </div>
        <span
          class="shrink-0 text-sm {b.pro ? 'font-semibold text-emerald-400' : 'text-zinc-400'}"
        >
          {b.verdict}
        </span>
      </div>
    {/each}
  </div>
  <p class="mt-3 max-w-3xl text-xs leading-relaxed text-zinc-500">
    {$_("pricing.bands_note")}
  </p>

  <!-- billing cycle -->
  <div class="mb-4 mt-8 flex items-center gap-3">
    <div class="flex gap-1 rounded-lg border border-white/[0.08] bg-layer-2 p-1">
      {#each [["monthly", "pricing.toggle_monthly"], ["yearly", "pricing.toggle_yearly"]] as const as [c, label] (c)}
        <button
          type="button"
          onclick={() => (cycle = c)}
          aria-pressed={cycle === c}
          class="rounded-md px-3 py-1 text-xs font-medium ring-1 ring-inset transition {cycle === c
            ? 'bg-emerald-600/20 text-emerald-300 ring-emerald-600/40'
            : 'text-zinc-400 ring-transparent hover:ring-white/15'}"
        >
          {$_(label)}
        </button>
      {/each}
    </div>
    <span
      class="font-mono text-[11px] text-emerald-400 transition-opacity"
      style="opacity: {cycle === 'yearly' ? 1 : 0.5}"
    >
      {$_("pricing.yearly_badge")}
    </span>
  </div>

  <!-- comparison -->
  <div
    class="grid grid-cols-[1.5fr_1fr_1.2fr] rounded-2xl border border-white/[0.08] bg-layer-1"
  >
    <div class="border-b border-white/[0.08] p-5"></div>
    <div class="flex flex-col gap-1.5 border-b border-l border-white/[0.08] p-5 text-center">
      <h2 class="font-display text-base font-semibold text-zinc-100">Hoard Free</h2>
      <p class="font-mono text-xl text-zinc-100">0 €</p>
      <p class="text-xs text-zinc-500">{$_("pricing.free_forever")}</p>
      {#if plan === "free"}
        <p class="mt-auto pt-3 text-xs font-medium text-zinc-400">
          {$_("pricing.cta_current")}
        </p>
      {/if}
    </div>
    <div
      class="flex flex-col gap-1.5 rounded-tr-2xl border-b border-l border-emerald-800/60 p-5 text-center"
    >
      <h2 class="font-display text-base font-semibold text-emerald-400">Hoard Pro</h2>
      <p class="font-mono text-xl text-zinc-100">
        {proPriceLabel}<span class="text-sm text-zinc-500"
          >{cycle === "monthly" ? $_("pricing.per_month") : $_("pricing.per_year")}</span
        >
      </p>
      <p class="text-xs text-zinc-500">
        {cycle === "monthly" ? $_("pricing.billed_monthly") : $_("pricing.billed_yearly")}
      </p>
      <p class="relative flex items-center justify-center gap-1 text-xs text-zinc-500">
        {$_("pricing.tax_note")}
        <span class="group inline-flex">
          <CircleHelp size={13} class="cursor-help text-emerald-500/70" />
          <span
            class="pointer-events-none absolute left-1/2 top-6 z-20 w-56 -translate-x-1/2 rounded-lg border border-zinc-800 bg-zinc-950 p-2.5 text-left text-[11px] leading-relaxed text-zinc-300 opacity-0 shadow-lg transition-opacity group-hover:opacity-100"
          >
            {$_("pricing.tax_tip")}
          </span>
        </span>
      </p>
      <div class="mt-auto pt-3">
        {#if plan === "pro"}
          <p class="text-xs font-medium text-emerald-400">{$_("pricing.cta_current")}</p>
        {:else}
          <Button
            variant="outline"
            class="w-full"
            loading={opening}
            onclick={continueToPayment}
          >
            {$_("common.continue")}
            {#if !opening}
              <ArrowRight size={16} data-anim="nudge" />
            {/if}
          </Button>
        {/if}
      </div>
    </div>

    {#each rows as r, i (r.label)}
      {@const last = i === rows.length - 1}
      <div
        class="flex items-center border-t border-white/[0.08] px-5 py-3 text-sm text-zinc-400"
      >
        {r.label}
      </div>
      <div
        class="grid place-items-center border-l border-t border-white/[0.08] px-5 py-3 font-mono text-sm text-zinc-300"
      >
        {#if typeof r.free === "boolean"}
          {#if r.free}<Check size={16} class="text-emerald-400" />{:else}<Minus
              size={16}
              class="text-zinc-600"
            />{/if}
        {:else}{r.free}{/if}
      </div>
      <div
        class="relative flex items-center justify-center gap-1.5 border-l border-t border-emerald-800/60 px-5 py-3 font-mono text-sm text-emerald-400 {last
          ? 'rounded-br-2xl'
          : ''}"
      >
        {#if typeof r.pro === "boolean"}
          {#if r.pro}<Check size={16} />{:else}<Minus size={16} class="text-zinc-600" />{/if}
        {:else}{r.pro}{/if}
        {#if r.proTip}
          <span class="group inline-flex">
            <CircleHelp size={13} class="cursor-help text-emerald-500/70" />
            <span
              class="pointer-events-none absolute right-3 top-10 z-20 w-60 rounded-lg border border-zinc-800 bg-zinc-950 p-2.5 text-left font-sans text-[11px] leading-relaxed text-zinc-300 opacity-0 shadow-lg transition-opacity group-hover:opacity-100"
            >
              {r.proTip}
            </span>
          </span>
        {/if}
      </div>
    {/each}
  </div>

  <p class="mt-6 max-w-3xl text-sm leading-relaxed text-zinc-400">
    {$_("pricing.who_pays")}
  </p>

  <div class="mt-6 grid gap-4 sm:grid-cols-3">
    {#each notes as n (n.title)}
      <div class="anim-host rounded-2xl border border-white/[0.08] bg-layer-1 p-5">
        <n.icon size={16} class="text-emerald-400" data-anim="pop" />
        <h3 class="mt-3 text-sm font-semibold text-zinc-100">{$_(n.title)}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-zinc-400">{$_(n.body)}</p>
      </div>
    {/each}
  </div>
</div>
