<script lang="ts">
  /**
   * SaveGameCard, the Dashboard's per-game card (1.1.0 redesign).
   *
   * Presentational component: every mutating action is delegated to the
   * parent through callbacks; live status is read from the agent activity
   * store (read-only here). Layout: big portrait cover on top (the Steam
   * capsule or the user's custom cover), name + quick actions, status pill,
   * and the per-game stats the old list row never had room for (last save
   * date, total cloud size across versions, stored-version count).
   *
   * INVARIANT (ADR 0021 D.10): the status pill in the body reflects the
   * version **this device** holds; the chip overlaid on the cover is the
   * **cloud** head. They are rendered by separate elements with separate
   * labels, never merge them.
   */
  import {
    AlertTriangle,
    Check,
    CircleDot,
    Cloud,
    Clock,
    History,
    Loader2,
    MoreHorizontal,
    PauseCircle,
    Pencil,
    PlayCircle,
    Trash2,
    UploadCloud,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import AnimIcon from "./AnimIcon.svelte";
  import Button from "./Button.svelte";
  import CardResizeHandle from "./CardResizeHandle.svelte";
  import Cover from "./Cover.svelte";
  import { byKeys, navFocus } from "../actions/spatialNav";
  import type { TrackedSave } from "../api";
  import {
    coverAspectStyle,
    setCoverAspect,
    POSTER,
    SQUARE,
  } from "../stores/coverAspect.svelte";
  import { activity } from "../stores/agent";
  import { customNames } from "../stores/gameNames";
  import {
    formatBytes,
    fmtDateTime,
    fmtRelativeTime,
    prettifySlug,
  } from "../utils/format";

  let {
    save,
    now,
    versions,
    footprintBytes,
    agentRunning,
    showLabel,
    aspect,
    removing,
    onRename,
    onBackup,
    onTogglePause,
    onHistory,
    onDelete,
  }: {
    save: TrackedSave;
    /** Ticking clock (epoch ms) from the parent, drives the scheduled
     *  countdown and the "last saved" relative time. */
    now: number;
    /** Stored-version count for this save. `undefined` = still loading,
     *  `null` = couldn't be fetched (rendered as "—"). */
    versions: number | null | undefined;
    /** What this game actually occupies in the cloud: every stored version,
     *  deduplicated. `undefined` = still loading, `null` = unavailable
     *  (self-hosted, offline), and then the card falls back to the head
     *  version's size, which is all the manifest carries. The two differ by
     *  a lot on a game whose folder changes wholesale every save, and the
     *  head-only number is the one that made a user ask why 35 MB of saves
     *  reported 79 MB of quota. */
    footprintBytes: number | null | undefined;
    /** Whether the sync service is up, gates the manual backup button. */
    agentRunning: boolean;
    /** True when another tracked save shares this slug: the label chip is
     *  what tells the two folders apart. */
    showLabel: boolean;
    /** Cover height ÷ width (1.5 = the 2:3 poster, 1 = square). The user drags
     *  it from the corner handle; the parent holds it so the grid agrees. */
    aspect: number;
    /** A delete or untrack for this row is waiting on its answer. */
    removing: boolean;
    onRename: (save: TrackedSave) => void;
    onBackup: (save: TrackedSave) => void;
    onTogglePause: (save: TrackedSave) => void;
    onHistory: (save: TrackedSave) => void;
    onDelete: (save: TrackedSave) => void;
  } = $props();

  /** Flipped on every press so the upload icon has something to react to. */
  let backupPressed = $state(false);

  // The backup button is just the cloud until the pointer is on it; then its
  // label types itself in, and erases itself on the way out. Counted in code
  // points, not UTF-16 units, so Japanese and Chinese don't split a character.
  let backupHover = $state(false);
  let typed = $state(0);
  const backupLabel = $derived([...$_("dashboard.back_up")]);
  $effect(() => {
    const target = backupHover ? backupLabel.length : 0;
    if (typed === target) return;
    const id = setTimeout(
      () => (typed += typed < target ? 1 : -1),
      backupHover ? 35 : 15,
    );
    return () => clearTimeout(id);
  });

  /** The visible name: the user's per-device override when set, otherwise a
   *  prettified slug. The slug itself never changes, it's the sync key. */
  const displayName = $derived(
    $customNames[save.game_slug] ?? prettifySlug(save.game_slug),
  );

  /**
   * The version **this device** holds: the backend's local cursor, moved
   * forward by anything the live session has since confirmed (an upload that
   * committed, or an auto-restore that landed). Never the cloud head.
   */
  function localVersion(save: TrackedSave): number | null {
    const live = $activity[save.save_id]?.last_version;
    const stored = save.local_version_num;
    if (live == null) return stored;
    if (stored == null) return live;
    return Math.max(live, stored);
  }

  /** `true` when the cloud holds a version this device doesn't have yet. */
  function cloudAhead(save: TrackedSave): boolean {
    if (save.cloud_version_num == null) return false;
    const local = localVersion(save);
    return local == null || save.cloud_version_num > local;
  }

  /** Tooltip for the cloud-version chip: always spells out both numbers, so
   *  "the cloud has vN" can never be read as "this device has vN". */
  function cloudTitle(save: TrackedSave): string {
    const cloud = save.cloud_version_num;
    const local = localVersion(save);
    if (!cloudAhead(save)) return $_("dashboard.cloud_version_title");
    if (local == null) {
      return $_("dashboard.cloud_ahead_no_local_title", { values: { cloud } });
    }
    return $_("dashboard.cloud_ahead_title", { values: { cloud, local } });
  }

  /** The status pill. Reflects **local** state only, what this machine has
   *  on disk and what the agent is doing about it. The cloud head gets its
   *  own chip on the cover so the two are never conflated. */
  function pillFor(save: TrackedSave) {
    const a = $activity[save.save_id];
    const local = localVersion(save);
    // Live activity always wins, if the agent reports anything, the pill
    // reflects *that* (it's the freshest signal on screen). Falls through
    // to the server-side history check only when we have no in-memory state
    // for this save, which is the case on a cold app launch.
    if (!a) {
      return idlePill(save, local);
    }
    switch (a.state) {
      case "running":
        return {
          label: $_("dashboard.pill_running"),
          icon: PlayCircle,
          dot: "bg-sky-500",
          chip: "bg-sky-500/10 text-sky-400 ring-sky-500/30",
        };
      case "scheduled": {
        const secs = Math.max(
          0,
          Math.round(((a.next_backup_at ?? now) - now) / 1000),
        );
        return {
          label: $_("dashboard.pill_scheduled", { values: { seconds: secs } }),
          icon: Clock,
          dot: "bg-amber-500",
          chip: "bg-amber-500/10 text-amber-400 ring-amber-500/30",
        };
      }
      case "uploading":
        return {
          label: $_("dashboard.pill_uploading"),
          icon: UploadCloud,
          dot: "bg-amber-500",
          chip: "bg-amber-500/10 text-amber-400 ring-amber-500/30",
        };
      case "ok":
        return {
          label: local
            ? $_("dashboard.pill_saved_local_v", { values: { version: local } })
            : $_("dashboard.pill_saved"),
          icon: Check,
          dot: null,
          chip: "bg-emerald-500/10 text-emerald-400 ring-emerald-500/30",
        };
      case "partial":
        return {
          label: $_("dashboard.pill_partial"),
          icon: AlertTriangle,
          dot: null,
          chip: "bg-amber-500/10 text-amber-400 ring-amber-500/30",
        };
      case "failed":
        return {
          label: a.will_retry
            ? $_("dashboard.pill_failed_retry")
            : $_("dashboard.pill_failed"),
          icon: AlertTriangle,
          dot: null,
          chip: "bg-red-500/10 text-red-400 ring-red-500/30",
        };
      default:
        return idlePill(save, local);
    }
  }

  /** Resting pill (no live activity): what this device holds, or, when it
   *  holds nothing, an explicit "only in the cloud" instead of a version
   *  number the user would read as their own. */
  function idlePill(save: TrackedSave, local: number | null) {
    if (local != null) {
      return {
        label: $_("dashboard.pill_saved_local_v", {
          values: { version: local },
        }),
        icon: Check,
        dot: null,
        chip: "bg-emerald-500/10 text-emerald-400 ring-emerald-500/30",
      };
    }
    if (save.cloud_version_num != null) {
      return {
        label: $_("dashboard.pill_cloud_only_v", {
          values: { version: save.cloud_version_num },
        }),
        icon: Cloud,
        dot: null,
        chip: "bg-white/[0.04] text-zinc-400 ring-white/[0.08]",
      };
    }
    return {
      label: $_("dashboard.pill_no_backup"),
      icon: CircleDot,
      dot: null,
      chip: "bg-white/[0.04] text-zinc-400 ring-white/[0.08]",
    };
  }

  /** Live status pill for this save (LOCAL state, see the invariant). */
  const pill = $derived(pillFor(save));

  /**
   * Vertical half of the corner drag: how tall the cover is relative to the
   * card. The handle opens every gesture with `dy = 0`, which is where the
   * anchor is taken; from there the ratio only ever moves *relative* to it, so
   * a mis-measured width changes how fast the cover reshapes and nothing
   * else, it can't make it jump as the drag begins.
   */
  let aspectAtDragStart: number | null = null;

  function reshapeCover(dy: number, cardW: number) {
    if (dy === 0 || aspectAtDragStart === null) aspectAtDragStart = aspect;
    setCoverAspect(snapCanonical(aspectAtDragStart + dy / cardW));
  }

  /** 2:3 and 1:1 are the two ratios art is actually authored in, worth
   *  landing on exactly, not near. Everything between them is free. */
  function snapCanonical(a: number): number {
    for (const target of [POSTER, SQUARE]) {
      if (Math.abs(a - target) < 0.04) return target;
    }
    return a;
  }

  // Overflow ("…") menu on the cover. Closes on any pointerdown outside the
  // wrapper (which contains both the trigger and the dropdown).
  let menuOpen = $state(false);
  let menuEl = $state<HTMLDivElement | null>(null);
  let menuButton = $state<HTMLButtonElement | null>(null);
  $effect(() => {
    if (!menuOpen) return;
    const onDown = (e: PointerEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) setMenu(false);
    };
    // Escape closes the menu before the arrows take it as leaving the card.
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      const within = !!menuEl?.contains(document.activeElement);
      setMenu(false);
      if (within && menuButton) navFocus(menuButton);
    };
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey, true);
    };
  });

  // Opening or closing the menu while its trigger drew a focus ring made
  // WebKitGTK scroll the page to near its top, on any card below the first row
  // (Oct 2026). Whatever the engine scrolls in the moment after is put back,
  // unless the card is gone (an item that leaves for History).
  function setMenu(open: boolean) {
    if (open === menuOpen) return;
    const scroller = menuEl?.closest("main");
    const top = scroller?.scrollTop ?? 0;
    menuOpen = open;
    if (!scroller) return;
    const hold = () => {
      if (menuEl?.isConnected) scroller.scrollTop = top;
    };
    scroller.addEventListener("scroll", hold);
    setTimeout(() => scroller.removeEventListener("scroll", hold), 250);
  }

  // Arrowed out of the open menu: it closes behind the focus. A click that
  // leaves the focus on nothing is the pointerdown's to handle.
  function onMenuFocusOut(e: FocusEvent) {
    if (e.relatedTarget instanceof Node && !menuEl?.contains(e.relatedTarget)) {
      setMenu(false);
    }
  }

  // The cover's pencil otherwise needs the pointer on the cover. With the
  // arrows inside this card it comes up so they can reach it; a card that is
  // only selected, like every other one, keeps it hidden.
  let cardEl = $state<HTMLDivElement | null>(null);
  let coverHovered = $state(false);
  let keysInside = $state(false);
  function onCardFocusIn(e: FocusEvent) {
    keysInside = e.target !== cardEl && byKeys();
  }
  function onCardFocusOut(e: FocusEvent) {
    const next = e.relatedTarget;
    if (!(next instanceof Node) || next === cardEl || !cardEl?.contains(next)) {
      keysInside = false;
    }
  }

  /** A row that exists only in the cloud (uploaded from another machine, or
   *  not yet linked here) has no entry in `state.json`, so anything that
   *  mutates it by `save_id` fails with the backend's raw "that save isn't
   *  tracked on this machine". Gate those controls instead of offering them. */
  const cloudOnly = $derived(save.orphan || !save.local_path);

  const menuItemClass =
    "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs text-zinc-300 transition-colors hover:bg-white/[0.06] hover:text-zinc-50";
  const menuItemDisabledClass =
    "flex w-full cursor-not-allowed items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs text-zinc-500 opacity-50";
  const iconBtnClass =
    "flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-white/[0.06] hover:text-zinc-200";
