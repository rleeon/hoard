<script lang="ts">
  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    href?: string;
    variant?: Variant;
    size?: Size;
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
    target?: string;
    onclick?: (e: MouseEvent) => void;
    children: import('svelte').Snippet;
    full?: boolean;
  }

  let {
    href,
    variant = 'primary',
    size = 'md',
    disabled = false,
    type = 'button',
    target,
    onclick,
    children,
    full = false
  }: Props = $props();

  const base =
    'inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/60 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:opacity-50 disabled:cursor-not-allowed';

  const variants: Record<Variant, string> = {
    primary:
      'bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-900/30 hover:shadow-emerald-700/40 hover:-translate-y-px',
    secondary:
      'bg-zinc-800 hover:bg-zinc-700 text-zinc-100 border border-zinc-700 hover:border-zinc-600',
    ghost: 'text-zinc-300 hover:text-white hover:bg-zinc-800/60',
    danger: 'bg-red-600 hover:bg-red-500 text-white'
  };

  const sizes: Record<Size, string> = {
    sm: 'h-8 px-3 text-sm',
    md: 'h-10 px-4 text-sm',
    lg: 'h-12 px-6 text-base'
  };

  let classes = $derived(
    `${base} ${variants[variant]} ${sizes[size]} ${full ? 'w-full' : ''}`
  );
</script>

{#if href}
  <a {href} {target} class={classes} role="button" aria-disabled={disabled}>
    {@render children()}
  </a>
{:else}
  <button {type} {disabled} {onclick} class={classes}>
    {@render children()}
  </button>
{/if}
