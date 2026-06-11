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
  // Desktop login handoff: the Hoard app opens this page with ?desktop=1 and
  // expects the session to bounce back via the `hoard://` deep link instead of
  // staying in the browser. We carry the flag through to /auth/callback, which
  // does the actual redirect to the app.
  let desktop = $derived($page.url.searchParams.get('desktop') === '1');
  // Loopback port the desktop app is listening on for the OAuth handoff. When
  // present we bounce the session to http://127.0.0.1:<port> (confined browsers
  // like Ubuntu's snap Firefox can open that) instead of the custom hoard://
  // scheme (which they silently drop). Carried through to /auth/callback.
  let port = $derived($page.url.searchParams.get('port') ?? '');
  let dlExtra = $derived(
    `${desktop ? '&desktop=1' : ''}${port ? `&port=${encodeURIComponent(port)}` : ''}`
  );
  let redirectTo = $derived(
    typeof window !== 'undefined'
      ? `${window.location.origin}/auth/callback?next=${encodeURIComponent(next)}${dlExtra}`
      : ''
  );

  // Where to send an already-authenticated browser. For desktop we route to the
  // callback (which bounces to the app); otherwise straight to `next`.
  let postLogin = $derived(
    desktop ? `/auth/callback?next=${encodeURIComponent(next)}${dlExtra}` : next
  );

  onMount(() => {
    if ($session) goto(postLogin);
  });

  $effect(() => {
    if ($session) goto(postLogin);
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

<svelte:head>
  <title>{`${$_('login.title')} — Hoard`}</title>
</svelte:head>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-16 sm:px-6 sm:py-20">
  <LogoMark size={48} />
  <h1 class="mt-5 font-display text-3xl font-semibold tracking-tight text-ink">{$_('login.title')}</h1>
  <p class="mt-2 text-sm text-ink-soft">{$_('login.subtitle')}</p>

  <div class="mt-10 w-full space-y-5">
    <button
      class="ring-focus flex w-full items-center justify-center gap-3 rounded-lg border border-line-strong bg-surface px-4 py-3 text-sm font-medium text-ink transition-colors hover:bg-bg disabled:opacity-50"
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
      <div class="h-px flex-1 bg-line"></div>
      <span class="font-mono text-[10px] uppercase tracking-[0.18em] text-ink-faint">
        {$_('login.email_label')}
      </span>
      <div class="h-px flex-1 bg-line"></div>
    </div>

    {#if sent}
      <div
        class="rounded-xl border border-accent bg-accent-tint p-4 text-sm text-accent"
      >
        {$_('login.email_sent', { values: { email } })}
      </div>
    {:else}
      <form class="space-y-3" onsubmit={withEmail}>
        <input
          type="email"
          required
          bind:value={email}
          placeholder={$_('login.email_placeholder')}
          class="ring-focus w-full rounded-lg border border-line bg-surface px-4 py-2.5 text-sm text-ink placeholder:text-ink-faint transition-colors focus:border-accent focus:outline-none"
        />
        <Button type="submit" variant="primary" full disabled={busy} loading={busy}>
          {$_('login.email_cta')}
        </Button>
      </form>
    {/if}

    {#if error}
      <p class="text-sm text-red-400">{error}</p>
    {/if}

    <p
      class="text-center text-xs text-ink-faint [&_a]:text-accent [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-accent"
    >
      {@html $_('login.terms_html')}
    </p>
  </div>
</section>
