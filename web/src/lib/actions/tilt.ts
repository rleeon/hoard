/**
 * 3D tilt action, the element leans toward the cursor on hover and a soft
 * directional glow follows the pointer. Ported from the desktop app
 * (crates/hoard-desktop/ui/src/lib/actions/tilt.ts) so the site and the app
 * share the same physical feel.
 *
 * The transform + glow are driven by CSS variables this action sets
 * (--tilt-rx/--tilt-ry and --tilt-glow-x/--tilt-glow-y); the `.tilt` class in
 * app.css consumes them, so the action itself stays transform-free (cheap to
 * attach/teardown). It no-ops when the OS asks for reduced motion.
 *
 * Anti-flicker: the CSS transform skews the bounding box so `mouseleave`
 * fires when the cursor is still visually on the card. We listen for
 * `pointermove` on the document and only reset when the pointer truly
 * exits the element's original (un-transformed) rect.
 */
export function tilt(node: HTMLElement, opts: { max?: number } = {}) {
  if (typeof window === 'undefined') return {};

  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reducedMotion) return {};

  const max = opts.max ?? 6;

  let raf = 0;
  let active = false;

  /** Original (un-transformed) bounding rect, captured on enter. */
  let rect: DOMRect | null = null;

  function onMove(e: PointerEvent) {
    if (!active || !rect) return;
    const px = (e.clientX - rect.left) / rect.width; // 0..1
    const py = (e.clientY - rect.top) / rect.height;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      node.style.setProperty('--tilt-ry', `${(px - 0.5) * 2 * max}deg`);
      node.style.setProperty('--tilt-rx', `${(0.5 - py) * 2 * max}deg`);
      node.style.setProperty('--tilt-glow-x', `${(px * 100).toFixed(1)}%`);
      node.style.setProperty('--tilt-glow-y', `${(py * 100).toFixed(1)}%`);
    });
  }

  function reset() {
    active = false;
    rect = null;
    cancelAnimationFrame(raf);
    node.style.setProperty('--tilt-ry', '0deg');
    node.style.setProperty('--tilt-rx', '0deg');
  }

  function onEnter(e: PointerEvent) {
    rect = node.getBoundingClientRect();
    active = true;
    onMove(e);
  }

  function onDocMove(e: PointerEvent) {
    if (!active || !rect) return;
    const inBounds =
      e.clientX >= rect.left &&
      e.clientX <= rect.right &&
      e.clientY >= rect.top &&
      e.clientY <= rect.bottom;
    if (inBounds) onMove(e);
    else reset();
  }

  function onDocUp() {
    if (active) rect = node.getBoundingClientRect();
  }

  node.addEventListener('pointerenter', onEnter);
  document.addEventListener('pointermove', onDocMove);
  document.addEventListener('pointerup', onDocUp);
  return {
    destroy() {
      node.removeEventListener('pointerenter', onEnter);
      document.removeEventListener('pointermove', onDocMove);
      document.removeEventListener('pointerup', onDocUp);
      cancelAnimationFrame(raf);
    }
  };
}
