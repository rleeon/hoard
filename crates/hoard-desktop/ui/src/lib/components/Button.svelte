<script lang="ts">
  /**
   * Reusable button with three variants and a loading state.
   *
   * - `primary`   — emerald, used for the page's main action (one per screen).
   * - `secondary` — neutral, used for "Back" / "Cancel" style actions.
   * - `ghost`     — transparent, used inside cards or as quiet icon buttons.
   */
  import type { Snippet } from "svelte";
  import type {
    HTMLButtonAttributes,
    HTMLAnchorAttributes,
  } from "svelte/elements";
  import { Loader2 } from "lucide-svelte";

  type Variant = "primary" | "secondary" | "ghost" | "danger";
  type Size = "md" | "lg";

  type Props = {
    variant?: Variant;
    size?: Size;
    loading?: boolean;
    href?: string;
    children: Snippet;
  } & HTMLButtonAttributes &
    HTMLAnchorAttributes;

  let {
    variant = "primary",
    size = "md",
    loading = false,
    href,
    disabled,
    children,
    type = "button",
    class: extraClass = "",
    ...rest
  }: Props = $props();

  const variantClasses: Record<Variant, string> = {
    // Primary = the gem. Emerald with a faint inner top-light and an
    // outer glow that intensifies on hover.
    primary:
      "bg-emerald-600 text-emerald-50 shadow-[0_1px_0_0_rgba(255,255,255,0.18)_inset,0_6px_20px_-8px_rgba(16,185,129,0.55)] " +
      "hover:bg-emerald-500 hover:shadow-[0_1px_0_0_rgba(255,255,255,0.22)_inset,0_8px_26px_-6px_rgba(16,185,129,0.75)] " +
      "focus-visible:ring-emerald-400/70 disabled:bg-zinc-800 disabled:text-zinc-500 disabled:shadow-none",
    secondary:
      "border border-white/[0.08] bg-white/[0.04] text-zinc-100 hover:bg-white/[0.08] hover:border-white/[0.14] " +
      "focus-visible:ring-zinc-500 disabled:bg-transparent disabled:text-zinc-600",
    ghost:
      "bg-transparent text-zinc-300 hover:bg-white/[0.06] hover:text-zinc-50 focus-visible:ring-zinc-500 disabled:text-zinc-600",
    danger:
      "bg-red-600 text-red-50 shadow-[0_1px_0_0_rgba(255,255,255,0.15)_inset,0_6px_20px_-8px_rgba(239,68,68,0.5)] " +
      "hover:bg-red-500 focus-visible:ring-red-400/70 disabled:bg-zinc-800 disabled:text-zinc-500 disabled:shadow-none",
  };

  const sizeClasses: Record<Size, string> = {
    md: "px-4 py-2 text-sm",
    lg: "px-5 py-2.5 text-base",
  };

  const baseClass =
    "inline-flex items-center justify-center gap-2 rounded-lg font-medium tracking-[-0.005em] " +
    "transition-all duration-150 active:scale-[0.98] " +
    "focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 " +
    "focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:active:scale-100";
</script>

{#if href}
  <a
    {href}
    class="{baseClass} {variantClasses[variant]} {sizeClasses[size]} {extraClass}"
    aria-disabled={disabled || loading ? "true" : undefined}
    {...rest}
  >
    {#if loading}
      <Loader2 size={16} class="animate-spin" />
    {/if}
    {@render children()}
  </a>
{:else}
  <button
    {type}
    class="{baseClass} {variantClasses[variant]} {sizeClasses[size]} {extraClass}"
    disabled={disabled || loading}
    {...rest}
  >
    {#if loading}
      <Loader2 size={16} class="animate-spin" />
    {/if}
    {@render children()}
  </button>
{/if}
