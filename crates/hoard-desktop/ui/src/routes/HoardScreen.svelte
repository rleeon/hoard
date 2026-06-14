<script lang="ts">
  // Hoard-Screen entry. The Pro overlay UI ships inside the official binary
  // (`PRO` true when the private `$pro` layer is linked), but it only renders
  // when the server entitlement says this account may use it — paid Pro or an
  // active trial. Free / signed-out / expired fall back to the `ProGate` (trial
  // offer or lock + upgrade). `PRO_DEV_UNLOCK` is the owner's local test escape
  // hatch and is never set in the public build.
  import { onMount } from "svelte";
  import { MonitorPlay } from "lucide-svelte";
  import ProGate from "../lib/components/ProGate.svelte";
  import { PRO, Screen } from "$pro";
  import {
    entitlements,
    refreshEntitlements,
    featureUnlocked,
    PRO_DEV_UNLOCK,
  } from "../lib/stores/entitlements";

  onMount(refreshEntitlements);

  const unlocked = $derived(
    PRO_DEV_UNLOCK || featureUnlocked($entitlements?.features.screen ?? null),
  );
</script>

{#if PRO && unlocked}
  <Screen />
{:else}
  <ProGate titleKey="hoard_screen.title" feature="screen">
    {#snippet icon()}
      <MonitorPlay size={30} class="text-emerald-400" />
    {/snippet}
  </ProGate>
{/if}
