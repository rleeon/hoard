<script lang="ts">
  /**
   * Reusable button with three variants and a loading state.
   *
   * - `primary`   — amber, used for the page's main action (one per screen).
   * - `secondary` — neutral, used for "Back" / "Cancel" style actions.
   * - `ghost`     — transparent, used inside cards or as quiet icon buttons.
   */
  import type { Snippet } from "svelte";
  import type {
    HTMLButtonAttributes,
    HTMLAnchorAttributes,
  } from "svelte/elements";
  import { Loader2 } from "lucide-svelte";

  type Variant = "primary" | "secondary" | "ghost";
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
    primary:
      "bg-amber-500 text-zinc-950 hover:bg-amber-400 focus-visible:ring-amber-400 disabled:bg-zinc-700 disabled:text-zinc-400",
    secondary:
      "bg-zinc-800 text-zinc-100 hover:bg-zinc-700 focus-visible:ring-zinc-500 disabled:bg-zinc-900 disabled:text-zinc-600",
    ghost:
      "bg-transparent text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100 focus-visible:ring-zinc-500 disabled:text-zinc-600",
  };

  const sizeClasses: Record<Size, string> = {
    md: "px-4 py-2 text-sm",
    lg: "px-5 py-2.5 text-base",
  };

  const baseClass =
    "inline-flex items-center justify-center gap-2 rounded-md font-medium transition-colors " +
    "focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 " +
    "focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed";
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
