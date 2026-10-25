/**
 * Arrow keys move the focus between controls, and Enter presses them. Steam's
 * desktop layout sends the Deck's d-pad as arrows and its face button as Enter,
 * so this is what lets the app be driven without the trackpad; it is also how
 * to try that from a PC keyboard.
 *
 * Up and Down go line by line, the way the page reads, inside the area the
 * focus is in (the sidebar, the page, a dialog), and leave it only for an area
 * straight above or below. Measuring plain distance instead sent Down from a
 * page into the sidebar's buttons whenever those happened to be closer. Left
 * and Right stay on the line and cross into the area beside it. Ctrl+Up/Down
 * scroll the page and leave the focus where it is.
 *
 * It only takes an arrow nobody else wanted: a component with arrows of its own
 * (an open `Select`, a slider) calls `preventDefault` first. Inside a text field
 * Left and Right move the caret until it reaches either end of the text.
 */

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
  "[contenteditable='true']",
  // Decoration that is still a place to land on (the Halloween spider).
  "[data-nav-stop]",
].join(",");

const REGION = '[role="dialog"][aria-modal="true"], aside, main, [data-nav-region]';

const TEXT_INPUTS = new Set(["text", "search", "url", "tel", "email", "password", "number"]);

/** Roles a keyboard expects Enter to press, which only native controls do alone. */
const PRESSABLE = new Set(["button", "link", "checkbox", "switch", "tab", "menuitem"]);

type Dir = "up" | "down" | "left" | "right";
type Box = { el: HTMLElement; r: DOMRect };

const DIRS: Record<string, Dir> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

let lastEl: HTMLElement | null = null;
let lastRect: DOMRect | null = null;
let marked: Element | null = null;
/** The last control focused outside any dialog: where the focus goes back to
 *  when the dialog closes, not to whatever was under its buttons. */
let outside: HTMLElement | null = null;

function scope(): ParentNode {
  const dialogs = document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]');
  return dialogs[dialogs.length - 1] ?? document;
}

function regionOf(el: Element): Element {
  return el.closest(REGION) ?? document.body;
}

/** What the eye sees of a control. A switch is a 1 px checkbox drawn by the
 *  spans around it: its own outline is invisible, and its own box is in the
 *  wrong place to measure from. */
function visualOf(el: HTMLElement): HTMLElement {
  let node = el;
  while (node.parentElement) {
    const r = node.getBoundingClientRect();
    if (r.width >= 8 && r.height >= 8) break;
    node = node.parentElement;
  }
  return node;
}

function rectOf(el: HTMLElement): DOMRect {
  return visualOf(el).getBoundingClientRect();
}

function candidates(root: ParentNode): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => {
    if (el.closest("[inert]")) return false;
    if (!el.hasAttribute("data-nav-stop") && el.closest("[aria-hidden='true']")) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== "hidden";
  });
}

/** Whether the arrow belongs to the field itself: moving its caret, or its value. */
function fieldKeepsArrow(el: Element, dir: Dir): boolean {
  if (el instanceof HTMLElement && el.isContentEditable) return true;
  const horizontal = dir === "left" || dir === "right";
  if (el instanceof HTMLInputElement) {
    if (el.type === "range") return horizontal;
    if (!TEXT_INPUTS.has(el.type) || !horizontal) return false;
  } else if (!(el instanceof HTMLTextAreaElement)) {
    return false;
  }
  const { selectionStart: start, selectionEnd: end, value } = el;
  // `number` and `email` give no caret in WebKit: the arrow leaves.
  if (start === null || end === null) return false;
  if (start !== end) return true;
  switch (dir) {
    case "left":
      return start > 0;
    case "right":
      return end < value.length;
    case "up":
      return value.slice(0, start).includes("\n");
    case "down":
      return value.slice(end).includes("\n");
  }
}

function closest(boxes: Box[], score: (r: DOMRect) => number): HTMLElement | null {
  let best: Box | null = null;
  let bestScore = Infinity;
  for (const b of boxes) {
    const s = score(b.r);
    if (s < bestScore) {
      bestScore = s;
      best = b;
    }
  }
  return best?.el ?? null;
}

/** The next line up or down, and on it the control nearest ours across. */
function vertical(from: DOMRect, dir: "up" | "down", boxes: Box[]): HTMLElement | null {
  const mid = from.top + from.height / 2;
  const ahead = boxes.filter(({ r }) => (dir === "down" ? r.top >= mid : r.bottom <= mid));
  if (ahead.length === 0) return null;
  let first = ahead[0];
  for (const b of ahead) {
    if (dir === "down" ? b.r.top < first.r.top : b.r.bottom > first.r.bottom) first = b;
  }
  const line = ahead.filter(({ r }) => r.top < first.r.bottom && r.bottom > first.r.top);
  const cx = from.left + from.width / 2;
  return closest(
    line,
    (r) => Math.max(0, r.left - from.right, from.left - r.right) + Math.abs(r.left + r.width / 2 - cx) * 0.01,
  );
}

