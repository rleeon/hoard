<script lang="ts">
  /**
   * The account icon: a small Hylian in silhouette, Link's cap curling off to
   * the right, the long pointed ears, and a pair of shoulders under the chin.
   * Filled like its neighbours (the Triforce, the star) because an outline
   * person with a cap and ears on top turned into a smudge at 18 px; the brim
   * and the eyes are cut out of the fill with a mask, so they read on any
   * background.
   *
   * Lucide's prop type, so it drops in where the house was (`icon: typeof Home`)
   * and keeps the nav's hover pop.
   */
  import type { IconProps } from "@lucide/svelte";

  let {
    size = 24,
    color = "currentColor",
    strokeWidth: _strokeWidth,
    class: klass = "",
    ...rest
  }: IconProps = $props();

  // The icon renders more than once (the nav and the tour), and a mask is
  // looked up by id document-wide.
  const uid = $props.id();
  const cut = `hylian-cut-${uid}`;
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
  <mask id={cut}>
    <rect width="24" height="24" fill="white" />
    <path d="M7.25 10.6h9.5" stroke="black" stroke-width="1.15" />
    <circle cx="10.42" cy="13.06" r=".92" fill="black" />
    <circle cx="13.58" cy="13.06" r=".92" fill="black" />
  </mask>
  <g fill={color} stroke={color} stroke-width="1" stroke-linejoin="round">
    <g mask="url(#{cut})">
      <circle cx="12" cy="12.01" r="4.22" />
      <path d="M7.78 10.6h8.44c.18-2.82 1.76-4.22 3.7-4.58-.7-3.69-4.58-4.22-7.57-2.81-2.99 1.05-4.57 3.69-4.57 7.39Z" />
      <path d="M8.04 11.39 3.46 9.1l4.84 4.32Z" />
      <path d="M15.96 11.39l4.58-2.29-4.84 4.32Z" />
    </g>
    <path d="M6.2 22c0-2.3 2.5-3.9 5.8-3.9s5.8 1.6 5.8 3.9Z" />
  </g>
</svg>
