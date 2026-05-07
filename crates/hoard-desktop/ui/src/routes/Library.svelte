<script lang="ts">
  /**
   * Library page — auto-detection results + tracked saves.
   *
   * On mount we hydrate from the in-memory detection cache (if a previous
   * scan happened this session) and also fetch the user's tracked saves so
   * the "tracked" badge can render immediately.
   *
   * "Scan" kicks off `scan_library`; while it runs we listen for
   * `library://scan-progress` events to drive the progress bar.
   */
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    Search as SearchIcon,
    RefreshCw,
    Plus,
    Check,
    Filter,
    HardDrive,
    Gamepad2,
  } from "lucide-svelte";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import type {
    Confidence,
    DetectedGame,
    DetectionReport,
    DetectionSource,
    ScanProgress,
    TrackedSave,
  } from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  let report = $state<DetectionReport | null>(null);
  let tracked = $state<TrackedSave[]>([]);
  let scanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let search = $state("");
  let confidenceFilter = $state<"all" | Confidence>("all");
  let sourceFilter = $state<"all" | DetectionSource>("all");
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    // Wire the progress event before we trigger anything that emits.
    unlisten = await listen<ScanProgress>(
      "library://scan-progress",
      (event) => {
        progress = event.payload;
      },
    );
    try {
      const [cached, t] = await Promise.all([
        api.cachedDetection(),
        api.listTrackedSaves(),
      ]);
      report = cached;
      tracked = t;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function runScan() {
    scanning = true;
    progress = { done: 0, total: 0 };
    try {
      report = await api.scanLibrary();
      toastSuccess(
        `Found ${report.games.length} game${
          report.games.length === 1 ? "" : "s"
        }.`,
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanning = false;
      progress = null;
    }
  }

  async function track(game: DetectedGame) {
    if (!game.found_paths.length) {
      toastError(
        "No save folder yet — start the game once, then re-scan to track it.",
      );
      return;
    }
    try {
      const saved = await api.addGameToTracking({
        game_slug: game.slug,
        local_path: game.found_paths[0],
      });
      tracked = [...tracked, saved];
      toastSuccess(`Now tracking ${game.display_name}.`);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  // ---- derived views -------------------------------------------------

  const trackedSlugs = $derived(new Set(tracked.map((t) => t.game_slug)));

  const filtered = $derived.by(() => {
    if (!report) return [];
    const q = search.trim().toLowerCase();
    return report.games.filter((g) => {
      if (q && !g.display_name.toLowerCase().includes(q)) return false;
      if (confidenceFilter !== "all" && g.confidence !== confidenceFilter)
        return false;
      if (sourceFilter !== "all" && g.source !== sourceFilter) return false;
      return true;
    });
  });

  function confidenceLabel(c: Confidence): string {
    return c === "high" ? "High" : c === "medium" ? "Medium" : "Low";
  }

  function sourceLabel(s: DetectionSource): string {
    if (s === "both") return "Steam + filesystem";
    if (s === "steam_library") return "Steam";
    return "Filesystem";
  }

  function sourceBadgeClass(s: DetectionSource): string {
    if (s === "both")
      return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    if (s === "steam_library")
      return "bg-sky-500/10 text-sky-300 ring-sky-500/30";
    return "bg-zinc-500/10 text-zinc-300 ring-zinc-500/30";
  }

  function confidenceBadgeClass(c: Confidence): string {
    if (c === "high")
      return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    if (c === "medium")
      return "bg-amber-500/10 text-amber-300 ring-amber-500/30";
    return "bg-zinc-500/10 text-zinc-300 ring-zinc-500/30";
  }

  const pct = $derived.by(() => {
    if (!progress || progress.total === 0) return 0;
    return Math.min(100, Math.round((progress.done / progress.total) * 100));
  });

  // Same byte-formatter as QuotaBar; cheap enough to duplicate rather than
  // pull a util module for two callers.
  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  // Sum the on-server disk usage of every tracked save so the section
  // header can show "3 games · 142 MB" at a glance.
  const trackedTotalBytes = $derived(
    tracked.reduce((acc, s) => acc + (s.total_size_bytes ?? 0), 0),
  );

  // Map slug → tracked entry so the detection cards can show a tiny size
  // pill on already-tracked games without an extra render pass.
  const trackedBySlug = $derived.by(() => {
    const m = new Map<string, TrackedSave>();
    for (const s of tracked) m.set(s.game_slug, s);
    return m;
  });
</script>

<div class="mx-auto max-w-6xl px-8 py-8">
  <header class="mb-6 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">Library</h1>
      <p class="mt-1 text-sm text-zinc-400">
        {#if report}
          Scanned {report.catalog_size.toLocaleString()} catalog entries —
          found {report.games.length} on this machine
          {#if report.steam_apps_found > 0}
            (plus {report.steam_apps_found} installed Steam apps)
          {/if}
        {:else}
          Scan your machine to find games whose save folders Hoard knows
          about.
        {/if}
      </p>
    </div>
    <Button onclick={runScan} loading={scanning}>
      <RefreshCw size={16} />
      {scanning ? "Scanning…" : report ? "Rescan" : "Scan now"}
    </Button>
  </header>

  {#if tracked.length > 0}
    <!-- Per-game disk-usage strip. Shows what Hoard is actively backing up
         on this account and how much space each game occupies on the
         server. Sits above the detection results so the user always sees
         their commitment first. -->
    <section class="mb-6">
      <div
        class="mb-2 flex items-center justify-between gap-3 text-xs uppercase tracking-wide text-zinc-500"
      >
        <span>Tracked games</span>
        <span class="tabular-nums normal-case tracking-normal text-zinc-400">
          {tracked.length} game{tracked.length === 1 ? "" : "s"} ·
          {fmtBytes(trackedTotalBytes)}
        </span>
      </div>
      <div
        class="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4"
      >
        {#each tracked as save (save.save_id)}
          <div
            class="flex items-center justify-between gap-2 rounded-md border border-zinc-800 bg-zinc-900/40 p-3"
          >
            <div class="min-w-0">
              <p
                class="truncate text-sm font-medium text-zinc-100"
                title={save.game_slug}
              >
                {save.game_slug}
              </p>
              <p class="truncate text-[11px] text-zinc-500">
                {save.label}
              </p>
            </div>
            <span
              class="shrink-0 rounded bg-zinc-800 px-2 py-0.5 text-xs font-medium tabular-nums text-zinc-300"
              title="Total bytes stored on the server (sum of snapshots)"
            >
              {save.total_size_bytes > 0
                ? fmtBytes(save.total_size_bytes)
                : "—"}
            </span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if scanning && progress}
    <div class="mb-6 rounded-md border border-zinc-800 bg-zinc-900/50 p-4">
      <div class="mb-2 flex items-center justify-between text-xs text-zinc-400">
        <span>Scanning catalog…</span>
        <span>
          {progress.done.toLocaleString()} / {progress.total.toLocaleString()}
        </span>
      </div>
      <div class="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
        <div
          class="h-full bg-amber-500 transition-all"
          style="width: {pct}%"
        ></div>
      </div>
    </div>
  {/if}

  {#if report}
    <div class="mb-5 flex flex-wrap items-center gap-3">
      <div class="flex-1 min-w-[14rem]">
        <Input bind:value={search} placeholder="Search games" icon={SearchIcon} />
      </div>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        <Filter size={14} />
        Confidence
        <select
          bind:value={confidenceFilter}
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm text-zinc-100"
        >
          <option value="all">Any</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>
      </label>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        Source
        <select
          bind:value={sourceFilter}
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm text-zinc-100"
        >
          <option value="all">Any</option>
          <option value="both">Steam + filesystem</option>
          <option value="steam_library">Steam only</option>
          <option value="filesystem_heuristic">Filesystem only</option>
        </select>
      </label>
    </div>

    {#if filtered.length === 0}
      <Card>
        <div class="py-12 text-center text-sm text-zinc-400">
          {report.games.length === 0
            ? "Nothing detected yet. Try installing a few games or running a deeper scan."
            : "No games match those filters."}
        </div>
      </Card>
    {:else}
      <div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
        {#each filtered as game (game.slug)}
          {@const isTracked = trackedSlugs.has(game.slug)}
          <div
            class="flex flex-col rounded-lg border border-zinc-800 bg-zinc-900/40 p-4 transition-colors hover:border-zinc-700"
          >
            <div class="mb-2 flex items-start justify-between gap-2">
              <div class="min-w-0">
                <h3
                  class="truncate text-sm font-medium text-zinc-100"
                  title={game.display_name}
                >
                  {game.display_name}
                </h3>
                <p class="truncate text-xs text-zinc-500" title={game.slug}>
                  {game.slug}
                </p>
              </div>
              <div class="flex shrink-0 flex-col items-end gap-1">
                <span
                  class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ring-1 ring-inset {confidenceBadgeClass(
                    game.confidence,
                  )}"
                >
                  {confidenceLabel(game.confidence)}
                </span>
                <span
                  class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ring-1 ring-inset {sourceBadgeClass(
                    game.source,
                  )}"
                >
                  {#if game.source === "steam_library"}
                    <Gamepad2 size={10} />
                  {:else}
                    <HardDrive size={10} />
                  {/if}
                  {sourceLabel(game.source)}
                </span>
              </div>
            </div>

            {#if game.found_paths.length}
              <p
                class="mt-1 truncate font-mono text-[11px] text-zinc-500"
                title={game.found_paths.join("\n")}
              >
                {game.found_paths[0]}
                {#if game.found_paths.length > 1}
                  <span class="text-zinc-600">
                    (+{game.found_paths.length - 1} more)
                  </span>
                {/if}
              </p>
            {:else}
              <p class="mt-1 text-[11px] italic text-zinc-600">
                No save folder yet
              </p>
            {/if}

            <div class="mt-4 flex items-center gap-2">
              {#if isTracked}
                {@const t = trackedBySlug.get(game.slug)}
                <span
                  class="inline-flex items-center gap-1.5 rounded-md bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-300 ring-1 ring-inset ring-emerald-500/30"
                >
                  <Check size={12} />
                  Tracked
                  {#if t && t.total_size_bytes > 0}
                    <span class="text-emerald-400/70 tabular-nums">
                      · {fmtBytes(t.total_size_bytes)}
                    </span>
                  {/if}
                </span>
              {:else}
                <Button
                  variant="secondary"
                  size="md"
                  onclick={() => track(game)}
                  disabled={!game.found_paths.length}
                >
                  <Plus size={14} />
                  Track
                </Button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {:else if !scanning}
    <Card>
      <div class="py-16 text-center">
        <div
          class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-amber-500/10 text-amber-400 ring-1 ring-amber-500/30"
        >
          <RefreshCw size={20} />
        </div>
        <h2 class="text-base font-medium text-zinc-100">
          No scan yet on this session.
        </h2>
        <p class="mx-auto mt-2 max-w-md text-sm text-zinc-400">
          Hoard will check ~10,000 catalog entries against your filesystem
          and Steam library. The first scan takes 10–30 seconds.
        </p>
        <div class="mt-6">
          <Button onclick={runScan}>
            <RefreshCw size={16} />
            Scan now
          </Button>
        </div>
      </div>
    </Card>
  {/if}
</div>
