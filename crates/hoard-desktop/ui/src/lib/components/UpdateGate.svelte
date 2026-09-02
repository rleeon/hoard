<script lang="ts">
  /**
   * The "updating Hoard" screen: what makes the app update **when you open it**,
   * the way Steam or Discord do.
   *
   * It only appears when there is no alternative, and there are three distinct
   * reasons:
   *
   * 1. **The deadline ran out** (`mandatory`). The service has spent two days
   *    trying to apply something that needs somebody present (a `.deb` that wants
   *    polkit, a `.dmg` that wants a hand) and here is that somebody. It cannot be
   *    closed: it is the step the deadline exists to provoke.
   * 2. **The service already updated and this window fell behind.** That happens by
   *    design: the service swaps the binaries quietly and restarts, but it cannot
   *    touch an open window. Without this notice the user stays on the old version
   *    until they close the app themselves.
   * 3. **It is being applied right now.** It does not block for the fun of it: the
   *    binaries are being replaced underneath, and letting somebody keep clicking in
   *    an app whose engine is restarting only produces errors that mean nothing.
   *
   * What it does **not** do: appear every time a version ships. The normal case is
   * silent (the service downloads and applies, and the user finds out from the
   * version number); the "something is ready and it is optional" case is the usual
   * amber badge in the sidebar. A modal per release would be the opposite
   * problem.
   */
  import { onMount, onDestroy } from "svelte";
  import { _ } from "svelte-i18n";
  import { Loader2, Download, RefreshCw } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";

  import {
    applyStagedUpdate,
    fetchServiceUpdate,
    serviceUpdate,
    windowIsBehind,
    type UpdateState,
  } from "../stores/updates";
  import { APP_VERSION } from "../version";

  /** How often it asks again while the gate is on screen. Short on purpose: the
   *  user is staring at a bar that does not move. */
  const TICK_MS = 2_000;

  let timer: ReturnType<typeof setInterval> | null = null;
  let working = $state(false);
  let failed = $state<string | null>(null);

  const svc = $derived<UpdateState | null>($serviceUpdate);
  const phase = $derived(svc?.phase.phase ?? "up_to_date");

  /** This window is running an older binary than the service's. */
  const behind = $derived(windowIsBehind(svc, APP_VERSION));

  /** The service is touching the binaries right now. */
  const busy = $derived(phase === "applying" || phase === "restarting");

  /** Is anything shown? */
  const visible = $derived(
    !!svc && (svc.mandatory || behind || busy || working),
  );

  /** Can it be closed? Only the "it already updated, restart whenever you like"
   *  case takes a no; a passed deadline and an install in progress do not. */
  const dismissible = $derived(behind && !svc?.mandatory && !busy);
  let dismissed = $state(false);

  const showing = $derived(visible && !(dismissible && dismissed));

  onMount(() => {
    void nudgeOnOpen();
    timer = setInterval(() => void fetchServiceUpdate(), TICK_MS);
  });

  /**
   * "Update it when it opens", literally.
   *
   * Almost always there is nothing to do here: the service applied the update before
   * anybody opened anything, so this window **already is** the new one. This covers
   * the gap where the service had it downloaded and was waiting for its next cycle,
   * or waiting for you to close a game. Opening the app is the signal that now is a
   * good moment.
   *
   * Only when it applies itself (`unattended`): no dialogs, no privileges, nothing
   * to approve. The route that needs a human is offered, not fired, and when the
   * deadline passes it is this same component that covers the screen and asks.
   */
  async function nudgeOnOpen() {
    const s = await fetchServiceUpdate();
    if (!s || !s.unattended || !s.staged) return;
    if (s.phase.phase !== "ready" && s.phase.phase !== "waiting") return;
    try {
      await applyStagedUpdate(s.staged);
    } catch (e) {
      // Silent on purpose: nobody asked for this, so nobody deserves an error over
      // it. The service's background cycle retries.
      console.warn("update nudge on open failed:", e);
    }
  }

  onDestroy(() => {
    if (timer) clearInterval(timer);
    timer = null;
  });

  async function install() {
    working = true;
    failed = null;
    try {
      await applyStagedUpdate(svc?.latest ?? undefined);
    } catch (e) {
      // The common case here is not the updater failing: it is the user cancelling
      // the privilege dialog. It is said, and a retry is offered, rather than
      // leaving the screen spinning for ever.
      failed = e instanceof Error ? e.message : String(e);
      working = false;
    }
  }

  async function restartApp() {
    try {
      await invoke("restart_app");
    } catch (e) {
      console.warn("relaunch failed:", e);
      failed = e instanceof Error ? e.message : String(e);
    }
  }

  /** The headline, which is the only part almost nobody will read in full. */
  const title = $derived(
    busy || working
      ? $_("update_gate.installing_title")
      : behind
        ? $_("update_gate.restart_title")
        : $_("update_gate.required_title"),
  );

  const body = $derived(
    busy || working
      ? $_("update_gate.installing_body")
      : behind
        ? $_("update_gate.restart_body", {
            values: { version: svc?.current ?? "" },
          })
        : $_("update_gate.required_body", {
            values: { version: svc?.latest ?? "" },
          }),
  );
</script>

{#if showing}
  <!-- Por encima de todo, incluidos los modales: si esto está en pantalla es
       porque nada de lo de debajo puede seguir su curso. -->
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-zinc-950/95 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="update-gate-title"
  >
    <div class="w-full max-w-md px-8 text-center">
      <div class="mb-6 flex justify-center">
        {#if busy || working}
          <Loader2 class="h-10 w-10 animate-spin text-emerald-500" />
        {:else if behind}
          <RefreshCw class="h-10 w-10 text-emerald-500" />
        {:else}
          <Download class="h-10 w-10 text-amber-500" />
        {/if}
      </div>

      <h1
        id="update-gate-title"
        class="mb-3 text-xl font-semibold text-zinc-100"
      >
        {title}
      </h1>
      <p class="mb-8 text-sm leading-relaxed text-zinc-400">{body}</p>

      {#if failed}
        <p
          class="mb-6 rounded-md border border-red-500/40 bg-red-500/10 px-4 py-3 text-left text-xs text-red-300"
        >
          {failed}
        </p>
      {/if}

      {#if !busy && !working}
        <div class="flex flex-col gap-3">
          {#if behind}
            <button
              class="w-full rounded-md bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-500"
              onclick={restartApp}
            >
              {$_("update_gate.restart_now")}
            </button>
            {#if dismissible}
              <button
                class="w-full rounded-md px-4 py-2.5 text-sm text-zinc-400 transition hover:text-zinc-200"
                onclick={() => (dismissed = true)}
              >
                {$_("update_gate.later")}
              </button>
            {/if}
          {:else}
            <button
              class="w-full rounded-md bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-500"
              onclick={install}
            >
              {$_("update_gate.install_now")}
            </button>
            <p class="text-xs text-zinc-500">
              {$_("update_gate.privileges_hint")}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
