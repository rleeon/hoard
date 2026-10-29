<script lang="ts">
  /**
   * While the arrows drive the app, how to move and what Enter and Escape do
   * from where they are, at the bottom of the window as a console shows it.
   * Gone at the first touch of the pointer.
   *
   * Controller on top, keyboard underneath. Only one of the two once it is
   * known which is in the user's hands (`navInput`, from `gamepad.ts`), with
   * that controller's own labels; both while it cannot be told, as on the Deck,
   * where Steam sends the d-pad and the face buttons as plain keys.
   *
   * It sits at the bottom left of the page, clear of the sidebar, because the
   * bottom right is taken (toasts, live activity, the scale's percentage). On a
   * window too narrow for both, it rises above whatever of those is showing
   * rather than covering it.
   */
  import { Gamepad2, Keyboard } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import { navInput, type PadFamily } from "../actions/gamepad";
  import { navActions, navMarked } from "../actions/spatialNav";

  let el = $state<HTMLDivElement | null>(null);
  let left = $state(16);
  let bottom = $state(16);

  const actions = $derived($navMarked ? navActions($navMarked) : null);
  const shown = $derived(!!actions);
  const padRow = $derived($navInput.kind !== "keys");
  const keyRow = $derived($navInput.kind === "both" || $navInput.kind === "keys");
  /** The on-screen keyboard is Steam's (STEAM + X): no use to a keyboard, nor
   *  to a controller Steam is not handling. */
  const steamKeyboard = $derived(
    !!actions?.typing && ($navInput.kind === "both" || $navInput.kind === "steam"),
  );

  /** What each brand prints on the bottom and right face buttons and on the
   *  bumpers. By position: Nintendo's bottom button is its B. */
  const LABELS: Record<PadFamily, { a: string; b: string; lb: string; rb: string }> = {
    xbox: { a: "A", b: "B", lb: "LB", rb: "RB" },
    playstation: { a: "✕", b: "○", lb: "L1", rb: "R1" },
    nintendo: { a: "B", b: "A", lb: "L", rb: "R" },
    steam: { a: "A", b: "B", lb: "L1", rb: "R1" },
  };
  const pad = $derived(LABELS[$navInput.family]);

  /** The corner's occupants as drawn: a container's children, since the
   *  containers themselves span the whole column even when empty, and the
   *  scale badge only while it is faded in. */
  function obstacles(): DOMRect[] {
    const out: DOMRect[] = [];
    for (const host of document.querySelectorAll<HTMLElement>("[data-nav-hint-avoid]")) {
      const parts = host.children.length > 0 ? [...host.children] : [host];
      for (const part of parts) {
        if (Number(getComputedStyle(part).opacity) === 0) continue;
        const r = part.getBoundingClientRect();
        if (r.width > 0 && r.height > 0) out.push(r);
      }
    }
    return out;
  }

  function place() {
    if (!el) return;
    const aside = document.querySelector("aside");
    left = (aside?.getBoundingClientRect().right ?? 0) + 16;
    const w = el.offsetWidth;
    let top = window.innerHeight - 16;
    for (const r of obstacles()) {
      if (r.left < left + w + 12 && r.right > left) top = Math.min(top, r.top - 8);
    }
    bottom = window.innerHeight - top;
  }

  // Room under the page for the hint, so the last row can still scroll clear
  // of it. Never taken back: the page shrinking under the pointer as the hint
  // goes could move what a click is landing on.
  $effect(() => {
    if (shown) document.documentElement.classList.add("nav-hint-room");
  });

  $effect(() => {
    if (!shown || !el) return;
    // Placed again whenever what it shows changes its width.
    void [actions, $navInput];
    // After the labels are in, so the width measured is this one's.
    queueMicrotask(place);
    const watch = new MutationObserver(place);
    for (const host of document.querySelectorAll("[data-nav-hint-avoid]")) {
      watch.observe(host, { childList: true, subtree: true, attributes: true, characterData: true });
    }
    window.addEventListener("resize", place);
    return () => {
      watch.disconnect();
      window.removeEventListener("resize", place);
    };
  });
</script>

<div
  bind:this={el}
  class="nav-hint pointer-events-none fixed z-[55] flex items-center gap-3.5 rounded-2xl border py-2 pl-2.5 pr-3.5 text-xs text-zinc-300"
  class:is-shown={shown}
  style="left: {left}px; bottom: {bottom}px"
  aria-hidden="true"
