<script lang="ts">
  // Hoard-Wrapped entry. The Pro recap ships inside the official binary (`PRO`
  // true when the private `$pro` layer is linked), but only renders when the
  // server entitlement grants access — paid Pro or an active trial. Otherwise
  // the `ProGate` shows the trial offer or the lock + upgrade. `PRO_DEV_UNLOCK`
  // is the owner's local test escape hatch, never set in the public build.
  import { onMount } from "svelte";
  import { Sparkles } from "lucide-svelte";
  import ProGate from "../lib/components/ProGate.svelte";
  import { PRO, Wrapped } from "$pro";
  import {
    entitlements,
    refreshEntitlements,
    featureUnlocked,
    PRO_DEV_UNLOCK,
  } from "../lib/stores/entitlements";

  onMount(refreshEntitlements);

  const unlocked = $derived(
    PRO_DEV_UNLOCK || featureUnlocked($entitlements?.features.wrapple ?? null),
  );
</script>

{#if PRO && unlocked}
  <Wrapped />
{:else}
  <ProGate titleKey="hoard_wrapped.title" feature="wrapple">
    {#snippet icon()}
      <Sparkles size={30} class="text-emerald-400" />
    {/snippet}
  </ProGate>
{/if}
