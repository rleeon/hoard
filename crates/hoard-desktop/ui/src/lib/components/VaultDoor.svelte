<script lang="ts">
  /**
   * The settings icon: a vault door, the nine-toothed cog of the bunker games,
   * with a 13 on its centre plate. Chunky teeth with narrow valleys, a dark
   * channel between the rim and the plate, the plate and the number cut out of
   * one fill (a mask, so it reads on any background).
   *
   * The 13 is drawn as strokes, not set in a font. At the 16-18 px it renders at,
   * a system font's 13 is about six pixels tall and its hinted 1 fuses with the 3
   * into a B, whatever the spacing. Round-capped strokes stay apart, and short
   * plate gives the number most of the door's face.
   *
   * The pointer arriving on its host (the nav button) plays the door: it rolls to
   * the left along a floor line, turning anticlockwise as a wheel does, and leaves
   * the vault's cog-shaped mouth showing behind it; the turn is three teeth, so
   * the door lands square; two seconds later it rolls back,
   * turning clockwise, and seals it. JavaScript owns the phases because the close
   * has to happen on its own, after the pointer has gone; `app.css` ("Vault
   * door") draws each one.
   *
   * Lucide's prop type, so it fits anywhere a lucide icon is expected (the nav's
   * `icon: typeof Home`, the tour's steps).
   */
  import { onMount } from "svelte";
  import type { IconProps } from "@lucide/svelte";

  let {
    size = 24,
    color = "currentColor",
    strokeWidth: _strokeWidth,
    class: klass = "",
    ...rest
  }: IconProps = $props();

  const HOLD_MS = 2000;

  /** A cog of `teeth` trapezoids around (12, 12), the first pointing up. Each
   *  tooth is `root`° wide at the rim and `tip`° at the top (half-angles). */
  function cog(outer: number, rim: number, root = 16.5, tip = 11.5, teeth = 9): string {
    const at = (r: number, deg: number) => {
      const a = (deg * Math.PI) / 180;
      return `${(12 + r * Math.cos(a)).toFixed(2)} ${(12 + r * Math.sin(a)).toFixed(2)}`;
    };
    const step = 360 / teeth;
    let d = "";
    for (let k = 0; k < teeth; k++) {
      const t = -90 + step * k;
      d += `${k ? "L" : "M"}${at(rim, t - root)}L${at(outer, t - tip)}L${at(outer, t + tip)}`;
      d += `L${at(rim, t + root)}A${rim} ${rim} 0 0 1 ${at(rim, t + step - root)}`;
    }
    return `${d}Z`;
  }
  const DOOR = cog(11.3, 9.5);
  const MOUTH = cog(10.17, 8.55);

  // Two instances (the nav, the tour) must not share a mask id.
  const uid = $props.id();
  const cut = `vault-cut-${uid}`;
  const glow = `vault-glow-${uid}`;
  const beam = `vault-beam-${uid}`;
  const soft = `vault-soft-${uid}`;

  // On the frame, a couple of pixels clear of the door's upper-left and
  // upper-right teeth. Each throws one beam; the right one starts mirrored
  // and turns the other way, so the pair sweeps symmetrically.
  const BEACONS = [
    { x: 0.2, y: -3.77, flip: false },
    { x: 23.8, y: -3.77, flip: true },
  ];
  const BEAM_REACH = 36;
  // Three nested fans, faint to bright: stacked, the light is strongest down the
  // middle of the beam and dies off towards its edges, like a real one.
  const BEAM_LAYERS = [
    { half: 48, opacity: 0.35 },
    { half: 30, opacity: 0.5 },
    { half: 14, opacity: 1 },
  ];

  function fan(half: number, flip: boolean): string {
    const a = (half * Math.PI) / 180;
    const x = (flip ? -1 : 1) * BEAM_REACH * Math.cos(a);
    const y = (flip ? -1 : 1) * BEAM_REACH * Math.sin(a);
    const r = BEAM_REACH;
    return `M0 0L${x.toFixed(2)} ${(-y).toFixed(2)}A${r} ${r} 0 0 1 ${x.toFixed(2)} ${y.toFixed(2)}Z`;
  }

  let el = $state<SVGSVGElement | null>(null);
  let phase = $state<"idle" | "opening" | "open" | "closing">("idle");
  let hold: ReturnType<typeof setTimeout> | null = null;

  function play() {
    if (phase !== "idle") return;
    phase = "opening";
  }

  function settle(e: AnimationEvent) {
    if (e.animationName === "vault-door-open") {
      phase = "open";
      hold = setTimeout(() => (phase = "closing"), HOLD_MS);
    } else if (e.animationName === "vault-door-close") {
      phase = "idle";
    }
  }

  onMount(() => {
    const host = el?.parentElement;
    host?.addEventListener("pointerenter", play);
    return () => {
      host?.removeEventListener("pointerenter", play);
      if (hold) clearTimeout(hold);
    };
  });
