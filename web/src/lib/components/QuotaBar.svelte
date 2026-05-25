<script lang="ts">
  interface Props {
    used: number;
    total: number;
    label?: string;
    formatted?: string;
  }
  let { used, total, label, formatted }: Props = $props();

  let pct = $derived(total > 0 ? Math.min(100, (used / total) * 100) : 0);
  let color = $derived(
    pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-amber-500' : 'bg-emerald-500'
  );
</script>

<div class="space-y-1.5">
  {#if label}
    <div class="flex items-baseline justify-between text-sm">
      <span class="text-zinc-300">{label}</span>
      {#if formatted}<span class="font-medium text-zinc-200 tabular-nums">{formatted}</span>{/if}
    </div>
  {/if}
  <div class="h-2 w-full overflow-hidden rounded-full bg-zinc-800/80">
    <div
      class="h-full rounded-full {color} transition-all duration-700 ease-out"
      style="width: {pct}%"
    ></div>
  </div>
  <div class="text-right text-xs text-zinc-500 tabular-nums">{pct.toFixed(1)}%</div>
</div>
