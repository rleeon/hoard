<script lang="ts">
  // Hoard-Wrapped entry. The real server-generated wrapped lands in a later
  // phase; for now this renders the Pro entitlement gate (en construcción for
  // Pro, trial days for Free, locked otherwise).
  import { Sparkles } from "lucide-svelte";
  import ProGate from "../lib/components/ProGate.svelte";
</script>

<ProGate titleKey="hoard_wrapped.title" feature="wrapple">
  {#snippet icon()}
    <Sparkles size={30} class="text-emerald-400" />
  {/snippet}
</ProGate>
