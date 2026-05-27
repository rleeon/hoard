// Mousemove-driven CSS custom properties for radial spotlight effects.
// Pairs with the `.spotlight` class in app.css. Throttled via rAF so
// the per-card pointer listeners cost at most one style write per frame.
export function spotlight(node: HTMLElement) {
  if (typeof window === 'undefined') return {};

  let pending = false;
  let lastX = 50;
  let lastY = 50;

  function flush() {
    pending = false;
    node.style.setProperty('--mx', `${lastX}%`);
    node.style.setProperty('--my', `${lastY}%`);
  }

  function onMove(e: PointerEvent) {
    const rect = node.getBoundingClientRect();
    lastX = ((e.clientX - rect.left) / rect.width) * 100;
    lastY = ((e.clientY - rect.top) / rect.height) * 100;
    if (!pending) {
      pending = true;
      requestAnimationFrame(flush);
    }
  }

  node.addEventListener('pointermove', onMove, { passive: true });
  node.addEventListener('pointerenter', onMove, { passive: true });

  return {
    destroy() {
      node.removeEventListener('pointermove', onMove);
      node.removeEventListener('pointerenter', onMove);
    }
  };
}
