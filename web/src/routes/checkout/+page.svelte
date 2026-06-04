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
</svelte:head>

{#if $session === undefined || $session === null}
  <div class="grid min-h-[60vh] place-items-center">
    <div
      class="h-10 w-10 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent"
    ></div>
  </div>
{:else}
  <section class="relative mx-auto flex max-w-md flex-col items-center px-4 py-20 sm:px-6">
    <div
      class="pointer-events-none absolute -top-24 left-1/2 -z-10 h-72 w-[28rem] -translate-x-1/2 rounded-full bg-emerald-500/10 blur-3xl"
    ></div>

    <LogoMark size={44} />
    <h1 class="mt-5 text-balance text-center text-3xl font-bold tracking-tight text-white">
      {$_('checkout.title')}
    </h1>
    <p class="mt-2 text-center text-sm text-zinc-400">{$_('checkout.subtitle')}</p>

    <div
      class="card-edge mt-8 w-full rounded-2xl border border-white/[0.06] bg-white/[0.025] p-6 backdrop-blur-sm"
    >
      <div class="flex items-center gap-3 border-b border-white/[0.06] pb-4">
        {#if $session.avatarUrl}
          <img
            src={$session.avatarUrl}
            alt=""
            class="h-11 w-11 rounded-full"
            referrerpolicy="no-referrer"
          />
        {:else}
          <div
            class="grid h-11 w-11 place-items-center rounded-full bg-emerald-700 text-lg font-semibold text-white"
          >
            {($session.email[0] ?? '?').toUpperCase()}
          </div>
        {/if}
        <div class="min-w-0">
          <p class="truncate text-sm font-medium text-white">
            {$session.displayName ?? $session.email}
          </p>
          <p class="truncate text-xs text-zinc-400">{$session.email}</p>
        </div>
      </div>

      <p class="mt-4 text-sm text-zinc-300">{$_('checkout.question')}</p>

      <div
        class="mt-4 flex items-center justify-between rounded-xl border border-emerald-400/20 bg-emerald-500/[0.06] px-4 py-3"
      >
        <span class="text-sm font-semibold text-emerald-200">{$_('plan.pro')}</span>
        <span class="text-sm text-emerald-100">{priceLabel}</span>
      </div>

      {#if error}
        <p class="mt-4 text-sm text-red-400">{$_('checkout.error')}</p>
        <p class="mt-1 break-words text-xs text-zinc-500">{error}</p>
      {/if}

      <div class="mt-6 flex items-center gap-3">
        <button
          class="ring-focus flex-1 rounded-lg border border-white/10 bg-zinc-900 px-4 py-2.5 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-800 disabled:opacity-50"
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

    <p class="mt-5 flex items-center gap-1.5 text-xs text-zinc-500">
      <ShieldCheck class="h-3.5 w-3.5 text-emerald-400/70" />
      {$_('checkout.secured_by')}
    </p>
  </section>
{/if}
