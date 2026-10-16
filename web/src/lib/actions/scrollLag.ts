// The floating nav capsule lags behind the page: scrolling carries it a little
// way along with the content, and a spring pulls it back to its spot with a
// small bounce, so it reads as dragged rather than nailed to the screen.
//
// It moves through the `translate` property, not `transform`: the capsule's
// dock/drip keyframes own `transform` and end with `fill: both`, which would
// override an inline transform for good. The two properties compose.
//
// Runs on requestAnimationFrame only while there is motion left, and not at
// all under prefers-reduced-motion (the site-wide CSS rule cannot reach a
// JavaScript animation, so this checks the preference itself).

export type ScrollLagOptions = {
  /** Off while the capsule is docked to the top edge: it must stay flush. */
  active: boolean;
};

// Each scrolled pixel kicks the capsule's speed rather than its position, so a
// wheel notch that jumps 400px in one frame still reads as a drag and not as a
// twitch to the limit.
const KICK = 2.4;
// Never visibly further than this from its place: the offset saturates softly
// (tanh) instead of hitting a wall.
const MAX = 12;
// Spring back: stiffness (1/s²) and damping (1/s). Damping ratio ~0.57, so it
// overshoots once, slightly, and settles.
const STIFF = 200;
const DAMP = 16;

export function scrollLag(node: HTMLElement, opts: ScrollLagOptions) {
  if (typeof window === 'undefined') return {};

  let active = opts.active;
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  let y = 0;
  let v = 0;
  let lastScroll = window.scrollY;
  let last = 0;
  let frame = 0;

  const paint = () => {
    node.style.translate = y === 0 ? '' : `0 ${(MAX * Math.tanh(y / MAX)).toFixed(2)}px`;
  };

  const tick = (now: number) => {
    const dt = last ? Math.min((now - last) / 1000, 0.05) : 1 / 60;
    last = now;
    const sy = window.scrollY;
    const dy = sy - lastScroll;
    lastScroll = sy;

    // Scrolling down moves the content up, and the capsule goes up with it.
    if (active && !reduce.matches) v -= dy * KICK;
    v += (-STIFF * y - DAMP * v) * dt;
    y = Math.max(-3 * MAX, Math.min(3 * MAX, y + v * dt));

    if (dy === 0 && Math.abs(y) < 0.05 && Math.abs(v) < 0.5) {
      y = 0;
      v = 0;
      last = 0;
      frame = 0;
      paint();
      return;
    }
    paint();
    frame = requestAnimationFrame(tick);
  };

  const onScroll = () => {
    if (!frame) frame = requestAnimationFrame(tick);
  };
  window.addEventListener('scroll', onScroll, { passive: true });

  return {
    update(next: ScrollLagOptions) {
      active = next.active;
    },
    destroy() {
      window.removeEventListener('scroll', onScroll);
      cancelAnimationFrame(frame);
      node.style.translate = '';
    }
  };
}
