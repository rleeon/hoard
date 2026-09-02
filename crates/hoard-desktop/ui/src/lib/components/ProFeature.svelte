<script lang="ts">
  // Shared shell for the two Pro feature routes (Hoard-Screen / Hoard-Wrapped).
  //
  // Owns the "trial starts at first look" flow: on mount it refreshes the
  // entitlement snapshot and, if the feature is still `trial_available`,
  // activates it right away, opening this page is what starts the one-week
  // clock (never signup, never app start). While that round-trip is in flight
  // nothing is rendered, so the locked gate doesn't flash before the real UI
  // unlocks.
  //
  // During an active trial the real UI renders under a slim countdown strip
  // (whole days, rounded up) with the upgrade CTA next to it. Paid Pro renders
  // clean, and everything else falls back to `ProGate` (lock / expired).
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { _ } from "svelte-i18n";
  import ProGate from "./ProGate.svelte";
  import { push } from "svelte-spa-router";
  import { tourActive } from "../stores/tour";
  import {
    entitlements,
    refreshEntitlements,
    activateFeature,
    featureDaysLeft,
    featureUnlocked,
    PRO_DEV_UNLOCK,
    type FeatureKey,
  } from "../stores/entitlements";

  let {
    titleKey,
    feature,
    pro,
    icon,
    children,
  }: {
    titleKey: string;
    feature: FeatureKey;
    /** Whether the private Pro layer is linked (`PRO` from `$pro`). */
    pro: boolean;
    icon: Snippet;
    /** The real Pro UI, rendered only when the server unlocks it. */
    children: Snippet;
  } = $props();

  // Blank viewport (not a gate flash) until refresh + first-view activation
  // settle. Dev-unlocked builds skip the wait entirely.
  let deciding = $state(true);

  onMount(async () => {
    // Preview mode (guided tour): show the feature's promo/gate view without
    // spending the one-week trial the tour is only walking past.
    const preview = get(tourActive);
    const ent = await refreshEntitlements();
    if (!preview && ent?.features[feature]?.state === "trial_available") {
      await activateFeature(feature);
    }
    deciding = false;
  });

  const fs = $derived($entitlements?.features[feature] ?? null);
  const unlocked = $derived(PRO_DEV_UNLOCK || featureUnlocked(fs));
  const days = $derived(featureDaysLeft(fs));
</script>

{#if deciding && !PRO_DEV_UNLOCK}
  <!-- deciding — keep the viewport empty for the round-trip -->
{:else if pro && unlocked}
  <div class="flex h-full flex-col">
    {#if fs?.state === "trial"}
      <div
        class="flex items-center justify-center gap-3 border-b border-emerald-500/20 bg-emerald-500/[0.06] px-4 py-1.5 text-xs text-emerald-300"
      >
        <span>{$_("pro.trial_days_left", { values: { n: days } })}</span>
        <button
          onclick={() => push(`/pro?feature=${feature}`)}
          class="font-medium underline decoration-emerald-500/50 underline-offset-2 transition-colors hover:text-emerald-200"
        >
          {$_("pro.upgrade")}
        </button>
      </div>
    {/if}
    <div class="min-h-0 flex-1">
      {@render children()}
    </div>
  </div>
{:else}
  <ProGate {titleKey} {feature} {icon} />
{/if}
