<script lang="ts">
  /**
   * The hearts in the two plan dialogs: drawn, never an emoji.
   *
   * An emoji is painted by the system font, so the same character comes out flat on
   * Linux, glossy on Windows and a different colour on each, and none of them is the
   * application's colour. This is geometry: a `path` with a gradient, three hearts in
   * a cluster, and the same stroke on both faces (`broken` only splits each into two
   * halves separated by a crack and tilts them).
   *
   * The halves are rotated with the SVG's `transform` attribute and not with CSS:
   * CSS's `transform` *replaces* the attribute's on the same element, so the
   * heartbeat (which is CSS, in its own `<g>`) and the tilt live on different layers
   * so they cannot tread on each other. The heartbeat is decoration: under
   * `prefers-reduced-motion` the global rule in `app.css` freezes it and the drawing
   * stays still, which is exactly what should happen.
   */

  type Props = {
    /** Corazones partidos (despedida) en vez de enteros (agradecimiento). */
    broken?: boolean;
    /** Width in pixels. The height comes from the `viewBox`'s ratio. */
    width?: number;
    class?: string;
  };

  let { broken = false, width = 168, class: extraClass = "" }: Props = $props();

  /** A heart in a 24×24 box, point down at (12, 21.6). */
  const HEART =
    "M12 21.6C8.6 18.6 3 14.4 3 9.6C3 6.5 5.4 4.2 8.3 4.2" +
    "C10.1 4.2 11.4 5.1 12 6.3C12.6 5.1 13.9 4.2 15.7 4.2" +
    "C18.6 4.2 21 6.5 21 9.6C21 14.4 15.4 18.6 12 21.6Z";

  /** The crack: it zigzags down from the top notch to the point. The two masks are
   *  the same route walked backwards, so the halves fit without overlapping. */
  const CRACK_L = "0,0 12,0 12,5.2 10.3,9.6 13.5,12.2 10.7,16.2 12,21.6 12,24 0,24";
  const CRACK_R = "12,0 24,0 24,24 12,24 12,21.6 10.7,16.2 13.5,12.2 10.3,9.6 12,5.2";

  // Unique ids per instance: two dialogs mounted at once would share `url(#...)`
  // and the second would end up with no gradient.
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
  /* The heartbeat lives in its own `<g>`, with no `transform` attribute, so CSS's
     `transform` does not erase the outer group's placement. */
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
