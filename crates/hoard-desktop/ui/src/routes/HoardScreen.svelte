<script lang="ts">
  // Hoard-Screen entry. The real overlay editor lands in a later phase; for now
  // this renders the Pro entitlement gate (en construcción for Pro, trial days
  // for Free, locked otherwise).
  import { MonitorPlay } from "lucide-svelte";
  import ProGate from "../lib/components/ProGate.svelte";
</script>

<ProGate titleKey="hoard_screen.title">
  {#snippet icon()}
    <MonitorPlay size={30} class="text-emerald-400" />
  {/snippet}
</ProGate>
