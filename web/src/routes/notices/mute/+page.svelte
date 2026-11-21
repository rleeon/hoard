<script lang="ts">
  import { config } from '$lib/config';
  import Button from '$lib/components/Button.svelte';
  import { page } from '$app/stores';

  // The click is what mutes, never the load. Mail clients and security
  // scanners fetch the links in a message before anyone sees it, so a page
  // that muted on load would silence the emails of everyone whose provider
  // prefetches.
  let state = $state<'idle' | 'sending' | 'done' | 'error'>('idle');

  const token = $derived($page.url.searchParams.get('t') ?? '');
  // Which email the link came from. Links without it predate the second kind
  // and are all the cleanup notice's.
  const newDevice = $derived($page.url.searchParams.get('k') === 'new-device');

  async function mute() {
    if (!token) return;
    state = 'sending';
    try {
      const res = await fetch(`${config.api.baseUrl}/v1/notices/mute`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token })
      });
      state = res.ok ? 'done' : 'error';
    } catch {
      state = 'error';
    }
  }
</script>

<svelte:head>
  <title>Stop these emails · Hoard</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<section class="mx-auto max-w-lg px-4 pb-24 pt-24">
  {#if !token}
    <h1 class="text-2xl font-semibold text-ink">This link is incomplete</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Open it straight from the email, without editing the address.
    </p>
  {:else if newDevice && state === 'done'}
    <h1 class="text-2xl font-semibold text-ink">Done, no more of those</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Hoard will no longer email you when a device signs in to your account. Everything else about
      your account still reaches you.
    </p>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Your account page still lists every device signed in, and unlinking one signs it out.
    </p>
  {:else if newDevice}
    <h1 class="text-2xl font-semibold text-ink">Stop these emails?</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      You are about to turn off the email Hoard sends when a new device signs in to your account.
      It stays off for every device from now on. Every other message about your account still
      reaches you.
    </p>
    <p class="mt-4 leading-relaxed text-ink-soft">
      It is how you would hear about a machine you do not know, so turn it off only if you are sure.
    </p>
    <div class="mt-8">
      <Button onclick={mute} disabled={state === 'sending'}>
        {state === 'sending' ? 'Stopping…' : 'Stop these emails'}
      </Button>
    </div>
    {#if state === 'error'}
      <p class="mt-4 text-sm text-ink-faint">
        That did not go through. Try again in a moment, or ask on Discord.
      </p>
    {/if}
  {:else if state === 'done'}
    <h1 class="text-2xl font-semibold text-ink">Done, no more of those</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Hoard will stop emailing you about old versions being deleted. Everything else about your
      account still reaches you, and this one comes back on its own if a fortnight passes without a
      single version being deleted.
    </p>
    <p class="mt-4 leading-relaxed text-ink-soft">
      The deleting itself has not stopped. It runs whenever the account is over its limit, and the
      app always shows what is happening.
    </p>
  {:else}
    <h1 class="text-2xl font-semibold text-ink">Stop these emails?</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      You are about to turn off the notice that says Hoard is deleting your oldest versions to make
      room. Nothing else changes: the deleting carries on, and every other message about your
      account still reaches you.
    </p>
    <p class="mt-4 leading-relaxed text-ink-soft">
      It comes back by itself if a fortnight goes by without a single version being deleted.
    </p>
    <div class="mt-8">
      <Button onclick={mute} disabled={state === 'sending'}>
        {state === 'sending' ? 'Stopping…' : 'Stop these emails'}
      </Button>
    </div>
    {#if state === 'error'}
      <p class="mt-4 text-sm text-ink-faint">
        That did not go through. Try again in a moment, or ask on Discord.
      </p>
    {/if}
  {/if}
</section>
