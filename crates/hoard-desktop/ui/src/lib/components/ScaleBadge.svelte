<script lang="ts">
  /**
   * The interface scale, flashed in the bottom-right corner whenever it moves:
   * the slider, the typed field, Ctrl + wheel, Ctrl +/-/0. Only the number, so
   * there is nothing to translate. It lives inside the zoomed page, so it grows
   * and shrinks along with everything else, which is the point.
   */
  import { scaleChanged, uiScale } from "../stores/uiScale";

  const SHOW_MS = 1200;

  let shown = $state(false);

  $effect(() => {
    if ($scaleChanged === 0) return;
    shown = true;
    const t = setTimeout(() => (shown = false), SHOW_MS);
    return () => clearTimeout(t);
  });
</script>

<div
  class="scale-badge pointer-events-none fixed right-4 bottom-4 z-[60] rounded-lg border px-3 py-1.5 text-sm font-medium tabular-nums text-zinc-200"
  class:is-shown={shown}
  aria-hidden="true"
>
  {Math.round($uiScale * 100)}%
</div>

<style>
  /* The durations are `!important` against the reduced-motion rule in
     `app.css`, which flattens every transition: on Windows that query is on for
     anyone with "animation effects" off, and the badge blinked out instead of
     fading. */
  .scale-badge {
    background: oklch(0.09 0 0 / 0.97);
    border-color: oklch(0.21 0 0);
    box-shadow: 0 8px 24px -8px oklch(0 0 0 / 0.8);
    opacity: 0;
    transform: translateY(6px) scale(0.97);
    transition-property: opacity, transform;
    transition-duration: 450ms !important;
    transition-timing-function: ease-out;
  }
  .scale-badge.is-shown {
    opacity: 1;
    transform: none;
    transition-duration: 120ms !important;
  }
</style>
