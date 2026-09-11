<script lang="ts">
  /**
   * The Wrapped and automatic-mode icon: the Super Mario star, not any star.
   * Plump (shallow notches between the points), round-tipped (the fill is
   * outlined in its own colour with a round join, which is what softens the
   * points), and the two tall oval eyes that make it that star and no other.
   * The eyes are knocked out in black, like the sprite's.
   *
   * Centred in the 24-unit box; lucide's prop type so it drops in where the
   * sparkles were. Its move is `hop` (a little jump, then a small pop).
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
  <path
    d="M12.00 3.32 L15.23 8.47 L21.13 9.95 L17.23 14.62 L17.64 20.68 L12.00 18.42 L6.36 20.68 L6.77 14.62 L2.87 9.95 L8.77 8.47Z"
    fill={color}
    stroke={color}
    stroke-width="2.4"
    stroke-linejoin="round"
  />
  <ellipse cx="10.15" cy="12.02" rx="1.05" ry="2.25" fill="black" />
  <ellipse cx="13.85" cy="12.02" rx="1.05" ry="2.25" fill="black" />
</svg>
