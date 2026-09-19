<script lang="ts">
  import { config } from '$lib/config';
  import Button from '$lib/components/Button.svelte';
  import { page } from '$app/stores';

  // Same rule as the mute page: the click opts out, never the load, because
  // mail providers open every link in a message before a person does. Here it
  // matters twice over, since this is the refusal the law requires us to honour.
  let state = $state<'idle' | 'sending' | 'done' | 'error'>('idle');

  const token = $derived($page.url.searchParams.get('t') ?? '');

  async function optOut() {
    if (!token) return;
    state = 'sending';
    try {
      const res = await fetch(`${config.api.baseUrl}/v1/notices/no-offers`, {
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
  <title>Stop offers in Hoard emails · Hoard</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<section class="mx-auto max-w-lg px-4 pb-24 pt-24">
  {#if !token}
    <h1 class="text-2xl font-semibold text-ink">This link is incomplete</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Open it straight from the email, without editing the address.
    </p>
  {:else if state === 'done'}
    <h1 class="text-2xl font-semibold text-ink">Done, no more offers</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Hoard's emails will no longer include offers for Pro. The notices about your saves keep
      coming exactly as before, because those are about your account, not about selling you
      anything.
    </p>
  {:else}
    <h1 class="text-2xl font-semibold text-ink">Stop offers in Hoard's emails?</h1>
    <p class="mt-4 leading-relaxed text-ink-soft">
      Some of the notices Hoard sends about your saves include an offer for Pro. This turns those
      offers off for your account, in every email from now on.
    </p>
    <p class="mt-4 leading-relaxed text-ink-soft">
      The notices themselves keep coming: when a backup does not fit, when old versions are being
      deleted, when an archived game is about to go. Those are about your data, and you would want
      to know.
    </p>
    <div class="mt-8">
      <Button onclick={optOut} disabled={state === 'sending'}>
        {state === 'sending' ? 'Turning off…' : 'Stop the offers'}
      </Button>
    </div>
    {#if state === 'error'}
      <p class="mt-4 text-sm text-ink-faint">
        That did not go through. Try again in a moment, or ask on Discord.
      </p>
    {/if}
  {/if}
</section>
