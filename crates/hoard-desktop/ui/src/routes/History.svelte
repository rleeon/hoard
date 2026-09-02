<script lang="ts">
  /**
   * History, per-save timeline of snapshots with restore + soft-delete.
   *
   * The page accepts `:saveId` from the router; we look it up in the
   * tracked-saves list (cached from the dashboard) for the header info,
   * then fetch the full snapshot list from the server.
   *
   * Restore goes through a modal that gates the destructive bit behind a
   * confirmation, and offers a "back up current state first" safety toggle
   * (default ON). Deletion is soft on the server side, items go to a
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
    Snowflake,
    RotateCw,
    SlidersHorizontal,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import Input from "../lib/components/Input.svelte";
  import Cover from "../lib/components/Cover.svelte";
  import * as api from "../lib/api";
  import { NEEDS_DESTINATION } from "../lib/api";
  import type {
    SnapshotEntry,
    SnapshotDetail,
    TrackedSave,
    RestoreProgress,
  } from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";
  import {
    isCloudLoggedIn,
    archivedSaves,
    refreshArchivedSaves,
    reactivateAndRefresh,
  } from "../lib/stores/cloud";

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
  // Write the snapshot's config over this machine's as well. Off by default:
  // it carries the resolution, the GPU and the paths of the PC that uploaded
  // the copy, and it is what makes the game blow up. Every restore asks again,
  // unless the game has it settled in its settings (`allow_device_local`),
  // which is exactly for the games where the config and the save are the same
  // file and answering "no" every time restores half of it.
  let allowConfig = $state(false);
  let restoring = $state(false);
  let restoreProgress = $state<RestoreProgress | null>(null);

  // Qué le va a pasar a la carpeta. Se pide al abrir el modal y no descarga
  // nada: cruza el manifiesto de la versión con lo que hay en disco. `null`
  // mientras carga; `failed` si no se pudo mirar, en cuyo caso el restore
  // sigue disponible, no saber qué cambia no es motivo para bloquearlo.
  let preview = $state<api.RestorePreview | null>(null);
  let previewFailed = $state(false);

  function openRestore(snap: SnapshotEntry) {
    restoreTarget = snap;
    // Every restore starts from the game's setting (off when undecided): the
    // choice made for ONE restore is not remembered. Set BEFORE asking for the
    // preview, which is why opening and reloading are two functions, the
    // switch reloads without resetting itself, or it turned itself off the
    // moment you touched it.
    allowConfig = save?.allow_device_local ?? false;
    loadPreview(snap);
  }

  async function loadPreview(snap: SnapshotEntry) {
    preview = null;
    previewFailed = false;
    if (!save) return;
    try {
      const out = await api.previewRestore(
        save.save_id,
        snap.version_num,
        null,
        allowConfig,
      );
      // Otro clic pudo cambiar de versión mientras esto viajaba.
      if (restoreTarget?.version_num === snap.version_num) preview = out;
    } catch {
      if (restoreTarget?.version_num === snap.version_num) previewFailed = true;
    }
  }

  let deleteTarget = $state<SnapshotEntry | null>(null);
  let deleting = $state(false);

  // Multi-select for bulk deletion. Keys are version numbers; only live
  // (non-trashed) rows are selectable. Cleared on every hydrate so a stale
  // selection can never reference versions that no longer exist.
  let selected = $state<Set<number>>(new Set());
  let bulkConfirm = $state(false);
  let bulkDeleting = $state(false);

  const selectableVersions = $derived(
    snapshots.filter((s) => !s.deleted_at).map((s) => s.version_num),
  );
  const allSelected = $derived(
    selectableVersions.length > 0 &&
      selectableVersions.every((v) => selected.has(v)),
  );

  function toggleSelected(version: number) {
    const next = new Set(selected);
    if (next.has(version)) next.delete(version);
    else next.add(version);
    selected = next;
  }

  function toggleSelectAll() {
    selected = allSelected ? new Set() : new Set(selectableVersions);
  }

  async function confirmBulkDelete() {
    if (selected.size === 0) return;
    bulkDeleting = true;
    // Oldest first, so a mid-loop failure leaves the newest history intact.
    const versions = [...selected].sort((a, b) => a - b);
    let failed = 0;
    try {
      for (const version of versions) {
        try {
          await api.deleteSnapshot(saveId, version);
        } catch (e) {
          failed += 1;
          console.warn(`bulk delete: v${version} failed:`, e);
        }
      }
      if (failed > 0) {
        toastError(
          $_("history.bulk_delete_partial", {
            values: { done: versions.length - failed, failed },
          }),
        );
      } else {
        toastSuccess(
          $_("history.bulk_deleted_toast", {
            values: { count: versions.length },
          }),
        );
      }
      bulkConfirm = false;
      await hydrate();
    } finally {
      bulkDeleting = false;
    }
  }

  let editingPath = $state(false);
  let newPath = $state("");
  let savingPath = $state(false);

  // Pending-destination modal: shown when restore_snapshot returns
  // NEEDS_DESTINATION because there's no local mapping for this save yet.
  let pickingDestination = $state<SnapshotEntry | null>(null);

  let togglingPause = $state(false);
  let backingUp = $state(false);
  let reactivating = $state(false);

  // Localised hard-delete date if this save is frozen in the black box, else
  // null. Reads `$archivedSaves` so it re-derives when the map refreshes.
  const purgeDate = $derived.by(() => {
    const iso = $archivedSaves[saveId];
    return iso ? new Date(iso).toLocaleDateString() : null;
  });

  // Sync presets, the catalog comes from the backend; `savingPreset` gates
  // the selector while a change is in flight.
  let presets = $state<string[]>([]);
  let savingPreset = $state(false);

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await listen<RestoreProgress>("restore://progress", (e) => {
      if (e.payload.save_id === saveId) restoreProgress = e.payload;
    });
    api
      .listSavePresets()
      .then((p) => (presets = p))
      .catch(() => (presets = []));
    void refreshArchivedSaves();
    // Local y barato; si falla, los grupos se quedan sin la línea de horas.
    api
      .listPlaytime()
      .then((p) => (playtime = p))
      .catch(() => (playtime = null));
    await hydrate();
  });

  onDestroy(() => unlisten?.());

  async function hydrate() {
    loading = true;
    selected = new Set();
    try {
      const [tracked, snaps] = await Promise.all([
        api.listTrackedSaves(),
        api.listSaveSnapshots(saveId, includeDeleted),
      ]);
      save = tracked.find((s) => s.save_id === saveId) ?? null;
      // Newest first, server returns them descending too, but be defensive.
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

  // Horas jugadas por día, calculadas en local por el agente (no toca la red).
  // La cabecera de cada grupo puede así decir cuánto se jugó ESE día a ESTE
  // juego, que es el contexto que una fecha sola nunca da.
  let playtime = $state<api.PlaytimeSummary | null>(null);

  const playedByDay = $derived.by(() => {
    const slug = save?.game_slug;
    const daily = playtime?.daily_by_game;
    const out: Record<string, number> = {};
    if (!slug || !daily) return out;
    for (const [day, games] of Object.entries(daily)) {
      const secs = games[slug];
      if (secs) out[day] = secs;
    }
    return out;
  });

  /** Clave de día LOCAL (`YYYY-MM-DD`), la misma forma que usa el playtime. */
  function dayKey(iso: string): string {
    const d = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }

  type SnapshotGroup = {
    key: string;
    label: string;
    snaps: SnapshotEntry[];
    playedSecs: number;
  };

  /** La lista, partida por día.
   *
   *  Un juego que autoguarda cada minuto produce cuarenta filas idénticas, y
   *  leerlas como una lista plana es leer un muro. Agrupadas por día la
   *  ráfaga se lee como lo que fue: una tarde jugando. */
  const groups = $derived.by<SnapshotGroup[]>(() => {
    const out: SnapshotGroup[] = [];
    let current: SnapshotGroup | null = null;
    for (const snap of snapshots) {
      const key = dayKey(snap.created_at);
      if (!current || current.key !== key) {
        current = {
          key,
          label: dayLabel(snap.created_at),
          snaps: [],
          playedSecs: playedByDay[key] ?? 0,
        };
        out.push(current);
      }
      current.snaps.push(snap);
    }
    return out;
  });

  function dayLabel(iso: string): string {
    const key = dayKey(iso);
    const today = dayKey(new Date().toISOString());
    if (key === today) return $_("history.group_today");
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    if (key === dayKey(yesterday.toISOString()))
      return $_("history.group_yesterday");
    return new Date(iso).toLocaleDateString(undefined, {
      weekday: "long",
      day: "numeric",
      month: "long",
    });
  }

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.round((secs % 3600) / 60);
    if (h > 0) return $_("history.dur_hm", { values: { h, m } });
    return $_("history.dur_m", { values: { m: Math.max(m, 1) } });
  }

  /** Bytes con signo: lo que esta versión pesa de más (o de menos) que la
   *  anterior. El signo es la mitad del dato, "+2 MB" y "-2 MB" cuentan
   *  historias opuestas sobre la misma partida. */
  function formatDelta(n: number): string {
    const sign = n < 0 ? "-" : "+";
    return `${sign}${formatBytes(Math.abs(n))}`;
  }

  /** Contra qué versión se compara cada fila: la anterior que exista en la
   *  lista. Un "-29 MB" a secas no dice nada; "29 MB menos que la v41" sí. */
  const previousVersion = $derived.by(() => {
    const out: Record<number, number> = {};
    for (let i = 0; i < snapshots.length - 1; i++) {
      out[snapshots[i].version_num] = snapshots[i + 1].version_num;
    }
    return out;
  });

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
        allow_config: allowConfig,
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
        $isCloudLoggedIn
          ? $_("history.cloud_deleted_toast")
          : $_("history.trashed_toast", {
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

  async function reactivate() {
    if (!save || reactivating) return;
    reactivating = true;
    try {
      await reactivateAndRefresh(saveId);
      toastSuccess(
        $_("archived.reactivated_toast", {
          values: { name: save.game_slug },
        }),
      );
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      reactivating = false;
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

  async function changePreset(value: string) {
    if (!save) return;
    // The selector emits "standard" for the inherit-global option; the
    // backend treats that (and null) as clearing the override.
    const next = value === "standard" ? null : value;
    savingPreset = true;
    try {
      await api.setSavePreset(saveId, next);
      toastSuccess($_("presets.updated"));
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingPreset = false;
    }
  }

  let savingAllowConfig = $state(false);

  /** Settle, for this game, whether a restore writes its config back. The
   *  restore dialog starts from here and automatic restores honour it, which is
   *  what makes the setting worth having.
   *
   *  It belongs to the GAME, not the folder: the backend applies it to every
   *  tracked folder of the title at once (see `set_allow_device_local`). */
  async function changeAllowConfig(allow: boolean) {
    if (!save) return;
    savingAllowConfig = true;
    try {
      await api.setSaveAllowConfig(saveId, allow);
      await hydrate();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingAllowConfig = false;
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
    // flips. We don't await, fire and forget.
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
            <!-- The game's art, same thumbnail the Library and the dashboard
                 show. It used to be a hand-drawn tile with the slug's initial,
                 which meant the one page dedicated to a single game was also
                 the only place that never showed which game it was. `Cover`
                 keeps the initial as its fallback for games with no art. -->
            <Cover
              slug={save.game_slug}
              name={save.game_slug}
              class="h-11 w-11 shrink-0 rounded-xl"
              initialClass="text-lg"
            />
            <h1
              class="font-display min-w-0 truncate text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50"
            >
              {save.game_slug}
            </h1>
            <span
              class="shrink-0 rounded-md bg-white/[0.05] px-1.5 py-0.5 text-[11px] text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
            >
              {save.label}
            </span>
            {#if save.paused}
              <span
                class="inline-flex shrink-0 items-center gap-1 rounded-md bg-amber-500/10 px-1.5 py-0.5 text-[11px] font-medium text-amber-400 ring-1 ring-inset ring-amber-500/30"
              >
                <PauseCircle size={11} /> {$_("history.paused")}
              </span>
            {/if}
            {#if purgeDate}
              <span
                class="inline-flex shrink-0 items-center gap-1 rounded-md bg-sky-500/10 px-1.5 py-0.5 text-[11px] font-medium text-sky-300 ring-1 ring-inset ring-sky-500/30"
              >
                <Snowflake size={11} /> {$_("archived.badge")}
              </span>
            {/if}
          </div>
          <p class="mt-2 flex items-center gap-2 text-sm text-zinc-400">
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
        {#if presets.length > 0}
          <label class="flex items-center gap-2 text-xs text-zinc-400">
            <span class="text-zinc-500">{$_("presets.label")}</span>
            <select
              class="rounded-md border border-white/[0.08] bg-zinc-900 px-2 py-1.5 text-xs text-zinc-200 focus:border-emerald-500/40 focus:outline-none disabled:opacity-50"
              disabled={savingPreset}
              value={save.preset ?? "standard"}
              onchange={(e) =>
                changePreset((e.currentTarget as HTMLSelectElement).value)}
            >
              {#each presets as p (p)}
                <option value={p}>{$_(`presets.${p}.label`)}</option>
              {/each}
            </select>
          </label>
        {/if}
        <!-- A cloud-only row (`orphan`) has no `state.json` entry on this
             machine, so there is nowhere to store the decision: the backend
             answered a raw "That save isn't tracked on this machine". Disable
             the button and say why, rather than letting them press something
             that cannot work. -->
        <Button
          variant="secondary"
          disabled={save.orphan || !save.local_path}
          onclick={() => changeAllowConfig(!(save?.allow_device_local ?? false))}
          loading={savingAllowConfig}
          title={save.orphan || !save.local_path
            ? $_("history.allow_config_game_orphan")
            : save.allow_device_local
              ? $_("history.allow_config_game_hint_on")
              : $_("history.allow_config_game_hint_off")}
        >
          <SlidersHorizontal size={14} />
          {#if save.allow_device_local}
            {$_("history.allow_config_game_off")}
          {:else}
            {$_("history.allow_config_game_on")}
          {/if}
        </Button>
      </div>
      <p class="mt-2 text-xs text-zinc-500">
        {#if save.orphan || !save.local_path}
          {$_("history.allow_config_game_orphan")}
        {:else if save.allow_device_local}
          {$_("history.allow_config_game_hint_on")}
        {:else}
          {$_("history.allow_config_game_hint_off")}
        {/if}
      </p>
      {#if save.preset}
        <p class="mt-2 text-xs text-zinc-500">
          {$_(`presets.${save.preset}.desc`)}
        </p>
      {/if}

      {#if purgeDate}
        <div
          class="mt-4 flex flex-col gap-3 rounded-lg border border-sky-500/30 bg-sky-500/[0.07] p-3 sm:flex-row sm:items-center sm:justify-between"
        >
          <div class="flex items-start gap-2">
            <Snowflake size={16} class="mt-0.5 shrink-0 text-sky-300" />
            <div>
              <p class="text-sm font-medium text-sky-200">
                {$_("archived.banner_title")}
              </p>
              <p class="mt-0.5 text-xs text-sky-200/80">
                {$_("archived.banner_body", { values: { date: purgeDate } })}
              </p>
            </div>
          </div>
          <button
            type="button"
            onclick={reactivate}
            disabled={reactivating}
            class="inline-flex shrink-0 items-center justify-center gap-1.5 self-start rounded-lg bg-sky-500 px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-sky-400 disabled:opacity-50 sm:self-auto"
          >
            <RotateCw size={14} class={reactivating ? "animate-spin" : ""} />
            {reactivating
              ? $_("archived.reactivating")
              : $_("archived.reactivate")}
          </button>
        </div>
      {/if}
    </header>

    <section class="mb-3 flex flex-wrap items-center justify-between gap-2">
      <h2 class="flex items-center gap-2 text-sm font-medium text-zinc-300">
        <HistoryIcon size={14} class="text-zinc-500" />
        {$_("history.versions")}
      </h2>
      <div class="flex items-center gap-4">
        {#if selectableVersions.length > 0}
          <label class="flex items-center gap-2 text-xs text-zinc-400">
            <input
              type="checkbox"
              class="h-3.5 w-3.5 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
              checked={allSelected}
              onchange={toggleSelectAll}
            />
            {$_("history.select_all")}
          </label>
        {/if}
        <label class="flex items-center gap-2 text-xs text-zinc-400">
          <input
            type="checkbox"
            class="h-3.5 w-3.5 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
            checked={includeDeleted}
            onchange={(e) =>
              (includeDeleted = (e.currentTarget as HTMLInputElement).checked)}
          />
          {$_("history.show_recoverable")}
        </label>
      </div>
    </section>

    {#if selected.size > 0}
      <!-- Bulk action bar: appears as soon as one version is ticked. -->
      <div
        class="mb-3 flex items-center justify-between gap-3 rounded-lg border border-rose-500/25 bg-rose-500/[0.06] px-3 py-2"
      >
        <span class="text-xs text-rose-200">
          {$_("history.selected_count", { values: { count: selected.size } })}
        </span>
        <div class="flex items-center gap-2">
          <Button
            variant="ghost"
            size="md"
            onclick={() => (selected = new Set())}
            disabled={bulkDeleting}
          >
            {$_("common.cancel")}
          </Button>
          <Button
            variant="danger"
            size="md"
            onclick={() => (bulkConfirm = true)}
            loading={bulkDeleting}
          >
            <Trash2 size={12} />
            {$_("history.delete_selected")}
          </Button>
        </div>
      </div>
    {/if}

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
      {#each groups as group (group.key)}
        <section class="mb-5">
          <!-- Cabecera del día. Un juego que autoguarda cada minuto llena la
               lista de filas iguales; partirla por días la vuelve legible y le
               pone al lado lo único que explica la ráfaga: cuánto se jugó. -->
          <div class="mb-2 flex items-baseline gap-2 px-1">
            <h3 class="text-xs font-medium tracking-wide text-zinc-400 uppercase">
              {group.label}
            </h3>
            <span class="text-[11px] text-zinc-600">
              {$_("history.versions_count", {
                values: { count: group.snaps.length },
              })}
              {#if group.playedSecs > 0}
                · {$_("history.played_time", {
                  values: { time: formatDuration(group.playedSecs) },
                })}
              {/if}
            </span>
          </div>
          <ol class="space-y-2.5">
            {#each group.snaps as snap (snap.version_num)}
              {@const isOpen = expanded[snap.version_num] ?? false}
              {@const isDeleted = !!snap.deleted_at}
              {@const insight = snap.insight}
              <li
                class="group rounded-xl border border-white/[0.08] bg-zinc-950/40 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-white/[0.12]
                  {isDeleted ? 'opacity-60' : ''}"
              >
                <div class="flex items-center gap-3 px-4 py-3">
                  {#if !isDeleted}
                    <input
                      type="checkbox"
                      class="h-3.5 w-3.5 shrink-0 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
                      checked={selected.has(snap.version_num)}
                      onchange={() => toggleSelected(snap.version_num)}
                      aria-label={$_("history.select_version", {
                        values: { version: snap.version_num },
                      })}
                    />
                  {/if}
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
                  <!-- El sitio de la foto de la partida. Hasta que la haya,
                       la carátula del juego: una fila con imagen se lee de un
                       vistazo, y dejar el hueco vacío ahora obligaría a
                       recolocar la fila entera después. -->
                  <Cover
                    slug={save.game_slug}
                    name={insight?.t ?? save.game_slug}
                    class="h-10 w-10 shrink-0 rounded-lg"
                    initialClass="text-sm"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2 text-sm">
                      <!-- La partida manda. Cuando el server sabe de qué va la
                           versión, el nombre del save es el titular y el
                           `v47 · fecha · equipo` baja a la línea de abajo: con
                           70 mundos en la carpeta, saber cuál se tocó importa
                           más que el número de versión. Sin insight (versiones
                           viejas, o sin manifiesto por fichero) la fila se
                           queda con la etiqueta de siempre. -->
                      {#if insight?.t}
                        <span
                          class="truncate font-medium text-zinc-100"
                          title={insight.p ?? insight.t}
                        >
                          {insight.t}
                        </span>
                      {:else}
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
                      {/if}
                      {#if snap.is_pinned}
                        <Pin size={12} class="shrink-0 text-amber-400" />
                      {/if}
                      {#if isDeleted}
                        <span
                          class="inline-flex shrink-0 items-center gap-1 rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400"
                        >
                          <Trash2 size={10} /> {$_("history.recoverable")}
                        </span>
                      {/if}
                    </div>
                    <div
                      class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-zinc-500"
                    >
                      <span class="font-mono">v{snap.version_num}</span>
                      <span>·</span>
                      <span title={formatAbsolute(snap.created_at)}>
                        {formatRelative(snap.created_at)}
                      </span>
                      {#if snap.device_name}
                        <span>·</span>
                        <span class="truncate">{snap.device_name}</span>
                      {/if}
                    </div>
                    <!-- Qué cambió, no cuánto pesa todo. El peso total se
                         repetía idéntico en las cuarenta filas de una tarde;
                         el diff es lo que distingue una versión de la de
                         antes. Sigue estando entero al desplegar. -->
                    <div
                      class="mt-1 flex flex-wrap items-center gap-1.5 text-[11px]"
                    >
                      {#if insight && (insight.c || insight.r || insight.d)}
                        {#if insight.c}
                          <span
                            class="rounded bg-white/[0.05] px-1.5 py-0.5 text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
                            title={previousVersion[snap.version_num]
                              ? $_("history.delta_vs", {
                                  values: {
                                    version: previousVersion[snap.version_num],
                                  },
                                })
                              : $_("history.delta_vs_previous")}
                          >
                            {$_("history.changed_files", {
                              values: { count: insight.c },
                            })}
                          </span>
                        {/if}
                        {#if insight.r}
                          <span
                            class="rounded bg-white/[0.05] px-1.5 py-0.5 text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
                            title={previousVersion[snap.version_num]
                              ? $_("history.delta_vs", {
                                  values: {
                                    version: previousVersion[snap.version_num],
                                  },
                                })
                              : $_("history.delta_vs_previous")}
                          >
                            {$_("history.removed_files", {
                              values: { count: insight.r },
                            })}
                          </span>
                        {/if}
                        {#if insight.d}
                          <span
                            class="rounded px-1.5 py-0.5 ring-1 ring-inset {insight.d <
                            0
                              ? 'bg-emerald-500/[0.08] text-emerald-300/90 ring-emerald-500/20'
                              : 'bg-white/[0.05] text-zinc-400 ring-white/[0.06]'}"
                            title={previousVersion[snap.version_num]
                              ? $_("history.delta_vs", {
                                  values: {
                                    version: previousVersion[snap.version_num],
                                  },
                                })
                              : $_("history.delta_vs_previous")}
                          >
                            {formatDelta(insight.d)}
                          </span>
                        {/if}
                      {:else}
                        <span
                          class="rounded bg-white/[0.05] px-1.5 py-0.5 text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
                        >
                          {$_("history.files_count", {
                            values: { count: snap.file_count },
                          })}
                        </span>
                        <span
                          class="rounded bg-white/[0.05] px-1.5 py-0.5 text-zinc-400 ring-1 ring-inset ring-white/[0.06]"
                        >
                          {formatBytes(snap.total_size_bytes)}
                        </span>
                      {/if}
                      <!-- La versión se llevó la carpeta entera; el titular
                           nombra a una sola de las partidas que hay dentro.
                           Decirlo evita que la fila prometa menos de lo que
                           restaura. -->
                      {#if insight?.t && (insight.n ?? 0) > 1}
                        <span class="text-zinc-600">
                          {$_("history.more_saves", {
                            values: { count: (insight.n ?? 1) - 1 },
                          })}
                        </span>
                      {/if}
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
                        onclick={() => openRestore(snap)}
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
                  <div
                    class="border-t border-white/[0.08] bg-zinc-950/20 px-4 py-3"
                  >
                    <p class="mb-2 text-[11px] text-zinc-500">
                      {$_("history.files_count", {
                        values: { count: snap.file_count },
                      })}
                      · {formatBytes(snap.total_size_bytes)}
                    </p>
                    {#if detailCache[snap.version_num]}
                      {#if detailCache[snap.version_num].files.length === 0}
                        <p class="text-xs text-zinc-500">
                          {$isCloudLoggedIn
                            ? $_("history.cloud_no_file_index")
                            : $_("history.no_files")}
                        </p>
                      {:else}
                        <ul class="divide-y divide-zinc-900">
                          {#each detailCache[snap.version_num].files as f (f.relative_path)}
                            <li
                              class="flex items-center justify-between gap-3 py-1.5"
                            >
                              <span
                                class="truncate font-mono text-xs {f.relative_path ===
                                insight?.p
                                  ? 'text-zinc-100'
                                  : 'text-zinc-300'}"
                              >
                                {f.relative_path}
                              </span>
                              <span class="shrink-0 text-xs text-zinc-500">
                                {formatBytes(f.size_bytes)}
                              </span>
                            </li>
                          {/each}
                        </ul>
                      {/if}
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
        </section>
      {/each}
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
        class="mt-0.5 h-4 w-4 shrink-0 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
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

    <label class="flex items-start gap-3">
      <input
        type="checkbox"
        class="mt-0.5 h-4 w-4 shrink-0 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
        checked={allowConfig}
        disabled={restoring}
        onchange={(e) => {
          allowConfig = (e.currentTarget as HTMLInputElement).checked;
          if (restoreTarget) loadPreview(restoreTarget);
        }}
      />
      <span>
        <span class="font-medium text-zinc-100">
          {$_("history.allow_config_label")}
        </span>
        <span class="mt-0.5 block text-xs text-zinc-400">
          {$_("history.allow_config_hint")}
        </span>
      </span>
    </label>

    {#if !restoring}
      <div
        class="rounded-md border border-white/[0.08] bg-zinc-900/60 p-3 text-xs"
      >
        <div class="mb-1.5 font-medium text-zinc-300">
          {$_("history.preview_title")}
        </div>
        {#if previewFailed}
          <div class="text-zinc-400">{$_("history.preview_failed")}</div>
        {:else if !preview}
          <div class="text-zinc-500">{$_("history.preview_loading")}</div>
        {:else if !preview.comparable}
          <div class="text-zinc-400">{$_("history.preview_unavailable")}</div>
        {:else if preview.modified_count === 0 && preview.added_count === 0}
          <div class="text-zinc-400">{$_("history.preview_nothing")}</div>
        {:else}
          <ul class="space-y-1 text-zinc-300">
            {#if preview.modified_count > 0}
              <li class="text-amber-200">
                {$_("history.preview_modified", {
                  values: { count: preview.modified_count },
                })}
              </li>
            {/if}
            {#if preview.added_count > 0}
              <li class="text-emerald-300">
                {$_("history.preview_added", {
                  values: { count: preview.added_count },
                })}
              </li>
            {/if}
            {#if preview.unchanged > 0}
              <li class="text-zinc-500">
                {$_("history.preview_unchanged", {
                  values: { count: preview.unchanged },
                })}
              </li>
            {/if}
          </ul>
        {/if}
        {#if preview && preview.local_only_count > 0}
          <div class="mt-1.5 text-zinc-400">
            {$_("history.preview_local_only", {
              values: { count: preview.local_only_count },
            })}
          </div>
        {/if}
      </div>
    {/if}

    {#if restoreProgress}
      <div>
        <div class="text-xs text-zinc-400">
          {progressLabel(restoreProgress)}
        </div>
        {#if progressPercent(restoreProgress) !== null}
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-zinc-800">
            <div
              class="h-full bg-emerald-500 transition-all"
              style="width: {progressPercent(restoreProgress)}%"
            ></div>
          </div>
        {:else}
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-zinc-800">
            <div class="h-full w-1/3 animate-pulse bg-emerald-500"></div>
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
    ? $isCloudLoggedIn
      ? $_("history.cloud_delete_title")
      : $_("history.delete_title", {
          values: { version: deleteTarget.version_num },
        })
    : ""}
  description={$isCloudLoggedIn
    ? $_("history.cloud_delete_description")
    : $_("history.delete_description")}
  dismissible={!deleting}
  onClose={() => {
    if (!deleting) deleteTarget = null;
  }}
>
  <p class="text-sm text-zinc-300">
    {$isCloudLoggedIn
      ? $_("history.cloud_delete_body")
      : $_("history.delete_body")}
  </p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (deleteTarget = null)}
      disabled={deleting}
    >
      {$_("common.cancel")}
    </Button>
    <Button
      variant={$isCloudLoggedIn ? "danger" : "primary"}
      onclick={confirmDelete}
      loading={deleting}
    >
      {$isCloudLoggedIn
        ? $_("history.cloud_delete_confirm")
        : $_("history.send_to_trash")}
    </Button>
  {/snippet}
</Modal>

<!-- Bulk delete confirmation -->
<Modal
  open={bulkConfirm}
  title={$_("history.bulk_delete_title", {
    values: { count: selected.size },
  })}
  description={$isCloudLoggedIn
    ? $_("history.cloud_delete_description")
    : $_("history.delete_description")}
  dismissible={!bulkDeleting}
  onClose={() => {
    if (!bulkDeleting) bulkConfirm = false;
  }}
>
  <p class="text-sm text-zinc-300">
    {$_("history.bulk_delete_body", { values: { count: selected.size } })}
    {$isCloudLoggedIn
      ? $_("history.cloud_delete_body")
      : $_("history.delete_body")}
  </p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (bulkConfirm = false)}
      disabled={bulkDeleting}
    >
      {$_("common.cancel")}
    </Button>
    <Button
      variant={$isCloudLoggedIn ? "danger" : "primary"}
      onclick={confirmBulkDelete}
      loading={bulkDeleting}
    >
      {$isCloudLoggedIn
        ? $_("history.cloud_delete_confirm")
        : $_("history.send_to_trash")}
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
