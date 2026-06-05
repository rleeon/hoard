<script lang="ts">
  /** Visual container with the standard zinc surface + border treatment. */
  import type { Snippet } from "svelte";

  type Props = {
    children: Snippet;
    class?: string;
  };

  let { children, class: extraClass = "" }: Props = $props();
</script>

<!--
  Surface: a translucent obsidian panel lifted off the atmospheric
  background with a hairline top-highlight and a soft drop shadow, so
  cards read as layered glass rather than flat filled boxes.
-->
<div
  class="relative rounded-2xl border border-white/[0.07] bg-zinc-950/40 p-6
         shadow-[0_1px_0_0_rgba(255,255,255,0.04)_inset,0_8px_30px_-12px_rgba(0,0,0,0.7)]
         backdrop-blur-sm {extraClass}"
>
  {@render children()}
</div>
