<script lang="ts">
  // Hoard-Screen entry. The Pro overlay UI ships inside the official binary
  // (`PRO` true when the private `$pro` layer is linked), but it only renders
  // when the server entitlement says this account may use it, paid Pro or an
  // active trial. The `ProFeature` shell starts the one-week trial the first
  // time this page is opened and shows the days-left strip while it runs;
  // Free-expired / signed-out fall back to the gate inside it.
  import { MonitorPlay } from "@lucide/svelte";
  import ProFeature from "../lib/components/ProFeature.svelte";
  import { PRO, Screen } from "$pro";
</script>

<ProFeature titleKey="hoard_screen.title" feature="screen" pro={PRO}>
  {#snippet icon()}
    <MonitorPlay size={30} class="text-emerald-400" />
  {/snippet}
  <Screen />
</ProFeature>
