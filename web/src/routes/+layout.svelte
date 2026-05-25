<script lang="ts">
  import '../app.css';
  import Nav from '$lib/components/Nav.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import { waitLocale } from 'svelte-i18n';

  let ready = $state(false);
  waitLocale().then(() => (ready = true));

  interface Props {
    children: import('svelte').Snippet;
  }
  let { children }: Props = $props();
</script>

<div class="flex min-h-full flex-col bg-zinc-950 text-zinc-100">
  {#if ready}
    <Nav />
    <main class="flex-1">
      {@render children()}
    </main>
    <Footer />
  {/if}
</div>