</script>

<svg
  bind:this={el}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  overflow="visible"
  xmlns="http://www.w3.org/2000/svg"
  class="vault-{phase} {klass}"
  aria-hidden="true"
  onanimationend={settle}
  {...rest}
>
  <!-- The floor the door rolls on. It reaches past the left edge, which is
       where the door ends up. -->
  <line
    class="vault-floor"
    x1="-15"
    y1="23.8"
    x2="21"
    y2="23.8"
    stroke={color}
    stroke-width="1.1"
    stroke-linecap="round"
  />
  <!-- The vault's mouth: the door's cog, a touch smaller. -->
  <path
    class="vault-hole"
    d={MOUTH}
    fill="black"
    fill-opacity="0.55"
    stroke={color}
    stroke-opacity="0.55"
    stroke-width="1.1"
    stroke-linejoin="round"
  />
  <mask id={cut}>
    <rect x="-2" y="-2" width="28" height="28" fill="white" />
    <circle cx="12" cy="12" r="7.45" stroke="black" stroke-width="0.9" />
    <!-- Nudged 0.12 right: measured on the curves, that is what puts the
         number's box on the door's centre to the hundredth. -->
    <g
      transform="translate(12.12 12) scale(0.92) translate(-12 -12)"
      stroke="black"
      stroke-width="1.63"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M8 8.8 9.6 7.6v8.8" />
      <path
        d="M11.9 8.3c.7-.6 1.7-.8 2.6-.5 1.3.4 1.5 2.2.4 3.1-.5.4-1.2.6-1.8.6.7 0 1.5.2 2 .7 1.1 1 .8 3.1-.7 3.7-1 .4-2.1.2-2.8-.4"
      />
    </g>
  </mask>
  <g class="vault-door">
    <!-- Keeps the box the door turns about centred on (12, 12); the cog alone
         sits a hair low, having two teeth at the bottom and one at the top. -->
    <circle cx="12" cy="12" r="11.4" />
    <path
      d={DOOR}
      mask="url(#{cut})"
      fill={color}
      stroke={color}
      stroke-width="0.6"
      stroke-linejoin="round"
    />
  </g>
  <!-- The warning beacons. Like the mouth, they only exist while the door is
       away: a lamp, a halo that flares as the beam comes round, and one beam
       sweeping round it. The beams are long, wide and faint, fading with
       distance and blurred at the edges, so they read as light thrown across
       the rail rather than as shapes. -->
  <defs>
    <radialGradient id={glow}>
      <stop offset="0" stop-color="#f87171" stop-opacity="0.9" />
      <stop offset="1" stop-color="#ef4444" stop-opacity="0" />
    </radialGradient>
    <radialGradient id={beam} cx="0" cy="0" r={BEAM_REACH} gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f87171" stop-opacity="0.55" />
      <stop offset="0.5" stop-color="#ef4444" stop-opacity="0.22" />
      <stop offset="1" stop-color="#ef4444" stop-opacity="0" />
    </radialGradient>
    <filter id={soft} x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="1.6" />
    </filter>
  </defs>
  {#each BEACONS as b (b.x)}
    <g class="vault-beacon" transform="translate({b.x} {b.y})">
      <g class="vault-beam" class:vault-beam-flip={b.flip}>
        <!-- Centres the box the beam turns about on the lamp; a lone beam's own
             box sits off to one side of it. -->
        <circle r={BEAM_REACH} />
        <g fill="url(#{beam})" filter="url(#{soft})">
          {#each BEAM_LAYERS as l (l.half)}
            <path d={fan(l.half, b.flip)} fill-opacity={l.opacity} />
          {/each}
        </g>
      </g>
      <circle class="vault-halo" r="3.6" fill="url(#{glow})" />
      <circle r="1.7" fill="#ef4444" />
      <circle cx="-0.45" cy="-0.5" r="0.6" fill="#fecaca" />
    </g>
  {/each}
</svg>
