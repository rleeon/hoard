<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount, tick } from 'svelte';
  import { Check, ArrowRight } from 'lucide-svelte';
  import { reveal } from '$lib/actions/reveal';
  import { tilt } from '$lib/actions/tilt';
  import { localeHref } from '$lib/i18n/href';
  import AgentsStrip from './AgentsStrip.svelte';

  // The "AI" section of the home: Hoard runs no AI, the agent is the user's,
  // and the whole thing is optional. The full story lives on /ai.
  //
  // The terminal is a simulation, and the "?" in its corner says so. What it
  // shows is what the skill makes an agent do (SKILL.md, "Safety rules"): read
  // with --json, rehearse the restore with --dry-run, name the files it would
  // overwrite, then ask. Commands, flags and JSON keys are the real ones
  // (`SaveInfo`, `RestoreOut`, `PreviewOut` in hoard-cli) and stay in English in
  // every locale; what the person and the agent say is translated.
  // `_autosave1.zip` is what Factorio really writes.
  // Hover and focus open the note; a tap toggles it too, because iOS Safari
  // gives a tapped button neither.
  let tip = $state(false);

  // Anyone can type into it, and the agent always answers the same thing, on
  // the user's call: a joke for whoever talks to a simulation (the F.E.V. is
  // Fallout's). First a green line dressed as a command, then the agent's
  // sentence, both word for word as the user wrote them. The screen keeps the
  // height of the transcript it starts with; new lines push the old ones out of
  // the top instead of growing the card.
  const COMMAND = 'Releasing F.E.V..';
  const REPLY = 'F.E.V released, we triumphs.';
  type Line = { id: number; who: 'you' | 'command' | 'agent'; text: string };
  let lines = $state<Line[]>([]);
  let draft = $state('');
  let focused = $state(false);
  let screen: HTMLDivElement;
  let input: HTMLInputElement;
  let screenH = $state<number | null>(null);
  let seq = 0;

  function push(who: Line['who'], text: string) {
    // Only what still fits on screen matters; the rest is already off the top.
    lines = [...lines, { id: seq++, who, text }].slice(-24);
  }

  // A native caret can't be a block, so the input hides its own and this one
  // sits where the caret is, found by measuring the text before it in a hidden
  // copy. It restarts lit on every move and blinks while it rests, like a
  // terminal's. SCALE is the input's `transform` in `.term-input`.
  const SCALE = 0.78125;
  let mirror: HTMLSpanElement;
  let caretPos = $state(0);
  let caretX = $state(0);
  let caretShown = $state(true);
  let blink = $state(0);

  function trackCaret() {
    // A frame later, so a keydown has already moved the selection.
    requestAnimationFrame(async () => {
      if (!input) return;
      const start = input.selectionStart ?? draft.length;
      caretShown = start === (input.selectionEnd ?? start);
      caretPos = start;
      blink++;
      await tick();
      caretX = mirror.offsetWidth - input.scrollLeft * SCALE;
    });
  }

  function send(e: SubmitEvent) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    draft = '';
    trackCaret();
    push('you', text);
    setTimeout(() => push('command', COMMAND), 350);
    setTimeout(() => push('agent', REPLY), 1100);
  }

  onMount(() => {
    // Measured, not a fixed rem: the transcript wraps differently on every
    // width and in every language. Re-measured on a width change until the
    // first message, after which the screen stays as it is.
    let width = 0;
    const lock = () => {
      if (lines.length || !screen) return;
      screenH = null;
      requestAnimationFrame(() => (screenH = screen.offsetHeight));
    };
    const ro = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width === width) return;
      width = entry.contentRect.width;
      lock();
    });
    ro.observe(screen);
    document.fonts?.ready.then(lock);
    return () => ro.disconnect();
  });

  const calls = [
    {
      cmd: 'hoard saves --json',
      out: '{"ok":true,"data":[{"save_id":"9c1e2f7a-…","game_slug":"factorio","latest_version_num":42,…},…]}'
    },
    {
      cmd: 'hoard snapshots list 9c1e2f7a-… --json',
      out: '{"ok":true,"data":[{"version_num":42,…},{"version_num":41,…},…]}'
    },
    {
      cmd: 'hoard restore 9c1e2f7a-… --version 41 --dry-run --json',
      out: '{"ok":true,"data":{"version":41,"dry_run":true,"preview":{"modified":["_autosave1.zip","_autosave2.zip"],"added":[],"local_only":[]}}}'
    }
  ];
