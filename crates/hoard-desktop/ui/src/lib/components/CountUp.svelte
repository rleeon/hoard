<script lang="ts">
  /**
   * Count-up number. Animates from the previously-shown value to `value` with
   * an ease-out curve whenever `value` changes (and on first mount). Renders a
   * tabular-num span so digits don't jitter as they change.
   *
   * Under prefers-reduced-motion it snaps to `value` instantly, the figure is
   * still correct, just not animated.
   */
  type Props = {
    value: number;
    decimals?: number;
    /** Suffix like "%", kept out of the animated span so it never flashes. */
    suffix?: string;
    duration?: number;
  };

  let {
    value,
    decimals = 0,
    suffix = "",
    duration = 800,
  }: Props = $props();

  let display = $state(0);
  let last = 0;
  let raf = 0;

  const reduce =
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

  function frame(t: number, from: number, to: number, t0: number) {
    const p = Math.min(1, (t - t0) / duration);
    const e = 1 - Math.pow(1 - p, 3); // easeOutCubic
    display = from + (to - from) * e;
    if (p < 1) raf = requestAnimationFrame((nt) => frame(nt, from, to, t0));
    else display = to;
  }

  $effect(() => {
    const to = value;
    if (reduce) {
      display = to;
      last = to;
      return;
    }
    cancelAnimationFrame(raf);
    const from = last;
    last = to;
    raf = requestAnimationFrame((t) => frame(t, from, to, t));
    return () => cancelAnimationFrame(raf);
  });
</script>

<span class="tabular">{display.toFixed(decimals)}{suffix}</span>
