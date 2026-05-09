<script lang="ts">
  /**
   * Dashboard — live view of every tracked save.
   *
   * Hydrates from `list_tracked_saves` and then reactively renders status
   * pills driven by the agent activity store (which subscribes to
   * `agent://*` events at boot time).
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import {
    LogOut,
    PlayCircle,
    Clock,
    UploadCloud,
    Check,
    AlertTriangle,
    CircleDot,
    RefreshCw,
    History,
    PauseCircle,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import QuotaBar from "../lib/components/QuotaBar.svelte";
  import * as api from "../lib/api";
  import type { TrackedSave } from "../lib/api";
  import { auth, refreshQuota, signOut } from "../lib/stores/auth";
  import { activity, status } from "../lib/stores/agent";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  let saves = $state<TrackedSave[]>([]);
  let loading = $state(true);
  let signingOut = $state(false);
  let now = $state(Date.now());

  // Tick once a second so "next backup in 28s" countdowns animate.
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  // Poll the storage quota every 30s while the dashboard is open so the
  // QuotaBar tracks reality after backups land. The first poll runs
  // immediately on mount via `hydrateAuth`, so this just keeps it warm.
  $effect(() => {
    const id = setInterval(() => {
      refreshQuota().catch(() => {});
    }, 30_000);
    return () => clearInterval(id);
  });

  onMount(async () => {
    try {
      saves = await api.listTrackedSaves();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  });

  async function handleLogout() {
    signingOut = true;
    try {
      await signOut();
      toastSuccess($_("dashboard.signed_out"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      signingOut = false;
    }
  }

  async function backupNow(saveId: string) {
    try {
      await api.backupNow(saveId);
      toastSuccess($_("dashboard.backup_queued"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  function pillFor(saveId: string) {
    const a = $activity[saveId];
    if (!a) return { label: $_("dashboard.pill_idle"), icon: CircleDot, klass: "text-zinc-400" };
    switch (a.state) {
      case "running":
        return {
          label: $_("dashboard.pill_running"),
          icon: PlayCircle,
          klass: "text-sky-400",
        };
      case "scheduled": {
        const secs = Math.max(
          0,
          Math.round(((a.next_backup_at ?? now) - now) / 1000),
        );
        return {
          label: $_("dashboard.pill_scheduled", { values: { seconds: secs } }),
          icon: Clock,
          klass: "text-amber-400",
        };
      }
      case "uploading":
        return {
          label: $_("dashboard.pill_uploading"),
          icon: UploadCloud,
          klass: "text-amber-400",
        };
      case "ok":
        return {
          label: a.last_version
            ? $_("dashboard.pill_saved_v", { values: { version: a.last_version } })
            : $_("dashboard.pill_saved"),
          icon: Check,
          klass: "text-emerald-400",
        };
      case "failed":
        return {
          label: a.will_retry ? $_("dashboard.pill_failed_retry") : $_("dashboard.pill_failed"),
          icon: AlertTriangle,
          klass: "text-red-400",
        };
      default:
        return { label: $_("dashboard.pill_idle"), icon: CircleDot, klass: "text-zinc-400" };
    }
  }
</script>

<div class="mx-auto max-w-5xl px-8 py-8">
  <header class="mb-6 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">
        {#if $auth.user}
          {$_("dashboard.welcome_back", { values: { username: $auth.user.username } })}
        {:else}
          {$_("dashboard.title")}
        {/if}
      </h1>
      <p class="mt-1 flex items-center gap-2 text-sm text-zinc-400">
        <span
          class="inline-flex h-2 w-2 rounded-full {$status.running
            ? 'bg-emerald-400 animate-pulse'
            : 'bg-zinc-600'}"
        ></span>
        {$status.running ? $_("dashboard.agent_watching") : $_("dashboard.agent_offline")}
        {#if $status.running}
          · {$_("dashboard.tracked_count", { values: { count: saves.length } })}
        {/if}
      </p>
    </div>
    <Button
      variant="ghost"
      onclick={handleLogout}
      loading={signingOut}
      aria-label={$_("dashboard.sign_out")}
    >
      <LogOut size={16} />
      {$_("dashboard.sign_out")}
    </Button>
  </header>

  {#if $auth.user}
    <div class="mb-4">
      <QuotaBar user={$auth.user} />
    </div>
  {/if}

  {#if loading}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">{$_("common.loading")}</div>
    </Card>
  {:else if saves.length === 0}
    <Card>
      <div class="py-16 text-center">
        <div
          class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-amber-500/10 text-amber-400 ring-1 ring-amber-500/30"
        >
          <RefreshCw size={20} />
        </div>
        <h2 class="text-base font-medium text-zinc-100">
          {$_("dashboard.no_saves_title")}
        </h2>
        <p class="mx-auto mt-2 max-w-md text-sm text-zinc-400">
          {$_("dashboard.no_saves_body")}
        </p>
      </div>
    </Card>
  {:else}
    <div class="space-y-2">
      {#each saves as save (save.save_id)}
        {@const pill = pillFor(save.save_id)}
        <div
          class="flex items-center justify-between gap-4 rounded-lg border border-zinc-800 bg-zinc-900/40 p-4"
        >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-3">
              <span class="text-sm font-medium text-zinc-100">
                {save.game_slug}
              </span>
              <span class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
                {save.label}
              </span>
              {#if save.paused}
                <span
                  class="inline-flex items-center gap-1 rounded bg-amber-500/10 px-2 py-0.5 text-xs text-amber-400 ring-1 ring-amber-500/30"
                >
                  <PauseCircle size={12} /> {$_("dashboard.paused")}
                </span>
              {/if}
            </div>
            <p
              class="mt-1 truncate font-mono text-xs text-zinc-500"
              title={save.local_path}
            >
              {save.local_path}
            </p>
          </div>
          <div
            class="flex items-center gap-2 text-xs font-medium {pill.klass}"
          >
            <pill.icon size={14} />
            <span class="whitespace-nowrap">{pill.label}</span>
          </div>
          <Button
            variant="ghost"
            size="md"
            onclick={() => push(`/history/${save.save_id}`)}
            title={$_("dashboard.history_title")}
          >
            <History size={14} />
            {$_("dashboard.history")}
          </Button>
          <Button
            variant="secondary"
            size="md"
            onclick={() => backupNow(save.save_id)}
            disabled={!$status.running || save.paused}
            title={save.paused
              ? $_("dashboard.tooltip_paused")
              : !$status.running
                ? $_("dashboard.tooltip_offline")
                : $_("dashboard.tooltip_force")}
          >
            <UploadCloud size={14} />
            {$_("dashboard.back_up")}
          </Button>
        </div>
      {/each}
    </div>
  {/if}
</div>
