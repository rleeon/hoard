<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
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
  // Set when the user picks "use another account": suppresses the auto-forward
  // so they can sign in as someone else even though a session still lingers.
  let switching = $state(false);

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
  // CSRF nonce the desktop app generated for this login attempt. The loopback
  // listener only accepts the callback if this exact value comes back, so we
  // must thread it through Supabase's redirect to /auth/callback untouched.
  // NB: must not be named `state` — that collides with the `$state` rune and
  // makes the compiler read `$state(...)` above as a store subscription.
  let csrfState = $derived($page.url.searchParams.get('state') ?? '');
  let dlExtra = $derived(
    `${desktop ? '&desktop=1' : ''}${port ? `&port=${encodeURIComponent(port)}` : ''}` +
      `${csrfState ? `&state=${encodeURIComponent(csrfState)}` : ''}`
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

  // Show the account picker when the desktop app sends an already-signed-in
  // browser here, so the user can connect the app OR switch accounts. We only
  // auto-forward for plain web browsing. `browser &&` short-circuits so the
  // searchParams-backed `desktop` is never read during prerender (which would
  // throw "Cannot access url.searchParams on a page with prerendering enabled").
  let showAccountChoice = $derived(browser && desktop && !!$session && !switching);

  // Auto-forward an existing session for non-desktop browsing only. For the
  // desktop handoff we never auto-forward: a lingering browser session
  // (Supabase persists it in localStorage, which "clear cookies" doesn't touch)
  // would otherwise log the app into the previous account with no way to switch.
  onMount(() => {
    if ($session && !desktop) goto(postLogin);
  });

  $effect(() => {
    if ($session && !desktop) goto(postLogin);
  });

  function continueAsCurrent() {
    goto(postLogin);
  }

  async function useAnotherAccount() {
    switching = true;
    sent = false;
    email = '';
    error = null;
    try {
      await auth.signOut();
    } catch (e) {
      error = (e as Error).message;
    }
  }

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
  <meta name="robots" content="noindex,nofollow" />
</svelte:head>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-16 sm:px-6 sm:py-20">
  <LogoMark size={48} />
  <h1 class="mt-5 font-display text-3xl font-semibold tracking-tight text-ink">{$_('login.title')}</h1>
  <p class="mt-2 text-sm text-ink-soft">{$_('login.subtitle')}</p>

  {#if showAccountChoice}
    <div class="mt-10 w-full space-y-4">
      <div class="rounded-xl border border-line bg-surface p-4 text-center">
        <p class="text-xs text-ink-faint">{$_('login.signed_in_as')}</p>
        <p class="mt-1 text-sm font-medium text-ink">{$session?.email}</p>
      </div>
      <Button variant="primary" full onclick={continueAsCurrent}>
        {$_('login.connect_app')}
      </Button>
      <button
        class="ring-focus w-full rounded-lg border border-line px-4 py-2.5 text-sm font-medium text-ink-soft transition-colors hover:bg-bg"
        onclick={useAnotherAccount}
      >
        {$_('login.use_another_account')}
      </button>
    </div>
  {:else}
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
      {#if desktop}
        <div
          class="rounded-xl border border-amber-500/40 bg-amber-500/10 p-4 text-sm text-amber-300"
        >
          {$_('login.email_sent_desktop_note')}
        </div>
      {/if}
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
  {/if}
</section>
