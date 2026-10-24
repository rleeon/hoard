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
   *
   * At Halloween the H grows a pumpkin's stem and leaf off its left post.
   * They stand above the box, which `overflow` lets them do.
   */
  import { halloween } from "../stores/theme";

  /** `mono` drops the tile and the gem gradient and draws the H in
   *  `currentColor`. It is for the places where the mark is a piece of
   *  furniture and not the brand: the title bar, where a green tile next to the
   *  window buttons would be the loudest thing on screen. */
  /** `bare` drops only the tile: the green H on its own, cut to the letter so
   *  it can stand as the first letter of the word (the sidebar's "H" + "oard").
   *  `size` is then its height, which the caller matches to the font's cap
   *  height. */
  type Props = { size?: number; class?: string; mono?: boolean; bare?: boolean };
  let { size = 36, class: klass = "", mono = false, bare = false }: Props = $props();

  /** The title bar's H at Halloween: in `mono` it would be the bar's dark
   *  green, and a green H with a stem reads as a sprout, not a pumpkin. */
  const PUMPKIN = "oklch(0.68 0.17 50)";

  // The H spans 22 x 26 units of the 48-unit box.
  const viewBox = $derived(bare ? "13 11 22 26" : mono ? "10 10 28 28" : "0 0 48 48");
  const width = $derived(bare ? (size * 22) / 26 : size);

  // Unique gradient id per instance so multiple logos on a page don't clash.
  const gid = `hoard-h-${Math.random().toString(36).slice(2, 8)}`;
</script>

<!-- In `mono` the tile is gone, so the mark is just the H sitting in the middle
     of a 48-unit box with a lot of air around it: next to a line of text it
     reads as a small letter floating high. Cropping the box to the H itself
     makes the glyph fill its space and sit on the same line as the words. -->
<svg
  {width}
  height={size}
  {viewBox}
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class={klass}
  overflow={$halloween ? "visible" : undefined}
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
  {#if !mono && !bare}
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
  <g fill={mono ? ($halloween ? PUMPKIN : "currentColor") : `url(#${gid})`}>
    <rect x="13" y="11" width="6.5" height="26" rx="1" />
    <rect x="28.5" y="11" width="6.5" height="26" rx="1" />
    <rect x="13" y="21" width="22" height="6" rx="1" />
  </g>
  {#if $halloween}
    <path
      d="M16.2 11.8C16 8.4 17 6 19.8 4.4"
      stroke={mono ? "currentColor" : "oklch(0.48 0.09 130)"}
      stroke-width="3.4"
      stroke-linecap="round"
    />
    <path
      d="M18.6 7.6C20.6 3 26 2.6 28.6 5.4 25.8 8.8 21.2 9.4 18.6 7.6Z"
      fill="oklch(0.6 0.14 150)"
    />
  {/if}
</svg>
