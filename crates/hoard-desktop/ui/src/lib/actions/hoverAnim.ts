/**
 * Plays an icon's hover move to the end, whatever the pointer does next.
 *
 * The moves used to hang off `:hover` in CSS, so the animation only lasted as
 * long as the pointer stayed: leave halfway and the star snapped back to rest
 * mid-jump. Settings' vault door never had the problem because it plays from
 * script and waits for `animationend`. This does the same for every
 * `[data-anim]` icon: entering its parent (or an `.anim-host` above it) sets
 * `data-anim-play`, which carries the animation in `app.css`, and the attribute
 * only comes off when the animation is over.
 *
 * An attribute and not a class because Svelte owns `class` on these elements
 * (`AnimIcon` swaps its own classes) and would wipe ours on the next render.
 */

function play(icon: Element): void {
  if (icon.hasAttribute("data-anim-play")) return;
  icon.setAttribute("data-anim-play", "");
  const done = (e: Event) => {
    if (e.target !== icon) return;
    icon.removeAttribute("data-anim-play");
    icon.removeEventListener("animationend", done);
    icon.removeEventListener("animationcancel", done);
  };
  icon.addEventListener("animationend", done);
  icon.addEventListener("animationcancel", done);
}

export function installHoverAnim(): void {
  document.addEventListener(
    "pointerover",
    (e) => {
      const from = e.relatedTarget as Node | null;
      // Walk up from where the pointer is. Every element on the way that did
      // not already contain it is one it just entered; the first that did
      // means everything above was entered earlier.
      for (let el = e.target as Element | null; el; el = el.parentElement) {
        if (from && el.contains(from)) break;
        const icons = el.classList.contains("anim-host")
          ? el.querySelectorAll("[data-anim]")
          : el.querySelectorAll(":scope > [data-anim]");
        icons.forEach(play);
      }
    },
    { passive: true },
  );
}
