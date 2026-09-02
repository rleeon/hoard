<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';

  // ===========================================================================
  // SYNC DIAGRAM, the animated diagram of the "Sync" section on the home page.
  // ---------------------------------------------------------------------------
  // THE USER EXPLICITLY REQUESTED THIS DESIGN AND ANIMATION. Do not "simplify"
  // or "fix" it without asking them first. What they asked for, in their words:
  //
  //   "La bola tiene que salir de el PC1 a el server ya sea cloud o self-host,
  //    luego salen 2 volas al steamdeck y pc3 y ahi PARA, nada mas, luego la
  //    misma animacion pero que vaya a el server self-host."
  //
  //   "en la animacion 1 que el cloud este en verde, en la animacion 2 que el
  //    self-host este en verde, y siempre el contrario que este como apagado,
  //    es mas que no tenga ni lineas a los dispositivos."
  //
  // SCENE (always the same five nodes):
  //   - Two hubs on top: hoard-Cloud (badge "login") and hoard-server (badge
  //     "self-host"), left and right.
  //   - Three devices below: PC1, SteamDeck, PC3.
  //
  // DESIGN (the "active hub" concept):
  //   - The two hubs alternate: while one hub is ACTIVE it is fully lit (green
  //     accent) and ONLY its three lines to the devices are drawn. The other
  //     hub stays "off": dimmed at CHIP_IDLE opacity and with NO lines at all.
  //   - The switch happens with a CROSS-second crossfade that hugs the phase
  //     boundary: the active hub's lines fade out over the last CROSS seconds
  //     of its phase, the other hub's lines fade in over the first CROSS
  //     seconds of its own phase. The fade is continuous across the whole
  //     cycle (it never resets), so the scene never blinks.
  //
  // ANIMATION (one cycle = cloud phase + server phase, loops forever):
  //   - Each phase: ball 1 departs from PC1 and rides the drawn curve up to
  //     the active hub. When it arrives, two balls split off along their own
  //     drawn curves to SteamDeck and PC3 and STOP there for a beat. Then the
  //     phase ends and the same sequence plays for the other hub.
  //   - Every ball travels along the EXACT curve that is painted as the SVG
  //     path: the same curve data drives both the <path> elements and the
  //     ball positions (64-point sampling), so the balls always pass precisely
  //     on the drawn line, never near it.
  //
  // TECHNICAL NOTES (why it is built this way):
  //   - EVERYTHING is driven by a single requestAnimationFrame clock: ball
  //     positions, ball fade in/out, and the hub line/chip crossfade. There
  //     are NO CSS transitions and NO SMIL animations: on the user's machine
  //     (Windows / Brave) neither CSS transforms on SVG nor SMIL animated, so
  //     this is the only approach that is guaranteed to move everywhere.
  //   - There is deliberately NO prefers-reduced-motion gating: the user wants
  //     the loop to always run.
  //   - The balls are plain absolutely-positioned <div>s layered over the SVG
  //     (left/top in %), not SVG elements, for the same compatibility reason.
  // ===========================================================================

  type Pt = { x: number; y: number };
  type Curve = { a: Pt; c1: Pt; c2: Pt; b: Pt };

  let { className = '' }: { className?: string } = $props();

  const T = (x: number, y: number): Pt => ({ x, y });

  // A "bow" curve: a cubic bezier that leaves the hub vertically, arcs down
  // through a shared elbow and lands horizontally at the device. The same
  // object is used to draw the SVG path AND to move the balls along it.
  const bow = (hx: number, dx: number): Curve => ({
    a: T(hx, 52),
    c1: T(hx, 96),
    c2: T(dx, 96),
    b: T(dx, 140)
  });

  // cloud->PC1, cloud->SteamDeck, cloud->PC3, server->PC1, server->SteamDeck, server->PC3
  const conns: Curve[] = [bow(75, 45), bow(75, 150), bow(75, 255), bow(225, 45), bow(225, 150), bow(225, 255)];
  const paths = conns.map((c) => `M ${c.a.x} ${c.a.y} C ${c.c1.x} ${c.c1.y} ${c.c2.x} ${c.c2.y} ${c.b.x} ${c.b.y}`);

  const rev = (c: Curve): Curve => ({ a: c.b, c1: c.c2, c2: c.c1, b: c.a });

  const sample = (c: Curve, n: number): Pt[] => {
    const pts: Pt[] = [];
    for (let i = 0; i <= n; i++) {
      const t = i / n;
      const mt = 1 - t;
      pts.push(
        T(
          mt * mt * mt * c.a.x + 3 * mt * mt * t * c.c1.x + 3 * mt * t * t * c.c2.x + t * t * t * c.b.x,
          mt * mt * mt * c.a.y + 3 * mt * mt * t * c.c1.y + 3 * mt * t * t * c.c2.y + t * t * t * c.b.y
        )
      );
    }
    return pts;
  };

  // A walker maps a DISTANCE (in viewBox units) along a sampled curve to a
  // point on it. It is what makes the balls ride the painted line exactly.
  type Walker = { at: (d: number) => Pt; total: number };
  const makeWalker = (pts: Pt[]): Walker => {
    const cum: number[] = [0];
    for (let i = 0; i < pts.length - 1; i++) {
      cum.push(cum[i] + Math.hypot(pts[i + 1].x - pts[i].x, pts[i + 1].y - pts[i].y));
    }
    const total = cum.at(-1) ?? 1;
    return {
      total,
      at: (d: number) => {
        const t = Math.min(Math.max(d, 0), total);
        let i = 0;
        while (i < cum.length - 2 && cum[i + 1] < t) i++;
        const segLen = cum[i + 1] - cum[i];
        const k = segLen > 0 ? (t - cum[i]) / segLen : 0;
        return T(pts[i].x + (pts[i + 1].x - pts[i].x) * k, pts[i].y + (pts[i + 1].y - pts[i].y) * k);
      }
    };
  };

  // Progress along a walker by fraction of its total length, so the ball
  // covers the WHOLE painted line and not a single pixel of it.
  const ride = (w: Walker, frac: number): Pt => w.at(frac * w.total);

  const N = 64;
  const CLOUD = {
    toHub: makeWalker(sample(rev(conns[0]), N)),
    toDeck: makeWalker(sample(conns[1], N)),
    toPc3: makeWalker(sample(conns[2], N)),
    hubEnd: T(75, 52),
    deckEnd: T(150, 140),
    pc3End: T(255, 140)
  };
  const SERVER = {
    toHub: makeWalker(sample(rev(conns[3]), N)),
    toDeck: makeWalker(sample(conns[4], N)),
    toPc3: makeWalker(sample(conns[5], N)),
    hubEnd: T(225, 52),
    deckEnd: T(150, 140),
    pc3End: T(255, 140)
  };

  // One phase: ball 1 rides PC1 -> hub (S1 seconds), two balls split to
  // SteamDeck and PC3 (S2 seconds), then everything parks for a beat. Two
  // phases per cycle: the cloud, then the self-hosted server. Forever.
  const PHASE = 4.8;
  const S1 = 1.6;
  const S2 = 2.2;
  const f1 = S1 / PHASE;
  const f2 = (S1 + S2) / PHASE;

  // Every appearance and disappearance fades over FADE seconds, driven by the
  // same rAF clock as the motion, so a ball never pops in or out of a spot:
  // no teleporting, and no CSS transitions to depend on.
  const FADE = 0.25;
  const fadeK = FADE / PHASE;

  // The active hub's lines and chip stay on through its whole phase and
  // crossfade with the other hub over CROSS seconds around each boundary, on
  // the same rAF clock as the balls. The idle hub never fully disappears: it
  // stays dimmed at CHIP_IDLE, "off" but present.
  const CROSS = 0.25;
  const CHIP_IDLE = 0.35;

  let dot1: HTMLDivElement;
  let dot2: HTMLDivElement;
  let dot3: HTMLDivElement;
  let cloudLines: SVGGElement;
  let serverLines: SVGGElement;
  let cloudChip: SVGGElement;
  let serverChip: SVGGElement;

  const place = (el: HTMLDivElement | undefined, p: Pt | undefined, opacity: number) => {
    if (!el) return;
    el.style.opacity = String(opacity);
    if (!p) return;
    el.style.left = `${((p.x / 300) * 100).toFixed(3)}%`;
    el.style.top = `${((p.y / 210) * 100).toFixed(3)}%`;
  };

  // Ball 1 is on screen from the phase start (riding PC1 -> hub) until it
  // parks at the hub at f2. Balls 2 and 3 appear at the hub when the split
  // starts (f1) and stay parked at SteamDeck / PC3 until the phase ends.
  const envelope = (k: number, a: number, b: number): number => {
    if (k < a || k > b) return 0;
    const fadeIn = Math.min(1, (k - a) / fadeK);
    const fadeOut = Math.min(1, (b - k) / fadeK);
    return Math.min(fadeIn, fadeOut);
  };

  // 1 while the cloud phase is on, 0 while it is off. The crossfade hugs the
  // boundaries: cloud fades out over the last CROSS seconds of its phase and
  // fades in over the first CROSS seconds of the next one. The server's fade
  // is the same function shifted half a cycle, so the two hubs never overlap
  // and never reset, no blink, and the idle hub keeps zero lines.
  const cloudFadeOf = (tt: number): number => {
    const c = tt % (PHASE * 2);
    if (c < CROSS) return c / CROSS;
    if (c < PHASE - CROSS) return 1;
    if (c < PHASE) return (PHASE - c) / CROSS;
    return 0;
  };

  onMount(() => {
    let raf = 0;
    const tick = (now: number) => {
      const sec = now / 1000;
      const cycle = sec % (PHASE * 2);
      const cloudOn = cycle < PHASE;
      const hub = cloudOn ? CLOUD : SERVER;
      const t = cycle % PHASE;
      const k = t / PHASE;

      // Hub lines and chips: only the active hub is lit and shows lines; the
      // other hub is dimmed with no lines, crossfading at the phase boundary.
      const cf = cloudFadeOf(cycle);
      const sf = cloudFadeOf((cycle + PHASE) % (PHASE * 2));
      cloudLines.style.opacity = String(cf);
      serverLines.style.opacity = String(sf);
      cloudChip.style.opacity = String(CHIP_IDLE + (1 - CHIP_IDLE) * cf);
      serverChip.style.opacity = String(CHIP_IDLE + (1 - CHIP_IDLE) * sf);

      // The three balls: ball 1 rides PC1 -> hub, then balls 2 and 3 split
      // to SteamDeck and PC3 and park there until the phase ends.
      if (k < f1) {
        place(dot1, ride(hub.toHub, k / f1), envelope(k, 0, f2));
        place(dot2, hub.toDeck.at(0), envelope(k, f1, 1));
        place(dot3, hub.toPc3.at(0), envelope(k, f1, 1));
      } else if (k < f2) {
        const kk = (k - f1) / (f2 - f1);
        place(dot1, hub.hubEnd, envelope(k, 0, f2));
        place(dot2, ride(hub.toDeck, kk), envelope(k, f1, 1));
        place(dot3, ride(hub.toPc3, kk), envelope(k, f1, 1));
      } else {
        place(dot1, hub.hubEnd, envelope(k, 0, f2));
        place(dot2, hub.deckEnd, envelope(k, f1, 1));
        place(dot3, hub.pc3End, envelope(k, f1, 1));
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class={`relative mx-auto block w-full ${className}`}>
  <!-- The SVG paints the scene. The two line groups and the two hub-chip
       groups have their opacity driven from the tick (active hub lit with
       its lines, idle hub dimmed with no lines). -->
  <svg viewBox="0 0 300 210" role="img" aria-label={$_('sync.diagram_aria')} class="block w-full">
    <g bind:this={cloudLines}>
      <path d={paths[0]} class="conn" />
      <path d={paths[1]} class="conn" />
      <path d={paths[2]} class="conn" />
    </g>
    <g bind:this={serverLines}>
      <path d={paths[3]} class="conn" />
      <path d={paths[4]} class="conn" />
      <path d={paths[5]} class="conn" />
    </g>

    <g bind:this={cloudChip} aria-hidden="true">
      <rect class="hub-chip" x="27" y="18" width="96" height="34" rx="8" />
      <text class="l-chip" x="75" y="31">hoard-Cloud</text>
      <text class="l-chip-sub" x="75" y="42">login</text>
    </g>
    <g bind:this={serverChip} aria-hidden="true">
      <rect class="hub-chip" x="177" y="18" width="96" height="34" rx="8" />
      <text class="l-chip" x="225" y="31">hoard-server</text>
      <text class="l-chip-sub" x="225" y="42">self-host</text>
    </g>

    <g aria-hidden="true">
      <rect class="dev-chip" x="13" y="140" width="64" height="24" rx="6" />
      <rect class="dev-chip" x="118" y="140" width="64" height="24" rx="6" />
      <rect class="dev-chip" x="223" y="140" width="64" height="24" rx="6" />
      <text class="l-chip" x="45" y="157">PC1</text>
      <text class="l-chip" x="150" y="157">SteamDeck</text>
      <text class="l-chip" x="255" y="157">PC3</text>
    </g>
  </svg>

  <!-- The save is three plain HTML divs overlaid on the SVG and positioned in
       % of the wrapper (which exactly matches the SVG), so the rAF-driven
       left/top updates animate reliably in every browser. -->
  <div class="save-dot" bind:this={dot1} aria-hidden="true"></div>
  <div class="save-dot" bind:this={dot2} aria-hidden="true"></div>
  <div class="save-dot" bind:this={dot3} aria-hidden="true"></div>
</div>

<style>
  .save-dot {
    position: absolute;
    left: 15%;
    top: 67.62%;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 9999px;
    background: var(--color-accent);
    box-shadow: 0 0 8px color-mix(in oklab, var(--color-accent) 70%, transparent);
    will-change: left, top;
    opacity: 0;
  }

  .conn {
    fill: none;
    stroke: var(--color-line-strong);
    stroke-width: 1.5;
    stroke-linejoin: round;
  }

  .hub-chip {
    fill: var(--color-pine-elev);
    stroke: var(--color-accent);
    stroke-opacity: 0.55;
  }
  .dev-chip {
    fill: var(--color-surface);
    stroke: var(--color-line-strong);
  }
  .l-chip {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    fill: var(--color-ink);
    text-anchor: middle;
  }
  .l-chip-sub {
    font-family: var(--font-mono);
    font-size: 8px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    fill: var(--color-accent);
    text-anchor: middle;
  }
</style>
