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
 * A container marked `data-nav-layer` is a single stop seen from outside. Enter
 * goes into it, and from then on the arrows stay among what it holds until
 * Escape (B on the Deck), or an arrow past its edge, brings the focus back up
 * onto it. Layers nest: on the
 * Panel the games are one, and each game is one inside it. The title bar is
 * one too, so going up never lands straight on its star or its close button. A page marked
 * `data-nav-page` is left, at any edge or with Escape, for its own entry in the
 * sidebar, and Enter on that entry goes back in. A page with no entry (a game's
 * history) has its way back marked `data-nav-back`, and Escape lands there.
 * Enter that opens another page lands on that page's start.
 *
 * A dialog opened from the keys takes the arrows in at once, on its way out
 * (the first button of its footer, «Cancelar») whenever it has a dangerous
 * button (`data-nav-danger`, red when it is the one under the arrows), so an
 * Enter too many never deletes anything. Closing it brings them back to what
 * opened it. A panel that drops over the page (`navPopup`: the bell, the eye)
 * does the same, never starting on its dangerous one either (the bell's
 * «Borrar todo»), and Escape closes it.
 *
 * Q and E (L1 and R1 on a controller, `gamepad.ts`) go to the previous and
 * next page of the sidebar.
 *
 * `navMarked` says what the arrows are on, and `navActions` what Enter and
 * Escape do from there, for the hint at the bottom of the window.
 *
 * Where the geometry gets it wrong, a control names the next one itself:
 * `data-nav-down="rename"` goes to the control with `data-nav-id="rename"`
 * (several ids, the first one on screen). The buttons drawn over a cover all
 * sit inside the cover's own box, which no measuring sorts out.
 *
 * It only takes an arrow nobody else wanted: a component with arrows of its own
 * (an open `Select`, a slider) calls `preventDefault` first. Inside a text field
 * Left and Right move the caret until it reaches either end of the text.
 */

import { writable } from "svelte/store";

const LAYER = "[data-nav-layer]";

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
  LAYER,
].join(",");

/** Where the arrows are shut in while it is open: a dialog, or a panel over
 *  the page. Not while it fades out (Svelte makes it inert then). */
const DIALOG = '[role="dialog"][aria-modal="true"]:not([inert]), [data-nav-popup]:not([inert])';
const IN_DIALOG = '[role="dialog"], [data-nav-popup]';

const REGION =
  '[role="dialog"][aria-modal="true"], [data-nav-popup], aside, main, [data-nav-page], [data-nav-region]';

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
let keys = false;
/** When the arrows last moved the focus: a card is still gliding into view for
 *  a moment after, and is not "scrolled out of sight" while it does. */
let movedAt = 0;
/** The route `lastEl` was on: when it changed, the old place is gone. */
let lastRoute = "";
/** What each layer last held the focus on, one level down. */
const remembered = new WeakMap<Element, HTMLElement>();
/** Where the arrows last were on each page in layers, at its own level. */
const lastOnPage = new WeakMap<Element, HTMLElement>();

function scope(): Element | Document {
  const dialogs = document.querySelectorAll<HTMLElement>(DIALOG);
  return dialogs[dialogs.length - 1] ?? document;
}

function regionOf(el: Element): Element {
  return el.closest(REGION) ?? document.body;
}

/** The layer `el` is inside, not counting `el` itself, nor a layer outside the
 *  open dialog. */
function layerOf(el: Element, root: Element | Document = document): Element | null {
  const layer = el.parentElement?.closest(LAYER) ?? null;
  return layer && root.contains(layer) ? layer : null;
}

function sidebarEntry(): HTMLElement | null {
  return document.querySelector<HTMLElement>('aside [aria-current="page"]');
}

function currentPage(): Element | null {
  return document.querySelector("main [data-nav-page]");
}

