// Mousemove-driven CSS custom properties for radial spotlight effects.
// Pairs with the `.spotlight` class in app.css.
export function spotlight(node: HTMLElement) {
  if (typeof window === 'undefined') return {};

  function onMove(e: PointerEvent) {
    const rect = node.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 100;
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    node.style.setProperty('--mx', `${x}%`);
    node.style.setProperty('--my', `${y}%`);
  }

  node.addEventListener('pointermove', onMove);
  node.addEventListener('pointerenter', onMove);

  return {
    destroy() {
      node.removeEventListener('pointermove', onMove);
      node.removeEventListener('pointerenter', onMove);
    }
  };
}
