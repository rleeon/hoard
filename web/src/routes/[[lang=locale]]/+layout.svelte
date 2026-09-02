<script lang="ts">
  import { page } from '$app/stores';
  import { locale } from 'svelte-i18n';
  import { DEFAULT_LOCALE } from '$lib/i18n/locales';

  let { children } = $props();

  // The URL prefix is the source of truth for the language of these marketing
  // pages. The universal `+layout.ts` load sets the locale during SSR/prerender
  // (so the prerendered HTML comes out right), but it does NOT re-run on client
  // hydration, SvelteKit reuses the serialized load data, so its `locale.set`
  // side-effect is skipped. Without this, the client keeps whatever locale
  // `setupI18n` seeded from localStorage (the last language the visitor used),
  // showing e.g. Spanish on /en/guides. Re-assert the route locale on the
  // client whenever the prefix changes.
  $effect(() => {
    const lang = $page.params.lang ?? DEFAULT_LOCALE;
    if ($locale !== lang) locale.set(lang);
  });
</script>

{@render children()}
