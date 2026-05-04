<script lang="ts">
  /**
   * History — per-save timeline of snapshots with restore + soft-delete.
   *
   * The page accepts `:saveId` from the router; we look it up in the
   * tracked-saves list (cached from the dashboard) for the header info,
   * then fetch the full snapshot list from the server.
   *
   * Restore goes through a modal that gates the destructive bit behind a
   * confirmation, and offers a "back up current state first" safety toggle
   * (default ON). Deletion is soft on the server side — items go to a
   * recoverable trash for `retention_days`.
   */
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { push } from "svelte-spa-router";
  import {
    ArrowLeft,
    UploadCloud,
    RotateCcw,
    Trash2,
    Pin,
    PauseCircle,
    PlayCircle,
    Edit3,
    ChevronDown,
    ChevronRight,
    AlertTriangle,
    History as HistoryIcon,
    Folder,
  } from "lucide-svelte";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import type {
    SnapshotEntry,
    SnapshotDetail,
    TrackedSave,
    RestoreProgress,
  } from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  type Props = { params?: { saveId?: string } };
  let { params }: Props = $props();
  const saveId = $derived(params?.saveId ?? "");

  let save = $state<TrackedSave | null>(null);
  let snapshots = $state<SnapshotEntry[]>([]);
  let includeDeleted = $state(false);
  let loading = $state(true);
  let expanded = $state<Record<number, boolean>>({});
  let detailCache = $state<Record<number, SnapshotDetail>>({});

  // Modal state
  let restoreTarget = $state<SnapshotEntry | null>(null);
  let backupFirst = $state(true);
  let restoring = $state(false);
  let restoreProgress = $state<RestoreProgress | null>(null);

  let deleteTarget = $state<SnapshotEntry | null>(null);
  let deleting = $state(false);

  let editingPath = $state(false);
  let newPath = $state("");
  let savingPath = $state(false);

  let togglingPause = $state(false);
  let backingUp = $state(false);

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await listen<RestoreProgress>("restore://progress", (e) => {
      if (e.payload.save_id === saveId) restoreProgress = e.payload;
    });
    await hydrate();
  });

  onDestroy(() => unlisten?.());

  async function hydrate() {
    loading = true;
    try {
      const [tracked, snaps] = await Promise.all([
        api.listTrackedSaves(),
        api.listSaveSnapshots(saveId, includeDeleted),
      ]);
      save = tracked.find((s) => s.save_id === saveId) ?? null;
      // Newest first — server returns them descending too, but be defensive.
      snapshots = snaps.sort((a, b) => b.version_num - a.version_num);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  }

  async function toggleExpanded(version: number) {
    expanded = { ...expanded, [version]: !expanded[version] };
    if (expanded[version] && !detailCache[version]) {
      try {
        const d = await api.saveSnapshotDetail(saveId, version);
        detailCache = { ...detailCache, [version]: d };
      } catch (e) {
        toastError(typeof e === "string" ? e : (e as Error).message);
      }
    }
  }

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  function formatRelative(iso: string): string {
    const t = new Date(iso).getTime();
    const diff = (Date.now() - t) / 1000;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
    if (diff < 86400 * 7) return `${Math.floor(diff / 86400)} d ago`;
    return new Date(iso).toLocaleDateString();
  }

  function formatAbsolute(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function startRestore() {
    if (!restoreTarget) return;
    restoring = true;
    restoreProgress = null;
    try {
      const out = await api.restoreSnapshot({
        save_id: saveId,
        version: restoreTarget.version_num,
        backup_first: backupFirst,
      });
      const safety = out.safety_version
        ? ` (safety backup: v${out.safety_version})`
        : "";
      toastSuccess(
        `Restored v${restoreTarget.version_num} — ${out.files_extracted} file${
          out.files_extracted === 1 ? "" : "s"
        }${safety}.`,
      );
      restoreTarget = null;
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      restoring = false;
      restoreProgress = null;
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await api.deleteSnapshot(saveId, deleteTarget.version_num);
      toastSuccess(`Sent v${deleteTarget.version_num} to trash.`);
      deleteTarget = null;
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      deleting = false;
    }
  }

  async function recover(version: number) {
    try {
      await api.undeleteSnapshot(saveId, version);
      toastSuccess(`Recovered v${version}.`);
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function backupNow() {
    backingUp = true;
    try {
      await api.backupNow(saveId);
      toastSuccess("Backup queued.");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      backingUp = false;
    }
  }

  async function togglePause() {
    if (!save) return;
    togglingPause = true;
    try {
      const next = !save.paused;
      await api.setSavePaused(saveId, next);
      toastSuccess(
        next ? "Paused tracking for this save." : "Resumed tracking.",
      );
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      togglingPause = false;
    }
  }

  async function commitPath() {
    if (!newPath.trim()) {
      toastError("Path can't be empty.");
      return;
    }
    savingPath = true;
    try {
      await api.setSaveLocalPath(saveId, newPath.trim());
      toastSuccess("Updated save folder.");
      editingPath = false;
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingPath = false;
    }
  }

  $effect(() => {
    if (editingPath && save && !newPath) newPath = save.local_path;
  });

  // Toggle "show deleted" → re-fetch.
  $effect(() => {
    // Reading `includeDeleted` here makes Svelte rerun this effect when it
    // flips. We don't await — fire and forget.
    void includeDeleted;
    if (saveId) hydrate();
  });

  function progressLabel(p: RestoreProgress): string {
    if (p.phase === "pre_backup") {
      return p.total > 0
        ? `Backing up current state — ${formatBytes(p.downloaded)} / ${formatBytes(p.total)}`
        : `Backing up current state…`;
    }
    if (p.phase === "downloading") {
      return p.total > 0
        ? `Downloading — ${formatBytes(p.downloaded)} / ${formatBytes(p.total)}`
        : `Downloading…`;
    }
    return "Done.";
  }

  function progressPercent(p: RestoreProgress): number | null {
    if (p.total <= 0) return null;
    return Math.min(100, Math.round((p.downloaded / p.total) * 100));
  }
</script>

<div class="mx-auto max-w-4xl px-8 py-8">
  <button
    type="button"
    onclick={() => push("/dashboard")}
    class="mb-4 inline-flex items-center gap-2 text-sm text-zinc-400 transition-colors hover:text-zinc-100"
  >
    <ArrowLeft size={14} /> Back to dashboard
  </button>

  {#if loading && !save}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">Loading…</div>
    </Card>
  {:else if !save}
    <Card>
      <div class="py-12 text-center">
        <p class="text-sm text-zinc-300">We couldn't find that save.</p>
        <p class="mt-1 text-xs text-zinc-500">
          It may have been removed from this machine.
        </p>
      </div>
    </Card>
  {:else}
    <header class="mb-6">
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-3">
            <h1 class="truncate text-2xl font-semibold tracking-tight">
              {save.game_slug}
            </h1>
            <span class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
              {save.label}
            </span>
            {#if save.paused}
              <span
                class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300"
              >
                <PauseCircle size={12} /> Paused
              </span>
            {/if}
          </div>
          <p class="mt-1 flex items-center gap-2 text-sm text-zinc-400">
            <Folder size={14} class="text-zinc-500" />
            <span class="truncate font-mono text-xs">{save.local_path}</span>
          </p>
          <p class="mt-2 text-xs text-zinc-500">
            {snapshots.length} version{snapshots.length === 1 ? "" : "s"}
            {#if save.last_backup_at}
              · last activity {formatRelative(save.last_backup_at)}
            {/if}
          </p>
        </div>
      </div>

      <div class="mt-4 flex flex-wrap items-center gap-2">
        <Button variant="primary" onclick={backupNow} loading={backingUp}>
          <UploadCloud size={14} /> Back up now
        </Button>
        <Button variant="secondary" onclick={togglePause} loading={togglingPause}>
          {#if save.paused}
            <PlayCircle size={14} /> Resume tracking
          {:else}
            <PauseCircle size={14} /> Pause tracking
          {/if}
        </Button>
        <Button variant="secondary" onclick={() => (editingPath = true)}>
          <Edit3 size={14} /> Edit folder
        </Button>
      </div>
    </header>

    <section class="mb-3 flex items-center justify-between">
      <h2 class="flex items-center gap-2 text-sm font-medium text-zinc-300">
        <HistoryIcon size={14} class="text-zinc-500" /> Versions
      </h2>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        <input
          type="checkbox"
          class="h-3.5 w-3.5 rounded border-zinc-700 bg-zinc-900 text-amber-500"
          checked={includeDeleted}
          onchange={(e) => (includeDeleted = (e.currentTarget as HTMLInputElement).checked)}
        />
        Show recoverable (trash)
      </label>
    </section>

    {#if snapshots.length === 0}
      <Card>
        <div class="py-12 text-center">
          <p class="text-sm text-zinc-300">No backups yet.</p>
          <p class="mt-1 text-xs text-zinc-500">
            Your first backup runs as soon as the game writes new save data —
            or hit "Back up now" above.
          </p>
        </div>
      </Card>
    {:else}
      <ol class="space-y-2">
        {#each snapshots as snap (snap.version_num)}
          {@const isOpen = expanded[snap.version_num] ?? false}
          {@const isDeleted = !!snap.deleted_at}
          <li
            class="rounded-lg border border-zinc-800 bg-zinc-900/40 transition-colors
              {isDeleted ? 'opacity-70' : ''}"
          >
            <div class="flex items-center gap-3 px-4 py-3">
              <button
                type="button"
                onclick={() => toggleExpanded(snap.version_num)}
                class="text-zinc-500 transition-colors hover:text-zinc-200"
                aria-label={isOpen ? "Collapse" : "Expand"}
              >
                {#if isOpen}
                  <ChevronDown size={16} />
                {:else}
                  <ChevronRight size={16} />
                {/if}
              </button>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 text-sm">
                  <span class="font-mono font-medium text-zinc-100">
                    v{snap.version_num}
                  </span>
                  {#if snap.is_pinned}
                    <Pin size={12} class="text-amber-400" />
                  {/if}
                  {#if isDeleted}
                    <span
                      class="inline-flex items-center gap-1 rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400"
                    >
                      <Trash2 size={10} /> Recoverable
                    </span>
                  {/if}
                </div>
                <div class="mt-1 flex items-center gap-3 text-xs text-zinc-500">
                  <span title={formatAbsolute(snap.created_at)}>
                    {formatRelative(snap.created_at)}
                  </span>
                  <span>·</span>
                  <span>{snap.file_count} file{snap.file_count === 1 ? "" : "s"}</span>
                  <span>·</span>
                  <span>{formatBytes(snap.total_size_bytes)}</span>
                </div>
              </div>
              <div class="flex items-center gap-1">
                {#if isDeleted}
                  <Button
                    variant="secondary"
                    size="md"
                    onclick={() => recover(snap.version_num)}
                  >
                    <RotateCcw size={12} /> Recover
                  </Button>
                {:else}
                  <Button
                    variant="secondary"
                    size="md"
                    onclick={() => (restoreTarget = snap)}
                  >
                    <RotateCcw size={12} /> Restore
                  </Button>
                  <Button
                    variant="ghost"
                    size="md"
                    onclick={() => (deleteTarget = snap)}
                    aria-label="Delete this version"
                  >
                    <Trash2 size={12} />
                  </Button>
                {/if}
              </div>
            </div>

            {#if isOpen}
              <div class="border-t border-zinc-800 bg-zinc-950/40 px-4 py-3">
                {#if detailCache[snap.version_num]}
                  <ul class="divide-y divide-zinc-900">
                    {#each detailCache[snap.version_num].files as f (f.relative_path)}
                      <li class="flex items-center justify-between gap-3 py-1.5">
                        <span class="truncate font-mono text-xs text-zinc-300">
                          {f.relative_path}
                        </span>
                        <span class="shrink-0 text-xs text-zinc-500">
                          {formatBytes(f.size_bytes)}
                        </span>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="text-xs text-zinc-500">Loading file list…</p>
                {/if}
              </div>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  {/if}
</div>

<!-- Restore confirmation -->
<Modal
  open={!!restoreTarget}
  title={restoreTarget ? `Restore version ${restoreTarget.version_num}?` : ""}
  description={save
    ? `Files in ${save.local_path} will be overwritten with the contents of this version.`
    : ""}
  dismissible={!restoring}
  onClose={() => {
    if (!restoring) restoreTarget = null;
  }}
>
  <div class="space-y-4 text-sm text-zinc-300">
    <label class="flex items-start gap-3">
      <input
        type="checkbox"
        class="mt-0.5 h-4 w-4 shrink-0 rounded border-zinc-700 bg-zinc-900 text-amber-500"
        checked={backupFirst}
        disabled={restoring}
        onchange={(e) => (backupFirst = (e.currentTarget as HTMLInputElement).checked)}
      />
      <span>
        <span class="font-medium text-zinc-100">Back up current state first</span>
        <span class="mt-0.5 block text-xs text-zinc-400">
          Recommended. Saves what's in the folder right now as a fresh
          version, so you can roll back if you restored the wrong one.
        </span>
      </span>
    </label>

    {#if restoreProgress}
      <div>
        <div class="text-xs text-zinc-400">
          {progressLabel(restoreProgress)}
        </div>
        {#if progressPercent(restoreProgress) !== null}
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full bg-amber-500 transition-all"
              style="width: {progressPercent(restoreProgress)}%"
            ></div>
          </div>
        {:else}
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-zinc-800">
            <div class="h-full w-1/3 animate-pulse bg-amber-500"></div>
          </div>
        {/if}
      </div>
    {/if}

    {#if !restoring}
      <div class="flex items-start gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-200">
        <AlertTriangle size={14} class="mt-0.5 shrink-0" />
        <span>
          Restore overwrites the current files. If the game is running,
          quit it first or your progress in this session may be lost.
        </span>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (restoreTarget = null)}
      disabled={restoring}
    >
      Cancel
    </Button>
    <Button variant="primary" onclick={startRestore} loading={restoring}>
      Restore
    </Button>
  {/snippet}
</Modal>

<!-- Delete confirmation -->
<Modal
  open={!!deleteTarget}
  title={deleteTarget
    ? `Send version ${deleteTarget.version_num} to trash?`
    : ""}
  description="The version stays recoverable from the trash for the server's retention window (default 30 days). After that it's gone for good."
  dismissible={!deleting}
  onClose={() => {
    if (!deleting) deleteTarget = null;
  }}
>
  <p class="text-sm text-zinc-300">
    Deleted versions don't count against your storage quota and can be
    recovered from the "Show recoverable" toggle above this list.
  </p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (deleteTarget = null)}
      disabled={deleting}
    >
      Cancel
    </Button>
    <Button variant="primary" onclick={confirmDelete} loading={deleting}>
      Send to trash
    </Button>
  {/snippet}
</Modal>

<!-- Edit local path -->
<Modal
  open={editingPath}
  title="Edit save folder"
  description="Tell Hoard where this save lives now. Use the full path to the folder, exactly as it appears in your file manager."
  dismissible={!savingPath}
  onClose={() => {
    if (!savingPath) editingPath = false;
  }}
>
  <Input
    label="Save folder"
    bind:value={newPath}
    placeholder="/home/you/.local/share/Game/Saves"
    disabled={savingPath}
  />
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (editingPath = false)}
      disabled={savingPath}
    >
      Cancel
    </Button>
    <Button variant="primary" onclick={commitPath} loading={savingPath}>
      Update folder
    </Button>
  {/snippet}
</Modal>
