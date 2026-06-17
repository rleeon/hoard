<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { localeHref } from '$lib/i18n/href';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import QuotaBar from '$lib/components/QuotaBar.svelte';
  import { auth } from '$lib/auth';
  import { billing } from '$lib/billing';
  import { api } from '$lib/api';
  import { session } from '$lib/stores/session';
  import { PLANS, formatBytes, formatPlanQuota, daysUntil } from '$lib/plans';
  import type { AccountProfile, DeviceRow } from '$lib/types';
  import { LogOut, ExternalLink, Smartphone, Trash2 } from 'lucide-svelte';

  let profile = $state<AccountProfile | null>(null);
  let devices = $state<DeviceRow[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    if (!$session) return;
    loading = true;
    error = null;
    try {
      const [p, d] = await Promise.allSettled([api.me(), api.devices()]);
      if (p.status === 'fulfilled') {
        profile = p.value;
      } else {
        // No invented data: if the API is unreachable we say so instead of
        // rendering a fake free-plan profile.
        profile = null;
        error = (p.reason as Error)?.message ?? 'unreachable';
      }
      devices = d.status === 'fulfilled' ? d.value : [];
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function doSignOut() {
    await auth.signOut();
    goto(get(localeHref)('/'));
  }

  function gotoBilling() {
    window.location.href = billing.customerPortalUrl();
  }

  function gotoUpgrade() {
    // Free → straight into the upgrade confirmation flow; paid users go to
    // pricing to compare/switch cycle.
    if (profile?.plan === 'free') {
      goto('/checkout?plan=pro&cycle=monthly');
    } else {
      goto(get(localeHref)('/pricing'));
    }
  }

  async function unlink(id: string) {
    if (!confirm($_('account.confirm_unlink'))) return;
    await api.unlinkDevice(id);
    devices = devices.filter((d) => d.id !== id);
  }

  async function exportAll() {
    await api.requestAccountExport();
    alert($_('account.export_requested'));
  }

  async function deleteAccount() {
    const ok = confirm($_('account.confirm_delete_intro'));
    if (!ok) return;
    const c = prompt($_('account.confirm_delete_type'));
    if (c !== 'DELETE') return;
    await api.deleteAccount();
    await auth.signOut();
    goto(get(localeHref)('/'));
  }

  function timeAgo(iso: string): string {
    const ms = Date.now() - new Date(iso).getTime();
    const s = Math.floor(ms / 1000);
    if (s < 60) return $_('account.time_seconds_ago', { values: { n: s } });
    const m = Math.floor(s / 60);
    if (m < 60) return $_('account.time_minutes_ago', { values: { n: m } });
    const h = Math.floor(m / 60);
    if (h < 24) return $_('account.time_hours_ago', { values: { n: h } });
    return $_('account.time_days_ago', { values: { n: Math.floor(h / 24) } });
  }
</script>

<svelte:head>
  <title>{`${$_('account.title')} — Hoard`}</title>
  <meta name="robots" content="noindex,nofollow" />
</svelte:head>

<section class="mx-auto max-w-4xl space-y-6 px-4 py-12 sm:px-6">
  <div class="flex items-center justify-between">
    <h1 class="font-display text-3xl font-semibold text-ink">{$_('account.title')}</h1>
    <Button variant="ghost" onclick={doSignOut}>
      <LogOut class="h-4 w-4" />
      {$_('nav.signout')}
    </Button>
  </div>

  <div class="flex items-center gap-4">
    {#if $session?.avatarUrl}
      <img src={$session.avatarUrl} alt="" class="h-14 w-14 rounded-full" referrerpolicy="no-referrer" />
    {:else}
      <div class="grid h-14 w-14 place-items-center rounded-full bg-accent-deep text-xl font-semibold text-white">
        {($session?.email[0] ?? '?').toUpperCase()}
      </div>
    {/if}
    <div>
      <p class="text-lg font-medium text-ink">
        {$session?.displayName ?? $session?.email}
      </p>
      <p class="text-sm text-ink-soft">{$session?.email}</p>
    </div>
  </div>

  {#if loading}
    <Card>
      <div class="h-32 animate-pulse rounded-lg bg-ink/5"></div>
    </Card>
  {:else if !profile}
    <Card>
      <div class="flex flex-col items-start gap-4">
        <div>
          <h2 class="text-base font-semibold text-ink">{$_('account.load_error_title')}</h2>
          <p class="mt-1 text-sm leading-relaxed text-ink-soft">
            {$_('account.load_error_body')}
          </p>
          {#if error}
            <p class="mt-2 font-mono text-xs text-ink-faint">{error}</p>
          {/if}
        </div>
        <Button variant="secondary" size="sm" onclick={load}>{$_('common.retry')}</Button>
      </div>
    </Card>
  {:else}
    {@const p = profile}
    {@const plan = PLANS[p.plan]}
    {@const days = daysUntil(p.planRenewsAt)}
    {@const planLabel = $_(`plan.${p.plan}`)}

    <Card title={$_('account.plan_section')}>
      {#snippet actions()}
        <Button variant="secondary" size="sm" onclick={gotoUpgrade}>
          {$_('account.change_plan')}
        </Button>
        {#if p.plan !== 'free'}
          <Button variant="ghost" size="sm" onclick={gotoBilling}>
            {$_('account.manage_billing')}
            <ExternalLink class="h-3.5 w-3.5" />
          </Button>
        {/if}
      {/snippet}

      <div class="flex flex-wrap items-baseline gap-3">
        <span class="font-display text-3xl font-semibold text-ink">{planLabel}</span>
        {#if p.plan !== 'free'}
          <span class="text-sm text-ink-soft">
            {$_('account.price_per_month', {
              values: { price: plan.priceMonthly.toLocaleString('es-ES') }
            })}
          </span>
        {/if}
      </div>

      {#if p.planCancelAt}
        <p class="mt-3 text-sm text-amber-400">
          {$_('account.cancels_on', {
            values: { date: new Date(p.planCancelAt).toLocaleDateString() }
          })}
        </p>
      {:else if p.planRenewsAt && days !== null}
        <p class="mt-3 text-sm text-ink-soft">
          {$_('account.renews_in', { values: { days } })}
        </p>
      {/if}
    </Card>

    <Card title={$_('account.usage_section')}>
      <div class="space-y-5">
        <QuotaBar
          used={p.storageBytes}
          total={p.storageLimitBytes > 0 ? p.storageLimitBytes : plan.storageBytes}
          label={$_('account.storage_label')}
          formatted={$_('account.storage_used', {
            values: { used: formatBytes(p.storageBytes), quota: formatPlanQuota(p.plan) }
          })}
        />
        <div class="flex items-baseline justify-between text-sm">
          <span class="text-ink-soft">{$_('account.devices_label')}</span>
          <span class="font-medium tabular-nums text-ink">
            {#if p.devicesLimit < 0}
              {$_('account.devices_unlimited', { values: { used: p.devicesCount } })}
            {:else}
              {$_('account.devices_used', { values: { used: p.devicesCount, quota: p.devicesLimit } })}
            {/if}
          </span>
        </div>
      </div>
    </Card>

    <Card title={$_('account.devices_section')}>
      {#if devices.length === 0}
        <p class="text-sm text-ink-faint">{$_('account.no_devices')}</p>
      {:else}
        <ul class="divide-y divide-line">
          {#each devices as d (d.id)}
            <li class="flex items-center justify-between py-3">
              <div class="flex items-center gap-3">
                <Smartphone class="h-4 w-4 text-ink-faint" />
                <div>
                  <p class="text-sm font-medium text-ink">{d.deviceName}</p>
                  <p class="text-xs text-ink-faint">
                    {$_('account.last_seen', { values: { when: timeAgo(d.lastSeenAt) } })}
                  </p>
                </div>
              </div>
              <button
                class="ring-focus text-sm text-ink-soft transition-colors hover:text-red-400"
                onclick={() => unlink(d.id)}
              >
                {$_('account.unlink_device')}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </Card>

    <div id="account-danger"></div>
    <Card title={$_('account.danger_section')}>
      <div class="space-y-4">
        <div class="flex flex-wrap items-start justify-between gap-4">
          <div>
            <h3 class="text-sm font-semibold text-ink">{$_('account.export_title')}</h3>
            <p class="text-sm text-ink-soft">{$_('account.export_body')}</p>
          </div>
          <Button variant="secondary" size="sm" onclick={exportAll}>
            {$_('account.export_cta')}
          </Button>
        </div>
        <div class="flex flex-wrap items-start justify-between gap-4 border-t border-line pt-4">
          <div>
            <h3 class="text-sm font-semibold text-red-400">{$_('account.delete_title')}</h3>
            <p class="text-sm text-ink-soft">{$_('account.delete_body')}</p>
          </div>
          <Button variant="danger" size="sm" onclick={deleteAccount}>
            <Trash2 class="h-3.5 w-3.5" />
            {$_('account.delete_cta')}
          </Button>
        </div>
      </div>
    </Card>
  {/if}
</section>
