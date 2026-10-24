<script lang="ts">
  /**
   * Halloween's garland: a string of small pumpkins hanging from the green
   * title bar, scalloped between pins along its bottom edge. Every pumpkin
   * sways a little on its stem, and a few of them light up and go dark again,
   * slowly, each on its own clock.
   *
   * Nothing in it repeats: the loops are of different lengths and sag to one
   * side or the other, some hold two pumpkins and some none, and each pumpkin
   * has its own size, tilt, orange, stem and face. Evenly spaced identical
   * pumpkins read as a pattern, not as something someone hung up. It is rolled
   * afresh every time it goes up (the app starting, the mode switched back on)
   * and laid out wider than any screen, so a wider window only shows more of it
   * instead of shuffling it.
   *
   * It hangs over the page and takes no clicks. Over the menu the drop stays
   * short enough to clear the first item's label; past it some pumpkins hang
   * lower, on a longer thread.
   *
   * The motion is Web Animations for the same reason as the spider's: under
   * reduced motion `app.css` flattens CSS animations to nothing, and that is on
   * for anyone with Windows' animation effects off.
   */
  import { onMount } from "svelte";

  type Face = "smile" | "o" | "grin" | "none";
  type Pumpkin = {
    x: number;
    y: number;
    drop: number;
    scale: number;
    squat: number;
    tilt: number;
    hue: number;
    light: number;
    stem: number;
    face: Face;
    lit: boolean;
  };
  type Pin = { x: number; leaf: number };

  const STRING = "oklch(43.2% 0.095 166.913)";
  /** Past this the menu is behind and a pumpkin can hang lower. */
  const MENU_EDGE = 260;

  /** The whole garland, wider than any screen. */
  const GARLAND = (() => {
    const r = Math.random;
    const between = (a: number, b: number) => a + r() * (b - a);
    const pins: Pin[] = [];
    const loops: string[] = [];
    const pumpkins: Pumpkin[] = [];
    let x = -between(10, 60);
    while (x < 4200) {
      const span = between(90, 180);
      const sag = between(3, 7);
      const cx = x + span * between(0.35, 0.65);
      loops.push(`M${x.toFixed(1)} 0Q${cx.toFixed(1)} ${(sag * 2).toFixed(1)} ${(x + span).toFixed(1)} 0`);
      pins.push({ x, leaf: r() < 0.45 ? (r() < 0.5 ? -1 : 1) : 0 });
      const roll = r();
      const ts = roll < 0.12 ? [] : roll < 0.8 ? [between(0.32, 0.68)] : [between(0.22, 0.4), between(0.6, 0.8)];
      for (const t of ts) {
        // On the curve itself: a quadratic from (x, 0) through (cx, 2 sag).
        const px = (1 - t) ** 2 * x + 2 * (1 - t) * t * cx + t ** 2 * (x + span);
        const py = 2 * (1 - t) * t * sag * 2;
        const long = px > MENU_EDGE && r() < 0.3;
        const faceRoll = r();
        pumpkins.push({
          x: px,
          y: py,
          drop: long ? between(4, 9) : between(0, 1.5),
          scale: between(0.8, 1.15),
          squat: between(0.9, 1.2),
          tilt: between(-14, 14),
          hue: between(40, 60),
          light: between(0.64, 0.74),
          stem: r() < 0.5 ? -1 : 1,
          face: faceRoll < 0.45 ? "smile" : faceRoll < 0.65 ? "grin" : faceRoll < 0.8 ? "o" : "none",
          lit: r() < 0.3,
        });
      }
      x += span;
    }
    return { pins, loops, pumpkins };
  })();

  let width = $state(window.innerWidth);
  const loops = $derived(GARLAND.loops.filter((_, i) => GARLAND.pins[i].x <= width));
  const pins = $derived(GARLAND.pins.filter((p) => p.x <= width + 2));
  const pumpkins = $derived(GARLAND.pumpkins.filter((p) => p.x <= width + 12));

  const EYES = "M-2.8 7-1.7 5.4-.6 7ZM.6 7 1.7 5.4 2.8 7Z";
  const MOUTHS: Record<Exclude<Face, "none">, string> = {
    smile: "M-2.6 8.6Q0 10.8 2.6 8.6Q0 9.6-2.6 8.6Z",
    grin: "M-3 8.3Q0 11.4 3 8.3L1.6 8.9 1 9.6.2 8.9-.6 9.6-1.2 8.9Z",
    o: "M0 8.2a1 1.2 0 1 0 .01 0Z",
  };
  const face = (f: Face) => (f === "none" ? "" : EYES + MOUTHS[f]);

  let svg: SVGSVGElement | undefined = $state();

  onMount(() => {
    let running: Animation[] = [];
    const moving = new WeakSet<Element>();
    /** Sets off the pumpkins that are not moving yet. Restarting all of them
     *  on every resize made the whole garland jump while the window was being
     *  dragged wider. */
    function start() {
      running = running.filter((a) => (a.effect as KeyframeEffect | null)?.target?.isConnected);
      if (!svg) return;
      for (const el of svg.querySelectorAll<SVGGElement>("[data-sway]")) {
        if (moving.has(el)) continue;
        moving.add(el);
        const swing = 1.5 + Math.random() * 3;
        const a = el.animate(
          [{ transform: `rotate(${-swing}deg)` }, { transform: `rotate(${swing}deg)` }],
          {
            duration: 2200 + Math.random() * 2400,
            iterations: Infinity,
            direction: "alternate",
            easing: "ease-in-out",
          },
        );
        a.currentTime = Math.random() * 4000;
        running.push(a);
      }
      for (const el of svg.querySelectorAll<SVGGElement>("[data-glow]")) {
        if (moving.has(el)) continue;
        moving.add(el);
        const a = el.animate(
          [
            { opacity: 0, offset: 0 },
            { opacity: 1, offset: 0.25 },
            { opacity: 1, offset: 0.5 },
            { opacity: 0, offset: 0.75 },
            { opacity: 0, offset: 1 },
          ],
          { duration: 7000 + Math.random() * 5000, iterations: Infinity, easing: "ease-in-out" },
        );
        a.currentTime = Math.random() * 7000;
        running.push(a);
      }
    }
    // A wider window brings in pumpkins that start with no animation on them.
    const ro = new ResizeObserver(() => queueMicrotask(start));
    if (svg) ro.observe(svg);
    // The window spends its days in the tray, often while a game runs.
    const onVisibility = () => {
      for (const a of running) {
        if (document.hidden) a.pause();
        else a.play();
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      ro.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
      for (const a of running) a.cancel();
    };
  });
</script>

<svelte:window bind:innerWidth={width} />

<!-- Under the bar (400) and over everything in `#app`, modals included: it
     hangs off the bar, which stays above them too. -->
<svg
  bind:this={svg}
  class="pointer-events-none fixed inset-x-0 top-8 z-[399] h-4 w-full overflow-visible"
  aria-hidden="true"
>
  <defs>
    <radialGradient id="garland-glow">
      <stop offset="0" stop-color="oklch(0.85 0.17 75)" stop-opacity="0.75" />
      <stop offset="1" stop-color="oklch(0.85 0.17 75)" stop-opacity="0" />
    </radialGradient>
  </defs>
  <path d={loops.join("")} fill="none" stroke={STRING} stroke-width="1" />
  {#each pins as p, i (i)}
    <circle cx={p.x} cy="0.6" r="1.2" fill={STRING} />
    {#if p.leaf}
      <path
        d="M0 0C2 -1.4 5 -.6 6.4 1.6 4 2.6 1.4 2 0 0Z"
        transform="translate({p.x} 0.8) scale({p.leaf} 1)"
        fill="oklch(0.5 0.11 145)"
      />
    {/if}
  {/each}
  {#each pumpkins as p, i (i)}
    {@const body = `oklch(${p.light} 0.17 ${p.hue})`}
    {@const side = `oklch(${p.light - 0.1} 0.16 ${p.hue - 8})`}
    <g transform="translate({p.x.toFixed(1)} {p.y.toFixed(1)})">
      <g data-sway class="sway">
        <path d="M0 -0.4V{(p.drop + 1.6).toFixed(1)}" stroke={STRING} stroke-width="0.8" />
        <g transform="translate(0 {p.drop.toFixed(1)}) rotate({p.tilt.toFixed(1)}) scale({(p.scale * p.squat).toFixed(2)} {p.scale.toFixed(2)})">
          <path
            d="M0 1Q{p.stem * 0.3} 2.4 {p.stem * 0.9} 3.8"
            stroke="oklch(0.42 0.08 135)"
            stroke-width="1.5"
            stroke-linecap="round"
            fill="none"
          />
          {#if p.lit}
            <circle data-glow cx="0" cy="7.4" r="9" fill="url(#garland-glow)" opacity="0" />
          {/if}
          <ellipse cx="-2.7" cy="7.6" rx="3.2" ry="3.7" fill={side} />
          <ellipse cx="2.7" cy="7.6" rx="3.2" ry="3.7" fill={side} />
          <ellipse cx="0" cy="7.4" rx="3.3" ry="4" fill={body} />
          {#if p.face !== "none"}
            <path d={face(p.face)} fill="oklch(0.3 0.06 40)" />
            {#if p.lit}
              <path data-glow d={face(p.face)} fill="oklch(0.93 0.15 95)" opacity="0" />
            {/if}
          {/if}
        </g>
      </g>
    </g>
  {/each}
</svg>

<style>
  /* Swings from where the thread meets the string. */
  .sway {
    transform-box: view-box;
    transform-origin: 0 0;
  }
</style>
