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
   * other and drift; each save node has its own physics — a radial spring
   * gravitates it toward its OWN orb, a slack chain spring keeps a branch
   * legible, and sibling/cross-game repulsion plus gentle breathing keep them
   * alive. Both orbs and individual saves are draggable. Canvas + a single
   * redraw per frame keeps it
   * smooth under zoom where the old reactive-SVG version choked. Nodes are
   * drawn at a constant *screen* size so they never shrink into nothing when
   * you zoom out.
   */
  import { onMount, onDestroy } from "svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";
  import { Map as MapIcon, History as HistoryIcon, X, Pause, Play } from "lucide-svelte";

  import * as api from "../lib/api";
  import type { TrackedSave, SnapshotEntry, DetectedGame } from "../lib/api";
  import { formatBytes } from "../lib/utils/format";
  import { toastError } from "../lib/stores/toasts";
  import { coverUrl } from "../lib/stores/covers";
  import Cover from "../lib/components/Cover.svelte";

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
    steamAppId: number | null;
    /** `true` when this game has at least one tracked save (full orb with
     *  branches). `false` for games we merely detected on disk — those show
     *  as dimmed "not tracked yet" orbs you can click to add. */
    tracked: boolean;
    /** `true` once the physics loop has put this orb (and its nodes) to
     *  rest. Used so a dragged or freshly-added orb settles and then stops
     *  consuming CPU — the Obsidian-style "still until you nudge it" feel. */
    asleep: boolean;
  };

  let orbs: Orb[] = [];
  // slug → Steam app id, sourced best-effort from the cached detection sweep.
  // Used only to fetch cover art; absence just means we fall back to the
  // initial-letter orb.
  let appIdBySlug = new Map<string, number>();
  // appid → lazily-loaded cover <img>. `ready` flips on load; we request a
  // one-off repaint so the cover pops in even if the loop is asleep. A null
  // entry marks a permanent failure (offline / 404) so we don't retry.
  const coverCache = new Map<number, { img: HTMLImageElement; ready: boolean } | null>();

  function getCover(appId: number): HTMLImageElement | null {
    const hit = coverCache.get(appId);
    if (hit !== undefined) return hit && hit.ready ? hit.img : null;
    // Mark in-flight (blank, not-ready) so repeated frames don't refetch.
    const entry: { img: HTMLImageElement; ready: boolean } = { img: new Image(), ready: false };
    coverCache.set(appId, entry);
    // Bytes come from the on-device cache (downloaded once, then read from
    // disk) as an object URL — no per-frame network hit, no canvas taint.
    coverUrl(appId).then((url) => {
      if (!url) {
        coverCache.set(appId, null);
        return;
      }
      entry.img.onload = () => {
        entry.ready = true;
        requestDraw(); // the canvas may be asleep; repaint so the cover pops in
      };
      entry.img.onerror = () => coverCache.set(appId, null);
      entry.img.src = url;
    });
    return null;
  }
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

  function build(
    saves: TrackedSave[],
    snapsBySave: Map<string, SnapshotEntry[]>,
    detected: DetectedGame[],
  ): Orb[] {
    const groups = new Map<string, TrackedSave[]>();
    for (const s of saves) {
      const arr = groups.get(s.game_slug) ?? [];
      arr.push(s);
      groups.set(s.game_slug, arr);
    }

    type Pre = {
      slug: string;
      name: string;
      saves: TrackedSave[];
      tracked: boolean;
      appId: number | null;
      r: number;
      reach: number;
    };
    const pre: Pre[] = [];
    for (const [slug, gs] of groups) {
      const maxSnaps = Math.max(1, ...gs.map((g) => (snapsBySave.get(g.save_id) ?? []).length));
      const r = 26 + Math.min(18, gs.length * 3);
      pre.push({
        slug,
        name: prettify(slug),
        saves: gs,
        tracked: true,
        appId: appIdBySlug.get(slug) ?? null,
        r,
        reach: r + ARM_GAP + maxSnaps * NODE_STEP + 30,
      });
    }
    // Detected-but-untracked games: only those actually present on disk
    // (found_paths) and not already tracked. These become smaller, dimmed
    // orbs so the map shows the *whole* library, not just what's synced.
    const trackedSlugs = new Set(groups.keys());
    for (const g of detected) {
      if (trackedSlugs.has(g.slug)) continue;
      if (g.found_paths.length === 0) continue;
      pre.push({
        slug: g.slug,
        name: g.display_name || prettify(g.slug),
        saves: [],
        tracked: false,
        appId: g.steam_app_id,
        r: 22,
        reach: 22 + 24,
      });
    }

    // Tracked first (so they take the inner, tighter ring), then the rest.
    pre.sort((a, b) => {
      if (a.tracked !== b.tracked) return a.tracked ? -1 : 1;
      return b.reach - a.reach;
    });
    const maxReach = Math.max(140, ...pre.map((p) => p.reach));
    const c = maxReach * 1.05 + 80;

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
        name: p.name,
        initial: p.name.charAt(0).toUpperCase() || "?",
        x: ox,
        y: oy,
        vx: 0,
        vy: 0,
        phase: Math.random() * Math.PI * 2,
        r: p.r,
        branches,
        saveCount: p.saves.length,
        totalBytes: p.saves.reduce((acc, s) => acc + (s.total_size_bytes || 0), 0),
        steamAppId: p.appId,
        tracked: p.tracked,
        asleep: false,
      });
    });
    nodeCount = nc;
    return result;
  }

  // ---- physics ------------------------------------------------------------

  // Obsidian-style relaxation: nodes hold their position and only move to
  // *separate* when something overlaps them (a drag, a freshly-added orb,
  // the initial seed). No perpetual float, no constant centre drift — once
  // everything is spaced out the simulation goes to sleep and the canvas is
  // perfectly still until you nudge it.
  const DAMP = 0.78;
  const MAX_ACC = 3.2;
  const MAX_VEL = 6;
  const ORB_SEP_MARGIN = 36; // extra gap enforced between orb bodies
  const ORB_SEP_K = 0.5; // how hard overlapping orbs push apart
  const NODE_REPULSION = 5200; // short-range, only to fan siblings out
  const OTHER_ORB_REPULSION = 22000; // keep a save clear of a neighbour game
  const ORB_GRAVITY_K = 0.06; // each save's pull toward its OWN game orb
  const CHAIN_K = 0.035; // light spring keeping a branch's saves in a line
  // Gentle perpetual breathing so the constellation never freezes into a
  // dead, motionless line — every orb and save keeps drifting a little.
  const IDLE_FORCE = 0.05;
  const IDLE_SPIN = 0.018;
  const SLEEP_EPS = 0.04; // max speed below which we freeze the sim

  let reduceMotion = false;
  // Cleared to `false` by `wake()` (drag, zoom, resize, fresh data); set to
  // `true` by `step()` once the whole system is below `SLEEP_EPS`.
  let settled = false;
  // User-toggled freeze (bottom-left button). When on, the physics loop stops
  // integrating entirely — nodes stay exactly where they are so you can grab
  // and reposition them without the springs and breathing nudging them away.
  // Dragging still pins a node straight to the cursor, and on release it stays
  // put (no spring-back) because nothing steps.
  let frozen = $state(false);

  function toggleFrozen() {
    frozen = !frozen;
    if (!frozen) wake();
  }

  function wake() {
    settled = false;
    requestDraw();
  }

  function clampMag(ax: number, ay: number, max: number): [number, number] {
    const m = Math.hypot(ax, ay);
    if (m > max) {
      const s = max / m;
      return [ax * s, ay * s];
    }
    return [ax, ay];
  }

  /** One relaxation pass. Returns the largest speed seen, so the loop can
   *  decide when the system has come to rest. */
  function step(): number {
    const N = orbs.length;
    for (let i = 0; i < N; i++) {
      const o = orbs[i];
      if (o === draggedOrb) {
        // Pinned under the cursor — no forces, no integration.
        o.vx = 0;
        o.vy = 0;
        continue;
      }
      let ax = 0;
      let ay = 0;

      // orb ↔ orb separation: only when their bodies (plus margin) overlap.
      for (let k = 0; k < N; k++) {
        if (k === i) continue;
        const ob = orbs[k];
        const dx = o.x - ob.x;
        const dy = o.y - ob.y;
        const d = Math.hypot(dx, dy) || 0.0001;
        const minD = o.r + ob.r + ORB_SEP_MARGIN;
        if (d < minD) {
          const push = ((minD - d) / minD) * ORB_SEP_K * (MAX_ACC * 2);
          ax += (dx / d) * push;
          ay += (dy / d) * push;
        }
      }

      // gentle perpetual breathing so the orb never freezes stone-dead
      o.phase += IDLE_SPIN;
      ax += Math.cos(o.phase) * IDLE_FORCE;
      ay += Math.sin(o.phase) * IDLE_FORCE;

      [ax, ay] = clampMag(ax, ay, MAX_ACC);
      o.vx = (o.vx + ax) * DAMP;
      o.vy = (o.vy + ay) * DAMP;
      [o.vx, o.vy] = clampMag(o.vx, o.vy, MAX_VEL);
    }

    // ---- save nodes (still spring off their orb so chains stay readable) --
    // Flatten every node once so branches can repel *across games*, not just
    // their same-orb siblings: no two save-nodes (whoever owns them) should
    // sit on top of each other, and a node should steer clear of any orb body
    // it drifts near — not only its own parent.
    const allNodes: Node[] = [];
    for (const o of orbs) for (const b of o.branches) for (const n of b.nodes) allNodes.push(n);

    for (const o of orbs) {
      for (const b of o.branches) {
        for (let n = 0; n < b.nodes.length; n++) {
          const node = b.nodes[n];
          if (node === draggedNode) {
            // pinned under the cursor — no forces, no integration
            node.vx = 0;
            node.vy = 0;
            continue;
          }
          let nax = 0;
          let nay = 0;

          // (1) gravity toward its OWN orb: a radial spring that pulls the save
          // back to a rest distance scaled by its depth in the chain, so the
          // node genuinely orbits the game it belongs to instead of hanging off
          // a rigid arm.
          let dx = node.x - o.x;
          let dy = node.y - o.y;
          let d = Math.hypot(dx, dy) || 1;
          const restFromOrb = o.r + ARM_GAP + n * NODE_STEP;
          const pull = (d - restFromOrb) * ORB_GRAVITY_K;
          nax -= (dx / d) * pull;
          nay -= (dy / d) * pull;

          // (2) light chain spring to BOTH chain neighbours so a branch still
          // reads as a line — but slack enough to wobble, not a stiff rod. The
          // spring used to point only at the *previous* node (toward the orb),
          // so dragging a save pulled the nodes behind it but never the ones in
          // front (toward the game) — they stayed glued to the orb. Springing to
          // the next node too makes the drag symmetric: grab any save and the
          // whole arm trails with it on both sides.
          if (n > 0) {
            const prev = b.nodes[n - 1];
            dx = node.x - prev.x;
            dy = node.y - prev.y;
            d = Math.hypot(dx, dy) || 1;
            const stretch = d - NODE_STEP;
            nax -= (dx / d) * stretch * CHAIN_K;
            nay -= (dy / d) * stretch * CHAIN_K;
          }
          if (n < b.nodes.length - 1) {
            const next = b.nodes[n + 1];
            dx = node.x - next.x;
            dy = node.y - next.y;
            d = Math.hypot(dx, dy) || 1;
            const stretch = d - NODE_STEP;
            nax -= (dx / d) * stretch * CHAIN_K;
            nay -= (dy / d) * stretch * CHAIN_K;
          }

          // (3) avoid OTHER orbs' bodies (local — only when the node drifts close)
          let d2: number;
          let f: number;
          let inv: number;
          for (const ob of orbs) {
            if (ob === o) continue;
            dx = node.x - ob.x;
            dy = node.y - ob.y;
            d2 = dx * dx + dy * dy + 1;
            const reach = ob.r + 34;
            if (d2 > reach * reach) continue;
            f = OTHER_ORB_REPULSION / d2;
            inv = 1 / Math.sqrt(d2);
            nax += dx * inv * f;
            nay += dy * inv * f;
          }

          // (4) repel every other node within range (own siblings + other games)
          for (const other of allNodes) {
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

          // (5) gentle breathing so saves never freeze into a dead line
          node.phase += IDLE_SPIN;
          nax += Math.cos(node.phase) * IDLE_FORCE;
          nay += Math.sin(node.phase) * IDLE_FORCE;

          [nax, nay] = clampMag(nax, nay, MAX_ACC);
          node.vx = (node.vx + nax) * DAMP;
          node.vy = (node.vy + nay) * DAMP;
          [node.vx, node.vy] = clampMag(node.vx, node.vy, MAX_VEL);
        }
      }
    }

    // integrate + track peak speed
    let peak = 0;
    for (const o of orbs) {
      if (o !== draggedOrb) {
        o.x += o.vx;
        o.y += o.vy;
        peak = Math.max(peak, Math.abs(o.vx), Math.abs(o.vy));
      }
      for (const b of o.branches)
        for (const node of b.nodes) {
          if (node === draggedNode) continue;
          node.x += node.vx;
          node.y += node.vy;
          peak = Math.max(peak, Math.abs(node.vx), Math.abs(node.vy));
        }
    }
    return peak;
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
    // Upper clamp raised from 2 → 3.6 so a small library (a handful of games)
    // actually fills the viewport instead of huddling tiny in the middle.
    scale = clamp(Math.min(cssW / w, cssH / h) * 0.94, 0.2, 3.6);
    tx = cssW / 2 - ((minX + maxX) / 2) * scale;
    ty = cssH / 2 - ((minY + maxY) / 2) * scale;
    requestDraw();
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
    const showLabels = scale >= 0.85;
    const showVersions = scale >= 1.3;

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
      // Untracked (detected-only) games render dimmed so the synced library
      // stays visually dominant. A drag/hover doesn't change this — it's a
      // permanent "this one isn't backed up yet" cue.
      const dim = !o.tracked;
      ctx.globalAlpha = dim ? 0.45 : 1;

      // glow
      const glow = ctx.createRadialGradient(o.x, o.y, 0, o.x, o.y, o.r * 2.2);
      glow.addColorStop(0, dim ? "rgba(113,113,122,0.18)" : "rgba(16,185,129,0.30)");
      glow.addColorStop(1, "rgba(16,185,129,0)");
      ctx.beginPath();
      ctx.arc(o.x, o.y, o.r * 2.2, 0, Math.PI * 2);
      ctx.fillStyle = glow;
      ctx.fill();

      // body — Steam cover art clipped into the circle when available,
      // otherwise the emerald gem with the game's initial.
      const cover = o.steamAppId != null ? getCover(o.steamAppId) : null;
      if (cover) {
        ctx.save();
        ctx.beginPath();
        ctx.arc(o.x, o.y, o.r, 0, Math.PI * 2);
        ctx.clip();
        // cover-fit the (wide) header into the square circle bounds
        const iw = cover.naturalWidth || 460;
        const ih = cover.naturalHeight || 215;
        const side = o.r * 2;
        const sc = Math.max(side / iw, side / ih);
        const dw = iw * sc;
        const dh = ih * sc;
        ctx.drawImage(cover, o.x - dw / 2, o.y - dh / 2, dw, dh);
        ctx.restore();
        ctx.beginPath();
        ctx.arc(o.x, o.y, o.r, 0, Math.PI * 2);
        ctx.lineWidth = 2 / scale;
        ctx.strokeStyle = dim ? "rgba(113,113,122,0.7)" : "rgba(16,185,129,0.85)";
        ctx.stroke();
      } else {
        const fill = ctx.createRadialGradient(
          o.x - o.r * 0.3,
          o.y - o.r * 0.35,
          o.r * 0.2,
          o.x,
          o.y,
          o.r,
        );
        if (dim) {
          fill.addColorStop(0, "#52525b");
          fill.addColorStop(0.55, "#3f3f46");
          fill.addColorStop(1, "#27272a");
        } else {
          fill.addColorStop(0, "#34d399");
          fill.addColorStop(0.55, "#059669");
          fill.addColorStop(1, "#064e3b");
        }
        ctx.beginPath();
        ctx.arc(o.x, o.y, o.r, 0, Math.PI * 2);
        ctx.fillStyle = fill;
        ctx.fill();
        ctx.lineWidth = 1.5 / scale;
        ctx.strokeStyle = dim ? "rgba(113,113,122,0.5)" : "rgba(16,185,129,0.6)";
        ctx.stroke();

        // initial
        ctx.fillStyle = dim ? "#d4d4d8" : "#ecfdf5";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.font = `600 ${o.r * 0.9}px "Geist Sans", sans-serif`;
        ctx.fillText(o.initial, o.x, o.y + 1);
        ctx.textBaseline = "alphabetic";
      }

      // name + count
      ctx.fillStyle = "#e4e4e7";
      ctx.font = `600 ${12 / scale}px "Geist Sans", sans-serif`;
      ctx.fillText(o.name, o.x, o.y + o.r + 16 / scale);
      ctx.fillStyle = "#71717a";
      ctx.font = `${10 / scale}px "Geist Sans", sans-serif`;
      ctx.fillText(
        o.tracked
          ? $_("map.branch_count", { values: { count: o.saveCount } })
          : $_("map.not_tracked"),
        o.x,
        o.y + o.r + 30 / scale,
      );
      ctx.globalAlpha = 1;
    }
  }

  // ---- loop ---------------------------------------------------------------

  let raf = 0;
  let running = false;
  // A pending one-off repaint asked for by a visual-only change (hover,
  // selection, pan, zoom, a cover finishing loading). Lets the loop paint a
  // single frame and go back to sleep without re-running the physics.
  let needsDraw = false;

  // Start the rAF loop if it isn't already spinning and the tab is visible.
  function ensureLoop() {
    if (running || document.hidden) return;
    running = true;
    raf = requestAnimationFrame(loop);
  }

  // Ask for a repaint on the next frame. Cheap and idempotent — the canonical
  // way to refresh the canvas now that the loop sleeps when nothing moves.
  function requestDraw() {
    needsDraw = true;
    ensureLoop();
  }

  function hasMotion(): boolean {
    return !frozen && !reduceMotion && (!settled || !!draggedOrb || !!draggedNode);
  }

  function loop() {
    if (!running) return;
    // Tab hidden mid-flight: park the loop. `visibilitychange` restarts it,
    // so we never burn rAF frames painting a canvas nobody can see.
    if (document.hidden) {
      running = false;
      raf = 0;
      return;
    }
    if (hasMotion()) {
      const peak = step();
      if (peak < SLEEP_EPS && !draggedOrb && !draggedNode) settled = true;
      draw();
    } else if (needsDraw) {
      draw();
    }
    needsDraw = false;
    // Keep the loop alive only while the physics still has work to do.
    // Otherwise stop entirely — `wake()` / `requestDraw()` restart us — so an
    // idle Map costs zero CPU instead of redrawing 60×/s forever.
    if (hasMotion()) {
      raf = requestAnimationFrame(loop);
    } else {
      running = false;
      raf = 0;
    }
  }

  // Pause/resume with tab visibility. Hidden → stop the loop outright; visible
  // again → repaint and let any in-flight motion finish settling.
  function onVisibility() {
    if (document.hidden) {
      running = false;
      cancelAnimationFrame(raf);
      raf = 0;
    } else {
      wake();
    }
  }

  // ---- interaction --------------------------------------------------------

  let pointer = $state({ x: 0, y: 0 });
  let hovered = $state<{ branch: Branch; node: Node } | null>(null);
  let selected = $state<{ orb: Orb; branch: Branch; node: Node } | null>(null);
  // A whole game selected (orb click) — drives the game-level side panel,
  // mutually exclusive with `selected` (a single snapshot node).
  let selectedOrb = $state<Orb | null>(null);

  let dragging = $state(false); // panning the camera
  let draggedOrb: Orb | null = null; // a game orb being moved by the user
  let draggedNode: Node | null = null; // a single save node being moved
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
    const ns = clamp(scale * factor, 0.18, 6);
    tx = mx - (mx - tx) * (ns / scale);
    ty = my - (my - ty) * (ns / scale);
    scale = ns;
    requestDraw();
  }

  function onPointerDown(e: PointerEvent) {
    moved = false;
    startX = e.clientX;
    startY = e.clientY;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    // Pressing on a save node grabs *that node*; on a game orb grabs the orb;
    // on empty space pans the camera. Nodes are checked first because they sit
    // on top of (and near) their orb.
    const node = hitNode(px, py);
    if (node) {
      draggedNode = node.node;
      dragging = false;
      wake();
      return;
    }
    const orb = hitOrb(px, py);
    if (orb) {
      draggedOrb = orb;
      dragging = false;
      wake();
    } else {
      dragging = true;
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    pointer = { x: px, y: py };

    if (draggedNode) {
      const ddx = e.clientX - startX;
      const ddy = e.clientY - startY;
      if (Math.abs(ddx) > 3 || Math.abs(ddy) > 3) moved = true;
      // Pin the node straight to the cursor in world space; its chain springs
      // drag the rest of the branch's saves along behind it.
      const w = screenToWorld(px, py);
      draggedNode.x = w.x;
      draggedNode.y = w.y;
      draggedNode.vx = 0;
      draggedNode.vy = 0;
      wake();
      return;
    }

    if (draggedOrb) {
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) moved = true;
      // Move only the orb; its branches are NOT translated rigidly. The spring
      // chains in `step()` pull the nodes after it, so the rama trails and
      // swings a beat behind the orb — a living drag, not a rigid block. A tiny
      // fraction (0.25) is carried so far-out nodes don't snap to a hard stop
      // when you fling the orb, but most of the motion is the spring lag.
      const wdx = dx / scale;
      const wdy = dy / scale;
      draggedOrb.x += wdx;
      draggedOrb.y += wdy;
      // Frozen: no spring will catch the nodes up, so move them rigidly with
      // the orb. Live: carry a fraction and let the chain springs trail them.
      const carry = frozen ? 1 : 0.25;
      for (const b of draggedOrb.branches)
        for (const n of b.nodes) {
          n.x += wdx * carry;
          n.y += wdy * carry;
        }
      startX = e.clientX;
      startY = e.clientY;
      wake();
      return;
    }

    if (dragging) {
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      if (Math.abs(dx) > 3 || Math.abs(dy) > 3) moved = true;
      tx += dx;
      ty += dy;
      startX = e.clientX;
      startY = e.clientY;
      requestDraw();
      return;
    }
    const hit = hitNode(px, py);
    const next = hit ? { branch: hit.branch, node: hit.node } : null;
    // Only repaint when the hovered node actually changes — a mousemove over
    // empty space or the same node shouldn't wake the loop.
    if ((next?.node ?? null) !== (hovered?.node ?? null)) {
      hovered = next;
      requestDraw();
    } else {
      hovered = next;
    }
  }

  function onPointerUp(e: PointerEvent) {
    dragging = false;
    if (draggedNode) {
      draggedNode = null;
      wake(); // let the branch spring back into place
    }
    if (draggedOrb) {
      draggedOrb = null;
      wake(); // let neighbours settle around the new position
    }
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
      selectedOrb = null;
      requestDraw(); // the selected node is drawn larger
      return;
    }
    const orb = hitOrb(px, py);
    if (orb) {
      // Untracked games have nothing to inspect — jump to the Library so the
      // user can start tracking them.
      if (!orb.tracked) {
        push("/library");
        return;
      }
      // Open the game-level panel (same idea as the snapshot panel, but for
      // the whole partida set).
      selected = null;
      selectedOrb = orb;
      requestDraw();
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
      // Best-effort detection sweep: maps slug → Steam app id for cover art
      // and provides the detected-but-untracked games shown as dimmed orbs.
      let detectedGames: DetectedGame[] = [];
      try {
        const det = await api.cachedDetection();
        if (det) {
          detectedGames = det.games;
          for (const g of det.games) {
            if (g.steam_app_id != null) appIdBySlug.set(g.slug, g.steam_app_id);
          }
        }
      } catch {
        /* detection unavailable — orbs fall back to initials, no extra games */
      }

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
      orbs = build(saves, snapsBySave, detectedGames);
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
        requestDraw();
      });
      ro.observe(canvasEl);
      // pre-relax so the first paint is already tidy
      if (orbs.length > 0) {
        const settle = reduceMotion ? 220 : 40;
        for (let i = 0; i < settle; i++) step();
        frameAll();
      }
      // Kick the loop: with motion left to settle it runs until at rest, then
      // sleeps; with reduced motion it paints once and stops.
      requestDraw();
    });
    document.addEventListener("visibilitychange", onVisibility);
  });

  onDestroy(() => {
    running = false;
    cancelAnimationFrame(raf);
    ro?.disconnect();
    document.removeEventListener("visibilitychange", onVisibility);
  });

  function totalVersions(o: Orb): number {
    let t = 0;
    for (const b of o.branches) t += b.nodes.length;
    return t;
  }

  function lastBackup(o: Orb): string {
    let latest: string | null = null;
    for (const b of o.branches) {
      const t = b.save.last_backup_at;
      if (t && (!latest || t > latest)) latest = t;
    }
    return latest ? formatRelative(latest) : $_("map.never");
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
      <h1 class="font-display flex items-center gap-2.5 text-[22px] font-semibold tracking-[-0.02em] text-zinc-50">
        <MapIcon size={22} class="text-emerald-400" />
        {$_("map.title")}
      </h1>
      <p class="mt-1.5 text-xs text-zinc-500">{$_("map.subtitle")}</p>
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

    <!-- freeze / unfreeze physics so nodes can be grabbed without drifting -->
    <button
      type="button"
      onclick={toggleFrozen}
      class="pointer-events-auto absolute bottom-4 left-6 z-10 flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs backdrop-blur transition hover:brightness-125"
      style={frozen
        ? "border-color:rgba(245,158,11,0.4);background:rgba(245,158,11,0.1);color:#fcd34d"
        : "border-color:rgba(255,255,255,0.08);background:rgba(24,24,27,0.7);color:#d4d4d8"}
    >
      {#if frozen}
        <Play size={13} />{$_("map.resume")}
      {:else}
        <Pause size={13} />{$_("map.freeze")}
      {/if}
    </button>

    <!-- legend -->
    <div
      class="pointer-events-none absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 gap-3 text-[10px] text-zinc-500"
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
        <div class="flex items-center gap-3">
          <Cover
            appId={selected.orb.steamAppId}
            name={selected.orb.name}
            class="h-11 w-11 rounded-lg"
            initialClass="text-base"
          />
          <div>
            <p class="text-xs uppercase tracking-wide text-emerald-400">{selected.orb.name}</p>
            <h2 class="mt-0.5 text-lg font-semibold text-zinc-100">{selected.branch.label}</h2>
          </div>
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
          onclick={() => push(`/history/${selected!.branch.save.save_id}`)}
          class="flex w-full items-center justify-center gap-2 rounded-lg border border-white/[0.1] px-4 py-2 text-sm text-zinc-300 transition hover:border-white/[0.2] hover:text-zinc-100"
        >
          <HistoryIcon size={15} />
          {$_("map.open_history")}
        </button>
      </div>
    </aside>
  {/if}

  <!-- game-level panel (orb click) -->
  {#if selectedOrb}
    <aside
      class="absolute right-0 top-0 z-30 flex h-full w-72 flex-col border-l border-white/[0.08] bg-zinc-900/95 p-5 backdrop-blur"
    >
      <div class="flex items-start justify-between">
        <div class="flex items-center gap-3">
          <Cover
            appId={selectedOrb.steamAppId}
            name={selectedOrb.name}
            class="h-12 w-12 rounded-lg"
            initialClass="text-lg"
          />
          <div>
            <p class="text-xs uppercase tracking-wide text-emerald-400">{$_("map.panel_game")}</p>
            <h2 class="mt-0.5 text-lg font-semibold text-zinc-100">{selectedOrb.name}</h2>
          </div>
        </div>
        <button
          type="button"
          onclick={() => (selectedOrb = null)}
          class="rounded p-1 text-zinc-500 transition hover:bg-zinc-800 hover:text-zinc-200"
          aria-label="Close"
        >
          <X size={16} />
        </button>
      </div>

      <dl class="mt-5 space-y-2.5 text-sm">
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_partidas")}</dt>
          <dd class="font-medium text-zinc-200">{selectedOrb.branches.length}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_versions")}</dt>
          <dd class="font-medium text-zinc-200">{totalVersions(selectedOrb)}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_size")}</dt>
          <dd class="font-medium text-zinc-200">{formatBytes(selectedOrb.totalBytes)}</dd>
        </div>
        <div class="flex justify-between">
          <dt class="text-zinc-500">{$_("map.panel_last_backup")}</dt>
          <dd class="font-medium text-zinc-200">{lastBackup(selectedOrb)}</dd>
        </div>
      </dl>

      {#if selectedOrb.branches.length > 0}
        <div class="mt-5 flex-1 space-y-1.5 overflow-y-auto">
          {#each selectedOrb.branches as b (b.save.save_id)}
            <button
              type="button"
              onclick={() => push(`/history/${b.save.save_id}`)}
              class="flex w-full items-center justify-between gap-2 rounded-lg border border-white/[0.06] px-3 py-2 text-left text-sm text-zinc-300 transition hover:border-white/[0.16] hover:text-zinc-100"
            >
              <span class="flex min-w-0 items-center gap-2">
                <span
                  class="h-2 w-2 shrink-0 rounded-full"
                  style={`background:${STATUS_COLOR[b.status]}`}
                ></span>
                <span class="truncate">{b.label}</span>
              </span>
              <span class="shrink-0 text-xs text-zinc-500"
                >{$_("map.panel_versions_short", { values: { count: b.nodes.length } })}</span
              >
            </button>
          {/each}
        </div>
      {/if}

      <div class="mt-auto space-y-2 pt-5">
        <button
          type="button"
          onclick={() => push("/library")}
          class="flex w-full items-center justify-center gap-2 rounded-lg border border-white/[0.1] px-4 py-2 text-sm text-zinc-300 transition hover:border-white/[0.2] hover:text-zinc-100"
        >
          <MapIcon size={15} />
          {$_("map.view_library")}
        </button>
      </div>
    </aside>
  {/if}
</div>
