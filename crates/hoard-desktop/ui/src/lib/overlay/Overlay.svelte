<script lang="ts">
  /**
   * The HUD over the game, Hoard's Shift+Tab.
   *
   * It is **the normal app**, not Hoard-Screen: a second Tauri window (label
   * `overlay`, created in `commands/overlay.rs`), undecorated, transparent and
   * always on top, which mounts this component instead of `App.svelte`. `main.ts`
   * does the routing by looking at the label.
   *
   * Three areas, each answering a different question:
   *
   * - **Top, status**: is this working *right now*? The service, the cloud, how
   *   many saves it watches and when the next backup is due.
   * - **Left, the log**: what has happened? The service's feed, with filters and
   *   search. By default it shows neither everything nor only the uploads: it
   *   shows what tells the story of a session (starts, uploads, restores and
   *   failures), the same as the application's dashboard.
   * - **Right, the saves**: what state is each game in? Above all, which version
   *   is **here** against the one **in the cloud**, which is the figure that
   *   decides whether you can play in peace.
   *
   * It closes with the button at the top right, or with Escape.
   *
   * **This only looks.** It does not switch the event relay on, does not start the
   * service, sends no notifications and does not touch the tray: it asks for a
   * snapshot of what the application already knows (`agent_snapshot`, three
   * in-memory mutexes) and paints it. Opening a window to check the state must not
   * change it.
   *
   * And it reads instead of listening because listening here **does not work**:
   * this window is born on the first press of the shortcut, when the service's
   * backlog has already been emitted once, `attach_agent_events` is idempotent and
   * the engine's state is only re-emitted when it changes. With the `listen()`s
   * perfectly in place, the HUD came up with an empty log, "service down" in red
   * with the service alive, zero saves watched and no backup scheduled.
   */
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Search, X } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";

  import Logo from "../components/Logo.svelte";
  import { syncPersistedLocale } from "../i18n";
  import * as api from "../api";
  import type { AgentSlotStatus, TrackedSave } from "../api";
  import { status } from "../stores/agent";
  import {
    activityFeed,
    adoptCloud,
    adoptJournal,
    cloudLoop,
    type FeedEntry,
  } from "../stores/live";
  import { feedRelativeTime, feedSummary } from "../utils/feedText";
  import { formatBytes } from "../utils/format";
  import OverlayDebug from "./OverlayDebug.svelte";
  import { OVERLAY_DEBUG_PANEL, overlayDebug } from "./debug.svelte";

  // ── Registro ──────────────────────────────────────────────────────────

  type Filter = "default" | "all" | "errors";

  /**
   * The out-of-the-box filter.
   *
   * Neither everything at once (quota, the Pro padlock, the watcher arming itself,
   * noise that does not help with a game in front of you) nor uploads only (you
   * miss exactly the failure you wanted to see). It is what tells a session's
   * story: it started, it uploaded, it restored, or something broke.
   */
  const DEFAULT_KINDS = new Set<FeedEntry["kind"]>([
    "game_started",
    "game_stopped",
    "upload_started",
    "upload_completed",
    "upload_failed",
    "auto_restored",
    "auto_restore_failed",
    "auto_restore_stuck",
    "cloud_pull",
  ]);

  /** Lo que cuenta como problema, para el filtro de errores. */
  const ERROR_KINDS = new Set<FeedEntry["kind"]>([
    "upload_failed",
    "auto_restore_failed",
    "auto_restore_stuck",
    "backup_too_large",
    "quota_reached",
    "storage_full",
    "offline",
  ]);

  let filter = $state<Filter>("default");
  let query = $state("");
  let searchOpen = $state(false);

  const FILTERS: { id: Filter; labelKey: string }[] = [
    { id: "default", labelKey: "overlay.filter_default" },
    { id: "all", labelKey: "overlay.filter_all" },
    { id: "errors", labelKey: "overlay.filter_errors" },
  ];

  /** Reloj para que los "hace 3 s" no se queden congelados. */
  let tick = $state(0);
  $effect(() => {
    const id = setInterval(() => {
      if (!document.hidden) tick = tick + 1;
    }, 1000);
    return () => clearInterval(id);
  });

  /**
   * Rows already formatted. The text is computed here and not in the markup
   * because the search has to look at **what the user reads**, not the internal
   * `kind`: searching "factorio" has to find the row that says "factorio started",
   * and that text only exists after translation.
   *
   * `tick` is a deliberate dependency: without it Svelte has no reason to repaint
   * and the relative times freeze.
   */
  const rows = $derived.by(() => {
    tick;
    const q = query.trim().toLowerCase();
    return $activityFeed
      .filter((e) =>
        filter === "all"
          ? true
          : filter === "errors"
            ? ERROR_KINDS.has(e.kind)
            : DEFAULT_KINDS.has(e.kind),
      )
      .map((e) => ({
        e,
        when: feedRelativeTime(e.at, $_),
        text: feedSummary(e, $_),
        bad: ERROR_KINDS.has(e.kind),
      }))
      .filter((r) => !q || r.text.toLowerCase().includes(q))
      .slice(0, 200);
  });

  // ── Estado ────────────────────────────────────────────────────────────

  const cloudLabel = $derived.by(() => {
    switch ($cloudLoop.status) {
      case "online":
        return $_("overlay.status_cloud_online");
      case "offline":
        return $_("overlay.status_cloud_offline");
      case "throttled":
        return $_("overlay.status_cloud_throttled");
      default:
        return $_("overlay.status_cloud_unknown");
    }
  });

  /**
   * What the service says about each watched save, indexed by `save_id`. It is the
   * source of the "playing" state and of the next backup: state the engine already
   * keeps, not something this window deduces from the log.
   */
  let slots = $state<AgentSlotStatus[]>([]);
  const slotOf = $derived.by(() => {
    const by = new Map<string, AgentSlotStatus>();
    for (const s of slots) by.set(s.save_id, s);
    return by;
  });

  /** The soonest scheduled backup across every save. */
  const nextBackup = $derived.by(() => {
    tick;
    const now = Date.now();
    const times = slots
      .map((s) =>
        s.next_scheduled_backup_at
          ? new Date(s.next_scheduled_backup_at).getTime()
          : NaN,
      )
      .filter((t) => Number.isFinite(t) && t > now);
    if (times.length === 0) return null;
    const secs = Math.round((Math.min(...times) - now) / 1000);
    return secs < 60
      ? $_("activity.time_seconds_ago", { values: { count: secs } })
      : $_("history.relative_minutes", {
          values: { count: Math.round(secs / 60) },
        });
  });

  // ── Partidas ──────────────────────────────────────────────────────────

  let saves = $state<TrackedSave[]>([]);

  function isBehind(s: TrackedSave): boolean {
    return (
      s.cloud_version_num != null &&
      (s.local_version_num == null || s.cloud_version_num > s.local_version_num)
    );
  }

  /**
   * The saves, with the one being played first: with a game in front of you, that
   * game is what matters. Then the ones behind the cloud (the case that can cost
   * you a save), and then by date.
   */
  const savesSorted = $derived.by(() => {
    const rank = (s: TrackedSave) =>
      slotOf.get(s.save_id)?.process_running ? 0 : isBehind(s) ? 1 : 2;
    return [...saves].sort((a, b) => {
      const d = rank(a) - rank(b);
      if (d !== 0) return d;
      return (
        new Date(b.last_backup_at ?? 0).getTime() -
        new Date(a.last_backup_at ?? 0).getTime()
      );
    });
  });

  async function loadSaves() {
    try {
      saves = await api.listTrackedSaves();
    } catch (e) {
      console.warn("no se pudieron listar las partidas en el HUD:", e);
    }
  }

  /**
   * Copies the application's state into **this** window's stores.
   *
   * Every window has its own JS context and therefore its own stores; the ones here
   * start at their default values (service stopped, empty log, cloud unknown). This
   * brings them up to date with what the application already knows, and the log
   * goes through the same event mapping the app's dashboard uses (`adoptJournal`),
   * so the two surfaces cannot tell different stories about the same event.
   */
  async function refresh() {
    try {
      const snap = await api.agentSnapshot();
      status.set(snap.status);
      slots = snap.slots;
      adoptJournal(snap.rows);
      adoptCloud(snap.cloud, snap.cloud_retry_in);
    } catch (e) {
      console.warn("el HUD no pudo leer el estado:", e);
    }
  }

  function close() {
    void invoke("overlay_set_visible", { visible: false });
  }

  /**
   * How often it re-reads while the HUD is on screen.
   *
   * It reads the process's own memory, so the cost is the webview bridge's and
   * nothing else; with the HUD hidden it does not read at all. Two seconds is what
   * it takes for a backup starting with the panel open to be visible without
   * looking like a nervous counter.
   */
  const REFRESH_MS = 2000;

  /** The saves do come from the network, so they go at their own pace, not the loop's. */
  const SAVES_MIN_GAP_MS = 15000;
  let lastSavesAt = 0;

  async function loadSavesThrottled() {
    if (Date.now() - lastSavesAt < SAVES_MIN_GAP_MS) return;
    lastSavesAt = Date.now();
    await loadSaves();
  }

  onMount(() => {
    void refresh();
    void loadSavesThrottled();

    const timer = setInterval(() => {
      if (document.hidden) return;
      void refresh();
    }, REFRESH_MS);

    // On being shown again it is worth not waiting for the next tick, and worth
    // re-reading the saves: between one opening and the next, versions may have
    // changed or a new save may have been added. `focus` goes alongside
    // `visibilitychange` because showing the window asks for focus, and not every
    // platform marks the document hidden when it is hidden.
    const onShow = () => {
      if (document.hidden) return;
      void refresh();
      void loadSavesThrottled();
      // The language too: this window is not destroyed when it closes, so without
      // this it would keep whatever it had when it was created even if the user
      // changes it in Settings. svelte-i18n is reactive, so the text changes on its
      // own with nothing remounted.
      void syncPersistedLocale();
    };
    document.addEventListener("visibilitychange", onShow);
    window.addEventListener("focus", onShow);

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        // Escape leaves the search box before it leaves the HUD: if you are
        // filtering, you expect to leave the filter, not the window.
        if (query) {
          query = "";
          return;
        }
        if (searchOpen) {
          searchOpen = false;
          return;
        }
        close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onShow);
      window.removeEventListener("focus", onShow);
      window.removeEventListener("keydown", onKey);
    };
  });

  const bg = $derived(overlayDebug.bgOpacity);
  const fg = $derived(overlayDebug.textOpacity);
  const fs = $derived(overlayDebug.fontSize);
