<script lang="ts">
  import Router, { push, replace, location } from "svelte-spa-router";
  import { onMount, onDestroy } from "svelte";
  import {
    Archive,
    Library,
    Home,
    Settings as SettingsIcon,
    Map as MapIcon,
    Sparkles,
    AlertCircle,
    HardDrive,
    ScrollText,
    LogIn,
    RotateCw,
    RefreshCw,
    Boxes,
    ChevronDown,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import { formatBytes } from "./lib/utils/format";

  import Welcome from "./routes/Welcome.svelte";
  import ChooseMode from "./routes/ChooseMode.svelte";
  import ServerSetup from "./routes/ServerSetup.svelte";
  import TokenSetup from "./routes/TokenSetup.svelte";
  import OnboardingDone from "./routes/OnboardingDone.svelte";
  import Dashboard from "./routes/Dashboard.svelte";
  import LibraryRoute from "./routes/Library.svelte";
  import SettingsRoute from "./routes/Settings.svelte";
  import HistoryRoute from "./routes/History.svelte";
  import MapRoute from "./routes/Map.svelte";
  import LogsRoute from "./routes/Logs.svelte";
  import DiagnosticsRoute from "./routes/Diagnostics.svelte";
  import AccountRoute from "./routes/Account.svelte";

  import Toaster from "./lib/components/Toaster.svelte";
  import UpdateConfirmModal from "./lib/components/UpdateConfirmModal.svelte";
  import ErrorDialog from "./lib/components/ErrorDialog.svelte";
  import LiveStatus from "./lib/components/LiveStatus.svelte";
  import Logo from "./lib/components/Logo.svelte";
  import ActivityFeed from "./lib/components/ActivityFeed.svelte";
  import {
    subscribeLive,
    unsubscribeLive,
    resetCloudLoop,
  } from "./lib/stores/live";
  import { APP_VERSION } from "./lib/version";
  import { errorDialog, dismissError, showError } from "./lib/stores/error_dialog";
  import { auth, hydrateAuth, signOut } from "./lib/stores/auth";
  import {
    cloud,
    hydrateCloud,
    initCloudDeepLink,
    initCloudSessionWatch,
    openUpgradePage,
    planLabel,
  } from "./lib/stores/cloud";
  import {
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
    "/onboarding/choose": ChooseMode,
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
    "/map": MapRoute,
    "/logs": LogsRoute,
    "/diagnostics": DiagnosticsRoute,
    "/account": AccountRoute,
  };

  let booted = $state(false);
  let updateModalOpen = $state(false);
  let automaticMode = $state(false);
  let automaticBusy = $state(false);
  let globalSync = $state(false);
  let globalSyncBusy = $state(false);

  // Self-hosted reachability escape hatch.
  // ---------------------------------------
  // A self-hosted session (address + token) is persisted on disk + the OS
  // keyring, so it survives folder deletion AND a full reinstall. If the
  // server is gone (decommissioned, different network, never coming back) the
  // app would silently retry it forever with no obvious way out — the user
  // ends up reinstalling in a loop. We probe the saved server once at boot;
  // if it's unreachable we surface a banner offering "Retry" / "Forget server"
  // so the dead session can actually be dropped without hunting through
  // Settings. Cloud-only users (`$auth.user == null`) never see this.
  let serverUnreachable = $state(false);
  let probingServer = $state(false);
  let forgettingServer = $state(false);

  async function probeSelfHostedServer() {
    const u = $auth.user;
    if (!u?.server_url) {
      serverUnreachable = false;
      return;
    }
    probingServer = true;
    try {
      await api.healthCheck(u.server_url);
      serverUnreachable = false;
    } catch (e) {
      console.warn("self-hosted server probe failed:", e);
      serverUnreachable = true;
    } finally {
      probingServer = false;
    }
  }

  async function forgetUnreachableServer() {
    forgettingServer = true;
    try {
      await signOut();
      serverUnreachable = false;
      replace("/welcome");
    } catch (e) {
      showError(e);
    } finally {
      forgettingServer = false;
    }
  }

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
    } else if ($cloud.account) {
      // Cloud-only user (signed in via Gmail, no self-hosted server). Without
      // this branch they'd be dumped back into the onboarding wizard on every
      // launch because the old boot only checked `$auth.user`.
      replace("/account");
    } else {
      // No session at all → always start at the welcome screen. We used to
      // resume `routeForStep(loadStep())`, but a persisted "server" step (left
      // over from an old install or a session that was later logged out) trapped
      // the user straight on "Connect to your server" instead of showing the
      // welcome / chooser. The wizard re-hydrates the saved URL anyway, so
      // restarting from welcome loses nothing.
      replace("/welcome");
    }
    booted = true;

    // Fire-and-forget reachability probe for a restored self-hosted session.
    // Non-blocking so a slow/dead server never holds up the UI; the banner
    // appears once the probe settles. Cloud-only users short-circuit inside.
    if ($auth.user) {
      void probeSelfHostedServer();
    }

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

    // Terminal cloud session expiry (Supabase revoked the refresh token). The
    // Rust side already cleared the session + stopped the pollers; here we calm
    // the cloud dot, tell the user, and — if they're cloud-only — route them to
    // the welcome screen so they can sign in again. A self-hosted user stays put.
    initCloudSessionWatch(() => {
      resetCloudLoop();
      toastInfo($_("account.session_expired"));
      if (!$auth.user) replace("/welcome");
    });

    // Register the Tauri listeners exactly once for the lifetime of this
    // app instance. Modo Automático's detect/track/sweep work runs entirely in
    // Rust now (`commands/automatic.rs`); these listeners just mirror its
    // `automatic-phase` / `automatic-scan-complete` events into the sidebar UI.
    // Idempotent — safe to call from any onMount path.
    initAutomaticListener();

    // Hydrate the global prefs store so the sidebar toggle and any other
    // subscriber (Settings.svelte → `auto_restore`) paint with the correct
    // values the moment the boot blank lifts. `hydratePrefs` swallows its
    // own errors and falls back to pessimistic defaults.
    await hydratePrefs();
    automaticMode = $prefs?.automatic_mode ?? false;
    globalSync = $prefs?.global_sync ?? false;

    // No startup scan kicked from here anymore: Rust's `restart_if_enabled`
    // (run during Tauri `setup()`) fires the first scan immediately when the
    // toggle was left on, and the work runs headless regardless of whether the
    // WebView is alive to catch the `automatic-phase` events. The listener
    // above will pick up phases mid-flight and the resulting tracked saves are
    // already in CliState by the time the user opens Library/Dashboard.

    // Subscribe to the live event firehose once. LiveStatus + ActivityFeed
    // read from the resulting stores; subscribing here (vs. in each
    // component's onMount) means a panel toggle doesn't tear down or
    // re-arm the listener and miss events in the gap.
    void subscribeLive();

    // Fire-and-forget update probe. The client-update check hits GitHub and
    // needs no session at all; the server half returns `null` when there's no
    // self-hosted server configured. Gating this on `$auth.user` meant a
    // user signed in only to Hoard Cloud (or fully signed out) never got the
    // desktop "update available" banner — they had to open Settings and run a
    // manual check. So we probe unconditionally and keep the poller running
    // for the whole session regardless of auth state.
    checkForUpdates().catch((e) =>
      console.warn("update check failed:", e),
    );
    disposeUpdatePoller = startUpdatePoller();
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
        // The Rust scheduler (`set_automatic_mode` → `automatic::start`) fires
        // an immediate scan tick the moment we flip the pref on, so there's
        // nothing to kick from here — the `automatic-phase` listener will show
        // the progress and `automatic-scan-complete` will toast the result.
      } else {
        toastInfo($_("automatic.toggled_off"));
      }
    } catch (e) {
      showError(e);
    } finally {
      automaticBusy = false;
    }
  }

  async function toggleGlobalSync() {
    if (globalSyncBusy) return;
    globalSyncBusy = true;
    try {
      const next = !globalSync;
      const updated = await api.setGlobalSync(next);
      prefs.set(updated);
      globalSync = updated.global_sync;
      if (globalSync) {
        toastSuccess($_("sync.toggled_on"));
      } else {
        toastInfo($_("sync.toggled_off"));
      }
    } catch (e) {
      showError(e);
    } finally {
      globalSyncBusy = false;
    }
  }

  // Sidebar navigation model. Two kinds of entry keep the `{#each}`
  // declarative: a plain navigable `link`, and a collapsible `group`
  // (Hoard-Saves) that owns a list of `link` children. `labelKey` is
  // resolved through `$_()` at render time so the rail re-translates
  // instantly when the language changes in Settings — hard-coded English
  // here was the long-standing reason German/Spanish UIs still showed
  // "Library / Dashboard …" in the rail.
  type NavLink = {
    kind: "link";
    labelKey: string;
    icon: typeof Home;
    route: string;
  };
  type NavGroup = {
    kind: "group";
    id: string;
    labelKey: string;
    icon: typeof Home;
    children: NavLink[];
  };
  type NavEntry = NavLink | NavGroup;

  // Collapsed/expanded state for the Hoard-Saves group, remembered across
  // sessions. Defaults to open on first run or when storage is unreadable.
  let savesOpen = $state(readSavesOpen());
  function readSavesOpen(): boolean {
    try {
      return localStorage.getItem("hoard-nav-saves-open") !== "0";
    } catch {
      return true;
    }
  }
  function toggleSaves() {
    savesOpen = !savesOpen;
    try {
      localStorage.setItem("hoard-nav-saves-open", savesOpen ? "1" : "0");
    } catch {
      /* private mode / storage disabled — toggle still works for the session */
    }
  }

  // First entry is the account button: "Iniciar sesión" with no cloud
  // session, "Inicio" (the read-only account/plan view) once signed in.
  // Biblioteca / Panel / Mapa live under the collapsible Hoard-Saves group;
  // Ajustes stays last. The old "Historial" item was removed — it just
  // duplicated the Dashboard.
  const navEntries = $derived<NavEntry[]>([
    $cloud.account
      ? { kind: "link", labelKey: "nav.home", icon: Home, route: "/account" }
      : { kind: "link", labelKey: "nav.sign_in", icon: LogIn, route: "/account" },
    {
      kind: "group",
      id: "saves",
      labelKey: "nav.hoard_saves",
      icon: Boxes,
      children: [
        { kind: "link", labelKey: "nav.library", icon: Library, route: "/library" },
        { kind: "link", labelKey: "nav.dashboard", icon: Archive, route: "/dashboard" },
        { kind: "link", labelKey: "nav.map", icon: MapIcon, route: "/map" },
      ],
    },
    { kind: "link", labelKey: "nav.settings", icon: SettingsIcon, route: "/settings" },
  ]);

  // App-shell routes share the persistent sidebar; wizard routes own the
  // viewport. `/history` stays here so the per-save timeline (`/history/:id`)
  // still renders inside the rail even though the index was removed.
  const APP_ROUTE_PREFIXES = [
    "/dashboard",
    "/library",
    "/settings",
    "/history",
    "/map",
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

  // If the remote avatar 404/403s (Google's lh3 host sometimes rejects on
  // referer), fall back to the initial badge instead of a broken image.
  let avatarFailed = $state(false);
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
      class="flex w-60 shrink-0 flex-col border-r border-white/[0.06] bg-gradient-to-b from-zinc-950/80 via-zinc-950/60 to-zinc-900/40 backdrop-blur-sm shadow-[inset_-1px_0_0_0_rgba(255,255,255,0.04)]"
    >
      <div class="flex items-center gap-2 px-4 py-4">
        <Logo size={36} class="shrink-0 rounded-lg" />
        <div class="min-w-0 flex-1">
          <div class="font-display text-xl font-semibold leading-none text-zinc-50">
            Hoard
          </div>
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

      <!-- Shared button markup for top-level links and indented group
           children, so the two render paths can't drift apart. -->
      {#snippet navLink(item: NavLink, indented: boolean)}
        {@const active = $location === item.route}
        <button
          type="button"
          aria-label={$_(item.labelKey)}
          aria-current={active ? "page" : undefined}
          onclick={() => push(item.route)}
          class="group flex w-full items-center gap-3 rounded-md border-l-2 py-2 text-sm transition-colors duration-150
            {indented ? 'pl-9 pr-3' : 'px-3'}
            {active
            ? 'border-emerald-500 bg-zinc-800/50 text-zinc-50'
            : 'border-transparent text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100'}"
        >
          <item.icon size={indented ? 16 : 18} />
          <span>{$_(item.labelKey)}</span>
        </button>
      {/snippet}

      <nav class="flex-1 space-y-1 px-3 py-2">
        {#each navEntries as entry (entry.kind === "group" ? entry.id : entry.route)}
          {#if entry.kind === "link"}
            {@render navLink(entry, false)}
          {:else}
            <!-- A child on the active route forces the group open even if the
                 user had collapsed it, so the highlight is never hidden. -->
            {@const childActive = entry.children.some(
              (c) => $location === c.route,
            )}
            {@const open = savesOpen || childActive}
            <button
              type="button"
              aria-expanded={open}
              aria-label={$_(entry.labelKey)}
              onclick={toggleSaves}
              class="group flex w-full items-center gap-3 rounded-md border-l-2 border-transparent px-3 py-2 text-sm text-zinc-400 transition-colors duration-150 hover:bg-zinc-800/40 hover:text-zinc-100"
            >
              <entry.icon size={18} />
              <span class="flex-1 text-left">{$_(entry.labelKey)}</span>
              <ChevronDown
                size={16}
                class="shrink-0 transition-transform duration-150 {open
                  ? ''
                  : '-rotate-90'}"
              />
            </button>
            {#if open}
              <div class="space-y-1">
                {#each entry.children as child (child.route)}
                  {@render navLink(child, true)}
                {/each}
              </div>
            {/if}
          {/if}
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
                {#if $cloud.account.avatar_url && !avatarFailed}
                  <img
                    src={$cloud.account.avatar_url}
                    alt=""
                    referrerpolicy="no-referrer"
                    onerror={() => (avatarFailed = true)}
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
          onclick={toggleGlobalSync}
          disabled={globalSyncBusy}
          class="flex w-full items-center justify-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors {globalSync
            ? 'border-emerald-500/40 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'
            : 'border-rose-500/40 bg-rose-500/15 text-rose-300 hover:bg-rose-500/25'} disabled:cursor-wait disabled:opacity-60"
          aria-label={$_("sync.aria_toggle")}
          title={$_("sync.help_tooltip")}
        >
          <RefreshCw size={16} class={globalSync ? "animate-pulse" : ""} />
          <span>
            {$_("sync.title")} ·
            {globalSync ? $_("sync.on") : $_("sync.off")}
          </span>
        </button>
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
      {#if serverUnreachable && $auth.user}
        <div
          class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-amber-500/40 bg-amber-500/10 px-6 py-3"
        >
          <AlertCircle size={18} class="shrink-0 text-amber-300" />
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-amber-200">
              {$_("server.unreachable_title")}
            </p>
            <p class="mt-0.5 text-xs text-amber-200/70">
              {$_("server.unreachable_body", {
                values: { url: $auth.user.server_url },
              })}
            </p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <button
              type="button"
              onclick={probeSelfHostedServer}
              disabled={probingServer || forgettingServer}
              class="inline-flex items-center gap-1.5 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-1.5 text-xs font-medium text-amber-200 transition-colors hover:bg-amber-500/20 disabled:cursor-wait disabled:opacity-60"
            >
              <RotateCw
                size={13}
                class={probingServer ? "animate-spin" : ""}
              />
              {$_("server.retry")}
            </button>
            <button
              type="button"
              onclick={forgetUnreachableServer}
              disabled={forgettingServer || probingServer}
              class="inline-flex items-center gap-1.5 rounded-md bg-red-600 px-3 py-1.5 text-xs font-medium text-red-50 transition-colors hover:bg-red-500 disabled:cursor-wait disabled:opacity-60"
            >
              {$_("server.forget")}
            </button>
          </div>
        </div>
      {/if}
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
