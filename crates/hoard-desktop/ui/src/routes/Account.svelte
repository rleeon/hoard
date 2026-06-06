<script lang="ts">
  /**
   * Account page — the entry point for everything Hoard Cloud.
   *
   * Three states:
   *   - signed out → render provider buttons (Google / GitHub / Discord /
   *                  Apple) that open the OAuth flow in the system browser.
   *   - signed in  → render plan card, usage bars, billing actions, data
   *                  export, account deletion.
   *   - loading    → small spinner while `cloud_current_account` resolves.
   *
   * Deep-link callback is wired in App.svelte via `initCloudDeepLink`.
   */
  import { onMount, onDestroy } from "svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";
  import { LogOut, ArrowUpRight, Mail, Download, Trash2, RefreshCw, ShieldCheck, AlertTriangle, CreditCard, HardDrive, Layers, Clock, FileArchive, Gauge } from "lucide-svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import UpgradePlanModal from "../lib/components/UpgradePlanModal.svelte";
  import {
    cloud,
    hydrateCloud,
    refreshCloud,
    startCloudLogin,
    logoutCloud,
    exportAllCloudData,
    deleteCloudAccount,
    openBillingPortal,
    openUpgradePage,
    planLabel,
  } from "../lib/stores/cloud";
  import { toastError, toastInfo, toastSuccess } from "../lib/stores/toasts";
  import { clearOnboarding } from "../lib/stores/onboarding";
  import { formatBytes } from "../lib/utils/format";

  let busyAction = $state<
    "signin" | "logout" | "refresh" | "export" | "delete" | null
  >(null);
  let confirmDeleteOpen = $state(false);
  let deleteConfirmation = $state("");
  let compareOpen = $state(false);

  onMount(async () => {
    if (!$cloud.hydrated) {
      await hydrateCloud();
    }
  });

  async function handleSignIn() {
    if (busyAction) return;
    busyAction = "signin";
    try {
      await startCloudLogin();
      toastInfo($_("account.opened_browser"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  async function handleLogout() {
    if (busyAction) return;
    busyAction = "logout";
    try {
      await logoutCloud();
      // Reset the wizard so the next launch (and this navigation) lands on the
      // welcome screen, not a stale persisted step. Then leave /account
      // immediately instead of sitting on the signed-out view, which is what
      // made it look like the session "came back".
      await clearOnboarding();
      toastInfo($_("account.signed_out"));
      push("/welcome");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  async function handleRefresh() {
    if (busyAction) return;
    busyAction = "refresh";
    try {
      await refreshCloud();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  async function handleExport() {
    if (busyAction) return;
    busyAction = "export";
    try {
      const job = await exportAllCloudData();
      toastSuccess(
        $_("account.export_queued", { values: { id: job.job_id } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  function openDeleteModal() {
    deleteConfirmation = "";
    confirmDeleteOpen = true;
  }

  async function handleDelete() {
    if (busyAction) return;
    if (deleteConfirmation.trim().toUpperCase() !== "DELETE") {
      toastError($_("account.delete_type_confirm"));
      return;
    }
    busyAction = "delete";
    try {
      await deleteCloudAccount();
      confirmDeleteOpen = false;
      toastInfo($_("account.delete_scheduled"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  // ---- derived view models -------------------------------------------

  const account = $derived($cloud.account);

  /** Storage progress bar — 0 means "unlimited", which we surface as a
   *  flat emerald bar and a "∞" cap label. */
  const storageView = $derived.by(() => {
    const a = account;
    if (!a) return null;
    if (a.storage_limit_bytes <= 0) {
      return {
        usedLabel: formatBytes(a.storage_used_bytes),
        capLabel: "∞",
        pct: 0,
        color: "bg-emerald-500",
        unlimited: true,
      };
    }
    const pct = Math.max(
      0,
      Math.min(100, Math.round((a.storage_used_bytes / a.storage_limit_bytes) * 100)),
    );
    const color =
      pct >= 90 ? "bg-rose-500" : pct >= 75 ? "bg-amber-500" : "bg-emerald-500";
    return {
      usedLabel: formatBytes(a.storage_used_bytes),
      capLabel: formatBytes(a.storage_limit_bytes),
      pct,
      color,
      unlimited: false,
    };
  });

  const devicesView = $derived.by(() => {
    const a = account;
    if (!a) return null;
    const unlimited = a.devices_limit <= 0;
    return {
      used: a.devices_used,
      cap: unlimited ? "∞" : `${a.devices_limit}`,
      unlimited,
    };
  });

  const savesView = $derived.by(() => {
    const a = account;
    if (!a) return null;
    const unlimited = a.saves_limit <= 0;
    return {
      used: a.saves_used,
      cap: unlimited ? "∞" : `${a.saves_limit}`,
      unlimited,
    };
  });

  // Render `renews_at` / `cancel_at` as the user's local date — full RFC3339
  // is too noisy for a billing card.
  function fmtDate(iso: string | null | undefined): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

<div class="mx-auto max-w-3xl px-6 py-8">
  <div class="mb-6 flex items-center gap-2">
    <h1 class="font-display text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50">
      {$_("account.title")}
    </h1>
  </div>

  {#if !$cloud.hydrated}
    <Card>
      <div class="flex items-center gap-3 text-zinc-400">
        <RefreshCw size={16} class="animate-spin" />
        <span>{$_("common.loading")}</span>
      </div>
    </Card>
  {:else if !account}
    <!-- ───────────── Signed-out: provider grid + value props ─────── -->
    <Card class="mb-6">
      <div class="flex items-start gap-4">
        <div class="rounded-lg bg-emerald-500/10 p-3 text-emerald-400 ring-1 ring-emerald-500/30">
          <ShieldCheck size={24} />
        </div>
        <div class="flex-1">
          <h2 class="text-lg font-semibold">{$_("account.signin_title")}</h2>
          <p class="mt-1 text-sm text-zinc-400">
            {$_("account.signin_subtitle")}
          </p>
        </div>
      </div>

      <div class="mt-6 space-y-3">
        <Button
          variant="primary"
          size="lg"
          loading={busyAction === "signin"}
          disabled={busyAction !== null}
          onclick={handleSignIn}
          class="w-full"
        >
          <Mail size={18} />
          {$_("account.signin_button")}
        </Button>
        <p class="text-center text-xs text-zinc-500">
          {$_("account.signin_hint")}
        </p>
      </div>

      <div class="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
          <p class="text-xs font-medium text-emerald-400">
            {$_("account.feature_storage")}
          </p>
          <p class="mt-1 text-sm text-zinc-300">
            {$_("account.feature_storage_body")}
          </p>
        </div>
        <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
          <p class="text-xs font-medium text-emerald-400">
            {$_("account.feature_sync")}
          </p>
          <p class="mt-1 text-sm text-zinc-300">
            {$_("account.feature_sync_body")}
          </p>
        </div>
        <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
          <p class="text-xs font-medium text-emerald-400">
            {$_("account.feature_privacy")}
          </p>
          <p class="mt-1 text-sm text-zinc-300">
            {$_("account.feature_privacy_body")}
          </p>
        </div>
      </div>
    </Card>

    <Card>
      <h3 class="text-sm font-semibold">{$_("account.selfhost_title")}</h3>
      <p class="mt-1 text-sm text-zinc-400">
        {$_("account.selfhost_body")}
      </p>
      <Button
        variant="secondary"
        class="mt-3"
        onclick={() => push("/onboarding/server")}
      >
        {$_("account.selfhost_action")}
      </Button>
    </Card>
  {:else}
    <!-- ───────────── Signed-in: plan + usage + actions ───────────── -->
    <Card class="mb-4">
      <div class="flex items-start justify-between gap-4">
        <div class="flex min-w-0 items-center gap-3">
          {#if account.avatar_url}
            <img
              src={account.avatar_url}
              alt={account.display_name ?? account.email}
              referrerpolicy="no-referrer"
              class="h-12 w-12 shrink-0 rounded-full object-cover ring-1 ring-zinc-700"
            />
          {:else}
            <span
              class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-emerald-500/10 text-lg font-semibold uppercase text-emerald-300 ring-1 ring-emerald-500/30"
            >
              {(account.display_name ?? account.email).trim().charAt(0)}
            </span>
          {/if}
          <div class="min-w-0">
            <p class="text-xs uppercase tracking-wide text-zinc-500">
              {$_("account.signed_in_as")}
            </p>
            <p class="mt-1 truncate text-lg font-medium">
              {account.display_name ?? account.email}
            </p>
            <p class="truncate text-xs text-zinc-500">{account.email}</p>
          </div>
        </div>
        <Button
          variant="ghost"
          loading={busyAction === "refresh"}
          disabled={busyAction !== null}
          onclick={handleRefresh}
          aria-label={$_("account.refresh")}
        >
          <RefreshCw size={14} />
          {$_("account.refresh")}
        </Button>
      </div>
    </Card>

    <Card class="mb-4">
      <div class="mb-4 flex items-start justify-between gap-4">
        <div>
          <p class="text-xs uppercase tracking-wide text-zinc-500">
            {$_("account.plan")}
          </p>
          <p class="mt-1 text-xl font-semibold text-emerald-400">
            {planLabel(account.plan)}
          </p>
          {#if account.subscription_status}
            <p class="text-xs text-zinc-500">
              {$_("account.status_" + account.subscription_status, {
                default: account.subscription_status,
              })}
              {#if account.cancel_at}
                <!-- A scheduled cancellation means the period end is NOT a
                     renewal — it's the end. Show only "cancels on", never
                     "renews on", so the two dates don't contradict. -->
                · {$_("account.cancels_on", {
                  values: { date: fmtDate(account.cancel_at) },
                })}
              {:else if account.renews_at}
                · {$_("account.renews_on", {
                  values: { date: fmtDate(account.renews_at) },
                })}
              {/if}
            </p>
          {/if}
        </div>
        <div class="flex flex-col items-end gap-2">
          {#if account.plan === "free"}
            <Button variant="primary" onclick={() => openUpgradePage()}>
              <ArrowUpRight size={14} />
              {$_("account.upgrade")}
            </Button>
          {/if}
          {#if account.plan !== "free"}
            <Button variant="ghost" onclick={openBillingPortal}>
              <CreditCard size={14} />
              {$_("account.manage_billing")}
            </Button>
          {/if}
          <button
            type="button"
            class="text-xs text-zinc-400 underline-offset-2 hover:text-zinc-200 hover:underline"
            onclick={() => (compareOpen = true)}
          >
            {$_("account.compare_plans")}
          </button>
        </div>
      </div>

      <!-- Storage -->
      {#if storageView}
        <div class="mb-4">
          <div class="mb-1 flex items-center justify-between text-sm">
            <span class="flex items-center gap-2 text-zinc-300">
              <HardDrive size={14} />
              {$_("account.storage")}
            </span>
            <span class="font-mono text-xs text-zinc-400">
              {storageView.usedLabel} / {storageView.capLabel}
              {#if !storageView.unlimited}
                · {storageView.pct}%
              {/if}
            </span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full rounded-full transition-all duration-500 {storageView.color}"
              style={`width: ${storageView.unlimited ? 100 : storageView.pct}%`}
            ></div>
          </div>
        </div>
      {/if}

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {#if devicesView}
          <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
            <p class="flex items-center gap-2 text-xs text-zinc-500">
              <Layers size={12} />
              {$_("account.devices")}
            </p>
            <p class="mt-1 text-sm text-zinc-200">
              {devicesView.used} / {devicesView.cap}
            </p>
          </div>
        {/if}
        {#if savesView}
          <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
            <p class="flex items-center gap-2 text-xs text-zinc-500">
              <Layers size={12} />
              {$_("account.saves")}
            </p>
            <p class="mt-1 text-sm text-zinc-200">
              {savesView.used} / {savesView.cap}
            </p>
          </div>
        {/if}
        <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
          <p class="flex items-center gap-2 text-xs text-zinc-500">
            <Clock size={12} />
            {$_("account.history")}
          </p>
          <p class="mt-1 text-sm text-zinc-200">
            {account.version_history_forever
              ? $_("account.history_forever")
              : "—"}
          </p>
        </div>
        {#if account.max_save_size_bytes > 0}
          <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
            <p class="flex items-center gap-2 text-xs text-zinc-500">
              <FileArchive size={12} />
              {$_("account.max_save_size")}
            </p>
            <p class="mt-1 text-sm text-zinc-200">
              {formatBytes(account.max_save_size_bytes)}
            </p>
          </div>
        {/if}
        {#if account.bandwidth_quota_bytes > 0}
          <div class="rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3.5">
            <p class="flex items-center gap-2 text-xs text-zinc-500">
              <Gauge size={12} />
              {$_("account.bandwidth")}
            </p>
            <p class="mt-1 text-sm text-zinc-200">
              {$_("account.bandwidth_value", {
                values: {
                  amount: formatBytes(account.bandwidth_quota_bytes),
                  minutes: Math.round(account.bandwidth_window_secs / 60),
                },
              })}
            </p>
          </div>
        {/if}
      </div>
    </Card>

    <Card class="mb-4">
      <h3 class="text-sm font-semibold">{$_("account.data_section")}</h3>
      <p class="mt-1 text-sm text-zinc-400">
        {$_("account.data_section_body")}
      </p>
      <div class="mt-3 flex flex-wrap gap-2">
        <Button
          variant="secondary"
          loading={busyAction === "export"}
          disabled={busyAction !== null}
          onclick={handleExport}
        >
          <Download size={14} />
          {$_("account.export_all")}
        </Button>
      </div>
    </Card>

    <Card class="mb-4 border-rose-900/50 bg-rose-950/20">
      <h3 class="flex items-center gap-2 text-sm font-semibold text-rose-300">
        <AlertTriangle size={14} />
        {$_("account.danger_zone")}
      </h3>
      <p class="mt-1 text-sm text-rose-200/70">
        {$_("account.danger_zone_body")}
      </p>
      <div class="mt-3 flex flex-wrap gap-2">
        <Button
          variant="secondary"
          loading={busyAction === "logout"}
          disabled={busyAction !== null}
          onclick={handleLogout}
        >
          <LogOut size={14} />
          {$_("account.sign_out")}
        </Button>
        <Button
          variant="secondary"
          class="!bg-rose-900/40 !text-rose-200 hover:!bg-rose-900/60"
          disabled={busyAction !== null}
          onclick={openDeleteModal}
        >
          <Trash2 size={14} />
          {$_("account.delete_account")}
        </Button>
      </div>
    </Card>
  {/if}
</div>

<Modal
  open={confirmDeleteOpen}
  title={$_("account.delete_modal_title")}
  dismissible={true}
  onClose={() => (confirmDeleteOpen = false)}
>
  <p class="text-sm text-zinc-300">
    {$_("account.delete_modal_body")}
  </p>
  <p class="mt-2 text-xs text-zinc-500">
    {$_("account.delete_modal_grace")}
  </p>
  <div class="mt-4">
    <label class="block text-xs text-zinc-400" for="delete-confirm-input">
      {$_("account.delete_type_confirm")}
    </label>
    <input
      id="delete-confirm-input"
      type="text"
      bind:value={deleteConfirmation}
      placeholder="DELETE"
      class="mt-1 w-full rounded-md border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm font-mono text-zinc-100 placeholder:text-zinc-600 focus:border-rose-500 focus:outline-none"
      autocomplete="off"
      spellcheck="false"
    />
  </div>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (confirmDeleteOpen = false)}
      disabled={busyAction === "delete"}
    >
      {$_("common.cancel")}
    </Button>
    <Button
      variant="secondary"
      class="!bg-rose-600 !text-zinc-50 hover:!bg-rose-500"
      loading={busyAction === "delete"}
      disabled={
        busyAction !== null ||
        deleteConfirmation.trim().toUpperCase() !== "DELETE"
      }
      onclick={handleDelete}
    >
      <Trash2 size={14} />
      {$_("account.delete_confirm")}
    </Button>
  {/snippet}
</Modal>

<UpgradePlanModal
  open={compareOpen}
  onClose={() => (compareOpen = false)}
/>
