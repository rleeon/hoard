<script lang="ts">
  /**
   * Halloween's spider: a corner web in the empty space between the rail's menu
   * and its footer, with a small round spider in a witch's hat living on it.
   *
   * It takes whatever height that space has (the parent gives it `flex-1`), and
   * when a short window leaves too little for the web it draws nothing rather
   * than a web cut in half.
   *
   * It walks in bursts along the threads and rests between them. Moving all
   * the time it read as a loading spinner, and in an app that syncs that says
   * "busy". Its eyes follow the pointer anywhere in the window; hovering it
   * makes it smile and stay put.
   *
   * Every move is a Web Animation, not a CSS transition: `app.css` flattens
   * transitions to 0.001ms under reduced motion, which is on for anyone with
   * Windows' animation effects off, and the spider jumped from crossing to
   * crossing with its legs flickering on the spot.
   *
   * Every timer stops while the window is hidden: the app spends its days in
   * the tray, often while a game is running.
   */
  import { onDestroy, onMount } from "svelte";
  import { spiderTune, spiderWalkNow } from "../stores/spider";

  // Size, speeds and timings live in `stores/spider.ts`, where the debug
  // panel can move them on the running window.
  const tune = $derived($spiderTune);
  const SIZE = $derived({ w: tune.size, h: Math.round((tune.size * 40) / 44) });
  const RINGS = [0.27, 0.52, 0.76, 1];
  const RADIALS = 6;

  // ---- drawing (viewBox 32 x 29, the body centred on 16, 15.6)

  /** Left legs, hip first. The right ones are their mirror image. */
  const LEFT_LEGS = [
    "M9.4 12.4Q6.2 7.4 4 10.4",
    "M8.4 14.8Q4.6 11.8 2.8 15",
    "M8.4 17.4Q4.4 17 3.2 20.6",
    "M9.6 19.6Q7 22.4 6.4 25",
  ];
  const mirrorX = (d: string) =>
    d.replace(/([MQ ])(-?[\d.]+) (-?[\d.]+)/g, (_, c, x, y) => `${c}${(32 - Number(x)).toFixed(2)} ${y}`);
  const point = (d: string, last: boolean) => {
    const nums = d.match(/-?[\d.]+/g)!.map(Number);
    return last ? { x: nums[nums.length - 2], y: nums[nums.length - 1] } : { x: nums[0], y: nums[1] };
  };
  /** Alternate legs step together (the A set against the B set), which is
   *  what reads as a gait rather than a shiver. */
  const LEGS = [
    ...LEFT_LEGS.map((d, i) => ({ d, side: "l", set: i % 2 === 0 ? "a" : "b" })),
    ...LEFT_LEGS.map((d, i) => ({ d: mirrorX(d), side: "r", set: i % 2 === 0 ? "b" : "a" })),
  ].map((leg) => ({ ...leg, hip: point(leg.d, false), foot: point(leg.d, true) }));

  const EYES = [
    { x: 12.4, y: 15.4 },
    { x: 19.6, y: 15.4 },
  ];
  /** Where the pupils sit with nobody to look at: a little down, at you. */
  const REST_LOOK = { x: 0.3, y: 0.7 };

  // ---- the web

  let height = $state(0);
  let width = $state(0);
  // Never wider than the rail either: the debug panel's slider goes past it.
  const R = $derived(Math.min(tune.webMax, height - 16, width - 2));

  const at = (r: number, k: number) => {
    const a = (k / (RADIALS - 1)) * (Math.PI / 2);
    return { x: r * Math.cos(a), y: r * Math.sin(a) };
  };

  const web = $derived.by(() => {
    if (R < 60) return null;
    // The two walls are the outer radials already: the window's edge and the
    // footer's line. Drawing them again only thickens a border.
    const radials = Array.from({ length: RADIALS - 2 }, (_, k) => at(R + 2, k + 1));
    // Each ring sags towards the corner between two radials, which is what
    // makes a cobweb out of what would otherwise be a protractor.
    const rings = RINGS.map((f) => {
      const r = f * R;
      let d = "";
      for (let k = 0; k < RADIALS - 1; k++) {
        const p = at(r, k);
        const q = at(r, k + 1);
        const c = at(r * 0.84, k + 0.5);
        d += `${k === 0 ? `M${p.x} ${R - p.y}` : ""} Q${c.x} ${R - c.y} ${q.x} ${R - q.y}`;
      }
      return d;
    });
    return { radials, rings };
  });

  /** Where the spider can stand: every crossing of a ring and a radial far
   *  enough from both walls, and from the rail's far edge, that its legs stay
   *  inside. */
  const nodes = $derived.by(() => {
    if (!web) return [];
    const out: { ring: number; k: number; x: number; y: number }[] = [];
    for (let ring = 1; ring < RINGS.length; ring++) {
      for (let k = 1; k < RADIALS - 1; k++) {
        const p = at(RINGS[ring] * R, k);
        if (p.x >= SIZE.w / 2 && p.y >= SIZE.h / 2 && p.x + SIZE.w / 2 <= width) {
          out.push({ ring, k, ...p });
        }
      }
    }
    return out;
  });

  // ---- state

  let here = $state(0);
  let prev = -1;
  let placed = false;
  let walking = false;
  let happy = $state(false);

  const spot = $derived(nodes[here] ?? nodes[0]);

  let mover = $state<HTMLDivElement>();
  let leaner = $state<HTMLDivElement>();
  let art = $state<SVGSVGElement>();
  let body = $state<SVGGElement>();
  let hat = $state<SVGGElement>();
  let eyes = $state<SVGGElement>();
  let pupils = $state<SVGGElement>();

  let walkTimer: ReturnType<typeof setTimeout> | undefined;
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  let gait: Animation[] = [];
  let lean = 0;
  let gone = false;

  const rand = (a: number, b: number) => a + Math.random() * (b - a);
  const placeAt = (p: { x: number; y: number }) =>
    `translate(${p.x - SIZE.w / 2}px, ${-(p.y - SIZE.h / 2)}px)`;
  const legs = () => (art ? [...art.querySelectorAll<SVGGElement>("[data-leg]")] : []);

  /** Runs `fn` now, or on the first `visibilitychange` that shows the window. */
  function whenVisible(fn: () => void) {
    if (gone) return;
    if (!document.hidden) return fn();
    const wake = () => {
      if (document.hidden) return;
      document.removeEventListener("visibilitychange", wake);
      if (!gone) fn();
    };
    document.addEventListener("visibilitychange", wake);
  }

  function neighbours(i: number): number[] {
    const n = nodes[i];
    if (!n) return [];
    return nodes
      .map((m, j) => ({ m, j }))
      .filter(
        ({ m }) =>
          (m.ring === n.ring && Math.abs(m.k - n.k) === 1) ||
          (m.k === n.k && Math.abs(m.ring - n.ring) === 1),
      )
      .map(({ j }) => j);
  }

  // ---- little moves

  function setLean(deg: number) {
    if (!leaner || deg === lean) return;
    const from = `rotate(${lean}deg)`;
    const to = `rotate(${deg}deg)`;
    leaner.style.transform = to;
    leaner.animate([{ transform: from }, { transform: to }], { duration: 220, easing: "ease-out" });
    lean = deg;
  }

  /** A leg's swing, mirrored on the right so both sides lift together. */
  function swing(el: SVGGElement, deg: number, opts: KeyframeAnimationOptions) {
    const s = el.dataset.side === "l" ? 1 : -1;
    const a = (el.dataset.leg === "a" ? -deg : deg) * s;
    return el.animate([{ transform: `rotate(${a}deg)` }, { transform: `rotate(${-a}deg)` }], opts);
  }

  function startGait() {
    if (gait.length > 0) return;
    const beat = {
      duration: tune.beat,
      iterations: Infinity,
      direction: "alternate" as const,
      easing: "ease-in-out",
    };
    gait = legs().map((el) => swing(el, tune.legSwing, beat));
    // Squash and stretch on the same beat: the bounce is most of the charm.
    if (body) {
      gait.push(
        body.animate(
          [{ transform: "scale(1.035, 0.965)" }, { transform: "scale(0.98, 1.03) translateY(-0.4px)" }],
          beat,
        ),
      );
    }
  }

  function stopGait() {
    for (const a of gait) a.cancel();
    gait = [];
  }

  function wobbleHat() {
    hat?.animate(
      [
        { transform: "rotate(0deg)" },
        { transform: "rotate(-9deg)", offset: 0.25 },
        { transform: "rotate(6deg)", offset: 0.55 },
        { transform: "rotate(-2deg)", offset: 0.8 },
        { transform: "rotate(0deg)" },
      ],
      { duration: 700, easing: "ease-out" },
    );
  }

  /** The landing after a burst: a squash that settles, and the hat catching up. */
  function settle() {
    body?.animate(
      [
        { transform: "scale(1)" },
        { transform: "scale(1.08, 0.9)", offset: 0.3 },
        { transform: "scale(0.97, 1.04)", offset: 0.65 },
        { transform: "scale(1)" },
      ],
      { duration: 380, easing: "ease-out" },
    );
    wobbleHat();
  }

  /** Before setting off: a little crouch, so the start reads as a decision. */
  function crouch(then: () => void) {
    body?.animate(
      [{ transform: "scale(1)" }, { transform: "scale(1.05, 0.9)" }, { transform: "scale(1)" }],
      { duration: 160, easing: "ease-in-out" },
    );
    walkTimer = setTimeout(then, 130);
  }

  function blink() {
    const shut = [{ transform: "scaleY(1)" }, { transform: "scaleY(0.1)" }, { transform: "scaleY(1)" }];
    eyes?.animate(shut, { duration: 160 });
    if (Math.random() < 0.3) setTimeout(() => eyes?.animate(shut, { duration: 160 }), 230);
  }

  function tapFoot() {
    const all = legs();
    const leg = all[Math.floor(Math.random() * all.length)];
    if (leg) swing(leg, 12, { duration: 110, iterations: 4, direction: "alternate", easing: "ease-in-out" });
  }

  function hello() {
    happy = true;
    art?.animate(
      [
        { transform: "none" },
        { transform: "translateY(-3px) scale(1.07)", offset: 0.4 },
        { transform: "none" },
      ],
      { duration: 360, easing: "ease-out" },
    );
    for (const el of legs()) swing(el, 10, { duration: 120, iterations: 4, direction: "alternate" });
    wobbleHat();
  }

  // ---- walking

  function rest() {
    walking = false;
    stopGait();
    setLean(0);
    walkTimer = setTimeout(() => whenVisible(burst), rand(tune.restMin * 1000, tune.restMax * 1000));
  }

  function burst() {
    if (happy) return rest();
    crouch(() => step(1 + Math.floor(Math.random() * tune.maxSteps), true));
  }

  function step(left: number, first: boolean) {
    const options = neighbours(here);
    if (!mover || options.length === 0) return rest();
    // Never straight back the way it came, unless it is a dead end.
    const fresh = options.filter((j) => j !== prev);
    const pool = fresh.length > 0 ? fresh : options;
    const next = pool[Math.floor(Math.random() * pool.length)];
    const a = nodes[here];
    const b = nodes[next];
    const ms = Math.max(420, (Math.hypot(b.x - a.x, b.y - a.y) / tune.speed) * 1000);
    const last = left === 1;
    // One smooth run through the crossings: it only speeds up at the start of
    // a burst and slows down at its end.
    const easing = first && last ? "ease-in-out" : first ? "ease-in" : last ? "ease-out" : "linear";
    walking = true;
    startGait();
    setLean(Math.sign(b.x - a.x) * tune.lean);
    mover.style.transform = placeAt(b);
    mover.animate([{ transform: placeAt(a) }, { transform: placeAt(b) }], { duration: ms, easing });
    prev = here;
    here = next;
    walkTimer = setTimeout(() => {
      if (!last && !happy && !document.hidden) return step(left - 1, false);
      rest();
      settle();
    }, ms);
  }

  // ---- eyes on the pointer

  let pointer: { x: number; y: number } | null = null;
  let lastMove = 0;
  let look = { ...REST_LOOK };
  let lookRaf = 0;

  function lookTarget() {
    if (!pointer || !art) return REST_LOOK;
    const r = art.getBoundingClientRect();
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height * 0.55;
    const dx = pointer.x - cx;
    const dy = pointer.y - cy;
    const dist = Math.hypot(dx, dy) || 1;
    // Full turn of the eye from about 120 px away; closer, it only glances.
    const reach = Math.min(1, dist / 120);
    return { x: (dx / dist) * tune.lookReach * reach, y: (dy / dist) * tune.lookReach * 1.2 * reach };
  }

  function lookStep() {
    lookRaf = 0;
    const t = lookTarget();
    look = { x: look.x + (t.x - look.x) * 0.3, y: look.y + (t.y - look.y) * 0.3 };
    pupils?.style.setProperty("transform", `translate(${look.x}px, ${look.y}px)`);
    if (Math.abs(t.x - look.x) > 0.01 || Math.abs(t.y - look.y) > 0.01) {
      lookRaf = requestAnimationFrame(lookStep);
    }
  }

  function wantLook() {
    if (!lookRaf) lookRaf = requestAnimationFrame(lookStep);
  }

  function onPointerMove(e: PointerEvent) {
    pointer = { x: e.clientX, y: e.clientY };
    lastMove = performance.now();
    wantLook();
  }

  function onPointerOut() {
    pointer = null;
    wantLook();
  }

  /** Blinks, taps a foot, shakes its hat, and looks around when nobody is
   *  moving the pointer. */
  function idleLoop() {
    idleTimer = setTimeout(
      () =>
        whenVisible(() => {
          if (!happy && !walking) {
            const roll = Math.random();
            const watching = pointer && performance.now() - lastMove < 4000;
            if (roll < 0.15) tapFoot();
            else if (roll < 0.25) wobbleHat();
            else if (roll < 0.45 && !watching && pupils) {
              const frame = (x: number, y: number, offset?: number) => ({
                transform: `translate(${x}px, ${y}px)`,
                ...(offset === undefined ? {} : { offset }),
              });
              pupils.animate(
                [
                  frame(REST_LOOK.x, REST_LOOK.y),
                  frame(-1.1, 0.4, 0.15),
                  frame(-1.1, 0.4, 0.45),
                  frame(1.1, 0.4, 0.6),
                  frame(1.1, 0.4, 0.85),
                  frame(REST_LOOK.x, REST_LOOK.y),
                ],
                { duration: 2600, easing: "ease-in-out" },
              );
            } else blink();
          }
          idleLoop();
        }),
      rand(tune.idleMin * 1000, tune.idleMax * 1000),
    );
  }

  $effect(() => {
    // The height arrives after the first frame, and changes with the window's;
    // a crossing that no longer exists sends the spider to the middle of the web.
    if (!placed || here >= nodes.length) {
      here = Math.floor(nodes.length / 2);
      prev = -1;
      placed = nodes.length > 0;
    }
  });

  $effect(() => {
    // Resizing moves every crossing; a spider at rest goes with its own.
    if (mover && spot && !walking) mover.style.transform = placeAt(spot);
  });

  $effect(() => {
    // The pupils are redrawn when the smile goes away; put them back where
    // they were looking.
    if (pupils) {
      pupils.style.transform = `translate(${look.x}px, ${look.y}px)`;
      wantLook();
    }
  });

  // "Walk now" from the debug panel: cut the rest short.
  let walkNowSeen = $spiderWalkNow;
  $effect(() => {
    if ($spiderWalkNow === walkNowSeen) return;
    walkNowSeen = $spiderWalkNow;
    if (!walking) {
      clearTimeout(walkTimer);
      burst();
    }
  });

  onMount(() => {
    window.addEventListener("pointermove", onPointerMove, { passive: true });
    document.documentElement.addEventListener("pointerleave", onPointerOut);
    window.addEventListener("blur", onPointerOut);
    rest();
    idleLoop();
  });

  onDestroy(() => {
    gone = true;
    clearTimeout(walkTimer);
    clearTimeout(idleTimer);
    cancelAnimationFrame(lookRaf);
    stopGait();
    window.removeEventListener("pointermove", onPointerMove);
    document.documentElement.removeEventListener("pointerleave", onPointerOut);
    window.removeEventListener("blur", onPointerOut);
  });
