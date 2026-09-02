/**
 * Ripple action, an expanding highlight spawns from the click point, the
 * Material-style affordance that makes a button feel like it "gives". The
 * host element only needs `position: relative; overflow: hidden;` (the
 * `.ripple-host` class) so the spawned span clips to its rounded corners.
 *
 * Skipped under prefers-reduced-motion (the press scale alone is enough).
 */
export function ripple(node: HTMLElement) {
  const reduce =
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  if (reduce) return {};

  function onPointer(e: PointerEvent) {
    if (e.button !== 0) return;
    const r = node.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    const size = Math.max(r.width, r.height) * 2;
    const span = document.createElement("span");
    span.className = "ripple";
    span.style.width = `${size}px`;
    span.style.height = `${size}px`;
    span.style.left = `${x - size / 2}px`;
    span.style.top = `${y - size / 2}px`;
    node.append(span);
    window.setTimeout(() => span.remove(), 650);
  }

  node.addEventListener("pointerdown", onPointer);
  return {
    destroy() {
      node.removeEventListener("pointerdown", onPointer);
    },
  };
}