/** Along the line first; only when nothing is level with us, the nearest that way. */
function horizontal(from: DOMRect, dir: "left" | "right", boxes: Box[]): HTMLElement | null {
  // Clear of our own box, so a full-width row in the sidebar has nothing to its
  // left, rather than everything above it.
  const side = boxes.filter(({ r }) => (dir === "right" ? r.left >= from.right - 4 : r.right <= from.left + 4));
  const gap = (r: DOMRect) => Math.max(0, dir === "right" ? r.left - from.right : from.left - r.right);
  const cy = from.top + from.height / 2;
  const drift = (r: DOMRect) => Math.abs(r.top + r.height / 2 - cy);
  const level = side.filter(({ r }) => r.top < from.bottom && r.bottom > from.top);
  if (level.length > 0) return closest(level, (r) => gap(r) + drift(r) * 0.01);
  return closest(
    side,
    (r) => gap(r) + Math.max(0, r.top - from.bottom, from.top - r.bottom) * 2 + drift(r) * 0.1,
  );
}

function pick(from: DOMRect, dir: Dir, region: Element, boxes: Box[]): HTMLElement | null {
  if (dir === "left" || dir === "right") return horizontal(from, dir, boxes);
  // Outside every area (a notification floating over the page): anything goes.
  if (region === document.body) return vertical(from, dir, boxes);
  const own = vertical(from, dir, boxes.filter((b) => regionOf(b.el) === region));
  if (own) return own;
  // The end of this area: on into one straight above or below it (the title bar
  // over the page), never into one beside it.
  const rr = region.getBoundingClientRect();
  return vertical(
    from,
    dir,
    boxes.filter((b) => regionOf(b.el) !== region && b.r.left < rr.right && b.r.right > rr.left),
  );
}

/** With nothing focused, the first arrow picks the top-left control on screen. */
function firstOnScreen(pool: HTMLElement[]): HTMLElement | null {
  const onScreen = pool.filter((el) => {
    const r = rectOf(el);
    return r.bottom > 0 && r.top < window.innerHeight;
  });
  return closest(
    (onScreen.length > 0 ? onScreen : pool).map((el) => ({ el, r: rectOf(el) })),
    (r) => Math.max(0, r.top) + r.left * 0.5,
  );
}

function scrollerOf(el: Element | null): HTMLElement | null {
  for (let node = el?.parentElement ?? null; node; node = node.parentElement) {
    const overflow = getComputedStyle(node).overflowY;
    if ((overflow === "auto" || overflow === "scroll") && node.scrollHeight > node.clientHeight) {
      return node;
    }
  }
  return document.scrollingElement as HTMLElement | null;
}

/** What scrolls around `el`, or the page when nothing does (the sidebar). */
function pageScroller(el: Element | null): HTMLElement | null {
  const own = scrollerOf(el);
  return own && own !== document.scrollingElement ? own : document.querySelector<HTMLElement>("main");
}

function unmark(): void {
  marked?.removeAttribute("data-nav-focus");
  marked = null;
}

function move(el: HTMLElement, scroll = true): void {
  el.focus({ preventScroll: true });
  const visual = visualOf(el);
  if (visual !== el) {
    visual.setAttribute("data-nav-focus", "");
    marked = visual;
  }
  lastEl = el;
  lastRect = visual.getBoundingClientRect();
  if (scroll) {
    // A card is shown whole, cover and details, not only the button inside it.
    (el.closest("[data-nav-frame]") ?? visual).scrollIntoView({ block: "nearest", inline: "nearest" });
  }
}

function atPoint(r: DOMRect): HTMLElement | null {
  for (const node of document.elementsFromPoint(r.left + r.width / 2, r.top + r.height / 2)) {
    const hit =
      node.closest<HTMLElement>(FOCUSABLE) ??
      node.closest("label")?.querySelector<HTMLElement>(FOCUSABLE) ??
      null;
    if (hit && !hit.closest("[inert]")) return hit;
  }
  return null;
}

/**
 * After Enter, the control may redraw, or go disabled while it works (a switch
 * saving, Sync turning on), and the browser drops the focus on the page. For two
 * seconds it is put back on that control, or on whatever took its place; if
 * anything else takes the focus meanwhile (a dialog), that stands.
 *
 * Watched the whole time, not checked once: a disabled control stays the active
 * element for a moment and only then does the browser let go of it, so a first
 * look found nothing wrong (a switch in Settings, Oct 2026).
 */
