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
  } from "lucide-svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import SettingsRow from "../lib/components/SettingsRow.svelte";
  import { prefs, hydratePrefs, updatePrefs } from "../lib/stores/prefs";
  import { auth, signOut } from "../lib/stores/auth";
  import { supportedLocales, setLocale } from "../lib/i18n";
  import * as api from "../lib/api";
  import { toastError, toastInfo, toastSuccess } from "../lib/stores/toasts";
  import { checkForUpdates, lastReport } from "../lib/stores/updates";

  let saving = $state<string | null>(null);
  let signingOut = $state(false);

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

    diagnosticsUnlocked =
      sessionStorage.getItem("hoard-diagnostics-unlocked") === "1";
    if (diagnosticsUnlocked) {
      await refreshAgentSlots();
      diagnosticsTimer = setInterval(refreshAgentSlots, 2000);
    }
  });

  onDestroy(() => {
    unlistenCatalog?.();
    if (diagnosticsTimer) clearInterval(diagnosticsTimer);
  });

  // Self-hosted server panel — only renders when the user is signed into a
  // local network server (`is_local_server`) rather than a future
  // cloud-hosted Hoard. We pull the version info from the same
  // `lastReport.server` the sidebar amber badge consumes, so we don't have to
  // refetch on mount.
  const serverUpdate = $derived($lastReport?.server ?? null);
  const showServerCard = $derived($auth.user?.is_local_server === true);
  let refreshingServer = $state(false);
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

  async function handleServerUpgrade() {
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

  async function handleSignOut() {
    signingOut = true;
    try {
      await signOut();
      toastSuccess($_("settings.signed_out_toast"));
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
    <h1 class="text-2xl font-semibold tracking-tight">
      {$_("settings.title")}
    </h1>
    <p class="mt-1 text-sm text-zinc-400">{$_("settings.subtitle")}</p>
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
          <div class="divide-y divide-zinc-800">
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
          <div class="divide-y divide-zinc-800">
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
          <div class="divide-y divide-zinc-800">
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

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_privacy")}
        </h2>
        <Card>
          <div class="divide-y divide-zinc-800">
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

      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_account")}
        </h2>
        <Card>
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex-1">
              <p class="text-sm text-zinc-100">
                {#if $auth.user}
                  {$_("settings.signed_in_as")}
                  <span class="font-medium">{$auth.user.username}</span>
                {:else}
                  {$_("settings.not_signed_in")}
                {/if}
              </p>
              <p class="mt-1 truncate text-xs text-zinc-500">
                {$auth.user?.server_url ?? ""}
              </p>
            </div>
            <Button
              variant="ghost"
              onclick={handleSignOut}
              loading={signingOut}
              disabled={!$auth.user}
            >
              <LogOut size={14} />
              {$_("settings.sign_out")}
            </Button>
          </div>
        </Card>
      </section>

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
          {$_("settings.section_advanced")}
        </h2>
        <Card>
          <div class="divide-y divide-zinc-800">
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
                      <pre
                        class="mt-2 overflow-x-auto rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-emerald-300">sudo hoard-server upgrade</pre>
                      <div class="mt-3 flex flex-wrap gap-2">
                        <Button
                          variant="primary"
                          onclick={handleServerUpgrade}
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
              <p>{$_("settings.about_line_1")}</p>
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

<!--
  The actual row UI lives in `lib/components/SettingsRow.svelte`. Keeping it
  as its own component makes it reusable from future Settings sub-pages and
  keeps this file focused on the page composition.
-->
