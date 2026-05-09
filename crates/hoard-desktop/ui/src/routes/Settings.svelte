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
  } from "lucide-svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import SettingsRow from "../lib/components/SettingsRow.svelte";
  import { prefs, hydratePrefs, updatePrefs } from "../lib/stores/prefs";
  import { auth, signOut } from "../lib/stores/auth";
  import { supportedLocales, setLocale } from "../lib/i18n";
  import * as api from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

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
  });

  onDestroy(() => {
    unlistenCatalog?.();
  });

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
              class="shrink-0 rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-100 outline-none transition-colors hover:border-zinc-600 focus:border-amber-500"
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
          <button
            type="button"
            onclick={() => push("/logs")}
            class="-m-6 flex w-[calc(100%+3rem)] items-center justify-between gap-4 rounded-xl px-6 py-5 text-left transition-colors hover:bg-zinc-900/60"
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
        </Card>
      </section>

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
