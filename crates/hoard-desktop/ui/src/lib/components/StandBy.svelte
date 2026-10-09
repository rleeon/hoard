<script lang="ts" module>
  let nextId = 0;
</script>

<script lang="ts">
  /**
   * The cover a dashboard card shows when a game has no art: a nod to
   * Fallout's "Please stand by" screen, drawn from the broadcast test pattern
   * it borrows from rather than traced: the lit phosphor screen, the grid, the
   * big ring and its corner targets, the X, the inner circle with its two
   * shaded wedges, the ladder below. The headdress at the top of the original
   * is left out on purpose. The text is part of the drawing, so it is not
   * translated, and the palette is the screen's own olive, not the accent.
   */
  // Each card carries its own copy, and a pattern id has to be unique in the page.
  const uid = `standby-${nextId++}`;

  const INK = "#262d1c";
  const C = 100;
  const polar = (r: number, deg: number) => {
    const a = (deg * Math.PI) / 180;
    return { x: C + r * Math.cos(a), y: C + r * Math.sin(a) };
  };
  // A filled pie slice of the inner circle, the shaded wedges of the original.
  const wedge = (r: number, from: number, to: number) => {
    const a = polar(r, from);
    const b = polar(r, to);
    return `M${C} ${C}L${a.x} ${a.y}A${r} ${r} 0 0 1 ${b.x} ${b.y}Z`;
  };
  // The funnel under the inner circle, narrowing towards the bottom of the ring.
  const ladder = [52, 42, 33, 25, 18, 12, 7].map((w, i) => ({ y: 142 + i * 6.2, w }));
  const numbers = ["2", "3", "4", "5"];
</script>

<div class="standby relative h-full w-full overflow-hidden" aria-hidden="true">
  <svg class="absolute inset-0 h-full w-full" preserveAspectRatio="none">
    <defs>
      <pattern id="{uid}-grid" width="34" height="26" patternUnits="userSpaceOnUse">
        <path d="M34 0H0V26" fill="none" stroke={INK} stroke-width="1.2" />
      </pattern>
    </defs>
    <rect width="100%" height="100%" fill="url(#{uid}-grid)" opacity="0.32" />
  </svg>

  {#each ["-left-[3%] top-[3%]", "-right-[3%] top-[3%]", "-left-[3%] bottom-[3%]", "-right-[3%] bottom-[3%]"] as spot (spot)}
    <svg class="absolute aspect-square w-[30%] {spot}" viewBox="0 0 60 60" fill="none">
      <g stroke={INK} stroke-linecap="round">
        <circle cx="30" cy="30" r="27" stroke-width="1.8" opacity="0.75" />
        <circle cx="30" cy="30" r="20" stroke-width="0.9" opacity="0.45" />
        <path d="M30 3V57M3 30H57" stroke-width="0.9" opacity="0.45" />
      </g>
      <path d="M30 30 18 10H42ZM30 30 18 50H42Z" fill={INK} opacity="0.28" />
    </svg>
  {/each}

  <svg class="absolute inset-[4%] h-[92%] w-[92%]" viewBox="0 0 200 200" fill="none">
    <g stroke={INK} stroke-linecap="round">
      <path d="M100 8V136" stroke-width="1.4" opacity="0.7" />
      <path
        d="M37 37 72 72M163 37 128 72M37 163 72 128M163 163 128 128"
        stroke-width="1.6"
        opacity="0.75"
      />
      <circle cx="100" cy="100" r="93" stroke-width="2.4" opacity="0.85" />
    </g>
    <path d={wedge(40, 205, 240)} fill={INK} opacity="0.55" />
    <path d={wedge(40, 25, 60)} fill={INK} opacity="0.55" />
    <path d="M97 62 103 62 106 140 94 140Z" fill={INK} opacity="0.35" />
    <circle cx="100" cy="100" r="40" stroke={INK} stroke-width="2" opacity="0.85" />
    {#each numbers as n, i (n)}
      {@const p = polar(30, -68 + i * 11)}
      {@const q = polar(30, 112 + i * 11)}
      <text x={p.x} y={p.y} fill={INK} font-size="6.5" font-weight="700" text-anchor="middle" opacity="0.6"
        >{n}</text
      >
      <text x={q.x} y={q.y} fill={INK} font-size="6.5" font-weight="700" text-anchor="middle" opacity="0.6"
        >{n}</text
      >
    {/each}
    <g stroke={INK} stroke-linecap="round" stroke-width="2" opacity="0.7">
      {#each ladder as l (l.y)}
        <path d="M{C - l.w} {l.y}H{C + l.w}" />
      {/each}
    </g>
    <!-- Condensed by `textLength`: the font has no narrow cut, and the
         original's lettering is tall and tight. -->
    <text
      x="100"
      y="107.8"
      text-anchor="middle"
      textLength="174"
      lengthAdjust="spacingAndGlyphs"
      fill={INK}
      font-size="22"
      font-weight="800"
      class="font-sans"
      opacity="0.92">PLEASE STAND BY</text
    >
  </svg>

  <div class="scanlines pointer-events-none absolute inset-0"></div>
</div>

<style>
  /* The lit screen: bright in the middle, falling off to near black at the
     edges, as on a CRT warming up. */
  .standby {
    background:
      radial-gradient(ellipse 70% 45% at 50% 30%, rgb(206 219 196 / 0.55), transparent 70%),
      radial-gradient(
        ellipse 85% 75% at 50% 48%,
        #b9c6a8 0%,
        #8f9c78 34%,
        #56613d 62%,
        #1e2412 88%,
        #0b0d06 100%
      );
  }
  .scanlines {
    background: repeating-linear-gradient(to bottom, rgb(0 0 0 / 0.06) 0 1px, transparent 1px 3px);
  }
</style>
