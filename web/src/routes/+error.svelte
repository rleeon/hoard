<script lang="ts">
  import { page } from '$app/stores';
  import { reloadAfterChunkError } from '$lib/chunkReload';

  // A chunk that failed to download says nothing about the page, so it reloads
  // instead of painting "500 Internal Error" (this runs before the first paint).
  // Offline, the error stays up and the reload waits for the connection.
  let reloading = $state(false);
  let handled: App.Error | null = null;
  $effect.pre(() => {
    const err = $page.error;
    if (!err?.chunk || err === handled) return;
    handled = err;
    reloading = reloadAfterChunkError() && navigator.onLine;
  });
</script>

{#if !reloading}
  <section class="mx-auto max-w-4xl px-4 py-24 text-center sm:px-6 sm:py-32">
    <h1 class="text-5xl font-semibold text-ink sm:text-6xl">{$page.status}</h1>
    <p class="mt-4 text-lg text-ink-soft">{$page.error?.message}</p>
  </section>
{/if}
