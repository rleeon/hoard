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
      ? { bar: 'from-red-500 to-red-400', glow: 'shadow-[0_0_18px_rgba(239,68,68,0.45)]', text: 'text-red-300' }
      : pct >= 70
        ? {
            bar: 'from-amber-500 to-amber-300',
            glow: 'shadow-[0_0_18px_rgba(245,158,11,0.4)]',
            text: 'text-amber-200'
          }
        : {
            bar: 'from-emerald-500 to-teal-300',
            glow: 'shadow-[0_0_18px_rgba(16,185,129,0.45)]',
            text: 'text-emerald-200'
          }
  );
</script>

<div class="space-y-1.5">
  {#if label}
    <div class="flex items-baseline justify-between text-sm">
      <span class="text-zinc-300">{label}</span>
      {#if formatted}
        <span class="font-medium tabular-nums text-zinc-200">{formatted}</span>
      {/if}
    </div>
  {/if}
  <div class="relative h-2 w-full overflow-hidden rounded-full bg-white/[0.05]">
    <div
      class="absolute inset-y-0 left-0 rounded-full bg-gradient-to-r {palette.bar} {palette.glow} transition-[width] duration-700 ease-out"
      style="width: {pct}%"
    ></div>
  </div>
  <div class="text-right text-xs tabular-nums {palette.text}">{pct.toFixed(1)}%</div>
</div>
