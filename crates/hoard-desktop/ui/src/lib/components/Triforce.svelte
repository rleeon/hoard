<script lang="ts">
  /**
   * The account icon: the Triforce, three solid triangles, two below and one on
   * top, with the empty one in the middle. Each piece is pulled slightly toward
   * its own centre so thin gaps separate them, which is what makes it read as
   * the Triforce and not as one big triangle at 18 px.
   *
   * Equilateral and centred in the 24-unit box (side 20). Lucide's prop type, so
   * it drops in where the house was (`icon: typeof Home`) and keeps the nav's
   * hover pop: `data-anim` and the rest land on the `<svg>`.
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
  <g fill={color} stroke={color} stroke-width="0.4" stroke-linejoin="round">
    <path d="M12.00 3.92 16.50 11.71 7.50 11.71Z" />
    <path d="M7.00 12.58 11.50 20.37 2.50 20.37Z" />
    <path d="M17.00 12.58 21.50 20.37 12.50 20.37Z" />
  </g>
</svg>
