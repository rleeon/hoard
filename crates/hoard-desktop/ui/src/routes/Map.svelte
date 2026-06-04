<script lang="ts">
  /**
   * Map — an Obsidian-flavoured constellation of save trees.
   *
   * Semantics (NOT a connection graph): every game is an isolated orb that
   * floats on its own. Branches radiate out of each orb — one branch per
   * partida (the save `label`) — and each branch is a chain of save nodes
   * (the snapshot versions). It reads the existing data only: tracked saves
   * grouped by `game_slug`, and each save's snapshot history. Pure SVG, no
   * graph library — deterministic phyllotaxis layout for the orbs, radial
   * arms for the branches. Zoom/pan with level-of-detail so it stays light.
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";
  import { Map as MapIcon, RotateCcw, History as HistoryIcon, X } from "lucide-svelte";

  import * as api from "../lib/api";
  import type { TrackedSave, SnapshotEntry } from "../lib/api";
  import { formatBytes } from "../lib/utils/format";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  // ---- data ---------------------------------------------------------------

  type BranchStatus = "synced" | "local" | "paused" | "orphan";

  type SaveNode = {
    wx: number;
    wy: number;
    snap: SnapshotEntry;
  };

  type Branch = {
    save: TrackedSave;
    label: string;
    status: BranchStatus;
    nodes: SaveNode[];
    // path string from orb edge through every node
    path: string;
  };

  type GameOrb = {
    slug: string;
    name: string;
    initial: string;
    wx: number;
    wy: number;
    r: number;
    branches: Branch[];
    saveCount: number;
    totalBytes: number;
  };

  let orbs = $state<GameOrb[]>([]);
  let loading = $state(true);

  function prettify(slug: string): string {
    return slug
      .replace(/[-_]+/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase())
      .trim();
  }

  function branchStatus(save: TrackedSave): BranchStatus {
    if (save.orphan) return "orphan";
    if (save.paused) return "paused";
    if (save.last_backup_at == null) return "local";
    return "synced";
  }

  const STATUS_COLOR: Record<BranchStatus, string> = {
    synced: "#10b981", // emerald-500
    local: "#34d399", // emerald-400
    paused: "#f59e0b", // amber-500
    orphan: "#71717a", // zinc-500
  };

  // ---- layout (deterministic) --------------------------------------------

  const NODE_STEP = 30; // distance between save nodes along an arm
  const ARM_GAP = 26; // gap between orb edge and first node
  const GOLDEN = Math.PI * (3 - Math.sqrt(5)); // ~137.5°

  function buildLayout(
    saves: TrackedSave[],
    snapsBySave: Map<string, SnapshotEntry[]>,
  ): GameOrb[] {
    // group by game
    const groups = new Map<string, TrackedSave[]>();
    for (const s of saves) {
      const arr = groups.get(s.game_slug) ?? [];
      arr.push(s);
      groups.set(s.game_slug, arr);
    }

    // first pass: per-orb intrinsic size (radius + max arm reach)
    type Pre = {
      slug: string;
      saves: TrackedSave[];
      r: number;
      reach: number;
    };
    const pre: Pre[] = [];
    for (const [slug, gs] of groups) {
      const branchCount = gs.length;
      const maxSnaps = Math.max(
        1,
        ...gs.map((g) => (snapsBySave.get(g.save_id) ?? []).length),
      );
      const r = 26 + Math.min(18, branchCount * 3);
      const reach = r + ARM_GAP + maxSnaps * NODE_STEP + 30;
      pre.push({ slug, saves: gs, r, reach });
    }
    // largest orbs first → centre of the spiral, calmer composition
    pre.sort((a, b) => b.reach - a.reach);

    const maxReach = Math.max(120, ...pre.map((p) => p.reach));
    // phyllotaxis: min spacing between consecutive points ≈ c, so make c big
    // enough that two neighbouring orbs (each up to maxReach) never overlap.
    const c = maxReach * 1.15 + 70;

    const result: GameOrb[] = [];
    pre.forEach((p, i) => {
      const radius = c * Math.sqrt(i + 0.5);
      const angle = i * GOLDEN;
      const wx = Math.cos(angle) * radius;
      const wy = Math.sin(angle) * radius;

      const branches: Branch[] = [];
      const n = p.saves.length;
      // rotate the fan a touch per-orb so they don't all look identical
      const base = angle * 0.6;
      p.saves.forEach((save, j) => {
        const snaps = (snapsBySave.get(save.save_id) ?? [])
          .slice()
          .sort((a, b) => a.version_num - b.version_num);
        const a = base + (j * 2 * Math.PI) / Math.max(1, n);
        const dirx = Math.cos(a);
        const diry = Math.sin(a);
        // perpendicular for a gentle bow
        const perpx = -diry;
        const perpy = dirx;
        const bow = 14;

        const nodes: SaveNode[] = snaps.map((snap, k) => {
          const d = p.r + ARM_GAP + k * NODE_STEP;
          const t = snaps.length > 1 ? k / (snaps.length - 1) : 0;
          const curve = Math.sin(t * Math.PI) * bow;
          return {
            wx: wx + dirx * d + perpx * curve,
            wy: wy + diry * d + perpy * curve,
            snap,
          };
        });

        // path: from orb edge, through nodes
        const startx = wx + dirx * p.r;
        const starty = wy + diry * p.r;
        let path = `M ${startx.toFixed(1)} ${starty.toFixed(1)}`;
        for (const nd of nodes) path += ` L ${nd.wx.toFixed(1)} ${nd.wy.toFixed(1)}`;

        branches.push({
          save,
          label: save.label,
          status: branchStatus(save),
          nodes,
          path,
        });
      });

      result.push({
        slug: p.slug,
        name: prettify(p.slug),
        initial: prettify(p.slug).charAt(0).toUpperCase() || "?",
        wx,
        wy,
        r: p.r,
        branches,
        saveCount: p.saves.length,
        totalBytes: p.saves.reduce((acc, s) => acc + (s.total_size_bytes || 0), 0),
      });
    });

    return result;
  }

  onMount(async () => {
    try {
      const saves = await api.listTrackedSaves();
      const snapsBySave = new Map<string, SnapshotEntry[]>();
      await Promise.all(
        saves.map(async (s) => {
          try {
            snapsBySave.set(s.save_id, await api.listSaveSnapshots(s.save_id, false));
          } catch {
            snapsBySave.set(s.save_id, []);
          }
        }),
      );
      orbs = buildLayout(saves, snapsBySave);
      // frame everything on first paint
      queueFrameAll();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  });

  // ---- zoom / pan ---------------------------------------------------------

  let svgEl = $state<SVGSVGElement | null>(null);
  let scale = $state(1);
  let tx = $state(0);
  let ty = $state(0);

  // level-of-detail thresholds
  const showBranches = $derived(scale >= 0.45);
  const showNodes = $derived(scale >= 0.9);
  const showLabels = $derived(scale >= 1.3);

  function clamp(v: number, lo: number, hi: number) {
    return Math.max(lo, Math.min(hi, v));
  }

  function frameAll() {
    if (!svgEl || orbs.length === 0) return;
    const rect = svgEl.getBoundingClientRect();
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const o of orbs) {
      const pad = o.r + 60;
      minX = Math.min(minX, o.wx - pad);
      minY = Math.min(minY, o.wy - pad);
      maxX = Math.max(maxX, o.wx + pad);
      maxY = Math.max(maxY, o.wy + pad);
      for (const b of o.branches)
        for (const nd of b.nodes) {
          minX = Math.min(minX, nd.wx - 20);
          minY = Math.min(minY, nd.wy - 20);
          maxX = Math.max(maxX, nd.wx + 20);
          maxY = Math.max(maxY, nd.wy + 20);
        }
    }
    const w = maxX - minX || 1;
    const h = maxY - minY || 1;
    const s = clamp(Math.min(rect.width / w, rect.height / h) * 0.9, 0.2, 2);
    scale = s;
    tx = rect.width / 2 - ((minX + maxX) / 2) * s;
    ty = rect.height / 2 - ((minY + maxY) / 2) * s;
  }

  function queueFrameAll() {
    requestAnimationFrame(() => requestAnimationFrame(frameAll));
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    if (!svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
    const ns = clamp(scale * factor, 0.2, 4);
    tx = mx - (mx - tx) * (ns / scale);
    ty = my - (my - ty) * (ns / scale);
    scale = ns;
  }

  let dragging = $state(false);
  let moved = false;
  let startX = 0;
  let startY = 0;

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    moved = false;
    startX = e.clientX;
    startY = e.clientY;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }
  function onPointerMove(e: PointerEvent) {
    if (dragging) {
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) moved = true;
      tx += dx;
      ty += dy;
      startX = e.clientX;
      startY = e.clientY;
    }
    // tooltip follow
    if (svgEl) {
      const rect = svgEl.getBoundingClientRect();
      pointer = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    }
  }
  function onPointerUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
  }

  function focusOrb(o: GameOrb) {
    if (moved || !svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const ns = clamp(1.5, 0.2, 4);
    scale = ns;
    tx = rect.width / 2 - o.wx * ns;
    ty = rect.height / 2 - o.wy * ns;
  }

  // ---- hover / selection --------------------------------------------------

  let pointer = $state({ x: 0, y: 0 });
  let hovered = $state<{ branch: Branch; node: SaveNode } | null>(null);
  let selected = $state<{ orb: GameOrb; branch: Branch; node: SaveNode } | null>(null);

  function selectNode(orb: GameOrb, branch: Branch, node: SaveNode) {
    if (moved) return;
    selected = { orb, branch, node };
  }

  async function restoreSelected() {
    if (!selected) return;
    const { branch, node } = selected;
    try {
      await api.restoreSnapshot({
        save_id: branch.save.save_id,
        version: node.snap.version_num,
        backup_first: true,
      });
      toastSuccess($_("map.restored", { values: { v: node.snap.version_num } }));
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      if (msg === api.NEEDS_DESTINATION) {
        push(`/history/${branch.save.save_id}`);
      } else {
        toastError(msg);
      }
    }
  }

  function formatRelative(iso: string): string {
    const diff = (Date.now() - new Date(iso).getTime()) / 1000;
    if (diff < 60) return $_("history.relative_just_now");
    if (diff < 3600)
      return $_("history.relative_minutes", { values: { count: Math.floor(diff / 60) } });
    if (diff < 86400)
      return $_("history.relative_hours", { values: { count: Math.floor(diff / 3600) } });
    if (diff < 86400 * 7)
      return $_("history.relative_days", { values: { count: Math.floor(diff / 86400) } });
    return new Date(iso).toLocaleDateString();
  }

  // screen position of a world point (for the HTML tooltip overlay)
  function sx(wx: number) {
    return wx * scale + tx;
  }
  function sy(wy: number) {
    return wy * scale + ty;
  }
</script>

<div class="relative h-full w-full overflow-hidden bg-zinc-950">
  <!-- header overlay -->
  <header
    class="pointer-events-none absolute left-0 right-0 top-0 z-10 flex items-start justify-between p-6"
  >
    <div>
      <h1 class="flex items-center gap-2 text-xl font-semibold tracking-tight text-zinc-100">
        <MapIcon size={20} class="text-emerald-400" />
        {$_("map.title")}
      </h1>
      <p class="mt-1 text-xs text-zinc-500">{$_("map.subtitle")}</p>
    </div>
    {#if orbs.length > 0}
      <button
        type="button"
        onclick={frameAll}
        class="pointer-events-auto rounded-lg border border-zinc-800 bg-zinc-900/80 px-3 py-1.5 text-xs text-zinc-300 backdrop-blur transition hover:border-zinc-700 hover:text-zinc-100"
      >
        {$_("map.recenter")}
      </button>
    {/if}
  </header>

  {#if loading}
    <div class="flex h-full items-center justify-center text-sm text-zinc-500">
      {$_("common.loading")}
    </div>
  {:else if orbs.length === 0}
    <div class="flex h-full flex-col items-center justify-center gap-3">
      <MapIcon size={42} class="text-zinc-700" />
      <p class="text-sm text-zinc-300">{$_("map.empty_title")}</p>
      <p class="max-w-xs text-center text-xs text-zinc-500">{$_("map.empty_body")}</p>
      <button
        type="button"
        onclick={() => push("/library")}
        class="mt-1 rounded-lg bg-emerald-600 px-4 py-1.5 text-sm font-medium text-white transition hover:bg-emerald-500"
      >
        {$_("map.go_library")}
      </button>
    </div>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <svg
      bind:this={svgEl}
      class="h-full w-full touch-none select-none"
      class:cursor-grab={!dragging}
      class:cursor-grabbing={dragging}
      onwheel={onWheel}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointerleave={onPointerUp}
    >
      <defs>
        <radialGradient id="orbFill" cx="35%" cy="30%" r="75%">
          <stop offset="0%" stop-color="#34d399" />
          <stop offset="55%" stop-color="#059669" />
          <stop offset="100%" stop-color="#064e3b" />
        </radialGradient>
        <radialGradient id="orbGlow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="#10b981" stop-opacity="0.35" />
          <stop offset="100%" stop-color="#10b981" stop-opacity="0" />
        </radialGradient>
      </defs>

      <g transform={`translate(${tx} ${ty}) scale(${scale})`}>
        {#each orbs as orb (orb.slug)}
          <!-- branches -->
          {#if showBranches}
            {#each orb.branches as branch (branch.save.save_id)}
              <path
                d={branch.path}
                fill="none"
                stroke={STATUS_COLOR[branch.status]}
                stroke-opacity="0.5"
                stroke-width={2 / scale + 0.5}
                stroke-linecap="round"
              />
              {#if showNodes}
                {#each branch.nodes as node (node.snap.version_num)}
                  <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
                  <circle
                    cx={node.wx}
                    cy={node.wy}
                    r={selected?.node === node ? 7 : 5}
                    fill="#0a0a0a"
                    stroke={STATUS_COLOR[branch.status]}
                    stroke-width={selected?.node === node ? 3 : 2}
                    class="cursor-pointer transition-[r]"
                    onpointerenter={() => (hovered = { branch, node })}
                    onpointerleave={() => (hovered = null)}
                    onclick={() => selectNode(orb, branch, node)}
                  />
                  {#if showLabels}
                    <text
                      x={node.wx}
                      y={node.wy - 10}
                      text-anchor="middle"
                      font-size="9"
                      fill="#a1a1aa"
                      class="pointer-events-none"
                    >
                      v{node.snap.version_num}
                    </text>
                  {/if}
                {/each}
              {/if}
              {#if showLabels && branch.nodes.length > 0}
                <text
                  x={branch.nodes[branch.nodes.length - 1].wx}
                  y={branch.nodes[branch.nodes.length - 1].wy + 18}
                  text-anchor="middle"
                  font-size="10"
                  fill={STATUS_COLOR[branch.status]}
                  class="pointer-events-none font-medium"
                >
                  {branch.label}
                </text>
              {/if}
            {/each}
          {/if}

          <!-- orb -->
          <circle cx={orb.wx} cy={orb.wy} r={orb.r * 2.2} fill="url(#orbGlow)" class="pointer-events-none" />
          <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
          <circle
            cx={orb.wx}
            cy={orb.wy}
            r={orb.r}
            fill="url(#orbFill)"
            stroke="#10b981"
            stroke-opacity="0.6"
            stroke-width="1.5"
            class="cursor-pointer"
            onclick={() => focusOrb(orb)}
          />
          <text
            x={orb.wx}
            y={orb.wy + orb.r * 0.34}
            text-anchor="middle"
            font-size={orb.r}
            font-weight="700"
            fill="#ecfdf5"
            class="pointer-events-none"
          >
            {orb.initial}
          </text>
          <text
            x={orb.wx}
            y={orb.wy + orb.r + 16}
            text-anchor="middle"
            font-size="12"
            font-weight="600"
            fill="#e4e4e7"
            class="pointer-events-none"
          >
            {orb.name}
          </text>
          <text
            x={orb.wx}
            y={orb.wy + orb.r + 30}
            text-anchor="middle"
            font-size="10"
            fill="#71717a"
            class="pointer-events-none"
          >
            {$_("map.branch_count", { values: { count: orb.saveCount } })}
          </text>
        {/each}
      </g>
    </svg>

    <!-- hover tooltip -->
    {#if hovered}
      <div
        class="pointer-events-none absolute z-20 -translate-x-1/2 -translate-y-full rounded-lg border border-zinc-700 bg-zinc-900/95 px-3 py-2 text-xs shadow-xl backdrop-blur"
        style={`left:${sx(hovered.node.wx)}px; top:${sy(hovered.node.wy) - 12}px;`}
      >
        <div class="font-semibold text-zinc-100">
          {hovered.branch.label} · v{hovered.node.snap.version_num}
        </div>
        <div class="mt-0.5 text-zinc-400">
          {formatBytes(hovered.node.snap.total_size_bytes)} ·
          {formatRelative(hovered.node.snap.created_at)}
        </div>
      </div>
    {/if}

    <!-- legend -->
    <div
      class="pointer-events-none absolute bottom-4 left-6 z-10 flex gap-3 text-[10px] text-zinc-500"
    >
      <span class="flex items-center gap-1"
        ><span class="h-2 w-2 rounded-full" style="background:#10b981"></span>{$_("map.status_synced")}</span
      >
      <span class="flex items-center gap-1"
        ><span class="h-2 w-2 rounded-full" style="background:#f59e0b"></span>{$_("map.status_paused")}</span
      >
      <span class="flex items-center gap-1"
        ><span class="h-2 w-2 rounded-full" style="background:#71717a"></span>{$_("map.status_orphan")}</span
      >
    </div>
  {/if}

  <!-- side panel -->
  {#if selected}
    <aside
      class="absolute right-0 top-0 z-30 flex h-full w-72 flex-col border-l border-zinc-800 bg-zinc-900/95 p-5 backdrop-blur"
    >
      <div class="flex items-start justify-between">
        <div>
          <p class="text-xs uppercase tracking-wide text-emerald-400">{selected.orb.name}</p>
          <h2 class="mt-1 text-lg font-semibold text-zinc-100">
            {selected.branch.label}
          </h2>
        </div>
        <button
          type="button"
          onclick={() => (selected = null)}
          class="rounded p-1 text-zinc-500 transition hover:bg-zinc-800 hover:text-zinc-200"
          aria-label="Close"
        >
          <X size={16} />
        </button>
      </div>

      <dl class="mt-5 space-y-2.5 text-sm">
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_version")}</dt>
          <dd class="font-medium text-zinc-200">v{selected.node.snap.version_num}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_size")}</dt>
          <dd class="font-medium text-zinc-200">
            {formatBytes(selected.node.snap.total_size_bytes)}
          </dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_files")}</dt>
          <dd class="font-medium text-zinc-200">{selected.node.snap.file_count}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_date")}</dt>
          <dd class="font-medium text-zinc-200">{formatRelative(selected.node.snap.created_at)}</dd>
        </div>
      </dl>

      <div class="mt-auto space-y-2 pt-5">
        <button
          type="button"
          onclick={restoreSelected}
          class="flex w-full items-center justify-center gap-2 rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-emerald-500"
        >
          <RotateCcw size={15} />
          {$_("map.restore")}
        </button>
        <button
          type="button"
          onclick={() => push(`/history/${selected!.branch.save.save_id}`)}
          class="flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-700 px-4 py-2 text-sm text-zinc-300 transition hover:border-zinc-600 hover:text-zinc-100"
        >
          <HistoryIcon size={15} />
          {$_("map.open_history")}
        </button>
      </div>
    </aside>
  {/if}
</div>
