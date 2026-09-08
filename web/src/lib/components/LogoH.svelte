<script lang="ts">
  /** The mark on its own: the H, drawn as three bars, sized by cap height. */
  interface Props {
    height?: number;
    color?: string;
    class?: string;
  }
  let { height = 18, color = 'var(--color-accent)', class: klass = '' }: Props = $props();
</script>

<svg
  viewBox="0 0 33 44"
  {height}
  width={(height * 33) / 44}
  fill={color}
  aria-hidden="true"
  class={klass}
>
  <rect x="0" y="0" width="9" height="44" />
  <rect x="24" y="0" width="9" height="44" />
  <rect x="0" y="17" width="33" height="9" />
</svg>
