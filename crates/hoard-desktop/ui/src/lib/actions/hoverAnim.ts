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

/** Walk up from `target`. Every element on the way that did not already contain
 *  `from` is one just entered; the first that did means everything above was
 *  entered earlier. */
function enter(target: Element | null, from: Node | null): void {
  for (let el = target; el; el = el.parentElement) {
    if (from && el.contains(from)) break;
    const icons = el.classList.contains("anim-host")
      ? el.querySelectorAll("[data-anim]")
      : el.querySelectorAll(":scope > [data-anim]");
    icons.forEach(play);
  }
}

export function installHoverAnim(): void {
  document.addEventListener(
    "pointerover",
    (e) => enter(e.target as Element | null, e.relatedTarget as Node | null),
    { passive: true },
  );
  // The keyboard passing over a control (arrows, Tab) plays what the pointer
  // would have. A click also focuses, and must not play it a second time:
  // `:focus-visible` is false then. The control's own icons count however deep
  // they sit, since the pointer would have crossed every wrapper on its way in.
  document.addEventListener("focusin", (e) => {
    const el = e.target;
    if (!(el instanceof Element) || !el.matches(":focus-visible")) return;
    el.querySelectorAll("[data-anim]").forEach(play);
    enter(el, e.relatedTarget as Node | null);
  });
}
