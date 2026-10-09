<script lang="ts" module>
  export type SelectOption = {
    value: string;
    label: string;
    /** Quiet text on the right of the row: an emulator's system, why a slot
     *  number is taken. Only the list shows it. */
    hint?: string;
    disabled?: boolean;
  };

  let nextId = 0;
</script>

<script lang="ts">
  /**
   * The app's dropdown, in place of `<select>`.
   *
   * A native select only lets the closed box be styled. The list that opens is
   * drawn by the system: a GTK menu in Ubuntu's theme and font on Linux, a
   * Win32 one on Windows, and the box wore a green focus ring on every click.
   * This one is ours from end to end and looks the same on every platform.
   *
   * Focus never leaves the button. The list is a `listbox` the button points
   * into with `aria-activedescendant`, so the keyboard works as on a native
   * select (arrows, Home/End, Enter, Esc, typing to jump) with no focus dance.
   * The list is moved to `<body>` and placed with fixed coordinates: a modal's
   * transform or a card's `overflow: hidden` would otherwise clip it.
   */
  import { tick } from "svelte";
  import { cubicOut } from "svelte/easing";

  type Props = {
    value?: string;
    options: SelectOption[];
    onchange?: (value: string) => void;
    placeholder?: string;
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    id?: string;
    title?: string;
    "aria-label"?: string;
    class?: string;
  };
  let {
    value = $bindable(""),
    options,
    onchange,
    placeholder = "",
    size = "md",
    disabled = false,
    id,
    title,
    "aria-label": ariaLabel,
    class: klass = "",
  }: Props = $props();

  const uid = `hd-select-${nextId++}`;

  // `lg` is the height of `Input`, for the places where the two sit side by side.
  const TRIGGER = {
    sm: "h-8 pl-2.5 pr-2 text-xs",
    md: "h-9 pl-3 pr-2.5 text-sm",
    lg: "h-[42px] pl-3 pr-3 text-sm",
  };
  const ROW_TEXT = { sm: "text-xs", md: "text-sm", lg: "text-sm" };
  // Rows are a fixed 32 px and the list has 4 px of padding and a 1 px border,
  // so its height is known before it exists and the first frame already opens
  // on the right side of the button.
  const ROW = 32;
  const CHROME = 10;
  const GAP = 6;
  const EDGE = 8;

  let open = $state(false);
  let active = $state(-1);
  let trigger = $state<HTMLButtonElement | null>(null);
  let panel = $state<HTMLUListElement | null>(null);
  let pos = $state({ top: 0, left: 0, minWidth: 0, maxHeight: 320, above: false });

  const selectedIndex = $derived(options.findIndex((o) => o.value === value));
  const current = $derived(selectedIndex >= 0 ? options[selectedIndex] : null);

  function place() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const natural = options.length * ROW + CHROME;
    const below = window.innerHeight - r.bottom - GAP - EDGE;
    const aboveRoom = r.top - GAP - EDGE;
    const above = natural > below && aboveRoom > below;
    const maxHeight = Math.max(ROW * 3, Math.min(320, above ? aboveRoom : below));
    const height = Math.min(natural, maxHeight);
    const width = Math.max(r.width, panel?.offsetWidth ?? 0);
    // A list wider than its button grows away from the nearer window edge, so
    // a button on the right keeps both edges of the pair lined up.
    const onRight = r.left + r.width / 2 > window.innerWidth / 2;
    const left = onRight ? r.right - width : r.left;
    pos = {
      top: above ? r.top - GAP - height : r.bottom + GAP,
      left: Math.max(EDGE, Math.min(left, window.innerWidth - width - EDGE)),
      minWidth: r.width,
      maxHeight,
      above,
    };
  }

  function step(from: number, dir: 1 | -1): number {
    for (let i = from; i >= 0 && i < options.length; i += dir) {
      if (!options[i].disabled) return i;
    }
    return -1;
  }

  function reveal() {
    panel
      ?.querySelector<HTMLElement>(`#${uid}-${active}`)
      ?.scrollIntoView({ block: "nearest" });
  }

  async function show() {
    if (disabled || open) return;
    place();
    active = selectedIndex >= 0 ? selectedIndex : step(0, 1);
    open = true;
    await tick();
    // Second pass with the real width, for a list wider than its button near
    // the right edge of the window.
    place();
    reveal();
  }

  function hide(refocus = false) {
    open = false;
    typed = "";
    if (refocus) trigger?.focus();
  }

  function choose(i: number) {
    const o = options[i];
    if (!o || o.disabled) return;
    hide(true);
    if (o.value === value) return;
    value = o.value;
    onchange?.(o.value);
  }

  function move(to: number) {
    if (to < 0) return;
    active = to;
    reveal();
  }

  // Type to jump, as a native select does: letters typed within 600 ms of each
  // other add up, accents and case aside ("es" finds «Español»).
  let typed = "";
  let typedAt = 0;
  const fold = (s: string) =>
    s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
  function typeahead(ch: string) {
    const now = Date.now();
    typed = now - typedAt > 600 ? ch : typed + ch;
    typedAt = now;
    const q = fold(typed);
    const from = typed.length === 1 ? active + 1 : Math.max(active, 0);
    for (let k = 0; k < options.length; k++) {
      const i = (from + k) % options.length;
      if (!options[i].disabled && fold(options[i].label).startsWith(q)) {
        move(i);
        return;
      }
    }
  }

  function onKey(e: KeyboardEvent) {
    if (disabled) return;
    switch (e.key) {
      case "ArrowDown":
      case "ArrowUp": {
        e.preventDefault();
        if (!open) void show();
        else if (e.key === "ArrowDown") move(step(active + 1, 1));
        else move(step(active - 1, -1));
        return;
      }
      case "Home":
      case "End":
        if (!open) return;
        e.preventDefault();
        move(e.key === "Home" ? step(0, 1) : step(options.length - 1, -1));
        return;
      case "Enter":
      case " ":
        if (e.key === " " && open && typed) {
          e.preventDefault();
          typeahead(" ");
          return;
        }
        e.preventDefault();
        if (!open) void show();
        else if (active >= 0) choose(active);
        return;
      case "Escape":
        if (!open) return;
        // A modal closes on Escape too; this one is ours.
        e.preventDefault();
        e.stopPropagation();
        hide(true);
        return;
      case "Tab":
        if (open) hide();
        return;
      default:
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
          if (open) typeahead(e.key);
          else void show().then(() => typeahead(e.key));
        }
    }
  }

  // Only the pointer opens from a click: the keyboard's Enter and Space are
  // handled above, and their synthetic click (`detail === 0`) would toggle the
  // list straight back.
  function onClick(e: MouseEvent) {
    if (e.detail === 0) return;
    if (open) hide();
    else void show();
  }

  $effect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => {
      const t = e.target as Node;
      if (trigger?.contains(t) || panel?.contains(t)) return;
      hide();
    };
    // Anything else scrolling would leave the list floating away from its
    // button; a native select closes too.
    const scrolled = (e: Event) => {
      if (panel && e.target instanceof Node && panel.contains(e.target)) return;
      hide();
    };
    const away = () => hide();
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("scroll", scrolled, true);
    window.addEventListener("resize", away);
    window.addEventListener("blur", away);
    return () => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("scroll", scrolled, true);
      window.removeEventListener("resize", away);
      window.removeEventListener("blur", away);
    };
  });

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  function rise(_: Element, { above }: { above: boolean }) {
    return {
      duration: 150,
      easing: cubicOut,
      css: (t: number) =>
        `opacity:${t};transform:translateY(${(1 - t) * (above ? 4 : -4)}px) scale(${0.985 + 0.015 * t})`,
    };
  }
