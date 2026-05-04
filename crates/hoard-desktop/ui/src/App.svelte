<script lang="ts">
  import Router, { push, replace, location } from "svelte-spa-router";
  import { onMount } from "svelte";
  import { Archive, Library, History, Settings as SettingsIcon } from "lucide-svelte";

  import Welcome from "./routes/Welcome.svelte";
  import ServerSetup from "./routes/ServerSetup.svelte";
  import TokenSetup from "./routes/TokenSetup.svelte";
  import OnboardingDone from "./routes/OnboardingDone.svelte";
  import Dashboard from "./routes/Dashboard.svelte";

  import Toaster from "./lib/components/Toaster.svelte";
  import { auth, hydrateAuth } from "./lib/stores/auth";
  import { loadStep, routeForStep } from "./lib/stores/onboarding";

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
  };

  let booted = $state(false);

  onMount(async () => {
    await hydrateAuth();
    if ($auth.user) {
      replace("/dashboard");
    } else {
      const step = await loadStep();
      replace(routeForStep(step));
    }
    booted = true;
  });

  const sidebarItems = [
    { label: "Library", icon: Library, route: "/library" },
    { label: "Dashboard", icon: Archive, route: "/dashboard" },
    { label: "History", icon: History, route: "/history" },
    { label: "Settings", icon: SettingsIcon, route: "/settings" },
  ];

  const isAppRoute = $derived($location.startsWith("/dashboard"));
</script>

{#if !booted}
  <!--
    Tiny boot blank. We keep the markup minimal so users on slow disks don't
    see a flash of the welcome screen before the auth check completes.
  -->
  <div class="flex h-full items-center justify-center bg-zinc-950">
    <div
      class="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-amber-500"
    ></div>
  </div>
{:else if isAppRoute}
  <div class="flex h-full">
    <aside
      class="flex w-60 shrink-0 flex-col border-r border-zinc-800 bg-zinc-950"
    >
      <div class="flex items-center gap-2 px-5 py-5">
        <div
          class="flex h-9 w-9 items-center justify-center rounded-lg bg-amber-500/10 text-amber-500 ring-1 ring-amber-500/40"
        >
          <Archive size={20} />
        </div>
        <div>
          <div class="text-base font-semibold tracking-tight">Hoard</div>
          <div class="text-xs text-zinc-500">v0.2.0-dev</div>
        </div>
      </div>

      <nav class="flex-1 space-y-1 px-3 py-2">
        {#each sidebarItems as item (item.label)}
          {@const active = $location === item.route}
          <button
            type="button"
            disabled={item.route !== "/dashboard"}
            onclick={() => push(item.route)}
            class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors
              {active
              ? 'bg-zinc-800 text-zinc-50'
              : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100'}
              disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-zinc-400"
            title={item.route === "/dashboard" ? undefined : "Coming in a later phase"}
          >
            <item.icon size={18} />
            <span>{item.label}</span>
          </button>
        {/each}
      </nav>

      <div class="border-t border-zinc-800 px-5 py-4 text-xs text-zinc-500">
        Self-hosted save sync.<br />
        Your server, your data.
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

<Toaster />
