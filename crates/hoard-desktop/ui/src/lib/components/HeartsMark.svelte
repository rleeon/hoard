<script lang="ts">
  /**
   * Los corazones de los dos diálogos de plan: dibujados, nunca un emoji.
   *
   * Un emoji lo pinta la fuente del sistema, así que el mismo carácter sale
   * plano en Linux, brillante en Windows y de otro color en cada uno, y
   * ninguno de ellos es el color de la aplicación. Esto es geometría: un
   * `path` con degradado, tres corazones en racimo, y el mismo trazo en las dos
   * caras (`broken` sólo parte cada uno en dos mitades separadas por una grieta
   * y las inclina).
   *
   * Las mitades se rotan con el atributo `transform` del SVG y no con CSS: el
   * `transform` de CSS *sustituye* al del atributo en el mismo elemento, así
   * que el latido,que sí es CSS, en su propio `<g>`, y la inclinación viven en
   * capas distintas para no pisarse. El latido es decoración: bajo
   * `prefers-reduced-motion` la regla global de `app.css` lo congela y el
   * dibujo se queda quieto, que es exactamente lo que debe pasar.
   */

  type Props = {
    /** Corazones partidos (despedida) en vez de enteros (agradecimiento). */
    broken?: boolean;
    /** Ancho en píxeles. La altura sale de la proporción del `viewBox`. */
    width?: number;
    class?: string;
  };

  let { broken = false, width = 168, class: extraClass = "" }: Props = $props();

  /** Corazón en una caja de 24×24, punta abajo en (12, 21.6). */
  const HEART =
    "M12 21.6C8.6 18.6 3 14.4 3 9.6C3 6.5 5.4 4.2 8.3 4.2" +
    "C10.1 4.2 11.4 5.1 12 6.3C12.6 5.1 13.9 4.2 15.7 4.2" +
    "C18.6 4.2 21 6.5 21 9.6C21 14.4 15.4 18.6 12 21.6Z";

  /** La grieta: baja del hueco superior a la punta zigzagueando. Las dos
   *  máscaras son el mismo recorrido recorrido al revés, así que las mitades
   *  encajan sin solaparse. */
  const CRACK_L = "0,0 12,0 12,5.2 10.3,9.6 13.5,12.2 10.7,16.2 12,21.6 12,24 0,24";
  const CRACK_R = "12,0 24,0 24,24 12,24 12,21.6 10.7,16.2 13.5,12.2 10.3,9.6 12,5.2";

  // Ids únicos por instancia: dos diálogos montados a la vez compartirían
  // `url(#…)` y el segundo se quedaría sin degradado.
  const uid = `hearts-${Math.random().toString(36).slice(2, 9)}`;
</script>

<svg
  viewBox="0 0 120 64"
  {width}
  height={(width * 64) / 120}
  fill="none"
  aria-hidden="true"
  class={extraClass}
>
  <defs>
    <linearGradient id="{uid}-fill" x1="0" y1="0" x2="0" y2="1">
      {#if broken}
        <stop offset="0%" stop-color="#a1707d" />
        <stop offset="100%" stop-color="#5f1f33" />
      {:else}
        <stop offset="0%" stop-color="#fda4af" />
        <stop offset="55%" stop-color="#f43f5e" />
        <stop offset="100%" stop-color="#be123c" />
      {/if}
    </linearGradient>
    <clipPath id="{uid}-l"><polygon points={CRACK_L} /></clipPath>
    <clipPath id="{uid}-r"><polygon points={CRACK_R} /></clipPath>
  </defs>

  {#snippet heart(place: string, delay: number, tilt: number)}
    <g transform={place}>
      <g class="beat" style="--beat-delay: {delay}ms">
        {#if broken}
          <g transform="rotate({-tilt} 12 21.6) translate(-0.5 0)">
            <path d={HEART} fill="url(#{uid}-fill)" clip-path="url(#{uid}-l)" />
          </g>
          <g transform="rotate({tilt} 12 21.6) translate(0.5 0)">
            <path d={HEART} fill="url(#{uid}-fill)" clip-path="url(#{uid}-r)" />
          </g>
        {:else}
          <path d={HEART} fill="url(#{uid}-fill)" />
        {/if}
      </g>
    </g>
  {/snippet}

  <g opacity="0.72">
    {@render heart("translate(6 18) scale(0.92) rotate(-14 12 12)", 420, 9)}
    {@render heart("translate(88 14) scale(0.86) rotate(16 12 12)", 760, 8)}
  </g>
  {@render heart("translate(36.6 5) scale(1.95)", 0, 7)}
</svg>

<style>
  /* El latido va en su propio `<g>`, sin `transform` de atributo, para que el
     `transform` de CSS no borre la colocación del grupo de fuera. */
  .beat {
    transform-box: fill-box;
    transform-origin: center;
    animation: beat 2600ms ease-in-out infinite;
    animation-delay: var(--beat-delay, 0ms);
  }

  @keyframes beat {
    0%,
    72%,
    100% {
      transform: scale(1);
    }
    78% {
      transform: scale(1.07);
    }
    84% {
      transform: scale(0.99);
    }
    90% {
      transform: scale(1.04);
    }
  }
</style>
