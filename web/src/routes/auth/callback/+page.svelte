<script lang="ts">
  import { onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { supabase } from '$lib/auth/supabase';
  import { CheckCircle2 } from 'lucide-svelte';
  import type { Session } from '@supabase/supabase-js';

  let message = $state('');
  let error = $state<string | null>(null);
  // Once we've handed off to the desktop app we stop the spinner and show a
  // terminal success state. Setting `window.location.href = "hoard://…"` blocks
  // the page (the browser hands control to the OS handler), so a CSS spinner
  // appears frozen — we render a static checkmark instead and keep the deep
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
  // concern — 127.0.0.1 stays on the box and `hoard://` never hits a server.
  function bounceToApp(s: Session) {
    const port = $page.url.searchParams.get('port');
    const qs =
      `access_token=${encodeURIComponent(s.access_token)}` +
      `&refresh_token=${encodeURIComponent(s.refresh_token ?? '')}`;
    const url = port
      ? `http://127.0.0.1:${port}/callback?${qs}`
      : `hoard://auth/callback?${qs}`;
    handoffUrl = url;
    message = $_('callback.desktop_return');
    window.location.href = url;
  }

  function done(s: Session) {
    if (isDesktop()) {
      bounceToApp(s);
      return;
    }
    const next = $page.url.searchParams.get('next') ?? '/account';
    goto(next, { replaceState: true });
  }

  onMount(async () => {
    message = $_('callback.signing_in');
    try {
      const { data, error: e } = await supabase.auth.getSession();
      if (e) throw e;
      if (data.session) {
        done(data.session);
        return;
      }
      // Supabase JS handles detectSessionInUrl automatically; give it a moment
      setTimeout(async () => {
        const { data: d2 } = await supabase.auth.getSession();
        if (d2.session) {
          done(d2.session);
        } else {
          error = $_('callback.failed_generic');
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
    <h1 class="text-2xl font-semibold text-white">{$_('callback.failed_title')}</h1>
    <p class="mt-2 text-sm text-zinc-400">{error}</p>
    <a href="/login" class="mt-6 text-sm text-emerald-400 hover:underline">
      {$_('callback.back_to_signin')}
    </a>
  {:else if handoffUrl}
    <CheckCircle2 size={40} class="text-emerald-400" />
    <p class="mt-6 text-sm text-zinc-300">{message}</p>
    <a href={handoffUrl} class="mt-4 text-sm text-emerald-400 hover:underline">
      {$_('callback.desktop_reopen')}
    </a>
  {:else}
    <div class="h-10 w-10 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent"></div>
    <p class="mt-6 text-sm text-zinc-400">{message}</p>
  {/if}
</section>
