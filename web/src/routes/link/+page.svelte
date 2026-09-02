<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { session, startSessionTracking } from '$lib/stores/session';
  import { api, ApiError } from '$lib/api';
  import { describeError } from '$lib/errors';
  import Button from '$lib/components/Button.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';

  type Phase = 'idle' | 'approving' | 'approved' | 'error';

  let code = $state('');
  let phase = $state<Phase>('idle');
  let errorMsg = $state<string | null>(null);
  let hostname = $state<string | null>(null);

  // Format loosely-typed input into the canonical XXXX-XXXX the CLI shows and
  // the server stores.
  function normalize(raw: string): string {
    const compact = raw
      .replace(/[^a-zA-Z0-9]/g, '')
      .toUpperCase()
      .slice(0, 8);
    return compact.length === 8 ? `${compact.slice(0, 4)}-${compact.slice(4)}` : compact;
  }

  onMount(() => {
    startSessionTracking();
    const q = $page.url.searchParams.get('code') ?? '';
    if (q) code = normalize(q);
  });

  // Preserve the code across the sign-in bounce so an unauthenticated user
  // lands back here with it still filled in.
  let signInHref = $derived(
    `/login?next=${encodeURIComponent(`/link${code ? `?code=${code}` : ''}`)}`
  );
  let canApprove = $derived(
    code.replace(/[^a-zA-Z0-9]/g, '').length === 8 && phase !== 'approving'
  );

  function oninput(e: Event) {
    code = normalize((e.target as HTMLInputElement).value);
  }

  async function approve() {
    phase = 'approving';
    errorMsg = null;
    try {
      const res = await api.approveDevice(normalize(code));
      hostname = res.hostname;
      phase = 'approved';
    } catch (e) {
      // Match the server's stable `code`, not the message, that now reads
      // "/v1/cloud/device/approve failed: 404 …" and would never compare equal.
      // Anything we don't have dedicated copy for shows its real reason rather
      // than a shrug.
      errorMsg =
        e instanceof ApiError && e.code === 'not_found'
          ? $_('link.err_not_found')
          : describeError(e, $_);
      phase = 'error';
    }
  }
</script>

<svelte:head>
  <title>{`${$_('link.title')} — Hoard`}</title>
  <meta name="robots" content="noindex,nofollow" />
</svelte:head>

<section class="mx-auto flex max-w-md flex-col items-center px-4 py-16 sm:px-6 sm:py-20">
  <LogoMark size={48} />
  <h1 class="mt-5 font-display text-3xl font-semibold tracking-tight text-ink">
    {$_('link.title')}
  </h1>
  <p class="mt-2 text-center text-sm text-ink-soft">{$_('link.subtitle')}</p>

  {#if $session === undefined}
    <p class="mt-10 text-sm text-ink-faint">…</p>
  {:else if $session === null}
    <div class="mt-10 w-full space-y-4 text-center">
      <p class="text-sm text-ink-soft">{$_('link.need_signin')}</p>
      <Button variant="primary" full href={signInHref}>{$_('nav.signin')}</Button>
    </div>
  {:else if phase === 'approved'}
    <div class="mt-10 w-full rounded-xl border border-line bg-surface p-6 text-center">
      <p class="text-sm font-medium text-ink">{$_('link.approved')}</p>
      {#if hostname}
        <p class="mt-1 font-mono text-xs text-ink-faint">{hostname}</p>
      {/if}
      <p class="mt-3 text-xs text-ink-soft">{$_('link.approved_hint')}</p>
    </div>
  {:else}
    <div class="mt-10 w-full space-y-4">
      <div class="rounded-xl border border-line bg-surface p-4 text-center">
        <p class="text-xs text-ink-faint">{$_('link.signed_in_as')}</p>
        <p class="mt-1 text-sm font-medium text-ink">{$session?.email}</p>
      </div>

      <label class="block">
        <span class="text-xs text-ink-faint">{$_('link.code_label')}</span>
        <input
          value={code}
          oninput={oninput}
          autocomplete="off"
          spellcheck="false"
          placeholder="XXXX-XXXX"
          class="ring-focus mt-1 w-full rounded-lg border border-line bg-surface px-4 py-2.5 text-center font-mono text-lg tracking-widest text-ink placeholder:text-ink-faint focus:border-accent focus:outline-none"
        />
      </label>

      {#if phase === 'error' && errorMsg}
        <p class="text-sm text-red-400">{errorMsg}</p>
      {/if}

      <Button
        variant="primary"
        full
        disabled={!canApprove}
        loading={phase === 'approving'}
        onclick={approve}
      >
        {$_('link.approve')}
      </Button>
      <p class="text-center text-xs text-ink-faint">{$_('link.approve_hint')}</p>
    </div>
  {/if}
</section>
