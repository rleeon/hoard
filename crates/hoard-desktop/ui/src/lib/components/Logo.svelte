<script lang="ts">
  /**
   * Hoard mark, the green gradient "H" inside a dark rounded tile, matching
   * the app icon (`crates/hoard-desktop/icons/icon.png`). Replaces the generic
   * lucide `Archive` box that used to stand in for the logo in the sidebar.
   *
   * Drawn as inline SVG (rather than shipping the PNG into the webview bundle)
   * so it stays crisp at any size, and so the gem can be recoloured: the two
   * stops and the tile's ring read `--logo-gem-*`, which the accent picker in
   * Settings repoints to the user's chosen hue. The tile itself stays
   * near-black on every theme, it's the mark, not a surface.
   */
  /** `mono` drops the tile and the gem gradient and draws the H in
   *  `currentColor`. It is for the places where the mark is a piece of
   *  furniture and not the brand: the title bar, where a green tile next to the
   *  window buttons would be the loudest thing on screen. */
  type Props = { size?: number; class?: string; mono?: boolean };
  let { size = 36, class: klass = "", mono = false }: Props = $props();

  // Unique gradient id per instance so multiple logos on a page don't clash.
  const gid = `hoard-h-${Math.random().toString(36).slice(2, 8)}`;
</script>

<!-- In `mono` the tile is gone, so the mark is just the H sitting in the middle
     of a 48-unit box with a lot of air around it: next to a line of text it
     reads as a small letter floating high. Cropping the box to the H itself
     makes the glyph fill its space and sit on the same line as the words. -->
<svg
  width={size}
  height={size}
  viewBox={mono ? "10 10 28 28" : "0 0 48 48"}
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class={klass}
  role="img"
  aria-label="Hoard"
>
  <defs>
    <linearGradient
      id={gid}
      x1="14"
      y1="10"
      x2="34"
      y2="38"
      gradientUnits="userSpaceOnUse"
    >
      <stop stop-color="var(--logo-gem-from)" />
      <stop offset="1" stop-color="var(--logo-gem-to)" />
    </linearGradient>
  </defs>
  {#if !mono}
    <!-- Dark rounded tile -->
    <rect x="1" y="1" width="46" height="46" rx="12" fill="#0a0a0a" />
    <rect
      x="1"
      y="1"
      width="46"
      height="46"
      rx="12"
      stroke="var(--logo-gem-ring)"
      stroke-opacity="0.25"
      stroke-width="1"
    />
  {/if}
  <!-- The H: two posts + crossbar -->
  <g fill={mono ? "currentColor" : `url(#${gid})`}>
    <rect x="13" y="11" width="6.5" height="26" rx="1" />
    <rect x="28.5" y="11" width="6.5" height="26" rx="1" />
    <rect x="13" y="21" width="22" height="6" rx="1" />
  </g>
</svg>
