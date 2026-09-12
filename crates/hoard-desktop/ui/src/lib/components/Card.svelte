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
<!--
  El fondo, el filo, el radio y la sombra ya no se escriben aquí: los pone
  `.panel` (app.css), la misma superficie que usa el resto de paneles. Antes
  esta tarjeta tenía su propio `border-white/[0.09]` mientras el panel de
  estadísticas del Panel usaba `border-white/[0.08]` y la barra lateral
  `border-zinc-800/60`: tres filos distintos para la misma idea.
-->
<div class="card-glass panel relative p-6 backdrop-blur-xl {extraClass}">
  {@render children()}
</div>