</script>

<!-- One layer for the arrows (`spatialNav.ts`): from the grid it is a single
     stop, and Enter goes in among its buttons. -->
<div
  bind:this={cardEl}
  data-card
  data-nav-frame
  data-nav-layer
  tabindex="-1"
  onfocusin={onCardFocusIn}
  onfocusout={onCardFocusOut}
  class="panel group relative flex flex-col overflow-hidden transition-[background-color,border-color,box-shadow] duration-200 hover:border-[var(--edge-strong)] hover:shadow-[var(--edge-top),var(--shadow-raised)]"
>
  <!-- Esquina de arrastre, la misma de la biblioteca: a lo ancho manda el
       tamaño de la tarjeta, a lo alto la forma de la carátula. Sustituye al
       botón 2:3 / cuadrada, que solo daba dos puntos de todo ese recorrido. -->
  {#if removing}
    <div
      class="absolute inset-0 z-30 flex items-center justify-center gap-2 bg-black/60 text-sm text-zinc-200"
      aria-live="polite"
    >
      <Loader2 size={15} class="animate-spin" />
      {$_("dashboard.removing")}
    </div>
  {/if}

  <CardResizeHandle
    section="dashboard"
    onVerticalDrag={reshapeCover}
    title={$_("dashboard.cover_resize_hint")}
  />

  <!-- Big cover, framed however the user dragged the corner handle: 2:3 (the
       store-standard ratio) by default, square, or the landscape capsule.
       Rust prefers Steam's vertical `library_600x900`, so most games fill the
       poster exactly; the ones that only ship the landscape header get
       letterboxed by `fit="smart"` rather than center-cropped to a third of
       themselves. Own art beats both: hover the cover → pencil in the corner. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-cover
    class="relative w-full overflow-hidden"
    style={coverAspectStyle(aspect)}
    onmouseenter={() => (coverHovered = true)}
    onmouseleave={() => (coverHovered = false)}
  >
    <Cover
      slug={save.game_slug}
      name={displayName}
      class="h-full w-full"
      initialClass="text-4xl"
      fit="smart"
      editor="corner"
      fallback="standby"
      showEditor={coverHovered || keysInside}
      editorNav={{
        "data-nav-id": "cover-edit",
        "data-nav-up": "cover",
        "data-nav-down": "rename",
      }}
    />
    <!-- The whole cover opens the history. It lies under the pencil, the chips
         and the overflow menu, which sit on top of it and keep their clicks. -->
    <button
      type="button"
      data-nav-id="cover"
      data-nav-down="cover-edit"
      data-nav-right="menu"
      data-nav-inset
      class="cover-history absolute inset-0 cursor-pointer"
      aria-label={$_("dashboard.history_title")}
      onclick={() => onHistory(save)}
    >
      <span class="cover-history-veil" aria-hidden="true">
        <span class="cover-history-badge"><History size={26} strokeWidth={1.75} /></span>
        <span class="cover-history-label">{$_("dashboard.history")}</span>
      </span>
    </button>
    <div
      class="pointer-events-none absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-zinc-950/70 to-transparent"
      aria-hidden="true"
    ></div>

    <!-- Top-left stack: CLOUD head chip (never the local pill — see the
         invariant above) and the paused marker. -->
    <div class="absolute left-2.5 top-2.5 flex flex-col items-start gap-1.5">
      {#if save.cloud_version_num != null}
        <!-- Amber when the cloud is ahead: "there's something newer waiting",
             same semantics as the update-available badge. The amber chips sit
             on solid black: a tinted fill lets the cover through, and on Linux,
             where the blur is off, that read as a smudge.
             A real border, not a `ring`: the ring is a 1 px inset shadow, which
             the engine does not snap to whole pixels the way it snaps borders,
             so at the fractional offsets a resized card lands on it showed only
             its top edge, or only the bottom, or nothing. -->
        <span
          class="inline-flex items-center gap-1 rounded-md border px-[5px] py-px text-[11px] tabular-nums {cloudAhead(
            save,
          )
            ? 'border-amber-500/80 bg-black text-amber-300'
            : 'border-white/[0.12] bg-layer-2 text-zinc-300'}"
          title={cloudTitle(save)}
        >
          <Cloud size={11} class={cloudAhead(save) ? "" : "text-zinc-500"} />
          {$_("dashboard.cloud_version_badge", {
            values: { version: save.cloud_version_num },
          })}
        </span>
      {/if}
      {#if save.paused}
        <span
          class="inline-flex items-center gap-1 rounded-md border border-amber-500/80 bg-black px-[5px] py-px text-[11px] text-amber-300"
        >
          <PauseCircle size={11} /> {$_("dashboard.paused")}
        </span>
      {/if}
    </div>

    <!-- Overflow menu: rename / pause-resume / history. -->
    <div class="absolute right-2.5 top-2.5" bind:this={menuEl} onfocusout={onMenuFocusOut}>
      <button
        bind:this={menuButton}
        type="button"
        data-nav-id="menu"
        data-nav-left="cover"
        onclick={() => setMenu(!menuOpen)}
        aria-label={$_("dashboard.menu_open")}
        title={$_("dashboard.menu_open")}
        aria-expanded={menuOpen}
        class="flex h-7 w-7 items-center justify-center rounded-md border border-white/[0.08] bg-layer-2 text-zinc-300 transition-colors hover:border-white/[0.18] hover:text-zinc-50"
      >
        <MoreHorizontal size={14} />
      </button>
      {#if menuOpen}
        <div
          class="absolute right-0 top-full z-20 mt-1 w-44 overflow-hidden rounded-lg border border-white/[0.08] bg-layer-3 p-1 shadow-xl"
        >
          <button
            type="button"
            class={menuItemClass}
            onclick={() => {
              setMenu(false);
              onRename(save);
            }}
          >
            <Pencil size={13} class="shrink-0 text-zinc-500" />
            {$_("dashboard.rename_aria")}
          </button>
          <button
            type="button"
            class={cloudOnly ? menuItemDisabledClass : menuItemClass}
            disabled={cloudOnly}
            title={cloudOnly ? $_("common.cloud_only_no_local") : undefined}
            onclick={() => {
              setMenu(false);
              onTogglePause(save);
            }}
          >
            {#if save.paused}
              <PlayCircle size={13} class="shrink-0 text-zinc-500" />
              {$_("dashboard.resume")}
            {:else}
              <PauseCircle size={13} class="shrink-0 text-zinc-500" />
              {$_("dashboard.pause")}
            {/if}
          </button>
          <button
            type="button"
            class={menuItemClass}
            onclick={() => {
              setMenu(false);
              onHistory(save);
            }}
          >
            <History size={13} class="shrink-0 text-zinc-500" />
            {$_("dashboard.history")}
          </button>
        </div>
      {/if}
    </div>
  </div>

  <div class="flex flex-1 flex-col p-4">
    <!-- Name row: visible name (custom override > prettified slug), with the
         label chip only when two saves share a slug and need telling apart. -->
    <div class="flex items-center gap-1">
      <h3
        class="min-w-0 flex-1 truncate text-[15px] font-medium text-zinc-50"
        title={save.game_slug}
      >
        {displayName}
      </h3>
      {#if showLabel}
        <span
          class="shrink-0 rounded-md bg-white/[0.05] px-1.5 py-0.5 text-[10px] text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
        >
          {save.label}
        </span>
      {/if}
      <button
        type="button"
        data-nav-id="rename"
        data-nav-up="cover-edit cover"
        class={iconBtnClass}
        onclick={() => onRename(save)}
        title={$_("dashboard.rename_aria")}
        aria-label={$_("dashboard.rename_aria")}
      >
        <Pencil size={13} />
      </button>
      <button
        type="button"
        data-nav-up="cover-edit cover"
        class={iconBtnClass}
        onclick={() => onHistory(save)}
        title={$_("dashboard.history_title")}
        aria-label={$_("dashboard.history")}
      >
        <History size={13} />
      </button>
      <button
        type="button"
        data-nav-up="cover-edit cover"
        class="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-red-500/10 hover:text-red-400"
        onclick={() => onDelete(save)}
        title={$_("dashboard.delete_title")}
        aria-label={$_("dashboard.delete_title")}
      >
        <Trash2 size={13} />
      </button>
    </div>

    <!-- Status pill (LOCAL) + manual backup. -->
    <div class="mt-3 flex items-center justify-between gap-2">
      <span
        class="inline-flex min-w-0 items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium ring-1 ring-inset {pill.chip}"
      >
        {#if pill.dot}
          <span class="relative flex h-1.5 w-1.5 shrink-0">
            <span
              class="absolute inline-flex h-full w-full animate-ping rounded-full {pill.dot} opacity-60"
            ></span>
            <span
              class="relative inline-flex h-1.5 w-1.5 rounded-full {pill.dot}"
            ></span>
          </span>
        {:else}
          <pill.icon size={12} class="shrink-0" />
        {/if}
        <span class="truncate">{pill.label}</span>
      </span>
      <Button
        variant="secondary"
        size="md"
        class="shrink-0 !px-2 !py-1.5 !text-xs"
        aria-label={$_("dashboard.back_up")}
        onmouseenter={() => (backupHover = true)}
        onmouseleave={() => (backupHover = false)}
        onfocus={() => (backupHover = true)}
        onblur={() => (backupHover = false)}
        onclick={() => {
          // A fire-and-forget button has no state to animate from, so it flips a
          // bit of its own: with the animation playing both ways, every press
          // moves the cloud.
          backupPressed = !backupPressed;
          onBackup(save);
        }}
        disabled={!agentRunning || cloudOnly}
        title={cloudOnly
          ? $_("common.cloud_only_no_local")
          : !agentRunning
            ? $_("dashboard.tooltip_offline")
            : save.paused
              ? $_("dashboard.tooltip_force_paused")
              : $_("dashboard.tooltip_force")}
      >
        <AnimIcon icon={UploadCloud} on={backupPressed} kind="pop" size={15} />
        {#if typed > 0}
          <span class="whitespace-nowrap" aria-hidden="true">
            {backupLabel.slice(0, typed).join("")}
          </span>
        {/if}
      </Button>
    </div>

    <!-- Per-game stats: the answers "is it working?" (last save) and "how
         much am I dedicating to this game?" (total size across versions,
         stored-version count) without diving into History. -->
    <div class="mt-4 space-y-2 border-t border-white/[0.08] pt-3 text-xs">
      <div class="flex items-baseline justify-between gap-3">
        <span class="shrink-0 text-zinc-500">{$_("dashboard.last_saved")}</span>
        {#if save.last_backup_at}
          <span class="min-w-0 text-right">
            <span class="block truncate font-medium text-zinc-200">
              {$fmtRelativeTime(save.last_backup_at, now)}
            </span>
            <span class="block text-[11px] tabular-nums text-zinc-500">
              {$fmtDateTime(save.last_backup_at)}
            </span>
          </span>
        {:else}
          <span class="text-zinc-500">{$_("dashboard.never_saved")}</span>
        {/if}
      </div>
      <div class="flex items-baseline justify-between gap-3">
        <span class="text-zinc-500">{$_("dashboard.cloud_footprint")}</span>
        <span
          class="tabular-nums text-zinc-200"
          title={footprintBytes != null
            ? $_("dashboard.cloud_footprint_title")
            : $_("dashboard.cloud_size_title")}
        >
          {formatBytes(footprintBytes ?? save.total_size_bytes)}
        </span>
      </div>
      <div class="flex items-baseline justify-between gap-3">
        <span class="text-zinc-500">{$_("dashboard.total_versions")}</span>
        <span class="tabular-nums text-zinc-200">
          {versions === undefined ? "…" : versions === null ? "—" : versions}
        </span>
      </div>
    </div>
  </div>
</div>
