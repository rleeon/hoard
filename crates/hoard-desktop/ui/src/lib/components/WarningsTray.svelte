<script lang="ts">
  /**
   * The warnings about tracked folders, folded behind one line.
   *
   * Each one used to be a banner of its own at the top of the panel, and a
   * library with a few installs tracked by hand opened on a wall of amber (seven
   * of them for six games) before a single card showed. They are all still
   * here, one click away; the line only says how many, and opened, the box it
   * heads holds every card so they read as one thing.
   *
   * The banners stay mounted while folded, so their "not now" and the count
   * they report survive opening and closing.
   */
  import { AlertTriangle, ChevronDown, ChevronUp } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import FolderWarningBanner from "./FolderWarningBanner.svelte";
  import LinkWarningBanner from "./LinkWarningBanner.svelte";
  import MirrorWarningBanner from "./MirrorWarningBanner.svelte";
  import { glow } from "../actions/glow";
  import type { FolderWarning, LinkWarning, MirrorWarning } from "../api";

  type Props = {
    mirror: MirrorWarning[];
    links: LinkWarning[];
    folders: FolderWarning[];
    footprints?: Record<string, number>;
    onMirrorFixed?: () => void;
    onFolderFixed?: () => void;
  };

  let {
    mirror,
    links,
    folders,
    footprints = {},
    onMirrorFixed,
    onFolderFixed,
  }: Props = $props();

  let open = $state(false);
  let box: HTMLDivElement;
  let clip: HTMLDivElement;
  let inner: HTMLDivElement;
  let shownMirror = $state(0);
  let shownLinks = $state(0);
  let shownFolders = $state(0);
  const count = $derived(shownMirror + shownLinks + shownFolders);

  // Pixel heights rather than `grid-template-rows: 0fr -> 1fr`: WebKitGTK does
  // not interpolate grid tracks, so the tray jumped open there and only the fade
  // showed. Open ends at `auto` so a dismissed card still lets it shrink.
  function toggle() {
    const full = `${inner.scrollHeight}px`;
    if (open) {
      clip.style.height = full;
      // Commit the start, or going from `auto` to 0 has nothing to animate from.
      clip.getBoundingClientRect();
      clip.style.height = "0px";
    } else {
      clip.style.height = full;
    }
    open = !open;
  }

  // Closed from the bottom of a long list, the header would end up above the
  // window with nothing on screen saying where the cards went.
  function closeFromBottom() {
    toggle();
    box.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }

  function settled(e: TransitionEvent) {
    if (e.target === clip && e.propertyName === "height" && open) {
      clip.style.height = "auto";
    }
  }
</script>

<div
  bind:this={box}
  class="warnings-box mb-5 rounded-lg border border-amber-500/30 bg-amber-500/[0.05]"
  class:hidden={count === 0}
>
  <button
    type="button"
    use:glow
    class="glow warnings-head flex w-full items-center gap-2.5 rounded-[7px] px-3.5 py-2 text-left text-[13px] text-amber-200 transition-all duration-200 hover:bg-amber-500/[0.16] hover:text-amber-100 active:scale-[0.985]"
    aria-expanded={open}
    aria-controls="warnings-tray"
    onclick={toggle}
  >
    <AlertTriangle size={14} class="shrink-0 text-amber-400" data-anim="ring" />
    <span class="min-w-0 flex-1 font-medium">
      {$_("warnings.summary", { values: { count } })}
    </span>
    <span
      class="flex size-6 shrink-0 items-center justify-center rounded-md bg-amber-500/10 text-amber-300"
      data-anim="pop"
      title={$_(open ? "warnings.collapse" : "warnings.expand")}
    >
      <ChevronDown
        size={14}
        class="warnings-chevron {open ? 'rotate-180' : ''}"
      />
    </span>
  </button>

  <div
    id="warnings-tray"
    class="warnings-tray overflow-hidden"
    style="height: 0px"
    inert={!open}
    bind:this={clip}
    ontransitionend={settled}
  >
    <div bind:this={inner}>
      <div class="warnings-tray-body px-2.5 pb-1.5 pt-1" class:is-open={open}>
        <div>
          <MirrorWarningBanner
            warnings={mirror}
            {footprints}
            onFixed={onMirrorFixed}
            bind:shown={shownMirror}
          />
          <LinkWarningBanner warnings={links} bind:shown={shownLinks} />
          <FolderWarningBanner
            warnings={folders}
            onFixed={onFolderFixed}
            bind:shown={shownFolders}
          />
        </div>
        <div class="tray-card tray-close mt-1.5">
          <button
            type="button"
            use:glow
            class="glow warnings-foot flex w-full items-center justify-center gap-1.5 rounded-md border border-transparent py-1.5 text-xs text-amber-200/70 transition-all duration-200 hover:-translate-y-0.5 hover:border-amber-500/45 hover:bg-amber-500/[0.15] hover:text-amber-100 hover:shadow-[0_10px_26px_-14px_rgb(245_158_11/0.55)] active:scale-[0.97]"
            onclick={closeFromBottom}
          >
            <ChevronUp size={13} data-anim="pop" />
            {$_("warnings.collapse")}
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  /* In and out alike: an ease-out curve gave away a third of the height in the
     first frame, and the tray read as jumping open. */
  .warnings-tray {
    transition: height var(--tray-dur, 650ms) cubic-bezier(0.45, 0, 0.25, 1);
  }
  /* The cards rise into place one after another as the tray opens, with the
     overshoot the icons pop with. Closing takes them all at once. */
  .warnings-tray-body :global(.tray-card) {
    opacity: 0;
    transform: translateY(8px) scale(0.985);
    transition:
      opacity 320ms ease var(--rise-delay, 0ms),
      transform 520ms cubic-bezier(0.34, 1.56, 0.64, 1) var(--rise-delay, 0ms),
      translate 200ms ease,
      background-color 200ms ease,
      border-color 200ms ease,
      box-shadow 200ms ease;
  }
  .warnings-tray-body.is-open :global(.tray-card) {
    opacity: 1;
    transform: none;
  }
  .warnings-tray-body.is-open :global(.tray-card:nth-child(2)) {
    --rise-delay: 50ms;
  }
  .warnings-tray-body.is-open :global(.tray-card:nth-child(3)) {
    --rise-delay: 100ms;
  }
  .warnings-tray-body.is-open :global(.tray-card:nth-child(4)) {
    --rise-delay: 150ms;
  }
  .warnings-tray-body.is-open :global(.tray-card:nth-child(5)) {
    --rise-delay: 200ms;
  }
  .warnings-tray-body.is-open :global(.tray-card:nth-child(n + 6)) {
    --rise-delay: 250ms;
  }
  /* The highlight under the pointer at full strength: at the half every button
     uses it all but vanished on amber. */
  :global(.tray-card.glow:hover::after),
  .warnings-head:hover::after,
  .warnings-foot:hover::after {
    opacity: 1;
  }
  /* Pointing at the header lights the whole box it heads. */
  .warnings-box {
    transition:
      border-color 200ms ease,
      box-shadow 200ms ease;
  }
  .warnings-box:has(> .warnings-head:hover) {
    border-color: rgb(245 158 11 / 0.55);
    box-shadow: 0 10px 28px -14px rgb(245 158 11 / 0.55);
  }
  /* As specific as the `nth-child` rules above, and after them, so it wins. */
  .warnings-tray-body.is-open :global(.tray-card.tray-close) {
    --rise-delay: 300ms;
  }
  :global(.warnings-chevron) {
    transition: transform var(--tray-dur, 650ms) cubic-bezier(0.45, 0, 0.25, 1);
  }
</style>
