<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { auth } from '$lib/auth';
  import { config } from '$lib/config';
  import { safeNext } from '$lib/safeNext';
  import { session } from '$lib/stores/session';
  import Button from '$lib/components/Button.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';
  import Turnstile from '$lib/components/Turnstile.svelte';

  // Cloudflare Turnstile. Empty site key => captcha disabled, and the email
  // form behaves exactly as before (no widget, no token gating).
  const captchaSiteKey = config.turnstile.siteKey;
  const captchaEnabled = !!captchaSiteKey;
  let captchaToken = $state('');
  let turnstile = $state<Turnstile | null>(null);
  // Turnstile tokens are single-use: after a submit consumes one we must fetch
  // a fresh challenge before the next email/resend attempt.
  function refreshCaptcha() {
    captchaToken = '';
    turnstile?.reset();
  }

  let email = $state('');
  // Explicit acceptance, matching what the desktop app has always asked for.
  // "By continuing you accept" is an inference; a tick is a decision, and it is
  // the one we can actually record against the account afterwards.
  let accepted = $state(false);
  let sent = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  // Seconds left before a resend is allowed. Non-zero disables the resend
  // button so an impatient user can't hammer Supabase's OTP endpoint (which
  // rate-limits and would start bouncing the requests).
  let cooldown = $state(0);
  let cooldownTimer: ReturnType<typeof setInterval> | null = null;

  function startCooldown(secs = 30) {
    cooldown = secs;
    if (cooldownTimer) clearInterval(cooldownTimer);
    cooldownTimer = setInterval(() => {
      cooldown -= 1;
      if (cooldown <= 0 && cooldownTimer) {
        clearInterval(cooldownTimer);
        cooldownTimer = null;
      }
    }, 1000);
  }
  onDestroy(() => {
    if (cooldownTimer) clearInterval(cooldownTimer);
  });
  // Set when the user picks "use another account": suppresses the auto-forward
  // so they can sign in as someone else even though a session still lingers.
  let switching = $state(false);

  let next = $derived(browser ? safeNext($page.url.searchParams.get('next')) : '/');
  // Desktop login handoff: the Hoard app opens this page with ?desktop=1 and
  // expects the session to bounce back via the `hoard://` deep link instead of
  // staying in the browser. We carry the flag through to /auth/callback, which
  // does the actual redirect to the app.
  let desktop = $derived(browser && $page.url.searchParams.get('desktop') === '1');
  // Loopback port the desktop app is listening on for the OAuth handoff. When
  // present we bounce the session to http://127.0.0.1:<port> (confined browsers
  // like Ubuntu's snap Firefox can open that) instead of the custom hoard://
  // scheme (which they silently drop). Carried through to /auth/callback.
  let port = $derived(browser ? $page.url.searchParams.get('port') ?? '' : '');
  // CSRF nonce the desktop app generated for this login attempt. The loopback
  // listener only accepts the callback if this exact value comes back, so we
  // must thread it through Supabase's redirect to /auth/callback untouched.
  // NB: must not be named `state`, that collides with the `$state` rune and
  // makes the compiler read `$state(...)` above as a store subscription.
  let csrfState = $derived(browser ? $page.url.searchParams.get('state') ?? '' : '');
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

  /** Guard shared by the three sign-in paths. Returns false and explains
   *  itself rather than silently doing nothing on a disabled-looking button. */
  function requireAcceptance(): boolean {
    if (accepted) return true;
    error = $_('login.terms_required');
    return false;
  }

  async function withGoogle() {
    if (!requireAcceptance()) return;
    error = null;
    busy = true;
    try {
      await auth.signInWithGoogle(redirectTo);
    } catch (e) {
      error = (e as Error).message;
      busy = false;
    }
  }

  async function withGithub() {
    if (!requireAcceptance()) return;
    error = null;
    busy = true;
    try {
      await auth.signInWithGithub(redirectTo);
    } catch (e) {
      error = (e as Error).message;
      busy = false;
    }
  }

  // Supabase's own OTP rate limit (distinct from our client-side resend
  // cooldown) surfaces as AuthApiError with this code, show a message that
  // explains it's temporary rather than the raw "email rate limit exceeded".
  function describeAuthError(err: unknown): string {
    if ((err as { code?: string })?.code === 'over_email_send_rate_limit') {
      return $_('login.error_rate_limit');
    }
    return (err as Error).message;
  }

  async function withEmail(e: SubmitEvent) {
    e.preventDefault();
    if (!requireAcceptance()) return;
    if (!email) return;
    if (captchaEnabled && !captchaToken) {
      error = $_('login.captcha_required');
      return;
    }
    error = null;
    busy = true;
    try {
      await auth.signInWithEmail(email, redirectTo, captchaToken || undefined);
      sent = true;
      startCooldown();
    } catch (err) {
      error = describeAuthError(err);
    } finally {
      busy = false;
      // The token was consumed (or the attempt failed); force a new challenge.
      if (captchaEnabled) refreshCaptcha();
    }
  }

  // Re-send the magic link to the same address. Gated behind the cooldown.
  async function resend() {
    if (busy || cooldown > 0 || !email) return;
    if (captchaEnabled && !captchaToken) {
      error = $_('login.captcha_required');
      return;
    }
    error = null;
    busy = true;
    try {
      await auth.signInWithEmail(email, redirectTo, captchaToken || undefined);
      startCooldown();
    } catch (err) {
      error = describeAuthError(err);
    } finally {
      busy = false;
      if (captchaEnabled) refreshCaptcha();
    }
  }

  // Back to the form to correct a mistyped address.
  function useAnotherEmail() {
    sent = false;
    error = null;
    if (cooldownTimer) {
      clearInterval(cooldownTimer);
      cooldownTimer = null;
    }
    cooldown = 0;
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
    <label
      class="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-surface p-4 text-left text-xs leading-relaxed text-ink-soft [&_a]:text-accent [&_a]:underline [&_a]:underline-offset-2"
    >
      <input
        type="checkbox"
        bind:checked={accepted}
        class="ring-focus mt-0.5 h-4 w-4 shrink-0 rounded border-line-strong bg-bg text-accent"
      />
      <span>{@html $_('login.terms_html')}</span>
    </label>

    <button
      class="ring-focus flex w-full items-center justify-center gap-3 rounded-lg border border-line-strong bg-surface px-4 py-3 text-sm font-medium text-ink transition-colors hover:bg-bg disabled:opacity-50"
      onclick={withGoogle}
      disabled={busy || !accepted}
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

    <button
      class="ring-focus flex w-full items-center justify-center gap-3 rounded-lg border border-line-strong bg-surface px-4 py-3 text-sm font-medium text-ink transition-colors hover:bg-bg disabled:opacity-50"
      onclick={withGithub}
      disabled={busy || !accepted}
    >
      <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path
          d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
        />
      </svg>
      {$_('login.github')}
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
      {#if captchaEnabled}
        <Turnstile
          bind:this={turnstile}
          siteKey={captchaSiteKey}
          onToken={(t) => (captchaToken = t)}
        />
      {/if}
      <div class="flex flex-col gap-2">
        <button
          class="ring-focus w-full rounded-lg border border-line px-4 py-2.5 text-sm font-medium text-ink-soft transition-colors hover:bg-bg disabled:opacity-50"
          onclick={resend}
          disabled={busy || cooldown > 0 || (captchaEnabled && !captchaToken)}
        >
          {cooldown > 0
            ? $_('login.email_resend_wait', { values: { secs: cooldown } })
            : $_('login.email_resend')}
        </button>
        <button
          class="ring-focus w-full rounded-lg px-4 py-2 text-xs font-medium text-ink-faint transition-colors hover:text-ink-soft"
          onclick={useAnotherEmail}
        >
          {$_('login.use_another_email')}
        </button>
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
        {#if captchaEnabled}
          <Turnstile
            bind:this={turnstile}
            siteKey={captchaSiteKey}
            onToken={(t) => (captchaToken = t)}
          />
        {/if}
        <Button
          type="submit"
          variant="primary"
          full
          disabled={busy || !accepted || (captchaEnabled && !captchaToken)}
          loading={busy}
        >
          {$_('login.email_cta')}
        </Button>
      </form>
    {/if}

    {#if error}
      <p class="text-sm text-red-400">{error}</p>
    {/if}

  </div>
  {/if}
</section>
