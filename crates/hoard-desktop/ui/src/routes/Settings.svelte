<script lang="ts">
  /**
   * Settings, every persistent app preference lives here.
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
    Clock,
    RefreshCw,
    Database,
    Languages,
    Palette,
    Activity,
    DownloadCloud,
    HardDrive,
    Server,
    ServerCog,
    MousePointer2,
    Gamepad2,
    Sparkles,
    ZoomIn,
  } from "@lucide/svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import SettingsRow from "../lib/components/SettingsRow.svelte";
  import { prefs, hydratePrefs, updatePrefs } from "../lib/stores/prefs";
  import {
    theme,
    themes,
    type ThemeId,
    accentHue,
    setAccentHue,
    gems,
    gemSwatch,
    gemFor,
  } from "../lib/stores/theme";
  import {
    atmosphere,
    atmospheres,
    setAtmosphere,
    type AtmosphereId,
  } from "../lib/stores/atmosphere";
  import {
    uiScale,
    setUiScale,
    resetUiScale,
    MIN_SCALE,
    MAX_SCALE,
  } from "../lib/stores/uiScale";
  import {
    motionIntensity,
    setMotionIntensity,
  } from "../lib/stores/motion";
  import {
    overlayEnabled,
    overlayHotkey,
    setOverlayEnabled,
    setOverlayHotkey,
    DEFAULT_HOTKEY,
  } from "../lib/stores/gameOverlay";
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
  import { clearOnboarding, clearTourSeen } from "../lib/stores/onboarding";

  let saving = $state<string | null>(null);
  let signingOut = $state(false);
  // Gate the "forget server" action behind a confirm modal. Forgetting wipes
  // the saved address + token (session.toml + keyring), which is what stops
  // the app reconnecting to a dead/abandoned self-hosted box on every launch.
  let forgetModalOpen = $state(false);

  // Theme picker swatch previews. Each is a representative bg + accent dot so
  // the user can tell the palettes apart without applying each one. "Auto"
  // paints half-dark / half-light to signal it follows the OS scheme.
  const swatchBg: Record<ThemeId, string> = {
    obsidian: "linear-gradient(135deg, #0e1210, #141a17)",
    quartz: "linear-gradient(135deg, #f3f0ea, #e7e3d9)",
    auto: "linear-gradient(135deg, #0e1210 0 50%, #f3f0ea 50% 100%)",
  };
  const swatchAccent: Record<ThemeId, string> = {
    obsidian: "#34d399",
    quartz: "#10b981",
    auto: "#34d399",
  };

  // Accent picker: repoints the "gem" hue live via CSS variables on <html>,
  // compositing on top of whichever theme is active.
  function onAccentInput(e: Event): void {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    setAccentHue(Number.isFinite(v) ? v : null);
  }
  function resetAccent(): void {
    setAccentHue(null);
  }

  // The named gems. `gemFor` returns null for a hue that matches no preset,
  // which is exactly when the custom slider should already be open, otherwise
  // someone who picked 187 degrees last week reopens Settings and finds no
  // gem selected and no slider to explain why.
  let customOpen = $state(gemFor($accentHue) == null);
  const selectedGem = $derived(gemFor($accentHue));

  /** The mark's gradient for a given hue, as an inline `background`, same
   *  maths the logo uses, so a swatch previews the real thing. */
  function gemStyle(hue: number | null): string {
    const { from, to } = gemSwatch(hue);
    return `background: linear-gradient(140deg, ${from}, ${to});`;
  }

  // Background atmosphere. Small inline previews rather than words alone:
  // "vignette" means nothing until you have seen one.
  const atmosPreview: Record<AtmosphereId, string> = {
    grain:
      "background: #0a0a0a; background-image: radial-gradient(oklch(1 0 0 / 0.14) 0.5px, transparent 0.5px); background-size: 3px 3px;",
    flat: "background: #0a0a0a;",
    glow: "background: radial-gradient(120% 80% at 50% -30%, color-mix(in oklch, var(--color-accent) 55%, transparent), #0a0a0a 70%);",
    vignette:
      "background: radial-gradient(90% 90% at 50% 45%, oklch(0.32 0.01 165), #050505 100%);",
  };

  // Interface scale. Engine zoom, so `onchange` would feel laggy, `oninput`
  // zooms while dragging, and the slider rides its own zoom, which is odd for
  // a second and then obviously right.
  function onScaleInput(e: Event): void {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) setUiScale(v / 100);
  }

  // The tilt's intensity: 0 turns it off, 100 is the historic 8 degrees, and 50,
  // the default, is half. On `oninput` (not `onchange`) so it is visible while you
  // drag.
  function onMotionInput(e: Event): void {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) setMotionIntensity(v);
  }

  // ---- capturing the overlay's shortcut
  //
  // It listens in the capture phase so the combination activates nothing on the
  // page while it is being assigned. It is only accepted with at least one
  // modifier: a global single-key shortcut would eat that key in EVERY
  // application, which is a very fast way to break somebody's keyboard.
  let capturingHotkey = $state(false);

  const MODS: [string, string][] = [
    ["ctrlKey", "Ctrl"],
    ["altKey", "Alt"],
    ["shiftKey", "Shift"],
    ["metaKey", "Super"],
  ];

  function accelFrom(e: KeyboardEvent): string | null {
    const parts = MODS.filter(([k]) => e[k as keyof KeyboardEvent]).map(
      ([, name]) => name,
    );
    if (parts.length === 0) return null;
    const code = e.code;
    let key: string | null = null;
    if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
    else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
    else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) key = code;
    else if (code === "Space") key = "Space";
    if (!key) return null;
    return [...parts, key].join("+");
  }

  function onHotkeyDown(e: KeyboardEvent) {
    if (!capturingHotkey) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape") {
      capturingHotkey = false;
      return;
    }
    const accel = accelFrom(e);
    if (!accel) return; // aún sólo modificadores: se sigue esperando
    capturingHotkey = false;
    setOverlayHotkey(accel);
  }

  $effect(() => {
    if (!capturingHotkey) return;
    window.addEventListener("keydown", onHotkeyDown, true);
    return () => window.removeEventListener("keydown", onHotkeyDown, true);
  });

  // The single user-facing operating mode, derived from the internal flags.
  const syncMode = $derived(
    $prefs ? api.syncModeOf($prefs) : "backup_only",
  );

  async function commitSyncMode(mode: api.SyncMode) {
    if (!$prefs || mode === syncMode) return;
    saving = "sync_mode";
    try {
      const updated = await api.setSyncMode(mode);
      prefs.set(updated);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
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

  // Why the sync *service* isn't set to start at login, when it isn't. The
  // switch below registers two things, the app's launcher entry and the
  // service's own unit, and the second one can fail where the first can't:
  // an AppImage runs from a mount that's gone by the next login, and a machine
  // without systemd has nowhere to declare a unit. Saying so here is the whole
  // point: this used to be a line in the service's log, so the switch read "on"
  // while the sync only ever ran with the window open.
  let serviceAutostart = $state<api.ServiceAutostart | null>(null);

  async function refreshServiceAutostart() {
    try {
      serviceAutostart = await api.serviceAutostartState();
    } catch (e) {
      console.warn("serviceAutostartState failed:", e);
    }
  }

  /** The sentence for a login start that can't happen here. Only ever shown
   *  when the switch is on: with it off there's nothing to explain. */
  const serviceAutostartBlocked = $derived(
    serviceAutostart?.enabled && serviceAutostart?.unsupported
      ? serviceAutostart
      : null,
  );

  function serviceAutostartMessageKey(reason: api.ServiceAutostartBlock) {
    switch (reason) {
      case "no_stable_path":
        return "settings.service_autostart_no_stable_path";
      case "no_service_manager":
        return "settings.service_autostart_no_service_manager";
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

  // Hidden diagnostics panel, only visible after 5 clicks on the sidebar
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
    await refreshServiceAutostart();
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
  // another box. The only requirement is an admin token, the server rejects
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
      // The command fails with a structured AppError (i18n keys), surface it
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
        // The app's launcher entry and the sync service are two registrations,
        // and the second one can fail on its own (an AppImage with nowhere
        // stable to run from). `set_autostart` waits for it, so by now the
        // outcome is there to read.
        await refreshServiceAutostart();
      } else {
        await updatePrefs({ [field]: value });
      }
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      saving = null;
    }
  }

  /** The sync service's login start. Not a pref: it writes to the service
   *  manager, which is also where `hoard sync autostart` writes, so the answer
   *  is read back rather than assumed. */
  async function toggleServiceAutostart(value: boolean) {
    saving = "service_autostart" as keyof api.Prefs;
    try {
      serviceAutostart = await api.setServiceAutostart(value);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      // The switch must not be left showing what we asked for when the service
      // manager refused it.
      await refreshServiceAutostart();
    } finally {
      saving = null;
    }
  }

  async function handleForgetServer() {
    signingOut = true;
    try {
      await signOut();
      // Reset the wizard + tour so forgetting drops the user back on the
      // welcome flow (instead of leaving them sitting in Settings), and the
      // tour replays when they connect to a server again.
      await clearOnboarding();
      await clearTourSeen();
      forgetModalOpen = false;
      toastSuccess($_("settings.forgotten_toast"));
      push("/onboarding/language");
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

  // Two switches, not one: the diagnostics telemetry promises in its own text that
  // it never sends game names, and playtime is game names by construction. Turning
  // Wrapple's off sends nothing, not even a notice that it is off, and its
  // description says what that costs.
  const privacyRows: Row[] = $derived([
    {
      field: "anonymous_telemetry",
      label: $_("settings.telemetry_label"),
      description: $_("settings.telemetry_desc"),
      icon: BarChart3,
    },
    {
      field: "wrapple_telemetry",
      label: $_("settings.wrapple_telemetry_label"),
      description: $_("settings.wrapple_telemetry_desc"),
      icon: Clock,
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

  // Cloud-only toggles. The old "Modo ahorro (solo subida)"
  // (`cloud_savings_mode`) toggle was removed from the UI (2026-07-04): it
  // confused more than it helped and the pref was never actually consumed by
  // the agent. The pref field is kept as dead code for possible future use,
  // see `Prefs.cloud_savings_mode`.
  const cloudRows: Row[] = $derived([
    {
      field: "live_activity_visible",
      label: $_("settings.live_activity_label"),
      description: $_("settings.live_activity_desc"),
      icon: Activity,
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
    <h1 class="font-display text-[28px] leading-tight font-semibold tracking-[-0.02em] text-zinc-50">
      {$_("settings.title")}
    </h1>
    <p class="mt-2 text-sm text-zinc-400">{$_("settings.subtitle")}</p>
  </header>

  {#if !$prefs}
    <Card>
      <div class="shimmer py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else}
    <div class="space-y-6">
      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_mode")}
        </h2>
        <Card>
          <div class="space-y-2">
            {#each [{ mode: "backup_only", icon: HardDrive, title: $_("settings.mode_backup_title"), desc: $_("settings.mode_backup_desc") }, { mode: "full_sync", icon: DownloadCloud, title: $_("settings.mode_sync_title"), desc: $_("settings.mode_sync_desc") }] as opt (opt.mode)}
              <button
                type="button"
                disabled={saving === "sync_mode"}
                onclick={() => commitSyncMode(opt.mode as api.SyncMode)}
                class="flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors disabled:opacity-60 {syncMode ===
                opt.mode
                  ? 'border-emerald-500/60 bg-emerald-500/10'
                  : 'border-white/[0.08] hover:bg-zinc-800/40'}"
              >
                <span
                  class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md {syncMode ===
                  opt.mode
                    ? 'bg-emerald-500/20 text-emerald-400'
                    : 'bg-zinc-800 text-zinc-400'}"
                >
                  <opt.icon size={16} />
                </span>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-zinc-100"
                      >{opt.title}</span
                    >
                    {#if syncMode === opt.mode}
                      <span
                        class="rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-400"
                        >{$_("settings.mode_active_badge")}</span
                      >
                    {/if}
                  </div>
                  <p class="mt-0.5 text-xs text-zinc-400">{opt.desc}</p>
                </div>
              </button>
            {/each}
          </div>
        </Card>
      </section>

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
          {$_("settings.section_themes")}
        </h2>
        <Card>
          <div class="flex items-start gap-3 pb-4">
            <Palette size={16} class="mt-0.5 shrink-0 text-zinc-500" />
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-zinc-100">
                {$_("settings.themes_label")}
              </p>
              <p class="mt-0.5 text-xs text-zinc-500">
                {$_("settings.themes_desc")}
              </p>
            </div>
          </div>
          <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
            {#each themes as t (t.id)}
              {@const active = $theme === t.id}
              <button
                type="button"
                onclick={() => theme.set(t.id)}
                aria-pressed={active}
                class="group flex flex-col items-start gap-2 rounded-lg border p-2.5 text-left transition-colors {active
                  ? 'border-emerald-500/60 bg-emerald-500/10'
                  : 'border-white/[0.08] hover:bg-zinc-800/40'}"
              >
                <span
                  class="relative flex h-12 w-full items-center justify-center overflow-hidden rounded-md border border-white/[0.08]"
                  style="background: {swatchBg[t.id]};"
                >
                  <span
                    class="absolute inset-x-0 bottom-0 h-1.5"
                    style="background: {swatchAccent[t.id]};"
                  ></span>
                </span>
                <span class="min-w-0 w-full">
                  <span class="block text-sm font-medium text-zinc-100">
                    {$_(t.labelKey)}
                  </span>
                  {#if t.id === "auto"}
                    <span class="block text-[11px] text-zinc-500">
                      {$_("settings.theme_auto_hint")}
                    </span>
                  {/if}
                </span>
              </button>
            {/each}
          </div>
          <!-- Named gems. The hue wheel is still here, one click away, but it
               is no longer the only way in: nobody thinks "I want 265 degrees",
               they think "I want it blue". Emerald is the `null` hue — the
               theme's own gem — so picking it is what Reset used to be, and
               Quartz keeps its darker emerald instead of being overridden with
               a hue tuned for a black background. -->
          <div class="mt-4 border-t border-white/[0.08] pt-4">
            <div class="flex items-start gap-3">
              <Palette size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("settings.accent_label")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {$_("settings.accent_desc")}
                </p>
              </div>
              <button
                type="button"
                onclick={() => (customOpen = !customOpen)}
                aria-expanded={customOpen}
                class="shrink-0 rounded-md border px-2 py-1 text-xs transition-colors {customOpen
                  ? 'border-[var(--color-accent)]/40 bg-[var(--color-accent)]/10 text-zinc-100'
                  : 'border-white/[0.08] text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100'}"
              >
                {$_("settings.accent_custom")}
              </button>
            </div>
            <div class="mt-3 flex flex-wrap gap-2">
              {#each gems as g (g.id)}
                {@const active = selectedGem?.id === g.id}
                <button
                  type="button"
                  onclick={() => {
                    setAccentHue(g.hue);
                    customOpen = false;
                  }}
                  aria-pressed={active}
                  class="group flex w-20 flex-col items-center gap-1.5 rounded-lg border p-1.5 transition-colors {active
                    ? 'border-[var(--color-accent)]/50 bg-white/[0.04]'
                    : 'border-transparent hover:bg-white/[0.03]'}"
                >
                  <!-- The mark itself, not a coloured dot: same tile, same
                       gradient maths, so the swatch shows what the logo will
                       actually become. -->
                  <span
                    class="flex h-8 w-8 items-center justify-center rounded-lg bg-[#0a0a0a] ring-1 ring-inset ring-white/[0.08]"
                  >
                    <span
                      class="h-4 w-4 rounded-[3px]"
                      style={gemStyle(g.hue)}
                      aria-hidden="true"
                    ></span>
                  </span>
                  <span
                    class="w-full truncate text-center text-[10px] {active
                      ? 'text-zinc-100'
                      : 'text-zinc-500 group-hover:text-zinc-300'}"
                  >
                    {$_(g.labelKey)}
                  </span>
                </button>
              {/each}
            </div>
            {#if customOpen}
              <div class="mt-3 flex items-center gap-3">
                <input
                  type="range"
                  min="0"
                  max="359"
                  step="1"
                  value={$accentHue ?? 160}
                  oninput={onAccentInput}
                  class="hue-slider min-w-0 flex-1"
                  aria-label={$_("settings.accent_label")}
                />
                <button
                  type="button"
                  onclick={resetAccent}
                  class="shrink-0 rounded-md border border-white/[0.08] px-2 py-1 text-xs text-zinc-400 transition-colors hover:bg-zinc-800/40 hover:text-zinc-100"
                >
                  {$_("settings.accent_reset")}
                </button>
              </div>
            {/if}
          </div>

          <!-- Background atmosphere. The pure-black-plus-grain canvas was a
               good call for WOLED panels and a bad decree for everyone else;
               it stays the default and becomes a choice. Previews rather than
               words alone — "vignette" means nothing until you've seen one. -->
          <div class="mt-4 border-t border-white/[0.08] pt-4">
            <div class="flex items-start gap-3">
              <Sparkles size={16} class="mt-0.5 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("settings.atmos_label")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {$_("settings.atmos_desc")}
                </p>
              </div>
            </div>
            <div class="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-4">
              {#each atmospheres as a (a.id)}
                {@const active = $atmosphere === a.id}
                <button
                  type="button"
                  onclick={() => setAtmosphere(a.id)}
                  aria-pressed={active}
                  class="flex flex-col gap-1.5 rounded-lg border p-1.5 text-left transition-colors {active
                    ? 'border-[var(--color-accent)]/50 bg-white/[0.04]'
                    : 'border-white/[0.08] hover:bg-white/[0.03]'}"
                >
                  <span
                    class="block h-10 w-full rounded-md ring-1 ring-inset ring-white/[0.08]"
                    style={atmosPreview[a.id]}
                    aria-hidden="true"
                  ></span>
                  <span
                    class="truncate text-[11px] {active
                      ? 'text-zinc-100'
                      : 'text-zinc-500'}"
                  >
                    {$_(a.labelKey)}
                  </span>
                </button>
              {/each}
            </div>
          </div>

          <!-- Interface scale. This is the engine's own zoom, not a CSS trick:
               the app has ~130 hard-coded pixel sizes and icons sized by
               number, and none of those would move for a font-size hack. Also
               bound to Ctrl+wheel and Ctrl +/-/0 app-wide, which is the first
               thing anyone tries. -->
          <div class="mt-4 flex items-center gap-3 border-t border-white/[0.08] pt-4">
            <ZoomIn size={16} class="mt-0.5 shrink-0 text-zinc-500" />
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-zinc-100">
                {$_("settings.scale_label")}
              </p>
              <p class="mt-0.5 text-xs text-zinc-500">
                {$_("settings.scale_desc")}
              </p>
            </div>
            <input
              type="range"
              min={Math.round(MIN_SCALE * 100)}
              max={Math.round(MAX_SCALE * 100)}
              step="5"
              value={Math.round($uiScale * 100)}
              oninput={onScaleInput}
              class="motion-slider w-40 shrink-0"
              aria-label={$_("settings.scale_label")}
              aria-valuetext="{Math.round($uiScale * 100)}%"
            />
            <span class="w-10 shrink-0 text-right text-xs tabular-nums text-zinc-400">
              {Math.round($uiScale * 100)}%
            </span>
            <button
              type="button"
              onclick={resetUiScale}
              class="shrink-0 rounded-md border border-white/[0.08] px-2 py-1 text-xs text-zinc-400 transition-colors hover:bg-zinc-800/40 hover:text-zinc-100"
            >
              {$_("settings.scale_reset")}
            </button>
          </div>

          <!-- Intensidad del relieve. Va aquí, junto al tema y al acento,
               porque es lo mismo: aspecto puro, guardado en el navegador y sin
               pasar por Rust. Tres niveles en vez de un interruptor — apagarlo
               del todo era la única salida para quien lo encuentra excesivo, y
               se llevaba por delante un efecto que a otros les gusta. -->
          <div class="mt-4 flex items-center gap-3 border-t border-white/[0.08] pt-4">
            <MousePointer2 size={16} class="shrink-0 text-zinc-500" />
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-zinc-100">
                {$_("settings.motion_label")}
              </p>
              <p class="mt-0.5 text-xs text-zinc-500">
                {$_("settings.motion_desc")}
              </p>
            </div>
            <input
              type="range"
              min="0"
              max="100"
              step="5"
              value={$motionIntensity}
              oninput={onMotionInput}
              class="motion-slider w-40 shrink-0"
              aria-label={$_("settings.motion_label")}
              aria-valuetext="{$motionIntensity}%"
            />
            <span class="w-10 shrink-0 text-right text-xs tabular-nums text-zinc-400">
              {$motionIntensity}%
            </span>
          </div>
        </Card>
      </section>

      <!-- ── Overlay sobre el juego ─────────────────────────────────── -->
      <section>
        <h2
          class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500"
        >
          {$_("settings.section_overlay")}
        </h2>
        <Card>
          <div class="flex items-start gap-3">
            <Gamepad2 size={16} class="mt-0.5 shrink-0 text-zinc-500" />
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-zinc-100">
                {$_("settings.overlay_label")}
              </p>
              <p class="mt-0.5 text-xs leading-relaxed text-zinc-500">
                {$_("settings.overlay_desc")}
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={$overlayEnabled}
              aria-label={$_("settings.overlay_label")}
              onclick={() => setOverlayEnabled(!$overlayEnabled)}
              class="mt-0.5 flex h-6 w-11 shrink-0 items-center rounded-full p-0.5 transition-colors {$overlayEnabled
                ? 'bg-emerald-600'
                : 'bg-zinc-700'}"
            >
              <span
                class="h-5 w-5 rounded-full bg-white transition-transform {$overlayEnabled
                  ? 'translate-x-5'
                  : ''}"
              ></span>
            </button>
          </div>

          {#if $overlayEnabled}
            <div class="mt-4 flex items-center gap-3 border-t border-white/[0.08] pt-4">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-zinc-100">
                  {$_("settings.overlay_hotkey_label")}
                </p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  {$_("settings.overlay_hotkey_desc")}
                </p>
              </div>
              <button
                type="button"
                onclick={() => (capturingHotkey = true)}
                class="w-56 shrink-0 rounded-md border px-3 py-1.5 text-xs transition-colors {capturingHotkey
                  ? 'animate-pulse border-emerald-500 bg-emerald-600/20 text-emerald-200'
                  : 'border-white/[0.08] text-zinc-200 hover:bg-zinc-800/40'}"
              >
                {capturingHotkey
                  ? $_("settings.overlay_hotkey_capture")
                  : $overlayHotkey}
              </button>
              {#if $overlayHotkey !== DEFAULT_HOTKEY && !capturingHotkey}
                <button
                  type="button"
                  onclick={() => setOverlayHotkey(DEFAULT_HOTKEY)}
                  class="shrink-0 rounded-md border border-white/[0.08] px-2 py-1 text-xs text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100"
                  >{$_("settings.overlay_hotkey_reset")}</button
                >
              {/if}
            </div>
          {/if}
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
            <SettingsRow
              row={{
                field: "service_autostart",
                label: $_("settings.service_autostart_label"),
                description: $_("settings.service_autostart_desc"),
                icon: RefreshCw,
              }}
              value={serviceAutostart?.enabled ?? false}
              disabled={saving === ("service_autostart" as keyof api.Prefs)}
              onChange={toggleServiceAutostart}
            />
          </div>
          {#if serviceAutostartBlocked}
            <div
              class="border-t border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-200"
            >
              <p>
                {$_(
                  serviceAutostartMessageKey(
                    serviceAutostartBlocked.unsupported!,
                  ),
                )}
              </p>
              {#if serviceAutostartBlocked.detail}
                <p class="mt-1 break-words text-xs text-amber-200/70">
                  {serviceAutostartBlocked.detail}
                </p>
              {/if}
            </div>
          {/if}
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
        Cloud-only settings section. Shown only when signed in to Hoard Cloud.
        (The former "Modo ahorro (solo subida)" toggle was removed on
        2026-07-04 — it was confusing and never wired to the agent.)
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
            </div>
          </Card>
        </section>
      {/if}

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
              <!-- Attribution for the save-path catalogue. CC BY-NC-SA 3.0
                   requires crediting the source wherever the data is used, and
                   the app is where it is actually used. -->
              <p class="mt-2 text-xs text-zinc-600">
                {$_("settings.about_catalog_credit")}
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
