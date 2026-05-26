<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { auth } from '$lib/auth';
  import { session } from '$lib/stores/session';
  import Button from '$lib/components/Button.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';

  let email = $state('');
  let sent = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  let next = $derived($page.url.searchParams.get('next') ?? '/account');
  let redirectTo = $derived(
    typeof window !== 'undefined' ? `${window.location.origin}/auth/callback?next=${encodeURIComponent(next)}` : ''
  );

  onMount(() => {
    if ($session) goto(next);
  });

  $effect(() => {
    if ($session) goto(next);
  });

  async function withGoogle() {
    error = null;
    busy = true;
    try {
      await auth.signInWithGoogle(redirectTo);
    } catch (e) {
      error = (e as Error).message;
      busy = false;
    }
  }

  async function withEmail(e: SubmitEvent) {
    e.preventDefault();
    if (!email) return;
    error = null;
    busy = true;
    try {
      await auth.signInWithEmail(email, redirectTo);
      sent = true;
    } catch (err) {
      error = (err as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-20 sm:px-6">
  <LogoMark size={44} />
  <h1 class="mt-4 text-3xl font-bold tracking-tight text-white">{$_('login.title')}</h1>
  <p class="mt-2 text-sm text-zinc-400">{$_('login.subtitle')}</p>

  <div class="mt-10 w-full space-y-5">
    <button
      class="flex w-full items-center justify-center gap-3 rounded-lg border border-zinc-700 bg-white px-4 py-3 text-sm font-medium text-zinc-900 transition-transform hover:-translate-y-px disabled:opacity-50"
      onclick={withGoogle}
      disabled={busy}
    >
      <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
        <path
          fill="#4285F4"
          d="M17.64 9.2c0-.64-.06-1.25-.16-1.84H9v3.48h4.84a4.14 4.14 0 0 1-1.8 2.72v2.26h2.92c1.71-1.57 2.68-3.88 2.68-6.62z"
        />
        <path
          fill="#34A853"
          d="M9 18c2.43 0 4.47-.81 5.96-2.18l-2.92-2.26c-.81.54-1.84.86-3.04.86-2.34 0-4.32-1.58-5.03-3.7H.96v2.32A9 9 0 0 0 9 18z"
        />
        <path
          fill="#FBBC05"
          d="M3.97 10.71a5.41 5.41 0 0 1 0-3.42V4.96H.96a9 9 0 0 0 0 8.08l3-2.33z"
        />
        <path
          fill="#EA4335"
          d="M9 3.58c1.32 0 2.5.45 3.44 1.35l2.58-2.58A9 9 0 0 0 9 0a9 9 0 0 0-8.04 4.96l3 2.33C4.68 5.16 6.66 3.58 9 3.58z"
        />
      </svg>
      {$_('login.google')}
    </button>

    <div class="flex items-center gap-3">
      <div class="h-px flex-1 bg-zinc-800"></div>
      <span class="text-xs uppercase tracking-wider text-zinc-500">{$_('login.email_label')}</span>
      <div class="h-px flex-1 bg-zinc-800"></div>
    </div>

    {#if sent}
      <div class="rounded-xl border border-emerald-700/40 bg-emerald-900/20 p-4 text-sm text-emerald-200">
        {$_('login.email_sent', { values: { email } })}
      </div>
    {:else}
      <form class="space-y-3" onsubmit={withEmail}>
        <input
          type="email"
          required
          bind:value={email}
          placeholder={$_('login.email_placeholder')}
          class="w-full rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-2.5 text-sm text-white placeholder:text-zinc-500 focus:border-emerald-600 focus:outline-none focus:ring-2 focus:ring-emerald-600/30"
        />
        <Button type="submit" variant="primary" full disabled={busy}>
          {$_('login.email_cta')}
        </Button>
      </form>
    {/if}

    {#if error}
      <p class="text-sm text-red-400">{error}</p>
    {/if}

    <p
      class="text-center text-xs text-zinc-500 [&_a]:text-emerald-400 [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-emerald-300"
    >
      {@html $_('login.terms_html')}
    </p>
  </div>
</section>
