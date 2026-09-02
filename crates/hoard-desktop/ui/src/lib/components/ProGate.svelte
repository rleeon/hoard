<script lang="ts">
  // Shared entitlement gate for the Pro features (Hoard-Screen / Hoard-Wrapped).
  // Reads the per-feature snapshot from the server (`GET /v1/cloud/entitlements`)
  // and maps it to one of three views:
  //   - entitled (paid Pro): "under construction" (the feature is being built)
  //   - trial_available or trial: "trial, N days left"
  //   - trial_expired or signed-out: "Pro feature" plus the upgrade CTA
  //
  // The trial is one week per feature and normally starts automatically the
  // first time the user opens the feature's page (`ProFeature` activates it on
  // first look). The `trial_available` branch here is the fallback for when
  // that auto-activation failed (offline, transient error): it shows the offer
  // with a manual retry button instead of a dead end.
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Lock } from "@lucide/svelte";
  import { push } from "svelte-spa-router";
  import {
    entitlements,
    refreshEntitlements,
    activateFeature,
    featureDaysLeft,
    type FeatureKey,
  } from "../stores/entitlements";

  let {
    titleKey,
    feature,
    icon,
  }: { titleKey: string; feature: FeatureKey; icon: Snippet } = $props();

  onMount(() => {
    refreshEntitlements();
  });

  const fs = $derived($entitlements?.features[feature] ?? null);
  const phase = $derived(fs?.state ?? "locked");
  const days = $derived(featureDaysLeft(fs));

  let starting = $state(false);

  // Start the one-week trial (retry path, the normal start is automatic on
  // first look). The server flips the feature to `trial`, which re-renders the
  // parent shell into the real Pro UI.
  async function startTrial() {
    if (starting) return;
    starting = true;
    try {
      await activateFeature(feature);
    } finally {
      starting = false;
    }
  }
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

  {#if phase === "entitled"}
    <span
      class="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
    >
      {$_("pro.under_construction")}
    </span>
    <p class="max-w-sm text-sm text-zinc-500">
      {$_("pro.under_construction_desc")}
    </p>
  {:else if phase === "trial_available"}
    <span
      class="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
    >
      {$_("pro.trial_available", { values: { n: days } })}
    </span>
    <p class="max-w-sm text-sm text-zinc-500">{$_("pro.trial_desc")}</p>
    <button
      onclick={startTrial}
      disabled={starting}
      class="mt-1 rounded-md bg-emerald-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-500 disabled:opacity-60"
    >
      {$_("pro.start_trial")}
    </button>
  {:else if phase === "trial"}
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
      onclick={() => push(`/pro?feature=${feature}`)}
      class="mt-1 rounded-md bg-emerald-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-500"
    >
      {$_("pro.upgrade")}
    </button>
  {/if}
</div>
