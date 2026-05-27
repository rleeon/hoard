<script lang="ts">
  import { _ } from 'svelte-i18n';

  const metrics = [
    { value: '12,394', label: 'metrics.snapshots' },
    { value: '99.99%', label: 'metrics.uptime' },
    { value: '10+', label: 'metrics.platforms' },
    { value: '100%', label: 'metrics.verified' }
  ];
</script>

<div
  class="grid grid-cols-2 gap-px overflow-hidden rounded-2xl border border-white/[0.06] bg-white/[0.02] sm:grid-cols-4"
>
  {#each metrics as m (m.label)}
    <div class="group relative bg-zinc-950/40 p-5 transition-colors hover:bg-white/[0.025]">
      <div
        class="font-mono text-2xl font-semibold tabular-nums tracking-tight text-white sm:text-3xl"
      >
        {m.value}
      </div>
      <div class="mt-1 text-xs uppercase tracking-wider text-zinc-500 sm:text-[11px]">
        {$_(m.label)}
      </div>
      <span
        class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-transparent via-emerald-500/50 to-transparent opacity-0 transition-opacity duration-500 group-hover:opacity-100"
        aria-hidden="true"
      ></span>
    </div>
  {/each}
</div>
