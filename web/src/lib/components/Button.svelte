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
    'group inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors duration-200 ring-focus disabled:cursor-not-allowed disabled:opacity-50 active:brightness-95';

  const variants: Record<Variant, string> = {
    primary: 'bg-accent text-pine hover:bg-emerald-300',
    secondary:
      'border border-line-strong bg-surface text-ink hover:border-ink/30 hover:bg-bg',
    ghost: 'text-ink-soft hover:bg-ink/5 hover:text-ink',
    danger: 'bg-red-700 text-white hover:bg-red-600'
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
  <span class="inline-flex items-center gap-2">
    {@render children()}
  </span>
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
