<script lang="ts">
  import '../app.css';
  import Nav from '$lib/components/Nav.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import { onNavigate } from '$app/navigation';

  interface Props {
    children: import('svelte').Snippet;
  }
  let { children }: Props = $props();

  // Smooth view transitions when the browser supports it. No-op elsewhere.
  onNavigate((navigation) => {
    if (typeof document === 'undefined') return;
    const doc = document as Document & {
      startViewTransition?: (cb: () => Promise<void> | void) => { finished: Promise<void> };
    };
    if (!doc.startViewTransition) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    return new Promise<void>((resolve) => {
      doc.startViewTransition!(async () => {
        resolve();
        await navigation.complete;
      });
    });
  });
</script>

<div
  class="relative isolate flex min-h-full flex-col text-zinc-100"
  data-sveltekit-preload-data="hover"
>
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-emerald-600 focus:px-3 focus:py-2 focus:text-sm focus:text-white"
  >
    Skip to content
  </a>
  <Nav />
  <main id="main" class="relative z-10 flex-1">
    {@render children()}
  </main>
  <Footer />
</div>
