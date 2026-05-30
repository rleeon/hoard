<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { health } from '$lib/stores/health';

  onMount(() => {
    void health.ensure();
  });

  let label = $derived(
    $health === 'loading'
      ? $_('hero.status_offline')
      : $health === 'ok'
        ? $_('hero.status_online')
        : $health === 'degraded'
          ? $_('hero.status_degraded')
          : $_('hero.status_down')
  );
  let dotClass = $derived(
    $health === 'loading'
      ? 'bg-zinc-500'
      : $health === 'ok'
        ? 'bg-emerald-400 shadow-[0_0_10px_rgba(16,185,129,0.7)] animate-pulse-glow'
        : $health === 'degraded'
          ? 'bg-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.55)]'
          : 'bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.6)]'
  );
  let borderClass = $derived(
    $health === 'ok'
      ? 'border-emerald-400/30 hover:border-emerald-400/55'
      : $health === 'degraded'
        ? 'border-amber-400/30 hover:border-amber-400/55'
        : $health === 'down'
          ? 'border-red-500/30 hover:border-red-500/55'
          : 'border-white/10 hover:border-white/20'
  );
</script>

<a
  href="/help"
  class="inline-flex items-center gap-2 rounded-full border bg-white/[0.03] px-3 py-1 text-xs text-zinc-300 backdrop-blur ring-focus transition-all hover:bg-white/[0.06] hover:text-white {borderClass}"
>
  <span class="relative grid h-2 w-2 place-items-center">
    <span class="h-1.5 w-1.5 rounded-full {dotClass} transition-all duration-500"></span>
  </span>
  {label}
</a>