</script>

<div class="relative min-h-0 flex-1 overflow-hidden" bind:clientHeight={height} bind:clientWidth={width} aria-hidden="true">
  {#if web && spot}
    <svg
      class="pointer-events-none absolute bottom-0 left-0"
      width={R + 2}
      height={R + 2}
      viewBox="0 -2 {R + 2} {R + 2}"
      fill="none"
    >
      <g stroke="var(--color-zinc-500)" stroke-opacity="0.5" stroke-width="0.8" stroke-linecap="round">
        {#each web.radials as p, k (k)}
          <line x1="0" y1={R} x2={p.x} y2={R - p.y} />
        {/each}
        {#each web.rings as d, i (i)}
          <path d={d} />
        {/each}
      </g>
    </svg>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={mover}
      class="absolute bottom-0 left-0 cursor-default"
      style:width="{SIZE.w}px"
      style:height="{SIZE.h}px"
      onpointerenter={hello}
      onpointerleave={() => (happy = false)}
    >
      <div bind:this={leaner} class="origin-bottom">
        <svg bind:this={art} viewBox="0 -1 32 29" width={SIZE.w} height={SIZE.h} class="spider">
          {#each LEGS as leg (leg.d)}
            <g
              data-leg={leg.set}
              data-side={leg.side}
              class="pivot"
              style:transform-origin="{leg.hip.x}px {leg.hip.y}px"
            >
              <path
                d={leg.d}
                stroke="var(--color-emerald-700)"
                stroke-width="1.25"
                stroke-linecap="round"
                fill="none"
              />
              <circle cx={leg.foot.x} cy={leg.foot.y} r="0.85" fill="var(--color-emerald-700)" />
            </g>
          {/each}
          <g bind:this={body} class="pivot body">
            <ellipse
              cx="16"
              cy="15.6"
              rx="9.4"
              ry="8.8"
              fill="var(--color-emerald-400)"
              stroke="var(--color-emerald-800)"
              stroke-width="0.9"
            />
            <ellipse
              cx="11.4"
              cy="10.4"
              rx="2.4"
              ry="1.15"
              transform="rotate(-30 11.4 10.4)"
              fill="white"
              fill-opacity="0.22"
            />
            <ellipse cx="9.8" cy="19.8" rx="1.9" ry="1.05" fill="oklch(0.74 0.15 5 / {happy ? 0.95 : 0.75})" />
            <ellipse cx="22.2" cy="19.8" rx="1.9" ry="1.05" fill="oklch(0.74 0.15 5 / {happy ? 0.95 : 0.75})" />
            {#if happy}
              <g stroke="white" stroke-width="1.35" stroke-linecap="round" fill="none">
                {#each EYES as e (e.x)}
                  <path d="M{e.x - 2.2} {e.y + 1}Q{e.x} {e.y - 2} {e.x + 2.2} {e.y + 1}" />
                {/each}
              </g>
            {:else}
              <g bind:this={eyes} class="pivot eyes">
                {#each EYES as e (e.x)}
                  <ellipse cx={e.x} cy={e.y} rx="3.15" ry="3.6" fill="white" />
                {/each}
                <g bind:this={pupils}>
                  {#each EYES as e (e.x)}
                    <circle cx={e.x} cy={e.y} r="2" fill="#0a0a0a" />
                    <circle cx={e.x - 0.8} cy={e.y - 1.1} r="0.85" fill="white" />
                    <circle cx={e.x + 0.8} cy={e.y + 1} r="0.4" fill="white" />
                  {/each}
                </g>
              </g>
            {/if}
            <path
              d="M14.3 20.4Q15.15 21.5 16 20.6Q16.85 21.5 17.7 20.4"
              stroke="var(--color-emerald-800)"
              stroke-width="0.85"
              stroke-linecap="round"
              stroke-linejoin="round"
              fill="none"
            />
            <path d="M16.75 21.05L17.35 20.85L17.15 21.95Z" fill="white" />
            <g bind:this={hat} class="pivot hat">
              <g transform="rotate(-12 16 6)">
                <path
                  d="M9.6 7.6Q16 5.6 22.4 7.6Q16 9.2 9.6 7.6Z"
                  fill="oklch(0.3 0.09 300)"
                  stroke="oklch(0.42 0.11 300)"
                  stroke-width="0.4"
                />
                <path
                  d="M12.6 7L15.6 0.6Q16.4-0.4 17 0.8L19.6 7Z"
                  fill="oklch(0.3 0.09 300)"
                  stroke="oklch(0.42 0.11 300)"
                  stroke-width="0.4"
                  stroke-linejoin="round"
                />
                <path d="M12.4 6.4Q16 5.4 19.8 6.4L19.6 7.4Q16 6.4 12.6 7.4Z" fill="var(--color-emerald-400)" />
              </g>
            </g>
          </g>
        </svg>
      </div>
    </div>
  {/if}
</div>

<style>
  .spider {
    overflow: visible;
  }
  .pivot {
    transform-box: view-box;
  }
  .body {
    transform-origin: 16px 24px;
  }
  .eyes {
    transform-origin: 16px 15.4px;
  }
  .hat {
    transform-origin: 16px 7.4px;
  }
</style>
