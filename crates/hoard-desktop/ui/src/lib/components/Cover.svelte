<script lang="ts">
  /**
   * Game cover thumbnail. Shows the downloaded art (served from the on-device
   * cache) when one exists, otherwise a tinted box with the game's initial.
   * The image is loaded lazily via the `covers` store so the same game is
   * fetched at most once per session.
   *
   * Users can override the cover with a custom local image. On hover a pencil
   * icon appears; clicking it opens a file picker. If a custom cover is set,
   * the pencil changes to a "restore" icon to revert to the downloaded art.
   * The pencil is offered for **every** game, including the ones no CDN has
   * art for, that is the only way a game like Minecraft Java ever gets a
   * cover, and it used to be the one case where the button never appeared.
   *
   * Two knobs for how the image meets its frame:
   *
   * - `fit="cover"` (default) fills the box, center-cropping the overflow.
   *   Right for the small square thumbnails in Library / Map, where the frame
   *   is tiny and letterboxing would leave a sliver of art.
   * - `fit="smart"` still fills the box when the image roughly matches its
   *   shape, but letterboxes (over a blurred blow-up of itself) when it
   *   clearly doesn't. That's the poster case: Rust now prefers Steam's 2:3
   *   `library_600x900`, yet games with no vertical art still fall back to the
   *   460×215 header, and cropping that to a poster throws ~70% of it away.
   */
  import { _ } from "svelte-i18n";
  import { Pencil, RotateCcw } from "@lucide/svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    coverKey,
    coverUrl,
    hasCustomCover,
    setCustomCover,
    removeCustomCover,
  } from "../stores/covers";
  import { tilt } from "../actions/tilt";

  let {
    appId = null,
    /** Game slug, the primary identity for cover art. Rust resolves it to
     *  real art through the catalog, our hosted index, or a store search. */
    slug = null,
    name = "",
    /** Tailwind size + radius classes for the outer box. */
    class: klass = "h-10 w-10 rounded-lg",
    /** Font-size class for the fallback initial. */
    initialClass = "text-sm",
    /** How the image meets the frame, see the note above. */
    fit = "cover",
    /** `overlay` dims the whole thumbnail on hover (fine at 40px); `corner`
     *  puts a small button in the bottom-right instead, so a big poster stays
     *  visible while you reach for the pencil. */
    editor = "overlay",
  }: {
    appId?: number | null;
    slug?: string | null;
    name?: string;
    class?: string;
    initialClass?: string;
    fit?: "cover" | "smart";
    editor?: "overlay" | "corner";
  } = $props();

  const initial = $derived((name.trim().charAt(0) || "?").toUpperCase());
  let url = $state<string | null>(null);
  let hovered = $state(false);
  let isCustom = $state(false);
  let resolvedKey = $state<string | null>(null);

  // Natural size of the loaded image + measured size of the frame. Both are
  // needed to tell "portrait art in a portrait frame" (fill it) from "landscape
  // capsule in a portrait frame" (letterbox it).
  let imgRatio = $state<number | null>(null);
  let boxW = $state(0);
  let boxH = $state(0);

  /**
   * True when image and frame disagree enough that cropping would eat the art.
   * The 40% threshold is calibrated against the four shapes that actually turn
   * up, measured as |imgRatio − boxRatio| / boxRatio:
   *
   *   2:3 art  in a 2:3 frame →   0%  fill
   *   2:3 art  in a square    →  33%  fill (a mild top/bottom crop)
   *   square   in a 2:3 frame →  50%  letterbox, custom square art, the case
   *                                   the square option exists for
   *   header   in either      → 114%+ letterbox, the 460×215 capsule
   */
  const letterbox = $derived.by(() => {
    if (fit !== "smart" || imgRatio == null || boxW === 0 || boxH === 0)
      return false;
    const box = boxW / boxH;
    return Math.abs(imgRatio - box) / box > 0.4;
  });

  function onImgLoad(e: Event) {
    const img = e.currentTarget as HTMLImageElement;
    imgRatio = img.naturalHeight > 0 ? img.naturalWidth / img.naturalHeight : null;
  }

  $effect(() => {
    url = null;
    imgRatio = null;
    isCustom = false;
    // The key is known synchronously, no await, no network. That's what lets
    // the pencil appear for a game with no art at all: the old code resolved a
    // Steam app id first and bailed when there wasn't one, so Minecraft Java
    // couldn't even be given a cover by hand.
    const key = coverKey(appId, slug);
    resolvedKey = key;
    if (key == null) return;
    let alive = true;
    (async () => {
      const [u, custom] = await Promise.all([coverUrl(key), hasCustomCover(key)]);
      if (alive) {
        url = u;
        isCustom = custom;
      }
    })();
    return () => {
      alive = false;
    };
  });

  async function pickCover(e: MouseEvent) {
    e.stopPropagation();
    const key = resolvedKey;
    if (key == null) return;
    try {
      const file = await openDialog({
        multiple: false,
        filters: [
          {
            name: "Images",
            extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp"],
          },
        ],
      });
      if (typeof file === "string" && file.length > 0) {
        await setCustomCover(key, file);
        // Reload the cover.
        url = null;
        imgRatio = null;
        url = await coverUrl(key);
        isCustom = true;
      }
    } catch {
      // User cancelled or file read error, ignore silently.
    }
  }

  async function restoreOriginal(e: MouseEvent) {
    e.stopPropagation();
    const key = resolvedKey;
    if (key == null) return;
    await removeCustomCover(key);
    // Reload the cover.
    url = null;
    imgRatio = null;
    url = await coverUrl(key);
    isCustom = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={`tilt group relative shrink-0 overflow-hidden border border-white/[0.08] bg-zinc-800 ${klass}`}
  use:tilt
  bind:clientWidth={boxW}
  bind:clientHeight={boxH}
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  {#if url}
    {#if letterbox}
      <!-- Fill the dead space with a blurred blow-up of the same art instead
           of a flat bar: the frame stays a solid block of colour and the real
           image sits on top, whole. -->
      <img
        src={url}
        alt=""
        aria-hidden="true"
        class="absolute inset-0 h-full w-full scale-110 object-cover blur-xl saturate-150"
        draggable="false"
      />
      <div class="absolute inset-0 bg-zinc-950/40" aria-hidden="true"></div>
    {/if}
    <img
      src={url}
      alt={name}
      onload={onImgLoad}
      class={`relative h-full w-full ${letterbox ? "object-contain" : "object-cover"}`}
      draggable="false"
    />
  {:else}
    <div
      class={`flex h-full w-full items-center justify-center bg-gradient-to-br from-emerald-600/40 to-emerald-900/40 font-semibold text-emerald-100 ${initialClass}`}
    >
      {initial}
    </div>
  {/if}

  {#if resolvedKey != null && hovered}
    {@const isRestore = isCustom}
    {@const label = isRestore ? $_("covers.restore") : $_("covers.change")}
    <button
      type="button"
      onclick={isRestore ? restoreOriginal : pickCover}
      title={label}
      aria-label={label}
      class={editor === "corner"
        ? "absolute bottom-2 right-2 z-10 flex h-7 w-7 items-center justify-center rounded-md border border-white/[0.12] bg-zinc-950/70 text-zinc-100 backdrop-blur-md transition-colors hover:border-white/[0.24] hover:bg-zinc-950/90"
        : "absolute inset-0 z-10 flex items-center justify-center bg-black/50 text-white transition-opacity"}
    >
      {#if isRestore}
        <RotateCcw class="h-3.5 w-3.5" />
      {:else}
        <Pencil class="h-3.5 w-3.5" />
      {/if}
    </button>
  {/if}
</div>
