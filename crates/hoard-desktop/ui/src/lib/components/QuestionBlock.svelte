<script lang="ts">
  /**
   * The Hoard-help icon: the question block. Filled in its own colour like the
   * star next to it in the rail, with the four rivets and the question mark
   * knocked out in black. The mark is strokes, not a glyph: at 18 px a font's
   * "?" is a smudge.
   *
   * Its move is `bump`: the hop a block makes when it is hit from below
   * (`app.css`).
   */
  import type { IconProps } from "@lucide/svelte";

  let {
    size = 24,
    color = "currentColor",
    strokeWidth: _strokeWidth,
    class: klass = "",
    ...rest
  }: IconProps = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class={klass}
  aria-hidden="true"
  {...rest}
>
  <rect x="2.6" y="2.6" width="18.8" height="18.8" rx="3.6" fill={color} />
  <circle cx="5.9" cy="5.9" r="1" fill="black" />
  <circle cx="18.1" cy="5.9" r="1" fill="black" />
  <circle cx="5.9" cy="18.1" r="1" fill="black" />
  <circle cx="18.1" cy="18.1" r="1" fill="black" />
  <path
    d="M9.25 9.7 A2.75 2.75 0 1 1 13.7 11.85 C12.6 12.6 12 13.15 12 14.3"
    stroke="black"
    stroke-width="2.3"
    stroke-linecap="round"
    fill="none"
  />
  <circle cx="12" cy="17.35" r="1.3" fill="black" />
</svg>
