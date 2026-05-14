<script lang="ts">
  /**
   * One toggleable setting in the Settings page.
   *
   * The shape is fixed (icon + label/description + switch) so the page reads
   * as a uniform list — different visuals per row would make scanning the
   * page slower. The switch is a styled checkbox; we don't depend on a third
   * party UI kit for something this small.
   */
  type Props = {
    row: {
      field: string;
      label: string;
      description: string;
      icon: any;
    };
    value: boolean;
    disabled?: boolean;
    onChange: (next: boolean) => void;
  };

  let { row, value, disabled = false, onChange }: Props = $props();
  // Reactive alias so the icon swaps if the row prop ever changes. Capturing
  // it as `const` would freeze it to the initial render — which Svelte warns
  // about explicitly, and which would bite future callers.
  const Icon = $derived(row.icon);
</script>

<label
  class="flex items-start gap-4 py-4 first:pt-0 last:pb-0
    {disabled ? 'opacity-60' : 'cursor-pointer'}"
>
  <div
    class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-zinc-800/60 text-zinc-400"
  >
    <Icon size={16} />
  </div>
  <div class="min-w-0 flex-1">
    <div class="text-sm font-medium text-zinc-100">{row.label}</div>
    <p class="mt-1 text-xs text-zinc-400">{row.description}</p>
  </div>
  <span class="relative inline-flex h-5 w-9 shrink-0 items-center">
    <input
      type="checkbox"
      class="peer sr-only"
      checked={value}
      {disabled}
      onchange={(e) => onChange((e.currentTarget as HTMLInputElement).checked)}
    />
    <span
      class="absolute inset-0 rounded-full bg-zinc-700 transition-colors peer-checked:bg-emerald-600/90
        peer-disabled:bg-zinc-800"
    ></span>
    <span
      class="absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-zinc-100 shadow transition-transform
        peer-checked:translate-x-4 peer-disabled:bg-zinc-500"
    ></span>
  </span>
</label>
