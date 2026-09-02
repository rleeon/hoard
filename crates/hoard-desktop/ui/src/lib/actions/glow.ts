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
  /** Rect cacheado al entrar. `getBoundingClientRect` fuerza un cálculo de
   *  layout síncrono, y llamarlo en cada `mousemove` significaba pagarlo
   *  decenas de veces por segundo mientras el cursor cruza un botón. El
   *  tamaño de un botón no cambia mientras lo apuntas, así que basta medirlo
   *  al entrar. */
  let rect: DOMRect | null = null;

  function onEnter() {
    rect = node.getBoundingClientRect();
  }

  function onMove(e: MouseEvent) {
    // `mouseenter` siempre precede a `mousemove` sobre el mismo elemento,
    // pero medimos aquí también por si la acción se monta con el cursor ya
    // encima (una lista que se re-renderiza bajo el ratón).
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
