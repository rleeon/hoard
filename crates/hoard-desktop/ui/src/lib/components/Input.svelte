<script lang="ts">
  /**
   * Form input with an optional label, hint, leading icon, and inline error.
   *
   * Designed for the wizard screens where each field stands on its own. The
   * error message takes priority over the hint when both are present.
   */
  import type { HTMLInputAttributes } from "svelte/elements";

  // Lucide icons in svelte-5 mode have their own Component<...> signature
  // that does not match a generic `Component<{ size?, class? }>` shape, so
  // we keep this `any`-typed and rely on the prop name to document intent.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type IconComponent = any;

  type Props = {
    label?: string;
    hint?: string;
    error?: string | null;
    icon?: IconComponent;
    value?: string;
  } & Omit<HTMLInputAttributes, "value">;

  let {
    label,
    hint,
    error = null,
    icon: Icon,
    value = $bindable(""),
    type = "text",
    id = `input-${Math.random().toString(36).slice(2, 9)}`,
    class: extraClass = "",
    ...rest
  }: Props = $props();
</script>

<div class={`flex flex-col gap-1.5 ${extraClass}`}>
  {#if label}
    <label for={id} class="text-sm font-medium text-zinc-200">{label}</label>
  {/if}

  <div class="relative">
    {#if Icon}
      <span
        class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500"
      >
        <Icon size={16} />
      </span>
    {/if}
    <input
      {id}
      {type}
      bind:value
      class="block w-full rounded-md border bg-zinc-950/40 px-3 py-2.5 text-sm text-zinc-100 placeholder-zinc-500
             outline-none transition-colors focus:ring-1
             {error
        ? 'border-red-500/60 focus:border-red-500 focus:ring-red-500'
        : 'border-zinc-700 focus:border-amber-500 focus:ring-amber-500'}
             {Icon ? 'pl-9' : ''}"
      aria-invalid={error ? "true" : undefined}
      aria-describedby={error ? `${id}-error` : hint ? `${id}-hint` : undefined}
      {...rest}
    />
  </div>

  {#if error}
    <p id={`${id}-error`} class="text-xs text-red-400">{error}</p>
  {:else if hint}
    <p id={`${id}-hint`} class="text-xs text-zinc-500">{hint}</p>
  {/if}
</div>
