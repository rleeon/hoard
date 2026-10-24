<script lang="ts" module>
  /**
   * A cut stone, drawn: what the gem pickers show instead of a coloured square.
   *
   * Each gem gets the cut that stone is known for: the emerald its stepped
   * octagon, the ruby a round brilliant, the sapphire an oval, the amethyst a
   * trillion, the aquamarine a pear, the citrine a cushion. Amber is resin, not
   * crystal, so it is a polished cabochon with no facets.
   *
   * The facets are flat fills, lighter where they face the top-left and darker
   * where they turn away, all from the same ramp the accent uses, so a stone
   * shows exactly the colour the app will wear.
   */
  import { ACCENT_STOPS, DEFAULT_GEM_HUE } from "../stores/theme";

  export type GemCut = "step" | "round" | "oval" | "cushion" | "pear" | "trillion" | "cabochon";

  const CUT_BY_ID: Record<string, GemCut> = {
    emerald: "step",
    citrine: "cushion",
    amber: "cabochon",
    ruby: "round",
    amethyst: "trillion",
    sapphire: "oval",
    aquamarine: "pear",
  };

  /** A custom hue, or the app's when it is not one of the named gems, is a
   *  round brilliant: the stone anyone pictures first. */
  export function cutFor(id: string | null | undefined): GemCut {
    return (id && CUT_BY_ID[id]) || "round";
  }

  type Pt = [number, number];
  type Facet = { d: string; tone: number };
  type Drawing = { facets: Facet[]; table: string; tableTone: number; sheen: string; glint: Pt };

  const TAU = Math.PI * 2;
  /** Where the light comes from: up and to the left. */
  const LIGHT = Math.atan2(-1, -0.8);

  const path = (pts: Pt[]) => "M" + pts.map(([x, y]) => `${x.toFixed(2)} ${y.toFixed(2)}`).join("L") + "Z";
  const toward = (c: Pt, p: Pt, k: number): Pt => [c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k];
  const centroid = (pts: Pt[]): Pt => [
    pts.reduce((s, p) => s + p[0], 0) / pts.length,
    pts.reduce((s, p) => s + p[1], 0) / pts.length,
  ];
  /** 0 is the lightest tone (the ramp's 200), 6 the darkest (its 800). */
  function shade(c: Pt, p: Pt, bias = 0): number {
    const facing = Math.cos(Math.atan2(p[1] - c[1], p[0] - c[0]) - LIGHT);
    return Math.max(0, Math.min(6, Math.round(((1 - facing) / 2) * 6) + bias));
  }

  /** Sixteen points round an outline, starting at the top and going clockwise. */
  function outline(at: (t: number) => Pt): Pt[] {
    return Array.from({ length: 16 }, (_, k) => at(-Math.PI / 2 + (k * TAU) / 16));
  }

  /** Kites from the girdle up to each corner of an eight-sided table, star
   *  facets in between: the crown of a brilliant, seen from above. */
  function brilliant(o: Pt[], c: Pt): Drawing {
    const table = Array.from({ length: 8 }, (_, j) => toward(c, o[2 * j], 0.5));
    const facets: Facet[] = [];
    for (let j = 0; j < 8; j++) {
      const kite = [o[(2 * j + 15) % 16], o[2 * j], o[2 * j + 1], table[j]];
      const star = [table[j], o[2 * j + 1], table[(j + 1) % 8]];
      facets.push({ d: path(kite), tone: shade(c, centroid(kite), j % 2) });
      facets.push({ d: path(star), tone: shade(c, centroid(star), -1) });
    }
    return finish(table, c, facets);
  }

  /** Two steps down from the table to the girdle, the emerald's own cut. */
  function stepped(o: Pt[], c: Pt): Drawing {
    const mid = o.map((p) => toward(c, p, 0.76));
    const table = o.map((p) => toward(c, p, 0.5));
    const facets: Facet[] = [];
    for (let i = 0; i < o.length; i++) {
      const n = (i + 1) % o.length;
      const outer = [o[i], o[n], mid[n], mid[i]];
      const inner = [mid[i], mid[n], table[n], table[i]];
      facets.push({ d: path(outer), tone: shade(c, centroid(outer)) });
      facets.push({ d: path(inner), tone: shade(c, centroid(inner), -1) });
    }
    return finish(table, c, facets);
  }

  /** The table, a lighter wedge across its upper left, and the glint. */
  function finish(table: Pt[], c: Pt, facets: Facet[]): Drawing {
    const lit = table.filter(([x, y]) => x <= c[0] + 0.01 && y <= c[1] + 0.01);
    const corner = table.reduce((a, p) => (p[0] + p[1] < a[0] + a[1] ? p : a));
    return {
      facets,
      table: path(table),
      tableTone: 2,
      sheen: path([c, ...lit]),
      glint: toward(c, corner, 0.72),
    };
  }

  function draw(cut: GemCut): Drawing | null {
    switch (cut) {
      case "round":
        return brilliant(outline((t) => [16 + 13.5 * Math.cos(t), 16 + 13.5 * Math.sin(t)]), [16, 16]);
      case "oval":
        return brilliant(outline((t) => [16 + 10.5 * Math.cos(t), 16 + 14 * Math.sin(t)]), [16, 16]);
      case "cushion": {
        const sq = (v: number) => Math.sign(v) * Math.abs(v) ** (2 / 3.2);
        return brilliant(outline((t) => [16 + 13 * sq(Math.cos(t)), 16 + 13 * sq(Math.sin(t))]), [16, 16]);
      }
      case "pear": {
        // A drop with its point up; `t` runs from the point all the way round.
        const pts = Array.from({ length: 16 }, (_, k): Pt => {
          const t = (k * TAU) / 16;
          return [16 + Math.sin(t) * Math.sin(t / 2) * 15, 17 - Math.cos(t) * 14];
        });
        return brilliant(pts, [16, 19.5]);
      }
      case "trillion": {
        // A triangle point down with its sides let out a little.
        const c: Pt = [16, 14];
        return brilliant(
          outline((t) => {
            const m = ((((t + Math.PI / 2 - Math.PI / 3) % ((2 * Math.PI) / 3)) + (2 * Math.PI) / 3) %
              ((2 * Math.PI) / 3)) - Math.PI / 3;
            const r = 0.72 * (7.8 / Math.cos(m)) + 0.28 * 13;
            return [c[0] + r * Math.cos(t), c[1] + r * Math.sin(t)];
          }),
          c,
        );
      }
      case "step":
        return stepped(
          [
            [11, 3],
            [21, 3],
            [26, 8],
            [26, 24],
            [21, 29],
            [11, 29],
            [6, 24],
            [6, 8],
          ],
          [16, 16],
        );
      case "cabochon":
        return null;
    }
  }

  const DRAWINGS = new Map<GemCut, Drawing | null>();
  function drawing(cut: GemCut): Drawing | null {
    if (!DRAWINGS.has(cut)) DRAWINGS.set(cut, draw(cut));
    return DRAWINGS.get(cut)!;
  }
