<script lang="ts">
  /**
   * Account page, the entry point for everything Hoard Cloud.
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
  import { LogOut, ArrowUpRight, Mail, Download, Trash2, RefreshCw, ShieldCheck, AlertTriangle, CreditCard, HardDrive, Layers, Clock, FileArchive, Gauge, Server } from "@lucide/svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import { openLiberate } from "../lib/stores/liberate";
  import {
    cloud,
    hydrateCloud,
    refreshCloud,
    startCloudLogin,
    logoutCloud,
    exportAllCloudData,
    exportStatusCloud,
    downloadCloudExport,
    deleteCloudAccount,
    openBillingPortal,
    planLabel,
    type CloudExportStatus,
  } from "../lib/stores/cloud";
  import { auth, signOut, refreshQuota } from "../lib/stores/auth";
  import { remoteDevices, refreshDevices } from "../lib/stores/devices";
  import { healthCheck, type HealthInfo } from "../lib/api";
  import { toastError, toastInfo } from "../lib/stores/toasts";
  import { clearOnboarding, clearTourSeen } from "../lib/stores/onboarding";
  import { formatBytes } from "../lib/utils/format";

  let busyAction = $state<
    "signin" | "logout" | "refresh" | "export" | "delete" | null
  >(null);
  let confirmDeleteOpen = $state(false);
  let deleteConfirmation = $state("");

  // Latest export job + a poll while it's building. The worker is async, so the
  // download link appears here (and by email) once the ZIP is ready.
  let exportState = $state<CloudExportStatus | null>(null);
  let exportPollTimer: ReturnType<typeof setInterval> | null = null;

  const exportBusy = $derived(
    exportState?.status === "pending" || exportState?.status === "running",
  );

  onMount(async () => {
    if (!$cloud.hydrated) {
      await hydrateCloud();
    }
    if ($cloud.account) {
      await loadExportStatus();
    } else if ($auth.user) {
      await probeServer();
    }
  });

  onDestroy(stopExportPoll);

  async function loadExportStatus() {
    try {
      exportState = await exportStatusCloud();
      if (exportState.status === "pending" || exportState.status === "running") {
        startExportPoll();
      }
    } catch {
      // Non-fatal: the export card just won't show prior state.
    }
  }

  function startExportPoll() {
    if (exportPollTimer) return;
    exportPollTimer = setInterval(async () => {
      try {
        exportState = await exportStatusCloud();
        const s = exportState.status;
        if (s !== "pending" && s !== "running") stopExportPoll();
      } catch {
        stopExportPoll();
      }
    }, 4000);
  }

  function stopExportPoll() {
    if (exportPollTimer) {
      clearInterval(exportPollTimer);
      exportPollTimer = null;
    }
  }

  async function handleDownloadExport() {
    const url = exportState?.download_url;
    if (!url) return;
    try {
      await downloadCloudExport(url);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

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
      // made it look like the session "came back". Clear the tour too so it
      // replays when you sign into another account.
      await clearOnboarding();
      await clearTourSeen();
      toastInfo($_("account.signed_out"));
      push("/onboarding/language");
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
    if (busyAction || exportBusy) return;
    busyAction = "export";
    try {
      const job = await exportAllCloudData();
      toastInfo($_("account.export_started"));
      exportState = {
        job_id: job.job_id,
        status: job.status as CloudExportStatus["status"],
        requested_at: null,
        size_bytes: null,
        expires_at: null,
        download_url: null,
        error: null,
      };
      startExportPoll();
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
      // Same exit as logout: drop the local session and land on the welcome
      // flow instead of sitting on a stale /account view of a deleted account.
      await logoutCloud();
      await clearOnboarding();
      await clearTourSeen();
      confirmDeleteOpen = false;
      toastInfo($_("account.delete_scheduled"));
      push("/onboarding/language");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busyAction = null;
    }
  }

  // ---- derived view models -------------------------------------------

  const account = $derived($cloud.account);

  /**
   * The self-hosted session, when this page has no Cloud account to render.
   *
   * This page was Cloud-only: someone running their own server got the sign-up
   * pitch for a service they had deliberately not joined, with no view of the
   * server they *were* signed in to, and, because the sidebar keys its first
   * entry off the cloud session too, a permanent "Sign in" while their backups
   * were reaching their own box just fine.
   *
   * Cloud wins when both sessions exist: that's the one with a plan, billing
   * and an export job behind it. The self-hosted card stays in Settings for it.
   */
  const server = $derived(account ? null : $auth.user);

  let serverHealth = $state<HealthInfo | null>(null);
  let forgetOpen = $state(false);
  let forgetting = $state(false);

  const serverStorage = $derived.by(() => {
    if (!server || server.storage_quota_bytes <= 0) return null;
    const pct = Math.min(
      100,
      Math.round((server.storage_used_bytes / server.storage_quota_bytes) * 100),
    );
    return {
      usedLabel: formatBytes(server.storage_used_bytes),
      capLabel: formatBytes(server.storage_quota_bytes),
      pct,
      color:
        pct >= 90 ? "bg-rose-500" : pct >= 75 ? "bg-amber-500" : "bg-emerald-500",
    };
  });

  /** `null` on a server too old to report the limit, a dash, never a zero. */
  const maxSnapshotLabel = $derived(
    server?.max_snapshot_size_bytes && server.max_snapshot_size_bytes > 0
      ? formatBytes(server.max_snapshot_size_bytes)
      : null,
  );

  function capLabel(cap: number | null | undefined): string {
    return cap == null ? $_("account.selfhost_unlimited") : String(cap);
  }

  function uptimeLabel(secs: number): string {
    const days = Math.floor(secs / 86400);
    if (days >= 1) return $_("account.uptime_days", { values: { days } });
    const hours = Math.floor(secs / 3600);
    if (hours >= 1) return $_("account.uptime_hours", { values: { hours } });
    return $_("account.uptime_minutes", {
      values: { minutes: Math.max(1, Math.floor(secs / 60)) },
    });
  }

  /** Version + uptime of the box we're pointed at. Best-effort: an unreachable
   *  server leaves the tile blank rather than painting an error over a page
   *  whose other numbers are still valid. */
  async function probeServer() {
    if (!server) return;
    try {
      serverHealth = await healthCheck(server.server_url);
    } catch {
      serverHealth = null;
    }
    void refreshDevices();
  }

  async function handleServerRefresh() {
    busyAction = "refresh";
    try {
      await refreshQuota();
      await probeServer();
    } catch (e) {
      toastError(String(e));
    } finally {
      busyAction = null;
    }
  }

  async function handleForget() {
    forgetting = true;
    try {
      await signOut();
      forgetOpen = false;
      toastInfo($_("account.signed_out"));
    } catch (e) {
      toastError(String(e));
    } finally {
      forgetting = false;
    }
  }

  /** Storage progress bar, 0 means "unlimited", which we surface as a
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
        status: "ok" as const,
      };
    }
    const pct = Math.max(
      0,
      Math.min(100, Math.round((a.storage_used_bytes / a.storage_limit_bytes) * 100)),
    );
    // The server reports the authoritative pressure level (it knows the
    // plan-specific purge threshold: 80% free / 90% pro). Prefer it; fall back
    // to the raw % only for older servers that don't send `storage_status`.
    const status = a.storage_status ?? "ok";
    const color =
      // Sky first: inside a downgrade window the account is *fine*, it still
      // has its old limit and nothing is being deleted. Painting it red or
      // amber would announce a problem that hasn't happened yet.
      status === "grace"
        ? "bg-sky-500"
        : status === "full" || pct >= 100
          ? "bg-rose-500"
          : status === "purging"
            ? "bg-amber-500"
            : pct >= 90
              ? "bg-rose-500"
              : pct >= 75
                ? "bg-amber-500"
                : "bg-emerald-500";
    return {
      usedLabel: formatBytes(a.storage_used_bytes),
      capLabel: formatBytes(a.storage_limit_bytes),
      pct,
      color,
      unlimited: false,
      status,
    };
  });

  /** Scheduled storage downgrade not yet in effect. While this is set the user
   *  keeps their current (larger) limit and nothing is purged, they have
   *  until `at` to export or trim. `null` when no change is pending. */
  const pendingDowngrade = $derived.by(() => {
    const a = account;
    if (!a?.storage_limit_change_at || a.pending_storage_limit_bytes == null) {
      return null;
    }
    const at = new Date(a.storage_limit_change_at);
    const days = Math.max(0, Math.ceil((at.getTime() - Date.now()) / 86_400_000));
    return {
      newLimitLabel: formatBytes(a.pending_storage_limit_bytes),
      dateLabel: at.toLocaleDateString(),
      days,
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

  // Render `renews_at` / `cancel_at` as the user's local date, full RFC3339
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
  {:else if server}
    <!-- ───────────── Self-hosted: the server IS the account ──────── -->
    <Card class="mb-4">
      <div class="flex items-start justify-between gap-4">
        <div class="flex min-w-0 items-center gap-3">
          <span
            class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-sky-500/10 text-sky-300 ring-1 ring-sky-500/30"
          >
            <Server size={22} />
          </span>
          <div class="min-w-0">
            <p class="text-xs uppercase tracking-wide text-zinc-500">
              {$_("settings.signed_in_as")}
            </p>
            <p class="mt-1 flex items-center gap-2 truncate text-lg font-medium">
              {server.username}
              {#if server.is_admin}
                <span
                  class="rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-zinc-300"
                >
                  {$_("onboarding_done.administrator")}
                </span>
              {/if}
            </p>
            <p class="truncate text-xs text-zinc-500">{server.server_url}</p>
          </div>
        </div>
        <Button
          variant="ghost"
          loading={busyAction === "refresh"}
          disabled={busyAction !== null}
          onclick={handleServerRefresh}
          aria-label={$_("account.refresh")}
        >
          <RefreshCw size={14} />
          {$_("account.refresh")}
        </Button>
      </div>
    </Card>

    <Card class="mb-4">
      {#if serverStorage}
        <div class="mb-4">
          <div class="mb-1 flex items-center justify-between text-sm">
            <span class="flex items-center gap-2 text-zinc-300">
              <HardDrive size={14} />
              {$_("account.storage")}
            </span>
            <span class="font-mono text-xs text-zinc-400">
              {serverStorage.usedLabel} / {serverStorage.capLabel} · {serverStorage.pct}%
            </span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full rounded-full transition-all duration-500 {serverStorage.color}"
              style={`width: ${serverStorage.pct}%`}
            ></div>
          </div>
        </div>
      {/if}

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <!-- The ceiling that turns a backup into a 413. It is the operator's
             own knob, and until it was shown here the only way to learn it
             existed was to hit it. -->
        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
          <p class="flex items-center gap-2 text-xs text-zinc-500">
            <FileArchive size={12} />
            {$_("account.selfhost_max_snapshot")}
          </p>
          <p class="mt-1 text-sm text-zinc-200">{maxSnapshotLabel ?? "—"}</p>
          {#if maxSnapshotLabel}
            <p class="mt-1 text-[11px] leading-snug text-zinc-500">
              {$_("account.selfhost_max_snapshot_hint")}
            </p>
          {/if}
        </div>

        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
          <p class="flex items-center gap-2 text-xs text-zinc-500">
            <Clock size={12} />
            {$_("account.selfhost_versions")}
          </p>
          <p class="mt-1 text-sm text-zinc-200">{capLabel(server.max_versions)}</p>
          <p class="mt-1 text-[11px] leading-snug text-zinc-500">
            {$_("account.selfhost_versions_manual", {
              values: { count: capLabel(server.max_manual_versions) },
            })}
          </p>
        </div>

        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
          <p class="flex items-center gap-2 text-xs text-zinc-500">
            <Layers size={12} />
            {$_("account.devices")}
          </p>
          <p class="mt-1 text-sm text-zinc-200">
            {$remoteDevices.length > 0 ? $remoteDevices.length : "—"}
          </p>
        </div>

        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5 sm:col-span-3">
          <p class="flex items-center gap-2 text-xs text-zinc-500">
            <Server size={12} />
            {$_("account.selfhost_server")}
          </p>
          <p class="mt-1 text-sm text-zinc-200">
            {#if serverHealth}
              {serverHealth.version} · {uptimeLabel(serverHealth.uptime_secs)}
            {:else}
              —
            {/if}
          </p>
        </div>
      </div>
    </Card>

    <Card class="mb-4">
      <h3 class="text-sm font-semibold">{$_("account.selfhost_data_title")}</h3>
      <p class="mt-1 text-sm text-zinc-400">
        {$_("account.selfhost_data_body")}
      </p>
      <Button variant="danger" class="mt-3" onclick={() => (forgetOpen = true)}>
        <LogOut size={14} />
        {$_("settings.forget_server")}
      </Button>
      <p class="mt-2 text-xs leading-relaxed text-zinc-500">
        {$_("settings.forget_server_desc")}
      </p>
    </Card>

    <Card>
      <h3 class="text-sm font-semibold">{$_("account.selfhost_cloud_title")}</h3>
      <p class="mt-1 text-sm text-zinc-400">
        {$_("account.selfhost_cloud_body")}
      </p>
      <Button
        variant="secondary"
        class="mt-3"
        loading={busyAction === "signin"}
        disabled={busyAction !== null}
        onclick={handleSignIn}
      >
        <ArrowUpRight size={14} />
        {$_("account.selfhost_cloud_action")}
      </Button>
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
        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
          <p class="text-xs font-medium text-emerald-400">
            {$_("account.feature_storage")}
          </p>
          <p class="mt-1 text-sm text-zinc-300">
            {$_("account.feature_storage_body")}
          </p>
        </div>
        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
          <p class="text-xs font-medium text-emerald-400">
            {$_("account.feature_sync")}
          </p>
          <p class="mt-1 text-sm text-zinc-300">
            {$_("account.feature_sync_body")}
          </p>
        </div>
        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
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
            <Button variant="primary" onclick={() => push("/pro")}>
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
          {#if pendingDowngrade}
            <div class="mt-2 rounded-lg border border-sky-500/40 bg-sky-500/10 p-2.5">
              <p class="text-xs text-sky-200/90">
                {$_("account.storage_downgrade_pending", {
                  values: {
                    size: pendingDowngrade.newLimitLabel,
                    date: pendingDowngrade.dateLabel,
                    days: pendingDowngrade.days,
                  },
                })}
              </p>
              <div class="mt-2">
                <Button
                  variant="secondary"
                  loading={busyAction === "export" || exportBusy}
                  disabled={busyAction !== null || exportBusy}
                  onclick={handleExport}
                >
                  <Download size={14} />
                  {$_("account.export_all")}
                </Button>
              </div>
            </div>
          {/if}
          {#if storageView.status === "purging"}
            <p class="mt-1.5 text-xs text-amber-400">{$_("account.storage_purging")}</p>
          {:else if storageView.status === "full"}
            <p class="mt-1.5 text-xs text-rose-400">{$_("account.storage_full")}</p>
            <div class="mt-2">
              <Button onclick={openLiberate}>
                <HardDrive size={14} />
                {$_("liberate.cta")}
              </Button>
            </div>
          {/if}
          {#if storageView.status === "purging" || storageView.status === "full"}
            <div
              class="mt-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-2.5"
            >
              <p class="text-xs text-amber-200/90">{$_("account.storage_export_warning")}</p>
              <div class="mt-2">
                <Button
                  variant="secondary"
                  loading={busyAction === "export" || exportBusy}
                  disabled={busyAction !== null || exportBusy}
                  onclick={handleExport}
                >
                  <Download size={14} />
                  {$_("account.export_all")}
                </Button>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {#if devicesView}
          <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
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
          <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
            <p class="flex items-center gap-2 text-xs text-zinc-500">
              <Layers size={12} />
              {$_("account.saves")}
            </p>
            <p class="mt-1 text-sm text-zinc-200">
              {savesView.used} / {savesView.cap}
            </p>
          </div>
        {/if}
        <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
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
          <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
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
          <div class="rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3.5">
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
          loading={busyAction === "export" || exportBusy}
          disabled={busyAction !== null || exportBusy}
          onclick={handleExport}
        >
          <Download size={14} />
          {$_("account.export_all")}
        </Button>
        {#if exportState?.status === "done" && exportState.download_url}
          <Button variant="primary" onclick={handleDownloadExport}>
            <Download size={14} />
            {$_("account.export_download")}
          </Button>
        {/if}
      </div>
      {#if exportBusy}
        <p class="mt-2 text-xs text-zinc-400">{$_("account.export_building")}</p>
      {:else if exportState?.status === "done" && exportState.download_url}
        <p class="mt-2 text-xs text-emerald-300/90">
          {$_("account.export_ready")}
        </p>
      {:else if exportState?.status === "expired" || (exportState?.status === "done" && !exportState.download_url)}
        <p class="mt-2 text-xs text-zinc-400">{$_("account.export_expired")}</p>
      {:else if exportState?.status === "failed"}
        <p class="mt-2 text-xs text-rose-400">{$_("account.export_failed")}</p>
      {/if}
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
  open={forgetOpen}
  title={$_("settings.forget_confirm_title")}
  dismissible={!forgetting}
  onClose={() => (forgetOpen = false)}
>
  <p class="text-sm text-zinc-300">
    {$_("settings.forget_confirm_body", {
      values: { url: server?.server_url ?? "—" },
    })}
  </p>
  {#snippet footer()}
    <Button variant="secondary" disabled={forgetting} onclick={() => (forgetOpen = false)}>
      {$_("common.cancel")}
    </Button>
    <Button variant="danger" loading={forgetting} onclick={handleForget}>
      {$_("settings.forget_confirm_cta")}
    </Button>
  {/snippet}
</Modal>

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

<!-- El diálogo lo monta `App.svelte` a nivel de shell (hace falta desde
     cualquier pantalla, no sólo desde aquí); esta página sólo lo abre. -->
