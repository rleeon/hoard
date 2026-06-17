<script lang="ts">
  /**
   * Library page — auto-detection results + tracked saves.
   *
   * On mount we hydrate from the in-memory detection cache (if a previous
   * scan happened this session) and also fetch the user's tracked saves so
   * the "tracked" badge can render immediately.
   *
   * "Scan" kicks off `scan_library`; while it runs we listen for
   * `library://scan-progress` events to drive the progress bar.
   */
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    Search as SearchIcon,
    RefreshCw,
    Plus,
    Check,
    Filter,
    HardDrive,
    Gamepad2,
    AlertTriangle,
    Trash2,
    Trash,
    FolderOpen,
    Pencil,
    RotateCcw,
    Link,
    Clock,
    X,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Cover from "../lib/components/Cover.svelte";
  import Input from "../lib/components/Input.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import * as api from "../lib/api";
  import type {
    Confidence,
    DetectedGame,
    DetectionReport,
    DetectionSource,
    PlaytimeGameInfo,
    ScanProgress,
    TrackedSave,
  } from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";
  import { showError } from "../lib/stores/error_dialog";

  let report = $state<DetectionReport | null>(null);
  let tracked = $state<TrackedSave[]>([]);
  // Playtime-only games (Fortnite, Rust…): tracked for hours, never backed up.
  let playtimeGames = $state<PlaytimeGameInfo[]>([]);
  let playtimeBusy = $state(new Set<string>());
  // slug → Steam app id, sourced from the detection report. Used only to show
  // cover art; absence just falls back to the initial-letter placeholder.
  const appIdBySlug = $derived.by(() => {
    const m = new Map<string, number>();
    for (const g of report?.games ?? []) {
      if (g.steam_app_id != null) m.set(g.slug, g.steam_app_id);
    }
    return m;
  });
  let scanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let search = $state("");
  let confidenceFilter = $state<"all" | Confidence>("all");
  let sourceFilter = $state<"all" | DetectionSource>("all");
  let unlisten: UnlistenFn | null = null;

  /** Currently-open "no save folder" alert. We render a single shared Modal
   *  driven by this state instead of one Modal per card — keeps the DOM lean
   *  for catalogs with hundreds of detected games. */
  let alertGame = $state<DetectedGame | null>(null);
  /** Currently-open untrack confirmation. Same single-modal pattern. */
  let untrackTarget = $state<TrackedSave | null>(null);
  let untracking = $state(false);

  /** Currently-open "delete completely" confirmation. Destructive: wipes the
   *  server-side row, snapshots, and clears the CliState entry so a re-scan
   *  can re-track from scratch. Distinct modal from `untrackTarget` because
   *  the consequences are not the same (untrack keeps backups; delete
   *  wipes them). */
  let deleteTarget = $state<TrackedSave | null>(null);
  let deleting = $state(false);

  /** Currently-open rename-label modal. Pre-fills `renameDraft` from the
   *  target save when set. Single-modal pattern again. */
  let renameTarget = $state<TrackedSave | null>(null);
  let renameDraft = $state("");
  let renaming = $state(false);

  /** Currently-open "dismiss detected game" modal. Two flavours: a
   *  session-only filter (default, no persistence) or a permanent blacklist
   *  entry recorded in CliState. The checkbox `dismissBlacklist` drives the
   *  branch — unchecked drops the slug into `sessionDismissed`, checked
   *  calls `ignoreDetectedGame`. */
  let dismissTarget = $state<DetectedGame | null>(null);
  let dismissBlacklist = $state(false);
  let dismissBusy = $state(false);
  /** Slugs the user dismissed in this session only — wiped on reload. They
   *  reappear on the next scan unless the user also blacklisted them. */
  let sessionDismissed = $state(new Set<string>());
  /** Slugs whose multi-path detection card is expanded to show every save
   *  folder (collapsed cards show only the strongest path). Per-session UI
   *  state, wiped on reload. */
  let expandedPaths = $state(new Set<string>());

  onMount(async () => {
    // Wire the progress event before we trigger anything that emits.
    unlisten = await listen<ScanProgress>(
      "library://scan-progress",
      (event) => {
        progress = event.payload;
      },
    );
    try {
      const [cached, t, pg] = await Promise.all([
        api.cachedDetection(),
        api.listTrackedSaves(),
        api.listPlaytimeGames().catch(() => [] as PlaytimeGameInfo[]),
      ]);
      report = cached;
      tracked = t;
      playtimeGames = pg;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  });

  // Opt a playtime-only game out of (or back into) the recap. Optimistic: we
  // flip `excluded` locally and reconcile from the backend, so the amber card
  // updates instantly without a full re-list.
  async function togglePlaytime(game: PlaytimeGameInfo) {
    if (playtimeBusy.has(game.slug)) return;
    playtimeBusy = new Set(playtimeBusy).add(game.slug);
    const wasExcluded = game.excluded;
    try {
      if (wasExcluded) {
        await api.includePlaytimeGame(game.slug);
      } else {
        await api.excludePlaytimeGame(game.slug);
      }
      playtimeGames = playtimeGames.map((g) =>
        g.slug === game.slug ? { ...g, excluded: !wasExcluded } : g,
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      const next = new Set(playtimeBusy);
      next.delete(game.slug);
      playtimeBusy = next;
    }
  }

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function runScan() {
    scanning = true;
    progress = { done: 0, total: 0 };
    // An explicit (re)scan means "show me everything again": drop the
    // in-memory session dismissals so a card the user hid earlier this
    // session reappears instead of needing an app restart to clear them.
    sessionDismissed = new Set();
    try {
      // Use rescan when we already have a report — same wire payload, but the
      // explicit intent helps backend logs distinguish "user mashed the
      // button" from "page just mounted with no cache".
      report = report ? await api.rescanLibrary() : await api.scanLibrary();
      toastSuccess(
        $_("library.scan_toast", { values: { count: report.games.length } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanning = false;
      progress = null;
    }
  }

  /** Deep-scan tile click. Same flow as `runScan` but hits the exhaustive
   *  backend command that looks at places the periodic scan skips (arbitrary
   *  Wine prefixes, Flatpak/Snap/EmuDeck roots, deeper walks). */
  async function runDeepScan() {
    scanning = true;
    progress = { done: 0, total: 0 };
    sessionDismissed = new Set();
    try {
      report = await api.deepScanLibrary();
      toastSuccess(
        $_("library.scan_toast", { values: { count: report.games.length } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanning = false;
      progress = null;
    }
  }

  /** Shared "actually commit the tracking" path. Used by both the auto-track
   *  flow and the explicit "Choose save folder…" button inside the alert
   *  modal. Kept separate so the alert dialog can close itself before the
   *  network call starts. */
  async function trackWithPath(
    game: DetectedGame,
    chosen: string,
    label?: string,
  ) {
    try {
      const saved = await api.addGameToTracking({
        game_slug: game.slug,
        local_path: chosen,
        // A per-path track of an *additional* save folder needs a distinct
        // label so the server's UNIQUE(user, slug, label) doesn't collide
        // with the primary save (both folders often end in `…/saves`). The
        // first folder keeps the default label; extras carry one.
        label,
        // Pass the catalog metadata so the server can self-heal its games
        // table when its Ludusavi catalog is older than ours. Older servers
        // (pre-v1.3.0) ignore the extra fields; newer servers insert a
        // stub row instead of replying 422 "game not found".
        display_name: game.display_name,
        steam_app_id: game.steam_app_id,
      });
      tracked = [...tracked, saved];
      toastSuccess(
        $_("library.now_tracking", { values: { name: game.display_name } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  /** Track one specific save folder from the per-path list inside a card.
   *  Unlike the folder-picker flow it does NOT set a manual override — the
   *  other detected paths must stay visible so the user can monitor them too.
   *  The first folder tracked for a slug keeps the server default label; any
   *  extra folder carries its own path as the label to stay collision-free. */
  async function trackPath(game: DetectedGame, path: string) {
    // Only count THIS machine's own folders as "already tracked" — an orphan
    // (cloud save from another machine) is not a local branch.
    const localFolders = tracked.filter(
      (t) => t.game_slug === game.slug && !t.orphan && t.local_path,
    );
    // If the slug lives in the cloud only as an orphan and this machine has no
    // folder yet, bind to that existing save instead of forking a new branch.
    if (localFolders.length === 0) {
      const orphan = orphanForSlug(game.slug);
      if (orphan) {
        await adoptOrphan(orphan, path);
        return;
      }
    }
    // The path becomes the label only when this machine genuinely tracks
    // another folder for the slug (real multi-folder). The first folder keeps
    // the default label.
    const slugHasTracked = localFolders.length > 0;
    await trackWithPath(game, path, slugHasTracked ? path : undefined);
  }

  function toggleExpand(slug: string) {
    const next = new Set(expandedPaths);
    if (next.has(slug)) next.delete(slug);
    else next.add(slug);
    expandedPaths = next;
  }

  /** Per-path confidence, falling back to the game's rolled-up grade when an
   *  older cached report didn't carry `path_confidences`. */
  function pathConf(game: DetectedGame, i: number): Confidence {
    return game.path_confidences?.[i] ?? game.confidence;
  }

  function sizeForSlug(slug: string): number {
    // Local footprint on THIS machine (what the user actually pays for here),
    // not the server-side total across every snapshot.
    return localSaves
      .filter((t) => t.game_slug === slug)
      .reduce((acc, s) => acc + (s.local_size_bytes ?? 0), 0);
  }

  /** "Choose save folder…" button inside the alert modal. Pops the picker
   *  *after* the user has read the explanation — no surprise dialogs. */
  async function chooseFromAlert() {
    if (!alertGame) return;
    const game = alertGame;
    alertGame = null; // Close the modal so the OS picker isn't on top.
    const chosen = await pickFolder(game.display_name);
    if (!chosen) return;
    await persistManualPath(game, chosen);
    await trackWithPath(game, chosen);
  }

  /** "Pick a different folder" icon button next to the auto-Track action.
   *  Used when the user wants to override the detected save path — e.g.
   *  Stellaris on Windows expands to the Documents folder, but the user
   *  keeps their saves on another drive, or wants to pick a sibling folder
   *  with custom data. Bypasses the alert modal entirely (that one is for
   *  the no-found-paths case). */
  async function trackWithCustomPath(game: DetectedGame) {
    const chosen = await pickFolder(game.display_name, saveDirFor(game));
    if (!chosen) return;
    await persistManualPath(game, chosen);
    await trackWithPath(game, chosen);
  }

  /** Best-known save folder for a game: the tracked local path if it's already
   *  being monitored, else the detection heuristic's first hit. Used to seed
   *  the OS folder picker so it opens on the saves instead of Documents. */
  function saveDirFor(game: DetectedGame): string | undefined {
    const t = trackedBySlug.get(game.slug);
    if (t && t.local_path) return t.local_path;
    return game.found_paths[0] ?? undefined;
  }

  /** Persist the user's hand-picked folder as a manual override so a
   *  re-scan doesn't revert to the heuristic guess. The detection cache
   *  refresh happens server-side; we just update our local `report.games`
   *  so the source badge flips to "manual" without a roundtrip. */
  async function persistManualPath(game: DetectedGame, chosen: string) {
    try {
      await api.setManualPath(game.slug, chosen);
      if (report) {
        report = {
          ...report,
          games: report.games.map((g) =>
            g.slug === game.slug
              ? {
                  ...g,
                  found_paths: [chosen],
                  source: "manual_override",
                  confidence: "high",
                }
              : g,
          ),
        };
      }
      toastSuccess(
        $_("library.manual_path_set", {
          values: { name: game.display_name },
        }),
      );
    } catch (e) {
      // Persistence failed but the user already picked a folder — surface
      // the error so they know the override won't survive a re-scan, then
      // let `trackWithPath` continue so this session's tracking still
      // works.
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  /** "Volver a sugerencia automática" entry on the per-game menu. Drops the
   *  manual_paths override and refreshes the in-memory report so the row
   *  reverts to whatever the heuristic detected (or, if nothing, the amber
   *  alert). */
  async function revertToAutoDetection(save: TrackedSave) {
    try {
      await api.clearManualPath(save.game_slug);
      // Re-fetch the freshly-rebuilt cache so source/found_paths reflect
      // the heuristic again. Cheaper than re-running the scan client-side
      // and matches what set_manual_path already wrote to disk.
      report = await api.cachedDetection();
      toastSuccess(
        $_("library.manual_path_cleared", {
          values: { name: save.game_slug },
        }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  /** Pop the untrack confirmation modal. The actual delete happens in
   *  `confirmUntrack` so the user has a clear "are you sure" beat. */
  function askUntrack(save: TrackedSave) {
    untrackTarget = save;
  }

  async function confirmUntrack() {
    if (!untrackTarget) return;
    const target = untrackTarget;
    untracking = true;
    try {
      await api.untrackSave(target.save_id);
      tracked = tracked.filter((t) => t.save_id !== target.save_id);
      toastSuccess(
        $_("library.untracked_toast", {
          values: { name: target.game_slug },
        }),
      );
      untrackTarget = null;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      untracking = false;
    }
  }

  /** Pop the "delete completely" confirmation modal. Distinct from the
   *  untrack flow: this wipes server-side state (snapshots + row) so a
   *  subsequent re-scan can re-track from scratch instead of resurrecting
   *  the old row via 409-recovery. */
  function askDelete(save: TrackedSave) {
    deleteTarget = save;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    const target = deleteTarget;
    deleting = true;
    try {
      await api.deleteSaveCompletely(target.save_id);
      // Re-fetch instead of filtering locally so any orphan rows the
      // server still surfaces stay accurate.
      tracked = await api.listTrackedSaves();
      toastSuccess(
        $_("library.deleted_toast", {
          values: { name: target.game_slug },
        }),
      );
      deleteTarget = null;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      deleting = false;
    }
  }

  /** Open the rename-label modal for a tracked save. The draft is seeded with
   *  the current label so the user can tweak rather than retype. */
  function askRename(save: TrackedSave) {
    renameTarget = save;
    renameDraft = save.label;
  }

  async function confirmRename() {
    if (!renameTarget) return;
    const target = renameTarget;
    const trimmed = renameDraft.trim();
    if (!trimmed || trimmed === target.label) {
      renameTarget = null;
      return;
    }
    renaming = true;
    try {
      const updated = await api.renameSaveLabel(target.save_id, trimmed);
      tracked = tracked.map((t) =>
        t.save_id === updated.save_id ? updated : t,
      );
      toastSuccess(
        $_("library.rename_success", { values: { label: updated.label } }),
      );
      renameTarget = null;
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      if (msg === api.LABEL_COLLISION) {
        toastError($_("library.rename_error_conflict"));
      } else {
        toastError(msg);
      }
    } finally {
      renaming = false;
    }
  }

  /** Pop the "dismiss detected game" modal. Source of two outcomes
   *  depending on whether the user ticks the blacklist checkbox: a session
   *  filter (in-memory `sessionDismissed`) or a permanent CliState
   *  blacklist entry. */
  function askDismiss(game: DetectedGame) {
    dismissTarget = game;
    dismissBlacklist = false;
  }

  async function confirmDismiss() {
    if (!dismissTarget) return;
    const target = dismissTarget;
    dismissBusy = true;
    try {
      if (dismissBlacklist) {
        try {
          await api.ignoreDetectedGame(target.slug);
        } catch (e) {
          showError(e);
          return;
        }
        // Re-fetch so the new blacklist entry takes effect immediately —
        // the backend already filters on read, so we just consume the
        // current cached report.
        try {
          report = await api.cachedDetection();
        } catch (e) {
          // Non-fatal: filter locally if the cache fetch hiccups.
          if (report) {
            report = {
              ...report,
              games: report.games.filter((g) => g.slug !== target.slug),
            };
          }
        }
        toastSuccess($_("library.ignored_toast"));
      } else {
        sessionDismissed.add(target.slug);
        // Force a reactive update — Svelte 5 runes don't track Set mutations.
        sessionDismissed = new Set(sessionDismissed);
      }
      dismissTarget = null;
      dismissBlacklist = false;
    } finally {
      dismissBusy = false;
    }
  }

  /** Open the OS folder picker. Returns the selected absolute path or null
   *  on cancel. We pass the game's display name as the dialog title so the
   *  user knows which game they're picking the save folder for. */
  async function pickFolder(
    displayName: string,
    defaultPath?: string,
  ): Promise<string | null> {
    try {
      const result = await openDialog({
        directory: true,
        multiple: false,
        // Open the picker *at* the game's detected/tracked save folder when we
        // know it, so the user lands on their saves instead of Documents.
        defaultPath: defaultPath || undefined,
        title: $_("library.pick_folder_title", {
          values: { name: displayName },
        }),
      });
      // Tauri's dialog plugin returns string | null; normalise just in case.
      if (typeof result === "string" && result.length > 0) return result;
      return null;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      return null;
    }
  }

  // ---- derived views -------------------------------------------------

  // Slugs THIS machine actually monitors (has a local folder for). Orphan rows
  // are cloud saves from another machine — they must NOT count as tracked here,
  // or a detected game shows a lying "Monitorizado" badge and the "+" never
  // adopts it (BUG 3/4).
  const trackedSlugs = $derived(
    new Set(
      tracked.filter((t) => !t.orphan && t.local_path).map((t) => t.game_slug),
    ),
  );

  // Saves with a local folder on this machine vs cloud-only saves from other
  // machines. Drives the split between "Juegos monitorizados" (this machine,
  // local size) and "En la nube — otras máquinas" (adoptable) — BUG 4.
  const localSaves = $derived(tracked.filter((t) => !t.orphan));
  const cloudOrphans = $derived(tracked.filter((t) => t.orphan));

  /** The cloud orphan for a slug, if any (prefers the "main" label). Lets the
   *  "+" / track flow adopt an existing cloud save instead of forking a new
   *  branch (BUG 3). */
  function orphanForSlug(slug: string): TrackedSave | undefined {
    const matches = cloudOrphans.filter((t) => t.game_slug === slug);
    return matches.find((t) => t.label === "main") ?? matches[0];
  }

  /** Adopt a cloud orphan into a local folder: bind this machine to the
   *  existing save_id and refresh the list. */
  async function adoptOrphan(orphan: TrackedSave, path: string) {
    try {
      await api.adoptSave({
        save_id: orphan.save_id,
        game_slug: orphan.game_slug,
        label: orphan.label,
        local_path: path,
      });
      tracked = await api.listTrackedSaves();
      toastSuccess(
        $_("library.now_linked", { values: { name: orphan.game_slug } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  /** "Vincular a esta máquina…" on a cloud-orphan card: pick a folder, then
   *  adopt the existing save into it. */
  async function linkOrphan(orphan: TrackedSave) {
    const chosen = await pickFolder(orphan.game_slug);
    if (!chosen) return;
    await adoptOrphan(orphan, chosen);
  }

  const filtered = $derived.by(() => {
    if (!report) return [];
    const q = search.trim().toLowerCase();
    return report.games.filter((g) => {
      // Session dismissals: same UX as permanent blacklist, just without
      // the persistence beat. Reset on app reload.
      if (sessionDismissed.has(g.slug)) return false;
      if (q && !g.display_name.toLowerCase().includes(q)) return false;
      if (confidenceFilter !== "all" && g.confidence !== confidenceFilter)
        return false;
      if (sourceFilter !== "all" && g.source !== sourceFilter) return false;
      return true;
    });
  });

  function confidenceLabel(c: Confidence): string {
    return c === "high" ? $_("library.high") : c === "medium" ? $_("library.medium") : $_("library.low");
  }

  function sourceLabel(s: DetectionSource): string {
    if (s === "both") return $_("library.both_sources");
    if (s === "steam_library") return $_("library.steam_label");
    if (s === "manual_override") return $_("library.manual_label");
    return $_("library.filesystem_label");
  }

  function sourceBadgeClass(s: DetectionSource): string {
    if (s === "both")
      return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    if (s === "steam_library")
      return "bg-sky-500/10 text-sky-300 ring-sky-500/30";
    if (s === "manual_override")
      return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    return "bg-zinc-500/10 text-zinc-300 ring-zinc-500/30";
  }

  function confidenceBadgeClass(c: Confidence): string {
    if (c === "high")
      return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    if (c === "medium")
      return "bg-amber-500/10 text-amber-300 ring-amber-500/30";
    return "bg-zinc-500/10 text-zinc-300 ring-zinc-500/30";
  }

  const pct = $derived.by(() => {
    if (!progress || progress.total === 0) return 0;
    return Math.min(100, Math.round((progress.done / progress.total) * 100));
  });

  // Same byte-formatter as QuotaBar; cheap enough to duplicate rather than
  // pull a util module for two callers.
  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  // Sum the LOCAL disk usage of saves monitored on this machine so the section
  // header can show "3 games · 142 MB" of real on-disk footprint here.
  const trackedTotalBytes = $derived(
    localSaves.reduce((acc, s) => acc + (s.local_size_bytes ?? 0), 0),
  );

  // Sum the cloud footprint of saves that live only on other machines, for the
  // "En la nube — otras máquinas" header.
  const cloudTotalBytes = $derived(
    cloudOrphans.reduce((acc, s) => acc + (s.total_size_bytes ?? 0), 0),
  );

  // Map slug → locally-tracked entry so the detection cards can show a tiny
  // size pill on already-monitored games without an extra render pass.
  const trackedBySlug = $derived.by(() => {
    const m = new Map<string, TrackedSave>();
    for (const s of localSaves) m.set(s.game_slug, s);
    return m;
  });

  // Map local_path → tracked entry so each per-path row inside a grouped
  // detection card can tell whether *that specific folder* is monitored.
  const trackedByPath = $derived.by(() => {
    const m = new Map<string, TrackedSave>();
    for (const s of tracked) if (s.local_path) m.set(s.local_path, s);
    return m;
  });

  // Slugs whose detection row has source=manual_override. Drives the
  // "Volver a sugerencia automática" button on the tracked-games strip — we
  // only show it when the user actually has an override to clear.
  const slugsWithManualOverride = $derived.by(() => {
    const s = new Set<string>();
    if (!report) return s;
    for (const g of report.games) {
      if (g.source === "manual_override") s.add(g.slug);
    }
    return s;
  });

  function hasManualOverride(slug: string): boolean {
    return slugsWithManualOverride.has(slug);
  }
</script>

<div class="mx-auto max-w-6xl px-8 py-8">
  <header class="mb-6 flex items-start justify-between gap-4">
    <div>
      <h1 class="font-display text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50">{$_("library.title")}</h1>
      <p class="mt-2 text-sm text-zinc-400">
        {#if report}
          {$_("library.subtitle_scanned", { values: { catalog: report.catalog_size.toLocaleString(), found: report.games.length } })}
          {#if report.steam_apps_found > 0}
            {$_("library.subtitle_steam_addendum", { values: { steam: report.steam_apps_found } })}
          {/if}
        {:else}
          {$_("library.subtitle_initial")}
        {/if}
      </p>
    </div>
    <Button onclick={runScan} loading={scanning}>
      <RefreshCw size={16} />
      {scanning ? $_("library.scanning") : report ? $_("library.rescan") : $_("library.scan_now")}
    </Button>
  </header>

  {#if localSaves.length > 0}
    <!-- Per-game disk-usage strip for THIS machine. Shows what Hoard monitors
         here and how much space each save occupies *on this device* (local
         folder size, not the server total). Saves that live only on other
         machines go in the "En la nube" section below. -->
    <section class="mb-6">
      <div
        class="mb-2 flex items-center justify-between gap-3 text-xs uppercase tracking-wide text-zinc-500"
      >
        <span>{$_("library.tracked_games")}</span>
        <span class="tabular-nums normal-case tracking-normal text-zinc-400">
          {$_("library.tracked_summary", { values: { count: localSaves.length, size: fmtBytes(trackedTotalBytes) } })}
        </span>
      </div>
      <div
        class="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4"
      >
        {#each localSaves as save (save.save_id)}
          <div
            class="group flex items-center justify-between gap-2 rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-emerald-500/25 hover:bg-zinc-900/50"
          >
            <Cover
              appId={appIdBySlug.get(save.game_slug) ?? null}
              slug={save.game_slug}
              name={save.game_slug}
              class="h-9 w-9 rounded-md"
              initialClass="text-sm"
            />
            <div class="min-w-0 flex-1">
              <p
                class="truncate text-sm font-medium text-zinc-100"
                title={save.game_slug}
              >
                {save.game_slug}
              </p>
              <p class="truncate text-[11px] text-zinc-500">
                {save.label}
              </p>
            </div>
            <span
              class="shrink-0 rounded bg-zinc-800 px-2 py-0.5 text-xs font-medium tabular-nums text-zinc-300"
              title={$_("library.local_size_title")}
            >
              {save.local_size_bytes && save.local_size_bytes > 0
                ? fmtBytes(save.local_size_bytes)
                : "—"}
            </span>
            <button
              type="button"
              onclick={() => askRename(save)}
              aria-label={$_("library.rename_button")}
              title={$_("library.rename_title")}
              class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
            >
              <Pencil size={14} />
            </button>
            {#if hasManualOverride(save.game_slug)}
              <button
                type="button"
                onclick={() => revertToAutoDetection(save)}
                aria-label={$_("library.use_auto_detection")}
                title={$_("library.use_auto_detection")}
                class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
              >
                <RotateCcw size={14} />
              </button>
            {/if}
            <button
              type="button"
              onclick={() => askUntrack(save)}
              aria-label={$_("library.untrack_button")}
              title={$_("library.untrack_title")}
              class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300"
            >
              <Trash2 size={14} />
            </button>
            <button
              type="button"
              onclick={() => askDelete(save)}
              aria-label={$_("library.delete_button")}
              title={$_("library.delete_title")}
              class="shrink-0 rounded p-1 text-rose-500 transition-colors hover:bg-rose-500/20 hover:text-rose-300"
            >
              <Trash size={14} />
            </button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if cloudOrphans.length > 0}
    <!-- Saves that live in the cloud but on OTHER machines (no local folder
         here). The primary action is to adopt them — link a folder on this
         machine to the existing cloud save so it starts syncing here (BUG 3/4). -->
    <section class="mb-6">
      <div
        class="mb-2 flex items-center justify-between gap-3 text-xs uppercase tracking-wide text-zinc-500"
      >
        <span>{$_("library.cloud_other_machines")}</span>
        <span class="tabular-nums normal-case tracking-normal text-zinc-400">
          {$_("library.tracked_summary", { values: { count: cloudOrphans.length, size: fmtBytes(cloudTotalBytes) } })}
        </span>
      </div>
      <div
        class="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4"
      >
        {#each cloudOrphans as save (save.save_id)}
          <div
            class="group flex items-center justify-between gap-2 rounded-xl border border-white/[0.06] bg-zinc-950/40 p-3 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-emerald-500/25 hover:bg-zinc-900/50"
          >
            <Cover
              appId={appIdBySlug.get(save.game_slug) ?? null}
              slug={save.game_slug}
              name={save.game_slug}
              class="h-9 w-9 rounded-md"
              initialClass="text-sm"
            />
            <div class="min-w-0 flex-1">
              <p
                class="truncate text-sm font-medium text-zinc-100"
                title={save.game_slug}
              >
                {save.game_slug}
              </p>
              <p class="truncate text-[11px] text-zinc-500">
                {save.label} · {$_("library.cloud_only_badge")}
              </p>
            </div>
            <span
              class="shrink-0 rounded bg-zinc-800 px-2 py-0.5 text-xs font-medium tabular-nums text-zinc-300"
              title={$_("library.tracked_size_title")}
            >
              {save.total_size_bytes > 0
                ? fmtBytes(save.total_size_bytes)
                : "—"}
            </span>
            <button
              type="button"
              onclick={() => linkOrphan(save)}
              aria-label={$_("library.link_to_machine")}
              title={$_("library.link_to_machine")}
              class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-emerald-500/10 hover:text-emerald-300"
            >
              <Link size={14} />
            </button>
            <button
              type="button"
              onclick={() => askDelete(save)}
              aria-label={$_("library.delete_button")}
              title={$_("library.delete_title")}
              class="shrink-0 rounded p-1 text-rose-500 transition-colors hover:bg-rose-500/20 hover:text-rose-300"
            >
              <Trash size={14} />
            </button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if playtimeGames.length > 0}
    <!-- Playtime-only games: always-online titles (Fortnite, Rust…) with no
         save worth syncing. We track their hours for the recap but back up
         NOTHING — amber signals "no copia". Auto-enrolled; the user can drop
         any of them (and re-add later). -->
    <section class="mb-6">
      <div
        class="mb-2 flex items-center gap-2 text-xs uppercase tracking-wide text-amber-500/80"
      >
        <Clock size={13} />
        <span>{$_("library.playtime_section")}</span>
      </div>
      <p class="mb-2 text-[11px] text-zinc-500">
        {$_("library.playtime_hint")}
      </p>
      <div
        class="grid grid-cols-1 gap-2 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-4"
      >
        {#each playtimeGames as game (game.slug)}
          <div
            class="group flex items-center justify-between gap-2 rounded-xl border p-3 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 {game.excluded
              ? 'border-white/[0.06] bg-zinc-950/40 opacity-50'
              : 'border-amber-500/40 bg-amber-500/10 hover:border-amber-500/60'}"
          >
            <Cover
              appId={appIdBySlug.get(game.slug) ?? null}
              slug={game.slug}
              name={game.display_name}
              class="h-9 w-9 rounded-md"
              initialClass="text-sm"
            />
            <div class="min-w-0 flex-1">
              <p
                class="truncate text-sm font-medium text-zinc-100"
                title={game.display_name}
              >
                {game.display_name}
              </p>
              <p class="truncate text-[11px] text-amber-500/70">
                {game.excluded
                  ? $_("library.playtime_excluded_badge")
                  : $_("library.playtime_badge")}
              </p>
            </div>
            {#if game.excluded}
              <button
                type="button"
                disabled={playtimeBusy.has(game.slug)}
                onclick={() => togglePlaytime(game)}
                aria-label={$_("library.playtime_include")}
                title={$_("library.playtime_include")}
                class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-amber-500/15 hover:text-amber-300 disabled:opacity-40"
              >
                <RotateCcw size={14} />
              </button>
            {:else}
              <button
                type="button"
                disabled={playtimeBusy.has(game.slug)}
                onclick={() => togglePlaytime(game)}
                aria-label={$_("library.playtime_exclude")}
                title={$_("library.playtime_exclude")}
                class="shrink-0 rounded p-1 text-amber-500/70 transition-colors hover:bg-amber-500/15 hover:text-amber-200 disabled:opacity-40"
              >
                <X size={14} />
              </button>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if scanning && progress}
    <div class="mb-6 rounded-xl border border-white/[0.06] bg-zinc-950/40 p-4 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)]">
      <div class="mb-2 flex items-center justify-between text-xs text-zinc-400">
        <span>{$_("library.scanning_catalog")}</span>
        <span class="tabular-nums">
          {progress.done.toLocaleString()} / {progress.total.toLocaleString()}
        </span>
      </div>
      <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.06]">
        <div
          class="h-full rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)] transition-all"
          style="width: {pct}%"
        ></div>
      </div>
    </div>
  {/if}

  {#if report}
    <div class="mb-5 flex flex-wrap items-center gap-3">
      <div class="flex-1 min-w-[14rem]">
        <Input bind:value={search} placeholder={$_("library.search")} icon={SearchIcon} />
      </div>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        <Filter size={14} />
        {$_("library.confidence")}
        <select
          bind:value={confidenceFilter}
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm text-zinc-100"
        >
          <option value="all">{$_("library.any")}</option>
          <option value="high">{$_("library.high")}</option>
          <option value="medium">{$_("library.medium")}</option>
          <option value="low">{$_("library.low")}</option>
        </select>
      </label>
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        {$_("library.source")}
        <select
          bind:value={sourceFilter}
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1.5 text-sm text-zinc-100"
        >
          <option value="all">{$_("library.any")}</option>
          <option value="both">{$_("library.both_sources")}</option>
          <option value="steam_library">{$_("library.steam_only")}</option>
          <option value="filesystem_heuristic">{$_("library.filesystem_only")}</option>
        </select>
      </label>
    </div>

    {#snippet deepScanTile()}
      <button
        type="button"
        onclick={runDeepScan}
        disabled={scanning}
        title={$_("library.deep_scan_hint")}
        class="group flex flex-col rounded-xl border border-red-500/30 bg-red-950/20 p-4 text-left shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-red-500/55 hover:bg-red-950/30 disabled:cursor-not-allowed disabled:opacity-60"
      >
        <div class="mb-2 flex items-start gap-2.5">
          <div
            class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-red-500/15 text-red-400"
          >
            <AlertTriangle class="h-5 w-5" />
          </div>
          <div class="min-w-0">
            <h3 class="truncate text-sm font-medium text-red-300">
              {$_("library.deep_scan_title")}
            </h3>
            <p class="truncate text-xs text-red-400/70">
              {$_("library.deep_scan_subtitle")}
            </p>
          </div>
        </div>
        <p class="mt-auto pt-2 text-xs leading-snug text-red-300/60">
          {$_("library.deep_scan_hint")}
        </p>
      </button>
    {/snippet}

    {#if filtered.length === 0}
      <Card>
        <div class="py-12 text-center text-sm text-zinc-400">
          {report.games.length === 0
            ? $_("library.no_results_empty")
            : $_("library.no_results_filtered")}
        </div>
      </Card>
      <div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
        {@render deepScanTile()}
      </div>
    {:else}
      <div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
        {#each filtered as game (game.slug)}
          {@const isTracked = trackedSlugs.has(game.slug)}
          <div
            class="group flex flex-col rounded-xl border border-white/[0.06] bg-zinc-950/40 p-4 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-emerald-500/25 hover:bg-zinc-900/50"
          >
            <div class="mb-2 flex items-start justify-between gap-2">
              <div class="flex min-w-0 items-center gap-2.5">
                <Cover
                  appId={game.steam_app_id}
                  slug={game.slug}
                  name={game.display_name}
                  class="h-10 w-10 rounded-md"
                  initialClass="text-base"
                />
                <div class="min-w-0">
                  <h3
                    class="truncate text-sm font-medium text-zinc-100"
                    title={game.display_name}
                  >
                    {game.display_name}
                  </h3>
                  <p class="truncate text-xs text-zinc-500" title={game.slug}>
                    {game.slug}
                  </p>
                </div>
              </div>
              <div class="flex shrink-0 flex-col items-end gap-1">
                <span
                  class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ring-1 ring-inset {confidenceBadgeClass(
                    game.confidence,
                  )}"
                >
                  {confidenceLabel(game.confidence)}
                </span>
                <span
                  class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ring-1 ring-inset {sourceBadgeClass(
                    game.source,
                  )}"
                >
                  {#if game.source === "steam_library"}
                    <Gamepad2 size={10} />
                  {:else}
                    <HardDrive size={10} />
                  {/if}
                  {sourceLabel(game.source)}
                </span>
              </div>
            </div>

            {#if game.found_paths.length}
              <!-- Grouped per-path list. A game can write to several save
                   folders with very different confidence (e.g. the real
                   rotating-save dir vs. a Steam-cloud marker stub). We show
                   ONE card per game and let each folder carry its own grade +
                   monitor toggle, so a HIGH path doesn't drag a LOW sibling
                   into tracking. Collapsed to the strongest path until the
                   user expands. -->
              {@const expanded = expandedPaths.has(game.slug)}
              {@const visible = expanded ? game.found_paths.length : 1}
              <div class="mt-2 flex flex-col gap-1.5">
                {#each game.found_paths.slice(0, visible) as path, i (path)}
                  {@const ts = trackedByPath.get(path)}
                  <div class="flex items-center gap-2">
                    <span
                      class="inline-flex shrink-0 items-center rounded-full px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide ring-1 ring-inset {confidenceBadgeClass(
                        pathConf(game, i),
                      )}"
                    >
                      {confidenceLabel(pathConf(game, i))}
                    </span>
                    <span
                      class="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-500"
                      title={path}
                    >
                      {path}
                    </span>
                    {#if ts}
                      <span
                        class="shrink-0 text-emerald-300"
                        title={$_("library.tracked_badge")}
                      >
                        <Check size={13} />
                      </span>
                      <button
                        type="button"
                        onclick={() => askUntrack(ts)}
                        aria-label={$_("library.untrack_button")}
                        title={$_("library.untrack_title")}
                        class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300"
                      >
                        <Trash2 size={13} />
                      </button>
                    {:else}
                      <button
                        type="button"
                        onclick={() => trackPath(game, path)}
                        aria-label={$_("library.track_this_path")}
                        title={$_("library.track_this_path")}
                        class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-emerald-500/10 hover:text-emerald-300"
                      >
                        <Plus size={13} />
                      </button>
                    {/if}
                  </div>
                {/each}
                {#if game.found_paths.length > 1}
                  <button
                    type="button"
                    onclick={() => toggleExpand(game.slug)}
                    class="self-start text-[11px] text-zinc-500 transition-colors hover:text-zinc-300"
                  >
                    {expanded
                      ? $_("library.show_less")
                      : $_("library.found_more", { values: { count: game.found_paths.length - 1 } })}
                  </button>
                {/if}
              </div>

              <div class="mt-4 flex items-center gap-2">
                {#if isTracked}
                  {@const sz = sizeForSlug(game.slug)}
                  <span
                    class="inline-flex flex-1 items-center gap-1.5 rounded-md bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-300 ring-1 ring-inset ring-emerald-500/30"
                  >
                    <Check size={12} />
                    {$_("library.tracked_badge")}
                    {#if sz > 0}
                      <span class="text-emerald-400/70 tabular-nums">
                        · {fmtBytes(sz)}
                      </span>
                    {/if}
                  </span>
                {/if}
                <!-- Folder picker: hand-pick a save folder we didn't detect.
                     Sets a manual override so a re-scan keeps it. -->
                <button
                  type="button"
                  onclick={() => trackWithCustomPath(game)}
                  aria-label={$_("library.track_pick_folder_aria")}
                  title={$_("library.track_pick_folder_aria")}
                  class="shrink-0 rounded p-1.5 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
                >
                  <FolderOpen size={14} />
                </button>
                {#if !isTracked}
                  <button
                    type="button"
                    onclick={() => askDismiss(game)}
                    aria-label={$_("library.ignore_confirm")}
                    title={$_("library.ignore_confirm")}
                    class="ml-auto shrink-0 rounded p-1.5 text-rose-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300"
                  >
                    <Trash size={14} />
                  </button>
                {/if}
              </div>
            {:else}
              <p class="mt-1 text-[11px] italic text-zinc-600">
                {$_("library.no_save_folder_yet")}
              </p>

              <div class="mt-4 flex items-center gap-2">
                {#if isTracked}
                  {@const t = trackedBySlug.get(game.slug)}
                  <span
                    class="inline-flex flex-1 items-center gap-1.5 rounded-md bg-emerald-500/10 px-2.5 py-1 text-xs font-medium text-emerald-300 ring-1 ring-inset ring-emerald-500/30"
                  >
                    <Check size={12} />
                    {$_("library.tracked_badge")}
                    {#if t && t.total_size_bytes > 0}
                      <span class="text-emerald-400/70 tabular-nums">
                        · {fmtBytes(t.total_size_bytes)}
                      </span>
                    {/if}
                  </span>
                  {#if t}
                    <button
                      type="button"
                      onclick={() => askUntrack(t)}
                      aria-label={$_("library.untrack_button")}
                      title={$_("library.untrack_title")}
                      class="shrink-0 rounded p-1.5 text-zinc-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300"
                    >
                      <Trash2 size={14} />
                    </button>
                  {/if}
                {:else}
                  <!-- Steam-only match with no save folder yet. Amber alert
                       instead of auto-popping the OS picker or doing nothing:
                       a modal explains why and offers an explicit folder pick. -->
                  <button
                    type="button"
                    onclick={() => (alertGame = game)}
                    aria-label={$_("library.no_save_alert_aria")}
                    title={$_("library.no_save_alert_aria")}
                    class="inline-flex items-center gap-1.5 rounded-md bg-amber-500/10 px-2.5 py-1.5 text-xs font-medium text-amber-300 ring-1 ring-inset ring-amber-500/30 transition-colors hover:bg-amber-500/20"
                  >
                    <AlertTriangle size={14} />
                    {$_("library.no_save_folder_yet")}
                  </button>
                  <button
                    type="button"
                    onclick={() => askDismiss(game)}
                    aria-label={$_("library.ignore_confirm")}
                    title={$_("library.ignore_confirm")}
                    class="ml-auto shrink-0 rounded p-1.5 text-rose-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300"
                  >
                    <Trash size={14} />
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
        {@render deepScanTile()}
      </div>
    {/if}
  {:else if !scanning}
    <Card>
      <div class="py-16 text-center">
        <div
          class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-amber-500/10 text-amber-400 ring-1 ring-amber-500/30"
        >
          <RefreshCw size={20} />
        </div>
        <h2 class="text-base font-medium text-zinc-100">
          {$_("library.no_scan_title")}
        </h2>
        <p class="mx-auto mt-2 max-w-md text-sm text-zinc-400">
          {$_("library.no_scan_body")}
        </p>
        <div class="mt-6">
          <Button onclick={runScan}>
            <RefreshCw size={16} />
            {$_("library.scan_now")}
          </Button>
        </div>
      </div>
    </Card>
  {/if}

  <!-- "No save folder yet" alert. Triggered by clicking the amber warning
       button on a Steam-only detection card. Explains why we don't have a
       path and offers an explicit "pick folder" action — no surprise OS
       dialog. -->
  <Modal
    open={alertGame !== null}
    title={$_("library.no_save_alert_title", {
      values: { name: alertGame?.display_name ?? "" },
    })}
    onClose={() => (alertGame = null)}
  >
    <p class="text-sm text-zinc-300">
      {$_("library.no_save_alert_body", {
        values: { name: alertGame?.display_name ?? "" },
      })}
    </p>
    {#if alertGame?.install_dir}
      <p class="mt-3 break-all rounded-md bg-zinc-950/60 px-3 py-2 font-mono text-[11px] text-zinc-400 ring-1 ring-inset ring-zinc-800">
        {$_("library.no_save_alert_install_hint", {
          values: { path: alertGame.install_dir },
        })}
      </p>
    {/if}
    {#snippet footer()}
      <Button variant="ghost" onclick={() => (alertGame = null)}>
        {$_("common.cancel")}
      </Button>
      <Button onclick={chooseFromAlert}>
        <FolderOpen size={14} />
        {$_("library.no_save_alert_choose")}
      </Button>
    {/snippet}
  </Modal>

  <!-- Destructive: stop tracking a game. Snapshots already on the server
       are NOT deleted by this — only the local watch/auto-backup link.
       We surface that nuance in the body copy so the user isn't scared
       off thinking they're wiping their backups. -->
  <Modal
    open={untrackTarget !== null}
    title={$_("library.untrack_confirm_title", {
      values: { name: untrackTarget?.game_slug ?? "" },
    })}
    onClose={() => {
      if (!untracking) untrackTarget = null;
    }}
    dismissible={!untracking}
  >
    <p class="text-sm text-zinc-300">
      {$_("library.untrack_confirm_body")}
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => (untrackTarget = null)}
        disabled={untracking}
      >
        {$_("common.cancel")}
      </Button>
      <button
        type="button"
        onclick={confirmUntrack}
        disabled={untracking}
        class="inline-flex items-center justify-center gap-2 rounded-md bg-rose-500 px-4 py-2 text-sm font-medium text-zinc-950 transition-colors hover:bg-rose-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-400 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
      >
        <Trash2 size={14} />
        {$_("library.untrack_confirm_action")}
      </button>
    {/snippet}
  </Modal>

  <!-- Destructive: wipe the save server-side. Distinct from untrack — this
       deletes every snapshot for this game on the server, clears the local
       CliState entry, and lets a subsequent scan re-track from scratch
       without 409-recovery resurrecting the bad row. -->
  <Modal
    open={deleteTarget !== null}
    title={$_("library.delete_confirm_title", {
      values: { name: deleteTarget?.game_slug ?? "" },
    })}
    onClose={() => {
      if (!deleting) deleteTarget = null;
    }}
    dismissible={!deleting}
  >
    <p class="text-sm text-zinc-300">
      {$_("library.delete_confirm_body", {
        values: { name: deleteTarget?.game_slug ?? "" },
      })}
    </p>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => (deleteTarget = null)}
        disabled={deleting}
      >
        {$_("common.cancel")}
      </Button>
      <button
        type="button"
        onclick={confirmDelete}
        disabled={deleting}
        class="inline-flex items-center justify-center gap-2 rounded-md bg-rose-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-rose-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
      >
        <Trash size={14} />
        {$_("library.delete_confirm_action")}
      </button>
    {/snippet}
  </Modal>

  <!-- Dismiss a detected game from the Library. Without the checkbox this
       is just a session filter; with the checkbox it persists in CliState
       so the slug stops appearing in future scans until reactivated from
       Settings → "Juegos ignorados". -->
  <Modal
    open={dismissTarget !== null}
    title={$_("library.ignore_title", {
      values: { name: dismissTarget?.display_name ?? "" },
    })}
    onClose={() => {
      if (!dismissBusy) {
        dismissTarget = null;
        dismissBlacklist = false;
      }
    }}
    dismissible={!dismissBusy}
  >
    <p class="text-sm text-zinc-300">
      {$_("library.ignore_body", {
        values: { name: dismissTarget?.display_name ?? "" },
      })}
    </p>
    <label class="mt-4 flex items-start gap-2 text-sm text-zinc-300">
      <input
        type="checkbox"
        bind:checked={dismissBlacklist}
        disabled={dismissBusy}
        class="mt-0.5 h-4 w-4 shrink-0 rounded border-zinc-700 bg-zinc-900 accent-emerald-500"
      />
      <span>{$_("library.ignore_blacklist_check")}</span>
    </label>
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => {
          dismissTarget = null;
          dismissBlacklist = false;
        }}
        disabled={dismissBusy}
      >
        {$_("common.cancel")}
      </Button>
      <button
        type="button"
        onclick={confirmDismiss}
        disabled={dismissBusy}
        class="inline-flex items-center justify-center gap-2 rounded-md bg-rose-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-rose-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
      >
        <Trash size={14} />
        {$_("library.ignore_confirm")}
      </button>
    {/snippet}
  </Modal>

  <!-- Rename label modal. The on-disk snapshot directory is renamed
       atomically server-side, so a 409 means another save under this
       (user, game) already owns the requested label. We show a localized
       error and keep the modal open so the user can pick another. -->
  <Modal
    open={renameTarget !== null}
    title={$_("library.rename_modal_title", {
      values: { name: renameTarget?.game_slug ?? "" },
    })}
    onClose={() => {
      if (!renaming) renameTarget = null;
    }}
    dismissible={!renaming}
  >
    <p class="mb-3 text-sm text-zinc-300">
      {$_("library.rename_modal_body")}
    </p>
    <Input
      bind:value={renameDraft}
      placeholder={$_("library.rename_input_placeholder")}
      disabled={renaming}
    />
    {#snippet footer()}
      <Button
        variant="ghost"
        onclick={() => (renameTarget = null)}
        disabled={renaming}
      >
        {$_("common.cancel")}
      </Button>
      <Button onclick={confirmRename} loading={renaming}>
        {$_("library.rename_confirm")}
      </Button>
    {/snippet}
  </Modal>
</div>
