<script lang="ts">
  import Router, { push, replace, location } from "svelte-spa-router";
  import { onMount, onDestroy } from "svelte";
  import {
    Archive,
    Library,
    Home,
    Settings as SettingsIcon,
    Sparkles,
    AlertCircle,
    HardDrive,
    ScrollText,
    LogIn,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { formatBytes } from "./lib/utils/format";

  import Welcome from "./routes/Welcome.svelte";
  import ServerSetup from "./routes/ServerSetup.svelte";
  import TokenSetup from "./routes/TokenSetup.svelte";
  import OnboardingDone from "./routes/OnboardingDone.svelte";
  import Dashboard from "./routes/Dashboard.svelte";
  import LibraryRoute from "./routes/Library.svelte";
  import SettingsRoute from "./routes/Settings.svelte";
  import HistoryRoute from "./routes/History.svelte";
  import LogsRoute from "./routes/Logs.svelte";
  import DiagnosticsRoute from "./routes/Diagnostics.svelte";
  import AccountRoute from "./routes/Account.svelte";

  import Toaster from "./lib/components/Toaster.svelte";
  import UpdateConfirmModal from "./lib/components/UpdateConfirmModal.svelte";
  import ErrorDialog from "./lib/components/ErrorDialog.svelte";
  import LiveStatus from "./lib/components/LiveStatus.svelte";
  import Logo from "./lib/components/Logo.svelte";
  import ActivityFeed from "./lib/components/ActivityFeed.svelte";
  import { subscribeLive, unsubscribeLive } from "./lib/stores/live";
  import { APP_VERSION } from "./lib/version";
  import { errorDialog, dismissError, showError } from "./lib/stores/error_dialog";
  import { auth, hydrateAuth } from "./lib/stores/auth";
  import {
    cloud,
    hydrateCloud,
    initCloudDeepLink,
    openUpgradePage,
    planLabel,
  } from "./lib/stores/cloud";
  import { loadStep, routeForStep } from "./lib/stores/onboarding";
  import {
    runAutomaticSetup,
    automaticState,
    initAutomaticListener,
  } from "./lib/stores/automatic";
  import { toastInfo, toastSuccess } from "./lib/stores/toasts";
  import { prefs, hydratePrefs } from "./lib/stores/prefs";
  import * as api from "./lib/api";
  import {
    checkForUpdates,
    lastReport,
    startUpdatePoller,
    type UpdateReport,
  } from "./lib/stores/updates";

  /**
   * Routing layout
   * --------------
   * The wizard routes (`/welcome`, `/onboarding/*`) render full-screen and
   * own the entire viewport. The app routes (`/dashboard`, …) render inside
   * the persistent sidebar shell. We pick which to show based on the current
   * URL — auth state decides which URL we land on at boot.
   */
  // svelte-spa-router routes. The catch-all is handled in `onMount`: we
  // hydrate auth, then `replace()` to the appropriate destination, so we
  // don't need a `*` route here.
  const routes = {
    "/welcome": Welcome,
    "/onboarding/server": ServerSetup,
    "/onboarding/token": TokenSetup,
    "/onboarding/done": OnboardingDone,
    "/dashboard": Dashboard,
    "/library": LibraryRoute,
    "/settings": SettingsRoute,
    // The old `/history` index was a duplicate of the Dashboard, so it was
    // dropped from the nav. The per-save timeline still lives here and is
    // reached by clicking a save in the Dashboard / Library.
    "/history/:saveId": HistoryRoute,
    "/logs": LogsRoute,
    "/diagnostics": DiagnosticsRoute,
    "/account": AccountRoute,
  };

  let booted = $state(false);
  let updateModalOpen = $state(false);
  let automaticMode = $state(false);
  let automaticBusy = $state(false);

  // The update report is owned by `lastReport` in `stores/updates.ts` — both
  // the boot probe and the periodic re-check write to it. Reading via
  // `$lastReport` keeps this view in sync without a local mirror state.
  const updates = $derived<UpdateReport | null>($lastReport);

  // Long-running sessions need a periodic re-check; boot probe alone misses
  // releases shipped after the app was opened. The poller fires every 6h with
  // exponential backoff on failure (24h cap). Captured here so logout / unmount
  // can cancel it.
  let disposeUpdatePoller: (() => void) | null = null;

  // Hidden diagnostics unlock — 5 consecutive clicks on the sidebar version
  // string flips a session flag that reveals the Agent Diagnostics card in
  // Settings. Deliberately undocumented; only useful for triaging the silent
  // autobackup failure mode introduced before P1.4.0-0.
  let versionClicks = $state(0);
  let lastVersionClick = 0;
  function handleVersionClick() {
    const now = Date.now();
    // Reset the streak if the user pauses for >1.5s between taps. Keeps the
    // gesture deliberate — a stray double-click on idle UI shouldn't drift
    // toward unlocking.
    versionClicks = now - lastVersionClick > 1500 ? 1 : versionClicks + 1;
    lastVersionClick = now;
    if (versionClicks >= 5) {
      sessionStorage.setItem("hoard-diagnostics-unlocked", "1");
      versionClicks = 0;
      // Lazy import keeps the toast store out of the boot path.
      import("./lib/stores/toasts").then(({ toastSuccess }) =>
        toastSuccess($_("diagnostics.unlocked_toast")),
      );
    }
  }

  // Used by the small alert button next to the version. True when either the
  // client or the user's server has a newer version available.
  const hasUpdate = $derived(
    !!(updates && (updates.client.available || updates.server?.available)),
  );

  onMount(async () => {
    // Cheap OS detection so the global stylesheet can swap font-family per
    // platform without pulling `@tauri-apps/plugin-os` (not installed). The
    // Tauri WebView keeps the host UA on each platform, so this heuristic is
    // reliable enough for cosmetic tweaks. Idempotent — classList dedupes.
    const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
    let osTag: "linux" | "macos" | "windows" | "unknown" = "unknown";
    if (/Linux/i.test(ua) && !/Android/i.test(ua)) osTag = "linux";
    else if (/Mac/i.test(ua)) osTag = "macos";
    else if (/Windows/i.test(ua)) osTag = "windows";
    document.documentElement.classList.add(`is-${osTag}`);

    await Promise.all([hydrateAuth(), hydrateCloud()]);
    if ($auth.user) {
      replace("/dashboard");
    } else {
      const step = await loadStep();
      replace(routeForStep(step));
    }
    booted = true;

    // Cloud OAuth callback handler: when the system browser hits
    // `hoard://auth/callback#access_token=…`, the Rust deep-link plugin
    // emits `deep-link://new-url`; we parse the fragment and call
    // `cloud_complete_login`. Route to /account on success so the user
    // sees their freshly-loaded plan + usage.
    initCloudDeepLink(
      () => {
        push("/account");
        toastSuccess($_("account.signin_success"));
      },
      (e) => {
        const msg = typeof e === "string" ? e : (e as Error).message;
        toastInfo($_("account.signin_failed", { values: { error: msg } }));
      },
    );

    // Register the Tauri listener exactly once for the lifetime of this
    // app instance. The Rust scheduler emits `automatic-tick` on its
    // interval; this listener turns each tick into a `runAutomaticSetup`
    // invocation. Idempotent — safe to call from any onMount path.
    initAutomaticListener();

    // Hydrate the global prefs store so the sidebar toggle and any other
    // subscriber (Settings.svelte → `auto_restore`) paint with the correct
    // values the moment the boot blank lifts. `hydratePrefs` swallows its
    // own errors and falls back to pessimistic defaults.
    await hydratePrefs();
    automaticMode = $prefs?.automatic_mode ?? false;

    // Startup scan when Modo Automático is on. The Rust scheduler also emits
    // an immediate `automatic-tick` from `restart_if_enabled`, but that fires
    // during Tauri `setup()` — before this component mounts and installs the
    // listener above — so it's almost always dropped (the emit is
    // best-effort, no queue). Relying on it meant the app could open with the
    // toggle on yet not scan/track/monitor anything until the next interval
    // (hours later). Kicking the flow here makes the first scan reliable on
    // every launch. `runAutomaticSetup` guards against re-entrancy, so if the
    // Rust tick *was* caught this is a no-op.
    if (automaticMode && $auth.user) {
      void runAutomaticSetup();
    }

    // Subscribe to the live event firehose once. LiveStatus + ActivityFeed
    // read from the resulting stores; subscribing here (vs. in each
    // component's onMount) means a panel toggle doesn't tear down or
    // re-arm the listener and miss events in the gap.
    void subscribeLive();

    // Fire-and-forget update probe once auth is settled. The result feeds
    // the small "Update available" banner above the sidebar footer; a
    // network blip silently leaves it hidden, which is the right default.
    if ($auth.user) {
      // `checkForUpdates` writes into `lastReport`; the `$derived` above picks it up.
      checkForUpdates().catch((e) =>
        console.warn("update check failed:", e),
      );
      // And keep checking quietly while the session stays open.
      disposeUpdatePoller = startUpdatePoller();
    }
  });

  onDestroy(() => {
    disposeUpdatePoller?.();
    disposeUpdatePoller = null;
    void unsubscribeLive();
  });

  async function toggleActivityFeed() {
    const visible = !($prefs?.live_activity_visible ?? true);
    try {
      const updated = await api.setLiveActivityVisible(visible);
      prefs.set(updated);
    } catch (e) {
      showError(e);
    }
  }

  // Live phase label used while a scan is in progress. When the scheduler
  // is idle we show the plain "Modo Automático · ON/OFF" string built
  // inline in the markup; this only kicks in for the transient phases the
  // background flow walks through.
  const automaticPhaseLabel = $derived.by(() => {
    const s = $automaticState;
    switch (s.kind) {
      case "idle":
        return null;
      case "detecting":
        return $_("automatic.detecting");
      case "tracking":
        return $_("automatic.tracking", {
          values: { done: s.done, total: s.total },
        });
      case "starting_agent":
        return $_("automatic.starting_agent");
      case "syncing":
        return $_("automatic.syncing");
    }
  });

  // Sidebar plan-usage indicator. Reads straight from `$auth.user` which
  // the `auth` store already refreshes every 30 s via `refreshQuota` —
  // we don't poll here. Three shapes:
  //   - `null`     when no user, or when remote+unlimited quota (rare cloud
  //                edge case we don't pretend to render a bar for).
  //   - `local`    when `is_local_server` — disk usage on your own box is
  //                meaningless as a percentage, so we render a static
  //                "Local server" label.
  //   - `remote`   otherwise — used / quota bytes + percentage + colour
  //                ramp (emerald < 75% < amber < 90% < rose).
  const quotaInfo = $derived.by(() => {
    const u = $auth.user;
    if (!u) return null;
    if (u.is_local_server) return { kind: "local" as const };
    const quota = u.storage_quota_bytes;
    if (quota === 0) return null; // unlimited cloud — no bar to draw.
    const used = u.storage_used_bytes;
    const rawPct = Math.round((used / quota) * 100);
    const pct = Math.max(0, Math.min(rawPct, 100));
    const barColor =
      pct > 90 ? "bg-rose-500" : pct > 75 ? "bg-amber-500" : "bg-emerald-500";
    return { kind: "remote" as const, used, quota, pct, barColor };
  });

  async function toggleAutomatic() {
    if (automaticBusy) return;
    automaticBusy = true;
    try {
      const next = !automaticMode;
      const updated = await api.setAutomaticMode(next);
      // Fan the updated Prefs out to every subscriber (Settings.svelte
      // watches `$prefs.auto_restore`, which `set_automatic_mode` cascades
      // to true). Without this, the Settings toggle lies until the user
      // reloads the page.
      prefs.set(updated);
      automaticMode = updated.automatic_mode;
      if (automaticMode) {
        toastSuccess($_("automatic.toggled_on"));
        // Fire the first scan immediately so the user doesn't wait the
        // full interval to see the toggle do something. Subsequent runs
        // are driven by the Rust scheduler via `automatic-tick`.
        void runAutomaticSetup();
      } else {
        toastInfo($_("automatic.toggled_off"));
      }
    } catch (e) {
      showError(e);
    } finally {
      automaticBusy = false;
    }
  }

  // The `labelKey` is resolved through `$_()` at render time so the sidebar
  // re-translates instantly when the user switches language in Settings —
  // hard-coded English here was the long-standing reason German/Spanish UIs
  // still showed "Library / Dashboard …" in the rail.
  //
  // First entry is the account button, sitting above Library: "Iniciar
  // sesión" when there's no cloud session, "Inicio" (the read-only account /
  // plan-status view) once signed in. Derived so it swaps the moment the
  // cloud session hydrates. The old "Historial" item was removed — it just
  // duplicated the Dashboard.
  const sidebarItems = $derived([
    $cloud.account
      ? { labelKey: "nav.home", icon: Home, route: "/account" }
      : { labelKey: "nav.sign_in", icon: LogIn, route: "/account" },
    { labelKey: "nav.library", icon: Library, route: "/library" },
    { labelKey: "nav.dashboard", icon: Archive, route: "/dashboard" },
    { labelKey: "nav.settings", icon: SettingsIcon, route: "/settings" },
  ]);

  // App-shell routes share the persistent sidebar; wizard routes own the
  // viewport. `/history` stays here so the per-save timeline (`/history/:id`)
  // still renders inside the rail even though the index was removed.
  const APP_ROUTE_PREFIXES = [
    "/dashboard",
    "/library",
    "/settings",
    "/history",
    "/logs",
    "/diagnostics",
    "/account",
  ];
  const isAppRoute = $derived(
    APP_ROUTE_PREFIXES.some((p) => $location.startsWith(p)),
  );

  // First letter for the avatar fallback when the cloud account has no
  // `avatar_url` (email/password sign-ups, or providers that don't return
  // a picture). Display name wins over email so it reads as a person.
  const accountInitial = $derived.by(() => {
    const a = $cloud.account;
    if (!a) return "?";
    const src = a.display_name?.trim() || a.email;
    return src ? src.charAt(0).toUpperCase() : "?";
  });
</script>

{#if !booted}
  <!--
    Tiny boot blank. We keep the markup minimal so users on slow disks don't
    see a flash of the welcome screen before the auth check completes.
  -->
  <div class="flex h-full items-center justify-center bg-zinc-950">
    <div
      class="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-emerald-500"
    ></div>
  </div>
{:else if isAppRoute}
  <div class="flex h-full">
    <aside
      class="flex w-60 shrink-0 flex-col border-r border-zinc-800/80 bg-gradient-to-b from-zinc-950 via-zinc-950 to-zinc-900 shadow-[inset_-1px_0_0_0_rgba(255,255,255,0.03)]"
    >
      <div class="flex items-center gap-2 px-4 py-4">
        <Logo size={36} class="shrink-0 rounded-lg" />
        <div class="min-w-0 flex-1">
          <div class="text-base font-semibold tracking-tight">Hoard</div>
          <button
            type="button"
            onclick={handleVersionClick}
            class="cursor-default select-none text-left text-xs text-zinc-500 outline-none"
            tabindex="-1"
            aria-hidden="true"
          >
            v{APP_VERSION}
          </button>
        </div>
        <!-- ActivityFeed toggle: small scroll icon, dim when the panel is
             hidden so the affordance reads as "off". -->
        <button
          type="button"
          onclick={toggleActivityFeed}
          aria-label={$_("activity.toggle_label")}
          title={$_("activity.toggle_label")}
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border transition-colors {$prefs?.live_activity_visible ?? true
            ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300 hover:bg-emerald-500/20'
            : 'border-zinc-700 bg-zinc-900 text-zinc-400 hover:text-zinc-100'}"
        >
          <ScrollText size={14} />
        </button>
        <!-- Small amber alert button. Same visual language as "Sin carpeta":
             border + tinted background, no rounded-full pill. Click pops a
             confirmation modal; we don't auto-install behind the user's back. -->
        {#if hasUpdate}
          <button
            type="button"
            onclick={() => (updateModalOpen = true)}
            title={updates?.client.available
              ? $_("updates.client_available", {
                  values: { latest: updates?.client.latest ?? "?" },
                })
              : $_("updates.server_available", {
                  values: { latest: updates?.server?.latest ?? "?" },
                })}
            aria-label={$_("updates.button_label")}
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/10 text-amber-300 transition-colors hover:bg-amber-500/20"
          >
            <AlertCircle size={14} />
          </button>
        {/if}
      </div>

      <nav class="flex-1 space-y-1 px-3 py-2">
        {#each sidebarItems as item (item.route)}
          {@const active = $location === item.route}
          {@const enabled =
            item.route === "/account" ||
            item.route === "/dashboard" ||
            item.route === "/library" ||
            item.route === "/settings"}
          <button
            type="button"
            disabled={!enabled}
            aria-disabled={!enabled}
            aria-label={$_(item.labelKey)}
            onclick={() => push(item.route)}
            class="group flex w-full items-center gap-3 rounded-md border-l-2 px-3 py-2 text-sm transition-colors duration-150
              {active
              ? 'border-emerald-500 bg-zinc-800/50 text-zinc-50'
              : 'border-transparent text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100'}
              disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-zinc-400"
            title={enabled ? undefined : $_("nav.coming_later")}
          >
            <item.icon size={18} />
            <span>{$_(item.labelKey)}</span>
          </button>
        {/each}
      </nav>

      <!-- Sidebar footer: "Modo Automático" toggle. The "Update available"
           alert lives next to the version up top now, not here. Colour
           swaps between emerald (on) and rose (off); a transient scan
           phase pulses the icon while the background flow is running. -->
      <div class="border-t border-zinc-800/60 px-3 py-3 space-y-3">
        <!-- Compact live status (watcher + cloud). Always rendered when in
             the app shell — degrades to a neutral dot until the agent or
             cloud loop come online. -->
        <LiveStatus />
        <!-- Cloud account chip: avatar + plan (+ "Mejorar plan" when on Free),
             routing to /account. The signed-out "Iniciar sesión" entry now
             lives at the top of the nav, so we only render this when signed
             in to avoid a duplicate button. -->
        {#if $cloud.hydrated && $cloud.account}
            <div class="flex items-center gap-2">
              <button
                type="button"
                onclick={() => push("/account")}
                class="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-zinc-800 bg-zinc-900/60 px-2 py-1.5 text-left transition-colors hover:border-zinc-700 hover:bg-zinc-800/60"
                title={$_("sidebar.account_tooltip")}
              >
                {#if $cloud.account.avatar_url}
                  <img
                    src={$cloud.account.avatar_url}
                    alt=""
                    class="h-7 w-7 shrink-0 rounded-full object-cover ring-1 ring-zinc-700"
                  />
                {:else}
                  <span
                    class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-xs font-semibold text-emerald-300 ring-1 ring-emerald-500/30"
                  >
                    {accountInitial}
                  </span>
                {/if}
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-xs font-medium text-zinc-100">
                    {$cloud.account.display_name ?? $cloud.account.email}
                  </span>
                  <span class="block truncate text-[10px] text-zinc-500">
                    {planLabel($cloud.account.plan)}
                  </span>
                </span>
              </button>
              {#if $cloud.account.plan === "free"}
                <button
                  type="button"
                  onclick={() => openUpgradePage("pro")}
                  class="flex shrink-0 items-center gap-1 rounded-md bg-gradient-to-r from-emerald-400 to-teal-400 px-2.5 py-2 text-[11px] font-semibold text-emerald-950 shadow-sm shadow-emerald-500/30 transition-all hover:from-emerald-300 hover:to-teal-300 hover:shadow-emerald-500/50"
                  title={$_("sidebar.upgrade_tooltip")}
                >
                  <Sparkles size={12} />
                  {$_("sidebar.upgrade")}
                </button>
              {/if}
            </div>
        {/if}
        {#if quotaInfo}
          {#if quotaInfo.kind === "local"}
            <div
              class="flex items-center gap-2 px-1 text-[11px] text-zinc-400"
            >
              <HardDrive size={12} />
              <span>{$_("sidebar.plan_local")}</span>
            </div>
          {:else}
            <div class="space-y-1 px-1">
              <div
                class="flex items-center justify-between text-[11px] text-zinc-400"
              >
                <span
                  >{formatBytes(quotaInfo.used)} / {formatBytes(
                    quotaInfo.quota,
                  )}</span
                >
                <span>{quotaInfo.pct}%</span>
              </div>
              <div class="h-1.5 overflow-hidden rounded-full bg-zinc-800">
                <div
                  class="h-full rounded-full transition-all duration-500 {quotaInfo.barColor}"
                  style="width: {Math.min(quotaInfo.pct, 100)}%"
                ></div>
              </div>
            </div>
          {/if}
        {/if}
        <button
          type="button"
          onclick={toggleAutomatic}
          disabled={automaticBusy}
          class="flex w-full items-center justify-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors {automaticMode
            ? 'border-emerald-500/40 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'
            : 'border-rose-500/40 bg-rose-500/15 text-rose-300 hover:bg-rose-500/25'} disabled:cursor-wait disabled:opacity-60"
          aria-label={$_("automatic.aria_toggle")}
          title={$_("automatic.help_tooltip")}
        >
          <Sparkles
            size={16}
            class={$automaticState.kind === "idle" ? "" : "animate-pulse"}
          />
          <span>
            {#if automaticPhaseLabel}
              {automaticPhaseLabel}
            {:else}
              {$_("automatic.title")} ·
              {automaticMode ? $_("automatic.on") : $_("automatic.off")}
            {/if}
          </span>
        </button>
        <p class="px-1 text-[11px] leading-tight text-zinc-500">
          {automaticMode
            ? $_("automatic.subtitle_on")
            : $_("automatic.subtitle_off")}
        </p>
      </div>
    </aside>

    <main class="flex-1 overflow-y-auto">
      <Router {routes} />
    </main>
  </div>
{:else}
  <!-- Wizard routes render full-screen, no sidebar. -->
  <div class="h-full overflow-y-auto">
    <Router {routes} />
  </div>
{/if}

<UpdateConfirmModal
  open={updateModalOpen}
  report={updates}
  onClose={() => (updateModalOpen = false)}
/>

<ErrorDialog error={$errorDialog} onClose={dismissError} />

{#if isAppRoute && ($prefs?.live_activity_visible ?? true)}
  <ActivityFeed onClose={() => toggleActivityFeed()} />
{/if}

<Toaster />
