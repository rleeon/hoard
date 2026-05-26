<script lang="ts">
  interface Props {
    size?: number;
    animated?: boolean;
  }
  let { size = 28, animated = false }: Props = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 64 64"
  width={size}
  height={size}
  aria-hidden="true"
  class={animated ? 'transition-transform duration-500' : ''}
>
  <defs>
    <linearGradient id="hoard-mark-grad" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#5eead4" />
      <stop offset="0.55" stop-color="#34d399" />
      <stop offset="1" stop-color="#047857" />
    </linearGradient>
    <radialGradient id="hoard-mark-shine" cx="0.3" cy="0.2" r="0.6">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.45" />
      <stop offset="0.6" stop-color="#ffffff" stop-opacity="0" />
    </radialGradient>
  </defs>
  <path
    d="M16 18 L32 12 L48 18 L48 40 C48 48 40 52 32 52 C24 52 16 48 16 40 Z"
    fill="url(#hoard-mark-grad)"
  />
  <path
    d="M16 18 L32 12 L48 18 L48 40 C48 48 40 52 32 52 C24 52 16 48 16 40 Z"
    fill="url(#hoard-mark-shine)"
  />
  <path
    d="M24 28 L32 24 L40 28 L40 38 C40 42 36 44 32 44 C28 44 24 42 24 38 Z"
    fill="#020403"
    opacity="0.55"
  />
</svg>
