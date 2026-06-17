<script lang="ts">
  /**
   * Game cover thumbnail. Shows the Steam capsule (served from the on-device
   * cache) when one exists, otherwise a tinted square with the game's initial.
   * The image is loaded lazily via the `covers` store so the same app id is
   * fetched at most once per session.
   */
  import { coverUrl, steamIdForSlug } from "../stores/covers";

  let {
    appId = null,
    /** Game slug, used to recover the Steam app id from the catalog when
     *  `appId` is null (e.g. a save tracked on another device). */
    slug = null,
    name = "",
    /** Tailwind size + radius classes for the outer box. */
    class: klass = "h-10 w-10 rounded-lg",
    /** Font-size class for the fallback initial. */
    initialClass = "text-sm",
  }: {
    appId?: number | null;
    slug?: string | null;
    name?: string;
    class?: string;
    initialClass?: string;
  } = $props();

  const initial = $derived((name.trim().charAt(0) || "?").toUpperCase());
  let url = $state<string | null>(null);

  $effect(() => {
    url = null;
    const directId = appId;
    const s = slug;
    let alive = true;
    (async () => {
      // Prefer the id detection already resolved; otherwise recover it from the
      // catalog by slug so cross-device saves still get a cover.
      const id = directId ?? (s ? await steamIdForSlug(s) : null);
      if (id == null || !alive) return;
      const u = await coverUrl(id);
      if (alive) url = u;
    })();
    return () => {
      alive = false;
    };
  });
</script>

<div
  class={`relative shrink-0 overflow-hidden border border-white/[0.08] bg-zinc-800 ${klass}`}
>
  {#if url}
    <img src={url} alt={name} class="h-full w-full object-cover" draggable="false" />
  {:else}
    <div
      class={`flex h-full w-full items-center justify-center bg-gradient-to-br from-emerald-600/40 to-emerald-900/40 font-semibold text-emerald-100 ${initialClass}`}
    >
      {initial}
    </div>
  {/if}
</div>