</script>

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 pb-14 pt-20 sm:px-6 sm:pb-16 sm:pt-24">
    <div class="grid items-center gap-12 lg:grid-cols-2">
      <div class="reveal min-w-0" use:reveal>
        <p class="kicker">{$_('aihome.kicker')}</p>
        <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
          {$_('aihome.title')}
        </h2>
        <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
          {$_('aihome.body')}
        </p>
        <ul class="mt-6 space-y-3 text-sm">
          {#each ['p1', 'p2', 'p3'] as p (p)}
            <li class="flex items-start gap-2.5 text-ink-soft">
              <Check class="mt-0.5 h-4 w-4 flex-none text-ink" />
              {$_(`aihome.${p}`)}
            </li>
          {/each}
        </ul>
        <a
          href={$localeHref('/ai')}
          class="link-underline mt-7 inline-flex items-center gap-1.5 text-sm font-medium text-ink ring-focus hover:text-white"
        >
          {$_('aihome.cta')}
          <ArrowRight class="h-4 w-4" />
        </a>
      </div>

      <div class="reveal min-w-0" use:reveal={{ delay: 100 }}>
        <div
          class="tilt rounded-2xl border bg-capsule transition-colors {focused
            ? 'border-white/25'
            : 'border-white/10'}"
          use:tilt
        >
          <div class="flex items-center justify-between border-b border-white/[0.08] px-4 py-2.5">
            <span class="font-mono text-[11px] text-ink-faint">~/games</span>
            <div class="group relative">
              <button
                type="button"
                class="grid h-6 w-6 place-items-center rounded-full border border-white/20 font-mono text-[11px] text-ink-faint ring-focus transition-colors hover:border-white/40 hover:text-ink"
                aria-label={$_('aihome.demo_note')}
                aria-describedby="ai-demo-note"
                aria-expanded={tip}
                onclick={() => (tip = !tip)}
                onblur={() => (tip = false)}
              >
                ?
              </button>
              <div
                id="ai-demo-note"
                role="tooltip"
                class="pointer-events-none absolute right-0 top-8 z-10 w-60 rounded-xl border border-white/10 bg-black px-3 py-2 text-xs leading-relaxed text-ink-soft shadow-lg transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 {tip
                  ? 'opacity-100'
                  : 'opacity-0'}"
              >
                {$_('aihome.demo_note')}
              </div>
            </div>
          </div>

          <!-- A click anywhere on the screen lands in the prompt, as in a real
               terminal; keyboard users tab straight to the input. -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            bind:this={screen}
            class="flex flex-col justify-end overflow-hidden px-5 py-5 font-mono text-[12.5px] leading-relaxed sm:px-6 {lines.length
              ? 'term-fade'
              : ''}"
            style:height={screenH ? `${screenH}px` : undefined}
            onclick={() => input?.focus()}
          >
            <div class="shrink-0" aria-hidden="true">
              <p class="text-ink"><span class="text-ink-faint">&gt;</span> {$_('aihome.demo_ask')}</p>

              {#each calls as c (c.cmd)}
                <div class="mt-4">
                  <p class="truncate text-ink">
                    <span class="text-emerald-500">●</span> {c.cmd}
                  </p>
                  <p class="truncate pl-4 text-ink-faint">└ {c.out}</p>
                </div>
              {/each}

              <div class="mt-5 flex gap-2 text-ink-soft">
                <span class="text-ink">●</span>
                <p class="font-sans text-[13.5px] leading-relaxed">
                  {$_('aihome.demo_reply')}
                  <span class="text-ink">{$_('aihome.demo_confirm')}</span>
                </p>
              </div>
            </div>

            <div class="shrink-0" aria-live="polite">
              {#each lines as l (l.id)}
                {#if l.who === 'you'}
                  <p class="mt-4 break-words text-ink">
                    <span class="text-ink-faint">&gt;</span> {l.text}
                  </p>
                {:else if l.who === 'command'}
                  <p class="mt-3 truncate text-emerald-500">● {l.text}</p>
                {:else}
                  <div class="mt-3 flex gap-2 text-ink-soft">
                    <span class="text-ink">●</span>
                    <p class="font-sans text-[13.5px] leading-relaxed">{l.text}</p>
                  </div>
                {/if}
              {/each}
            </div>

            <form class="mt-5 flex shrink-0 items-center gap-2" onsubmit={send}>
              <span class="text-ink-faint">&gt;</span>
              <div class="relative min-w-0 flex-1">
                <input
                  bind:this={input}
                  bind:value={draft}
                  onfocus={() => {
                    focused = true;
                    trackCaret();
                  }}
                  onblur={() => {
                    focused = false;
                    trackCaret();
                  }}
                  oninput={trackCaret}
                  onkeydown={trackCaret}
                  onclick={trackCaret}
                  onselect={trackCaret}
                  aria-label={$_('aihome.demo_input')}
                  maxlength="200"
                  autocomplete="off"
                  autocapitalize="off"
                  spellcheck="false"
                  class="term-input bg-transparent text-ink caret-transparent outline-none"
                />
                <span
                  bind:this={mirror}
                  class="invisible absolute left-0 top-0 whitespace-pre"
                  aria-hidden="true">{draft.slice(0, caretPos)}</span
                >
                {#if caretShown}
                  {#key blink}
                    <span
                      class="caret pointer-events-none absolute top-1/2 -translate-y-1/2"
                      style:left="{caretX}px"
                      data-motion="keep"
                    ></span>
                  {/key}
                {/if}
              </div>
            </form>
          </div>
        </div>
      </div>
    </div>
  </div>

  <AgentsStrip class="border-t border-line" />
</section>

<style>
  /* iOS zooms the page into any input under 16px. The field is 16px, scaled
     down to the terminal's 12.5px, which the zoom check does not look at. */
  .term-input {
    font-size: 16px;
    width: 128%;
    transform: scale(0.78125);
    transform-origin: left center;
  }

  /* Once lines start leaving through the top, the one halfway out fades instead
     of showing as a sliced row of glyphs. */
  .term-fade {
    mask-image: linear-gradient(to bottom, transparent 0, #000 2.5rem);
  }

  /* A terminal's block caret. It keeps blinking under reduced-motion
     (`data-motion="keep"`), at the owner's request: it blinks in place and
     travels nowhere. `difference` turns the letter under it dark, as a block
     caret does. */
  .caret {
    width: 1ch;
    height: 1.05em;
    background: var(--color-ink);
    mix-blend-mode: difference;
    animation: caret 1.1s steps(1) infinite;
  }
  @keyframes caret {
    50% {
      opacity: 0;
    }
  }
</style>