</script>

<!-- `fixed inset-0` en vez de `h-full`: encadenar alturas al 100 % deja
     asomar una franja de uno o dos píxeles en cuanto hay redondeo de
     subpíxel, y en una ventana transparente eso se ve como una línea. -->
<div
  class="fixed inset-0 flex flex-col text-zinc-100"
  style="background: rgba(9, 9, 11, {bg}); opacity: {fg}; font-size: {fs}px;"
>
  <!-- ── Zona 1 · Estado ──────────────────────────────────────────────── -->
  <header
    class="flex shrink-0 items-start justify-between gap-6 border-b border-white/[0.08] px-6 py-4"
  >
    <div class="flex min-w-0 items-center gap-3">
      <Logo size={32} class="shrink-0 rounded-lg" />
      <span class="font-display text-lg font-semibold tracking-[-0.014em]">Hoard</span>

      <div class="ml-4 flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1">
        <span class="flex items-center gap-1.5" style="font-size: 0.85em;">
          <span
            class="h-2 w-2 shrink-0 rounded-full {$status.running
              ? 'bg-emerald-500'
              : 'bg-rose-500'}"
          ></span>
          <span class={$status.running ? "text-zinc-300" : "text-rose-300"}>
            {$status.running
              ? $_("overlay.status_service_on")
              : $_("overlay.status_service_off")}
          </span>
        </span>

        <span class="text-zinc-400" style="font-size: 0.85em;">{cloudLabel}</span>

        <!-- El número lo dice el servicio (`watched_count`), no lo cuenta esta
             ventana sumando `watcher-armed`: esos avisos se emiten una vez por
             slot al armarse, mucho antes de que el HUD exista. -->
        <span class="text-zinc-400" style="font-size: 0.85em;">
          {$_("overlay.status_watched", {
            values: { count: $status.watched_count },
          })}
        </span>

        <span class="text-zinc-400" style="font-size: 0.85em;">
          {nextBackup
            ? $_("overlay.status_next", { values: { when: nextBackup } })
            : $_("overlay.status_next_none")}
        </span>
      </div>
    </div>

    <div class="flex shrink-0 flex-col items-end gap-1">
      <button
        type="button"
        onclick={close}
        aria-label={$_("overlay.close")}
        class="flex h-8 w-8 items-center justify-center rounded-lg border border-white/[0.14] bg-white/[0.06] text-zinc-200 transition-colors hover:bg-white/[0.14] hover:text-white"
      >
        <X size={16} />
      </button>
      <span class="text-zinc-400" style="font-size: 0.8em;"
        >{$_("overlay.escape_hint")}</span
      >
    </div>
  </header>

  <div class="flex min-h-0 flex-1">
    <!-- ── Zona 2 · Registro ──────────────────────────────────────────── -->
    <section class="flex min-h-0 flex-[3] flex-col border-r border-white/[0.08]">
      <div class="flex shrink-0 flex-wrap items-center gap-2 px-6 py-3">
        <span class="mr-1 font-medium text-zinc-300" style="font-size: 0.9em;"
          >{$_("overlay.log_title")}</span
        >
        <div class="inline-flex rounded-md border border-white/[0.10] p-0.5">
          {#each FILTERS as f (f.id)}
            <button
              type="button"
              onclick={() => (filter = f.id)}
              class="rounded px-2.5 py-1 transition-colors {filter === f.id
                ? 'bg-emerald-600 text-white'
                : 'text-zinc-300 hover:bg-white/[0.08]'}"
              style="font-size: 0.8em;">{$_(f.labelKey)}</button
            >
          {/each}
        </div>

        <button
          type="button"
          onclick={() => (searchOpen = !searchOpen)}
          aria-label={$_("overlay.search")}
          aria-expanded={searchOpen}
          class="flex h-7 w-7 items-center justify-center rounded-md border border-white/[0.10] text-zinc-300 transition-colors hover:bg-white/[0.08] {searchOpen
            ? 'bg-white/[0.08]'
            : ''}"
        >
          <Search size={13} />
        </button>

        {#if searchOpen}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            autofocus
            bind:value={query}
            placeholder={$_("overlay.search_placeholder")}
            aria-label={$_("overlay.search")}
            class="min-w-0 flex-1 rounded-md border border-white/[0.10] bg-black/30 px-2 py-1 text-zinc-100 placeholder:text-zinc-500 focus:border-emerald-500/50"
            style="font-size: 0.85em;"
          />
        {/if}
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto px-6 pb-5">
        {#if rows.length === 0}
          <p class="pt-6 text-zinc-400" style="font-size: 0.9em;">
            {query ? $_("overlay.no_matches") : $_("overlay.empty")}
          </p>
        {:else}
          <ul class="space-y-1.5">
            {#each rows as r (r.e.id)}
              <li class="flex items-baseline gap-3">
                <span
                  class="w-20 shrink-0 tabular-nums text-zinc-500"
                  style="font-size: 0.8em;">{r.when}</span
                >
                <span
                  class="min-w-0 flex-1 leading-snug {r.bad ? 'text-rose-300' : ''}"
                  >{r.text}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </section>

    <!-- ── Zona 3 · Partidas ──────────────────────────────────────────── -->
    <section class="flex min-h-0 flex-[2] flex-col">
      <div class="shrink-0 px-6 py-3">
        <span class="font-medium text-zinc-300" style="font-size: 0.9em;"
          >{$_("overlay.saves_title")}</span
        >
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto px-6 pb-5">
        {#if savesSorted.length === 0}
          <p class="text-zinc-400" style="font-size: 0.9em;">
            {$_("overlay.saves_empty")}
          </p>
        {:else}
          <ul class="space-y-2">
            {#each savesSorted as s (s.save_id)}
              {@const playing = slotOf.get(s.save_id)?.process_running ?? false}
              {@const behind = isBehind(s)}
              <li class="rounded-lg border border-white/[0.08] bg-white/[0.03] px-3 py-2">
                <div class="flex items-baseline justify-between gap-2">
                  <span class="min-w-0 truncate font-medium">{s.game_slug}</span>
                  {#if playing}
                    <span class="shrink-0 text-emerald-300" style="font-size: 0.75em;"
                      >{$_("overlay.save_playing")}</span
                    >
                  {:else if s.paused}
                    <span class="shrink-0 text-amber-300" style="font-size: 0.75em;"
                      >{$_("overlay.save_paused")}</span
                    >
                  {:else if behind}
                    <span class="shrink-0 text-amber-300" style="font-size: 0.75em;"
                      >{$_("overlay.save_behind")}</span
                    >
                  {/if}
                </div>
                <!-- Local y nube SIEMPRE por separado y etiquetados. Enseñar un
                     solo número invita a jugar sobre una partida vieja y
                     empujarla como si fuera la buena (ADR 0021 D.10). -->
                <div
                  class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-zinc-400"
                  style="font-size: 0.78em;"
                >
                  <span class={behind ? "text-amber-300" : ""}>
                    {s.local_version_num != null
                      ? $_("overlay.save_local", {
                          values: { version: s.local_version_num },
                        })
                      : $_("overlay.save_local_none")}
                  </span>
                  <span>
                    {s.cloud_version_num != null
                      ? $_("overlay.save_cloud", {
                          values: { version: s.cloud_version_num },
                        })
                      : $_("overlay.save_cloud_none")}
                  </span>
                  <span class="tabular-nums">{formatBytes(s.total_size_bytes)}</span>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </section>
  </div>

  <!-- Los mandos de depuración, apagados por el interruptor de
       `debug.svelte.ts`. Los valores en vivo salen igualmente de ahí
       (0.72 / 1 / 14px), así que afinar a mano es cambiar esos tres números;
       sacar los deslizadores es poner `OVERLAY_DEBUG_PANEL` en `true`. -->
  {#if OVERLAY_DEBUG_PANEL}
    <OverlayDebug />
  {/if}
</div>
