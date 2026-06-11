<script lang="ts">
  interface Props {
    used: number;
    total: number;
    label?: string;
    formatted?: string;
  }
  let { used, total, label, formatted }: Props = $props();

  let pct = $derived(total > 0 ? Math.min(100, (used / total) * 100) : 0);
  let palette = $derived(
    pct >= 90
      ? { bar: 'bg-red-500', text: 'text-red-400' }
      : pct >= 70
        ? { bar: 'bg-amber-400', text: 'text-amber-400' }
        : { bar: 'bg-accent', text: 'text-accent' }
  );
</script>

<div class="space-y-1.5">
  {#if label}
    <div class="flex items-baseline justify-between text-sm">
      <span class="text-ink-soft">{label}</span>
      {#if formatted}
        <span class="font-medium tabular-nums text-ink">{formatted}</span>
      {/if}
    </div>
  {/if}
  <div class="relative h-2 w-full overflow-hidden rounded-full bg-ink/10">
    <div
      class="absolute inset-y-0 left-0 rounded-full {palette.bar} transition-[width] duration-700 ease-out"
      style="width: {pct}%"
    ></div>
  </div>
  <div class="text-right text-xs tabular-nums {palette.text}">{pct.toFixed(1)}%</div>
</div>
