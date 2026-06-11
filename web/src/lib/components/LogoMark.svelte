<script lang="ts">
  interface Props {
    size?: number;
  }
  let { size = 28 }: Props = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 64 64"
  width={size}
  height={size}
  aria-hidden="true"
>
  <rect
    x="1"
    y="1"
    width="62"
    height="62"
    rx="13"
    fill="var(--color-pine-elev, #19231c)"
    stroke="var(--color-pine-line, #28332b)"
    stroke-width="2"
  />
  <path d="M12 8 h9 v16 h22 v-16 h9 v48 h-9 v-16 h-22 v16 h-9 z" fill="#34d399" />
</svg>
