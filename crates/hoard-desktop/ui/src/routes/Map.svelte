<script lang="ts">
  /**
   * Map — a living constellation of save trees.
   *
   * Semantics (NOT a connection graph): every game is an isolated orb. Branches
   * radiate out of each orb — one per partida (`label`) — and each branch is a
   * chain of save nodes (snapshot versions). Read-only over existing data:
   * tracked saves grouped by `game_slug` + each save's snapshot history.
   *
   * Rendered on a <canvas> with a lightweight physics loop: orbs repel each
   * other and drift; save nodes hang off their orb by spring chains, repel
   * their siblings, and float. Canvas + a single redraw per frame keeps it
   * smooth under zoom where the old reactive-SVG version choked. Nodes are
   * drawn at a constant *screen* size so they never shrink into nothing when
   * you zoom out.
   */
  import { onMount, onDestroy } from "svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";
  import { Map as MapIcon, RotateCcw, History as HistoryIcon, X } from "lucide-svelte";

  import * as api from "../lib/api";
  import type { TrackedSave, SnapshotEntry } from "../lib/api";
  import { formatBytes } from "../lib/utils/format";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  // ---- model --------------------------------------------------------------

  type BranchStatus = "synced" | "local" | "paused" | "orphan";

  type Node = {
    x: number;
    y: number;
    vx: number;
    vy: number;
    phase: number;
    snap: SnapshotEntry;
  };

  type Branch = {
    save: TrackedSave;
    label: string;
    status: BranchStatus;
    nodes: Node[];
  };

  type Orb = {
    slug: string;
    name: string;
    initial: string;
    x: number;
    y: number;
    vx: number;
    vy: number;
    phase: number;
    r: number;
    branches: Branch[];
    saveCount: number;
    totalBytes: number;
  };

  let orbs: Orb[] = [];
  let nodeCount = 0;
  let loading = $state(true);
  let isEmpty = $state(false);

  const STATUS_COLOR: Record<BranchStatus, string> = {
    synced: "#10b981",
    local: "#34d399",
    paused: "#f59e0b",
    orphan: "#71717a",
  };

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

  // ---- initial layout (seed positions; physics relaxes from here) ---------

  const NODE_STEP = 34;
  const ARM_GAP = 30;
  const GOLDEN = Math.PI * (3 - Math.sqrt(5));

  function build(saves: TrackedSave[], snapsBySave: Map<string, SnapshotEntry[]>): Orb[] {
    const groups = new Map<string, TrackedSave[]>();
    for (const s of saves) {
      const arr = groups.get(s.game_slug) ?? [];
      arr.push(s);
      groups.set(s.game_slug, arr);
    }

    type Pre = { slug: string; saves: TrackedSave[]; r: number; reach: number };
    const pre: Pre[] = [];
    for (const [slug, gs] of groups) {
      const maxSnaps = Math.max(1, ...gs.map((g) => (snapsBySave.get(g.save_id) ?? []).length));
      const r = 26 + Math.min(18, gs.length * 3);
      pre.push({ slug, saves: gs, r, reach: r + ARM_GAP + maxSnaps * NODE_STEP + 30 });
    }
    pre.sort((a, b) => b.reach - a.reach);
    const maxReach = Math.max(140, ...pre.map((p) => p.reach));
    const c = maxReach * 1.25 + 90;

    const result: Orb[] = [];
    let nc = 0;
    pre.forEach((p, i) => {
      const radius = c * Math.sqrt(i + 0.5);
      const angle = i * GOLDEN;
      const ox = Math.cos(angle) * radius;
      const oy = Math.sin(angle) * radius;

      const branches: Branch[] = [];
      const n = p.saves.length;
      const base = angle * 0.6;
      p.saves.forEach((save, j) => {
        const snaps = (snapsBySave.get(save.save_id) ?? [])
          .slice()
          .sort((a, b) => a.version_num - b.version_num);
        const a = base + (j * 2 * Math.PI) / Math.max(1, n);
        const dx = Math.cos(a);
        const dy = Math.sin(a);
        const nodes: Node[] = snaps.map((snap, k) => {
          const d = p.r + ARM_GAP + k * NODE_STEP;
          nc++;
          return {
            x: ox + dx * d,
            y: oy + dy * d,
            vx: 0,
            vy: 0,
            phase: Math.random() * Math.PI * 2,
            snap,
          };
        });
        branches.push({ save, label: save.label, status: branchStatus(save), nodes });
      });

      result.push({
        slug: p.slug,
        name: prettify(p.slug),
        initial: prettify(p.slug).charAt(0).toUpperCase() || "?",
        x: ox,
        y: oy,
        vx: 0,
        vy: 0,
        phase: Math.random() * Math.PI * 2,
        r: p.r,
        branches,
        saveCount: p.saves.length,
        totalBytes: p.saves.reduce((acc, s) => acc + (s.total_size_bytes || 0), 0),
      });
    });
    nodeCount = nc;
    return result;
  }

  // ---- physics ------------------------------------------------------------

  const DAMP = 0.9;
  const MAX_ACC = 2.8;
  const MAX_VEL = 5;
  const ORB_REPULSION = 90000;
  const NODE_REPULSION = 6000;
  const ORB_NODE_REPULSION = 26000;
  const SPRING_K = 0.06;
  const CENTER_PULL = 0.0009;
  const FLOAT_AMP = 0.22;

  let reduceMotion = false;
  let t = 0;

  function clampMag(ax: number, ay: number, max: number): [number, number] {
    const m = Math.hypot(ax, ay);
    if (m > max) {
      const s = max / m;
      return [ax * s, ay * s];
    }
    return [ax, ay];
  }

  function step() {
    t += 0.016;

    // accumulate accelerations
    const N = orbs.length;
    for (let i = 0; i < N; i++) {
      const o = orbs[i];
      let ax = -CENTER_PULL * o.x;
      let ay = -CENTER_PULL * o.y;

      // orb ↔ orb repulsion
      for (let k = 0; k < N; k++) {
        if (k === i) continue;
        const ob = orbs[k];
        const dx = o.x - ob.x;
        const dy = o.y - ob.y;
        const d2 = dx * dx + dy * dy + 1;
        if (d2 > 1_200_000) continue; // cutoff
        const f = ORB_REPULSION / d2;
        const inv = 1 / Math.sqrt(d2);
        ax += dx * inv * f;
        ay += dy * inv * f;
      }

      // gentle perpetual float
      ax += Math.cos(t * 0.6 + o.phase) * FLOAT_AMP;
      ay += Math.sin(t * 0.5 + o.phase * 1.3) * FLOAT_AMP;

      [ax, ay] = clampMag(ax, ay, MAX_ACC);
      o.vx = (o.vx + ax) * DAMP;
      o.vy = (o.vy + ay) * DAMP;
      [o.vx, o.vy] = clampMag(o.vx, o.vy, MAX_VEL);

      // ---- nodes belonging to this orb ----
      for (const b of o.branches) {
        for (let n = 0; n < b.nodes.length; n++) {
          const node = b.nodes[n];
          let nax = 0;
          let nay = 0;

          // spring to parent (orb for first node, previous node otherwise)
          const px = n === 0 ? o.x : b.nodes[n - 1].x;
          const py = n === 0 ? o.y : b.nodes[n - 1].y;
          const rest = n === 0 ? o.r + ARM_GAP : NODE_STEP;
          let dx = node.x - px;
          let dy = node.y - py;
          let d = Math.hypot(dx, dy) || 1;
          const stretch = d - rest;
          nax -= (dx / d) * stretch * SPRING_K;
          nay -= (dy / d) * stretch * SPRING_K;

          // push away from the orb body
          dx = node.x - o.x;
          dy = node.y - o.y;
          let d2 = dx * dx + dy * dy + 1;
          let f = ORB_NODE_REPULSION / d2;
          let inv = 1 / Math.sqrt(d2);
          nax += dx * inv * f;
          nay += dy * inv * f;

          // repel sibling nodes of the same orb so branches fan out
          for (const b2 of o.branches) {
            for (const other of b2.nodes) {
              if (other === node) continue;
              dx = node.x - other.x;
              dy = node.y - other.y;
              d2 = dx * dx + dy * dy + 1;
              if (d2 > 40000) continue;
              f = NODE_REPULSION / d2;
              inv = 1 / Math.sqrt(d2);
              nax += dx * inv * f;
              nay += dy * inv * f;
            }
          }

          // float
          nax += Math.cos(t * 0.9 + node.phase) * FLOAT_AMP * 0.8;
          nay += Math.sin(t * 0.8 + node.phase * 1.2) * FLOAT_AMP * 0.8;

          [nax, nay] = clampMag(nax, nay, MAX_ACC);
          node.vx = (node.vx + nax) * DAMP;
          node.vy = (node.vy + nay) * DAMP;
          [node.vx, node.vy] = clampMag(node.vx, node.vy, MAX_VEL);
        }
      }
    }

    // integrate
    for (const o of orbs) {
      o.x += o.vx;
      o.y += o.vy;
      for (const b of o.branches)
        for (const node of b.nodes) {
          node.x += node.vx;
          node.y += node.vy;
        }
    }
  }

  // ---- camera -------------------------------------------------------------

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let ctx: CanvasRenderingContext2D | null = null;
  let dpr = 1;
  let cssW = 0;
  let cssH = 0;

  let scale = 1;
  let tx = 0;
  let ty = 0;

  function clamp(v: number, lo: number, hi: number) {
    return Math.max(lo, Math.min(hi, v));
  }

  function worldBounds() {
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const o of orbs) {
      const pad = o.r + 70;
      minX = Math.min(minX, o.x - pad);
      minY = Math.min(minY, o.y - pad);
      maxX = Math.max(maxX, o.x + pad);
      maxY = Math.max(maxY, o.y + pad);
      for (const b of o.branches)
        for (const n of b.nodes) {
          minX = Math.min(minX, n.x - 24);
          minY = Math.min(minY, n.y - 24);
          maxX = Math.max(maxX, n.x + 24);
          maxY = Math.max(maxY, n.y + 24);
        }
    }
    return { minX, minY, maxX, maxY };
  }

  function frameAll() {
    if (orbs.length === 0 || cssW === 0) return;
    const { minX, minY, maxX, maxY } = worldBounds();
    const w = maxX - minX || 1;
    const h = maxY - minY || 1;
    scale = clamp(Math.min(cssW / w, cssH / h) * 0.9, 0.2, 2);
    tx = cssW / 2 - ((minX + maxX) / 2) * scale;
    ty = cssH / 2 - ((minY + maxY) / 2) * scale;
  }

  // ---- rendering ----------------------------------------------------------

  const NODE_SCREEN_R = 4.5; // constant on-screen px so nodes never vanish

  function draw() {
    if (!ctx) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, cssW * dpr, cssH * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.translate(tx, ty);
    ctx.scale(scale, scale);

    const nodeR = NODE_SCREEN_R / scale;
    const lineW = 1.6 / scale;
    const showLabels = scale >= 1.15;
    const showVersions = scale >= 1.7;

    // branches + nodes
    for (const o of orbs) {
      for (const b of o.branches) {
        const col = STATUS_COLOR[b.status];
        // path from orb through nodes
        ctx.beginPath();
        ctx.moveTo(o.x, o.y);
        for (const n of b.nodes) ctx.lineTo(n.x, n.y);
        ctx.strokeStyle = col;
        ctx.globalAlpha = 0.45;
        ctx.lineWidth = lineW;
        ctx.lineJoin = "round";
        ctx.lineCap = "round";
        ctx.stroke();
        ctx.globalAlpha = 1;

        // nodes
        for (const n of b.nodes) {
          const isSel = selected?.node === n;
          const isHov = hovered?.node === n;
          const r = (isSel || isHov ? NODE_SCREEN_R + 2 : NODE_SCREEN_R) / scale;
          ctx.beginPath();
          ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
          ctx.fillStyle = "#0a0a0a";
          ctx.fill();
          ctx.lineWidth = (isSel ? 3 : 2) / scale;
          ctx.strokeStyle = col;
          ctx.stroke();
        }

        if (showVersions) {
          ctx.fillStyle = "#a1a1aa";
          ctx.font = `${10 / scale}px "Geist Sans", sans-serif`;
          ctx.textAlign = "center";
          for (const n of b.nodes) ctx.fillText(`v${n.snap.version_num}`, n.x, n.y - nodeR - 6 / scale);
        }
        if (showLabels && b.nodes.length > 0) {
          const last = b.nodes[b.nodes.length - 1];
          ctx.fillStyle = col;
          ctx.font = `600 ${11 / scale}px "Geist Sans", sans-serif`;
          ctx.textAlign = "center";
          ctx.fillText(b.label, last.x, last.y + 16 / scale);
        }
      }
    }

    // orbs
    for (const o of orbs) {
      // glow
      const glow = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, o.r * 2.2);
      glow.addColorStop(0, "rgba(16,185,129,0.30)");
      glow.addColorStop(1, "rgba(16,185,129,0)");
      ctx.beginPath();
      ctx.arc(o.x, o.y, o.r * 2.2, 0, Math.PI * 2);
      ctx.fillStyle = glow;
      ctx.fill();

      // body
      const fill = ctx.createRadialGradient(
        o.x - o.r * 0.3,
        o.y - o.r * 0.35,
        o.r * 0.2,
        o.x,
        o.y,
        o.r,
      );
      fill.addColorStop(0, "#34d399");
      fill.addColorStop(0.55, "#059669");
      fill.addColorStop(1, "#064e3b");
      ctx.beginPath();
      ctx.arc(o.x, o.y, o.r, 0, Math.PI * 2);
      ctx.fillStyle = fill;
      ctx.fill();
      ctx.lineWidth = 1.5 / scale;
      ctx.strokeStyle = "rgba(16,185,129,0.6)";
      ctx.stroke();

      // initial
      ctx.fillStyle = "#ecfdf5";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.font = `700 ${o.r}px "Fraunces", serif`;
      ctx.fillText(o.initial, o.x, o.y + 1);
      ctx.textBaseline = "alphabetic";

      // name + count
      ctx.fillStyle = "#e4e4e7";
      ctx.font = `600 ${12 / scale}px "Geist Sans", sans-serif`;
      ctx.fillText(o.name, o.x, o.y + o.r + 16 / scale);
      ctx.fillStyle = "#71717a";
      ctx.font = `${10 / scale}px "Geist Sans", sans-serif`;
      ctx.fillText(
        $_("map.branch_count", { values: { count: o.saveCount } }),
        o.x,
        o.y + o.r + 30 / scale,
      );
    }
  }

  // ---- loop ---------------------------------------------------------------

  let raf = 0;
  let running = false;

  function loop() {
    if (!running) return;
    if (!document.hidden) {
      if (!reduceMotion) step();
      draw();
    }
    raf = requestAnimationFrame(loop);
  }

  function startLoop() {
    if (running) return;
    running = true;
    raf = requestAnimationFrame(loop);
  }

  // ---- interaction --------------------------------------------------------

  let pointer = $state({ x: 0, y: 0 });
  let hovered = $state<{ branch: Branch; node: Node } | null>(null);
  let selected = $state<{ orb: Orb; branch: Branch; node: Node } | null>(null);

  let dragging = $state(false);
  let moved = false;
  let startX = 0;
  let startY = 0;

  function screenToWorld(sx: number, sy: number) {
    return { x: (sx - tx) / scale, y: (sy - ty) / scale };
  }

  function hitNode(sx: number, sy: number) {
    const w = screenToWorld(sx, sy);
    const rWorld = (NODE_SCREEN_R + 6) / scale;
    let best: { orb: Orb; branch: Branch; node: Node } | null = null;
    let bestD = rWorld * rWorld;
    for (const o of orbs)
      for (const b of o.branches)
        for (const n of b.nodes) {
          const dx = n.x - w.x;
          const dy = n.y - w.y;
          const d2 = dx * dx + dy * dy;
          if (d2 < bestD) {
            bestD = d2;
            best = { orb: o, branch: b, node: n };
          }
        }
    return best;
  }

  function hitOrb(sx: number, sy: number) {
    const w = screenToWorld(sx, sy);
    for (const o of orbs) {
      const dx = o.x - w.x;
      const dy = o.y - w.y;
      if (dx * dx + dy * dy <= o.r * o.r) return o;
    }
    return null;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
    const ns = clamp(scale * factor, 0.18, 4.5);
    tx = mx - (mx - tx) * (ns / scale);
    ty = my - (my - ty) * (ns / scale);
    scale = ns;
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    moved = false;
    startX = e.clientX;
    startY = e.clientY;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    pointer = { x: px, y: py };

    if (dragging) {
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) moved = true;
      tx += dx;
      ty += dy;
      startX = e.clientX;
      startY = e.clientY;
      return;
    }
    const hit = hitNode(px, py);
    hovered = hit ? { branch: hit.branch, node: hit.node } : null;
  }

  function onPointerUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
  }

  function onClick(e: PointerEvent) {
    if (moved || !canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    const node = hitNode(px, py);
    if (node) {
      selected = node;
      return;
    }
    const orb = hitOrb(px, py);
    if (orb) {
      const ns = clamp(1.6, 0.18, 4.5);
      scale = ns;
      tx = cssW / 2 - orb.x * ns;
      ty = cssH / 2 - orb.y * ns;
    }
  }

  // ---- sizing -------------------------------------------------------------

  let ro: ResizeObserver | null = null;

  function resize() {
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    cssW = rect.width;
    cssH = rect.height;
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvasEl.width = Math.round(cssW * dpr);
    canvasEl.height = Math.round(cssH * dpr);
  }

  // ---- lifecycle ----------------------------------------------------------

  onMount(async () => {
    reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
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
      orbs = build(saves, snapsBySave);
      isEmpty = orbs.length === 0;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }

    // wait a tick for the canvas to mount, then size + frame + run
    requestAnimationFrame(() => {
      if (!canvasEl) return;
      ctx = canvasEl.getContext("2d");
      resize();
      ro = new ResizeObserver(() => {
        resize();
        draw();
      });
      ro.observe(canvasEl);
      // pre-relax so the first paint is already tidy
      if (orbs.length > 0) {
        const settle = reduceMotion ? 220 : 40;
        for (let i = 0; i < settle; i++) step();
        frameAll();
      }
      startLoop();
    });
  });

  onDestroy(() => {
    running = false;
    cancelAnimationFrame(raf);
    ro?.disconnect();
  });

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
      if (msg === api.NEEDS_DESTINATION) push(`/history/${branch.save.save_id}`);
      else toastError(msg);
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
</script>

