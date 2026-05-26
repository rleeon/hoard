// IntersectionObserver-based reveal-on-scroll action.
// Usage: <div use:reveal={{ delay: 80 }} class="reveal">…</div>
export type RevealOptions = {
  delay?: number;
  threshold?: number;
  rootMargin?: string;
  once?: boolean;
};

export function reveal(node: HTMLElement, opts: RevealOptions = {}) {
  const { delay = 0, threshold = 0.15, rootMargin = '0px 0px -8% 0px', once = true } = opts;

  if (typeof window === 'undefined') return {};

  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reducedMotion) {
    node.classList.add('is-visible');
    return {};
  }

  if (delay) node.style.transitionDelay = `${delay}ms`;

  const obs = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          node.classList.add('is-visible');
          if (once) obs.disconnect();
        } else if (!once) {
          node.classList.remove('is-visible');
        }
      }
    },
    { threshold, rootMargin }
  );

  obs.observe(node);

  return {
    destroy() {
      obs.disconnect();
    }
  };
}