</script>

<script lang="ts">
  type Props = { cut: GemCut; hue: number | null; size?: number; class?: string };
  let { cut, hue, size = 28, class: klass = "" }: Props = $props();

  const h = $derived(hue ?? DEFAULT_GEM_HUE);
  /** The ramp's 200 to 800, then its 950 for the seams. */
  const tones = $derived(
    ACCENT_STOPS.slice(2, 9).map(([, l, c]) => `oklch(${l} ${c} ${h})`),
  );
  const seam = $derived(`oklch(0.27 0.06 ${h})`);
  const g = $derived(drawing(cut));
</script>

<svg width={size} height={size} viewBox="0 0 32 32" class={klass} aria-hidden="true" data-anim="pop">
  {#if g}
    <g stroke={seam} stroke-width="0.45" stroke-linejoin="round" stroke-opacity="0.55">
      {#each g.facets as f, i (i)}
        <path d={f.d} fill={tones[f.tone]} />
      {/each}
      <path d={g.table} fill={tones[g.tableTone]} />
    </g>
    <path d={g.sheen} fill={tones[1]} opacity="0.55" />
    <path
      d="M{g.glint[0]} {g.glint[1] - 2.6}Q{g.glint[0]} {g.glint[1]} {g.glint[0] + 2.6} {g.glint[1]}Q{g.glint[0]} {g.glint[1]} {g.glint[0]} {g.glint[1] + 2.6}Q{g.glint[0]} {g.glint[1]} {g.glint[0] - 2.6} {g.glint[1]}Q{g.glint[0]} {g.glint[1]} {g.glint[0]} {g.glint[1] - 2.6}Z"
      fill="white"
      opacity="0.9"
    />
  {:else}
    <!-- Cabochon: a polished dome, light pooling at the top left and a warm
         rim along the bottom where it comes back out. -->
    <ellipse cx="16" cy="16.5" rx="11.5" ry="13" fill={tones[4]} />
    <ellipse cx="16" cy="19" rx="9.5" ry="9.5" fill={tones[3]} />
    <ellipse cx="14.6" cy="14.2" rx="8" ry="9" fill={tones[2]} />
    <path d="M7.6 21.6Q16 30.2 24.4 21.6Q16 27.6 7.6 21.6Z" fill={tones[1]} opacity="0.7" />
    <ellipse cx="11.6" cy="9.6" rx="3.6" ry="2.1" transform="rotate(-32 11.6 9.6)" fill={tones[0]} />
    <circle cx="10.2" cy="8.8" r="1.1" fill="white" opacity="0.9" />
  {/if}
</svg>
