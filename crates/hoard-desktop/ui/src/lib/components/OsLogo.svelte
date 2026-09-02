<script lang="ts">
  /**
   * Real OS logos as inline SVG, monochrome, inherits `currentColor` so the
   * caller controls the tint (green when online, grey when offline).
   *
   * No icon font, no external assets, the paths are baked in. Each logo is
   * drawn at 16×16 by default and scaled via the `size` prop.
   *
   * Sources: the official Windows 4-squares mark, the Linux/Tux penguin
   * silhouette, and the Apple logo. Simplified to single-path silhouettes
   * so they render crisp at small sizes and stay one colour.
   */
  type Os = "windows" | "linux" | "macos" | "unknown";

  let {
    os,
    size = 16,
    class: klass = "",
  }: { os: Os; size?: number; class?: string } = $props();
</script>

{#if os === "windows"}
  <!-- Windows: four squares (the modern 4-pane mark) -->
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    class={klass}
    aria-hidden="true"
  >
    <path d="M3 5.5L10.5 4.4V11.3H3V5.5ZM3 12.7H10.5V19.6L3 18.5V12.7ZM11.7 4.2L21 3V11.3H11.7V4.2ZM11.7 12.7H21V21L11.7 19.8V12.7Z" />
  </svg>
{:else if os === "linux"}
  <!-- Linux: Tux penguin silhouette -->
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    class={klass}
    aria-hidden="true"
  >
    <path d="M12 2C10.3 2 9 3.3 9 5c0 .5.1 1 .3 1.4C7.8 7.5 6.5 9.7 6 12c-.7 3-1 5-2 6.5-.3.5-.5 1-.5 1.5 0 .8.6 1.5 1.5 1.8-.2.3-.3.6-.3.9 0 .5.4.8 1 .8h7.3c.6 0 1-.3 1-.8 0-.3-.1-.6-.3-.9.9-.3 1.5-1 1.5-1.8 0-.5-.2-1-.5-1.5-1-1.5-1.3-3.5-2-6.5-.5-2.3-1.8-4.5-3.3-5.6.2-.4.3-.9.3-1.4 0-1.7-1.3-3-3-3Zm-1.2 5.8c.3 0 .5.2.5.5s-.2.5-.5.5-.5-.2-.5-.5.2-.5.5-.5Zm2.4 0c.3 0 .5.2.5.5s-.2.5-.5.5-.5-.2-.5-.5.2-.5.5-.5Z" />
  </svg>
{:else if os === "macos"}
  <!-- macOS: Apple logo silhouette -->
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    class={klass}
    aria-hidden="true"
  >
    <path d="M17.05 12.04c-.03-2.3 1.88-3.41 1.97-3.46-1.07-1.57-2.74-1.79-3.33-1.81-1.42-.14-2.77.83-3.49.83-.72 0-1.74-.82-2.86-.79-1.47.02-2.83.85-3.59 2.17-1.53 2.65-.39 6.58 1.1 8.74.73 1.06 1.6 2.25 2.74 2.21 1.1-.04 1.52-.71 2.85-.71 1.33 0 1.7.71 2.86.69 1.18-.02 1.93-1.07 2.65-2.14.84-1.23 1.18-2.42 1.2-2.48-.03-.01-2.29-.88-2.31-3.48zM14.9 5.42c.61-.73 1.02-1.75.91-2.77-.88.03-1.94.58-2.57 1.31-.57.65-1.06 1.69-.93 2.69.98.08 1.98-.5 2.59-1.23z" />
  </svg>
{:else}
  <!-- Unknown: a simple monitor glyph -->
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    class={klass}
    aria-hidden="true"
  >
    <path d="M3 4h18v12H3V4zm1 1v10h16V5H4zm6 13h4l1 2H9l1-2z" />
  </svg>
{/if}
