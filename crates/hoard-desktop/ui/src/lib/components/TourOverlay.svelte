<script lang="ts">
  /**
   * Guided app tour, shown once, right after the user finishes signing in for
   * the first time. It is a coach-mark walkthrough over the *real* app: each
   * step navigates the content area to its section (via the `navigate` prop the
   * parent owns) and glides a spotlight across the matching sidebar rail item,
   * anchoring an explanation card beside it. The background genuinely changes
   * section and zoom-settles into view, this is not a centered modal.
   *
   * The parent (`App.svelte`) owns navigation + the persisted "seen" flag, so
   * this component only choreographs the highlight and the card. Targets are
   * located by the `data-tour*` markers on the rail, measured with
   * `getBoundingClientRect`, so we never fork the sidebar's own markup.
   *
   * Controls: "Skip" (red, leaves early), "Back", and "Continue"; the final
   * step's primary button reads "Get started" and closes the tour.
   */
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { onMount, onDestroy, tick } from "svelte";
  import {
    Home,
    Boxes,
    Archive,
    RotateCw,
    MonitorPlay,
    Sparkles,
    Settings as SettingsIcon,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import Button from "./Button.svelte";

  type Props = {
    onClose: () => void;
    /**
     * Navigate the app content to a route, or pass `null` to stay put (used by
     * the conceptual + Pro steps, opening the Pro routes would start the
     * one-week trial, so we only spotlight their rail item).
     */
    navigate: (route: string | null) => void;
  };
  let { onClose, navigate }: Props = $props();

  type Step = {
    icon: typeof Home;
    titleKey: string;
    bodyKey: string;
    /** Route pushed on entering the step; `null` keeps the current view. */
    route: string | null;
    /**
     * What the spotlight lands on. `content` highlights the whole content
     * viewport, the real screen the step just navigated to, so the section
     * change is what the user sees. `item` highlights a sidebar rail entry,
     * used by the concept / Pro steps that must not open their route.
     */
    focus: "content" | "item";
    /** Rail selector, required when `focus === "item"`. */
    itemTarget?: string;
    pro?: boolean;
  };

  const steps: Step[] = [
    {
      icon: Home,
      titleKey: "tour.account_title",
      bodyKey: "tour.account_body",
      route: "/account",
      focus: "content",
      itemTarget: '[data-tour-route="/account"]',
    },
    {
      icon: Boxes,
      titleKey: "tour.library_title",
      bodyKey: "tour.library_body",
      route: "/library",
      focus: "content",
      itemTarget: '[data-tour-route="/library"]',
    },
    {
      icon: Archive,
      titleKey: "tour.dashboard_title",
      bodyKey: "tour.dashboard_body",
      route: "/dashboard",
      focus: "content",
      itemTarget: '[data-tour-route="/dashboard"]',
    },
    {
      icon: MonitorPlay,
      titleKey: "tour.screen_title",
      bodyKey: "tour.screen_body",
      // Pro: navigating opens the section in preview mode (see `tourActive`),
      // so it shows the feature without starting the one-week trial.
      route: "/hoard-screen",
      focus: "content",
      itemTarget: '[data-tour-route="/hoard-screen"]',
      pro: true,
    },
    {
      icon: Sparkles,
      titleKey: "tour.wrapped_title",
      bodyKey: "tour.wrapped_body",
      route: "/hoard-wrapped",
      focus: "content",
      itemTarget: '[data-tour-route="/hoard-wrapped"]',
      pro: true,
    },
    {
      icon: SettingsIcon,
      titleKey: "tour.settings_title",
      bodyKey: "tour.settings_body",
      route: "/settings",
      focus: "content",
      itemTarget: '[data-tour-route="/settings"]',
    },
    {
      icon: RotateCw,
      titleKey: "tour.automatic_title",
      bodyKey: "tour.automatic_body",
      // Concept, the toggle lives in the sidebar footer; don't change route.
      route: null,
      focus: "item",
      itemTarget: '[data-tour="automatic"]',
    },
  ];

  const CARD_W = 340;
  const MARGIN = 16;
  // Breathing room baked around the highlighted element.
  const PAD = 6;

  let i = $state(0);
  const step = $derived(steps[i]);
  const isLast = $derived(i === steps.length - 1);
  const isFirst = $derived(i === 0);
  const StepIcon = $derived(step.icon);

  // Reduced-motion degrades the glide, the background zoom and the card
  // transition to a plain fade, no aggressive movement.
  let reduce = $state(false);

  let cardEl = $state<HTMLDivElement | null>(null);
  let cardH = $state(240);
  let vw = $state(typeof window !== "undefined" ? window.innerWidth : 1280);
  let vh = $state(typeof window !== "undefined" ? window.innerHeight : 800);

  type Rect = { top: number; left: number; width: number; height: number };
  // The spotlight target (the whole content area on navigating steps, the rail
  // item on concept/Pro steps) and the card anchor (always the section's rail
  // item, so the card sits in its place beside the menu entry it describes).
  let spotRect = $state<Rect | null>(null);
  let anchorRect = $state<Rect | null>(null);

  function rectOf(selector: string | undefined, pad: number): Rect | null {
    if (!selector) return null;
    const el = document.querySelector(selector) as HTMLElement | null;
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return {
      top: r.top - pad,
      left: r.left - pad,
      width: r.width + pad * 2,
      height: r.height + pad * 2,
    };
  }

  function measure() {
    const spotSel =
      step.focus === "content" ? '[data-tour="content"]' : step.itemTarget;
    // Hug the content viewport edge-to-edge; give rail items a little room.
    spotRect = rectOf(spotSel, step.focus === "content" ? 0 : PAD);
    anchorRect = rectOf(step.itemTarget, PAD);
  }

  const spotStyle = $derived(
    spotRect
      ? `top:${spotRect.top}px;left:${spotRect.left}px;width:${spotRect.width}px;height:${spotRect.height}px;`
      : "",
  );

  // Dock the card beside the section's rail item, clamped inside the viewport;
  // flips to the item's other side, then to a centered fallback, when there's
  // no room or the anchor is missing.
  const cardStyle = $derived.by(() => {
    const ref = anchorRect ?? spotRect;
    if (!ref) {
      const left = Math.max(MARGIN, (vw - CARD_W) / 2);
      const top = Math.max(MARGIN, (vh - cardH) / 2);
      return `left:${left}px;top:${top}px;width:${CARD_W}px;`;
    }
    const gap = 18;
    let left = ref.left + ref.width + gap;
    if (left + CARD_W + MARGIN > vw) {
      left = ref.left - CARD_W - gap;
      if (left < MARGIN) left = MARGIN;
    }
    let top = ref.top - 6;
    top = Math.min(top, vh - cardH - MARGIN);
    top = Math.max(MARGIN, top);
    return `left:${left}px;top:${top}px;width:${CARD_W}px;`;
  });

  // Keep the measured card height current so the clamp above stays accurate as
  // the body copy changes length between steps.
  $effect(() => {
    // `i` is read so this re-runs on every step.
    void i;
    if (cardEl) cardH = cardEl.offsetHeight;
  });

  // Card entrance: a directional fly + scale under normal motion, a plain fade
  // when reduced motion is requested. One static function so the directive
  // resolves at compile time while still honouring the live `reduce` flag.
  function cardIn(_node: Element) {
    const duration = reduce ? 130 : 340;
    return {
      duration,
      css: (t: number) => {
        if (reduce) return `opacity:${t};`;
        const e = cubicOut(t);
        const x = (1 - e) * -10;
        const y = (1 - e) * 8;
        const s = 0.97 + e * 0.03;
        return `opacity:${t};transform:translateX(${x}px) translateY(${y}px) scale(${s});`;
      },
    };
  }

  async function applyStep() {
    navigate(step.route);
    // Let the route swap + any group expansion paint before we measure, then
    // settle focus on the primary action for keyboard users.
    await tick();
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        measure();
        focusPrimary();
      }),
    );
  }

  function next() {
    if (isLast) {
      onClose();
      return;
    }
    i += 1;
    void applyStep();
  }

  function prev() {
    if (isFirst) return;
    i -= 1;
    void applyStep();
  }

  function focusables(): HTMLElement[] {
    if (!cardEl) return [];
    return Array.from(
      cardEl.querySelectorAll<HTMLElement>("button:not([disabled])"),
    );
  }

  function focusPrimary() {
    const f = focusables();
    f[f.length - 1]?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === "ArrowRight") {
      e.preventDefault();
      next();
      return;
    }
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      prev();
      return;
    }
    if (e.key === "Tab") {
      // Trap focus inside the card so tabbing never reaches the (inert) app.
      const f = focusables();
      if (f.length === 0) return;
      const first = f[0];
      const last = f[f.length - 1];
      const active = document.activeElement;
      if (!cardEl?.contains(active)) {
        e.preventDefault();
        first.focus();
      } else if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  function onResize() {
    vw = window.innerWidth;
    vh = window.innerHeight;
    measure();
  }

  onMount(() => {
    reduce =
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("resize", onResize);
    void applyStep();
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeydown);
    window.removeEventListener("resize", onResize);
  });