>
  <span class="nav-hint-inputs text-zinc-400">
    {#if padRow}<Gamepad2 size={14} strokeWidth={2} />{/if}
    {#if keyRow}<Keyboard size={14} strokeWidth={2} />{/if}
  </span>
  <span class="w-px self-stretch bg-zinc-800"></span>
  <span class="flex items-center gap-2">
    <span class="nav-hint-inputs">
      {#if padRow}
        <!-- The d-pad; the left stick does the same. -->
        <svg class="nav-hint-dpad" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M6 1.5h4a.5.5 0 01.5.5v3.5H14a.5.5 0 01.5.5v4a.5.5 0 01-.5.5h-3.5V14a.5.5 0 01-.5.5H6a.5.5 0 01-.5-.5v-3.5H2a.5.5 0 01-.5-.5V6a.5.5 0 01.5-.5h3.5V2a.5.5 0 01.5-.5z"
            fill="currentColor"
          />
        </svg>
      {/if}
      {#if keyRow}<span class="nav-hint-key nav-hint-arrows">←↑↓→</span>{/if}
    </span>
    {$_("nav_hint.move")}
  </span>
  {#if actions?.enter}
    <span class="flex items-center gap-2">
      <span class="nav-hint-inputs">
        {#if padRow}<span class="nav-hint-pad">{pad.a}</span>{/if}
        {#if keyRow}<span class="nav-hint-key">Enter</span>{/if}
      </span>
      {actions.enter === "enter" ? $_("nav_hint.enter") : $_("nav_hint.select")}
    </span>
  {/if}
  {#if actions?.back}
    <span class="flex items-center gap-2">
      <span class="nav-hint-inputs">
        {#if padRow}<span class="nav-hint-pad">{pad.b}</span>{/if}
        {#if keyRow}<span class="nav-hint-key">Esc</span>{/if}
      </span>
      {actions.back === "close" ? $_("common.close") : $_("common.back")}
    </span>
  {/if}
  {#if actions?.pages}
    <span class="flex items-center gap-2">
      <span class="nav-hint-inputs">
        {#if padRow}
          <span class="flex gap-1">
            <span class="nav-hint-bumper">{pad.lb}</span><span class="nav-hint-bumper">{pad.rb}</span>
          </span>
        {/if}
        {#if keyRow}
          <span class="flex gap-1"><span class="nav-hint-key">Q</span><span class="nav-hint-key">E</span></span>
        {/if}
      </span>
      {$_("nav_hint.page")}
    </span>
  {/if}
  {#if steamKeyboard && padRow}
    <span class="flex items-center gap-2">
      <span class="nav-hint-inputs">
        <span class="nav-hint-bumper">STEAM + X</span>
        {#if keyRow}<span></span>{/if}
      </span>
      {$_("nav_hint.keyboard")}
    </span>
  {/if}
</div>

<style>
  :global(.nav-hint-room main) {
    padding-bottom: 84px;
  }
  /* Faded, never slid: an animated transform flashes black on WebKitGTK. */
  .nav-hint {
    background: oklch(0.09 0 0 / 0.97);
    border-color: oklch(0.24 0 0);
    box-shadow: 0 8px 24px -8px oklch(0 0 0 / 0.8);
    opacity: 0;
    transition: opacity 200ms ease-out;
  }
  .nav-hint.is-shown {
    opacity: 1;
  }
  /* Two rows that line up across the whole hint: the controller's, then the
     keyboard's. */
  .nav-hint-inputs {
    display: grid;
    grid-auto-rows: 16px;
    justify-items: center;
    align-items: center;
    row-gap: 4px;
  }
  .nav-hint-key {
    display: inline-flex;
    align-items: center;
    height: 16px;
    border-radius: 5px;
    padding: 0 5px;
    font-size: 10px;
    font-weight: 600;
    color: oklch(0.85 0 0);
    background: oklch(0.2 0 0);
    box-shadow:
      inset 0 -1px 0 oklch(0 0 0 / 0.5),
      inset 0 0 0 1px oklch(1 0 0 / 0.08);
  }
  .nav-hint-arrows {
    letter-spacing: 1px;
  }
  /* A and B are the Deck's face buttons, which Steam sends as Enter and Escape. */
  .nav-hint-pad {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 9999px;
    font-size: 9px;
    font-weight: 700;
    color: oklch(0.12 0 0);
    background: oklch(0.88 0 0);
  }
  .nav-hint-bumper {
    display: inline-flex;
    align-items: center;
    height: 16px;
    border-radius: 9999px;
    padding: 0 6px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: oklch(0.12 0 0);
    background: oklch(0.88 0 0);
  }
  .nav-hint-dpad {
    color: oklch(0.88 0 0);
  }
</style>
