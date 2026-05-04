<script lang="ts">
  /**
   * Settings — every persistent app preference lives here.
   *
   * Each row is its own atomic toggle: flipping a switch writes to disk
   * immediately, so users don't have to hunt for a Save button. The "danger"
   * actions (clear cache, sign out) sit at the bottom in a separate card so
   * they can't be hit by reflex.
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import {
    Power,
    BellRing,
    BellOff,
    Minimize2,
    LogIn,
    LogOut,
    Trash2,
    Info,
    FileText,
    ChevronRight,
    BarChart3,
  } from "lucide-svelte";

  import Card from "../lib/components/Card.svelte";
  import Button from "../lib/components/Button.svelte";
  import SettingsRow from "../lib/components/SettingsRow.svelte";
  import { prefs, hydratePrefs, updatePrefs } from "../lib/stores/prefs";
  import { auth, signOut } from "../lib/stores/auth";
  import * as api from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  let saving = $state<string | null>(null);
  let signingOut = $state(false);

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
  });

  async function toggle(field: keyof api.Prefs, value: boolean) {
    if (!$prefs) return;
    saving = field;
    try {
      if (field === "autostart") {
        const actual = await api.setAutostart(value);
        if (actual !== value) {
          toastError(
            "The OS rejected the autostart change — try enabling autostart from your launcher settings.",
          );
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
      toastSuccess("Signed out.");
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

  const generalRows: Row[] = [
    {
      field: "close_to_tray",
      label: "Close to tray",
      description:
        "Closing the window keeps Hoard running in the background so backups continue. Quit from the tray icon to stop it.",
      icon: Minimize2,
    },
  ];

  const startupRows: Row[] = [
    {
      field: "autostart",
      label: "Start Hoard at login",
      description:
        "Adds Hoard to your launcher. Pair with “Start minimised” for a quiet boot.",
      icon: LogIn,
    },
    {
      field: "start_minimised",
      label: "Start minimised",
      description:
        "Launch with the window hidden — only the tray icon shows up.",
      icon: Power,
    },
  ];

  const privacyRows: Row[] = [
    {
      field: "anonymous_telemetry",
      label: "Send anonymous usage pings",
      description:
        "Help us see how many people are using Hoard. We only count events (successful backup, restore, first-run completed) — never your username, game names, save paths, file contents, or tokens. Off by default.",
      icon: BarChart3,
    },
  ];

  const notifyRows: Row[] = [
    {
      field: "notify_on_success",
      label: "Notify on successful backup",
      description:
        "A small desktop notification when a backup uploads successfully. Turn this off once you trust the agent.",
      icon: BellRing,
    },
    {
      field: "notify_on_failure",
      label: "Notify on backup failure",
      description:
        "Alerts you when a backup fails, even after retries. We strongly recommend leaving this on.",
      icon: BellOff,
    },
  ];
</script>

<div class="mx-auto max-w-3xl px-8 py-8">
  <header class="mb-6">
    <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>
    <p class="mt-1 text-sm text-zinc-400">
      Tweak how Hoard behaves on this machine. Changes are saved as soon as
      you toggle them.
    </p>
  </header>

  {#if !$prefs}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">Loading…</div>
    </Card>
  {:else}
    <div class="space-y-6">
      <section>
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          General
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
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          Startup
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
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          Notifications
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
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          Privacy
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
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          Account
        </h2>
        <Card>
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex-1">
              <p class="text-sm text-zinc-100">
                {#if $auth.user}
                  Signed in as
                  <span class="font-medium">{$auth.user.username}</span>
                {:else}
                  Not signed in
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
              Sign out
            </Button>
          </div>
        </Card>
      </section>

      <section>
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          Advanced
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
                <p class="text-sm font-medium text-zinc-100">View logs</p>
                <p class="mt-0.5 text-xs text-zinc-500">
                  Inspect the agent's activity log. Handy for bug reports.
                </p>
              </div>
            </div>
            <ChevronRight size={16} class="shrink-0 text-zinc-500" />
          </button>
        </Card>
      </section>

      <section>
        <h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
          About
        </h2>
        <Card>
          <div class="flex items-start gap-3 text-sm text-zinc-300">
            <Info size={16} class="mt-0.5 shrink-0 text-zinc-500" />
            <div>
              <p>Hoard 0.2.0 — self-hosted save sync.</p>
              <p class="mt-1 text-xs text-zinc-500">
                Your server, your data. Source code & docs at hoard.dev.
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

