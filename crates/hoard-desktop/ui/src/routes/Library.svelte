<script lang="ts">
  /**
   * Library page, auto-detection results + tracked saves.
   *
   * On mount we hydrate from the in-memory detection cache (if a previous
   * scan happened this session) and also fetch the user's tracked saves so
   * the "tracked" badge can render immediately.
   *
   * "Scan" kicks off `scan_library`; while it runs we listen for
   * `library://scan-progress` events to drive the progress bar.
   */
  import { onDestroy, onMount } from "svelte";
  import { tilt } from "../lib/actions/tilt";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    Search as SearchIcon,
    RefreshCw,
    Plus,
    Filter,
    HardDrive,
    Gamepad2,
    FolderSearch,
    AlertTriangle,
    Trash2,
    Trash,
    FolderOpen,
    Pencil,
    RotateCcw,
    RotateCw,
    Link,
    Clock,
    Snowflake,
    X,
    PauseCircle,
    Cloud,
    History,
  } from "@lucide/svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Cover from "../lib/components/Cover.svelte";
  import Input from "../lib/components/Input.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import LinkOrphanModal from "../lib/components/LinkOrphanModal.svelte";
  import ManualTrackModal from "../lib/components/ManualTrackModal.svelte";
  import ScanFolderModal from "../lib/components/ScanFolderModal.svelte";
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
  import {
    archivedSaves,
    refreshArchivedSaves,
    reactivateAndRefresh,
  } from "../lib/stores/cloud";
  import { cardWidth } from "../lib/stores/cardSizes.svelte";
  import {
    backupBlocked,
    filesUnreadable,
    wrongPathSuspected,
  } from "../lib/stores/agent";
  import CardResizeHandle from "../lib/components/CardResizeHandle.svelte";

  let report = $state<DetectionReport | null>(null);
  let tracked = $state<TrackedSave[]>([]);
  // Manual-track dialog (game or emulator by hand) and the folder-scan dialog,
  // both independent of the auto-detection flow.
  let emulatorModalOpen = $state(false);
  let scanFolderOpen = $state(false);
  /** Cloud orphan whose "Vincular a este equipo…" dialog is open. */
  let linkingOrphan = $state<TrackedSave | null>(null);

  // Manually-added emulator saves carry a synthesized slug: `emu-<id>` for a
  // catalog pick or `emu-<slugified name>` for a custom one. The Library shows
  // them alongside detected games, so map the slug back to a friendly name and
  // flag them as emulators instead of printing the raw `emu-melonds`.
  const EMU_NAMES: Record<string, string> = {
    pcsx2: "PCSX2",
    rpcs3: "RPCS3",
    duckstation: "DuckStation",
    ppsspp: "PPSSPP",
    dolphin: "Dolphin",
    cemu: "Cemu",
    ryujinx: "Ryujinx",
    citra: "Citra",
    retroarch: "RetroArch",
    mgba: "mGBA",
    melonds: "melonDS",
    project64: "Project64",
  };
  function isEmu(slug: string): boolean {
    return slug.startsWith("emu-");
  }
  /** Friendly display name for a tracked save's slug. Emulators resolve to
   *  their catalog name (or a title-cased custom name); everything else keeps
   *  the slug, exactly as the list rendered before. */
  function displayName(slug: string): string {
    if (!isEmu(slug)) return slug;
    const id = slug.slice(4);
    if (EMU_NAMES[id]) return EMU_NAMES[id];
    return id
      .split("-")
      .filter(Boolean)
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ");
  }
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

  /** Game whose folder is being chosen by scanning ("track with another
   *  folder", "no save folder yet"). Drives ScanFolderModal in target mode; a
   *  single shared modal, not one per card, keeps the DOM lean for catalogs
   *  with hundreds of detected games. */
  let folderTargetGame = $state<DetectedGame | null>(null);
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
   *  branch, unchecked drops the slug into `sessionDismissed`, checked
   *  calls `ignoreDetectedGame`. */
  let dismissTarget = $state<DetectedGame | null>(null);
  let dismissBlacklist = $state(false);
  let dismissBusy = $state(false);
  /** How many saves this machine tracks under the slug being dismissed.
   *  Blacklisting untracks them, so the modal warns first. Orphans don't
   *  count: they're another machine's rows and nothing here watches them. */
  const dismissTrackedCount = $derived(
    dismissTarget
      ? tracked.filter((t) => t.game_slug === dismissTarget!.slug && !t.orphan)
          .length
      : 0,
  );
  /** Slugs the user dismissed in this session only, wiped on reload. They
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
    // Which tracked saves are frozen in the black box (drives the badge +
    // Reactivar button). Cloud-only; a no-op when self-hosted.
    void refreshArchivedSaves();
  });

  // Reactivate an archived save: recovers quota + resumes sync (within the
  // 7-day window). Per-save busy set so only the pressed row spins.
  let reactivating = $state<Set<string>>(new Set());

  /** Localised purge date for an archived save, or null when it isn't
   *  archived. Reads `$archivedSaves` so it re-derives when the map changes. */
  function purgeDate(saveId: string): string | null {
    const iso = $archivedSaves[saveId];
    return iso ? new Date(iso).toLocaleDateString() : null;
  }

  async function reactivate(save: TrackedSave) {
    if (reactivating.has(save.save_id)) return;
    reactivating = new Set(reactivating).add(save.save_id);
    try {
      await reactivateAndRefresh(save.save_id);
      toastSuccess(
        $_("archived.reactivated_toast", {
          values: { name: displayName(save.game_slug) },
        }),
      );
      tracked = await api.listTrackedSaves();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      const next = new Set(reactivating);
      next.delete(save.save_id);
      reactivating = next;
    }
  }

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
      // Use rescan when we already have a report, same wire payload, but the
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
    slot?: number,
    repoint = false,
  ) {
    try {
      const saved = await api.addGameToTracking({
        game_slug: game.slug,
        local_path: chosen,
        // Which folder of the title this is. 1 is the saved games; from 2 up
        // it is everything else (config, mods), synced exactly the same way.
        // The number is what pairs this folder with the one on another machine,
        // so it goes in explicitly, not derived from the path.
        slot,
        repoint,
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
      // The slot already points somewhere else. Don't decide for the user:
      // until aug-2026 this overwrote the path in silence, and anyone pointing
      // at a second folder ended up with the game backing up the new one and
      // the real one out of sync without a single warning.
      const busy = api.slotOccupied(e);
      if (busy) {
        slotClash = { game, chosen, busy };
        return;
      }
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  /** An add that landed on an occupied slot, waiting for the user to say
   *  whether to move what's there or whether this is a new folder. */
  let slotClash = $state<{
    game: DetectedGame;
    chosen: string;
    busy: api.SlotOccupied;
  } | null>(null);

  /** "Yes, same folder as always, it moved": move the slot. */
  async function resolveClashByMoving() {
    const c = slotClash;
    slotClash = null;
    if (c) await trackWithPath(c.game, c.chosen, undefined, true);
  }

  /** "No, this is something else of the same game": take the first free slot. */
  async function resolveClashAsNewSlot() {
    const c = slotClash;
    slotClash = null;
    if (c) await trackWithPath(c.game, c.chosen, c.busy.free_slot);
  }

  /** Track one specific save folder from the per-path list inside a card.
   *  Unlike the folder-picker flow it does NOT set a manual override, the
   *  other detected paths must stay visible so the user can monitor them too.
   *  The first folder tracked for a slug keeps the server default label; any
   *  extra folder carries its own path as the label to stay collision-free. */
  async function trackPath(game: DetectedGame, path: string) {
    // Only count THIS machine's own folders as "already tracked", an orphan
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
    // The title's first folder is slot 1 (the saved games) and there is
    // nothing to ask. After that there is: the number decides which folder on
    // the other machines this one pairs with, and Hoard cannot guess that.
    await addFolder(game, path);
  }

  /** Add a folder to a title. The first one goes to slot 1 with no questions.
   *  Once the title has slots, here or in the cloud, this opens the picker.
   *
   *  Auto-numbering does not work, and this is why: the same folder added on two
   *  machines came out with two different numbers (2 on Windows, 3 on Linux
   *  because by then Linux could see Windows' 2 taken) and then they never
   *  synced with each other, which was the whole point. Picking the number is
   *  picking what it pairs with. */
  async function addFolder(game: DetectedGame, chosen: string) {
    const opts = slotMap(game.slug);
    const usados = opts.filter((o) => o.kind !== "free").length;
    if (usados === 0) {
      await trackWithPath(game, chosen, 1);
      return;
    }
    slotPick = { game, chosen, opts, sel: nextFreeSlot(game.slug) };
  }

  /** An add waiting on the user to say which number it is. */
  let slotPick = $state<{
    game: DetectedGame;
    chosen: string;
    opts: SlotOption[];
    sel: number;
  } | null>(null);

  /** Commit the chosen number. Hooking onto a slot that already exists in the
   *  cloud means adopting its row, that is how the two machines share history,
   *  which is the entire point, not minting a new one with the same number. */
  async function confirmSlotPick() {
    const p = slotPick;
    slotPick = null;
    if (!p) return;
    const opt = p.opts.find((o) => o.n === p.sel);
    if (opt?.kind === "cloud" && opt.orphan) {
      await adoptOrphan(opt.orphan, p.chosen);
      return;
    }
    // A slot held here by another folder hits the engine's guard, which comes
    // back as the "did it move, or is it another one?" dialog.
    await trackWithPath(p.game, p.chosen, p.sel);
  }

  /** One slot of the title as offered to the user when adding a folder.
   *  `here` = already has a folder on this machine; `cloud` = exists in the
   *  cloud from another machine but nothing here; `free` = unused number. */
  type SlotOption = {
    n: number;
    kind: "here" | "cloud" | "free";
    /** The folder holding the slot on this machine, when `kind === "here"`. */
    path?: string;
    /** The cloud row to hook onto, when `kind === "cloud"`. */
    orphan?: TrackedSave;
  };

  /** A title's slot map: which numbers are taken here, which are waiting in the
   *  cloud, and which is the first free one.
   *
   *  It counts the cloud rows, not just this machine's, because the number IS
   *  the identity across machines: if the config ended up as 2 on Windows, it has
   *  to be able to be 2 on Linux too, or the two folders never see each other. */
  function slotMap(slug: string): SlotOption[] {
    const rows = tracked.filter((t) => t.game_slug === slug && t.slot !== null);
    const top = rows.reduce((m, t) => Math.max(m, t.slot as number), 0);
    const out: SlotOption[] = [];
    for (let n = 1; n <= top + 1; n += 1) {
      const here = rows.find((t) => t.slot === n && !t.orphan && t.local_path);
      if (here) {
        out.push({ n, kind: "here", path: here.local_path });
        continue;
      }
      const cloud = rows.find((t) => t.slot === n);
      out.push(
        cloud ? { n, kind: "cloud", orphan: cloud } : { n, kind: "free" },
      );
    }
    return out;
  }

  /** The lowest number nobody uses, here or in the cloud. What gets proposed by
   *  default for a folder that really is new. */
  function nextFreeSlot(slug: string): number {
    const opts = slotMap(slug);
    return (opts.find((o) => o.kind === "free") ?? opts[opts.length - 1]).n;
  }

  /** How a slot is labelled: its number, plus the user's name for it when they
   *  gave it one. Rows from before slots existed show their free-form label. */
  function slotLabel(save: TrackedSave): string {
    if (save.slot === null) return save.label;
    const n =
      save.slot === 1
        ? $_("library.slot_saves", { values: { n: save.slot } })
        : $_("library.slot_other", { values: { n: save.slot } });
    return save.name ? `${n} · ${save.name}` : n;
  }

  /** Just the name, for messages that talk about "this folder". */
  function slotName(save: TrackedSave): string {
    return save.name ?? slotLabel(save);
  }

  /** Number picked in the rename dialog, when the user changed it. */
  let renumberDraft = $state<number | null>(null);

  /** The numbers offered when renaming a folder.
   *
   *  Three kinds, and the middle one is the whole point. A number held by
   *  another folder **on this machine** is disabled: moving onto it would need
   *  that one out of the way first, and silently swapping two folders' numbers
   *  is the last thing anybody wants from a rename dialog. A number that exists
   *  only **in the cloud** is offered, because that is the other machine's copy
   *  of this same folder and joining it is what the user came here to do, it is
   *  how a folder that came out 3 on the second machine pairs with the 2 on the
   *  first. Everything else is free. */
  function renumberChoices(
    save: TrackedSave,
  ): { n: number; kind: "self" | "here" | "cloud" | "free" }[] {
    return slotMap(save.game_slug).map((o) => ({
      n: o.n,
      kind:
        o.n === save.slot
          ? "self"
          : o.kind === "here"
            ? "here"
            : o.kind === "cloud"
              ? "cloud"
              : "free",
    }));
  }

  /** The other machine's copy of this folder under number `n`, if that is what
   *  the number holds. Changing to it is a join, not a rename. */
  function cloudRowFor(save: TrackedSave, n: number): TrackedSave | null {
    const opt = slotMap(save.game_slug).find((o) => o.n === n);
    return opt?.kind === "cloud" ? (opt.orphan ?? null) : null;
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

  /** Folder chosen for a game through the scan dialog, both the "another
   *  folder" button and the "no save folder yet" one land here. The override is
   *  persisted so a re-scan doesn't revert to the heuristic guess. */
  async function useFolderForTarget(chosen: string) {
    const game = folderTargetGame;
    folderTargetGame = null;
    if (!game) return;
    // The override only makes sense for the saved-games slot: that is what
    // detection proposes and what automatic tracking picks up. Pinning it at the
    // config folder would put that one first on every rescan, so it is only
    // written when this folder is going to be slot 1.
    //
    // If it gets rejected, the folder belongs to another game, tracking anyway
    // would keep the bad half of the deal: this game watching someone else's
    // bytes and the real owner unable to track its own.
    const primera = slotMap(game.slug).every((o) => o.kind === "free");
    if (primera && !(await persistManualPath(game, chosen))) return;
    await addFolder(game, chosen);
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
  async function persistManualPath(
    game: DetectedGame,
    chosen: string,
  ): Promise<boolean> {
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
      return true;
    } catch (e) {
      // Antes se seguía adelante «para que al menos esta sesión funcione». Ya
      // no: el motivo más probable del rechazo es que esa carpeta sea de otro
      // juego, y ahí seguir es justo lo que hay que evitar. El error se
      // enseña y el llamante decide (no rastrea).
      toastError(typeof e === "string" ? e : (e as Error).message);
      return false;
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
    // Only the name half is editable. The number is what pairs this folder with
    // the same one on the other machines; typing it as free text is how naming a
    // slot "2 - Mods" used to drop it out of slot 2 without a word.
    renameDraft = save.name ?? "";
    renumberDraft = null;
  }

  async function confirmRename() {
    if (!renameTarget) return;
    const target = renameTarget;
    const trimmed = renameDraft.trim();
    const movingTo =
      renumberDraft !== null && renumberDraft !== target.slot
        ? renumberDraft
        : null;
    if (trimmed === (target.name ?? "") && movingTo === null) {
      renameTarget = null;
      return;
    }
    renaming = true;
    try {
      // Name first, number second, against the same row. Renumbering rewrites
      // the label the name lives in, so the other order would write the name
      // onto the old number and lose it on the move.
      let updated =
        target.slot === null
          ? await api.renameSaveLabel(target.save_id, trimmed)
          : await api.setSaveSlotName(target.save_id, trimmed || null);
      const joining = movingTo === null ? null : cloudRowFor(target, movingTo);
      if (joining) {
        // That number is the same folder on another machine. Pairing means
        // taking over ITS row, where the shared history lives, so this
        // machine's own row steps aside first. Renaming into it would only
        // bounce off `UNIQUE(user_id, game_slug, label)`.
        const path = target.local_path;
        await api.untrackSave(target.save_id);
        await adoptOrphan(joining, path);
        renameTarget = null;
        return;
      }
      if (movingTo !== null) {
        updated = await api.renumberSaveSlot(target.save_id, movingTo);
      }
      tracked = tracked.map((t) =>
        t.save_id === updated.save_id ? updated : t,
      );
      toastSuccess(
        $_("library.rename_success", { values: { name: slotName(updated) } }),
      );
      renameTarget = null;
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      const taken = api.slotTaken(e);
      if (taken !== null) {
        toastError($_("library.renumber_taken", { values: { n: taken } }));
      } else if (msg === api.LABEL_COLLISION) {
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
        let untracked = 0;
        try {
          untracked = await api.ignoreDetectedGame(target.slug);
        } catch (e) {
          showError(e);
          return;
        }
        // Blacklisting also stops tracking the saves filed under that slug,
        // so the list the user is looking at is stale.
        if (untracked > 0) {
          try {
            tracked = await api.listTrackedSaves();
          } catch {
            tracked = tracked.filter((t) => t.game_slug !== target.slug);
          }
        }
        // Re-fetch so the new blacklist entry takes effect immediately,
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
        toastSuccess(
          untracked > 0
            ? $_("library.ignored_untracked_toast", {
                values: { count: untracked },
              })
            : $_("library.ignored_toast"),
        );
      } else {
        sessionDismissed.add(target.slug);
        // Force a reactive update, Svelte 5 runes don't track Set mutations.
        sessionDismissed = new Set(sessionDismissed);
      }
      dismissTarget = null;
      dismissBlacklist = false;
    } finally {
      dismissBusy = false;
    }
  }

  // ---- derived views -------------------------------------------------

  // Saves with a local folder on this machine vs cloud-only saves from other
  // machines. Los primeros son los que salen marcados dentro de la rejilla
  // única; los segundos van a "En la nube, otras máquinas" (adoptables), que
  // sigue siendo una sección aparte porque la acción es otra, BUG 4.
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

  /** "Vincular a esta máquina…" on a cloud-orphan card. Opens the modal that
   *  offers what detection already found here, the folders for this slug, or
   *  any other detected game picked by name, and keeps the folder picker as
   *  the escape hatch. Going straight to the OS dialog (as 1.0.4 did after the
   *  UI rewrite dropped this wiring) makes the user hand-find a folder Hoard
   *  already knows. */
  function linkOrphan(orphan: TrackedSave) {
    linkingOrphan = orphan;
  }

  /** Folders this machine already tracks: the modal drops them from its
   *  candidate list so two saves can't land on one folder. */
  const trackedPaths = $derived(
    localSaves.map((t) => t.local_path).filter((p) => p.length > 0),
  );

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
  // "En la nube, otras máquinas" header.
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
  // "Volver a sugerencia automática" de las partidas monitorizadas, sólo se
  // enseña cuando el usuario tiene de verdad un override que limpiar.
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

  // ── Rejilla única ──────────────────────────────────────────────────────────
  //
  // Antes esta página tenía los juegos monitorizados arriba y, al final del
  // todo, la lista de detectados desde la que se dan de alta. Un juego ya
  // monitorizado salía en las dos, y lo que había que buscar para añadir algo
  // nuevo era justo lo que quedaba más lejos. Ahora hay una sola rejilla: una
  // tarjeta por juego, los monitorizados primero, y dentro de cada tarjeta
  // conviven lo que ya vigilamos y las rutas que la detección propone.

  /** Un juego de la biblioteca, con las dos mitades de lo que sabemos de él. */
  type LibraryEntry = {
    slug: string;
    name: string;
    appId: number | null;
    /** Fila de detección, si el escaneo lo encontró. */
    game: DetectedGame | null;
    /** Partidas monitorizadas en ESTA máquina para ese juego (pueden ser
     *  varias: un mismo juego con dos carpetas dadas de alta). */
    saves: TrackedSave[];
  };

  /** Chip de la barra: sólo los monitorizados, o todo lo que hay. */
  let onlyTracked = $state(false);

  const savesBySlug = $derived.by(() => {
    const m = new Map<string, TrackedSave[]>();
    for (const s of localSaves) {
      const arr = m.get(s.game_slug);
      if (arr) arr.push(s);
      else m.set(s.game_slug, [s]);
    }
    return m;
  });

  /** Todo lo que el escaneo conoce, filtros aparte: sirve para saber si un
   *  monitorizado tiene fila de detección o hay que sacarlo por su cuenta. */
  const detectedSlugs = $derived(
    new Set((report?.games ?? []).map((g) => g.slug)),
  );

  const allEntries = $derived.by(() => {
    const out: LibraryEntry[] = [];

    for (const g of filtered) {
      out.push({
        slug: g.slug,
        name: g.display_name,
        appId: g.steam_app_id,
        game: g,
        saves: savesBySlug.get(g.slug) ?? [],
      });
    }

    // Monitorizados que la detección no encuentra: añadidos a mano, emuladores,
    // o un juego que el escaneo ya no ve. Sin ellos, juntar las dos listas
    // perdería justo las partidas que el usuario dio de alta él mismo.
    const q = search.trim().toLowerCase();
    const filtersOff = confidenceFilter === "all" && sourceFilter === "all";
    for (const [slug, saves] of savesBySlug) {
      if (detectedSlugs.has(slug)) continue;
      const name = displayName(slug);
      if (q && !name.toLowerCase().includes(q)) continue;
      // No tienen grado ni origen que comparar, así que con un filtro de
      // detección puesto no pintan nada en el resultado.
      if (!filtersOff) continue;
      out.push({
        slug,
        name,
        appId: appIdBySlug.get(slug) ?? null,
        game: null,
        saves,
      });
    }

    // Lo que ya vigilas, primero; el resto por nombre.
    out.sort((a, b) => {
      const at = a.saves.length > 0;
      const bt = b.saves.length > 0;
      if (at !== bt) return at ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    return out;
  });

  const trackedEntryCount = $derived(
    allEntries.filter((e) => e.saves.length > 0).length,
  );

  const entries = $derived(
    onlyTracked ? allEntries.filter((e) => e.saves.length > 0) : allEntries,
  );

  /** Rutas detectadas que todavía no monitorizamos, con su índice original
   *  para que `pathConf` siga leyendo el grado correcto. Las que ya están
   *  dadas de alta viven en el bloque verde de la tarjeta, no aquí. */
  function untrackedPaths(game: DetectedGame): { path: string; i: number }[] {
    return game.found_paths
      .map((path, i) => ({ path, i }))
      .filter(({ path }) => !trackedByPath.has(path));
  }
</script>

<div class="mx-auto max-w-6xl px-8 py-8">
  <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
    <div class="min-w-0">
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
    <div class="flex shrink-0 items-center gap-2">
      <Button
        variant="secondary"
        onclick={() => (scanFolderOpen = true)}
        title={$_("scan_folder.title")}
        aria-label={$_("scan_folder.title")}
      >
        <FolderSearch size={16} />
      </Button>
      <Button variant="secondary" onclick={() => (emulatorModalOpen = true)}>
        <Gamepad2 size={16} />
        {$_("manual.add_button")}
      </Button>
      <Button onclick={runScan} loading={scanning}>
        <RefreshCw size={16} />
        {scanning ? $_("library.scanning") : report ? $_("library.rescan") : $_("library.scan_now")}
      </Button>
    </div>
  </header>

  {#if scanning && progress}
    <div class="mb-6 rounded-xl border border-white/[0.08] bg-zinc-950/40 p-4 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)]">
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

  {#snippet deepScanTile()}
    <button
      type="button"
      onclick={runDeepScan}
      disabled={scanning}
      use:tilt
      title={$_("library.deep_scan_hint")}
      class="tilt group flex flex-col rounded-xl border border-red-500/30 bg-red-950/20 p-4 text-left shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-red-500/55 hover:bg-red-950/30 disabled:cursor-not-allowed disabled:opacity-60"
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

  <!-- Una sola rejilla: monitorizados y detectados en la misma lista, con los
       primeros arriba. Ver el bloque "Rejilla única" del <script>. -->
  {#if report || localSaves.length > 0}
    <div class="mb-4 flex flex-wrap items-center gap-3">
      <div
        class="flex items-center gap-0.5 rounded-lg border border-white/[0.08] bg-zinc-900 p-0.5"
        role="group"
        aria-label={$_("library.filter_scope")}
      >
        <button
          type="button"
          onclick={() => (onlyTracked = true)}
          aria-pressed={onlyTracked}
          class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors {onlyTracked
            ? 'bg-emerald-500/15 text-emerald-300'
            : 'text-zinc-400 hover:text-zinc-200'}"
        >
          {$_("library.chip_tracked", { values: { count: trackedEntryCount } })}
        </button>
        <button
          type="button"
          onclick={() => (onlyTracked = false)}
          aria-pressed={!onlyTracked}
          class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors {onlyTracked
            ? 'text-zinc-400 hover:text-zinc-200'
            : 'bg-zinc-700/50 text-zinc-100'}"
        >
          {$_("library.chip_all", { values: { count: allEntries.length } })}
        </button>
      </div>

      <div class="min-w-[12rem] flex-1">
        <Input bind:value={search} placeholder={$_("library.search")} icon={SearchIcon} />
      </div>

      {#if report}
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
      {/if}

      {#if localSaves.length > 0}
        <!-- Total = huella LOCAL en esta máquina; las tarjetas etiquetan por
             separado el tamaño en la nube. Ver summary_local / size_server_title. -->
        <span
          class="inline-flex items-center gap-1.5 text-xs tabular-nums text-zinc-400"
          title={$_("library.size_local_title")}
        >
          <HardDrive size={11} class="shrink-0 text-zinc-500" />
          {$_("library.summary_local", { values: { count: localSaves.length, size: fmtBytes(trackedTotalBytes) } })}
        </span>
      {/if}
    </div>

    {#if entries.length === 0}
      <Card class={report ? "" : "mb-6"}>
        <div class="py-12 text-center text-sm text-zinc-400">
          {#if onlyTracked}
            {$_("library.no_tracked_yet")}
          {:else if report && report.games.length === 0}
            {$_("library.no_results_empty")}
          {:else}
            {$_("library.no_results_filtered")}
          {/if}
        </div>
      </Card>
      {#if report}
        <div class="mb-6 mt-3 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {@render deepScanTile()}
        </div>
      {/if}
    {:else}
      <!-- mb-6: la rejilla ya no es lo último de la página (debajo van la nube
           y las horas jugadas), así que necesita el mismo aire que el resto de
           secciones o el cubo del escaneo profundo queda pegado al siguiente
           encabezado. -->
      <div
        class="mb-6 grid gap-3"
        style="grid-template-columns: repeat(auto-fill, minmax({cardWidth('detected')}px, 1fr))"
      >
        {#each entries as entry (entry.slug)}
          {@const isTracked = entry.saves.length > 0}
          {@const pending = entry.game ? untrackedPaths(entry.game) : []}
          {@const expanded = expandedPaths.has(entry.slug)}
          <div
            class="tilt panel group relative flex flex-col overflow-hidden transition-[background-color,border-color,box-shadow] duration-150 hover:bg-[var(--surface-2)] {isTracked
              ? 'border-emerald-500/25 hover:border-emerald-500/40'
              : 'hover:border-[var(--edge-strong)]'}"
            use:tilt
          >
            <CardResizeHandle section="detected" />

            <div class="flex items-start justify-between gap-2 p-4 pb-3">
              <div class="flex min-w-0 items-center gap-3">
                <Cover
                  appId={entry.appId}
                  slug={entry.slug}
                  name={entry.name}
                  class="h-12 w-12 shrink-0 rounded-xl"
                  initialClass="text-lg"
                />
                <div class="min-w-0">
                  <h3 class="truncate text-sm font-medium text-zinc-100" title={entry.name}>
                    {entry.name}
                  </h3>
                  <p class="truncate text-xs text-zinc-500" title={entry.slug}>
                    {entry.slug}
                  </p>
                </div>
              </div>
              {#if entry.game}
                {@const game = entry.game}
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
                  <!-- Sólo una nota: Steam Cloud cubre la copia de Steam, se
                       puede desactivar por juego y no guarda histórico, así que
                       querer copia propia encima es de lo más normal. No cambia
                       el orden, ni el grado, ni el auto-track. -->
                  {#if game.steam_cloud}
                    <span
                      class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-zinc-400 ring-1 ring-inset ring-zinc-600/50"
                      title={$_("library.steam_cloud_note_title")}
                    >
                      <Cloud size={10} />
                      {$_("library.steam_cloud_note")}
                    </span>
                  {/if}
                </div>
              {/if}
            </div>

            {#if isTracked}
              <!-- Lo que esta máquina ya vigila del juego. Un slug puede tener
                   más de una partida dada de alta (dos carpetas distintas), así
                   que cada una trae su estado y sus botones. -->
              <div
                class="mx-3 mb-2 flex flex-col gap-2.5 rounded-lg bg-emerald-500/[0.06] p-2.5 ring-1 ring-inset ring-emerald-500/20"
              >
                {#each entry.saves as save (save.save_id)}
                  <div class="flex flex-col gap-1">
                    <div class="flex items-center justify-between gap-2">
                      {#if purgeDate(save.save_id)}
                        <span class="flex min-w-0 items-center gap-1 text-[11px] text-sky-300">
                          <Snowflake size={11} class="shrink-0" />
                          <span class="truncate">{$_("archived.frozen_note", { values: { date: purgeDate(save.save_id) } })}</span>
                        </span>
                      {:else}
                        <span class="flex min-w-0 items-center gap-1.5 text-[11px]">
                          <span
                            class="inline-block h-1.5 w-1.5 shrink-0 rounded-full {save.paused
                              ? 'bg-amber-400'
                              : 'bg-emerald-400'}"
                          ></span>
                          <span class={save.paused ? "text-amber-400" : "text-emerald-300"}>
                            {save.paused ? $_("library.paused_badge") : $_("library.monitored_badge")}
                          </span>
                          <!-- Which folder of the game this is. Only shown
                               when there is more than one: with a single
                               folder, saying "1 · saved games" is noise. -->
                          {#if entry.saves.length > 1}
                            <span
                              class="truncate rounded px-1.5 py-0.5 text-[10px] font-medium {save.slot === 1 ||
                              save.slot === null
                                ? 'bg-emerald-500/15 text-emerald-300'
                                : 'bg-zinc-700/40 text-zinc-400'}"
                            >
                              {slotLabel(save)}
                            </span>
                          {/if}
                        </span>
                      {/if}
                      <div class="flex shrink-0 items-center gap-0.5">
                        {#if purgeDate(save.save_id)}
                          <button
                            type="button"
                            onclick={() => reactivate(save)}
                            disabled={reactivating.has(save.save_id)}
                            title={$_("archived.reactivate")}
                            class="inline-flex shrink-0 items-center rounded-md bg-sky-500/15 p-1 text-sky-300 transition-colors hover:bg-sky-500/25 disabled:opacity-50"
                          >
                            <RotateCw
                              size={11}
                              class={reactivating.has(save.save_id) ? "animate-spin" : ""}
                            />
                          </button>
                        {/if}
                        <!-- Every folder is a full save with its own history,
                             so every folder needs its own way in. Restore lived
                             only behind the Dashboard card, which shows one
                             entry per game: with two folders tracked there was
                             no route at all to the second one's versions. -->
                        <button
                          type="button"
                          onclick={() => push(`/history/${save.save_id}`)}
                          aria-label={$_("library.open_history")}
                          title={$_("library.open_history")}
                          class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-emerald-300"
                        >
                          <History size={11} />
                        </button>
                        <button
                          type="button"
                          onclick={() => askRename(save)}
                          aria-label={$_("library.rename_button")}
                          title={$_("library.rename_title")}
                          class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
                        >
                          <Pencil size={11} />
                        </button>
                        {#if hasManualOverride(entry.slug)}
                          <button
                            type="button"
                            onclick={() => revertToAutoDetection(save)}
                            class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
                            title={$_("library.revert_to_auto")}
                          >
                            <RotateCcw size={11} />
                          </button>
                        {/if}
                        <button
                          type="button"
                          onclick={() => askUntrack(save)}
                          class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
                          title={$_("library.untrack_title")}
                        >
                          <Trash size={11} />
                        </button>
                        <button
                          type="button"
                          onclick={() => askDelete(save)}
                          class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-rose-400"
                          title={$_("library.delete_title")}
                        >
                          <Trash size={11} class="text-rose-500" />
                        </button>
                      </div>
                    </div>

                    <p
                      class="flex items-center gap-1 text-[10px] text-zinc-500"
                      title={save.local_path}
                    >
                      <FolderOpen size={10} class="shrink-0" />
                      <span class="truncate">{save.local_path}</span>
                    </p>

                    <!-- Carpeta vacía y ni una copia jamás: casi siempre la
                         ruta rastreada no es donde guarda el juego (la nativa
                         mientras corre por Proton, el contenedor en vez de su
                         `remote/`…). Pegajoso, no un toast: el barrido lo
                         reevalúa cada ciclo y un aviso efímero sonaría sin
                         parar. Se va solo en cuanto una copia aterriza. -->
                    {#if $wrongPathSuspected[save.save_id]}
                      <p
                        class="flex items-start gap-1 text-[10px] text-amber-400/90"
                        title={$_("library.wrong_path_hint")}
                      >
                        <AlertTriangle size={10} class="mt-px shrink-0" />
                        <span>{$_("library.wrong_path_hint")}</span>
                      </p>
                    {/if}

                    <!-- La última copia se dejó ficheros fuera porque no se
                         dejaron leer (un placeholder de OneDrive sin hidratar,
                         un permiso). Pegajoso y sin toast: la causa dura
                         mientras dure, y el aviso sonaría en cada copia. Se va
                         solo con la primera copia completa. El error del
                         sistema va en el `title` porque es lo único que dice
                         qué hay que arreglar. -->
                    <!-- La subida se rindió: el 409 que la reconciliación no
                         sabe resolver, cinco veces seguidas. Rojo y no ámbar
                         porque no hay reintento en camino — hasta que el
                         usuario pulse "copiar ahora" (o otro equipo publique
                         una versión), este save no sube. Es el aviso que faltó
                         durante 14 días en el caso que lo destapó. -->
                    {#if $backupBlocked[save.save_id]}
                      {@const blocked = $backupBlocked[save.save_id]}
                      <p
                        class="flex items-start gap-1 text-[10px] text-rose-400/90"
                        title={`${$_("library.backup_blocked_help")}\n\n${blocked.error}`}
                      >
                        <AlertTriangle size={10} class="mt-px shrink-0" />
                        <span>
                          {$_("library.backup_blocked_hint", {
                            values: { count: blocked.conflicts },
                          })}
                        </span>
                      </p>
                    {/if}

                    {#if $filesUnreadable[save.save_id]}
                      {@const bad = $filesUnreadable[save.save_id]}
                      <p
                        class="flex items-start gap-1 text-[10px] {bad.uploaded
                          ? 'text-amber-400/90'
                          : 'text-rose-400/90'}"
                        title={`${bad.path} — ${bad.error}`}
                      >
                        <AlertTriangle size={10} class="mt-px shrink-0" />
                        <span>
                          {bad.uploaded
                            ? $_("library.files_unreadable_hint", {
                                values: { count: bad.count },
                              })
                            : $_("library.files_unreadable_none_hint")}
                        </span>
                      </p>
                    {/if}

                    {#if (save.local_size_bytes ?? 0) > 0 || save.total_size_bytes > 0}
                      <div class="flex items-center gap-3 text-[10px] tabular-nums text-zinc-400">
                        {#if (save.local_size_bytes ?? 0) > 0}
                          <span
                            class="inline-flex items-center gap-1"
                            title={$_("library.size_local_title")}
                          >
                            <HardDrive size={10} class="shrink-0 text-zinc-500" />
                            {fmtBytes(save.local_size_bytes ?? 0)}
                          </span>
                        {/if}
                        {#if save.total_size_bytes > 0}
                          <span
                            class="inline-flex items-center gap-1"
                            title={$_("library.size_server_title")}
                          >
                            <Cloud size={10} class="shrink-0 text-zinc-500" />
                            {fmtBytes(save.total_size_bytes)}
                          </span>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}

            {#if entry.game}
              {@const game = entry.game}
              {#if pending.length}
                <!-- Rutas que la detección propone y todavía no seguimos. Las
                     ya dadas de alta salen arriba, en el bloque verde, para no
                     repetir la misma carpeta dos veces en la tarjeta. Cada una
                     lleva su propio grado: una carpeta ALTA no debe arrastrar a
                     una hermana BAJA. -->
                <div class="mx-3 mb-2 flex flex-col gap-1.5">
                  <span class="text-[10px] uppercase tracking-wide text-zinc-600">
                    {$_("library.suggested_paths")}
                  </span>
                  {#each expanded ? pending : pending.slice(0, 1) as entryPath (entryPath.path)}
                    <div class="flex items-center gap-2">
                      <span
                        class="inline-flex shrink-0 items-center rounded-full px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide ring-1 ring-inset {confidenceBadgeClass(
                          pathConf(game, entryPath.i),
                        )}"
                      >
                        {confidenceLabel(pathConf(game, entryPath.i))}
                      </span>
                      <span
                        class="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-500"
                        title={entryPath.path}
                      >
                        {entryPath.path}
                      </span>
                      <button
                        type="button"
                        onclick={() => trackPath(game, entryPath.path)}
                        aria-label={$_("library.track_this_path")}
                        title={$_("library.track_this_path")}
                        class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-emerald-500/10 hover:text-emerald-300"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                  {/each}
                  {#if pending.length > 1}
                    <button
                      type="button"
                      onclick={() => toggleExpand(entry.slug)}
                      class="self-start text-[11px] text-zinc-500 transition-colors hover:text-zinc-300"
                    >
                      {expanded
                        ? $_("library.show_less")
                        : $_("library.found_more", { values: { count: pending.length - 1 } })}
                    </button>
                  {/if}
                </div>
              {:else if !isTracked}
                <p class="mx-4 mb-2 text-[11px] italic text-zinc-600">
                  {$_("library.no_save_folder_yet")}
                </p>
              {/if}

              <div class="mt-auto flex items-center gap-2 p-3 pt-1">
                {#if !isTracked && !game.found_paths.length}
                  <!-- Coincidencia de Steam sin carpeta de guardado. Abre el
                       mismo escaneo de carpeta que el resto, con la explicación
                       y la ruta de instalación dentro (y escaneándola ya). -->
                  <button
                    type="button"
                    onclick={() => (folderTargetGame = game)}
                    aria-label={$_("library.no_save_alert_aria")}
                    title={$_("library.no_save_alert_aria")}
                    class="inline-flex items-center gap-1.5 rounded-md bg-amber-500/10 px-2.5 py-1.5 text-xs font-medium text-amber-300 ring-1 ring-inset ring-amber-500/30 transition-colors hover:bg-amber-500/20"
                  >
                    <AlertTriangle size={14} />
                    {$_("library.no_save_folder_yet")}
                  </button>
                {/if}
                <!-- Another folder for this game: a scan, not a file browser.
                     With the game already tracked the folder lands in the next
                     free slot (config, mods…), and the manual override is only
                     pinned for the saved-games one. -->
                <button
                  type="button"
                  onclick={() => (folderTargetGame = game)}
                  aria-label={isTracked
                    ? $_("library.add_folder_aria")
                    : $_("library.track_pick_folder_aria")}
                  title={isTracked
                    ? $_("library.add_folder_aria")
                    : $_("library.track_pick_folder_aria")}
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
            {/if}
          </div>
        {/each}
        {#if report}
          {@render deepScanTile()}
        {/if}
      </div>
    {/if}
  {:else if !scanning}
    <Card class="mb-6">
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

  {#if cloudOrphans.length > 0}
    <!-- Saves that live in the cloud but on OTHER machines (no local folder
         here). The primary action is to adopt them — link a folder on this
         machine to the existing cloud save so it starts syncing here (BUG 3/4). -->
    <section class="mb-6">
      <div
        class="mb-2 flex items-center justify-between gap-3 text-xs uppercase tracking-wide text-zinc-500"
      >
        <span>{$_("library.cloud_other_machines")}</span>
        <span
          class="inline-flex items-center gap-1.5 tabular-nums normal-case tracking-normal text-zinc-400"
          title={$_("library.size_server_title")}
        >
          <Cloud size={11} class="shrink-0 text-zinc-500" />
          {$_("library.summary_cloud", { values: { count: cloudOrphans.length, size: fmtBytes(cloudTotalBytes) } })}
        </span>
      </div>
      <div
        class="grid gap-2"
        style="grid-template-columns: repeat(auto-fill, minmax({cardWidth("orphans")}px, 1fr))"
      >
        {#each cloudOrphans as save (save.save_id)}
          <div
            class="tilt group relative flex flex-col overflow-hidden rounded-xl border border-white/[0.08] bg-zinc-950/40 p-3 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 hover:border-white/[0.12] hover:bg-zinc-900/50"
            use:tilt
          >
            <CardResizeHandle section="orphans" />
            <div class="flex items-start gap-2.5">
              <Cover
                appId={appIdBySlug.get(save.game_slug) ?? null}
                slug={save.game_slug}
                name={save.game_slug}
                class="h-9 w-9 shrink-0 rounded-lg"
                initialClass="text-xs"
              />
              <div class="min-w-0 flex-1 flex flex-col gap-0.5">
                <div class="flex items-center justify-between gap-1">
                  <p
                    class="truncate text-sm font-medium text-zinc-100"
                    title={save.game_slug}
                  >
                    {save.game_slug}
                  </p>
                  <div class="flex shrink-0 items-center gap-0.5">
                    <button
                      type="button"
                      onclick={() => linkOrphan(save)}
                      aria-label={$_("library.link_to_machine")}
                      title={$_("library.link_to_machine")}
                      class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-emerald-300"
                    >
                      <Link size={11} />
                    </button>
                    <button
                      type="button"
                      onclick={() => askDelete(save)}
                      aria-label={$_("library.delete_button")}
                      title={$_("library.delete_title")}
                      class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-rose-400"
                    >
                      <Trash size={11} class="text-rose-500" />
                    </button>
                  </div>
                </div>
                <p class="truncate text-[10px] text-zinc-500">
                  {save.label} · {$_("library.cloud_only_badge")}
                </p>
              </div>
            </div>

            <div class="mt-1.5 flex items-center justify-between text-[10px]">
              <span class="flex items-center gap-1.5 text-zinc-500">
                <span class="inline-block h-1.5 w-1.5 shrink-0 rounded-full bg-zinc-500"></span>
                {$_("library.cloud_only_badge")}
              </span>
              {#if save.total_size_bytes > 0}
                <span
                  class="inline-flex items-center gap-1 font-medium tabular-nums text-zinc-300"
                  title={$_("library.size_server_title")}
                >
                  <Cloud size={10} class="shrink-0 text-zinc-500" />
                  {fmtBytes(save.total_size_bytes)}
                </span>
              {/if}
            </div>
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
        class="grid gap-2"
        style="grid-template-columns: repeat(auto-fill, minmax({cardWidth("playtime")}px, 1fr))"
      >
        {#each playtimeGames as game (game.slug)}
          <div
            class="group relative flex items-center justify-between gap-2 rounded-xl border p-3 shadow-[inset_0_1px_0_0_rgba(255,255,255,0.03)] transition-all duration-150 {game.excluded
              ? 'border-white/[0.08] bg-zinc-950/40 opacity-50'
              : 'border-amber-500/40 bg-amber-500/10 hover:border-amber-500/60'}"
          >
            <CardResizeHandle section="playtime" />
            <Cover
              appId={appIdBySlug.get(game.slug) ?? null}
              slug={game.slug}
              name={game.display_name}
              class="h-9 w-9 shrink-0 rounded-md"
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


  <ManualTrackModal
    open={emulatorModalOpen}
    onClose={() => (emulatorModalOpen = false)}
    onAdded={(saved) => {
      tracked = [...tracked, saved];
    }}
  />

  <ScanFolderModal
    open={scanFolderOpen}
    onClose={() => (scanFolderOpen = false)}
    onAdded={(saved) => {
      tracked = [...tracked, saved];
    }}
  />

  <LinkOrphanModal
    open={linkingOrphan !== null}
    orphan={linkingOrphan}
    {trackedPaths}
    onClose={() => (linkingOrphan = null)}
    onPick={async (path) => {
      const target = linkingOrphan;
      linkingOrphan = null;
      if (target) await adoptOrphan(target, path);
    }}
    onScanned={(r) => {
      report = r;
    }}
  />

  <!-- Carpeta para UN juego concreto: el botón de "otra carpeta" y el aviso de
       "sin carpeta todavía" abren esto. Es el mismo escaneo que el general —
       antes uno abría el explorador a pelo y el otro un aviso con un botón que
       abría el explorador — con la explicación y la ruta de instalación de
       Steam dentro, que es la que se quiere mirar. -->
  <ScanFolderModal
    open={folderTargetGame !== null}
    target={folderTargetGame
      ? {
          slug: folderTargetGame.slug,
          display_name: folderTargetGame.display_name,
          install_dir: folderTargetGame.install_dir,
          seed_path: saveDirFor(folderTargetGame),
          steam_app_id: folderTargetGame.steam_app_id,
          no_paths: !folderTargetGame.found_paths.length,
        }
      : null}
    onClose={() => (folderTargetGame = null)}
    onPick={useFolderForTarget}
    onAdded={(saved) => {
      tracked = [...tracked, saved];
    }}
  />

  <!-- Which folder of the game this is. The number is not decoration: it is
       what pairs this folder with the ones on the other machines, so it gets
       asked instead of handed out in arrival order. Auto-numbered, the same
       folder added on Windows and on Linux came out 2 and 3 and never met. -->
  <Modal
    open={slotPick !== null}
    title={$_("library.slot_pick_title")}
    onClose={() => (slotPick = null)}
  >
    {#if slotPick}
      <p class="text-sm text-zinc-300">
        {$_("library.slot_pick_body", {
          values: { name: slotPick.game.display_name, path: slotPick.chosen },
        })}
      </p>
      <div class="mt-3 flex flex-col gap-1.5">
        {#each slotPick.opts as opt (opt.n)}
          <label
            class="flex cursor-pointer items-start gap-2.5 rounded-lg border p-2.5 transition-colors {slotPick.sel ===
            opt.n
              ? 'border-emerald-500/40 bg-emerald-500/[0.07]'
              : 'border-white/[0.08] hover:border-white/[0.14]'}"
          >
            <input
              type="radio"
              name="slot-pick"
              class="mt-0.5 accent-emerald-600"
              checked={slotPick.sel === opt.n}
              onchange={() => slotPick && (slotPick.sel = opt.n)}
            />
            <span class="min-w-0 flex-1">
              <span class="block text-sm text-zinc-200">
                {opt.n === 1
                  ? $_("library.slot_saves", { values: { n: opt.n } })
                  : $_("library.slot_other", { values: { n: opt.n } })}
              </span>
              <span class="block truncate text-xs text-zinc-500">
                {#if opt.kind === "here"}
                  {$_("library.slot_pick_here", { values: { path: opt.path } })}
                {:else if opt.kind === "cloud"}
                  {$_("library.slot_pick_cloud")}
                {:else}
                  {$_("library.slot_pick_free")}
                {/if}
              </span>
            </span>
          </label>
        {/each}
      </div>
    {/if}
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (slotPick = null)}>
        {$_("common.cancel")}
      </Button>
      <Button onclick={confirmSlotPick}>{$_("common.continue")}</Button>
    {/snippet}
  </Modal>

  <!-- The requested slot already points at another folder. This used to be a
       silent overwrite: pointing at a second folder moved the game onto it and
       left the real one out of sync with no warning. Both answers are
       legitimate — a game reinstalled on another drive DOES move its folder —
       so it asks instead of choosing for them. -->
  <Modal
    open={slotClash !== null}
    title={$_("library.slot_clash_title")}
    onClose={() => (slotClash = null)}
  >
    {#if slotClash}
      <p class="text-sm text-zinc-300">
        {$_("library.slot_clash_body", {
          values: {
            name: slotClash.game.display_name,
            current: slotClash.busy.current_path,
            chosen: slotClash.chosen,
          },
        })}
      </p>
    {/if}
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (slotClash = null)}>
        {$_("common.cancel")}
      </Button>
      <Button variant="secondary" onclick={resolveClashByMoving}>
        {$_("library.slot_clash_move")}
      </Button>
      <Button onclick={resolveClashAsNewSlot}>
        {$_("library.slot_clash_add", {
          values: { n: slotClash?.busy.free_slot ?? 2 },
        })}
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
    <!-- The blacklist also stops tracking whatever is filed under that slug,
         so say it before the click, not in the toast afterwards. Only shown
         when there is something to lose. -->
    {#if dismissBlacklist && dismissTrackedCount > 0}
      <p class="mt-2 pl-6 text-xs text-amber-300/90">
        {$_("library.ignore_blacklist_untracks", {
          values: { count: dismissTrackedCount },
        })}
      </p>
    {/if}
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
      placeholder={$_("library.rename_placeholder")}
      disabled={renaming}
    />
    <!-- The number is its own control on purpose. It is the identity the two
         machines match on, so it gets picked from the numbers this title
         actually has instead of typed into the name — writing "2 - Mods" as one
         string is what used to drop a folder out of slot 2 without a word. -->
    {#if renameTarget !== null && renameTarget.slot !== null}
      <label class="mt-3 flex items-center gap-2 text-xs text-zinc-400">
        <span class="text-zinc-500">{$_("library.slot_number_label")}</span>
        <select
          class="rounded-md border border-white/[0.08] bg-zinc-900 px-2 py-1.5 text-xs text-zinc-200 focus:border-emerald-500/40 focus:outline-none disabled:opacity-50"
          disabled={renaming}
          value={String(renumberDraft ?? renameTarget.slot)}
          onchange={(e) =>
            (renumberDraft = Number(
              (e.currentTarget as HTMLSelectElement).value,
            ))}
        >
          {#each renumberChoices(renameTarget) as opt (opt.n)}
            <option value={String(opt.n)} disabled={opt.kind === "here"}>
              {opt.n}{opt.kind === "here"
                ? ` — ${$_("library.slot_number_taken")}`
                : opt.kind === "cloud"
                  ? ` — ${$_("library.slot_number_join")}`
                  : ""}
            </option>
          {/each}
        </select>
      </label>
      <p class="mt-1.5 text-xs text-zinc-500">
        {$_("library.rename_keeps_number")}
      </p>
    {/if}
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
