<script lang="ts">
  // Hand-drawn underline squiggle. Animates stroke-dashoffset to draw
  // itself on first paint. Pass color, width, and stroke thickness.
  interface Props {
    width?: number;
    height?: number;
    stroke?: string;
    thickness?: number;
    class?: string;
  }
  let {
    width = 260,
    height = 14,
    stroke = '#34d399',
    thickness = 3,
    class: extra = ''
  }: Props = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox={`0 0 ${width} ${height}`}
  width={width}
  height={height}
  aria-hidden="true"
  class={extra}
>
  <path
    d={`M2 ${height - 4} Q ${width * 0.18} ${height - 12}, ${width * 0.35} ${height - 6} T ${width * 0.7} ${height - 6} T ${width - 2} ${height - 5}`}
    fill="none"
    stroke={stroke}
    stroke-width={thickness}
    stroke-linecap="round"
    class="squiggle"
  />
</svg>
