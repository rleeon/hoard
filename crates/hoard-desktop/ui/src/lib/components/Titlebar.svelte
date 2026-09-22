<script lang="ts">
  /**
   * The window's own title bar, on Windows and Linux.
   *
   * Both draw a light caption with square corners that sits on a near-black app
   * like a strip of tape. Without it the bar is ours: same black, same edge, and
   * the buttons keep the geometry everyone already knows (46 x 32, close goes
   * red on hover), because a title bar is the one place where being inventive
   * only makes people miss.
   *
   * `data-tauri-drag-region` is what moves the window, and it also gives us the
   * double-click to maximise for free. What it does not give back is the resize
   * border, hence the invisible grips below.
   *
   * Which bar is mounted at all is Rust's call, asked for in `main.ts`.
   */
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import { AlertCircle, Bell, Eye, EyeOff, Scroll, ScrollText } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import AnimIcon from "./AnimIcon.svelte";
  import Logo from "./Logo.svelte";
  import MarioStar from "./MarioStar.svelte";
  import { notifications } from "../stores/notifications";
  import {
    eyeOpen,
    notifOpen,
    openRepoPage,
    toggleEye,
    toggleLiveActivity,
    toggleNotif,
    updatePromptOpen,
  } from "../stores/panels";
  import { prefs } from "../stores/prefs";
  import { isWindows } from "../os";
  import { lastReport } from "../stores/updates";
  import { tapVersion } from "../stores/versionTap";
  import { APP_VERSION } from "../version";

  const win = getCurrentWindow();
  /** Three things differ between the two platforms: the blur, which WebKitGTK
   *  paints as a black rectangle the moment anything hovers over it (the bug
   *  that cost the sidebar its glass), how thick the resize grips can be, and
   *  who draws the window's edge. */
  const windows = isWindows();

  const activityVisible = $derived($prefs?.live_activity_visible ?? true);
  const hasUpdate = $derived(
    !!($lastReport && ($lastReport.client.available || $lastReport.server?.available)),
  );
  let maximized = $state(false);

  onMount(() => {
    let dispose: (() => void) | null = null;
    void win.isMaximized().then((v) => (maximized = v));
    void win
      .onResized(() => {
        void win.isMaximized().then((v) => (maximized = v));
      })
      .then((un) => (dispose = un));
    return () => dispose?.();
  });

  /** The eight grips that replace the frame the decoration took with it. `dir`
   *  is the direction as `startResizeDragging` names it.
   *
   *  Windows keeps the one-pixel filo it has always had: the engine's scrollbar
   *  sits flush against the right edge and a thicker grip eats the last pixels
   *  of its thumb. Linux gets 4 px because there the edge is all there is, and
   *  tao's own 5 px border for undecorated windows never gets a look in: the
   *  webview takes the press first. */
  const GRIPS = windows
    ? ([
        { dir: "North", class: "left-2 right-2 top-0 h-1 cursor-ns-resize" },
        { dir: "South", class: "bottom-0 left-2 right-2 h-1 cursor-ns-resize" },
        { dir: "West", class: "bottom-2 left-0 top-2 w-1 cursor-ew-resize" },
        { dir: "East", class: "bottom-2 right-0 top-2 w-1 cursor-ew-resize" },
        { dir: "NorthWest", class: "left-0 top-0 h-2 w-2 cursor-nwse-resize" },
        { dir: "NorthEast", class: "right-0 top-0 h-2 w-2 cursor-nesw-resize" },
        { dir: "SouthWest", class: "bottom-0 left-0 h-2 w-2 cursor-nesw-resize" },
        { dir: "SouthEast", class: "bottom-0 right-0 h-2 w-2 cursor-nwse-resize" },
      ] as const)
    : ([
        { dir: "North", class: "left-3 right-3 top-0 h-[4px] cursor-ns-resize" },
        { dir: "South", class: "bottom-0 left-3 right-3 h-[4px] cursor-ns-resize" },
        { dir: "West", class: "bottom-3 left-0 top-3 w-[4px] cursor-ew-resize" },
        { dir: "East", class: "bottom-3 right-0 top-3 w-[4px] cursor-ew-resize" },
        { dir: "NorthWest", class: "left-0 top-0 h-3 w-3 cursor-nwse-resize" },
        { dir: "NorthEast", class: "right-0 top-0 h-3 w-3 cursor-nesw-resize" },
        { dir: "SouthWest", class: "bottom-0 left-0 h-3 w-3 cursor-nesw-resize" },
        { dir: "SouthEast", class: "bottom-0 right-0 h-3 w-3 cursor-nwse-resize" },
      ] as const);

  function grip(e: MouseEvent, dir: (typeof GRIPS)[number]["dir"]) {
    if (e.button !== 0) return;
    // The top grips overlap the bar, and the bar is a drag region: without this
    // the press reaches Tauri's own handler too and the window is moved instead
    // of resized, which is why the top edge did nothing on Linux.
    e.preventDefault();
    e.stopPropagation();
    // The cast keeps this readable: the enum is a plain string union on the
    // wire and the API accepts it as such.
    void win.startResizeDragging(dir as never);
  }
