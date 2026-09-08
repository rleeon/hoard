<script lang="ts">
  import '../app.css';
  import Nav from '$lib/components/Nav.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import { onMount } from 'svelte';
  import { onNavigate } from '$app/navigation';
  import { smoothWheel } from '$lib/actions/smoothWheel';
  import { stripLocale } from '$lib/i18n/locales';

  interface Props {
    children: import('svelte').Snippet;
  }
  let { children }: Props = $props();

  // A language switch lands on the same page in another tongue, so the root
  // cross-fade of a view transition blends two near-identical screens and reads
  // as a blink. Those get their own treatment instead: the new copy settles in
  // from the top down, section by section, which is the only motion that says
  // "the words changed" rather than "the page flashed".
  let langFading = $state(false);
  let fadeGuard: ReturnType<typeof setTimeout>;

  onMount(() => smoothWheel());

  onNavigate((navigation) => {
    if (typeof document === 'undefined') return;
    const from = navigation.from?.url.pathname;
    const to = navigation.to?.url.pathname;
    if (from && to && stripLocale(from) === stripLocale(to) && from !== to) {
      // Fade the old copy out, hand control back while it is still fading so the
      // swap lands unseen, then let it come back. The fade survives
      // reduced-motion on purpose: it is a plain opacity dissolve, and without it
      // the change is a hard cut. Everything else still honours the preference.
      langFading = true;
      clearTimeout(fadeGuard);
      // A navigation can be abandoned halfway (another link, the back button, a
      // failed load) and then `complete` never settles. Without this the page
      // would sit at opacity 0.04 forever, which is to say invisible.
      fadeGuard = setTimeout(() => (langFading = false), 1500);
      const reveal = () => {
        clearTimeout(fadeGuard);
        requestAnimationFrame(() => (langFading = false));
      };
      return new Promise<void>((resolve) => {
        setTimeout(() => {
          resolve();
          navigation.complete.then(reveal, reveal);
        }, 110);
      });
    }
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

<!-- The fade wraps the whole shell, not just <main>: the bar and the footer
     carry translated text too, and leaving them out made them snap to the new
     language while the content was still dissolving. -->
<div
  class="lang-fade relative isolate flex min-h-full flex-col text-ink {langFading
    ? 'is-fading'
    : ''}"
  data-sveltekit-preload-data="hover"
>
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-accent-deep focus:px-3 focus:py-2 focus:text-sm focus:text-white"
  >
    Skip to content
  </a>
  <Nav />
  <main id="main" class="relative z-10 flex-1">
    {@render children()}
  </main>
  <Footer />
</div>