<div class="relative h-full w-full overflow-hidden">
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
    {#if !isEmpty && !loading}
      <button
        type="button"
        onclick={frameAll}
        class="pointer-events-auto rounded-lg border border-white/[0.08] bg-zinc-900/70 px-3 py-1.5 text-xs text-zinc-300 backdrop-blur transition hover:border-white/[0.16] hover:text-zinc-100"
      >
        {$_("map.recenter")}
      </button>
    {/if}
  </header>

  {#if loading}
    <div class="flex h-full items-center justify-center text-sm text-zinc-500">
      {$_("common.loading")}
    </div>
  {:else if isEmpty}
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
  {/if}

  <!-- canvas is always mounted (unless empty/loading) so the ctx is ready -->
  {#if !loading && !isEmpty}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <canvas
      bind:this={canvasEl}
      class="h-full w-full touch-none select-none"
      class:cursor-grab={!dragging}
      class:cursor-grabbing={dragging}
      onwheel={onWheel}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={(e) => {
        onPointerUp(e);
        onClick(e);
      }}
      onpointerleave={onPointerUp}
    ></canvas>

    <!-- hover tooltip -->
    {#if hovered}
      <div
        class="pointer-events-none absolute z-20 -translate-x-1/2 -translate-y-full rounded-lg border border-white/[0.08] bg-zinc-900/95 px-3 py-2 text-xs shadow-xl backdrop-blur"
        style={`left:${pointer.x}px; top:${pointer.y - 14}px;`}
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
      class="absolute right-0 top-0 z-30 flex h-full w-72 flex-col border-l border-white/[0.08] bg-zinc-900/95 p-5 backdrop-blur"
    >
      <div class="flex items-start justify-between">
        <div>
          <p class="text-xs uppercase tracking-wide text-emerald-400">{selected.orb.name}</p>
          <h2 class="mt-1 text-lg font-semibold text-zinc-100">{selected.branch.label}</h2>
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
          <dd class="font-medium text-zinc-200">{formatBytes(selected.node.snap.total_size_bytes)}</dd>
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
          class="flex w-full items-center justify-center gap-2 rounded-lg border border-white/[0.1] px-4 py-2 text-sm text-zinc-300 transition hover:border-white/[0.2] hover:text-zinc-100"
        >
          <HistoryIcon size={15} />
          {$_("map.open_history")}
        </button>
      </div>
    </aside>
  {/if}
</div>
