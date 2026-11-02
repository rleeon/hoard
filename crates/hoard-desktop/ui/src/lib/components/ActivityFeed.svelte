<script lang="ts">
  /**
   * Floating bottom-right panel listing recent `agent://*` activity.
   *
   * Toggled from the sidebar (📜 button) and persisted via
   * `prefs.live_activity_visible`. Read-only, the entries come from
   * `live.ts`'s `activityFeed` circular buffer.
   *
   * We render at most 50 visible rows. The buffer holds more (80) so
   * scrolling history isn't truncated mid-session; the cap above is a
   * cheap CSS overflow cutoff.
   *
   * The user can drag it by its header and resize it from any edge, and it
   * comes back where it was left (`liveActivityLayout`). A Hoard-help report
   * (01-11-2026) could not read the rows at all: each was cut to one line in a
   * 22rem box that nothing could widen, and the line it lost was the fix
   * (`max_snapshot_size_mb`). Rows now show two lines and open on demand.
   */
  import { _ } from "svelte-i18n";
  import { feedRelativeTime, feedSummary } from "../utils/feedText";
  import { fly } from "svelte/transition";
  import {
    Eye,
    Play,
    Square,
    Clock,
    UploadCloud,
    CheckCircle2,
    XCircle,
    Download,
    RefreshCcw,
    Ban,
    WifiOff,
    Scissors,
    FileWarning,
    Trash2,
    HardDrive,
    AlertTriangle,
    X,
  } from "@lucide/svelte";

  import { activityFeed, type FeedEntry } from "../stores/live";
  import { openLiberate } from "../stores/liberate";
  import { byKeys, navFocus } from "../actions/spatialNav";
  import { liveActivityEnter } from "../stores/panels";
  import {
    EDGE_GAP,
    MIN_HEIGHT,
    MIN_WIDTH,
    dockLiveActivity,
    liveActivityLayout,
    setLiveActivityLayout,
  } from "../stores/liveActivityLayout.svelte";
  import * as api from "../api";
  import { formatBytes } from "../utils/format";

  /** Hide affordance. Persisted via prefs so the next boot remembers. */
  let { onClose }: { onClose: () => void } = $props();

  const ICONS = {
    watcher_armed: Eye,
    game_started: Play,
    game_stopped: Square,
    throttled: Clock,
    upload_started: UploadCloud,
    upload_completed: CheckCircle2,
    upload_failed: XCircle,
    bandwidth_throttled: Clock,
    auto_restored: Download,
    cloud_pull: RefreshCcw,
    quota_reached: Ban,
    offline: WifiOff,
    online: CheckCircle2,
    backup_too_large: XCircle,
    backup_quota_full: HardDrive,
    backup_trimmed: Scissors,
    backup_files_unreadable: FileWarning,
    auto_restore_failed: XCircle,
    auto_restore_stuck: AlertTriangle,
    auto_restore_recovered: CheckCircle2,
    backup_blocked: AlertTriangle,
    backup_unblocked: CheckCircle2,
    storage_purging: Trash2,
    storage_full: AlertTriangle,
    storage_grace: Clock,
  } as const;

  const TINTS = {
    watcher_armed: "text-emerald-400",
    game_started: "text-emerald-300",
    game_stopped: "text-zinc-400",
    throttled: "text-amber-300",
    upload_started: "text-emerald-300",
    upload_completed: "text-emerald-400",
    upload_failed: "text-red-400",
    bandwidth_throttled: "text-amber-300",
    auto_restored: "text-sky-300",
    cloud_pull: "text-emerald-300",
    quota_reached: "text-amber-400",
    offline: "text-red-400",
    online: "text-emerald-400",
    backup_too_large: "text-red-400",
    backup_quota_full: "text-red-400",
    backup_trimmed: "text-amber-300",
    backup_files_unreadable: "text-amber-300",
    auto_restore_failed: "text-red-400",
    auto_restore_stuck: "text-amber-400",
    auto_restore_recovered: "text-emerald-400",
    backup_blocked: "text-red-400",
    backup_unblocked: "text-emerald-400",
    storage_purging: "text-amber-400",
    storage_full: "text-red-400",
    storage_grace: "text-sky-300",
  } as const;

  // Alert rows get a tinted "card" so plan-limit / storage-pressure events
  // read at a glance: amber for reversible pressure (trimming / purging),
  // red for a hard stop (over-cap upload, restore failure, storage full).
  const ROW_ACCENT: Partial<Record<FeedEntry["kind"], string>> = {
    backup_too_large: "my-1 rounded-md border border-red-500/60 bg-red-500/10",
    auto_restore_failed:
      "my-1 rounded-md border border-red-500/60 bg-red-500/10",
    // Amber, not red: the save still syncs once the cause clears, and the
    // agent keeps retrying on the escalating backoff.
    auto_restore_stuck:
      "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_full: "my-1 rounded-md border border-red-500/60 bg-red-500/10",
    // Red, not amber: unlike `auto_restore_stuck`, there is no retry waiting here.
    // Without a person, this save never uploads again.
    backup_blocked: "my-1 rounded-md border border-red-500/60 bg-red-500/10",
    backup_quota_full:
      "my-1 rounded-md border border-red-500/60 bg-red-500/10",
    backup_trimmed: "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    // Amber: the backup is usable, it is just missing a piece. The day nothing
    // uploads, `upload_failed`'s red row appears next to it and that one rules.
    backup_files_unreadable:
      "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_purging: "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_grace: "my-1 rounded-md border border-sky-500/60 bg-sky-500/10",
  };

  /** Rows whose problem the user can actually do something about from here. */
  const ACTIONABLE = new Set<FeedEntry["kind"]>([
    "backup_quota_full",
    "storage_full",
  ]);

  const relativeTime = (at: number) => feedRelativeTime(at, $_);
  const summary = (e: FeedEntry) => feedSummary(e, $_);

  async function hidePanel() {
    // Closed from the keys, they go back to the button that opened it rather
    // than nowhere.
    if (byKeys()) backOut();
    try {
      await api.setLiveActivityVisible(false);
    } catch (err) {
      console.warn("setLiveActivityVisible failed:", err);
    }
    onClose();
  }

  // ---- arrows
  //
  // The panel is a layer (`spatialNav`): its rows, its "show more" buttons and
  // its close button, and the arrows stay among them. Each row is a stop, so
  // Down walks the whole list and the list scrolls with it, not only the rows
  // that happen to carry a button. The way in is the button that toggles it
  // (`toggleLiveActivity`); Escape on the panel itself goes back there.

  let opener: HTMLElement | null = null;

  function backOut(): void {
    const to = opener?.isConnected
      ? opener
      : document.querySelector<HTMLElement>("[data-activity-toggle]");
    if (to && to.getBoundingClientRect().width > 0) navFocus(to);
  }

  $effect(() =>
    liveActivityEnter.subscribe((req) => {
      if (!req) return;
      opener = req.opener;
      liveActivityEnter.set(null);
      // Once the rows are drawn: the panel may have mounted with this request.
      setTimeout(() => {
        const first =
          aside?.querySelector<HTMLElement>("li[data-nav-stop]") ??
          aside?.querySelector<HTMLElement>("header button");
        if (first) navFocus(first);
      }, 0);
    }),
  );

  function onPanelKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && document.activeElement === aside) {
      e.preventDefault();
      backOut();
    }
  }

  function onRowKeydown(e: KeyboardEvent, id: number) {
    if (e.key !== "Enter" || e.target !== e.currentTarget) return;
    if (!expanded[id] && !overflowing[id]) return;
    e.preventDefault();
    expanded[id] = !expanded[id];
  }

  // Tick once a second so relative timestamps stay honest. Wrapped in a
  // counter so Svelte 5 notices the change. Skipped while the window is hidden
  //, no visible timestamps to keep honest, so don't force re-renders.
  let tick = $state(0);
  let timer: ReturnType<typeof setInterval>;
  $effect(() => {
    timer = setInterval(() => {
      if (!document.hidden) tick = tick + 1;
    }, 1000);
    return () => clearInterval(timer);
  });
  const _tickRef = $derived(tick); // touched in summary() via relativeTime call below

  // ---- long rows: two lines, then "show more"

  let overflowing = $state<Record<number, boolean>>({});
  let expanded = $state<Record<number, boolean>>({});

  /** Whether the clamped text runs past its two lines. Measured again whenever
   *  the row changes size, since widening the panel can make it fit. */
  function clampWatch(node: HTMLElement, id: number) {
    const measure = () => {
      const over = node.scrollHeight > node.clientHeight + 1;
      if (over !== Boolean(overflowing[id])) overflowing[id] = over;
    };
    const ro = new ResizeObserver(measure);
    ro.observe(node);
    measure();
    return { destroy: () => ro.disconnect() };
  }

  // ---- moving and resizing

  type Rect = { left: number; top: number; width: number; height: number };
  type Edge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

  /** Closest the panel gets to a window edge when it floats. */
  const MARGIN = 8;
  /** Let go this close to the corner and it docks again. */
  const SNAP = 24;
  const DRAG_THRESHOLD = 4;

  // Mostly outside the card, so the right one does not sit on the scrollbar.
  const EDGES: { edge: Edge; class: string }[] = [
    { edge: "n", class: "-top-[5px] left-3 right-3 h-[6px] cursor-ns-resize" },
    { edge: "s", class: "-bottom-[5px] left-3 right-3 h-[6px] cursor-ns-resize" },
    { edge: "e", class: "-right-[5px] top-3 bottom-3 w-[6px] cursor-ew-resize" },
    { edge: "w", class: "-left-[5px] top-3 bottom-3 w-[6px] cursor-ew-resize" },
    { edge: "nw", class: "-left-[5px] -top-[5px] h-4 w-4 cursor-nwse-resize" },
    { edge: "se", class: "-right-[5px] -bottom-[5px] h-4 w-4 cursor-nwse-resize" },
    { edge: "ne", class: "-right-[5px] -top-[5px] h-4 w-4 cursor-nesw-resize" },
    { edge: "sw", class: "-left-[5px] -bottom-[5px] h-4 w-4 cursor-nesw-resize" },
  ];

  const layout = $derived(liveActivityLayout());
  let vw = $state(window.innerWidth);
  let vh = $state(window.innerHeight);
  let boxW = $state(0);
  let boxH = $state(0);
  let aside = $state<HTMLElement | null>(null);
  /** The rect while a drag or a resize is under way; `null` otherwise. */
  let live = $state<Rect | null>(null);
  let stopGesture: (() => void) | null = null;

  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

  /** The custom title bar (Windows, Linux) is not somewhere to leave it. */
  function topLimit(): number {
    const bar = document.querySelector(".titlebar");
    return (bar ? bar.getBoundingClientRect().bottom : 0) + MARGIN;
  }

  const sized = $derived(live !== null || layout.height !== null);

  const boxStyle = $derived.by(() => {
    if (live) {
      return `position:fixed;left:${live.left}px;top:${live.top}px;width:${live.width}px;height:${live.height}px;`;
    }
    const top = topLimit();
    const height =
      layout.height === null
        ? ""
        : `height:min(${layout.height}px, calc(100vh - ${top + EDGE_GAP}px));`;
    if (layout.docked) return height;
    const width =
      layout.width === null
        ? "min(22rem, calc(100vw - 2rem))"
        : `min(${layout.width}px, calc(100vw - ${2 * MARGIN}px))`;
    // A window that shrank must not leave the far edge outside it.
    const x = clamp(layout.x, MARGIN, vw - boxW - MARGIN);
    const y =
      layout.anchorY === "top"
        ? clamp(layout.y, top, vh - boxH - MARGIN)
        : clamp(layout.y, MARGIN, vh - boxH - top);
    return `position:fixed;${layout.anchorX}:${x}px;${layout.anchorY}:${y}px;width:${width};max-height:calc(100vh - ${top + MARGIN}px);${height}`;
  });

  function gesture(cursor: string, move: (ev: PointerEvent) => void, end: () => void) {
    document.body.style.cursor = cursor;
    document.body.style.userSelect = "none";
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      stopGesture = null;
    };
    const up = () => {
      stop();
      end();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    stopGesture = stop;
  }

  $effect(() => () => stopGesture?.());

  /** Saves where the gesture left it, anchored to the nearest window corner.
   *  `keepDocked`: a resize that did not move the bottom-right corner. */
  function settle(touched: { width?: boolean; height?: boolean }, keepDocked: boolean) {
    const r = live;
    live = null;
    if (!r) return;
    const size = {
      ...(touched.width ? { width: Math.round(r.width) } : {}),
      ...(touched.height ? { height: Math.round(r.height) } : {}),
    };
    const nearDock =
      vw - (r.left + r.width) <= EDGE_GAP + SNAP && vh - (r.top + r.height) <= EDGE_GAP + SNAP;
    if (keepDocked || (nearDock && !touched.width && !touched.height)) {
      setLiveActivityLayout(size);
      if (!keepDocked) dockLiveActivity();
      return;
    }
    const anchorX = r.left + r.width / 2 < vw / 2 ? "left" : "right";
    const anchorY = r.top + r.height / 2 < vh / 2 ? "top" : "bottom";
    setLiveActivityLayout({
      ...size,
      docked: false,
      anchorX,
      anchorY,
      x: Math.round(anchorX === "left" ? r.left : vw - r.left - r.width),
      y: Math.round(anchorY === "top" ? r.top : vh - r.top - r.height),
    });
  }

  function onHeaderPointerDown(e: PointerEvent) {
    if (e.button !== 0 || !aside) return;
    if ((e.target as HTMLElement).closest("button")) return;
    const r = aside.getBoundingClientRect();
    const sx = e.clientX;
    const sy = e.clientY;
    let moved = false;
    gesture(
      "grabbing",
      (ev) => {
        const dx = ev.clientX - sx;
        const dy = ev.clientY - sy;
        // Below this it is a click (or the first half of a double click), and
        // a click must not undock it.
        if (!moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
        moved = true;
        live = {
          left: clamp(r.left + dx, MARGIN, vw - r.width - MARGIN),
          top: clamp(r.top + dy, topLimit(), vh - r.height - MARGIN),
          width: r.width,
          height: r.height,
        };
      },
      () => {
        if (moved) settle({}, false);
      },
    );
  }

  function onEdgePointerDown(e: PointerEvent, edge: Edge) {
    if (e.button !== 0 || !aside) return;
    e.preventDefault();
    e.stopPropagation();
    const r = aside.getBoundingClientRect();
    const sx = e.clientX;
    const sy = e.clientY;
    const wasDocked = layout.docked;
    const cursor = getComputedStyle(e.currentTarget as HTMLElement).cursor;
    live = { left: r.left, top: r.top, width: r.width, height: r.height };
    gesture(
      cursor,
      (ev) => {
        const dx = ev.clientX - sx;
        const dy = ev.clientY - sy;
        let { left, top, width, height } = r;
        if (edge.includes("e")) width = clamp(r.width + dx, MIN_WIDTH, vw - MARGIN - r.left);
        if (edge.includes("s")) height = clamp(r.height + dy, MIN_HEIGHT, vh - MARGIN - r.top);
        if (edge.includes("w")) {
          width = clamp(r.width - dx, MIN_WIDTH, r.right - MARGIN);
          left = r.right - width;
        }
        if (edge.includes("n")) {
          height = clamp(r.height - dy, MIN_HEIGHT, r.bottom - topLimit());
          top = r.bottom - height;
        }
        live = { left, top, width, height };
      },
      () =>
        settle(
          { width: /[ew]/.test(edge), height: /[ns]/.test(edge) },
          wasDocked && !/[es]/.test(edge),
        ),
    );
  }
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

<!-- Docked, positioning belongs to the bottom-right stack in App.svelte: the
     storage banner shares that corner and the two must not sit on top of each
     other. Floating, or mid-gesture, the card places itself (`boxStyle`). -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside
  bind:this={aside}
  bind:offsetWidth={boxW}
  bind:offsetHeight={boxH}
  class="pointer-events-auto relative flex w-full flex-col rounded-lg border border-zinc-800 bg-layer-3 shadow-xl"
  style={boxStyle}
  aria-label={$_("activity.panel_label")}
  data-nav-layer
  tabindex="-1"
  onkeydown={onPanelKeydown}
  transition:fly={{ y: 12, duration: 180 }}
>
  {#each EDGES as h (h.edge)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      aria-hidden="true"
      class="absolute z-10 {h.class}"
      onpointerdown={(e) => onEdgePointerDown(e, h.edge)}
    ></span>
  {/each}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header
    class="flex items-center justify-between border-b border-zinc-800 px-3 py-2 {live
      ? 'cursor-grabbing'
      : 'cursor-grab'}"
    title={$_("activity.panel_move_hint")}
    onpointerdown={onHeaderPointerDown}
    ondblclick={dockLiveActivity}
  >
    <div class="flex items-center gap-2 text-xs font-medium text-zinc-200">
      <span aria-hidden="true">
        <RefreshCcw size={12} />
      </span>
      <span>{$_("activity.panel_title")}</span>
    </div>
    <button
      type="button"
      class="rounded p-1 text-zinc-400 hover:text-zinc-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
      aria-label={$_("activity.hide_panel")}
      title={$_("activity.hide_panel")}
      onclick={hidePanel}
    >
      <X size={14} />
    </button>
  </header>

  {#if $activityFeed.length === 0}
    <p class="px-3 py-6 text-center text-xs text-zinc-500">
      {$_("activity.empty")}
    </p>
  {:else}
    <ul
      data-nav-start
      class="{sized
        ? 'min-h-0 flex-1'
        : 'max-h-[min(60vh,28rem)]'} divide-y divide-zinc-800/60 overflow-y-auto"
    >
      {#each $activityFeed.slice(0, 50) as entry (entry.id)}
        {@const Icon = ICONS[entry.kind]}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
        <li
          data-nav-stop
          data-nav-inset
          data-nav-id="activity-row-{entry.id}"
          tabindex="-1"
          onkeydown={(e) => onRowKeydown(e, entry.id)}
          class="flex items-start gap-3 px-3 py-2 {ROW_ACCENT[entry.kind] ?? ''}"
        >
          <span
            class="mt-0.5 shrink-0 {TINTS[entry.kind]}"
            aria-hidden="true"
          >
            <Icon size={14} />
          </span>
          <div class="min-w-0 flex-1 text-xs leading-snug">
            <!-- `anywhere`: a path has no spaces to break at, and an unbroken
                 one would push the row wider than the card. -->
            <p
              class="text-zinc-200 [overflow-wrap:anywhere] {ACTIONABLE.has(entry.kind) ||
              expanded[entry.id]
                ? ''
                : 'line-clamp-2'}"
              use:clampWatch={entry.id}
            >
              {summary(entry)}
            </p>
            {#if expanded[entry.id] || overflowing[entry.id]}
              <!-- Up goes back to its own row; measured, it skipped to the one
                   above, since the row's box holds the button. -->
              <button
                type="button"
                data-nav-up="activity-row-{entry.id}"
                class="mt-0.5 rounded text-[11px] font-medium text-emerald-400 hover:text-emerald-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
                onclick={() => (expanded[entry.id] = !expanded[entry.id])}
              >
                {expanded[entry.id] ? $_("activity.show_less") : $_("activity.show_more")}
              </button>
            {/if}
            <!-- Storage rows carry the way out. A feed that only *reports* a
                 full account leaves the user hunting through Settings for the
                 escape hatch at the worst possible moment. -->
            {#if ACTIONABLE.has(entry.kind)}
              <button
                type="button"
                class="mt-1 rounded-md border border-red-500/50 bg-red-500/10 px-2 py-1 text-[11px] font-medium text-red-200 transition-colors hover:bg-red-500/20"
                onclick={openLiberate}
              >
                {$_("liberate.cta")}
              </button>
            {/if}
            <p class="text-[10px] text-zinc-500">
              {_tickRef >= 0 ? relativeTime(entry.at) : ""}
            </p>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</aside>
