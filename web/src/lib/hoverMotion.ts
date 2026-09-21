/**
 * Hover motion for every button on the site, delegated from the document.
 *
 * Two things this does that a per-node `use:` action and a `:hover` rule
 * cannot:
 *
 *  - **It finishes.** A CSS animation hung off `:hover` is cancelled the
 *    instant the pointer leaves, so flicking the mouse across a button showed
 *    nothing at all: the bell was killed a few milliseconds in. Here the
 *    animation is driven by the `anim-run` class, added on the way in and
 *    removed on `animationend`, so a fast pass plays the whole move.
 *  - **It covers what isn't mounted yet.** One `pointerover` listener on the
 *    document catches any `.glow` / `.pop-self` / `.anim-host` on the page,
 *    including controls inside panels that open later.
 *
 * The glow rides on the same delegation, and it is a class rather than
 * `:hover` for the same reason: `:hover` only lights what the pointer is
 * sitting on. Lit on the way in and dropped on the way out, with a slow fade
 * underneath it, a sweep across a row of buttons leaves a trail of them
 * cooling off behind the cursor. `--tilt-glow-x/y` follow the pointer,
 * throttled to one frame.
 */

/** Controls that take part: the glow, the button's own bounce, or a host whose
 *  icon carries `data-anim`. */
const CONTROL = '.glow, .pop-self, .anim-host';

function play(node: Element) {
  // Re-adding a class the element already has does not restart an animation,
  // and a second pass over the same button within its 360ms is exactly when
  // you want it to. Drop it, force a reflow, add it back.
  node.classList.remove('anim-run');
  // `offsetWidth` is an HTMLElement-only trick and half of these nodes are
  // <svg> icons; `getBoundingClientRect` forces the same reflow on anything.
  node.getBoundingClientRect();
  node.classList.add('anim-run');
  node.addEventListener('animationend', () => node.classList.remove('anim-run'), {
    once: true
  });
}

export function initHoverMotion() {
  if (typeof window === 'undefined') return () => {};

  let current: Element | null = null;
  let rect: DOMRect | null = null;
  let raf = 0;

  function glowAt(el: Element, clientX: number, clientY: number) {
    if (!rect) rect = el.getBoundingClientRect();
    const px = (clientX - rect.left) / rect.width;
    const py = (clientY - rect.top) / rect.height;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      (el as HTMLElement).style.setProperty('--tilt-glow-x', `${(px * 100).toFixed(1)}%`);
      (el as HTMLElement).style.setProperty('--tilt-glow-y', `${(py * 100).toFixed(1)}%`);
    });
  }

  function onOver(e: PointerEvent | MouseEvent) {
    const target = e.target as Element | null;
    const el = target?.closest?.(CONTROL) ?? null;
    if (el === current) return;
    // The one we just left keeps its glow and fades out on its own: that fade
    // is the trail.
    current?.classList.remove('glow-lit');
    current = el;
    rect = null;
    if (!el) return;

    // The glow starts where the pointer actually is, not at the centre: with
    // no move event yet, a fast pass would light the middle of the button
    // rather than the edge you came in through.
    if (el.classList.contains('glow')) {
      glowAt(el, e.clientX, e.clientY);
      el.classList.add('glow-lit');
    }

    if (el.classList.contains('pop-self')) play(el);
    // A `data-anim` icon bounces with its host, whatever the pointer is
    // actually over inside it (the label, the padding, the icon itself).
    el.querySelectorAll('[data-anim]').forEach(play);
  }

  function onMove(e: PointerEvent | MouseEvent) {
    if (!current || !current.classList.contains('glow')) return;
    glowAt(current, e.clientX, e.clientY);
  }

  // The pointer leaving the window never fires `pointerover` on anything, so
  // without this the last control stays lit until you come back.
  function onOut(e: PointerEvent | MouseEvent) {
    if (!(e as PointerEvent).relatedTarget && current) {
      current.classList.remove('glow-lit');
      current = null;
      rect = null;
    }
  }
  document.addEventListener('pointerover', onOver, { passive: true });
  document.addEventListener('pointermove', onMove, { passive: true });
  document.addEventListener('pointerout', onOut, { passive: true });

  return () => {
    document.removeEventListener('pointerover', onOver);
    document.removeEventListener('pointermove', onMove);
    document.removeEventListener('pointerout', onOut);
    cancelAnimationFrame(raf);
  };
}
