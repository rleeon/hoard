<script lang="ts">
  /**
   * An icon that reacts when the thing it stands for changes: the bell rings,
   * the sync arrow turns, the scroll unrolls.
   *
   * The animation is a class the markup derives from the state, not something
   * an effect pokes into the DOM. Two earlier goes did it imperatively (a
   * Svelte action, then an `$effect`) and neither fired on a plain toggle: the
   * icon swapped shape, because that is ordinary rendering, and nothing moved.
   * A derived class cannot miss, since it rides the same render that already
   * works.
   *
   * The first paint stays still, so opening Settings does not set a dozen icons
   * off at once.
   */
  export type AnimKind = "spin" | "pop" | "ring" | "unfurl" | "hop";

  type Props = {
    icon: any;
    /** Drawn while `on` is false, when the off state has a shape of its own
     *  (a crossed-out bell, a rolled-up scroll). */
    iconOff?: any;
    on: boolean;
    kind?: AnimKind;
    /** Play it when it turns off as well, which is the default: switching
     *  something off is as much an answer as switching it on. Pass `false` to
     *  keep the off state still. */
    bothWays?: boolean;
    size?: number;
    class?: string;
  };

  let {
    icon,
    iconOff,
    on,
    kind = "pop",
    bothWays = true,
    size = 16,
    class: klass = "",
  }: Props = $props();

  const Icon = $derived(!on && iconOff ? iconOff : icon);

  // Every change of `on` bumps a counter, and the span below is keyed on it, so
  // each change mounts a fresh element that plays the animation from zero. The
  // earlier version kept the class on the element and swapped it for a twin
  // (`-alt`) to replay; a browser does not restart an animation whose
  // `animation-name` did not change, so nothing moved after the first time.
  // Plain variables on purpose: reading and writing runes here would loop.
  let lastOn: boolean | undefined;
  let changes = 0;
  const pulse = $derived.by(() => {
    const now = on;
    if (lastOn !== undefined && now !== lastOn) changes += 1;
    lastOn = now;
    return changes;
  });

  // Counter at 0 is the first paint: still, so opening a page does not set
  // every icon off at once. The class comes off when the animation ends, so it
  // never sits on the element and blocks the hover rule below from replaying it.
  let endedFor = $state(-1);
  const animClass = $derived(
    pulse === 0 ||
      endedFor === pulse ||
      (!on && !bothWays)
      ? ""
      : `icon-anim-${kind}`,
  );
</script>

<!-- `data-anim` names the move for the hover rule in `app.css`, which is the
     other half of the same contract: every icon moves when its state flips and
     when the pointer arrives, and always with its own move. -->
{#key pulse}
  <span
    data-anim={kind}
    onanimationend={() => (endedFor = pulse)}
    class="inline-flex items-center justify-center {animClass} {klass}"
  >
    <Icon size={size} />
  </span>
{/key}
