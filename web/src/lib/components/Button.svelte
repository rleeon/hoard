<script lang="ts">
  type Variant =
    | 'primary'
    | 'secondary'
    | 'outline'
    | 'ghost'
    | 'danger'
    | 'star'
    | 'mono';
  type Size = 'sm' | 'md' | 'lg' | 'xl';

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
    /** Cursor-following highlight, the same one the app's buttons wear. On by
     *  default: every button on the site reacts the same way. */
    glow?: boolean;
    /** Bounce the whole button when the pointer lands on it. Icons inside it
     *  carry their own `data-anim` and bounce with it. */
    pop?: boolean;
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
    full = false,
    glow = true,
    pop = true
  }: Props = $props();

  const base =
    'group anim-host inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors duration-200 ring-focus disabled:cursor-not-allowed disabled:opacity-50 active:brightness-95';

  const variants: Record<Variant, string> = {
    // The calls to action: the desktop app's `outline` class for class, so a
    // button reads the same on the site and in the app. Black in hover too,
    // no green fill and no halo.
    primary:
      'border border-emerald-800/80 bg-black text-emerald-500 hover:border-emerald-600 hover:text-emerald-400',
    secondary:
      'border border-line-strong bg-surface text-ink hover:border-ink/30 hover:bg-bg',
    outline: 'border border-white/25 bg-black text-ink hover:border-white/50',
    ghost: 'text-ink-soft hover:bg-ink/5 hover:text-ink',
    danger: 'bg-red-700 text-white hover:bg-red-600',
    // The GitHub star: pure black so it sits inside the page rather than on it,
    // with the border, the text and the glow all in the one amber. The glow is a
    // colour change, not movement, so it stands even under reduced-motion.
    star: 'border border-amber-400/60 bg-black text-amber-400 shadow-[0_0_0_0_rgba(251,191,36,0)] transition-[color,border-color,box-shadow] hover:border-amber-300 hover:text-amber-300 hover:shadow-[0_0_18px_-2px_rgba(251,191,36,0.45)]',
    // The star's black-and-hairline build without its colour, for sections that
    // keep the emerald out.
    mono: 'border border-white/25 bg-black text-ink hover:border-white/50'
  };

  const sizes: Record<Size, string> = {
    sm: 'h-8 px-3 text-sm',
    md: 'h-10 px-4 text-sm',
    lg: 'h-12 px-6 text-base',
    xl: 'h-14 px-7 text-base'
  };

  let classes = $derived(
    `${base} ${variants[variant]} ${sizes[size]} ${full ? 'w-full' : ''} ${
      glow ? 'glow' : ''
    } ${pop ? 'pop-self' : ''}`
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
