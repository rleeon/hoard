<script lang="ts">
  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    href?: string;
    variant?: Variant;
    size?: Size;
    disabled?: boolean;
    loading?: boolean;
    type?: 'button' | 'submit' | 'reset';
    target?: string;
    rel?: string;
    ariaLabel?: string;
    onclick?: (e: MouseEvent) => void;
    children: import('svelte').Snippet;
    full?: boolean;
  }

  let {
    href,
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    type = 'button',
    target,
    rel,
    ariaLabel,
    onclick,
    children,
    full = false
  }: Props = $props();

  const base =
    'group relative inline-flex items-center justify-center gap-2 overflow-hidden rounded-lg font-medium transition-all duration-300 ring-focus disabled:cursor-not-allowed disabled:opacity-50 active:scale-[0.985]';

  const variants: Record<Variant, string> = {
    primary:
      'bg-emerald-600 text-white shadow-[0_8px_24px_-10px_rgba(16,185,129,0.65)] hover:bg-emerald-500 hover:shadow-[0_14px_36px_-12px_rgba(16,185,129,0.85)]',
    secondary:
      'border border-white/10 bg-white/[0.04] text-zinc-100 hover:border-white/20 hover:bg-white/[0.08]',
    ghost: 'text-zinc-300 hover:bg-white/[0.05] hover:text-white',
    danger:
      'bg-red-600 text-white shadow-[0_8px_24px_-10px_rgba(220,38,38,0.55)] hover:bg-red-500 hover:shadow-[0_14px_36px_-12px_rgba(220,38,38,0.7)]'
  };

  const sizes: Record<Size, string> = {
    sm: 'h-8 px-3 text-sm',
    md: 'h-10 px-4 text-sm',
    lg: 'h-12 px-6 text-base'
  };

  let classes = $derived(
    `${base} ${variants[variant]} ${sizes[size]} ${full ? 'w-full' : ''}`
  );

  let isExternal = $derived(!!href && /^https?:\/\//i.test(href));
  let resolvedRel = $derived(
    rel ?? (target === '_blank' || isExternal ? 'noopener noreferrer' : undefined)
  );
</script>

{#snippet body()}
  {#if loading}
    <span
      class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
      aria-hidden="true"
    ></span>
  {/if}
  <span class="relative z-10 inline-flex items-center gap-2">
    {@render children()}
  </span>
  {#if variant === 'primary'}
    <span
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 -translate-x-[120%] bg-gradient-to-r from-transparent via-white/30 to-transparent transition-transform duration-700 group-hover:translate-x-[120%]"
    ></span>
  {/if}
{/snippet}

{#if href}
  <a
    {href}
    {target}
    rel={resolvedRel}
    class={classes}
    role="button"
    aria-label={ariaLabel}
    aria-disabled={disabled}
    tabindex={disabled ? -1 : 0}
  >
    {@render body()}
  </a>
{:else}
  <button
    {type}
    disabled={disabled || loading}
    {onclick}
    class={classes}
    aria-label={ariaLabel}
    aria-busy={loading}
  >
    {@render body()}
  </button>
{/if}
