/**
 * Eased wheel scrolling, for mice only.
 *
 * A mouse wheel arrives as a few big jumps a second (~100px a notch). When the
 * browser's own smooth scrolling is off, which is what `prefers-reduced-motion`
 * does among others, each notch lands in a single frame and the page appears to
 * teleport. This keeps a target offset and walks the page towards it, so a notch
 * reads as a short glide.
 *
 * Scrolling from script gives up the browser's compositor-only scroll path, so
 * every frame has to repaint whatever is expensive on the page. Layout reads are
 * kept out of the loop for that reason: the document height is sampled per notch
 * and on resize, never per frame.
 *
 * Deliberately narrow:
 *   - touch and trackpads are left alone; both already have inertia of their own,
 *     and a trackpad's small continuous deltas would fight this loop
 *   - ctrl+wheel (zoom) and anything inside a scrollable box passes straight
 *     through
 *   - the target resets whenever the page is scrolled by other means, so
 *     keyboard, anchors and scrollbar dragging keep working untouched
 */
const EASE = 0.22;
const MIN_MOUSE_DELTA = 40;
const LINE_HEIGHT = 16;

export function smoothWheel(): () => void {
  if (typeof window === 'undefined') return () => {};
  if (window.matchMedia('(pointer: coarse)').matches) return () => {};

  const root = document.documentElement;
  let target = window.scrollY;
  let raf = 0;
  let animating = false;
  let limit = 0;
  // Where the last frame left the page. If the next one does not start there,
  // something else moved it (a scrollbar drag, the keyboard, an anchor) and the
  // glide has to yield instead of hauling the page back to its own target.
  let expected = -1;

  const measure = () => {
    limit = root.scrollHeight - window.innerHeight;
  };
  measure();

  // Reading `scrollHeight` mid-glide would force a layout on every frame, so it
  // is sampled here instead: on resize, and once when a glide begins.
  const ro = new ResizeObserver(measure);
  ro.observe(document.body);

  const scrollableUnder = (node: EventTarget | null): boolean => {
    let el = node as HTMLElement | null;
    while (el && el !== document.body) {
      if (el.scrollHeight > el.clientHeight || el.scrollWidth > el.clientWidth) {
        const style = getComputedStyle(el);
        if (/(auto|scroll)/.test(style.overflowY + style.overflowX)) return true;
      }
      el = el.parentElement;
    }
    return false;
  };

  const stop = () => {
    animating = false;
    expected = -1;
    cancelAnimationFrame(raf);
  };

  const step = () => {
    const current = window.scrollY;
    if (expected >= 0 && Math.abs(current - expected) > 2) {
      target = current;
      stop();
      return;
    }
    const diff = target - current;
    if (Math.abs(diff) < 0.5) {
      window.scrollTo(0, Math.round(target));
      stop();
      return;
    }
    // Whole pixels: a fractional offset makes the browser resample every layer
    // it paints, which is exactly the work this loop cannot afford.
    window.scrollTo(0, Math.round(current + diff * EASE));
    const after = window.scrollY;
    // At the very top or the very bottom the page cannot move any further, so
    // `diff` never shrinks and the loop would spin forever: it kept the wheeling
    // class on (which tinted the background) and left a stale target that
    // blocked the last stretch of the page. If a frame changed nothing, we are
    // against a stop.
    if (Math.abs(after - current) < 0.5) {
      target = after;
      stop();
      return;
    }
    expected = after;
    raf = requestAnimationFrame(step);
  };

  const onWheel = (e: WheelEvent) => {
    if (e.ctrlKey || e.defaultPrevented) return;
    if (Math.abs(e.deltaY) < MIN_MOUSE_DELTA && e.deltaMode === 0) return;
    if (scrollableUnder(e.target)) return;

    e.preventDefault();
    measure();
    if (!animating) target = window.scrollY;
    const px = e.deltaMode === 1 ? e.deltaY * LINE_HEIGHT : e.deltaY;
    target = Math.max(0, Math.min(limit, target + px));
    if (!animating) {
      animating = true;
      raf = requestAnimationFrame(step);
    }
  };

  // Any scroll we did not start (keyboard, anchor, scrollbar) becomes the new
  // truth, otherwise the next notch would yank the page back to a stale target.
  const onScroll = () => {
    if (!animating) target = window.scrollY;
  };

  // Grabbing the scrollbar, or clicking anywhere at all, hands control back at
  // once: waiting for the drag to produce a scroll event would still let one
  // frame of the old glide through.
  const onPointerDown = () => {
    if (animating) {
      target = window.scrollY;
      stop();
    }
  };

  window.addEventListener('wheel', onWheel, { passive: false });
  window.addEventListener('scroll', onScroll, { passive: true });
  window.addEventListener('pointerdown', onPointerDown, { passive: true, capture: true });

  return () => {
    cancelAnimationFrame(raf);
    ro.disconnect();
    stop();
    window.removeEventListener('wheel', onWheel);
    window.removeEventListener('scroll', onScroll);
    window.removeEventListener('pointerdown', onPointerDown, { capture: true });
  };
}
