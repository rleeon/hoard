<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { supabase, dropLocalSession } from '$lib/auth/supabase';
  import { safeNext } from '$lib/safeNext';
  import { api } from '$lib/api';
  import { CheckCircle2 } from 'lucide-svelte';
  import type { Session } from '@supabase/supabase-js';

  let message = $state('');
  let error = $state<string | null>(null);
  // The machine-readable reason under the headline. Kept separate so the
  // friendly sentence stays friendly and the quotable detail still shows.
  let errorDetail = $state<string | null>(null);

  /**
   * A denied or failed provider round-trip comes back as `error` +
   * `error_description`, in the query on PKCE, in the fragment on implicit.
   * We used to ignore both and print "we couldn't finish signing you in",
   * which is how "you clicked Cancel on Google's consent screen" and "the
   * provider is misconfigured" became the same dead end.
   */
  function providerError(): string | null {
    const q = $page.url.searchParams;
    const hash = new URLSearchParams(window.location.hash.replace(/^#/, ''));
    const code = q.get('error_code') ?? hash.get('error_code');
    const kind = q.get('error') ?? hash.get('error');
    const desc = q.get('error_description') ?? hash.get('error_description');
    if (!kind && !desc && !code) return null;
    return [code ?? kind, desc].filter(Boolean).join(' · ');
  }
  // Once we've handed off to the desktop app we stop the spinner and show a
  // terminal success state. Setting `window.location.href = "hoard://…"` blocks
  // the page (the browser hands control to the OS handler), so a CSS spinner
  // appears frozen, we render a static checkmark instead and keep the deep
  // link around for a manual retry.
  let handoffUrl = $state<string | null>(null);

  const isDesktop = () => $page.url.searchParams.get('desktop') === '1';

  // Desktop handoff: hand the freshly-minted session back to the Hoard app.
  //
  // Preferred path is the loopback redirect (RFC 8252): the app passes the port
  // it's listening on as `?port=N`, and we navigate to
  // `http://127.0.0.1:N/callback?...`. This is the only handoff that survives
  // snap/flatpak-confined browsers (Ubuntu's default Firefox is a snap), which
  // silently drop custom `hoard://` schemes. When there's no port (older app,
  // non-confined browser, macOS) we fall back to the `hoard://` scheme.
  //
  // Either way the tokens ride in the QUERY string (not the fragment): a
  // fragment never reaches the loopback server, and on Linux/Windows the OS
  // scheme handler frequently drops it from argv too. There's no log-leak
  // concern, 127.0.0.1 stays on the box and `hoard://` never hits a server.
  async function bounceToApp(s: Session) {
    const port = $page.url.searchParams.get('port');
    // CSRF nonce minted by the desktop app. Both handoff paths reject the
    // login unless we echo it back verbatim: the loopback listener checks it
    // at the socket, and `cloud_complete_login` re-checks it for the hoard://
    // fallback too. Present for every desktop login now (the app always mints
    // and threads it); only an old app build would omit it.
    const state = $page.url.searchParams.get('state') ?? '';
    const qs =
      `access_token=${encodeURIComponent(s.access_token)}` +
      `&refresh_token=${encodeURIComponent(s.refresh_token ?? '')}` +
      `${state ? `&state=${encodeURIComponent(state)}` : ''}`;
    const url = port
      ? `http://127.0.0.1:${port}/callback?${qs}`
      : `hoard://auth/callback?${qs}`;
    handoffUrl = url;
    message = $_('callback.desktop_return');
    // The desktop app now owns this session (it shares this tab's refresh-token
    // family). Stop this tab from auto-refreshing it in the background and drop
    // its persisted copy, WITHOUT a server-side logout. See dropLocalSession:
    // signOut({ scope: 'local' }) hits POST /logout and would revoke the shared
    // session, signing the app out too.
    await dropLocalSession();
    window.location.href = url;
  }

  async function done(s: Session) {
    if (isDesktop()) {
      bounceToApp(s);
      return;
    }
    // File the acceptance ticked on /login. Best-effort and un-awaited: this
    // is bookkeeping, and a user who just signed in must not be left staring
    // at a spinner because a side call was slow. The desktop branch above does
    // its own recording from the app once it owns the session.
    api.acceptTerms().catch((e) => console.warn('terms acceptance not recorded:', e));
    const next = safeNext($page.url.searchParams.get('next'));
    goto(next, { replaceState: true });
  }

  onMount(() => {
    message = $_('callback.signing_in');

    // The provider said no before we even get to token exchange. Say what it
    // said instead of guessing.
    const denied = providerError();
    if (denied) {
      error = $_('callback.failed_provider');
      errorDetail = denied;
      message = '';
      return;
    }

    // Does THIS redirect carry a fresh auth payload? Magic-link / OAuth returns
    // land here either with `#access_token=…` (implicit) or `?code=…` (PKCE).
    const href = window.location.href;
    const hasFreshAuth = href.includes('access_token=') || /[?&]code=/.test(href);

    // No payload → an already-signed-in browser is just bouncing through to
    // hand its session to the app. The cached session is the right one.
    if (!hasFreshAuth) {
      (async () => {
        const { data, error: sessErr } = await supabase.auth.getSession();
        if (data.session) done(data.session);
        else {
          error = $_('callback.failed_no_session');
          errorDetail = sessErr?.message ?? null;
          message = '';
        }
      })();
      return;
    }

    // Fresh sign-in: the session MUST come from this URL, not the one still in
    // localStorage. Clearing cookies doesn't clear that store, so reading the
    // cached session here used to forward the *previous* account (and made
    // switching accounts impossible). Wait for the SIGNED_IN that
    // detectSessionInUrl fires once it has exchanged the new tokens.
    let settled = false;
    const { data: sub } = supabase.auth.onAuthStateChange((event, s) => {
      if (settled) return;
      if (event === 'SIGNED_IN' && s) {
        settled = true;
        sub.subscription.unsubscribe();
        done(s);
      }
    });
    setTimeout(() => {
      if (settled) return;
      settled = true;
      sub.subscription.unsubscribe();
      // The tokens were in the URL but `detectSessionInUrl` never fired
      // SIGNED_IN. Naming the timeout separates it from "the provider said no"
      // and from "there was nothing to sign in with".
      error = $_('callback.failed_timeout');
      message = '';
    }, 8000);
  });
</script>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-32 text-center sm:px-6">
  {#if error}
    <h1 class="text-2xl font-semibold text-ink">{$_('callback.failed_title')}</h1>
    <p class="mt-2 text-sm text-ink-soft">{error}</p>
    {#if errorDetail}
      <p class="mt-2 max-w-full break-words font-mono text-xs text-ink-faint">{errorDetail}</p>
    {/if}
    <a href="/login" class="mt-6 text-sm text-accent hover:underline">
      {$_('callback.back_to_signin')}
    </a>
  {:else if handoffUrl}
    <CheckCircle2 size={40} class="text-accent" />
    <p class="mt-6 text-sm text-ink-soft">{message}</p>
    <a href={handoffUrl} class="mt-4 text-sm text-accent hover:underline">
      {$_('callback.desktop_reopen')}
    </a>
  {:else}
    <div class="h-10 w-10 animate-spin rounded-full border-2 border-accent border-t-transparent"></div>
    <p class="mt-6 text-sm text-ink-soft">{message}</p>
  {/if}
</section>
