/**
 * 3D tilt action, the element leans toward the cursor on hover and a soft
 * directional glow follows the pointer. Applied to cards, panels, and covers
 * so the UI feels physical rather than flat.
 *
 * The transform + glow are driven by CSS variables this action sets
 * (--tilt-rx/--tilt-ry and --tilt-glow-x/--tilt-glow-y); the `.tilt` class in
 * app.css consumes them, so the action itself stays transform-free (cheap to
 * attach/teardown). The CSS's global reduced-motion rule flattens transitions
 * automatically when the OS asks for it, we don't skip the listeners here
 * because the glow (opacity-only) is still useful even without the 3D tilt.
 *
 * Anti-flicker: the CSS transform skews the bounding box so `mouseleave`
 * fires when the cursor is still visually on the card. We listen for
 * `pointermove` on the document and only reset when the pointer truly
 * exits the element's original (un-transformed) rect.
 *
 * One shared listener, but several nodes tilting at once
 * ------------------------------------------------------
 * These tilts **nest**: a save card carries `use:tilt` and the cover inside it
 * carries its own, so pointing at the cover moves both, the whole card and the
 * picture on top of it. That is the effect.
 *
 * The original version managed it by hooking `pointermove` and `pointerup` to the
 * document *inside the action*: every element carried its own pair and its own
 * state. It worked, but in the library that is two listeners per game, all of them
 * checking bounds on every pixel the mouse moves, with all but one discarding the
 * event.
 *
 * Reducing it to "one globally active node" does NOT work, and it is an easy mistake
 * to make: entering the cover releases the card and, since the pointer never *exits*
 * it, its `pointerenter` never fires again and it stays dead for the rest of the
 * journey. On the Dashboard, where the cover takes almost the whole card, the effect
 * disappears altogether.
 *
 * So the state is a **set**: every node the pointer has entered and not yet left.
 * Since only the ones stacked under the cursor can be in it, the set is the size of
 * the nesting (two, in practice) and not of the number of cards, which was the
 * point. The document listeners stay at one, shared by every instance, installed
 * with the first and removed with the last.
 */

import { tiltScale } from "../stores/motion";

/**
 * The maximum tilt in degrees on each axis, at full intensity.
 *
 * The trimming does not live here but in the Settings level (`stores/motion.ts`),
 * which multiplies this base: the default leaves it at 4 degrees, half the historic
 * effect. That way the user can get the 8 back without recompiling.
 *
 * Careful reading this number as "how much the card moves": nested tilts add up on
 * screen, so a cover inside a card reaches double. Scaling the base reduces both in
 * the same proportion, which is the point.
 */
const BASE_MAX = 8;

type Armed = {
  /** Rect capturado ANTES de que el transform lo deforme. */
  rect: DOMRect;
  /** THIS node's maximum degrees (a cover can ask for different ones from the card
   *  containing it). */
  max: number;
};

/** Nodos bajo el cursor ahora mismo. Es una cadena de anidamiento, no una
 *  lista de todo lo que hay en pantalla. */
const armed = new Map<HTMLElement, Armed>();
/** Instancias vivas, al llegar a 0 se retiran los listeners compartidos. */
let liveCount = 0;
let raf = 0;
/** The last event waiting to be painted. One rAF serves the whole set, so pointing
 *  at a card with a cover still costs one frame. */
let pending: PointerEvent | null = null;

function schedule(e: PointerEvent): void {
  pending = e;
  if (raf) return;
  raf = requestAnimationFrame(() => {
    raf = 0;
    const ev = pending;
    pending = null;
    if (!ev) return;
    for (const [node, { rect, max }] of armed) {
      const px = (ev.clientX - rect.left) / rect.width; // 0..1
      const py = (ev.clientY - rect.top) / rect.height;
      node.style.setProperty("--tilt-ry", `${(px - 0.5) * 2 * max}deg`);
      node.style.setProperty("--tilt-rx", `${(0.5 - py) * 2 * max}deg`);
      node.style.setProperty("--tilt-glow-x", `${(px * 100).toFixed(1)}%`);
      node.style.setProperty("--tilt-glow-y", `${(py * 100).toFixed(1)}%`);
    }
  });
}

function release(node: HTMLElement): void {
  if (!armed.delete(node)) return;
  node.style.setProperty("--tilt-ry", "0deg");
  node.style.setProperty("--tilt-rx", "0deg");
}

function onDocMove(e: PointerEvent): void {
  if (armed.size === 0) return;
  // Check against the ORIGINAL rects, ignore the transformed bboxes.
  for (const [node, { rect }] of armed) {
    const inBounds =
      e.clientX >= rect.left &&
      e.clientX <= rect.right &&
      e.clientY >= rect.top &&
      e.clientY <= rect.bottom;
    if (!inBounds) release(node);
  }
  if (armed.size > 0) schedule(e);
}

function onDocUp(): void {
  // After a click, re-measure so stale bounds from before a layout shift
  // don't linger.
  for (const [node, state] of armed) {
    state.rect = node.getBoundingClientRect();
  }
}

export function tilt(node: HTMLElement, opts: { max?: number } = {}) {
  const base = opts.max ?? BASE_MAX;

  function onEnter(e: PointerEvent) {
    // The level is consulted on enter, not on mount: changing it in Settings takes
    // effect on the next element you point at, with no screen remount. This does NOT
    // look at `prefers-reduced-motion`; the level's initial value decides that (see
    // `stores/motion.ts`), so an explicit choice by the user always wins.
    const max = base * tiltScale();
    armed.set(node, { rect: node.getBoundingClientRect(), max });
    schedule(e);
  }

  node.addEventListener("pointerenter", onEnter);
  if (liveCount === 0) {
    document.addEventListener("pointermove", onDocMove);
    document.addEventListener("pointerup", onDocUp);
  }
  liveCount += 1;

  return {
    destroy() {
      node.removeEventListener("pointerenter", onEnter);
      armed.delete(node);
      liveCount -= 1;
      if (liveCount === 0) {
        document.removeEventListener("pointermove", onDocMove);
        document.removeEventListener("pointerup", onDocUp);
        cancelAnimationFrame(raf);
        raf = 0;
        pending = null;
      }
    },
  };
}
