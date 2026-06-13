<script lang="ts">
  // Placeholder for the future Hoard-Wrapped feature. Intentionally logic-free:
  // a centred empty-state in the same visual language as the Map / Library
  // empty states. Real functionality lands in a later phase.
  import { Sparkles } from "lucide-svelte";
  import { _ } from "svelte-i18n";
</script>

<div class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
  <div
    class="flex h-16 w-16 items-center justify-center rounded-2xl border border-white/[0.08] bg-zinc-900/60"
  >
    <Sparkles size={30} class="text-emerald-400" />
  </div>
  <h1 class="font-display text-xl font-semibold text-zinc-100">
    {$_("hoard_wrapped.title")}
  </h1>
  <p class="max-w-sm text-sm text-zinc-500">{$_("hoard_wrapped.soon")}</p>
</div>
