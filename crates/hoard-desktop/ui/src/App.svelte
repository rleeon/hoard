<script lang="ts">
  import Router, { push, replace, location } from "svelte-spa-router";
  import { onMount, onDestroy } from "svelte";
  import {
    Archive,
    Library,
    History,
    Settings as SettingsIcon,
    Sparkles,
    AlertCircle,
  } from "lucide-svelte";
  import { _ } from "svelte-i18n";

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

  import Toaster from "./lib/components/Toaster.svelte";
  import UpdateConfirmModal from "./lib/components/UpdateConfirmModal.svelte";
  import { auth, hydrateAuth } from "./lib/stores/auth";
  import { loadStep, routeForStep } from "./lib/stores/onboarding";
  import { runMagicSetup, magicState } from "./lib/stores/magic";
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
    "/history/:saveId": HistoryRoute,
    "/logs": LogsRoute,
    "/diagnostics": DiagnosticsRoute,
  };

  let booted = $state(false);
  let updateModalOpen = $state(false);

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
    await hydrateAuth();
    if ($auth.user) {
      replace("/dashboard");
    } else {
      const step = await loadStep();
      replace(routeForStep(step));
    }
    booted = true;

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
  });

  function magicLabel(s: typeof $magicState): string {
    switch (s.kind) {
      case "idle":
        return $_("magic.idle");
      case "detecting":
        return $_("magic.detecting");
      case "tracking":
        return $_("magic.tracking", {
          values: { done: s.done, total: s.total },
        });
      case "starting_agent":
        return $_("magic.starting_agent");
    }
  }

  // The `labelKey` is resolved through `$_()` at render time so the sidebar
  // re-translates instantly when the user switches language in Settings —
  // hard-coded English here was the long-standing reason German/Spanish UIs
  // still showed "Library / Dashboard …" in the rail.
  const sidebarItems = [
    { labelKey: "nav.library", icon: Library, route: "/library" },
    { labelKey: "nav.dashboard", icon: Archive, route: "/dashboard" },
    { labelKey: "nav.history", icon: History, route: "/history" },
    { labelKey: "nav.settings", icon: SettingsIcon, route: "/settings" },
  ];

  // App-shell routes share the persistent sidebar; wizard routes own the
  // viewport. Keep this list in sync with `sidebarItems` above.
  const APP_ROUTE_PREFIXES = [
    "/dashboard",
    "/library",
    "/settings",
    "/history",
    "/logs",
    "/diagnostics",
  ];
  const isAppRoute = $derived(
    APP_ROUTE_PREFIXES.some((p) => $location.startsWith(p)),
  );
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
      class="flex w-60 shrink-0 flex-col border-r border-zinc-800 bg-zinc-950"
    >
      <div class="flex items-center gap-2 px-5 py-5">
        <div
          class="flex h-9 w-9 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/40"
        >
          <Archive size={20} />
        </div>
        <div class="min-w-0 flex-1">
          <div class="text-base font-semibold tracking-tight">Hoard</div>
          <button
            type="button"
            onclick={handleVersionClick}
            class="cursor-default select-none text-left text-xs text-zinc-500 outline-none"
            tabindex="-1"
            aria-hidden="true"
          >
            v{import.meta.env.VITE_HOARD_VERSION || "1.5.2"}
          </button>
        </div>
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
        {#each sidebarItems as item (item.labelKey)}
          {@const active = $location === item.route}
          {@const enabled =
            item.route === "/dashboard" ||
            item.route === "/library" ||
            item.route === "/settings"}
          <button
            type="button"
            disabled={!enabled}
            onclick={() => push(item.route)}
            class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors
              {active
              ? 'bg-zinc-800 text-zinc-50'
              : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100'}
              disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-zinc-400"
            title={enabled ? undefined : $_("nav.coming_later")}
          >
            <item.icon size={18} />
            <span>{$_(item.labelKey)}</span>
          </button>
        {/each}
      </nav>

      <!-- Sidebar footer: Magic auto-setup button. The "Update available"
           alert lives next to the version up top now, not here. -->
      <div class="border-t border-zinc-800 px-3 py-3 space-y-2">
        <button
          type="button"
          onclick={runMagicSetup}
          disabled={$magicState.kind !== "idle"}
          class="flex w-full items-center justify-center gap-2 rounded-md bg-emerald-600 px-3 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-emerald-500 disabled:cursor-wait disabled:bg-emerald-600/60"
          title={$_("magic.tooltip")}
        >
          <Sparkles size={16} />
          <span>{magicLabel($magicState)}</span>
        </button>

        <p class="px-1 text-[11px] leading-tight text-zinc-500">
          {$_("magic.subtitle")}
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

<Toaster />
