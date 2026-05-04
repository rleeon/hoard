<script lang="ts">
  /**
   * Dashboard — live view of every tracked save.
   *
   * Hydrates from `list_tracked_saves` and then reactively renders status
   * pills driven by the agent activity store (which subscribes to
   * `agent://*` events at boot time).
   */
  import { onMount } from "svelte";
  import {
    LogOut,
    PlayCircle,
    Clock,
    UploadCloud,
    Check,
    AlertTriangle,
    CircleDot,
    RefreshCw,
  } from "lucide-svelte";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import * as api from "../lib/api";
  import type { TrackedSave } from "../lib/api";
  import { auth, signOut } from "../lib/stores/auth";
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
      toastSuccess("Signed out.");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      signingOut = false;
    }
  }

  async function backupNow(saveId: string) {
    try {
      await api.backupNow(saveId);
      toastSuccess("Backup queued.");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  function pillFor(saveId: string) {
    const a = $activity[saveId];
    if (!a) return { label: "Idle", icon: CircleDot, klass: "text-zinc-400" };
    switch (a.state) {
      case "running":
        return {
          label: "Game running",
          icon: PlayCircle,
          klass: "text-sky-400",
        };
      case "scheduled": {
        const secs = Math.max(
          0,
          Math.round(((a.next_backup_at ?? now) - now) / 1000),
        );
        return {
          label: `Backup in ${secs}s`,
          icon: Clock,
          klass: "text-amber-400",
        };
      }
      case "uploading":
        return {
          label: "Uploading…",
          icon: UploadCloud,
          klass: "text-amber-400",
        };
      case "ok":
        return {
          label: a.last_version
            ? `Saved (v${a.last_version})`
            : "Saved",
          icon: Check,
          klass: "text-emerald-400",
        };
      case "failed":
        return {
          label: a.will_retry ? "Failed — retrying" : "Failed",
          icon: AlertTriangle,
          klass: "text-red-400",
        };
      default:
        return { label: "Idle", icon: CircleDot, klass: "text-zinc-400" };
    }
  }
</script>

<div class="mx-auto max-w-5xl px-8 py-8">
  <header class="mb-6 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">
        {#if $auth.user}
          Welcome back, {$auth.user.username}
        {:else}
          Dashboard
        {/if}
      </h1>
      <p class="mt-1 flex items-center gap-2 text-sm text-zinc-400">
        <span
          class="inline-flex h-2 w-2 rounded-full {$status.running
            ? 'bg-emerald-400 animate-pulse'
            : 'bg-zinc-600'}"
        ></span>
        {$status.running ? "Live agent watching" : "Agent offline"}
        {#if $status.running}
          · {saves.length} tracked save{saves.length === 1 ? "" : "s"}
        {/if}
      </p>
    </div>
    <Button
      variant="ghost"
      onclick={handleLogout}
      loading={signingOut}
      aria-label="Sign out"
    >
      <LogOut size={16} />
      Sign out
    </Button>
  </header>

  {#if loading}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">Loading…</div>
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
          No saves tracked yet.
        </h2>
        <p class="mx-auto mt-2 max-w-md text-sm text-zinc-400">
          Head to the Library and pick a game — Hoard will auto-back-up
          its saves whenever they change.
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
            variant="secondary"
            size="md"
            onclick={() => backupNow(save.save_id)}
            disabled={!$status.running}
            title={!$status.running
              ? "Start the agent first"
              : "Force a backup now"}
          >
            <UploadCloud size={14} />
            Back up
          </Button>
        </div>
      {/each}
    </div>
  {/if}
</div>
