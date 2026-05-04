<script lang="ts">
  /** Visual container with the standard zinc surface + border treatment. */
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
    class?: string;
  };

  let { children, class: extraClass = "" }: Props = $props();
</script>

<div
  class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 shadow-sm {extraClass}"
>
  {@render children()}
</div>
