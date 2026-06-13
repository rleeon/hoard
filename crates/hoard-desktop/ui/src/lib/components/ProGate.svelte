<script lang="ts">
  // Shared entitlement gate for the Pro features (Hoard-Screen / Hoard-Wrapped).
  // Three states driven by the cloud account:
  //   - Pro              → "En construcción" (the feature is being built)
  //   - Free, in trial   → "Prueba: quedan N días"
  //   - Free, lapsed     → "Función Pro" + upgrade CTA (also the signed-out case)
  //
  // NOTE: trial days come from the client store (created_at + 30d). The chosen
  // model is per-feature first-use (server `GET /v1/cloud/entitlements`); wiring
  // this view to that endpoint is the registered debt for the feature phase.
  import type { Snippet } from "svelte";
  import { _ } from "svelte-i18n";
  import { Lock } from "lucide-svelte";
  import { cloud, trialDaysLeft, openUpgradePage } from "../stores/cloud";

  let { titleKey, icon }: { titleKey: string; icon: Snippet } = $props();

  const plan = $derived($cloud.account?.plan ?? null);
  const isPro = $derived(plan === "pro" || plan === "proplus");
  const days = $derived($trialDaysLeft);
  const inTrial = $derived(!isPro && days > 0);
</script>

<div class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
  <div
    class="flex h-16 w-16 items-center justify-center rounded-2xl border border-white/[0.08] bg-zinc-900/60"
  >
    {@render icon()}
  </div>
  <h1 class="font-display text-xl font-semibold text-zinc-100">
    {$_(titleKey)}
  </h1>

  {#if isPro}
    <span
      class="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
    >
      {$_("pro.under_construction")}
    </span>
    <p class="max-w-sm text-sm text-zinc-500">
      {$_("pro.under_construction_desc")}
    </p>
  {:else if inTrial}
    <span
      class="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
    >
      {$_("pro.trial_days_left", { values: { n: days } })}
    </span>
    <p class="max-w-sm text-sm text-zinc-500">{$_("pro.trial_desc")}</p>
  {:else}
    <span
      class="inline-flex items-center gap-1.5 rounded-full border border-white/[0.08] bg-zinc-900/60 px-3 py-1 text-xs font-medium text-zinc-400"
    >
      <Lock size={12} />
      {$_("pro.locked_title")}
    </span>
    <p class="max-w-sm text-sm text-zinc-500">{$_("pro.locked_desc")}</p>
    <button
      onclick={() => openUpgradePage("pro")}
      class="mt-1 rounded-md bg-emerald-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-500"
    >
      {$_("pro.upgrade")}
    </button>
  {/if}
</div>
