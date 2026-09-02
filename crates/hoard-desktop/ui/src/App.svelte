<script lang="ts">
  // svelte-spa-router 5 dropped the `location`/`querystring`/`params` stores and
  // replaced them with the `router` object, which is rune-based state: reading
  // `router.location` inside a `$derived`, an `$effect` or the markup is already
  // reactive, with no `$` subscription.
  import Router, { push, replace, router } from "svelte-spa-router";
  import { onMount, onDestroy } from "svelte";
  import { fly } from "svelte/transition";
  import {
    Archive,
    Library,
    Home,
    Settings as SettingsIcon,
    Sparkles,
    AlertCircle,
    ScrollText,
    LogIn,
    RotateCw,
    RefreshCw,
    Boxes,
    ChevronDown,
    MonitorPlay,
    Lock,
    Bell,
    Eye,
  } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  // The routes are loaded on demand. Importing them statically put all fifteen
  // screens (Dashboard, Library, Settings, Logs, Diagnostics, the overlay and the
  // rest) into the entry chunk, so starting up parsed half the application before
  // painting the first one. Every `import()` is a chunk Vite splits out and the
  // router asks for when it needs it; see `routes` below.
  import { wrap } from "svelte-spa-router/wrap";
  import RouteFallback from "./lib/components/RouteFallback.svelte";

  const loadLanguage = () => import("./routes/Language.svelte");
  const loadChooseMode = () => import("./routes/ChooseMode.svelte");
  const loadTerms = () => import("./routes/Terms.svelte");
  const loadServerSetup = () => import("./routes/ServerSetup.svelte");
  const loadTokenSetup = () => import("./routes/TokenSetup.svelte");
  const loadOnboardingDone = () => import("./routes/OnboardingDone.svelte");
  const loadDashboard = () => import("./routes/Dashboard.svelte");
  const loadLibrary = () => import("./routes/Library.svelte");
  const loadSettings = () => import("./routes/Settings.svelte");
  const loadHistory = () => import("./routes/History.svelte");
  const loadLogs = () => import("./routes/Logs.svelte");
  const loadDiagnostics = () => import("./routes/Diagnostics.svelte");
  const loadAccount = () => import("./routes/Account.svelte");
  const loadHoardScreen = () => import("./routes/HoardScreen.svelte");
  const loadHoardWrapped = () => import("./routes/HoardWrapped.svelte");
  const loadPro = () => import("./routes/Pro.svelte");

  /** Sugar so `loadingComponent` is not repeated on every route. */
  const lazy = (asyncComponent: () => Promise<unknown>) =>
    wrap({
      asyncComponent: asyncComponent as never,
      loadingComponent: RouteFallback as never,
    });

  import Toaster from "./lib/components/Toaster.svelte";
  import TourOverlay from "./lib/components/TourOverlay.svelte";
  import AccountDeletedModal from "./lib/components/AccountDeletedModal.svelte";
  import EyePanel from "./lib/components/EyePanel.svelte";
  import NotificationsPanel from "./lib/components/NotificationsPanel.svelte";
  import { notifications as notifStore, initServerNotifications } from "./lib/stores/notifications";
  import { glow } from "./lib/actions/glow";
