<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { session } from '$lib/stores/session';
  import { api } from '$lib/api';
  import { PLANS } from '$lib/plans';
  import Button from '$lib/components/Button.svelte';
  import LogoMark from '$lib/components/LogoMark.svelte';
  import type { BillingCycle, PlanId } from '$lib/types';
  import { ShieldCheck } from 'lucide-svelte';

  // What the user picked on /pricing. Defaults keep the page sane if someone
  // lands on /checkout bare.
  let plan = $derived(($page.url.searchParams.get('plan') ?? 'pro') as Exclude<PlanId, 'free'>);
  let cycle = $derived(
    ($page.url.searchParams.get('cycle') === 'yearly' ? 'yearly' : 'monthly') as BillingCycle
  );

  // Where to come back to after login — this exact page, intent preserved.
  let selfUrl = $derived(`/checkout?plan=${plan}&cycle=${cycle}`);

  let busy = $state(false);
  let error = $state<string | null>(null);

  // No account → bounce to login, then return straight here (not /account),
  // so the purchase continues seamlessly.
  $effect(() => {
    if ($session === null) {
      goto(`/login?next=${encodeURIComponent(selfUrl)}`, { replaceState: true });
    }
  });

  let price = $derived(
    cycle === 'yearly' ? PLANS[plan].priceYearly : PLANS[plan].priceMonthly
  );
  let priceLabel = $derived(
    cycle === 'yearly'
      ? $_('checkout.price_yearly', { values: { price: price.toLocaleString('es-ES') } })
      : $_('checkout.price_monthly', { values: { price: price.toLocaleString('es-ES') } })
  );

  async function cont() {
    error = null;
    busy = true;
    try {
      const url = await api.createCheckout(plan, cycle);
      // Hand off to Polar's hosted checkout.
      window.location.href = url;
    } catch (e) {
      error = (e as Error).message;
      busy = false;
    }
  }

  function changeAccount() {
    // Send them to the account page's session/danger zone so signing out (to
    // switch accounts) is one click away without scrolling.
    goto('/account#account-danger');
  }
</script>

<svelte:head>
  <title>{`${$_('checkout.title')} — Hoard`}</title>
  <meta name="robots" content="noindex,nofollow" />
</svelte:head>

{#if $session === undefined || $session === null}
  <div class="grid min-h-[60vh] place-items-center">
    <div
      class="h-10 w-10 animate-spin rounded-full border-2 border-accent border-t-transparent"
    ></div>
  </div>
{:else}
  <section class="mx-auto flex max-w-md flex-col items-center px-4 py-16 sm:px-6 sm:py-20">
    <LogoMark size={44} />
    <h1 class="mt-5 text-balance text-center font-display text-3xl font-semibold tracking-tight text-ink">
      {$_('checkout.title')}
    </h1>
    <p class="mt-2 text-center text-sm text-ink-soft">{$_('checkout.subtitle')}</p>

    <div class="mt-8 w-full rounded-2xl border border-line bg-surface p-6">
      <div class="flex items-center gap-3 border-b border-line pb-4">
        {#if $session.avatarUrl}
          <img
            src={$session.avatarUrl}
            alt=""
            class="h-11 w-11 rounded-full"
            referrerpolicy="no-referrer"
          />
        {:else}
          <div
            class="grid h-11 w-11 place-items-center rounded-full bg-accent-deep text-lg font-semibold text-white"
          >
            {($session.email[0] ?? '?').toUpperCase()}
          </div>
        {/if}
        <div class="min-w-0">
          <p class="truncate text-sm font-medium text-ink">
            {$session.displayName ?? $session.email}
          </p>
          <p class="truncate text-xs text-ink-soft">{$session.email}</p>
        </div>
      </div>

      <p class="mt-4 text-sm text-ink-soft">{$_('checkout.question')}</p>

      <div
        class="mt-4 flex items-center justify-between rounded-xl border border-accent bg-accent-tint px-4 py-3"
      >
        <span class="text-sm font-semibold text-accent">{$_('plan.pro')}</span>
        <span class="text-sm text-ink">{priceLabel}</span>
      </div>

      {#if error}
        <p class="mt-4 text-sm text-red-400">{$_('checkout.error')}</p>
        <p class="mt-1 break-words text-xs text-ink-faint">{error}</p>
      {/if}

      <div class="mt-6 flex items-center gap-3">
        <button
          class="ring-focus flex-1 rounded-lg border border-line-strong bg-surface px-4 py-2.5 text-sm font-medium text-ink transition-colors hover:bg-bg disabled:opacity-50"
          onclick={changeAccount}
          disabled={busy}
        >
          {$_('checkout.change_account')}
        </button>
        <div class="flex-1">
          <Button variant="primary" full onclick={cont} disabled={busy} loading={busy}>
            {$_('checkout.continue')}
          </Button>
        </div>
      </div>
    </div>

    <p class="mt-5 flex items-center gap-1.5 text-xs text-ink-faint">
      <ShieldCheck class="h-3.5 w-3.5 text-accent" />
      {$_('checkout.secured_by')}
    </p>
  </section>
{/if}
