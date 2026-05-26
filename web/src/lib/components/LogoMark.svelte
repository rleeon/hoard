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
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.18" />
      <stop offset="0.7" stop-color="#ffffff" stop-opacity="0" />
    </radialGradient>
  </defs>
  <rect width="64" height="64" rx="14" fill="#0a0c0b" />
  <rect width="64" height="64" rx="14" fill="url(#hoard-mark-shine)" />
  <path
    d="M16 14 h8 v14 h16 v-14 h8 v36 h-8 v-14 h-16 v14 h-8 z"
    fill="url(#hoard-mark-grad)"
  />
</svg>