import { tilt } from "./lib/actions/tilt";
  import { loadTourSeen, markTourSeen } from "./lib/stores/onboarding";
  import { tourActive } from "./lib/stores/tour";
  import UpdateConfirmModal from "./lib/components/UpdateConfirmModal.svelte";
  import ErrorDialog from "./lib/components/ErrorDialog.svelte";
  import UpdateGate from "./lib/components/UpdateGate.svelte";
  import QuotaMini from "./lib/components/QuotaMini.svelte";
  import Logo from "./lib/components/Logo.svelte";
  import ActivityFeed from "./lib/components/ActivityFeed.svelte";
  import StorageFullBanner from "./lib/components/StorageFullBanner.svelte";
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
    planLabel,
    refreshCloud,
    exportAllCloudData,
  } from "./lib/stores/cloud";
  import {
    liberateOpen,
    closeLiberate,
  } from "./lib/stores/liberate";
  import LiberateStorageModal from "./lib/components/LiberateStorageModal.svelte";
  import { planEvent, dismissPlanEvent } from "./lib/stores/planEvents";
  import ProThanksModal from "./lib/components/ProThanksModal.svelte";
  import ProFarewellModal from "./lib/components/ProFarewellModal.svelte";
  import {
    automaticState,
    initAutomaticListener,
  } from "./lib/stores/automatic";
  import { toastInfo, toastSuccess } from "./lib/stores/toasts";
  import { prefs, hydratePrefs } from "./lib/stores/prefs";
  import { initGameOverlay } from "./lib/stores/gameOverlay";
  import { hydrateCardSizes } from "./lib/stores/cardSizes.svelte";
  import {
    entitlements,
    refreshEntitlements,
    featureDaysLeft,
    featureUnlocked,
    PRO_DEV_UNLOCK,
    type FeatureKey,
  } from "./lib/stores/entitlements";
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
   * The wizard routes (`/onboarding/*`) render full-screen and
   * own the entire viewport. The app routes (`/dashboard`, …) render inside
   * the persistent sidebar shell. We pick which to show based on the current
   * URL, auth state decides which URL we land on at boot.
   */
  // svelte-spa-router routes. The catch-all is handled in `onMount`: we
  // hydrate auth, then `replace()` to the appropriate destination, so we
  // don't need a `*` route here.
  const routes = {
    "/onboarding/language": lazy(loadLanguage),
    "/onboarding/choose": lazy(loadChooseMode),
    "/onboarding/terms": lazy(loadTerms),
    "/onboarding/server": lazy(loadServerSetup),
    "/onboarding/token": lazy(loadTokenSetup),
    "/onboarding/done": lazy(loadOnboardingDone),
    "/dashboard": lazy(loadDashboard),
    "/library": lazy(loadLibrary),
    "/settings": lazy(loadSettings),
    // The old `/history` index was a duplicate of the Dashboard, so it was
    // dropped from the nav. The per-save timeline still lives here and is
    // reached by clicking a save in the Dashboard / Library.
    "/history/:saveId": lazy(loadHistory),
    "/logs": lazy(loadLogs),
    "/diagnostics": lazy(loadDiagnostics),
    "/account": lazy(loadAccount),
    // Premium feature placeholders (gated in the sidebar; the routes
    // themselves are reachable so an unlocked user lands on the empty state).
    "/hoard-screen": lazy(loadHoardScreen),
    "/hoard-wrapped": lazy(loadHoardWrapped),
    // Where every padlock leads. It lives inside the application on purpose: these
    // buttons used to open the browser on the pricing page.
    "/pro": lazy(loadPro),
  };

  let booted = $state(false);
  let updateModalOpen = $state(false);

  // Top-right overlay buttons: notifications (bell) + live status (eye).
  // Fixed to the top-right of the app window, above the sidebar + content.
  // The eye dropdown shows machines online + running games; the bell is a
  // placeholder for a future notifications panel (empty for now).
  let eyeOpen = $state(false);
  let notifOpen = $state(false);
  function toggleEye() {
    eyeOpen = !eyeOpen;
    if (eyeOpen) notifOpen = false;
  }
  function toggleNotif() {
    notifOpen = !notifOpen;
    if (notifOpen) eyeOpen = false;
  }

  // Ticking clock (1s) for the Eye panel's elapsed-time counter.
  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => {
      if (!document.hidden) now = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  // Responsive sidebar: collapse to an icon-rail on narrow windows (Steam
  // Deck, half-width tiling). The aside swaps to `.is-narrow`, which CSS uses
  // to hide labels and center icons, declarative, no per-element branching.
  let narrow = $state(false);
  $effect(() => {
    const mq = window.matchMedia("(max-width: 760px)");
    const sync = () => (narrow = mq.matches);
    sync();
    mq.addEventListener("change", sync);
    return () => mq.removeEventListener("change", sync);
  });

  // Guided app tour. Opens after onboarding whenever the *identity* you signed
  // in as differs from the one the tour was last shown for, so switching
  // accounts or self-hosting a different server replays it, while an ordinary
  // relaunch of the same session stays quiet. forget/logout/delete clear the
  // stored identity (see `clearTourSeen`), so reconnecting shows it again too.
  let showTour = $state(false);
  let lastSigChecked: string | null = null;
  const tourSig = $derived(
    $auth.user
      ? `self:${$auth.user.server_url}#${$auth.user.user_id}`
      : $cloud.account
        ? `cloud:${$cloud.account.user_id}`
        : null,
  );
  $effect(() => {
    const sig = tourSig;
    const loc = router.location;
    if (!booted || !sig || sig === lastSigChecked) return;
    // The tour is post-onboarding: it must not start while the wizard is still on
    // screen (the session exists from the `token` step, but the mode still has to be
    // picked in `done`). It waits for `finish()` to navigate to an app route; since
    // it depends on `router.location`, this effect re-evaluates after that
    // navigation.
    if (loc.startsWith("/onboarding")) return;
    lastSigChecked = sig;
    void loadTourSeen().then((seen) => {
      if (seen !== sig) showTour = true;
    });
  });
  async function finishTour() {
    showTour = false;
    if (tourSig) await markTourSeen(tourSig);
  }

  // --- Guided tour choreography ------------------------------------------
  // The tour drives the real app shell: each step navigates the content area
  // to its section (`tourNavigate`) and the sidebar spotlight glides to the
  // matching rail item (measured by `TourOverlay` from the `data-tour*`
  // markers). Keeping this in `App`, the owner of `<main>` and the nav, lets
  // the overlay stay purely presentational while still moving the app behind
  // it. The Pro sections navigate too, `tourActive` puts ProFeature in preview
  // mode so opening `/hoard-screen` or `/hoard-wrapped` shows the feature
  // without burning the one-week trial. Only concept steps pass `null`.
  let mainViewport = $state<HTMLElement | null>(null);

  // While the tour runs, keep the Hoard-Saves group expanded so its Library /
  // Dashboard children exist for the spotlight to land on, and flag the tour so
  // the Pro sections render in preview mode (no trial spent on the walkthrough).
  $effect(() => {
    tourActive.set(showTour);
    if (showTour) savesOpen = true;
  });

  function tourNavigate(route: string | null) {
    savesOpen = true;
    if (!route) return;
    push(route);
    // Zoom-settle the content into view so the section change reads as
    // "flying into" the area, not a hard cut. Runs after the route swap has
    // painted; skipped under reduced-motion.
    const reduce = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reduce) return;
    requestAnimationFrame(() => {
      mainViewport?.animate(
        [
          { transform: "scale(1.05) translateY(6px)", opacity: 0.5 },
          { transform: "scale(1) translateY(0)", opacity: 1 },
        ],
        { duration: 480, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
      );
    });
  }

  // Route transition: a soft fade+rise on the content viewport whenever the
  // URL changes (outside the tour, which runs its own zoom-settle). Reuses the
  // same WAAPI surface as the tour, no library, no remount, so route state
  // (scroll, focus, onMount fetches) survives.
  $effect(() => {
    router.location;
    if (!booted || showTour) return;
    const reduce = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reduce || !mainViewport) return;
    mainViewport.animate(
      [{ opacity: 0 }, { opacity: 1 }],
      { duration: 220, easing: "ease-out" },
    );
  });

  // Both sidebar toggles are derived from `$prefs` (not local state set once
  // on mount) so they always reflect the source of truth, whether the change
  // came from this sidebar's own button or from Settings' mode picker. Before,
  // picking "Solo copia de seguridad" in Settings left the sidebar Sync button
  // stuck "on" because nothing re-derived it from the updated prefs.
  let automaticMode = $derived($prefs?.automatic_mode ?? false);
  let automaticBusy = $state(false);
  let globalSync = $derived($prefs?.global_sync ?? false);
  let globalSyncBusy = $state(false);

  // Self-hosted reachability escape hatch.
  // ---------------------------------------
  // A self-hosted session (address + token) is persisted on disk + the OS
  // keyring, so it survives folder deletion AND a full reinstall. If the
  // server is gone (decommissioned, different network, never coming back) the
  // app would silently retry it forever with no obvious way out, the user
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
      replace("/onboarding/language");
    } catch (e) {
      showError(e);
    } finally {
      forgettingServer = false;
    }
  }

  // The update report is owned by `lastReport` in `stores/updates.ts`, both
  // the boot probe and the periodic re-check write to it. Reading via
  // `$lastReport` keeps this view in sync without a local mirror state.
  const updates = $derived<UpdateReport | null>($lastReport);

  // Long-running sessions need a periodic re-check; boot probe alone misses
  // releases shipped after the app was opened. The poller fires every 6h with
  // exponential backoff on failure (24h cap). Captured here so logout / unmount
  // can cancel it.
  let disposeUpdatePoller: (() => void) | null = null;

  // Hidden diagnostics unlock, 5 consecutive clicks on the sidebar version
  // string flips a session flag that reveals the Agent Diagnostics card in
  // Settings. Deliberately undocumented; only useful for triaging the silent
  // autobackup failure mode introduced before P1.4.0-0.
  let versionClicks = $state(0);
  let lastVersionClick = 0;
  function handleVersionClick() {
    const now = Date.now();
    // Reset the streak if the user pauses for >1.5s between taps. Keeps the
    // gesture deliberate, a stray double-click on idle UI shouldn't drift
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
    // The window is born hidden (`"visible": false` in tauri.conf.json) so nobody
    // sees an empty rectangle while the webview starts. Ask for it as soon as the
    // DOM is up, *before* the hydrations: the window has to appear straight away,
    // not when the disk and the cloud answer.
    //
    // Not from `requestAnimationFrame`: a hidden window never repaints, so the
    // browser parks the callback and the call never leaves. The frontend was ready
    // and the window still sat there until the backend's 8-second deadline gave up
    // on it (`window.rs`), which is most of what "the app takes forever to open"
    // was. Showing before the first paint costs nothing here anyway, because the
    // window carries `backgroundColor` (`tauri.conf.json`): what appears in that
    // gap is the app's own background, not a white flash.
    void api.uiReady().catch(() => {
      // The backend has its own grace deadline; if the call fails the window
      // appears anyway. Nothing to do here.
    });

    // Warms both possible startup destinations while the session hydrates: with a
    // session you end up on /dashboard, without one on the wizard. By the time the
    // router asks for the chunk it is already cached, so splitting by route costs
    // the common path no wait. The other screens load when the user asks for
    // them.
    const warm = (load: () => Promise<unknown>) => void load().catch(() => {});
    warm(loadDashboard);
    warm(loadLanguage);

    // Cheap OS detection so the global stylesheet can swap font-family per
    // platform without pulling `@tauri-apps/plugin-os` (not installed). The
    // Tauri WebView keeps the host UA on each platform, so this heuristic is
    // reliable enough for cosmetic tweaks. Idempotent, classList dedupes.
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
      replace("/onboarding/language");
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
        // The Rust side returns `i18n:<key>` for errors that should be shown
        // localized (e.g. the per-device free-account cap). Render the
        // translation on its own instead of wrapping it in "sign-in failed".
        if (msg.startsWith("i18n:")) {
          toastInfo($_(msg.slice(5)));
        } else {
          toastInfo($_("account.signin_failed", { values: { error: msg } }));
        }
      },
    );

    // Terminal cloud session expiry (Supabase revoked the refresh token). The
    // Rust side already cleared the session + stopped the pollers; here we calm
    // the cloud dot, tell the user, and, if they're cloud-only, route them to
    // the welcome screen so they can sign in again. A self-hosted user stays put.
    initCloudSessionWatch(() => {
      resetCloudLoop();
      toastInfo($_("account.session_expired"));
      if (!$auth.user) replace("/onboarding/language");
    });

    // The global shortcut for the HUD over the game (Alt+H out of the box). It is
    // registered from the main window because that stays alive even in the tray,
    // which is exactly when it is needed: with the game in front there is no window
    // to go back to.
    initGameOverlay();

    // Register the Tauri listeners exactly once for the lifetime of this
    // app instance. Automatic mode's detect, track and sweep work runs entirely in
    // Rust now (`commands/automatic.rs`); these listeners just mirror its
    // `automatic-phase` / `automatic-scan-complete` events into the sidebar UI.
    // Idempotent, safe to call from any onMount path.
    initAutomaticListener();

    // Hydrate the global prefs store so the sidebar toggle and any other
    // subscriber (Settings.svelte → `auto_restore`) paint with the correct
    // values the moment the boot blank lifts. `hydratePrefs` swallows its
    // own errors and falls back to pessimistic defaults.
    await hydratePrefs();
    await hydrateCardSizes();
    // `automaticMode` / `globalSync` are $derived from `$prefs` above, so they
    // populate on their own once hydratePrefs resolves, no manual seed here.

    // No startup scan kicked from here anymore: Rust's `restart_if_enabled`
    // (run during Tauri `setup()`) fires the first scan immediately when the
    // toggle was left on, and the work runs headless regardless of whether the
    // WebView is alive to catch the `automatic-phase` events. The listener
    // above will pick up phases mid-flight and the resulting tracked saves are
    // already in CliState by the time the user opens Library/Dashboard.

    // Subscribe to the live event firehose once. ActivityFeed
    // reads from the resulting stores; subscribing here (vs. in each
    // component's onMount) means a panel toggle doesn't tear down or
    // re-arm the listener and miss events in the gap.
    void subscribeLive();

    // Listen for server-pushed notifications (hoard://notification Tauri
    // event). No-op in browser dev, the listener just doesn't attach.
    void initServerNotifications();

    // Fire-and-forget update probe. The client-update check hits GitHub and
    // needs no session at all; the server half returns `null` when there's no
    // self-hosted server configured. Gating this on `$auth.user` meant a
    // user signed in only to Hoard Cloud (or fully signed out) never got the
    // desktop "update available" banner, they had to open Settings and run a
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

  /**
   * "Descargar saves" from the liberate dialog: kick off the account export
   * and send the user to Account, which owns the progress + download UI. The
   * dialog stays open, grabbing a copy first and *then* archiving is the whole
   * point of that button, so closing it would undo the user's train of thought.
   */
  async function handleLiberateDownload() {
    try {
      await exportAllCloudData();
      toastInfo($_("account.export_started"));
      push("/account");
    } catch (e) {
      showError(e);
    }
  }

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
  // is idle we show the plain automatic-mode on/off string built
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

  // Sidebar storage indicator lives in <QuotaMini /> now (footer), it unifies
  // the self-hosted and cloud sources; the old duplicate bar was removed.

  async function toggleAutomatic() {
    if (automaticBusy) return;
    automaticBusy = true;
    try {
      const next = !automaticMode;
      const updated = await api.setAutomaticMode(next);
      // Fan the updated Prefs out to every subscriber (Settings.svelte
      // watches `$prefs.auto_restore`, which `set_automatic_mode` cascades
      // to true). `automaticMode` is $derived from `$prefs`, so it re-flows
      // from this same set, no separate local assignment needed.
      prefs.set(updated);
      if (automaticMode) {
        toastSuccess($_("automatic.toggled_on"));
        // The Rust scheduler (`set_automatic_mode` → `automatic::start`) fires
        // an immediate scan tick the moment we flip the pref on, so there's
        // nothing to kick from here, the `automatic-phase` listener will show
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
  // instantly when the language changes in Settings, hard-coded English
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
  // A premium feature (Hoard-Screen / Hoard-Wrapped): navigable while the
  // server entitlement allows it, paid Pro, an active trial, or a trial not
  // yet started (opening the page is what starts the one-week clock).
  // Otherwise rendered locked with an upgrade CTA.
  type NavFeature = {
    kind: "feature";
    labelKey: string;
    icon: typeof Home;
    route: string;
    feature: FeatureKey;
  };
  type NavEntry = NavLink | NavGroup | NavFeature;

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
      /* private mode / storage disabled, toggle still works for the session */
    }
  }

  // Keep the per-feature entitlement snapshot (nav gating + tooltips for
  // Hoard-Screen / Hoard-Wrapped) in step with the cloud session: boot
  // hydrate, sign-in, sign-out and account switches all change the account
  // identity. Key the refresh on `user_id` AND `plan` so an in-session upgrade
  // (Free → Pro, when the `/v1/me` poller flips `plan`) re-pulls entitlements
  // and unlocks the feature immediately, instead of keeping the pre-upgrade
  // snapshot (which shows Pro as still locked) until the app is restarted. The
  // store caches `null` when signed out, which renders both items locked.
  let lastEntitlementsKey: string | null | undefined = undefined;
  $effect(() => {
    const key = $cloud.account
      ? `${$cloud.account.user_id}:${$cloud.account.plan}`
      : null;
    if (key === lastEntitlementsKey) return;
    lastEntitlementsKey = key;
    void refreshEntitlements();
  });

  // Click on a locked premium item. A signed-in cloud user is sent to the
  // pricing page to upgrade; a self-hosted / signed-out user is sent to
  // /account to sign in to Hoard Cloud first (no plan to upgrade yet).
  // A padlocked item ALWAYS leads to the `/pro` screen, session or no session:
  // that is where Pro is explained and, when signing in first is needed or an
  // unused trial is left, that is offered before the payment. This used to open
  // `hoard.services/pricing` in the system browser the moment there was a session,
  // so pressing a menu item threw you out of the application. `feature` only serves
  // to let the screen name what you were about to open.
  function openPremiumUpsell(feature: FeatureKey) {
    push(`/pro?feature=${feature}`);
  }

  // The first entry is the account button: "sign in" with no session at all, the
  // account view once there is one, cloud **or** self-hosted. Keying it off the
  // cloud session alone left a self-hoster staring at "sign in" for ever while
  // their backups were reaching their own server: the app was asking them to sign
  // up for the one thing they had deliberately not signed up for.
  //
  // Library, Dashboard and Map live under the collapsible Hoard-Saves group;
  // Settings stays last. The old History item was removed, since it only
  // duplicated the Dashboard.
  const navEntries = $derived<NavEntry[]>([
    $cloud.account || $auth.user
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
      ],
    },
    // Hoard-Screen (overlay) is a Cloud-only paid feature: shown (and server
    // gated) only when signed in to Hoard Cloud. Self-hosted never sees it,
    // the overlay unlocks against a Cloud entitlement it can't obtain.
    ...($cloud.account
      ? [{ kind: "feature", labelKey: "nav.hoard_screen", icon: MonitorPlay, route: "/hoard-screen", feature: "screen" } as NavEntry]
      : []),
    // Hoard-Wrapped is free for everyone (Cloud and self-hosted): a plain link,
    // no entitlement gate.
    { kind: "link", labelKey: "nav.hoard_wrapped", icon: Sparkles, route: "/hoard-wrapped" },
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
    "/logs",
    "/diagnostics",
    "/account",
    "/hoard-screen",
    "/hoard-wrapped",
    "/pro",
  ];
  const isAppRoute = $derived(
    APP_ROUTE_PREFIXES.some((p) => router.location.startsWith(p)),
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
      class="sidebar-glass flex {narrow ? 'is-narrow' : 'w-60'} shrink-0 flex-col border-r border-white/[0.08] bg-gradient-to-b from-zinc-950/70 via-zinc-950/45 to-zinc-900/30 backdrop-blur-xl shadow-[inset_-1px_0_0_0_rgba(255,255,255,0.06)]"
    >
      <div class="flex items-center gap-2 px-4 py-4">
        <Logo size={36} class="shrink-0 rounded-lg" />
        <div class="hide-narrow min-w-0 flex-1">
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
          class="hide-narrow flex h-7 w-7 shrink-0 items-center justify-center rounded-md border transition-colors {$prefs?.live_activity_visible ?? true
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
            class="hide-narrow flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-amber-500/40 bg-amber-500/10 text-amber-300 transition-colors hover:bg-amber-500/20"
          >
            <AlertCircle size={14} />
          </button>
        {/if}

      </div>

      <!-- Shared button markup for top-level links and indented group
           children, so the two render paths can't drift apart. -->
{#snippet navLink(item: NavLink, indented: boolean)}
        {@const active = router.location === item.route}
          <button
            type="button"
            data-tour-route={item.route}
            aria-label={$_(item.labelKey)}
            title={$_(item.labelKey)}
            aria-current={active ? "page" : undefined}
            onclick={() => push(item.route)}
            use:glow
            use:tilt
            class="glow tilt group flex w-full items-center gap-3 rounded-md border-l-2 py-2 text-sm transition-colors duration-150 {indented ? 'pl-9 pr-3' : 'px-3'} {active ? 'border-emerald-500 bg-zinc-800/50 text-zinc-50' : 'border-transparent text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100'}"
          >
            <item.icon size={indented ? 16 : 18} />
            <span class="hide-narrow">{$_(item.labelKey)}</span>
          </button>
{/snippet}

      <nav class="flex-1 space-y-1 px-3 py-2">
        {#each navEntries as entry (entry.kind === "group" ? entry.id : entry.route)}
          {#if entry.kind === "link"}
            {@render navLink(entry, false)}
          {:else if entry.kind === "feature"}
            {@const active = router.location === entry.route}
            <!-- The per-feature server entitlement decides; `PRO_DEV_UNLOCK` is
                 the owner's local test override, never set in public builds.
                 `trial_available` stays navigable: the trial only starts when
                 the user actually opens the page (first look), never from
                 here. -->
            {@const fs = $entitlements?.features[entry.feature] ?? null}
            {#if PRO_DEV_UNLOCK || featureUnlocked(fs) || fs?.state === "trial_available"}
              <button
                type="button"
                data-tour-route={entry.route}
                aria-label={$_(entry.labelKey)}
                aria-current={active ? "page" : undefined}
                onclick={() => push(entry.route)}
                use:glow
                use:tilt
                title={fs?.state === "trial"
                  ? $_("nav.trial_days_left", { values: { n: featureDaysLeft(fs) } })
                  : fs?.state === "trial_available"
                    ? $_("pro.trial_available", { values: { n: fs.days } })
                    : undefined}
                class="glow tilt group flex w-full items-center gap-3 rounded-md border-l-2 px-3 py-2 text-sm transition-colors duration-150
                  {active
                  ? 'border-emerald-500 bg-zinc-800/50 text-zinc-50'
                  : 'border-transparent text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-100'}"
              >
                <entry.icon size={18} />
                <span>{$_(entry.labelKey)}</span>
              </button>
            {:else}
              <!-- Locked: neutral/zinc styling (not amber) + lock glyph. Click
                   routes to upgrade/sign-in instead of the gated view. -->
              <button
                type="button"
                data-tour-route={entry.route}
                aria-label={$_(entry.labelKey)}
                title={$cloud.account
                  ? $_("nav.locked_pro")
                  : $_("nav.locked_signin")}
                onclick={() => openPremiumUpsell(entry.feature)}
                class="group flex w-full items-center gap-3 rounded-md border-l-2 border-transparent px-3 py-2 text-sm text-zinc-500 transition-colors duration-150 hover:bg-zinc-800/40 hover:text-zinc-300"
              >
                <entry.icon size={18} class="opacity-70" />
                <span class="flex-1 text-left">{$_(entry.labelKey)}</span>
                <Lock size={14} class="shrink-0 opacity-70" />
              </button>
            {/if}
          {:else}
            <!-- A child on the active route forces the group open even if the
                 user had collapsed it, so the highlight is never hidden. -->
            {@const childActive = entry.children.some(
              (c) => router.location === c.route,
            )}
            {@const open = savesOpen || childActive}
            <button
              type="button"
              data-tour="saves"
              aria-expanded={open}
              aria-label={$_(entry.labelKey)}
              onclick={toggleSaves}
              use:glow
              class="glow group flex w-full items-center gap-3 rounded-md border-l-2 border-transparent px-3 py-2 text-sm text-zinc-400 transition-colors duration-150 hover:bg-zinc-800/40 hover:text-zinc-100"
            >
              <entry.icon size={18} />
              <span class="hide-narrow flex-1 text-left">{$_(entry.labelKey)}</span>
              <ChevronDown
                size={16}
                class="hide-narrow shrink-0 transition-transform duration-150 {open
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
        <!-- Compact storage bar (same colour scale as the dashboard's
             QuotaBar). Hidden until auth hydrates. -->
        {#if !narrow}<QuotaMini />{/if}
        <!-- Cloud account chip: avatar + plan (+ "Mejorar plan" when on Free),
             routing to /account. The signed-out "Iniciar sesión" entry now
             lives at the top of the nav, so we only render this when signed
             in to avoid a duplicate button. -->
        {#if $cloud.hydrated && $cloud.account && !narrow}
            <div class="flex items-center gap-2">
              <button
                type="button"
                onclick={() => push("/account")}
                use:glow
                use:tilt
                class="glow tilt flex min-w-0 flex-1 items-center gap-2 rounded-md border border-zinc-800 bg-zinc-900/60 px-2 py-1.5 text-left transition-colors hover:border-zinc-700 hover:bg-zinc-800/60"
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
                  onclick={() => push("/pro")}
                  class="flex shrink-0 items-center gap-1 rounded-md bg-gradient-to-r from-emerald-400 to-teal-400 px-2.5 py-2 text-[11px] font-semibold text-emerald-950 shadow-sm shadow-emerald-500/30 transition-all hover:from-emerald-300 hover:to-teal-300 hover:shadow-emerald-500/50"
                  title={$_("sidebar.upgrade_tooltip")}
                >
                  <Sparkles size={12} />
                  {$_("sidebar.upgrade")}
                </button>
              {/if}
            </div>
        {/if}
        <button
          type="button"
          onclick={toggleGlobalSync}
          disabled={globalSyncBusy}
          use:glow
          use:tilt
          class="glow tilt flex w-full items-center justify-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors {globalSync
            ? 'border-emerald-500/40 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'
            : 'border-rose-500/40 bg-rose-500/15 text-rose-300 hover:bg-rose-500/25'} disabled:cursor-wait disabled:opacity-60"
          aria-label={$_("sync.aria_toggle")}
          title={$_("sync.help_tooltip")}
        >
          <RefreshCw size={16} class={globalSync ? "animate-pulse" : ""} />
          <span class="hide-narrow">
            {$_("sync.title")} ·
            {globalSync ? $_("sync.on") : $_("sync.off")}
          </span>
        </button>
        <button
          type="button"
          data-tour="automatic"
          onclick={toggleAutomatic}
          disabled={automaticBusy}
          use:glow
          use:tilt
          class="glow tilt flex w-full items-center justify-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors {automaticMode
            ? 'border-emerald-500/40 bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'
            : 'border-rose-500/40 bg-rose-500/15 text-rose-300 hover:bg-rose-500/25'} disabled:cursor-wait disabled:opacity-60"
          aria-label={$_("automatic.aria_toggle")}
          title={$_("automatic.help_tooltip")}
        >
          <Sparkles
            size={16}
            class={$automaticState.kind === "idle" ? "" : "animate-pulse"}
          />
          <span class="hide-narrow">
            {#if automaticPhaseLabel}
              {automaticPhaseLabel}
            {:else}
              {$_("automatic.title")} ·
              {automaticMode ? $_("automatic.on") : $_("automatic.off")}
            {/if}
          </span>
        </button>
        <p class="hide-narrow px-1 text-[11px] leading-tight text-zinc-500">
          {automaticMode
            ? $_("automatic.subtitle_on")
            : $_("automatic.subtitle_off")}
        </p>
      </div>
    </aside>

    <main class="min-w-0 flex-1 overflow-y-auto" data-tour="content">
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
      <!-- Zoom-settle target for the guided tour (see `tourNavigate`). The
           wrapper persists across route swaps so the WAAPI animation can play
           on it without remounting the routed component. -->
      <div bind:this={mainViewport} class="h-full">
        <Router {routes} />
      </div>
    </main>
  </div>

  <!-- Top-right overlay: eye (live status) + bell (notifications), fixed to
       the top-right corner of the app window. Above both the sidebar and the
       content area. The eye opens a dropdown with machines + running games;
       the bell opens the notifications panel (server + app messages). -->
  <div class="pointer-events-none fixed right-8 top-3 z-[60] flex items-center gap-2">
    <button
      type="button"
      onclick={toggleNotif}
      aria-label={$_("notifications.title")}
      title={$_("notifications.title")}
      aria-expanded={notifOpen}
      class="pointer-events-auto relative flex h-8 w-8 items-center justify-center rounded-lg border transition-colors {notifOpen
        ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
        : 'border-white/[0.08] bg-zinc-950/60 text-zinc-400 backdrop-blur-md hover:text-zinc-100'}"
    >
      <Bell size={15} />
      {#if $notifStore.length > 0}
        <span class="absolute -right-1 -top-1 flex h-3 min-w-3 items-center justify-center rounded-full bg-emerald-500 px-1 text-[9px] font-bold text-emerald-950 ring-2 ring-zinc-950">
          {$notifStore.length}
        </span>
      {/if}
    </button>
    <button
      type="button"
      onclick={toggleEye}
      aria-label={$_("eye.title")}
      title={$_("eye.title")}
      aria-expanded={eyeOpen}
      class="pointer-events-auto flex h-8 w-8 items-center justify-center rounded-lg border transition-colors {eyeOpen
        ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
        : 'border-white/[0.08] bg-zinc-950/60 text-zinc-400 backdrop-blur-md hover:text-zinc-100'}"
    >
      <Eye size={15} />
    </button>
  </div>

  <!-- Eye dropdown — anchored below the eye button, top-right. -->
  {#if eyeOpen}
    <div
      class="fixed right-8 top-14 z-[61] w-72 overflow-hidden rounded-xl border border-white/[0.08] bg-zinc-950/95 shadow-xl backdrop-blur-xl"
      transition:fly={{ y: -8, duration: 180 }}
    >
      <EyePanel {now} />
    </div>
  {/if}

  <!-- Notifications dropdown — anchored below the bell, top-right. -->
  {#if notifOpen}
    <NotificationsPanel />
  {/if}
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

<!-- Por encima de todo lo demás, y montado siempre: es quien pregunta al
     servicio en qué punto va la actualización y quien tapa la pantalla cuando
     ya no hay nada útil que hacer debajo (plazo vencido, instalación en curso,
     o una ventana que se quedó atrás del servicio que ya se relevó). -->
<UpdateGate />

<!-- "Liberar espacio" lives at the shell level, not inside Account: it's needed
     the moment an upload bounces off a full account, and that happens while the
     user is on Library, Dashboard or nowhere at all. -->
<LiberateStorageModal
  open={$liberateOpen}
  onClose={closeLiberate}
  onDownload={handleLiberateDownload}
  onDone={() => void refreshCloud()}
/>

<!-- Los dos diálogos de plan: el agradecimiento al pagar Pro y la despedida al
     cancelarlo. Cada uno se ve UNA vez (el marcador vive en disco, por cuenta:
     `stores/planEvents.ts`) y en mitad de la aplicación, porque lo que los
     dispara pasa fuera —en el navegador, en Polar— y aquí sólo llega en el
     siguiente `/v1/me`. Sólo sobre las rutas de la aplicación: en mitad del
     asistente de alta no hay dónde volver, y con la cuenta congelada por
     borrado manda su pantalla. -->
{#if isAppRoute && !$cloud.account?.deleted_at}
  <ProThanksModal open={$planEvent === "thanks"} onClose={dismissPlanEvent} />
  <ProFarewellModal open={$planEvent === "farewell"} onClose={dismissPlanEvent} />
{/if}

<!-- Bottom-right stack. The storage banner rides above the activity panel and
     survives it being closed: a full account is the one state that has to stay
     on screen, since nothing gets backed up until it clears. -->
{#if isAppRoute}
  <div
    class="pointer-events-none fixed bottom-4 right-4 z-40 flex w-[min(22rem,calc(100vw-2rem))] flex-col items-stretch gap-2"
  >
    <StorageFullBanner />
    {#if $prefs?.live_activity_visible ?? true}
      <ActivityFeed onClose={() => toggleActivityFeed()} />
    {/if}
  </div>
{/if}

<Toaster />

<!-- Account scheduled for deletion: a blocking screen over everything (incl. the
     tour). The account is frozen server-side, so the app behind is dead until
     the user reactivates or signs out. -->
{#if $cloud.hydrated && $cloud.account?.deleted_at}
  <AccountDeletedModal />
{:else if showTour}
  <TourOverlay onClose={finishTour} navigate={tourNavigate} />
{/if}