</script>

<!-- Sits above everything, including modals: a dialog must never cover the way
     to close the window. -->
<div
  data-tauri-drag-region
  class="fixed inset-x-0 top-0 z-[400] flex h-8 select-none items-center justify-between border-b border-white/[0.08] bg-layer-3 pl-3 pr-1"
  class:backdrop-blur-xl={windows}
>
  <span
    data-tauri-drag-region
    class="pointer-events-none flex items-center gap-2 text-zinc-500"
  >
    <Logo size={15} mono class="opacity-90" />
    <span class="translate-y-[2px] text-[13px] font-medium tracking-wide">Hoard</span>
    <!-- The version lives here whenever the bar is ours, because the sidebar
         drops its brand row then. Five quick taps unlock diagnostics, the same
         gesture the sidebar's version has. -->
    <button
      type="button"
      tabindex="-1"
      onclick={() => tapVersion($_("diagnostics.unlocked_toast"))}
      class="pointer-events-auto translate-y-[4px] cursor-default select-none text-[11px] text-zinc-600 outline-none"
    >
      v{APP_VERSION}
    </button>
    {#if hasUpdate}
      <button
        type="button"
        onclick={() => updatePromptOpen.set(true)}
        title={$lastReport?.client.available
          ? $_("updates.client_available", {
              values: { latest: $lastReport?.client.latest ?? "?" },
            })
          : $_("updates.server_available", {
              values: { latest: $lastReport?.server?.latest ?? "?" },
            })}
        aria-label={$_("updates.button_label")}
        class="pointer-events-auto ml-1 flex h-5 items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/10 px-1.5 text-amber-300 transition-colors hover:bg-amber-500/20"
      >
        <AnimIcon icon={AlertCircle} on={hasUpdate} kind="pop" size={12} />
      </button>
    {/if}
  </span>

  <!-- Not the Windows geometry (46 x 32 filled to the edges): a rounded pill
       that lights up under the pointer, which is what the rest of the app does
       with its icon buttons. The glyphs are drawn here rather than taken from
       the icon set so they can be thicker and round-capped; at 10 px a hairline
       stroke reads as a smudge. -->
  <div class="flex items-center gap-0.5">
    <!-- `pop` rather than the star's usual `hop`: the jump goes up and the bar
         is 32 px tall, so it left the window. -->
    <button
      type="button"
      onclick={openRepoPage}
      aria-label={$_("github.star_label")}
      title={$_("github.star_label")}
      class="flex h-7 w-9 items-center justify-center rounded-lg text-yellow-400 ring-1 ring-transparent transition-colors hover:bg-white/[0.07] hover:ring-white/[0.10]"
    >
      <AnimIcon icon={MarioStar} on={false} kind="pop" size={14} />
    </button>
    <button
      type="button"
      onclick={toggleLiveActivity}
      aria-label={$_("activity.toggle_label")}
      title={$_("activity.toggle_label")}
      class="flex h-7 w-9 items-center justify-center rounded-lg ring-1 transition-colors {activityVisible
        ? 'bg-white/[0.07] text-emerald-400 ring-white/[0.10]'
        : 'text-zinc-400 ring-transparent hover:bg-white/[0.07] hover:text-zinc-100 hover:ring-white/[0.10]'}"
    >
      <AnimIcon icon={ScrollText} iconOff={Scroll} on={activityVisible} kind="unfurl" size={14} />
    </button>
    <!-- The bell and the eye, where every window keeps its global controls: next
         to the window buttons, not floating over the page. Same pill, same
         moves they had before (the bell rings, the eye blinks). -->
    <button
      type="button"
      onclick={toggleNotif}
      aria-label={$_("notifications.title")}
      aria-expanded={$notifOpen}
      class="relative flex h-7 w-9 items-center justify-center rounded-lg ring-1 transition-colors {$notifOpen
        ? 'bg-white/[0.07] text-emerald-400 ring-white/[0.10]'
        : 'text-zinc-400 ring-transparent hover:bg-white/[0.07] hover:text-zinc-100 hover:ring-white/[0.10]'}"
    >
      <AnimIcon icon={Bell} on={$notifOpen} kind="ring" size={14} />
      {#if $notifications.length > 0}
        <span
          class="absolute right-1 top-0.5 flex h-3 min-w-3 items-center justify-center rounded-full bg-emerald-500 px-1 text-[9px] font-bold text-emerald-950"
        >
          {$notifications.length}
        </span>
      {/if}
    </button>
    <button
      type="button"
      onclick={toggleEye}
      aria-label={$_("eye.title")}
      aria-expanded={$eyeOpen}
      class="flex h-7 w-9 items-center justify-center rounded-lg ring-1 transition-colors {$eyeOpen
        ? 'bg-white/[0.07] text-emerald-400 ring-white/[0.10]'
        : 'text-zinc-400 ring-transparent hover:bg-white/[0.07] hover:text-zinc-100 hover:ring-white/[0.10]'}"
    >
      <AnimIcon icon={Eye} iconOff={EyeOff} on={$eyeOpen} kind="pop" size={14} />
    </button>
    <span class="mx-1.5 h-4 w-px bg-white/[0.08]" aria-hidden="true"></span>
    <button
      type="button"
      class="group flex h-7 w-9 items-center justify-center rounded-lg text-zinc-400 ring-1 ring-transparent transition-colors hover:bg-white/[0.07] hover:text-zinc-100 hover:ring-white/[0.10]"
      aria-label={$_("titlebar.minimize")}
      onclick={() => win.minimize()}
    >
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true" data-anim="pop">
        <rect x="2.6" y="5.3" width="7.8" height="2.4" rx="1.2" fill="currentColor" />
      </svg>
    </button>
    <button
      type="button"
      class="flex h-7 w-9 items-center justify-center rounded-lg text-zinc-400 ring-1 ring-transparent transition-colors hover:bg-white/[0.07] hover:text-zinc-100 hover:ring-white/[0.10]"
      aria-label={maximized ? $_("titlebar.restore") : $_("titlebar.maximize")}
      onclick={() => win.toggleMaximize()}
    >
      {#if maximized}
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" data-anim="pop">
          <path
            d="M5 3.6h4.4c1.1 0 2 .9 2 2V9"
            stroke="currentColor"
            stroke-width="2.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            fill-rule="evenodd"
            clip-rule="evenodd"
            d="M2 7.2a2.4 2.4 0 012.4-2.4h3.8a2.4 2.4 0 012.4 2.4v2.4a2.4 2.4 0 01-2.4 2.4H4.4A2.4 2.4 0 012 9.6V7.2zm2.4-.4a.4.4 0 00-.4.4v2.4c0 .22.18.4.4.4h3.8a.4.4 0 00.4-.4V7.2a.4.4 0 00-.4-.4H4.4z"
            fill="currentColor"
          />
        </svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" data-anim="pop">
          <path
            fill-rule="evenodd"
            clip-rule="evenodd"
            d="M2 5.4A2.6 2.6 0 014.6 2.8h4.8A2.6 2.6 0 0112 5.4v3.2a2.6 2.6 0 01-2.6 2.6H4.6A2.6 2.6 0 012 8.6V5.4zm2.6-.5a.5.5 0 00-.5.5v3.2c0 .28.22.5.5.5h4.8a.5.5 0 00.5-.5V5.4a.5.5 0 00-.5-.5H4.6z"
            fill="currentColor"
          />
        </svg>
      {/if}
    </button>
    <button
      type="button"
      class="flex h-7 w-9 items-center justify-center rounded-lg text-zinc-400 ring-1 ring-transparent transition-colors hover:bg-red-600 hover:text-white hover:ring-red-400/40"
      aria-label={$_("titlebar.close")}
      onclick={() => win.close()}
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" data-anim="spin">
        <rect
          x="2.4"
          y="5.7"
          width="9.2"
          height="2.6"
          rx="1.3"
          fill="currentColor"
          transform="rotate(45 7 7)"
        />
        <rect
          x="2.4"
          y="5.7"
          width="9.2"
          height="2.6"
          rx="1.3"
          fill="currentColor"
          transform="rotate(-45 7 7)"
        />
      </svg>
    </button>
  </div>
</div>

<!-- The edge the decoration took with it. Windows keeps drawing a 1 px border
     of its own around an undecorated window; Linux draws nothing, and a
     near-black app on a dark desktop then has no visible edge at all. Above the
     bar and the grips (400 and 401), which would otherwise paint over its top
     edge, and it takes no clicks, so the grips underneath still get theirs.
     Gone while maximised, where there is no edge to mark. -->
{#if !windows && !maximized}
  <div class="pointer-events-none fixed inset-0 z-[402] border border-white/[0.22]"></div>
{/if}

{#each GRIPS as g (g.dir)}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed z-[401] {g.class}"
    onmousedown={(e) => grip(e, g.dir)}
  ></div>
{/each}
