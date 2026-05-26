<script lang="ts">
  import { onMount } from 'svelte';

  type Line = {
    kind: 'prompt' | 'output' | 'output-muted' | 'output-emerald' | 'spacer';
    text: string;
    /** characters per second when typing; ignored for instant/output rows */
    speed?: number;
    /** instant = full text appears at once, no per-char typing */
    instant?: boolean;
    /** delay before this line begins, in ms */
    delay?: number;
  };

  const SAVE_ID_FULL = '8e3a9f4c-2b71-4d68-a3f1-91d2b6c4f7e0';

  const script: Line[] = [
    { kind: 'prompt', text: 'hoard save list', speed: 60, delay: 250 },
    {
      kind: 'output-muted',
      text: 'ID                                    GAME             LABEL    VERS    SIZE',
      instant: true,
      delay: 280
    },
    {
      kind: 'output',
      text: `${SAVE_ID_FULL}  stardew-valley   default    47    8.20M`,
      instant: true,
      delay: 40
    },
    { kind: 'spacer', text: '', delay: 420 },
    {
      kind: 'prompt',
      text: 'hoard backup 8e3a9f4c --from ~/.config/StardewValley/Saves',
      speed: 55
    },
    {
      kind: 'output-muted',
      text: 'uploading from /home/raimundo/.config/StardewValley/Saves',
      instant: true,
      delay: 320
    },
    {
      kind: 'output-emerald',
      text: 'snapshot v48 created (12 files, 8.34 MiB)',
      instant: true,
      delay: 280
    },
    { kind: 'spacer', text: '', delay: 480 },
    { kind: 'prompt', text: 'hoard restore 8e3a9f4c --version 43', speed: 55 },
    {
      kind: 'output-muted',
      text: `restoring v43 of 8e3a9f4c-2b71-4d68-a3f1-… to ~/.config/StardewValley/Saves`,
      instant: true,
      delay: 300
    },
    {
      kind: 'output-emerald',
      text: 'restored 12 files (7.94 MiB)',
      instant: true,
      delay: 240
    }
  ];

  // visible[i] = how many chars of script[i] are shown
  let visible = $state<number[]>(script.map(() => 0));
  let activeIdx = $state(-1);
  let paused = $state(false);
  let reducedMotion = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function cancel() {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function showAll() {
    visible = script.map((l) => l.text.length);
    activeIdx = script.length - 1;
  }

  async function play() {
    visible = script.map(() => 0);
    activeIdx = -1;

    for (let i = 0; i < script.length; i++) {
      const line = script[i];
      activeIdx = i;

      if (line.delay) await wait(line.delay);
      if (paused) return;

      if (line.instant || line.kind === 'spacer') {
        visible[i] = line.text.length;
      } else {
        const speed = line.speed ?? 55; // chars per second
        const perChar = 1000 / speed;
        for (let c = 1; c <= line.text.length; c++) {
          if (paused) return;
          await wait(perChar + jitter());
          visible[i] = c;
        }
      }
    }

    // hold final state for a beat, then restart
    await wait(4200);
    if (!paused) play();
  }

  function jitter() {
    // ±25% so typing doesn't sound like a metronome
    return (Math.random() - 0.5) * 28;
  }

  function wait(ms: number) {
    return new Promise<void>((resolve) => {
      timer = setTimeout(resolve, ms);
    });
  }

  onMount(() => {
    if (typeof window !== 'undefined') {
      reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    }
    if (reducedMotion) {
      showAll();
      return;
    }
    play();
    return () => {
      paused = true;
      cancel();
    };
  });

  function handleEnter() {
    if (reducedMotion) return;
    paused = true;
    cancel();
  }
  function handleLeave() {
    if (reducedMotion) return;
    if (!paused) return;
    paused = false;
    play();
  }
</script>

<div
  class="relative overflow-hidden rounded-2xl border border-white/[0.06] bg-zinc-950/80 p-1 shadow-[0_30px_80px_-20px_rgba(0,0,0,0.65),0_0_0_1px_rgba(16,185,129,0.08)]"
  onmouseenter={handleEnter}
  onmouseleave={handleLeave}
  onfocusin={handleEnter}
  onfocusout={handleLeave}
  role="presentation"
>
  <!-- ambient glow ring -->
  <div
    class="pointer-events-none absolute -inset-px rounded-2xl bg-gradient-to-br from-emerald-500/20 via-transparent to-teal-500/10 opacity-60"
    aria-hidden="true"
  ></div>

  <div class="relative rounded-xl bg-[#06080a] p-5 sm:p-6">
    <div class="mb-4 flex items-center gap-1.5">
      <span class="h-3 w-3 rounded-full bg-red-500/70"></span>
      <span class="h-3 w-3 rounded-full bg-amber-500/70"></span>
      <span class="h-3 w-3 rounded-full bg-emerald-500/70"></span>
      <span class="ml-3 text-[11px] font-medium uppercase tracking-wider text-zinc-500">
        hoard — ~/.config/StardewValley/Saves
      </span>
      {#if paused}
        <span
          class="ml-auto rounded-full border border-amber-400/30 bg-amber-400/10 px-2 py-0.5 text-[10px] font-medium text-amber-300"
          aria-live="polite"
        >
          paused
        </span>
      {/if}
    </div>

    <pre
      class="m-0 overflow-x-auto whitespace-pre font-mono text-[13px] leading-[1.65] sm:text-sm"
    ><code class="block">{#each script as line, i (i)}{#if line.kind === 'spacer'}<span class="block h-2"></span>{:else}<span class="block">{#if line.kind === 'prompt'}<span class="text-emerald-500/80">$</span> <span class="text-zinc-100">{line.text.slice(0, visible[i])}</span>{:else if line.kind === 'output-emerald'}<span class="text-emerald-300"
              >→ {line.text.slice(0, visible[i])}</span
            >{:else if line.kind === 'output-muted'}<span class="text-zinc-500"
              >{line.text.slice(0, visible[i])}</span
            >{:else}<span class="text-zinc-300">{line.text.slice(0, visible[i])}</span>{/if}{#if activeIdx === i && visible[i] < line.text.length && !line.instant}<span class="term-caret animate-blink bg-emerald-400"></span>{/if}</span>{/if}{/each}</code></pre>
  </div>
</div>
