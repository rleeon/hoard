<script lang="ts">
  /**
   * Settings — every persistent app preference lives here.
   *
   * Each row is its own atomic toggle: flipping a switch writes to disk
   * immediately, so users don't have to hunt for a Save button. The "danger"
   * actions (clear cache, sign out) sit at the bottom in a separate card so
   * they can't be hit by reflex.
   */
  import { onMount, onDestroy } from "svelte";
  import { push } from "svelte-spa-router";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { _, locale } from "svelte-i18n";
  import {
    Power,
    BellRing,
    BellOff,
    Minimize2,
    LogIn,
    LogOut,
    Info,
    FileText,
    ChevronRight,
    BarChart3,
    RefreshCw,
    Database,
    Languages,
    Activity,
    DownloadCloud,
    Server,
    ServerCog,
    CloudOff,
  } from "lucide-svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import SettingsRow from "../lib/components/SettingsRow.svelte";
  import { prefs, hydratePrefs, updatePrefs } from "../lib/stores/prefs";
  import { auth, signOut } from "../lib/stores/auth";
  import { cloud, planLabel } from "../lib/stores/cloud";
  import { supportedLocales, setLocale } from "../lib/i18n";
  import * as api from "../lib/api";
  import { APP_VERSION } from "../lib/version";
  import { toastError, toastInfo, toastSuccess } from "../lib/stores/toasts";
  import { showError } from "../lib/stores/error_dialog";
  import {
    checkForUpdates,
    lastReport,
    triggerServerUpgrade,
  } from "../lib/stores/updates";

  let saving = $state<string | null>(null);
  let signingOut = $state(false);
  // Gate the "forget server" action behind a confirm modal. Forgetting wipes
  // the saved address + token (session.toml + keyring), which is what stops
  // the app reconnecting to a dead/abandoned self-hosted box on every launch.
  let forgetModalOpen = $state(false);

  // Local mirrors of the two slider-backed prefs so the thumb drags smoothly
  // without a round-trip on every `input` event. The committed value is only
  // pushed when the user releases (`onchange`). `$effect` keeps these in sync
  // when prefs hydrate or another part of the app updates them (e.g. the
  // sidebar toggle cascading defaults).
  let scanIntervalHours = $state(6);
  let conflictRetentionDays = $state(14);
  let cloudPollSecs = $state(10);
  // "Ahorro de datos" knob, kept as 0..100 for the slider; persisted as
  // 0..1 (ADR 0018). Mirrors the same drag-smoothly / commit-on-release
  // pattern as the other sliders.
  let dataSavingPct = $state(30);
  $effect(() => {
    const p = $prefs;
    if (!p) return;
    scanIntervalHours = p.automatic_scan_interval_hours ?? 6;
    conflictRetentionDays = p.conflict_retention_days ?? 14;
    cloudPollSecs = p.cloud_poll_interval_secs ?? 10;
    dataSavingPct = Math.round((p.data_saving ?? 0.3) * 100);
  });

  // Pro accounts get the snappy 2s floor; Free stays at 5s so a casual user
  // can't accidentally hammer the server. The backend mirrors this (2..=300)
  // and the cloud bandwidth window is the ultimate backstop.
  const cloudPollMin = $derived(
    $cloud.account && $cloud.account.plan !== "free" ? 2 : 5,
  );

  async function commitScanInterval() {
    if (!$prefs) return;
    const value = scanIntervalHours;
    saving = "automatic_scan_interval_hours";
    try {
      const updated = await api.setSchedulerInterval(value);
      prefs.set(updated);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      // Roll back the slider to the persisted value so the UI doesn't lie
      // about an unsaved change.
      scanIntervalHours = $prefs?.automatic_scan_interval_hours ?? 6;
    } finally {
      saving = null;
    }
  }

  async function commitConflictRetention() {
    if (!$prefs) return;
    const value = conflictRetentionDays;
    saving = "conflict_retention_days";
    try {
      const updated = await api.setConflictRetention(value);
      prefs.set(updated);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      conflictRetentionDays = $prefs?.conflict_retention_days ?? 14;
    } finally {
      saving = null;
    }
  }

  async function commitCloudPoll() {
    if (!$prefs) return;
    const value = cloudPollSecs;
    saving = "cloud_poll_interval_secs";
    try {
      const updated = await api.setCloudPollInterval(value);
      prefs.set(updated);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      cloudPollSecs = $prefs?.cloud_poll_interval_secs ?? 10;
    } finally {
      saving = null;
    }
  }

  async function commitDataSaving() {
    if (!$prefs) return;
    const value = dataSavingPct / 100;
    saving = "data_saving";
    try {
      const updated = await api.setDataSaving(value);
      prefs.set(updated);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      dataSavingPct = Math.round(($prefs?.data_saving ?? 0.3) * 100);
    } finally {
      saving = null;
    }
  }

  // User-blacklisted slugs from the Library page. Hydrated on mount; the
  // "Reactivar" button calls `unignoreDetectedGame` and re-fetches so the
  // list stays in step with disk state.
  let ignored = $state<string[]>([]);
  let ignoredBusy = $state<string | null>(null);

  async function refreshIgnored() {
    try {
      ignored = await api.listIgnoredSlugs();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function reactivateIgnored(slug: string) {
    ignoredBusy = slug;
    try {
      await api.unignoreDetectedGame(slug);
      await refreshIgnored();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      ignoredBusy = null;
    }
  }

  // Catalog (Ludusavi) update state. We poll status once on mount and listen
  // for `catalog://update-progress` events while a refresh is in flight so
  // the button can show "Downloading…" / "Parsing…" / "Saving…" instead of a
  // mute spinner.
  let catalog = $state<api.CatalogStatus | null>(null);
  let updatingCatalog = $state(false);
  let catalogStage = $state<string>("");
  let unlistenCatalog: UnlistenFn | null = null;

  // Hidden diagnostics panel — only visible after 5 clicks on the sidebar
  // version. We re-poll every 2s while the page is open; the round-trip is
  // cheap (just locks a Mutex + a oneshot through the agent loop).
  let diagnosticsUnlocked = $state(false);
  let agentSlots = $state<api.AgentSlotStatus[]>([]);
  let diagnosticsTimer: ReturnType<typeof setInterval> | null = null;

  async function refreshAgentSlots() {
    try {
      agentSlots = await api.agentStatus();
    } catch (e) {
      console.warn("agentStatus failed:", e);
    }
  }

  function formatTimestamp(iso: string | null): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    const ageSecs = Math.max(0, Math.floor((Date.now() - d.getTime()) / 1000));
    if (ageSecs < 5) return $_("history.relative_just_now");
    if (ageSecs < 60)
      return $_("diagnostics.relative_seconds", { values: { count: ageSecs } });
    if (ageSecs < 3600)
      return $_("history.relative_minutes", {
        values: { count: Math.floor(ageSecs / 60) },
      });
    if (ageSecs < 86400)
      return $_("history.relative_hours", {
        values: { count: Math.floor(ageSecs / 3600) },
      });
    return $_("history.relative_days", {
      values: { count: Math.floor(ageSecs / 86400) },
    });
  }

  onMount(async () => {
    await hydratePrefs();
    // Re-sync autostart from the OS so the toggle reflects truth even if the
    // user removed the launcher entry manually.
    try {
      const real = await api.isAutostartEnabled();
      const current = $prefs;
      if (current && current.autostart !== real) {
        await updatePrefs({ autostart: real });
      }
    } catch (e) {
      console.warn("isAutostartEnabled probe failed:", e);
    }
    try {
      catalog = await api.catalogStatus();
    } catch (e) {
      console.warn("catalogStatus failed:", e);
    }
    unlistenCatalog = await listen<string>("catalog://update-progress", (ev) => {
      catalogStage = ev.payload;
    });

    await refreshIgnored();

    diagnosticsUnlocked =
      sessionStorage.getItem("hoard-diagnostics-unlocked") === "1";
    if (diagnosticsUnlocked) {
      await refreshAgentSlots();
      diagnosticsTimer = setInterval(refreshAgentSlots, 2000);
    }

    // If we don't have a cached update report yet (user opened Settings
    // before the boot-time probe finished, or before the 30-min poller
    // ran), fire one now so the Server panel can show the version + any
    // pending upgrade without forcing a manual click.
    if ($lastReport == null) {
      checkForUpdates().catch((e) =>
        console.warn("Settings update probe failed:", e),
      );
    }
  });

  onDestroy(() => {
    unlistenCatalog?.();
    if (diagnosticsTimer) clearInterval(diagnosticsTimer);
  });

  // Hoard-server panel. Shown for self-hosted sessions (local or public-DNS
  // boxes) but hidden when connected to the managed Hoard Cloud: the cloud
  // upgrades itself and has no `/v1/admin/upgrade` route, so the button there
  // only ever returned "HTTP 404 Not Found". `is_cloud_server` is classified
  // by Rust on login (see commands/auth.rs::classify_cloud).
  const serverUpdate = $derived($lastReport?.server ?? null);
  const showServerCard = $derived(
    $auth.user != null && $auth.user.is_cloud_server !== true,
  );
  // The remote-upgrade button asks the server to upgrade *itself* over HTTP
  // (ADR 0017), so it works from any OS and whether the server is local or on
  // another box. The only requirement is an admin token — the server rejects
  // the request otherwise. Non-admins fall back to copying the shell command.
  const canInAppUpgrade = $derived(
    $auth.user?.is_admin === true && $auth.user?.is_cloud_server !== true,
  );
  let refreshingServer = $state(false);
  let upgradingServer = $state(false);
  let copyingUpgrade = $state(false);

  async function handleServerRefresh() {
    refreshingServer = true;
    try {
      await checkForUpdates();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      refreshingServer = false;
    }
  }

  /**
   * Admin: ask the self-hosted server to upgrade itself over HTTP (ADR 0017).
   * The Rust command POSTs `/v1/admin/upgrade` and then polls `/v1/health`
   * until the version flips. `confirmed` means it came back on the new
   * version; `scheduled` means the request was accepted but we couldn't
   * confirm the restart before the poll window closed (still likely fine).
   */
  async function handleServerRemoteUpgrade() {
    if (upgradingServer) return;
    upgradingServer = true;
    try {
      const outcome = await triggerServerUpgrade();
      if (outcome.kind === "confirmed") {
        toastSuccess(
          $_("settings.server_remote_upgrade_confirmed", {
            values: { version: outcome.version },
          }),
        );
      } else {
        toastInfo($_("settings.server_remote_upgrade_scheduled"));
      }
      // Re-probe so the version line updates without forcing a manual click.
      checkForUpdates().catch((e) =>
        console.warn("post-upgrade probe failed:", e),
      );
    } catch (e) {
      // The command fails with a structured AppError (i18n keys) — surface it
      // through the global error dialog so the user sees the actual reason
      // (not logged in, forbidden, unreachable…) rather than a raw string.
      showError(e);
    } finally {
      upgradingServer = false;
    }
  }

  /**
   * Remote server (or any non-Linux client): copy the command to the
   * clipboard so the user can paste it over SSH on the server host.
   */
  async function handleServerCopyCommand() {
    if (copyingUpgrade) return;
    copyingUpgrade = true;
    try {
      try {
        await navigator.clipboard.writeText("sudo hoard-server upgrade");
        toastSuccess($_("settings.server_command_copied"));
      } catch {
        toastInfo($_("settings.server_command_manual"));
      }
    } finally {
      copyingUpgrade = false;
    }
  }

  async function handleCatalogUpdate() {
    updatingCatalog = true;
    catalogStage = "downloading";
    try {
      const result = await api.updateCatalog();
      catalog = {
        games: result.games,
        has_runtime_override: true,
        updated_at: result.updated_at,
      };
      toastSuccess(
        $_("settings.catalog_updated_toast", {
          values: { count: result.games.toLocaleString() },
        }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      updatingCatalog = false;
      catalogStage = "";
    }
  }

  function formatRelative(epochSecs: number | null): string {
    if (!epochSecs) return $_("settings.catalog_using_bundled");
    const ageSecs = Math.max(0, Math.floor(Date.now() / 1000) - epochSecs);
    if (ageSecs < 60) return $_("settings.catalog_updated_just_now");
    if (ageSecs < 3600)
      return $_("settings.catalog_updated_minutes", {
        values: { count: Math.floor(ageSecs / 60) },
      });
    if (ageSecs < 86400)
      return $_("settings.catalog_updated_hours", {
        values: { count: Math.floor(ageSecs / 3600) },
      });
    const days = Math.floor(ageSecs / 86400);
    return days === 1
      ? $_("settings.catalog_updated_one_day")
      : $_("settings.catalog_updated_days", { values: { count: days } });
  }

  function stageLabel(stage: string): string {
    switch (stage) {
      case "downloading":
        return $_("settings.catalog_stage_downloading");
      case "parsing":
        return $_("settings.catalog_stage_parsing");
      case "saving":
        return $_("settings.catalog_stage_saving");
      case "done":
        return $_("settings.catalog_stage_done");
      default:
        return $_("settings.catalog_stage_working");
    }
  }

  async function toggle(field: keyof api.Prefs, value: boolean) {
    if (!$prefs) return;
    saving = field;
    try {
      if (field === "autostart") {
        const actual = await api.setAutostart(value);
        if (actual !== value) {
          toastError($_("settings.autostart_rejected"));
        }
        await hydratePrefs();
      } else {
        await updatePrefs({ [field]: value });
      }
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      saving = null;
    }
  }

  async function handleForgetServer() {
    signingOut = true;
    try {
      await signOut();
      forgetModalOpen = false;
      toastSuccess($_("settings.forgotten_toast"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      signingOut = false;
    }
  }

  type Row = {
    field: keyof api.Prefs;
    label: string;
    description: string;
    icon: any;
  };

  // Rows are derived so they re-render when the active locale changes. Using
  // `$derived` instead of plain `const` keeps the labels reactive without
  // forcing a remount of the Settings page on language switch.
  const generalRows: Row[] = $derived([
    {
      field: "close_to_tray",
      label: $_("settings.close_to_tray_label"),
      description: $_("settings.close_to_tray_desc"),
      icon: Minimize2,
    },
    // Auto-restore originally lived in its own "Sync" section, but the user
    // pointed out it's a one-toggle section masquerading as a category —
    // promoted into General alongside close-to-tray so all the "how Hoard
    // behaves day to day" switches live in one card.
    {
      field: "auto_restore",
      label: $_("settings.auto_restore_label"),
      description: $_("settings.auto_restore_desc"),
      icon: DownloadCloud,
    },
  ]);

  const startupRows: Row[] = $derived([
    {
      field: "autostart",
      label: $_("settings.autostart_label"),
      description: $_("settings.autostart_desc"),
      icon: LogIn,
    },
    {
      field: "start_minimised",
      label: $_("settings.start_minimised_label"),
      description: $_("settings.start_minimised_desc"),
      icon: Power,
    },
  ]);

  const privacyRows: Row[] = $derived([
    {
      field: "anonymous_telemetry",
      label: $_("settings.telemetry_label"),
      description: $_("settings.telemetry_desc"),
      icon: BarChart3,
    },
  ]);

  const notifyRows: Row[] = $derived([
    {
      field: "notify_on_success",
      label: $_("settings.notify_success_label"),
      description: $_("settings.notify_success_desc"),
      icon: BellRing,
    },
    {
      field: "notify_on_failure",
      label: $_("settings.notify_failure_label"),
      description: $_("settings.notify_failure_desc"),
      icon: BellOff,
    },
  ]);

  // Cloud-only toggle: "Modo ahorro" defaults every new cloud upload to
  // `backup_only` so it stays a one-way push from this device. Per-save
  // overrides live on the Library card.
  const cloudRows: Row[] = $derived([
    {
      field: "cloud_savings_mode",
      label: $_("settings.cloud_savings_mode_label"),
      description: $_("settings.cloud_savings_mode_desc"),
      icon: CloudOff,
    },
    {
      field: "live_activity_visible",
      label: $_("settings.live_activity_label"),
      description: $_("settings.live_activity_desc"),
      icon: Activity,
    },
  ]);

  /** Format the cloud-poll slider value as "10 s" / "2 min" so the user
   *  reads the right unit. The Rust side accepts 5..=300. */
  function formatCloudPoll(secs: number): string {
    if (secs < 60) return $_("settings.cloud_poll_value_secs", { values: { secs } });
    const minutes = Math.round(secs / 60);
    return $_("settings.cloud_poll_value_minutes", { values: { minutes } });
  }

  async function handleLanguageChange(e: Event) {
    const next = (e.currentTarget as HTMLSelectElement).value;
    try {
      await setLocale(next);
      // `setLocale` already persists to prefs; refresh the local store so the
      // value sticks if the page is remounted.
      await hydratePrefs();
    } catch (err) {
      toastError(typeof err === "string" ? err : (err as Error).message);
    }
  }
</script>

<div class="mx-auto max-w-3xl px-8 py-8">
  <header class="mb-6">
    <h1 class="font-display text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50">
      {$_("settings.title")}
    </h1>
    <p class="mt-2 text-sm text-zinc-400">{$_("settings.subtitle")}</p>
  </header>

  {#if !$prefs}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else}
    <div class="space-y-6">
      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_general")}
        </h2>
        <Card>
          <div class="divide-y divide-white/[0.06]">
            {#each generalRows as row (row.field)}
              <SettingsRow
                {row}
                value={$prefs[row.field] as boolean}
                disabled={saving === row.field}
                onChange={(v) => toggle(row.field, v)}
              />
            {/each}
          </div>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_account")}
        </h2>
        <Card>
          <button
            type="button"
            onclick={() => push("/account")}
            class="-mx-2 -my-1 flex w-[calc(100%+1rem)] items-center justify-between gap-3 rounded-md px-2 py-1 text-left transition-colors hover:bg-zinc-800/40"
          >
            <div class="flex min-w-0 flex-1 items-start gap-3">
              <Server size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("settings.account_label")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {#if $cloud.account}
                    {$cloud.account.email} · {$_("settings.account_plan", {
                      values: { plan: planLabel($cloud.account.plan) },
                    })}
                  {:else}
                    {$_("settings.account_desc")}
                  {/if}
                </p>
              </div>
            </div>
            <ChevronRight size={16} class="shrink-0 text-zinc-500" />
          </button>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_language")}
        </h2>
        <Card>
          <div class="flex items-start justify-between gap-4">
            <div class="flex min-w-0 flex-1 items-start gap-3">
              <Languages size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("settings.language_label")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {$_("settings.language_desc")}
                </p>
              </div>
            </div>
            <select
              value={$locale ?? "en"}
              onchange={handleLanguageChange}
              class="shrink-0 rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-100 outline-none transition-colors hover:border-zinc-600 focus:border-emerald-500"
              aria-label={$_("settings.language_label")}
            >
              {#each supportedLocales as loc (loc.code)}
                <option value={loc.code}>{loc.label}</option>
              {/each}
            </select>
          </div>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_startup")}
        </h2>
        <Card>
          <div class="divide-y divide-white/[0.06]">
            {#each startupRows as row (row.field)}
              <SettingsRow
                {row}
                value={$prefs[row.field] as boolean}
                disabled={saving === row.field}
                onChange={(v) => toggle(row.field, v)}
              />
            {/each}
          </div>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_notifications")}
        </h2>
        <Card>
          <div class="divide-y divide-white/[0.06]">
            {#each notifyRows as row (row.field)}
              <SettingsRow
                {row}
                value={$prefs[row.field] as boolean}
                disabled={saving === row.field}
                onChange={(v) => toggle(row.field, v)}
              />
            {/each}
          </div>
        </Card>
      </section>

      <!--
        Cloud "modo ahorro": single toggle that flips the default for new
        cloud uploads to `backup_only`. Visible only when the user is signed
        in to Hoard Cloud — for self-hosted users the field is still in
        prefs.json but has no observable effect, so we hide the row to keep
        the page focused.
      -->
      {#if $cloud.account}
        <section>
          <h2
            class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
          >
            {$_("settings.section_cloud")}
          </h2>
          <Card>
            <div class="divide-y divide-white/[0.06]">
              {#each cloudRows as row (row.field)}
                <SettingsRow
                  {row}
                  value={$prefs[row.field] as boolean}
                  disabled={saving === row.field}
                  onChange={(v) => toggle(row.field, v)}
                />
              {/each}
              <div class="pt-4">
                <div class="flex items-baseline justify-between gap-3">
                  <label
                    for="cloud-poll-slider"
                    class="text-sm font-medium text-zinc-100"
                  >
                    {$_("settings.cloud_poll_label")}
                  </label>
                  <span
                    class="font-mono text-xs text-zinc-400"
                    aria-live="polite"
                  >
                    {formatCloudPoll(cloudPollSecs)}
                  </span>
                </div>
                <input
                  id="cloud-poll-slider"
                  type="range"
                  min={cloudPollMin}
                  max="300"
                  step="1"
                  bind:value={cloudPollSecs}
                  onchange={commitCloudPoll}
                  disabled={saving === "cloud_poll_interval_secs"}
                  class="mt-2 w-full accent-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
                />
                <p class="mt-1 text-xs text-zinc-500">
                  {$_("settings.cloud_poll_hint")}
                </p>
              </div>
            </div>
          </Card>
        </section>
      {/if}

      <!--
        Modo Automático: dos sliders que mapean a los campos persistidos
        `automatic_scan_interval_hours` (1..=24) y `conflict_retention_days`
        (1..=30). El primero sólo está activo cuando `automatic_mode` está
        encendido — el sidebar es el lugar canónico para activarlo, así que
        aquí no replicamos el toggle. El backend reinicia el scheduler
        automáticamente al persistir el nuevo intervalo si el modo está
        activo.
      -->
      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.automatic_section_title")}
        </h2>
        <Card>
          <div class="space-y-6">
            <div>
              <div class="flex items-baseline justify-between gap-3">
                <label
                  for="scan-interval-slider"
                  class="text-sm font-medium text-zinc-100"
                >
                  {$_("settings.scan_interval_label")}
                </label>
                <span
                  class="font-mono text-xs text-zinc-400"
                  aria-live="polite"
                >
                  {$_("settings.scan_interval_value", {
                    values: { hours: scanIntervalHours },
                  })}
                </span>
              </div>
              <input
                id="scan-interval-slider"
                type="range"
                min="1"
                max="24"
                step="1"
                bind:value={scanIntervalHours}
                onchange={commitScanInterval}
                disabled={!$prefs.automatic_mode || saving === "automatic_scan_interval_hours"}
                class="mt-2 w-full accent-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
              />
              <p class="mt-1 text-xs text-zinc-500">
                {$_("settings.scan_interval_hint")}
              </p>
            </div>

            <div>
              <div class="flex items-baseline justify-between gap-3">
                <label
                  for="conflict-retention-slider"
                  class="text-sm font-medium text-zinc-100"
                >
                  {$_("settings.conflict_retention_label")}
                </label>
                <span
                  class="font-mono text-xs text-zinc-400"
                  aria-live="polite"
                >
                  {$_("settings.conflict_retention_value", {
                    values: { days: conflictRetentionDays },
                  })}
                </span>
              </div>
              <input
                id="conflict-retention-slider"
                type="range"
                min="1"
                max="30"
                step="1"
                bind:value={conflictRetentionDays}
                onchange={commitConflictRetention}
                disabled={saving === "conflict_retention_days"}
                class="mt-2 w-full accent-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
              />
              <p class="mt-1 text-xs text-zinc-500">
                {$_("settings.conflict_retention_hint")}
              </p>
            </div>
          </div>
        </Card>
      </section>

      <!--
        Ahorro de datos (ADR 0018): un único knob `data_saving ∈ [0,1]` que
        escala dos ejes — el intervalo mínimo entre snapshots del cliente
        (5s..10min) y la política de retención GFS del server. Se persiste
        como 0..1; el slider trabaja en 0..100 por comodidad. Surte efecto
        al reiniciar el agente (logout/login o reinicio de la app).
      -->
      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.data_saving_section_title")}
        </h2>
        <Card>
          <div>
            <div class="flex items-baseline justify-between gap-3">
              <label
                for="data-saving-slider"
                class="text-sm font-medium text-zinc-100"
              >
                {$_("settings.data_saving_label")}
              </label>
              <span class="font-mono text-xs text-zinc-400" aria-live="polite">
                {$_("settings.data_saving_value", {
                  values: { pct: dataSavingPct },
                })}
              </span>
            </div>
            <input
              id="data-saving-slider"
              type="range"
              min="0"
              max="100"
              step="5"
              bind:value={dataSavingPct}
              onchange={commitDataSaving}
              disabled={saving === "data_saving"}
              class="mt-2 w-full accent-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
            />
            <div
              class="mt-1 flex justify-between text-xs text-zinc-500"
            >
              <span>{$_("settings.data_saving_min")}</span>
              <span>{$_("settings.data_saving_max")}</span>
            </div>
            <p class="mt-2 text-xs text-zinc-500">
              {$_("settings.data_saving_hint")}
            </p>
          </div>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_privacy")}
        </h2>
        <Card>
          <div class="divide-y divide-white/[0.06]">
            {#each privacyRows as row (row.field)}
              <SettingsRow
                {row}
                value={$prefs[row.field] as boolean}
                disabled={saving === row.field}
                onChange={(v) => toggle(row.field, v)}
              />
            {/each}
          </div>
        </Card>
      </section>

      <!-- Self-hosted session card. Hidden entirely for cloud-only users:
           showing "Sin sesión" next to a dead sign-out button was confusing
           ("habla de servidor local, pero yo tengo la nube"). The cloud
           account lives in its own section above. -->
      {#if $auth.user}
        <section>
          <h2
            class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
          >
            {$_("settings.section_selfhost")}
          </h2>
          <Card>
            <div class="flex items-start justify-between gap-4">
              <div class="min-w-0 flex-1">
                <p class="text-sm text-zinc-100">
                  {$_("settings.signed_in_as")}
                  <span class="font-medium">{$auth.user.username}</span>
                </p>
                <p class="mt-1 truncate text-xs text-zinc-500">
                  {$auth.user.server_url}
                </p>
              </div>
              <Button
                variant="danger"
                onclick={() => (forgetModalOpen = true)}
                loading={signingOut}
              >
                <LogOut size={14} />
                {$_("settings.forget_server")}
              </Button>
            </div>
            <p class="mt-3 text-xs leading-relaxed text-zinc-500">
              {$_("settings.forget_server_desc")}
            </p>
          </Card>
        </section>
      {/if}

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_catalog")}
        </h2>
        <Card>
          <div class="flex items-start justify-between gap-4">
            <div class="flex min-w-0 flex-1 items-start gap-3">
              <Database size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm text-zinc-100">
                  {#if catalog}
                    {$_("settings.catalog_games", {
                      values: { count: catalog.games.toLocaleString() },
                    })}
                    <span class="text-zinc-500">·</span>
                    <span class="text-zinc-400">
                      {catalog.has_runtime_override
                        ? formatRelative(catalog.updated_at)
                        : $_("settings.catalog_using_bundled")}
                    </span>
                  {:else}
                    {$_("common.loading")}
                  {/if}
                </p>
                <p class="mt-1 text-xs text-zinc-500">
                  {$_("settings.catalog_desc")}
                </p>
                {#if updatingCatalog}
                  <p class="mt-2 text-xs text-zinc-400">
                    {stageLabel(catalogStage)}
                  </p>
                {/if}
              </div>
            </div>
            <Button
              variant="ghost"
              onclick={handleCatalogUpdate}
              loading={updatingCatalog}
              disabled={updatingCatalog}
            >
              <RefreshCw size={14} />
              {$_("settings.catalog_check")}
            </Button>
          </div>
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.ignored_section_title")}
        </h2>
        <Card>
          {#if ignored.length === 0}
            <p class="py-1 text-sm text-zinc-500">
              {$_("settings.ignored_empty")}
            </p>
          {:else}
            <ul class="divide-y divide-white/[0.06]">
              {#each ignored as slug (slug)}
                <li
                  class="flex items-center justify-between gap-3 py-3 first:pt-0 last:pb-0"
                >
                  <span
                    class="truncate font-mono text-sm text-zinc-200"
                    title={slug}
                  >
                    {slug}
                  </span>
                  <Button
                    variant="ghost"
                    onclick={() => reactivateIgnored(slug)}
                    loading={ignoredBusy === slug}
                    disabled={ignoredBusy === slug}
                  >
                    {$_("settings.ignored_reactivate")}
                  </Button>
                </li>
              {/each}
            </ul>
          {/if}
        </Card>
      </section>

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_advanced")}
        </h2>
        <Card>
          <div class="divide-y divide-white/[0.06]">
            <button
              type="button"
              onclick={() => push("/logs")}
              class="-mx-6 -mt-6 flex w-[calc(100%+3rem)] items-center justify-between gap-4 px-6 py-5 text-left transition-colors hover:bg-zinc-900/60"
            >
              <div class="flex items-start gap-3">
                <FileText size={16} class="mt-0.5 shrink-0 text-zinc-500" />
                <div>
                  <p class="text-sm font-medium text-zinc-100">
                    {$_("settings.view_logs_title")}
                  </p>
                  <p class="mt-0.5 text-xs text-zinc-500">
                    {$_("settings.view_logs_desc")}
                  </p>
                </div>
              </div>
              <ChevronRight size={16} class="shrink-0 text-zinc-500" />
            </button>

            {#if showServerCard}
              <!--
                Self-hosted server panel. Only renders when the user signed in
                against a private-network server (RFC1918 / localhost / .local);
                a future cloud-hosted Hoard won't expose this subsection because
                the upgrade is handled server-side.
              -->
              <div class="-mx-6 px-6 pb-2 pt-6">
                <div class="flex items-start gap-3">
                  <Server size={16} class="mt-0.5 shrink-0 text-zinc-500" />
                  <div class="min-w-0 flex-1">
                    <p class="text-sm font-medium text-zinc-100">
                      {$_("settings.server_section_title")}
                    </p>
                    <p class="mt-0.5 text-xs text-zinc-500">
                      {$_("settings.server_section_desc")}
                    </p>

                    <dl
                      class="mt-3 grid grid-cols-[max-content,1fr] gap-x-3 gap-y-1 text-xs"
                    >
                      <dt class="text-zinc-500">
                        {$_("settings.server_url_label")}
                      </dt>
                      <dd class="truncate font-mono text-zinc-300">
                        {$auth.user?.server_url ?? "—"}
                      </dd>
                      <dt class="text-zinc-500">
                        {$_("settings.server_version_label")}
                      </dt>
                      <dd class="text-zinc-300">
                        {#if serverUpdate}
                          v{serverUpdate.current}
                          {#if serverUpdate.available && serverUpdate.latest}
                            <span class="text-amber-300">
                              → v{serverUpdate.latest}
                            </span>
                          {/if}
                        {:else}
                          {$_("common.loading")}
                        {/if}
                      </dd>
                    </dl>

                    {#if serverUpdate?.available}
                      <p class="mt-3 text-xs text-amber-300">
                        {$_("settings.server_update_available", {
                          values: { latest: serverUpdate.latest ?? "?" },
                        })}
                      </p>
                      {#if canInAppUpgrade}
                        <!--
                          Admin: trigger the upgrade remotely over HTTP. The
                          server upgrades itself and restarts; we show the
                          shell command underneath as a subtle hint for users
                          who'd rather run it on the box themselves.
                        -->
                        <p class="mt-2 text-xs text-zinc-500">
                          {$_("settings.server_remote_upgrade_hint")}
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                          <Button
                            variant="primary"
                            onclick={handleServerRemoteUpgrade}
                            loading={upgradingServer}
                            disabled={upgradingServer}
                          >
                            <ServerCog size={14} />
                            {$_("settings.server_upgrade_button")}
                          </Button>
                          <Button
                            variant="ghost"
                            onclick={handleServerRefresh}
                            loading={refreshingServer}
                            disabled={refreshingServer || upgradingServer}
                          >
                            <RefreshCw size={14} />
                            {$_("settings.server_recheck")}
                          </Button>
                        </div>
                      {:else}
                        <pre
                          class="mt-2 overflow-x-auto rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-emerald-300">sudo hoard-server upgrade</pre>
                        <div class="mt-3 flex flex-wrap gap-2">
                          <Button
                            variant="primary"
                            onclick={handleServerCopyCommand}
                            loading={copyingUpgrade}
                            disabled={copyingUpgrade}
                          >
                            <ServerCog size={14} />
                            {$_("settings.server_copy_command")}
                          </Button>
                          <Button
                            variant="ghost"
                            onclick={handleServerRefresh}
                            loading={refreshingServer}
                            disabled={refreshingServer}
                          >
                            <RefreshCw size={14} />
                            {$_("settings.server_recheck")}
                          </Button>
                        </div>
                      {/if}
                    {:else if serverUpdate && !serverUpdate.error}
                      <p class="mt-3 text-xs text-emerald-300">
                        {$_("settings.server_up_to_date")}
                      </p>
                      <div class="mt-3">
                        <Button
                          variant="ghost"
                          onclick={handleServerRefresh}
                          loading={refreshingServer}
                          disabled={refreshingServer}
                        >
                          <RefreshCw size={14} />
                          {$_("settings.server_recheck")}
                        </Button>
                      </div>
                    {:else if serverUpdate?.error}
                      <p class="mt-3 text-xs text-red-400">
                        {$_("settings.server_probe_failed")}
                      </p>
                      <div class="mt-3">
                        <Button
                          variant="ghost"
                          onclick={handleServerRefresh}
                          loading={refreshingServer}
                          disabled={refreshingServer}
                        >
                          <RefreshCw size={14} />
                          {$_("settings.server_recheck")}
                        </Button>
                      </div>
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </Card>
      </section>

      {#if diagnosticsUnlocked}
        <section>
          <h2
            class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
          >
            {$_("diagnostics.section_title")}
          </h2>

          <Card>
            <button
              type="button"
              onclick={() => push("/diagnostics")}
              class="-m-4 flex w-[calc(100%+2rem)] items-center justify-between gap-3 rounded-lg p-4 text-left transition-colors hover:bg-zinc-900/60"
            >
              <span class="flex items-start gap-3">
                <Activity size={16} class="mt-0.5 shrink-0 text-zinc-500" />
                <span class="min-w-0">
                  <span class="block text-sm font-medium text-zinc-100">
                    {$_("diagnostics.title")}
                  </span>
                  <span class="mt-0.5 block text-xs text-zinc-500">
                    {$_("diagnostics.no_trace")}
                  </span>
                </span>
              </span>
              <ChevronRight size={16} class="shrink-0 text-zinc-500" />
            </button>
          </Card>

          <Card>
            <div class="flex items-start gap-3">
              <Activity size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("diagnostics.heading")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {$_("diagnostics.subtitle")}
                </p>

                {#if agentSlots.length === 0}
                  <p class="mt-4 text-xs text-zinc-500">
                    {$_("diagnostics.agent_stopped")}
                  </p>
                {:else}
                  <ul class="mt-4 space-y-3">
                    {#each agentSlots as slot (slot.save_id)}
                      <li
                        class="rounded-md border border-zinc-800 bg-zinc-900/40 p-3 text-xs"
                      >
                        <div class="flex items-center justify-between gap-2">
                          <span class="truncate font-medium text-zinc-100">
                            {slot.display_name}
                          </span>
                          <span class="flex shrink-0 items-center gap-1.5">
                            {#if slot.watcher_armed}
                              <span
                                class="rounded border border-emerald-500/40 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-300"
                                >{$_("diagnostics.watcher_armed")}</span
                              >
                            {:else}
                              <span
                                class="rounded border border-amber-500/40 bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-medium text-amber-300"
                                >{$_("diagnostics.watcher_off")}</span
                              >
                            {/if}
                            {#if slot.process_running}
                              <span
                                class="rounded border border-sky-500/40 bg-sky-500/10 px-1.5 py-0.5 text-[10px] font-medium text-sky-300"
                                >{$_("diagnostics.process_running")}</span
                              >
                            {/if}
                          </span>
                        </div>
                        <p class="mt-1 truncate text-[11px] text-zinc-500">
                          {slot.path}
                        </p>
                        <dl
                          class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-[11px]"
                        >
                          <dt class="text-zinc-500">
                            {$_("diagnostics.last_fs_event")}
                          </dt>
                          <dd class="text-zinc-300">
                            {formatTimestamp(slot.last_fs_event_at)}
                          </dd>
                          <dt class="text-zinc-500">
                            {$_("diagnostics.next_backup")}
                          </dt>
                          <dd class="text-zinc-300">
                            {formatTimestamp(slot.next_scheduled_backup_at)}
                          </dd>
                        </dl>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            </div>
          </Card>
        </section>
      {/if}

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_about")}
        </h2>
        <Card>
          <div class="flex items-start gap-3 text-sm text-zinc-300">
            <Info size={16} class="mt-0.5 shrink-0 text-zinc-500" />
            <div>
              <p>
                {$_("settings.about_line_1", {
                  values: { version: APP_VERSION },
                })}
              </p>
              <p class="mt-1 text-xs text-zinc-500">
                {$_("settings.about_line_2")}
              </p>
            </div>
          </div>
        </Card>
      </section>
    </div>
  {/if}
</div>

<Modal
  open={forgetModalOpen}
  title={$_("settings.forget_confirm_title")}
  dismissible={!signingOut}
  onClose={() => (forgetModalOpen = false)}
>
  <p class="text-sm leading-relaxed text-zinc-300">
    {$_("settings.forget_confirm_body", {
      values: { url: $auth.user?.server_url ?? "—" },
    })}
  </p>
  {#snippet footer()}
    <Button
      variant="secondary"
      onclick={() => (forgetModalOpen = false)}
      disabled={signingOut}
    >
      {$_("common.cancel")}
    </Button>
    <Button variant="danger" onclick={handleForgetServer} loading={signingOut}>
      <LogOut size={14} />
      {$_("settings.forget_confirm_cta")}
    </Button>
  {/snippet}
</Modal>

<!--
  The actual row UI lives in `lib/components/SettingsRow.svelte`. Keeping it
  as its own component makes it reusable from future Settings sub-pages and
  keeps this file focused on the page composition.
-->
