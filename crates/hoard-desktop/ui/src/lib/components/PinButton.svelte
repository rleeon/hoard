<script lang="ts">
  /**
   * The pin of a game. On, the game leads the Library and the Panel, and the
   * number beside it is its place among the pinned ones: typing another moves
   * it there and the rest make room.
   *
   * `icon` is the Library's button, `menu` the row inside the Panel's overflow
   * menu.
   */
  import { tick } from "svelte";
  import { Pin } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import { byKeys, navFocus } from "../actions/spatialNav";
  import { pinPosition, setPinPosition, togglePin } from "../stores/pins.svelte";

  let {
    slug,
    variant = "icon",
    onToggle,
  }: {
    slug: string;
    variant?: "icon" | "menu";
    onToggle?: (pinned: boolean) => void;
  } = $props();

  const position = $derived(pinPosition(slug));
  let button = $state<HTMLButtonElement | null>(null);
  let field = $state<HTMLInputElement | null>(null);

  // Pinning or renumbering moves the card in the grid, and a node that moves
  // loses the focus and may land off screen. Go after it, and leave the number
  // ready to be typed over. With the arrows or a controller the focus goes
  // back to the button instead: a text field there is a trap.
  async function follow() {
    await tick();
    const card = button?.closest<HTMLElement>("[data-card]");
    const scroller = card?.closest("main");
    if (card && scroller) {
      // By hand and only the page's scroller: `scrollIntoView` also moves
      // every ancestor that can scroll, the window under the title bar included.
      const c = card.getBoundingClientRect();
      const s = scroller.getBoundingClientRect();
      const margin = 16;
      let dy = 0;
      if (c.top < s.top + margin) dy = c.top - s.top - margin;
      else if (c.bottom > s.bottom - margin) {
        dy = Math.min(c.bottom - s.bottom + margin, c.top - s.top - margin);
      }
      if (dy) scroller.scrollBy({ top: dy, behavior: "smooth" });
    }
    if (byKeys()) {
      if (button) navFocus(button);
    } else if (field) {
      field.focus({ preventScroll: true });
      field.select();
    }
  }

  function toggle() {
    const pinning = !position;
    togglePin(slug);
    onToggle?.(pinning);
    if (pinning) void follow();
  }

  function commit() {
    if (!field) return;
    const n = parseInt(field.value, 10);
    const before = position;
    if (n >= 1) setPinPosition(slug, n);
    // Whatever was typed, the field shows the place the game ended up in.
    field.value = String(pinPosition(slug));
    if (pinPosition(slug) !== before) void follow();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commit();
      field?.select();
    } else if (e.key === "Escape" && field) {
      field.value = String(position);
      field.blur();
    }
  }
</script>

<!-- `field-inner`: the ring is drawn by the box around the number, not on it. -->
{#snippet place(cls: string)}
  <input
    bind:this={field}
    type="text"
    inputmode="numeric"
    maxlength="3"
    value={position}
    aria-label={$_("pin.position")}
    title={$_("pin.position")}
    class="field-inner {cls}"
    onfocus={() => field?.select()}
    oninput={() => field && (field.value = field.value.replace(/\D/g, ""))}
    onchange={commit}
    onkeydown={onKey}
  />
{/snippet}

{#if variant === "menu"}
  <div class="flex items-center gap-1 rounded-md pr-1.5 transition-colors hover:bg-white/[0.06]">
    <button
      bind:this={button}
      type="button"
      class="flex min-w-0 flex-1 items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs text-zinc-300 transition-colors hover:text-zinc-50"
      onclick={toggle}
    >
      <Pin
        size={13}
        class="shrink-0 {position ? 'text-emerald-400' : 'text-zinc-500'}"
        fill={position ? "currentColor" : "none"}
      />
      <span class="truncate">{position ? $_("pin.remove") : $_("pin.add")}</span>
    </button>
    {#if position}
      {@render place(
        "h-5 w-7 shrink-0 rounded border border-emerald-500/40 bg-transparent text-center text-[11px] tabular-nums text-emerald-300 outline-none transition-colors focus:border-emerald-400",
      )}
    {/if}
  </div>
{:else}
  <span
    class="inline-flex shrink-0 items-center rounded-md border transition-colors {position
      ? 'border-emerald-500/40 focus-within:border-emerald-400 hover:border-emerald-500/70'
      : 'border-transparent'}"
  >
    <button
      bind:this={button}
      type="button"
      onclick={toggle}
      aria-pressed={position > 0}
      aria-label={position ? $_("pin.remove") : $_("pin.add")}
      title={position ? $_("pin.remove") : $_("pin.add")}
      class="rounded p-1.5 transition-colors {position
        ? 'text-emerald-400'
        : 'text-zinc-500 hover:bg-zinc-700/40 hover:text-zinc-200'}"
    >
      <Pin size={14} fill={position ? "currentColor" : "none"} />
    </button>
    {#if position}
      {@render place(
        "w-6 bg-transparent pr-1.5 text-center text-xs tabular-nums text-emerald-300 outline-none",
      )}
    {/if}
  </span>
{/if}