function keepPlace(el: HTMLElement): void {
  const rect = rectOf(el);
  const inDialog = el.closest('[role="dialog"]') !== null;
  let ticks = 0;
  let current = el;
  const check = () => {
    const now = document.activeElement;
    const lost = !now || now === document.body || (now === current && !current.matches(FOCUSABLE));
    if (now && now !== document.body && now !== current) return;
    if (lost) {
      const back =
        el.isConnected && el.matches(FOCUSABLE)
          ? el
          : inDialog && !el.isConnected && outside?.isConnected
            ? outside
            : atPoint(rect);
      if (back) {
        current = back;
        move(back, false);
      }
    }
    if (++ticks < 20) setTimeout(check, 100);
  };
  setTimeout(check, 0);
}

function onKeydown(e: KeyboardEvent): void {
  if (e.isComposing || e.altKey || e.metaKey || e.shiftKey) return;
  const active =
    document.activeElement instanceof HTMLElement && document.activeElement !== document.body
      ? document.activeElement
      : null;
  // A control that took Enter for itself (choosing in a `Select`) can still go
  // disabled while the choice saves.
  if (e.defaultPrevented) {
    if (e.key === "Enter" && active) keepPlace(active);
    return;
  }

  if (e.ctrlKey) {
    if ((e.key === "ArrowDown" || e.key === "ArrowUp") && !(active instanceof HTMLTextAreaElement)) {
      const scroller = pageScroller(active);
      if (!scroller) return;
      e.preventDefault();
      scroller.scrollBy({
        top: (e.key === "ArrowDown" ? 0.4 : -0.4) * scroller.clientHeight,
        behavior: "smooth",
      });
    }
    return;
  }

  if (e.key === "Enter") {
    if (!active) return;
    const native = active instanceof HTMLButtonElement || active instanceof HTMLAnchorElement;
    const box =
      active instanceof HTMLInputElement && (active.type === "checkbox" || active.type === "radio");
    if (box || (!native && PRESSABLE.has(active.getAttribute("role") ?? ""))) {
      e.preventDefault();
      active.click();
    }
    keepPlace(active);
    return;
  }

  const dir = DIRS[e.key];
  if (!dir) return;
  if (active && fieldKeepsArrow(active, dir)) return;
  // An open dropdown's list is still on screen; Left and Right are not a way out.
  if (active?.getAttribute("role") === "combobox" && active.getAttribute("aria-expanded") === "true") {
    return;
  }

  const root = scope();
  const pool = candidates(root);
  const inScope = (el: Element) => root === document || (root as Element).contains(el);
  // Where to start: the focus, or where it last was when Enter or a redraw lost it.
  let origin: HTMLElement | null = active && inScope(active) ? active : null;
  let from: DOMRect | null = origin ? rectOf(origin) : null;
  // A dialog closed under the focus (Escape, its close button): back to the
  // control that opened it.
  if (!origin && root === document && lastEl && !lastEl.isConnected && outside?.isConnected) {
    e.preventDefault();
    move(outside);
    return;
  }
  if (!origin && lastEl && lastRect && inScope(lastEl)) {
    origin = lastEl.isConnected ? lastEl : null;
    from = origin ? rectOf(origin) : lastRect;
  }
  // Scrolled out of sight (Ctrl+arrows, the wheel): start again from what is on screen.
  if (from && (from.bottom < 0 || from.top > window.innerHeight)) from = null;

  e.preventDefault();
  let target: HTMLElement | null;
  if (from) {
    const region = origin
      ? regionOf(origin)
      : regionOf(document.elementFromPoint(from.left + from.width / 2, from.top + from.height / 2) ?? document.body);
    const boxes = pool.filter((el) => el !== origin).map((el) => ({ el, r: rectOf(el) }));
    target = pick(from, dir, region, boxes);
  } else {
    target = firstOnScreen(pool);
  }
  if (target) {
    move(target);
  } else if (dir === "up" || dir === "down") {
    // Nothing further that way: the page itself moves, so long text still reads.
    const scroller = pageScroller(origin);
    scroller?.scrollBy({ top: (dir === "down" ? 0.6 : -0.6) * scroller.clientHeight });
  }
}

export function installSpatialNav(): void {
  window.addEventListener("keydown", onKeydown);
  document.addEventListener("focusout", unmark);
  document.addEventListener("pointerdown", unmark, true);
  document.addEventListener("focusin", (e) => {
    if (e.target instanceof HTMLElement && e.target.matches(FOCUSABLE)) {
      lastEl = e.target;
      lastRect = rectOf(e.target);
      if (!e.target.closest('[role="dialog"]')) outside = e.target;
    }
  });
}