</script>

<button
  bind:this={trigger}
  type="button"
  {id}
  {title}
  {disabled}
  role="combobox"
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-controls={open ? `${uid}-list` : undefined}
  aria-activedescendant={open && active >= 0 ? `${uid}-${active}` : undefined}
  aria-label={ariaLabel}
  class="inline-flex items-center justify-between gap-2.5 rounded-md border bg-layer-2 text-left outline-none transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50 {TRIGGER[
    size
  ]} {open
    ? 'border-white/[0.26] text-zinc-50'
    : 'border-white/[0.09] text-zinc-200 hover:border-white/[0.18] focus-visible:border-white/[0.3]'} {klass}"
  onclick={onClick}
  onkeydown={onKey}
>
  <!-- Every label stacked in one cell, invisible: the button takes the width
       of the longest and does not jump when the choice changes. -->
  <span class="grid min-w-0 flex-1">
    {#each options as o (o.value)}
      <span class="invisible col-start-1 row-start-1 truncate" aria-hidden="true">{o.label}</span>
    {/each}
    {#if placeholder}
      <span class="invisible col-start-1 row-start-1 truncate" aria-hidden="true">{placeholder}</span>
    {/if}
    <span class="col-start-1 row-start-1 truncate {current ? '' : 'text-zinc-500'}">
      {current?.label ?? placeholder}
    </span>
  </span>
  <svg
    class="hd-select-chevron shrink-0 text-zinc-500"
    class:hd-select-chevron-open={open}
    width="12"
    height="12"
    viewBox="0 0 12 12"
    fill="none"
    aria-hidden="true"
  >
    <path
      d="M3 4.6 6 7.6 9 4.6"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</button>

{#if open}
  <ul
    use:portal
    bind:this={panel}
    in:rise={{ above: pos.above }}
    id="{uid}-list"
    role="listbox"
    tabindex="-1"
    aria-label={ariaLabel}
    class="hd-select-panel fixed z-[350] overflow-y-auto rounded-lg border border-white/[0.09] p-1"
    style="top:{pos.top}px;left:{pos.left}px;min-width:{pos.minWidth}px;max-height:{pos.maxHeight}px;transform-origin:{pos.above
      ? 'bottom'
      : 'top'}"
    onmousedown={(e) => e.preventDefault()}
  >
    {#each options as o, i (o.value)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li
        id="{uid}-{i}"
        role="option"
        aria-selected={i === selectedIndex}
        aria-disabled={o.disabled || undefined}
        class="flex h-8 cursor-default select-none items-center gap-3 rounded-md border pl-2.5 pr-2 {ROW_TEXT[
          size
        ]} {i === active && !o.disabled
          ? 'border-white/[0.12]'
          : 'border-transparent'} {o.disabled
          ? 'text-zinc-600'
          : i === selectedIndex
            ? 'text-zinc-50'
            : 'text-zinc-300'}"
        onpointermove={() => {
          if (!o.disabled) active = i;
        }}
        onclick={() => choose(i)}
      >
        <span class="min-w-0 flex-1 truncate">{o.label}</span>
        {#if o.hint}
          <span class="shrink-0 text-[11px] text-zinc-500">{o.hint}</span>
        {/if}
        <svg
          class="shrink-0 text-emerald-400 {i === selectedIndex ? '' : 'invisible'}"
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M3.2 7.4 5.8 10 10.8 4.2"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </li>
    {/each}
  </ul>
{/if}

<style>
  /* Opaque on purpose: the app's surfaces are translucent and rely on a blur
     that Linux does not get, and a list over a cover or a log has to read on
     its own. The top line of light and a black shadow, never a coloured one. */
  .hd-select-panel {
    background: oklch(0.075 0.006 var(--tint-hue));
    box-shadow:
      var(--edge-top),
      0 18px 40px -14px oklch(0 0 0 / 0.85),
      0 4px 12px -6px oklch(0 0 0 / 0.6);
  }
  .hd-select-chevron {
    transition:
      transform 200ms cubic-bezier(0.2, 0.8, 0.2, 1),
      color 150ms;
  }
  .hd-select-chevron-open {
    transform: rotate(180deg);
    color: var(--color-zinc-300);
  }
  /* Same exception as the warnings tray in app.css: reduced motion is on for
     anyone with system animations off, and a list appearing in one frame
     reads as a jump, not as calm. */
  @media (prefers-reduced-motion: reduce) {
    .hd-select-panel {
      animation-duration: 150ms !important;
    }
    .hd-select-chevron {
      transition-duration: 200ms, 150ms !important;
    }
  }
</style>
