<script lang="ts">
  /**
   * Drag-to-resize handle for the card grids.
   *
   * Place inside a card element (the card must be `relative`). On pointerdown
   * the user drags to change the minimum column width for the entire section.
   * The parent grid must use
   * `grid-template-columns: repeat(auto-fill, minmax(var(--card-w), 1fr))`
   * and read from the `cardSizes` store.
   *
   * The vertical axis is opt-in: pass `onVerticalDrag` and the same drag also
   * reports how far down the pointer went, which the Dashboard turns into the
   * cover's shape (2:3 ↔ square ↔ landscape). Library grids leave it out, their
   * height is whatever the content needs, and behave exactly as before.
   *
   * Usage:
   *   <CardResizeHandle section="tracked" />
   *   <CardResizeHandle section="dashboard" onVerticalDrag={(dy, w) => …} />
   */
  import type { SectionKey } from "../stores/cardSizes.svelte";
  import { setCardWidth, cardWidth } from "../stores/cardSizes.svelte";

  let {
    section,
    onVerticalDrag,
    title,
  }: {
    section: SectionKey;
    /**
     * Optional. Called on every move with the pixels dragged down since the
     * start and the card's laid-out width at that moment, enough to turn the
     * drag into a ratio without measuring anything mid-gesture.
     */
    onVerticalDrag?: (dy: number, startCardWidth: number) => void;
    title?: string;
  } = $props();

  let el = $state<HTMLSpanElement | null>(null);
  let dragging = $state(false);
  let startX = $state(0);
  let startY = $state(0);
  let startW = $state(0);
  let startCardW = $state(0);

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragging = true;
    startX = e.clientX;
    startY = e.clientY;
    startW = cardWidth(section);
    // The positioned container's `offsetWidth` (the card), not
    // `getBoundingClientRect`: the card is tilted by `use:tilt` and the rect would
    // come from the already-transformed bbox, a few points wider.
    const card = el?.offsetParent as HTMLElement | null;
    startCardW = card?.offsetWidth || startW;
    document.body.style.cursor = "nwse-resize";
    document.body.style.userSelect = "none";
    // An opening notice with dy=0: whoever listens anchors its starting value there
    // and no longer depends on where the first pointermove lands.
    onVerticalDrag?.(0, startCardW);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - startX;
    // Each px of drag = 1px of card width change.
    setCardWidth(section, startW + dx);
    onVerticalDrag?.(e.clientY - startY, startCardW);
  }

  function onPointerUp() {
    dragging = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  bind:this={el}
  class="absolute bottom-0 right-0 z-10 h-4 w-4 cursor-nwse-resize opacity-0 transition-opacity group-hover:opacity-100"
  onpointerdown={onPointerDown}
  {title}
>
  <!-- Three diagonal lines indicating resize -->
  <svg
    viewBox="0 0 16 16"
    class="h-full w-full text-zinc-500 group-hover:text-zinc-400"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
  >
    <line x1="11" y1="5" x2="5" y2="11" />
    <line x1="14" y1="8" x2="8" y2="14" />
    <line x1="14" y1="11" x2="11" y2="14" />
  </svg>
</span>
