<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { supabase } from '$lib/auth/supabase';

  let message = $state('Signing you in…');
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const { data, error: e } = await supabase.auth.getSession();
      if (e) throw e;
      if (data.session) {
        const next = $page.url.searchParams.get('next') ?? '/account';
        goto(next, { replaceState: true });
        return;
      }
      // Supabase JS handles detectSessionInUrl automatically; give it a moment
      setTimeout(async () => {
        const { data: d2 } = await supabase.auth.getSession();
        if (d2.session) {
          const next = $page.url.searchParams.get('next') ?? '/account';
          goto(next, { replaceState: true });
        } else {
          error = 'Could not complete sign-in. Try again.';
          message = '';
        }
      }, 500);
    } catch (err) {
      error = (err as Error).message;
      message = '';
    }
  });
</script>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-32 text-center sm:px-6">
  {#if error}
    <h1 class="text-2xl font-semibold text-white">Sign-in failed</h1>
    <p class="mt-2 text-sm text-zinc-400">{error}</p>
    <a href="/login" class="mt-6 text-sm text-emerald-400 hover:underline">Back to sign-in</a>
  {:else}
    <div class="h-10 w-10 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent"></div>
    <p class="mt-6 text-sm text-zinc-400">{message}</p>
  {/if}
</section>
