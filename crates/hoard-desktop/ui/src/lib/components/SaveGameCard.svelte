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
    Bell,
    Check,
    CircleDot,
    Cloud,
    Clock,
    History,
    MoreHorizontal,
    PauseCircle,
    Pencil,
    PlayCircle,
    UploadCloud,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "./Button.svelte";
  import CardResizeHandle from "./CardResizeHandle.svelte";
  import Cover from "./Cover.svelte";
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
    formatDateTime,
    formatRelativeTime,
    prettifySlug,
  } from "../utils/format";
  import { tilt } from "../actions/tilt";

  let {
    save,
    now,
    versions,
    footprintBytes,
    agentRunning,
    showLabel,
    aspect,
    onRename,
    onBackup,
    onTogglePause,
    onHistory,
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
    onRename: (save: TrackedSave) => void;
    onBackup: (save: TrackedSave) => void;
    onTogglePause: (save: TrackedSave) => void;
    onHistory: (save: TrackedSave) => void;
  } = $props();

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
  $effect(() => {
    if (!menuOpen) return;
    const onDown = (e: PointerEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
    };
    window.addEventListener("pointerdown", onDown);
    return () => window.removeEventListener("pointerdown", onDown);
  });

  const menuItemClass =
    "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs text-zinc-300 transition-colors hover:bg-white/[0.06] hover:text-zinc-50";
  const iconBtnClass =
    "flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-white/[0.06] hover:text-zinc-200";
</script>

<div
  class="tilt panel group relative flex flex-col overflow-hidden transition-[background-color,border-color,box-shadow] duration-200 hover:border-[var(--edge-strong)] hover:shadow-[var(--edge-top),var(--shadow-raised)]"
  use:tilt
>
  <!-- Esquina de arrastre, la misma de la biblioteca: a lo ancho manda el
       tamaño de la tarjeta, a lo alto la forma de la carátula. Sustituye al
       botón 2:3 / cuadrada, que solo daba dos puntos de todo ese recorrido. -->
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
  <div class="relative w-full overflow-hidden" style={coverAspectStyle(aspect)}>
    <Cover
      slug={save.game_slug}
      name={displayName}
      class="h-full w-full"
      initialClass="text-4xl"
      fit="smart"
      editor="corner"
    />
    <div
      class="pointer-events-none absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-zinc-950/70 to-transparent"
      aria-hidden="true"
    ></div>

    <!-- Top-left stack: CLOUD head chip (never the local pill — see the
         invariant above) and the paused marker. -->
    <div class="absolute left-2.5 top-2.5 flex flex-col items-start gap-1.5">
      {#if save.cloud_version_num != null}
        <!-- Amber when the cloud is ahead: "there's something newer waiting",
             same semantics as the update-available badge. -->
        <span
          class="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] tabular-nums ring-1 ring-inset backdrop-blur-md {cloudAhead(
            save,
          )
            ? 'bg-amber-500/15 text-amber-300 ring-amber-500/40'
            : 'bg-zinc-950/60 text-zinc-300 ring-white/[0.12]'}"
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
          class="inline-flex items-center gap-1 rounded-md bg-amber-500/15 px-1.5 py-0.5 text-[11px] text-amber-300 ring-1 ring-inset ring-amber-500/40 backdrop-blur-md"
        >
          <PauseCircle size={11} /> {$_("dashboard.paused")}
        </span>
      {/if}
    </div>

    <!-- Overflow menu: rename / pause-resume / history. -->
    <div class="absolute right-2.5 top-2.5" bind:this={menuEl}>
      <button
        type="button"
        onclick={() => (menuOpen = !menuOpen)}
        aria-label={$_("dashboard.menu_open")}
        title={$_("dashboard.menu_open")}
        aria-expanded={menuOpen}
        class="flex h-7 w-7 items-center justify-center rounded-md border border-white/[0.10] bg-zinc-950/60 text-zinc-300 backdrop-blur-md transition-colors hover:border-white/[0.18] hover:text-zinc-50"
      >
        <MoreHorizontal size={14} />
      </button>
      {#if menuOpen}
        <div
          class="absolute right-0 top-full z-20 mt-1 w-44 overflow-hidden rounded-lg border border-white/[0.08] bg-zinc-900/95 p-1 shadow-xl backdrop-blur-xl"
        >
          <button
            type="button"
            class={menuItemClass}
            onclick={() => {
              menuOpen = false;
              onRename(save);
            }}
          >
            <Pencil size={13} class="shrink-0 text-zinc-500" />
            {$_("dashboard.rename_aria")}
          </button>
          <button
            type="button"
            class={menuItemClass}
            onclick={() => {
              menuOpen = false;
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
              menuOpen = false;
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
        class={iconBtnClass}
        onclick={() => onRename(save)}
        title={$_("dashboard.rename_aria")}
        aria-label={$_("dashboard.rename_aria")}
      >
        <Pencil size={13} />
      </button>
      <!-- TODO(1.1.0): per-game notification mute. The engine only exposes
           GLOBAL notify prefs (`notify_on_success` / `notify_on_failure`);
           there is no per-save flag to wire this bell to. Left disabled on
           purpose — see UI-1.1.0.md "Pendiente de cablear". -->
      <button
        type="button"
        class="{iconBtnClass} cursor-not-allowed opacity-40 hover:bg-transparent hover:text-zinc-500"
        disabled
        title={$_("dashboard.notify_soon")}
        aria-label={$_("dashboard.notify_soon")}
      >
        <Bell size={13} />
      </button>
      <button
        type="button"
        class={iconBtnClass}
        onclick={() => onHistory(save)}
        title={$_("dashboard.history_title")}
        aria-label={$_("dashboard.history")}
      >
        <History size={13} />
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
        class="shrink-0 !px-3 !py-1.5 !text-xs"
        onclick={() => onBackup(save)}
        disabled={!agentRunning}
        title={!agentRunning
          ? $_("dashboard.tooltip_offline")
          : save.paused
            ? $_("dashboard.tooltip_force_paused")
            : $_("dashboard.tooltip_force")}
      >
        <UploadCloud size={13} />
        {$_("dashboard.back_up")}
      </Button>
    </div>

    <!-- Per-game stats: the answers "is it working?" (last save) and "how
         much am I dedicating to this game?" (total size across versions,
         stored-version count) without diving into History. -->
    <div class="mt-4 space-y-2 border-t border-white/[0.06] pt-3 text-xs">
      <div class="flex items-baseline justify-between gap-3">
        <span class="shrink-0 text-zinc-500">{$_("dashboard.last_saved")}</span>
        {#if save.last_backup_at}
          <span class="min-w-0 text-right">
            <span class="block truncate font-medium text-zinc-200">
              {formatRelativeTime(save.last_backup_at, now)}
            </span>
            <span class="block text-[11px] tabular-nums text-zinc-500">
              {formatDateTime(save.last_backup_at)}
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