</script>

<div
  class="fixed inset-0 z-[120]"
  role="dialog"
  aria-modal="true"
  aria-label={$_("tour.aria")}
>
  <!-- Click-blocker: keeps the app behind inert while the tour runs. The
       spotlight below is pointer-events:none, so this also swallows clicks on
       the highlighted rail item itself. -->
  <button
    type="button"
    class="absolute inset-0 cursor-default"
    tabindex="-1"
    aria-hidden="true"
    onclick={(e) => e.preventDefault()}
  ></button>

  {#if spotRect}
    <div
      class="tour-spotlight {reduce ? 'no-motion' : ''}"
      style={spotStyle}
      in:fade={{ duration: reduce ? 0 : 280 }}
      aria-hidden="true"
    ></div>
  {:else}
    <!-- Graceful fallback: dim everything and center the card. -->
    <div class="absolute inset-0 bg-zinc-950/55" aria-hidden="true"></div>
  {/if}

  <div
    bind:this={cardEl}
    class="pointer-events-auto fixed z-10"
    style={cardStyle}
    aria-live="polite"
  >
    {#key i}
      <div
        in:cardIn
        class="rounded-2xl border border-zinc-800 bg-zinc-900/95 p-6 shadow-2xl shadow-black/40 ring-1 ring-white/[0.04] backdrop-blur"
      >
        <div class="flex items-start gap-3">
          <span
            class="inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-emerald-600/15 text-emerald-400 ring-1 ring-emerald-500/25"
          >
            <StepIcon size={22} />
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h2
                class="truncate text-base font-semibold tracking-tight text-zinc-50"
              >
                {$_(step.titleKey)}
              </h2>
              {#if step.pro}
                <span
                  class="shrink-0 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-400 ring-1 ring-emerald-500/25"
                >
                  {$_("tour.pro_badge")}
                </span>
              {/if}
            </div>
            <p class="mt-0.5 text-xs font-medium tabular-nums text-zinc-500">
              {i + 1} / {steps.length}
            </p>
          </div>
        </div>
        <p class="mt-4 text-sm leading-relaxed text-zinc-300">
          {$_(step.bodyKey)}
        </p>
      </div>
    {/key}

    <!-- Progress + controls sit outside the {#key} so they don't re-animate on
         every step; only the content above flies in. -->
    <div class="mt-4 flex items-center justify-center gap-1.5">
      {#each steps as _s, idx (idx)}
        <span
          class="h-1.5 rounded-full transition-all duration-300 {idx === i
            ? 'w-5 bg-emerald-500'
            : idx < i
              ? 'w-1.5 bg-emerald-500/70'
              : 'w-1.5 bg-zinc-700'}"
          aria-hidden="true"
        ></span>
      {/each}
    </div>

    <div class="mt-5 flex items-center justify-between gap-2">
      <button
        type="button"
        onclick={onClose}
        class="rounded-lg border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm font-medium text-red-300 transition hover:border-red-500/60 hover:bg-red-500/20 hover:text-red-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500/50"
      >
        {$_("tour.skip")}
      </button>
      <div class="flex items-center gap-2">
        {#if !isFirst}
          <Button variant="secondary" onclick={prev}>
            {$_("tour.back")}
          </Button>
        {/if}
        <Button variant="primary" size="lg" onclick={next}>
          {isLast ? $_("tour.start") : $_("tour.next")}
        </Button>
      </div>
    </div>
  </div>
</div>

<style>
  .tour-spotlight {
    position: fixed;
    border-radius: 10px;
    pointer-events: none;
    box-shadow:
      0 0 0 9999px rgba(9, 9, 11, 0.55),
      0 0 0 1.5px rgba(52, 211, 153, 0.9),
      0 0 0 6px rgba(16, 185, 129, 0.16),
      0 0 34px 6px rgba(16, 185, 129, 0.28);
    transition:
      top 520ms cubic-bezier(0.22, 1, 0.36, 1),
      left 520ms cubic-bezier(0.22, 1, 0.36, 1),
      width 520ms cubic-bezier(0.22, 1, 0.36, 1),
      height 520ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .tour-spotlight.no-motion {
    transition: none;
  }
</style>
