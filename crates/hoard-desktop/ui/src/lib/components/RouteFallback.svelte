<script lang="ts">
  // The placeholder the router shows while a route's chunk loads.
  //
  // It is deliberately invisible at first: navigating to an already-cached chunk
  // resolves in a microtask, and without the delay the spinner would flash on
  // *every* page change. With it, it only appears when the load takes long enough
  // that showing nothing would feel odd.
</script>

<div class="route-fallback flex h-full items-center justify-center bg-zinc-950">
  <div
    class="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-emerald-500"
  ></div>
</div>

<style>
  .route-fallback {
    opacity: 0;
    animation: route-fallback-in 120ms ease-out 200ms forwards;
  }

  @keyframes route-fallback-in {
    to {
      opacity: 1;
    }
  }

  /* Honours anybody asking for less motion: no fade, but the delay is kept so fast
     navigations still do not flash. */
  @media (prefers-reduced-motion: reduce) {
    .route-fallback {
      animation: route-fallback-in 0ms linear 200ms forwards;
    }
  }
</style>
