/**
 * Glow action, a cursor-following highlight without the 3D tilt. Used by
 * buttons and small interactive elements where a perspective rotation would
 * feel wrong, but the light that follows the pointer is still a nice touch.
 *
 * Sets the same --tilt-glow-x/--tilt-glow-y CSS variables as `tilt`; the
 * `.glow` class in app.css consumes them. Cheaper than tilt (no transform,
 * no perspective) so it's safe to attach to every button.
 */
export function glow(node: HTMLElement) {
  let raf = 0;
  /** The rect cached on enter. `getBoundingClientRect` forces a synchronous layout
   *  calculation, and calling it on every `mousemove` meant paying for it dozens of
   *  times a second while the cursor crosses a button. A button's size does not
   *  change while you point at it, so measuring on enter is enough. */
  let rect: DOMRect | null = null;

  function onEnter() {
    rect = node.getBoundingClientRect();
  }

  function onMove(e: MouseEvent) {
    // `mouseenter` always precedes `mousemove` on the same element, but we measure
    // here too in case the action mounts with the cursor already on it (a list
    // re-rendering under the mouse).
    if (!rect) rect = node.getBoundingClientRect();
    const px = (e.clientX - rect.left) / rect.width;
    const py = (e.clientY - rect.top) / rect.height;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      node.style.setProperty("--tilt-glow-x", `${(px * 100).toFixed(1)}%`);
      node.style.setProperty("--tilt-glow-y", `${(py * 100).toFixed(1)}%`);
    });
  }

  function onLeave() {
    rect = null;
  }

  node.addEventListener("mouseenter", onEnter);
  node.addEventListener("mousemove", onMove);
  node.addEventListener("mouseleave", onLeave);
  return {
    destroy() {
      node.removeEventListener("mouseenter", onEnter);
      node.removeEventListener("mousemove", onMove);
      node.removeEventListener("mouseleave", onLeave);
      cancelAnimationFrame(raf);
    },
  };
}