/** One level above a page's layers: its sidebar entry, or its own way back. */
function pageBack(): HTMLElement | null {
  return sidebarEntry() ?? currentPage()?.querySelector<HTMLElement>("[data-nav-back]") ?? null;
}

/** Where coming into a page in layers lands: back where the arrows last were
 *  on it, else its first stop under `data-nav-start` (the Panel's first game),
 *  else its top, never whichever is level with what the focus left behind. */
function startOf(page: Element, pool: HTMLElement[]): HTMLElement | null {
  const last = lastOnPage.get(page);
  if (last && pool.includes(last)) return last;
  const start = page.querySelector("[data-nav-start]");
  const first = start && firstOnScreen(pool.filter((el) => start.contains(el)));
  if (first) return first;
  const units = pool.filter((el) => page.contains(el)).map((el) => ({ el, r: rectOf(el) }));
  return closest(units, (r) => r.top + r.left * 0.01);
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

/** The stops one level inside `layer` (the page itself when null): its own
 *  controls and the layers it holds, never what those layers hold. */
function candidates(root: Element | Document, layer: Element | null): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => {
    if (el.closest("[inert]")) return false;
    if (layerOf(el, root) !== layer) return false;
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

/** What the engine does with a real Left or Right in a field: a slider's step,
 *  or the caret one place over. False where it cannot be done from here. */
function nudge(el: Element, dir: Dir): boolean {
  if (dir !== "left" && dir !== "right") return false;
  if (el instanceof HTMLInputElement && el.type === "range") {
    if (dir === "right") el.stepUp();
    else el.stepDown();
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  }
  if (!(el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) return false;
  const { selectionStart: start, selectionEnd: end } = el;
  if (start === null || end === null) return false;
  const at = start !== end ? (dir === "left" ? start : end) : start + (dir === "left" ? -1 : 1);
  el.setSelectionRange(at, at);
  return true;
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

function pick(from: DOMRect, dir: Dir, region: Element, boxes: Box[], inLayer: boolean): HTMLElement | null {
  const paged = region.matches("[data-nav-page]");
  if (dir === "left" || dir === "right") {
    const next = horizontal(from, dir, boxes);
    return paged && next?.closest("aside") ? (sidebarEntry() ?? next) : next;
  }
  // Inside a layer, or outside every area (a notification floating over the
  // page): anything goes.
  if (inLayer || region === document.body) return vertical(from, dir, boxes);
  const own = vertical(from, dir, boxes.filter((b) => regionOf(b.el) === region));
  if (own) return own;
  if (paged) {
    const entry = sidebarEntry();
    if (entry) return entry;
  }
  // The end of this area: on into one straight above or below it (the title bar
  // over the page), never into one beside it.
  const rr = region.getBoundingClientRect();
  return vertical(
    from,
    dir,
    boxes.filter((b) => regionOf(b.el) !== region && b.r.left < rr.right && b.r.right > rr.left),
  );
}

/** Where `el` says this arrow goes, if that control is there. */
function linked(el: HTMLElement, dir: Dir, pool: HTMLElement[]): HTMLElement | null {
  for (const id of el.getAttribute(`data-nav-${dir}`)?.split(" ") ?? []) {
    const hit = pool.find((c) => c.dataset.navId === id);
    if (hit) return hit;
  }
  return null;
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

/** Going into a layer: back to the layer it last held, when it holds layers
 *  (the game the user was on), otherwise the control it marks `data-nav-start`
 *  (the title bar's bell, not the star at its left edge), otherwise its first. */
function inside(layer: Element): HTMLElement | null {
  const pool = candidates(scope(), layer);
  const last = remembered.get(layer);
  if (last?.matches(LAYER) && pool.includes(last)) return last;
  const start = layer.querySelector("[data-nav-start]");
  const first = start && pool.find((el) => start.contains(el));
  return first || firstOnScreen(pool);
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

/** What the arrows have the focus on; null once the pointer takes over. */
export const navMarked = writable<HTMLElement | null>(null);
let markedControl: HTMLElement | null = null;
let publishing = false;

/**
 * `navMarked` catches up a moment later, never at once. The focus can move
 * while Svelte is redrawing (the focused control removed, a dialog closing),
 * and Svelte throws on any state written then. The throw cut the redraw short:
 * on Windows, whose engine reports the focus leaving a removed control and
 * WebKitGTK's does not, Escape out of a dialog froze the hint on «Cerrar» and
 * on the controller's buttons for good (Oct 2026).
 */
function publish(): void {
  if (publishing) return;
  publishing = true;
  queueMicrotask(() => {
    publishing = false;
    navMarked.set(markedControl);
  });
}

function unmark(): void {
  marked?.removeAttribute("data-nav-focus");
  marked = null;
  markedControl = null;
  publish();
}

function move(el: HTMLElement, scroll = true): void {
  el.focus({ preventScroll: true });
  // Our own mark, not `:focus-visible`: WebKitGTK leaves that off a control
  // focused from script after a click. Settings clicked, then Up onto
  // Hoard-Wrapped: the focus was there and nothing showed (Linux, Oct 2026).
  const visual = visualOf(el);
  visual.setAttribute("data-nav-focus", "");
  marked = visual;
  markedControl = el;
  publish();
  lastEl = el;
  lastRect = visual.getBoundingClientRect();
  movedAt = performance.now();
  if (scroll) {
    // A card is shown whole, cover and details, not only the button inside it,
    // and glides there rather than jumping.
    const frame = el.closest("[data-nav-frame]");
    // A glide still running from the last card kept scrolling under the next
    // control, and a dropdown closes on any scroll: Enter on «Ordenar por»,
    // reached from the games, opened nothing. It stops here.
    const scroller = frame ? null : pageScroller(el);
    scroller?.scrollTo({ top: scroller.scrollTop, behavior: "instant" });
    // The page follows the focus and keeps it near the middle, not riding the
    // edge it came in by (Settings, going down: the control sat on the bottom
    // edge). What is taller than most of the screen (the grid of games taken
    // as a layer, a big card) only comes into view.
    const shown = frame ?? visual;
    const tall = shown.getBoundingClientRect().height > window.innerHeight * 0.6;
    shown.scrollIntoView({
      block: tall ? "nearest" : "center",
      inline: "nearest",
      behavior: frame ? "smooth" : "auto",
    });
  }
}

/**
 * Enter on a sidebar entry leaves the focus on the sidebar, which for a page in
 * layers meant no way in but sideways. Once the page has come up (lazy routes
 * take a moment) the focus goes onto its top layer, as a console menu does,
 * unless the user has moved on by then.
 */
function followInto(entry: HTMLElement): void {
  const before = currentPage();
  let ticks = 0;
  const check = () => {
    if (document.activeElement !== entry) return;
    const page = currentPage();
    if (page && page !== before && entry.getAttribute("aria-current") === "page") {
      const start = startOf(page, candidates(document, null));
      if (start) {
        move(start);
        return;
      }
    }
    if (++ticks < 30) setTimeout(check, 100);
  };
  setTimeout(check, 100);
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
  const inDialog = el.closest(IN_DIALOG) !== null;
  const route = location.hash;
  let ticks = 0;
  let current = el;
  const check = () => {
    const now = document.activeElement;
    const lost = !now || now === document.body || (now === current && !current.matches(FOCUSABLE));
    if (now && now !== document.body && now !== current) return;
    if (lost && location.hash !== route) {
      // It opened another page (a cover, its game's history): that page's
      // start once it is up, not whatever now sits where the control was. A
      // page not in layers is given a moment to come up before the sidebar.
      const page = currentPage();
      const start = page ? startOf(page, candidates(document, null)) : ticks >= 10 ? begin(document) : null;
      if (start) {
        current = start;
        move(start);
      }
    } else if (lost) {
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
  const fresh = !keys;
  keys = true;
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

  if ((e.key === "q" || e.key === "Q" || e.key === "e" || e.key === "E") && !typing(active)) {
    if (scope() !== document) return;
    e.preventDefault();
    navSwitchPage(e.key.toLowerCase() === "e" ? 1 : -1);
    return;
  }

  if (e.key === "Escape") {
    const root = scope();
    const up = active && layerOf(active, root);
    // Above a page's layers there is one more level: its entry in the sidebar.
    const back = up ?? (root === document && active?.closest("[data-nav-page]") ? pageBack() : null);
    if (!(back instanceof HTMLElement)) return;
    e.preventDefault();
    move(back);
    return;
  }

  if (e.key === "Enter") {
    if (!active) return;
    if (active.matches(LAYER)) {
      e.preventDefault();
      const first = inside(active);
      if (first) move(first);
      return;
    }
    const native = active instanceof HTMLButtonElement || active instanceof HTMLAnchorElement;
    if (native && active.closest("aside")) {
      const page = currentPage();
      if (page && active.getAttribute("aria-current") === "page") {
        // Already on that page: Enter goes into it.
        e.preventDefault();
        const start = startOf(page, candidates(document, null));
        if (start) move(start);
        return;
      }
      followInto(active);
    }
    // Nor does it send a form for one (the server address, the access key).
    if (!e.isTrusted && active instanceof HTMLInputElement && typing(active) && active.form) {
      e.preventDefault();
      active.form.requestSubmit();
      return;
    }
    const box =
      active instanceof HTMLInputElement && (active.type === "checkbox" || active.type === "radio");
    // An Enter from a controller (`gamepad.ts`) is a made-up event, and the
    // engine presses nothing for one of those: not even a plain button.
    const madeUp = !e.isTrusted && native;
    if (box || madeUp || (!native && PRESSABLE.has(active.getAttribute("role") ?? ""))) {
      e.preventDefault();
      active.click();
    }
    keepPlace(active);
    return;
  }

  const dir = DIRS[e.key];
  if (!dir) return;
  if (active && fieldKeepsArrow(active, dir)) {
    if (e.isTrusted) return;
    // A controller's arrow is made up too, and moves neither the caret nor a
    // slider by itself: it stuck in both. Done here; what cannot be (a line
    // up in a long text) leaves the field instead.
    if (nudge(active, dir)) {
      e.preventDefault();
      return;
    }
  }
  // An open dropdown's list is still on screen; Left and Right are not a way out.
  if (active?.getAttribute("role") === "combobox" && active.getAttribute("aria-expanded") === "true") {
    return;
  }

  const root = scope();
  const inScope = (el: Element) => root === document || root.contains(el);
  // After the pointer, with nothing focused (a click on empty space), the
  // arrows start where `begin` says rather than from where the focus was
  // before the click: that was often somewhere with no ring at all, and finding
  // the cursor took a handful of presses (Oct 2026). A control the click did
  // focus is a fine place to start from, and the arrow moves straight away.
  if (fresh && !(active && inScope(active))) {
    e.preventDefault();
    const start = begin(root);
    if (start) move(start);
    return;
  }
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
  // The page under it changed (Enter on a link, then the arrows): from the new
  // page's start.
  if (!origin && root === document && lastEl && !lastEl.isConnected && lastRoute !== location.hash) {
    e.preventDefault();
    const start = begin(root);
    if (start) move(start);
    return;
  }
  if (!origin && lastEl && lastRect && inScope(lastEl)) {
    origin = lastEl.isConnected ? lastEl : null;
    from = origin ? rectOf(origin) : lastRect;
  }
  // Scrolled out of sight (Ctrl+arrows, the wheel): start again from what is on screen.
  const gliding = performance.now() - movedAt < 700;
  if (from && !gliding && (from.bottom < 0 || from.top > window.innerHeight)) from = null;

  // Inside a layer the arrows stay in it, until its edge.
  const layer = origin ? layerOf(origin, root) : null;
  const pool = candidates(root, layer);
  e.preventDefault();
  let target: HTMLElement | null;
  if (from) {
    const region = origin
      ? regionOf(origin)
      : regionOf(document.elementFromPoint(from.left + from.width / 2, from.top + from.height / 2) ?? document.body);
    target = origin ? linked(origin, dir, pool) : null;
    if (!target) {
      const boxes = pool.filter((el) => el !== origin).map((el) => ({ el, r: rectOf(el) }));
      target = pick(from, dir, region, boxes, layer !== null);
    }
    // Into a page in layers from beside it (the sidebar).
    const page = target?.closest<HTMLElement>("[data-nav-page]");
    if (page && region !== page && root === document) target = startOf(page, pool) ?? target;
  } else {
    target = layer ? firstOnScreen(pool) : begin(root);
  }
  if (target) {
    move(target);
  } else if (layer instanceof HTMLElement) {
    // The edge of a layer: up one level, onto the layer itself, so an arrow
    // is a way out too and not only Escape (a game's buttons back to the game,
    // the games back to choosing between them and the bar).
    move(layer);
  } else if (dir === "up" || dir === "down") {
    // Nothing further that way: the page itself moves, so long text still reads.
    const scroller = pageScroller(origin);
    scroller?.scrollBy({ top: (dir === "down" ? 0.6 : -0.6) * scroller.clientHeight });
  }
}

/** Where the arrows begin with nothing to go from: on a page in layers its
 *  start, otherwise the page's entry in the sidebar; the control highest on
 *  screen was the title bar's version label. In a dialog, `dialogStart`. */
function begin(root: Element | Document): HTMLElement | null {
  const pool = candidates(root, null);
  if (root instanceof Element) return dialogStart(root, pool);
  const page = currentPage();
  return (page && startOf(page, pool)) || sidebarEntry() || firstOnScreen(pool);
}

/** Where a dialog takes the arrows in. What it marks `data-nav-start`; with a
 *  dangerous button in it, its way out, the first button of the footer (every
 *  footer here leads with «Cancelar»); otherwise the field to type in, if any,
 *  then the way out again. Never the dangerous one: the Enter that opened the
 *  dialog could be followed by another. */
function dialogStart(dialog: Element, pool: HTMLElement[]): HTMLElement | null {
  const start = dialog.querySelector("[data-nav-start]");
  const marked = start && pool.find((el) => start.contains(el));
  if (marked) return marked;
  const safe = pool.filter((el) => !el.matches("[data-nav-danger]"));
  const footer = dialog.querySelector("footer");
  const way = footer && safe.find((el) => footer.contains(el));
  if (safe.length < pool.length) return way || firstOnScreen(safe);
  return pool.find((el) => typing(el)) ?? way ?? firstOnScreen(pool);
}

function typing(el: Element | null): boolean {
  return (
    el instanceof HTMLTextAreaElement ||
    (el instanceof HTMLInputElement && TEXT_INPUTS.has(el.type)) ||
    (el instanceof HTMLElement && el.isContentEditable)
  );
}

/** The sidebar's pages, in order. */
function entries(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>("aside [data-nav-entry]")].filter(
    (el) => el.getBoundingClientRect().width > 0,
  );
}

/** The previous (-1) or next (1) page of the sidebar, as if its entry had been
 *  pressed: the focus goes onto the entry and then into the page once it is up.
 *  From a page with no entry (a game's history), the first or the last. */
export function navSwitchPage(step: 1 | -1): void {
  if (scope() !== document) return;
  const list = entries();
  if (list.length === 0) return;
  const at = list.findIndex((el) => el.getAttribute("aria-current") === "page");
  const next = at < 0 ? list[step > 0 ? 0 : list.length - 1] : list[(at + step + list.length) % list.length];
  move(next);
  next.click();
  followInto(next);
}

/** Into a dialog that just opened, as `dialogStart` says, when the keys are
 *  what is driving. */
export function navEnterDialog(dialog: HTMLElement): void {
  if (!keys) return;
  const start = dialogStart(dialog, candidates(dialog, null));
  if (start) move(start, false);
}

/**
 * A panel that drops over the page (the bell's notifications, the eye's
 * devices). Opened from the keys, the arrows go into it and stay in it, and
 * Escape closes it and puts them back on the button that opened it. A panel
 * with nothing to press takes the focus itself, so there is still somewhere
 * for Escape to come from.
 */
export function navPopup(node: HTMLElement, close: () => void): { destroy(): void } {
  node.setAttribute("data-nav-popup", "");
  const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  if (keys) {
    // Once the panel's own content is in.
    setTimeout(() => {
      if (!node.isConnected) return;
      const first = dialogStart(node, candidates(node, null));
      if (first) {
        move(first, false);
      } else {
        node.tabIndex = -1;
        move(node, false);
      }
    }, 0);
  }
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || e.defaultPrevented || scope() !== node) return;
    e.preventDefault();
    // Out of the arrows' way at once, not after it has faded.
    node.removeAttribute("data-nav-popup");
    close();
    if (keys && opener?.isConnected) move(opener);
  };
  window.addEventListener("keydown", onKey);
  return {
    destroy() {
      window.removeEventListener("keydown", onKey);
    },
  };
}

/** Whether the focus last moved by keyboard rather than by pointer. */
export function byKeys(): boolean {
  return keys;
}

/** What Enter and Escape do from `el`: go into a layer or press a control, and
 *  go back up or close the dialog. Null where the key does nothing of ours (a
 *  text field takes Enter; the sidebar has nowhere above it). `pages` when the
 *  bumpers can change page, `typing` in a field to type in. */
export function navActions(el: HTMLElement): {
  enter: "enter" | "select" | null;
  back: "back" | "close" | null;
  pages: boolean;
  typing: boolean;
} {
  const root = scope();
  const inField = typing(el);
  const enter = el.matches(LAYER) ? "enter" : inField || el === root ? null : "select";
  if (root !== document && root.contains(el)) {
    return { enter, back: "close", pages: false, typing: inField };
  }
  const up = layerOf(el, root) ?? (el.closest("[data-nav-page]") ? pageBack() : null);
  return { enter, back: up && up !== el ? "back" : null, pages: entries().length > 1, typing: inField };
}

/** Focus `el` as the arrows would, ring included. */
export function navFocus(el: HTMLElement): void {
  move(el);
}

export function installSpatialNav(): void {
  window.addEventListener("keydown", onKeydown);
  // The window losing focus (another app in front) is not the focus leaving:
  // the control stays focused, and unmarked it came back with no ring, so the
  // user no longer knew where the arrows were.
  document.addEventListener("focusout", (e) => {
    if (!e.relatedTarget && document.activeElement === e.target) return;
    unmark();
  });
  document.addEventListener(
    "pointerdown",
    () => {
      keys = false;
      unmark();
    },
    true,
  );
  document.addEventListener("focusin", (e) => {
    if (e.target instanceof HTMLElement && e.target.matches(FOCUSABLE)) {
      lastEl = e.target;
      lastRect = rectOf(e.target);
      lastRoute = location.hash;
      if (!e.target.closest(IN_DIALOG)) outside = e.target;
      const page = e.target.closest("[data-nav-page]");
      if (page) {
        let unit: Element = e.target;
        for (let layer = layerOf(unit); layer && page.contains(layer); layer = layerOf(layer)) unit = layer;
        lastOnPage.set(page, unit as HTMLElement);
      }
      let child: Element = e.target;
      for (let layer = layerOf(child); layer; child = layer, layer = layerOf(layer)) {
        remembered.set(layer, child as HTMLElement);
      }
    }
  });
}
