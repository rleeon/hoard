<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { api } from '$lib/api';

  let state = $state<'loading' | 'ok' | 'down'>('loading');

  onMount(async () => {
    const r = await api.serverHealth();
    state = r.ok ? 'ok' : 'down';
  });

  let label = $derived(
    state === 'loading'
      ? $_('hero.status_offline')
      : state === 'ok'
        ? $_('hero.status_online')
        : $_('hero.status_degraded')
  );
  let dotClass = $derived(
    state === 'loading'
      ? 'bg-zinc-500'
      : state === 'ok'
        ? 'bg-emerald-400 animate-pulse-glow'
        : 'bg-amber-400'
  );
</script>

<a
  href="/help"
  class="inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/60 px-3 py-1 text-xs text-zinc-300 backdrop-blur hover:border-zinc-700 hover:text-zinc-100"
>
  <span class="h-1.5 w-1.5 rounded-full {dotClass}"></span>
  {label}
</a>
