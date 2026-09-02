<script lang="ts">
  /**
   * Dashboard, live view of every tracked save (1.1.0 redesign).
   *
   * A cover grid (one SaveGameCard per save) instead of the old list rows,
   * plus a summary bar with library-wide totals. Hydrates from
   * `list_tracked_saves`; live status pills are driven by the agent activity
   * store (subscribed at boot). Stored-version counts are fetched lazily per
   * card (`list_save_snapshots`) and refetched when the live session confirms
   * a new version for that save.
   *
   * The panel distinguishes the version THIS device holds from the cloud
   * head (local_version_num vs cloud_version_num, ADR 0021 D.10): the pill
   * is local, the cover chip is cloud. That split lives in SaveGameCard.
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { tilt } from "../lib/actions/tilt";
  import {
    AlertTriangle,
    Clock,
    Gamepad2,
    HardDrive,
    Layers,
    LogOut,
    Pin,
    Plus,
    RefreshCw,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Input from "../lib/components/Input.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import SaveGameCard from "../lib/components/SaveGameCard.svelte";
  import MirrorWarningBanner from "../lib/components/MirrorWarningBanner.svelte";
  import * as api from "../lib/api";
  import type {
    EngineDownReason,
    KeyringFault,
    MirrorWarning,
    TrackedSave,
  } from "../lib/api";
  import { auth, refreshQuota, signOut } from "../lib/stores/auth";
  import { storageGamesCloud } from "../lib/stores/cloud";
  import { activity, status } from "../lib/stores/agent";
  import {
    customNames,
    hydrateGameNames,
    setGameName,
  } from "../lib/stores/gameNames";
  import { cardWidth } from "../lib/stores/cardSizes.svelte";
  import {
    coverAspect,
    hydrateCoverAspect,
  } from "../lib/stores/coverAspect.svelte";
  import { toastError, toastSuccess } from "../lib/stores/toasts";
  import {
    formatBytes,
    formatDateTime,
    formatRelativeTime,
    prettifySlug,
  } from "../lib/utils/format";

  let saves = $state<TrackedSave[]>([]);
  let loading = $state(true);
  let signingOut = $state(false);
  let now = $state(Date.now());

  // Panel ordering. "recent" (default) = newest last backup first;
  // "size" = biggest cloud footprint first. Every size in this view is the
  // SERVER-side one (`total_size_bytes`), the panel never shows local sizes,
  // that's the Library's job.
  let sortBy = $state<"recent" | "size">("recent");

  const sortedSaves = $derived.by(() => {
    const arr = [...saves];
    if (sortBy === "size") {
      // Rank by the real cloud footprint when we have it: "biggest" has to
      // mean "what's eating the quota", which is the whole point of sorting
      // by size. Falls back to the head size before the footprints land.
      const weight = (s: TrackedSave) =>
        footprints[s.save_id] ?? s.total_size_bytes ?? 0;
      arr.sort((a, b) => weight(b) - weight(a));
    } else {
      const t = (s: TrackedSave) =>
        s.last_backup_at ? new Date(s.last_backup_at).getTime() : 0;
      arr.sort((a, b) => t(b) - t(a));
    }
    return arr;
  });

  // Slugs tracked more than once (two folders of the same game): the card
  // shows the label chip only for these, so single-save games stay clean.
  const dupSlugs = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const s of saves) {
      counts.set(s.game_slug, (counts.get(s.game_slug) ?? 0) + 1);
    }
    return new Set(
      [...counts].filter(([, n]) => n > 1).map(([slug]) => slug),
    );
  });

  // ---------------------------------------------------------------------
  // Stored-version counts (per-card "Total versions" + summary bar).
  // Fetched lazily after the list lands, one read-only call per save, never
  // blocking the grid. Refetched when the live session confirms a new
  // version for that save (upload committed / auto-restore landed).
  // ---------------------------------------------------------------------
  let versionCounts = $state<Record<string, number | null>>({});
  const countsRequested = new Set<string>();
  const seenLiveVersion = new Map<string, number>();

  async function fetchVersions(saveId: string) {
    try {
      const snaps = await api.listSaveSnapshots(saveId, false);
      versionCounts = { ...versionCounts, [saveId]: snaps.length };
    } catch {
      // Unavailable (an offline server, a pruned save): the card shows a dash.
      versionCounts = { ...versionCounts, [saveId]: null };
    }
  }

  $effect(() => {
    let anyChanged = false;
    for (const s of saves) {
      const live = $activity[s.save_id]?.last_version;
      const changed = live != null && seenLiveVersion.get(s.save_id) !== live;
      if (changed) seenLiveVersion.set(s.save_id, live);
      if (!countsRequested.has(s.save_id) || changed) {
        countsRequested.add(s.save_id);
        void fetchVersions(s.save_id);
        anyChanged ||= changed;
      }
    }
    // A committed version moves the footprint too. One call for the whole
    // batch, not one per save: the endpoint is account-wide.
    if (anyChanged) void fetchFootprints();
  });

  // ---------------------------------------------------------------------
  // Real cloud footprint, per save and account-wide.
  //
  // The manifest only carries each save's HEAD version (`total_size_bytes`),
  // so summing it answers "what would a restore pull down", never "what am I
  // storing", version history is forever, and a game whose folder is
  // rewritten wholesale each save can hold many times its head size. Showing
  // the head sum next to the sidebar's quota bar put two contradictory
  // totals on the same screen; a user with 34.9 MB of heads and 79 MB of
  // quota reasonably read it as a billing bug.
  //
  // `/v1/cloud/storage/games` already computes the honest number (it drives
  // the "free up space" dialog): per save, its exclusive deduplicated blobs.
  // Cloud-only, self-hosted has no quota and no black box, so there we keep
  // falling back to the head sum.
  // ---------------------------------------------------------------------
  let footprints = $state<Record<string, number>>({});
  let cloudUsed = $state<number | null>(null);
  let footprintsLoaded = $state(false);

  // Backup-mirror warnings for saves already tracked (P9). Read from the scan
  // cache rather than triggering a scan: the tick refreshes it every 10 min
  // anyway, and the panel must not pay for a detection pass on mount.
  let mirrorWarnings = $state<MirrorWarning[]>([]);

  async function loadMirrorWarnings() {
    try {
      const report = await api.cachedDetection();
      mirrorWarnings = report?.mirror_warnings ?? [];
    } catch {
      // No cache yet (first run) or unreadable: nothing to warn about that we
      // can prove, so stay quiet rather than guess.
      mirrorWarnings = [];
    }
  }

  async function fetchFootprints() {
    // Cloud only. Self-hosted (and signed out) settle straight into "asked and
    // there's nothing", so `undefined` keeps meaning "still loading" and the
    // cards don't sit on a spinner that will never resolve.
    if ($auth.user?.is_local_server !== false) {
      footprintsLoaded = true;
      return;
    }
    try {
      const s = await storageGamesCloud();
      const next: Record<string, number> = {};
      for (const g of s.games) next[g.save_id] = g.freeable_bytes;
      // Blobs two live saves share belong to neither's exclusive footprint,
      // so the per-card figures fall short of the account total by exactly
      // this much. Kept aside rather than smeared across the cards: the
      // account total has to keep matching the quota bar to the byte.
      footprints = next;
      cloudUsed = s.used_bytes;
      footprintsLoaded = true;
    } catch {
      // Offline / signed out: the cards fall back to their head size and the
      // summary keeps summing those. No error surfaced, this is a nicety on
      // a view that must still render.
      footprintsLoaded = true;
    }
  }

  // Summary-bar aggregates. Prefer the account's real deduped footprint (the
  // same number the quota bar shows) and only fall back to the head sum when
  // it isn't available.
  const headSize = $derived(
    saves.reduce((sum, s) => sum + (s.total_size_bytes ?? 0), 0),
  );
  const totalSize = $derived(cloudUsed ?? headSize);
  const totalVersions = $derived.by(() => {
    let sum = 0;
    let seen = false;
    for (const s of saves) {
      const c = versionCounts[s.save_id];
      if (typeof c === "number") {
        sum += c;
        seen = true;
      }
    }
    return seen ? String(sum) : "…";
  });
  const lastBackupAt = $derived.by(() => {
    let best: string | null = null;
    for (const s of saves) {
      if (s.last_backup_at && (!best || s.last_backup_at > best)) {
        best = s.last_backup_at;
      }
    }
    return best;
  });

  // Per-user "max versions per save" cap. `null` = unlimited. Edited right
  // here in the panel (explicit user request: not in Settings). A numeric
  // input's bind:value yields `undefined` while empty/invalid, so normalise
  // through `?? null` everywhere.
  let maxVersions = $state<number | null>(null);
  let maxVersionsInput = $state<number | null>(null);
  let savingMaxVersions = $state(false);

  // The backups the user asks for by hand carry their own cap, unlimited by
  // default. With a shared cap, a save that autosaves every minute fills the
  // history in one session and takes out exactly the copy somebody made on purpose
  // before a boss.
  let maxManual = $state<number | null>(null);
  let maxManualInput = $state<number | null>(null);
  let savingMaxManual = $state(false);

  const maxVersionsDirty = $derived((maxVersionsInput ?? null) !== maxVersions);
  const maxManualDirty = $derived((maxManualInput ?? null) !== maxManual);

  // When applying a cap would delete versions, we stop and ask first. Set to
  // the pending {cap, count} while the confirmation modal is open.
  let confirmPrune = $state<{
    cap: number;
    count: number;
    manual: boolean;
  } | null>(null);

  async function applyMaxVersions(manual = false) {
    const next = (manual ? maxManualInput : maxVersionsInput) ?? null;
    if (next != null && (!Number.isInteger(next) || next < 1 || next > 10000)) {
      toastError($_("dashboard.max_versions_invalid"));
      return;
    }
    if (manual) savingMaxManual = true;
    else savingMaxVersions = true;
    try {
      if (next != null) {
        // Dry-run first: if this cap would prune stored versions, ask before
        // touching anything. Clearing the cap never prunes, no dialog.
        const count = await api.previewMaxVersions(next, manual);
        if (count > 0) {
          confirmPrune = { cap: next, count, manual };
          return;
        }
      }
      await commitMaxVersions(next, manual);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingMaxVersions = false;
      savingMaxManual = false;
    }
  }

  async function confirmPruneAndApply() {
    if (!confirmPrune) return;
    const { cap, manual } = confirmPrune;
    if (manual) savingMaxManual = true;
    else savingMaxVersions = true;
    try {
      await commitMaxVersions(cap, manual);
      confirmPrune = null;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      savingMaxVersions = false;
      savingMaxManual = false;
    }
  }

  async function commitMaxVersions(next: number | null, manual: boolean) {
    await api.setMaxVersions(next, manual);
    if (manual) maxManual = next;
    else maxVersions = next;
    toastSuccess($_("dashboard.max_versions_saved"));
    // Pruning frees server space right away, reflect it on the bar.
    refreshQuota().catch(() => {});
    // …and the stored-version counts this panel shows.
    for (const s of saves) void fetchVersions(s.save_id);
  }

  // ---------------------------------------------------------------------
  // Rename (display name). Purely presentational, per-device: it writes the
  // `gameNames` override store. The slug, the sync key, never changes.
  // ---------------------------------------------------------------------
  let renameTarget = $state<TrackedSave | null>(null);
  let renameDraft = $state("");
  let renaming = $state(false);

  function askRename(save: TrackedSave) {
    renameTarget = save;
    // Seed only with an existing override: an empty field means "revert to
    // the automatic name" (the placeholder shows what that is).
    renameDraft = $customNames[save.game_slug] ?? "";
  }

  async function confirmRename() {
    if (!renameTarget) return;
    const slug = renameTarget.game_slug;
    renaming = true;
    try {
      await setGameName(slug, renameDraft);
      toastSuccess($_("dashboard.rename_success"));
      renameTarget = null;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      renaming = false;
    }
  }

  // Tick once a second so "next backup in 28s" countdowns and relative times
  // animate. Skip the state write while the window is hidden (minimised / in
  // the tray): nobody can see them, and the write would force a pointless
  // re-render.
  $effect(() => {
    const id = setInterval(() => {
      if (!document.hidden) now = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  // Poll the storage quota every 30s while the dashboard is open. The panel
  // no longer renders its own QuotaBar (the sidebar's QuotaMini shows it),
  // but this poll is what keeps that sidebar figure fresh. Hidden window →
  // skip the round-trip; the next visible tick refreshes it.
  $effect(() => {
    const id = setInterval(() => {
      if (!document.hidden) refreshQuota().catch(() => {});
    }, 30_000);
    return () => clearInterval(id);
  });

  onMount(async () => {
    api
      .getMaxVersions()
      .then((n) => {
        maxVersions = n;
        maxVersionsInput = n;
      })
      .catch(() => {});
    api
      .getMaxVersions(true)
      .then((n) => {
        maxManual = n;
        maxManualInput = n;
      })
      .catch(() => {});
    void hydrateGameNames();
    void hydrateCoverAspect();
    try {
      saves = await api.listTrackedSaves();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
    void fetchFootprints();
    void loadMirrorWarnings();
  });

  /** Which sentence explains an engine that isn't up. An older service (or a
   *  failure we don't classify) reports nothing, and then the generic line is
   *  the honest answer, inventing a cause would be worse than "it's down". */
  function offlineMessageKey(
    reason: EngineDownReason | undefined,
    keyring: KeyringFault | null | undefined,
  ): string {
    switch (reason) {
      case "no_session":
        return "dashboard.service_offline_no_session";
      case "keyring_unreadable":
        return keyringMessageKey(keyring);
      case "session_expired":
        return "dashboard.service_offline_expired";
      case "unreachable":
        return "dashboard.service_offline_unreachable";
      default:
        return "dashboard.service_offline_banner";
    }
  }

  /** "The keyring won't hand it over" is one reason with four different next
   *  steps, and getting that wrong sends the user after the wrong thing: a
   *  machine with no secret-service daemon has nothing to unlock, and being told
   *  to unlock a login keyring there is a dead end. What all four share is that
   *  signing in again works, the session then lands in Hoard's own protected
   *  file instead, and stays there. An older service doesn't classify, and then
   *  the general sentence is what shows, exactly as before. */
  function keyringMessageKey(fault: KeyringFault | null | undefined): string {
    switch (fault) {
      case "missing":
        return "dashboard.service_offline_keyring_missing";
      case "locked":
        return "dashboard.service_offline_keyring_locked";
      case "refused":
        return "dashboard.service_offline_keyring_refused";
      case "damaged":
        return "dashboard.service_offline_keyring_damaged";
      default:
        return "dashboard.service_offline_keyring";
    }
  }

  /** Signing in again is the actual fix for the session cases: it hands the
   *  service a session and, when the keyring is the problem, rewrites the item
   *  under the service so it can read it from then on. For anything else the
   *  button would be a placebo, so it isn't shown. */
  function fixableBySigningIn(reason: EngineDownReason | undefined): boolean {
    return (
      reason === "no_session" ||
      reason === "keyring_unreadable" ||
      reason === "session_expired"
    );
  }

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

  async function backupNow(save: TrackedSave) {
    try {
      await api.backupNow(save.save_id);
      toastSuccess($_("dashboard.backup_queued"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function togglePause(save: TrackedSave) {
    const next = !save.paused;
    try {
      await api.setSavePaused(save.save_id, next);
      saves = saves.map((s) =>
        s.save_id === save.save_id ? { ...s, paused: next } : s,
      );
      toastSuccess(
        next ? $_("dashboard.paused_toast") : $_("dashboard.resumed_toast"),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }
</script>

<div class="mx-auto max-w-[1600px] px-8 py-8">
  <header class="mb-6 flex items-start justify-between gap-4">
    <div class="min-w-0">
      <h1
        class="font-display text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50"
      >
        {$_("dashboard.title")}
      </h1>
      <p class="mt-2 flex items-center gap-2.5 text-sm text-zinc-400">
        <span class="relative inline-flex h-3 w-3 shrink-0">
          {#if $status.running}
            <span
              class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400/70"
            ></span>
          {/if}
          <span
            class="relative inline-flex h-3 w-3 rounded-full {$status.running
              ? 'bg-emerald-400 shadow-[0_0_8px_2px_rgba(16,185,129,0.5)]'
              : 'bg-zinc-600'}"
          ></span>
        </span>
        <span>
          {$status.running
            ? $_("dashboard.agent_watching")
            : $_("dashboard.agent_offline")}
          {#if $status.running}
            <span class="text-zinc-600">·</span>
            {$_("dashboard.tracked_count", { values: { count: saves.length } })}
          {/if}
        </span>
      </p>
    </div>
    <!-- mr-20 clears the shell's fixed bell/eye overlay (top-right corner). -->
    <div class="mr-20 flex shrink-0 items-center gap-2">
      <Button
        variant="ghost"
        onclick={handleLogout}
        loading={signingOut}
        aria-label={$_("dashboard.sign_out")}
      >
        <LogOut size={16} />
        {$_("dashboard.sign_out")}
      </Button>
    </div>
  </header>

  <!-- Sync service down — and, since 1.1.1, *why*. The service reports a typed
       reason (`AgentStatus.reason`); this banner says it and offers the one
       action that fixes the session cases. The generic line stays as the
       fallback for an older service or an unclassified failure. -->
  <!-- Above the offline banner on purpose: the service being down is a
       transient the user already knows about, while syncing the wrong folder
       has been quietly costing them quota for weeks. -->
  {#if !loading && mirrorWarnings.length > 0}
    <MirrorWarningBanner
      warnings={mirrorWarnings}
      {footprints}
      onFixed={async () => {
        saves = await api.listTrackedSaves();
        await loadMirrorWarnings();
        void fetchFootprints();
        void refreshQuota();
      }}
    />
  {/if}

  <!-- `known` and not just `!running`: the store starts blank, and a banner
       that reads the blank paints "the service is stopped" over an app that is
       still opening — which is how a service with thirteen hours of uptime got
       reported as down. -->
  {#if !loading && $status.known && !$status.running}
    <div
      class="mb-5 flex items-start gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-2.5 text-sm text-amber-200"
    >
      <AlertTriangle size={15} class="mt-0.5 shrink-0 text-amber-400" />
      <div class="min-w-0 flex-1">
        <p>{$_(offlineMessageKey($status.reason, $status.keyring))}</p>
        <!-- El texto crudo del servicio. No es para el usuario medio, es para
             que pueda pegarlo en un reporte sin tener que encontrar el log. -->
        {#if $status.last_error}
          <p class="mt-1 break-words font-mono text-[11px] text-amber-200/60">
            {$status.last_error}
          </p>
        {/if}
      </div>
      {#if fixableBySigningIn($status.reason)}
        <Button variant="ghost" onclick={handleLogout} loading={signingOut}>
          {$_("dashboard.service_offline_signin")}
        </Button>
      {/if}
    </div>
  {/if}

  <!-- Resumen de toda la biblioteca. Vivía al pie de la rejilla, donde con
       muchos juegos no se veía sin bajar del todo; estos cuatro números son
       justo lo que se mira de un vistazo, así que van antes de las tarjetas. -->
  {#if !loading && saves.length > 0}
    <div
      class="tilt panel relative mb-5 grid grid-cols-2 gap-x-4 gap-y-5 px-6 py-5 md:grid-cols-4"
      use:tilt
    >
      <div>
        <p class="text-xs text-zinc-500">{$_("dashboard.total_games")}</p>
        <p
          class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-zinc-100"
        >
          <Gamepad2 size={18} class="text-zinc-500" />
          <span class="tabular-nums">{saves.length}</span>
        </p>
      </div>
      <div>
        <p class="text-xs text-zinc-500">{$_("dashboard.total_versions")}</p>
        <p
          class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-zinc-100"
        >
          <Layers size={18} class="text-zinc-500" />
          <span class="tabular-nums">{totalVersions}</span>
        </p>
      </div>
      <div>
        <p class="text-xs text-zinc-500">
          {$_(cloudUsed != null ? "dashboard.cloud_total" : "dashboard.total_size")}
        </p>
        <p
          class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-zinc-100"
          title={cloudUsed != null ? $_("dashboard.cloud_total_title") : undefined}
        >
          <HardDrive size={18} class="text-zinc-500" />
          <span class="tabular-nums">{formatBytes(totalSize)}</span>
        </p>
        <!-- The head sum is still worth showing — it's what a fresh restore
             would pull — but only once it's clear it isn't the total, and
             only when history actually makes the two differ. -->
        {#if cloudUsed != null && headSize > 0 && cloudUsed > headSize * 1.1}
          <p class="mt-0.5 pl-[26px] text-[11px] tabular-nums text-zinc-500">
            {$_("dashboard.current_versions_sub", {
              values: { size: formatBytes(headSize) },
            })}
          </p>
        {/if}
      </div>
      <div>
        <p class="text-xs text-zinc-500">{$_("dashboard.last_backup")}</p>
        {#if lastBackupAt}
          <p
            class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-zinc-100"
          >
            <Clock size={18} class="text-zinc-500" />
            {formatRelativeTime(lastBackupAt, now)}
          </p>
          <p class="mt-0.5 pl-[26px] text-[11px] tabular-nums text-zinc-500">
            {formatDateTime(lastBackupAt)}
          </p>
        {:else}
          <p
            class="mt-1.5 flex items-center gap-2 text-xl font-semibold text-zinc-100"
          >
            <Clock size={18} class="text-zinc-500" />
            <span>{$_("dashboard.never_saved")}</span>
          </p>
        {/if}
      </div>
    </div>

    <!-- Ya no hay selector de forma de carátula: el tamaño y el encuadre se
         arrastran desde la esquina de cualquier tarjeta, como en la
         biblioteca. -->
    <div
      class="mb-5 flex flex-wrap items-center justify-end gap-x-5 gap-y-3"
    >
      <label class="flex items-center gap-2 text-xs text-zinc-400">
        <span class="text-zinc-500">{$_("dashboard.sort_label")}</span>
        <select
          class="rounded-md border border-white/[0.08] bg-zinc-900 px-2 py-1.5 text-xs text-zinc-200 focus:border-emerald-500/40 focus:outline-none"
          bind:value={sortBy}
        >
          <option value="recent">{$_("dashboard.sort_recent")}</option>
          <option value="size">{$_("dashboard.sort_size")}</option>
        </select>
      </label>

      <!-- Max stored versions per game. Server-side, per-user; lowering it
           prunes the oldest versions immediately. -->
      <label
        class="flex items-center gap-2 text-xs text-zinc-400"
        title={$_("dashboard.max_versions_hint")}
      >
        <Layers size={13} class="text-zinc-500" />
        <span class="text-zinc-500">{$_("dashboard.max_versions_label")}</span>
        <input
          type="number"
          min="1"
          max="10000"
          placeholder="∞"
          bind:value={maxVersionsInput}
          disabled={savingMaxVersions}
          class="w-16 rounded-md border border-white/[0.08] bg-zinc-900 px-2 py-1.5 text-xs text-zinc-200 [appearance:textfield] focus:border-emerald-500/40 focus:outline-none disabled:opacity-50 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
        />
        {#if maxVersionsDirty}
          <Button
            variant="secondary"
            class="!px-2.5 !py-1.5 !text-xs"
            onclick={() => applyMaxVersions(false)}
            loading={savingMaxVersions}
          >
            {$_("dashboard.max_versions_apply")}
          </Button>
        {/if}
      </label>

      <!-- Cupo aparte para las copias que pide el usuario. Sin límite por
           defecto: son pocas y son las que se quieren conservar. -->
      <label
        class="flex items-center gap-2 text-xs text-zinc-400"
        title={$_("dashboard.max_manual_hint")}
      >
        <Pin size={13} class="text-zinc-500" />
        <span class="text-zinc-500">{$_("dashboard.max_manual_label")}</span>
        <input
          type="number"
          min="1"
          max="10000"
          placeholder="∞"
          bind:value={maxManualInput}
          disabled={savingMaxManual}
          class="w-16 rounded-md border border-white/[0.08] bg-zinc-900 px-2 py-1.5 text-xs text-zinc-200 [appearance:textfield] focus:border-emerald-500/40 focus:outline-none disabled:opacity-50 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
        />
        {#if maxManualDirty}
          <Button
            variant="secondary"
            class="!px-2.5 !py-1.5 !text-xs"
            onclick={() => applyMaxVersions(true)}
            loading={savingMaxManual}
          >
            {$_("dashboard.max_versions_apply")}
          </Button>
        {/if}
      </label>

      <Button onclick={() => push("/library")}>
        <Plus size={15} />
        {$_("dashboard.add_game")}
      </Button>
    </div>
  {/if}

  {#if loading}
    <Card>
      <div class="shimmer py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else if saves.length === 0}
    <Card>
      <div class="py-16 text-center">
        <div
          class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/30"
        >
          <RefreshCw size={20} />
        </div>
        <h2 class="text-base font-medium text-zinc-100">
          {$_("dashboard.no_saves_title")}
        </h2>
        <p class="mx-auto mt-2 max-w-md text-sm text-zinc-400">
          {$_("dashboard.no_saves_body")}
        </p>
        <div class="mt-6">
          <Button onclick={() => push("/library")}>
            <Plus size={15} />
            {$_("dashboard.add_game")}
          </Button>
        </div>
      </div>
    </Card>
  {:else}
    <!-- Columnas por ancho mínimo, no por breakpoint: ese ancho es justo lo
         que mueve la esquina de arrastre de las tarjetas. -->
    <div
      class="grid gap-5"
      style="grid-template-columns: repeat(auto-fill, minmax(min({cardWidth(
        'dashboard',
      )}px, 100%), 1fr))"
    >
      {#each sortedSaves as save (save.save_id)}
        <SaveGameCard
          {save}
          {now}
          versions={versionCounts[save.save_id]}
          footprintBytes={footprintsLoaded
            ? (footprints[save.save_id] ?? null)
            : undefined}
          agentRunning={$status.running}
          showLabel={dupSlugs.has(save.game_slug)}
          aspect={coverAspect()}
          onRename={askRename}
          onBackup={backupNow}
          onTogglePause={togglePause}
          onHistory={(s) => push(`/history/${s.save_id}`)}
        />
      {/each}
    </div>
  {/if}
</div>

<!-- Lowering the cap below the stored history is destructive — the server
     prunes immediately. The dry-run count feeds this confirmation, so the
     user sees exactly how many versions are about to go. -->
<Modal
  open={!!confirmPrune}
  title={$_("dashboard.max_versions_confirm_title")}
  dismissible={!savingMaxVersions}
  onClose={() => {
    if (!savingMaxVersions) confirmPrune = null;
  }}
>
  {#if confirmPrune}
    <div class="space-y-3 text-sm text-zinc-300">
      <p>
        {$_("dashboard.max_versions_confirm_body", {
          values: { cap: confirmPrune.cap, count: confirmPrune.count },
        })}
      </p>
      <div
        class="flex items-start gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-200"
      >
        <AlertTriangle size={14} class="mt-0.5 shrink-0" />
        <span>{$_("dashboard.max_versions_confirm_note")}</span>
      </div>
    </div>
  {/if}
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (confirmPrune = null)}
      disabled={savingMaxVersions}
    >
      {$_("common.cancel")}
    </Button>
    <Button
      variant="danger"
      onclick={confirmPruneAndApply}
      loading={savingMaxVersions}
    >
      {$_("dashboard.max_versions_confirm_apply")}
    </Button>
  {/snippet}
</Modal>

<!-- Rename (display name). Per-device override via the gameNames store; the
     slug that ties detection, covers and the server row together never
     changes. Empty field = revert to the automatic name. -->
<Modal
  open={renameTarget !== null}
  title={$_("dashboard.rename_modal_title")}
  dismissible={!renaming}
  onClose={() => {
    if (!renaming) renameTarget = null;
  }}
>
  <p class="mb-3 text-sm text-zinc-300">
    {$_("dashboard.rename_modal_body")}
  </p>
  <Input
    bind:value={renameDraft}
    placeholder={renameTarget ? prettifySlug(renameTarget.game_slug) : ""}
    disabled={renaming}
    onkeydown={(e) => {
      if (e.key === "Enter" && !renaming) void confirmRename();
    }}
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
      {$_("common.save")}
    </Button>
  {/snippet}
</Modal>
