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
   *
   * When the save isn't tracked locally yet (e.g. pulled from another
   * machine), restoring opens a folder-picker modal first; the chosen
   * destination is sent as `destination_override` and persisted to
   * `CliState` so subsequent restores skip the dialog.
   */
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { push } from "svelte-spa-router";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
    FolderOpen,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import { NEEDS_DESTINATION } from "../lib/api";
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

  // Pending-destination modal: shown when restore_snapshot returns
  // NEEDS_DESTINATION because there's no local mapping for this save yet.
  let pickingDestination = $state<SnapshotEntry | null>(null);

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
    if (diff < 60) return $_("history.relative_just_now");
    if (diff < 3600)
      return $_("history.relative_minutes", {
        values: { count: Math.floor(diff / 60) },
      });
    if (diff < 86400)
      return $_("history.relative_hours", {
        values: { count: Math.floor(diff / 3600) },
      });
    if (diff < 86400 * 7)
      return $_("history.relative_days", {
        values: { count: Math.floor(diff / 86400) },
      });
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

  /** Compact ISO-ish stamp for the snapshot label, e.g. "2026-05-08 14:30".
   *  Locale-aware date+time but with explicit numeric components so the
   *  result is sortable and unambiguous across regions. */
  function snapshotStamp(iso: string): string {
    const d = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, "0");
    return (
      `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
      `${pad(d.getHours())}:${pad(d.getMinutes())}`
    );
  }

  /** Run the actual restore call. Pulled out of `startRestore` so the
   *  destination-picker flow can also call it after the user chooses a
   *  folder. */
  async function performRestore(
    target: SnapshotEntry,
    destinationOverride: string | null,
  ) {
    restoring = true;
    restoreProgress = null;
    try {
      const out = await api.restoreSnapshot({
        save_id: saveId,
        version: target.version_num,
        backup_first: backupFirst,
        destination_override: destinationOverride,
      });
      const safety = out.safety_version
        ? $_("history.safety_suffix", {
            values: { version: out.safety_version },
          })
        : "";
      toastSuccess(
        $_("history.restored_toast", {
          values: {
            version: target.version_num,
            count: out.files_extracted,
            safety,
          },
        }),
      );
      restoreTarget = null;
      pickingDestination = null;
      await hydrate();
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      // Backend signals "no local path" with a sentinel string; turn that
      // into the folder-picker flow instead of bubbling the raw error to
      // the user.
      if (msg === NEEDS_DESTINATION) {
        pickingDestination = target;
        restoreTarget = null;
      } else {
        toastError(msg);
      }
    } finally {
      restoring = false;
      restoreProgress = null;
    }
  }

  async function startRestore() {
    if (!restoreTarget) return;
    await performRestore(restoreTarget, null);
  }

  async function pickDestinationAndRestore() {
    if (!pickingDestination) return;
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        title: $_("history.pick_destination_dialog_title"),
      });
      if (typeof picked !== "string" || picked.length === 0) {
        toastError($_("history.no_destination_picked"));
        return;
      }
      await performRestore(pickingDestination, picked);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await api.deleteSnapshot(saveId, deleteTarget.version_num);
      toastSuccess(
        $_("history.trashed_toast", {
          values: { version: deleteTarget.version_num },
        }),
      );
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
      toastSuccess($_("history.recovered_toast", { values: { version } }));
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function backupNow() {
    backingUp = true;
    try {
      await api.backupNow(saveId);
      toastSuccess($_("dashboard.backup_queued"));
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
        next ? $_("history.tracking_paused") : $_("history.tracking_resumed"),
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
      toastError($_("history.path_empty"));
      return;
    }
    savingPath = true;
    try {
      await api.setSaveLocalPath(saveId, newPath.trim());
      toastSuccess($_("history.path_updated"));
      editingPath = false;
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingPath = false;
    }
  }

  /** Open an OS folder dialog and pipe the result into the Edit-folder
   *  input. We don't auto-submit so the user can still tweak the path
   *  string before confirming. */
  async function browseEditFolder() {
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        title: $_("history.edit_folder_title"),
      });
      if (typeof picked === "string" && picked.length > 0) {
        newPath = picked;
      }
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
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
        ? $_("history.progress_pre_backup", {
            values: {
              downloaded: formatBytes(p.downloaded),
              total: formatBytes(p.total),
            },
          })
        : $_("history.progress_pre_backup_indeterminate");
    }
    if (p.phase === "downloading") {
      return p.total > 0
        ? $_("history.progress_downloading", {
            values: {
              downloaded: formatBytes(p.downloaded),
              total: formatBytes(p.total),
            },
          })
        : $_("history.progress_downloading_indeterminate");
    }
    return $_("history.progress_done");
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
    <ArrowLeft size={14} /> {$_("history.back_to_dashboard")}
  </button>

  {#if loading && !save}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else if !save}
    <Card>
      <div class="py-12 text-center">
        <p class="text-sm text-zinc-300">{$_("history.not_found")}</p>
        <p class="mt-1 text-xs text-zinc-500">
          {$_("history.not_found_hint")}
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
            <span
              class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400"
            >
              {save.label}
            </span>
            {#if save.paused}
              <span
                class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300"
              >
                <PauseCircle size={12} /> {$_("history.paused")}
              </span>
            {/if}
          </div>
          <p class="mt-1 flex items-center gap-2 text-sm text-zinc-400">
            <Folder size={14} class="text-zinc-500" />
            <span class="truncate font-mono text-xs">{save.local_path}</span>
          </p>
          <p class="mt-2 text-xs text-zinc-500">
            {$_("history.versions_count", {
              values: { count: snapshots.length },
            })}
            {#if save.last_backup_at}
              {$_("history.last_activity", {
                values: { when: formatRelative(save.last_backup_at) },
              })}
            {/if}
          </p>
        </div>
      </div>

      <div class="mt-4 flex flex-wrap items-center gap-2">
        <Button variant="primary" onclick={backupNow} loading={backingUp}>
          <UploadCloud size={14} /> {$_("history.back_up_now")}
        </Button>
        <Button
          variant="secondary"
          onclick={togglePause}
          loading={togglingPause}
        >
          {#if save.paused}
            <PlayCircle size={14} /> {$_("history.resume_tracking")}
          {:else}
            <PauseCircle size={14} /> {$_("history.pause_tracking")}
          {/if}
        </Button>
        <Button variant="secondary" onclick={() => (editingPath = true)}>
          <Edit3 size={14} /> {$_("history.edit_folder")}
        </Button>
      </div>
    </header>

    <section class="mb-3 flex items-center justify-between">
      <h2 class="flex items-center gap-2 text-sm font-medium text-zinc-300">
        <HistoryIcon size={14} class="text-zinc-500" />
        {$_("history.versions")}
      </h2>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        <input
          type="checkbox"
          class="h-3.5 w-3.5 rounded border-zinc-700 bg-zinc-900 text-amber-500"
          checked={includeDeleted}
          onchange={(e) =>
            (includeDeleted = (e.currentTarget as HTMLInputElement).checked)}
        />
        {$_("history.show_recoverable")}
      </label>
    </section>

    {#if snapshots.length === 0}
      <Card>
        <div class="py-12 text-center">
          <p class="text-sm text-zinc-300">
            {$_("history.no_backups_title")}
          </p>
          <p class="mt-1 text-xs text-zinc-500">
            {$_("history.no_backups_body")}
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
                aria-label={isOpen
                  ? $_("history.collapse")
                  : $_("history.expand")}
              >
                {#if isOpen}
                  <ChevronDown size={16} />
                {:else}
                  <ChevronRight size={16} />
                {/if}
              </button>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 text-sm">
                  <!-- Self-describing label: `save_v3 · 2026-05-08 14:30`.
                       The plain `v3` was useful but ambiguous in bug
                       reports; the timestamp suffix makes it copy-paste
                       friendly. -->
                  <span
                    class="font-mono font-medium text-zinc-100"
                    title={formatAbsolute(snap.created_at)}
                  >
                    {$_("history.snapshot_label", {
                      values: {
                        version: snap.version_num,
                        date: snapshotStamp(snap.created_at),
                      },
                    })}
                  </span>
                  {#if snap.is_pinned}
                    <Pin size={12} class="text-amber-400" />
                  {/if}
                  {#if isDeleted}
                    <span
                      class="inline-flex items-center gap-1 rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400"
                    >
                      <Trash2 size={10} /> {$_("history.recoverable")}
                    </span>
                  {/if}
                </div>
                <div
                  class="mt-1 flex items-center gap-3 text-xs text-zinc-500"
                >
                  <span title={formatAbsolute(snap.created_at)}>
                    {formatRelative(snap.created_at)}
                  </span>
                  <span>·</span>
                  <span>
                    {$_("history.files_count", {
                      values: { count: snap.file_count },
                    })}
                  </span>
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
                    <RotateCcw size={12} /> {$_("history.recover")}
                  </Button>
                {:else}
                  <Button
                    variant="secondary"
                    size="md"
                    onclick={() => (restoreTarget = snap)}
                  >
                    <RotateCcw size={12} /> {$_("history.restore")}
                  </Button>
                  <Button
                    variant="ghost"
                    size="md"
                    onclick={() => (deleteTarget = snap)}
                    aria-label={$_("history.delete_aria")}
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
                      <li
                        class="flex items-center justify-between gap-3 py-1.5"
                      >
                        <span
                          class="truncate font-mono text-xs text-zinc-300"
                        >
                          {f.relative_path}
                        </span>
                        <span class="shrink-0 text-xs text-zinc-500">
                          {formatBytes(f.size_bytes)}
                        </span>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="text-xs text-zinc-500">
                    {$_("history.loading_files")}
                  </p>
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
  title={restoreTarget
    ? $_("history.restore_title", {
        values: { version: restoreTarget.version_num },
      })
    : ""}
  description={save
    ? $_("history.restore_description", { values: { path: save.local_path } })
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
        onchange={(e) =>
          (backupFirst = (e.currentTarget as HTMLInputElement).checked)}
      />
      <span>
        <span class="font-medium text-zinc-100">
          {$_("history.backup_first_label")}
        </span>
        <span class="mt-0.5 block text-xs text-zinc-400">
          {$_("history.backup_first_hint")}
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
      <div
        class="flex items-start gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-200"
      >
        <AlertTriangle size={14} class="mt-0.5 shrink-0" />
        <span>{$_("history.restore_warning")}</span>
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (restoreTarget = null)}
      disabled={restoring}
    >
      {$_("common.cancel")}
    </Button>
    <Button variant="primary" onclick={startRestore} loading={restoring}>
      {$_("history.restore")}
    </Button>
  {/snippet}
</Modal>

<!-- Pick a destination when the save isn't tracked locally yet. The
     primary button opens the OS folder picker; on success we re-call
     restore with `destination_override`. -->
<Modal
  open={!!pickingDestination}
  title={$_("history.choose_destination_title")}
  description={$_("history.choose_destination_description")}
  dismissible={!restoring}
  onClose={() => {
    if (!restoring) pickingDestination = null;
  }}
>
  <p class="text-sm text-zinc-300">
    {$_("history.choose_destination_description")}
  </p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (pickingDestination = null)}
      disabled={restoring}
    >
      {$_("common.cancel")}
    </Button>
    <Button
      variant="primary"
      onclick={pickDestinationAndRestore}
      loading={restoring}
    >
      <FolderOpen size={14} />
      {$_("history.browse")}
    </Button>
  {/snippet}
</Modal>

<!-- Delete confirmation -->
<Modal
  open={!!deleteTarget}
  title={deleteTarget
    ? $_("history.delete_title", {
        values: { version: deleteTarget.version_num },
      })
    : ""}
  description={$_("history.delete_description")}
  dismissible={!deleting}
  onClose={() => {
    if (!deleting) deleteTarget = null;
  }}
>
  <p class="text-sm text-zinc-300">{$_("history.delete_body")}</p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (deleteTarget = null)}
      disabled={deleting}
    >
      {$_("common.cancel")}
    </Button>
    <Button variant="primary" onclick={confirmDelete} loading={deleting}>
      {$_("history.send_to_trash")}
    </Button>
  {/snippet}
</Modal>

<!-- Edit local path -->
<Modal
  open={editingPath}
  title={$_("history.edit_folder_title")}
  description={$_("history.edit_folder_description")}
  dismissible={!savingPath}
  onClose={() => {
    if (!savingPath) editingPath = false;
  }}
>
  <div class="flex items-end gap-2">
    <div class="flex-1">
      <Input
        label={$_("history.save_folder_label")}
        bind:value={newPath}
        placeholder="/home/you/.local/share/Game/Saves"
        disabled={savingPath}
      />
    </div>
    <Button
      variant="secondary"
      onclick={browseEditFolder}
      disabled={savingPath}
    >
      <FolderOpen size={14} />
      {$_("history.browse")}
    </Button>
  </div>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (editingPath = false)}
      disabled={savingPath}
    >
      {$_("common.cancel")}
    </Button>
    <Button variant="primary" onclick={commitPath} loading={savingPath}>
      {$_("history.update_folder")}
    </Button>
  {/snippet}
</Modal>
