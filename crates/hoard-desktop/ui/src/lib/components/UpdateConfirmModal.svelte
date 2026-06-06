<script lang="ts">
  /**
   * Confirmation dialog for the in-app update flow.
   *
   * The sidebar shows a small amber "alert" button when GitHub reports a
   * newer release. Clicking it opens this modal:
   *   - "Se necesita actualizar" + a one-line context line.
   *   - Sí (green primary): downloads the asset and launches the OS installer.
   *     For server updates we don't auto-run anything (the server doesn't
   *     self-update); we just show the `hoard-server upgrade` command.
   *   - No (red secondary): closes.
   *
   * The component owns its own "installing" lifecycle so the parent doesn't
   * have to thread loading state through props.
   */
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Loader2 } from "lucide-svelte";
  import Modal from "./Modal.svelte";
  import {
    applyDesktopUpdate,
    applyServerUpdate,
    checkForUpdates,
    type UpdateReport,
  } from "../stores/updates";
  import { toastInfo, toastSuccess } from "../stores/toasts";
  import { showError } from "../stores/error_dialog";
  import { auth } from "../stores/auth";

  type Props = {
    open: boolean;
    report: UpdateReport | null;
    onClose: () => void;
  };
  let { open, report, onClose }: Props = $props();

  // We don't depend on `@tauri-apps/plugin-os` (not installed in this
  // workspace as of 1.5.3). The desktop WebView's UA string always carries
  // the host OS family — that's good enough to gate a Linux-only pkexec
  // call. Hydrated in `onMount` to keep SSR-safe even though Tauri runs
  // CSR exclusively.
  let platformName = $state("");
  onMount(() => {
    const ua =
      typeof navigator !== "undefined" ? navigator.userAgent : "";
    if (/Linux/i.test(ua)) platformName = "linux";
    else if (/Windows/i.test(ua)) platformName = "windows";
    else if (/Mac OS X|Macintosh/i.test(ua)) platformName = "macos";
    else platformName = "";
  });

  // We treat the modal as serving one of two purposes:
  //   - clientUpdate: the desktop app itself is behind → green Sí = install.
  //   - serverUpdate: the user's self-hosted server is behind → green Sí just
  //     copies `hoard-server upgrade` to clipboard, because we have no remote
  //     execute story (and the user said the server must not self-update).
  // If both are available we show the client first; user can re-open later.
  const isClient = $derived(report?.client.available === true);
  const isServer = $derived(
    !isClient && report?.server?.available === true,
  );

  const targetVersion = $derived(
    isClient
      ? (report?.client.latest ?? "?")
      : (report?.server?.latest ?? "?"),
  );
  const currentVersion = $derived(
    isClient
      ? (report?.client.current ?? "?")
      : (report?.server?.current ?? "?"),
  );

  // The pkexec-based in-app server upgrade only makes sense when the
  // server lives on this same Linux box; for remote servers we keep the
  // copy-command fallback so the user pastes it over SSH. Mirrors the
  // gate `handleServerInAppUpgrade` uses in Settings.svelte.
  const canRunServerUpgrade = $derived(
    $auth.user?.is_local_server === true && platformName === "linux",
  );

  let installing = $state(false);
  let busy = $state(false);

  async function onYes() {
    if (installing) return;
    if (isClient) {
      installing = true;
      try {
        const r = await applyDesktopUpdate(report?.client.latest ?? undefined);
        if (r.kind === "superseded") {
          // A newer release landed since the modal opened. We didn't install
          // the stale one — refresh the report so the badge/modal now point at
          // the latest, and tell the user to confirm again for that version.
          toastInfo(
            $_("updates.superseded", { values: { latest: r.latest } }),
          );
          await checkForUpdates().catch(() => {});
        } else if (r.kind === "installer_launched") {
          toastInfo($_("updates.installer_launched"));
        } else {
          toastInfo(
            $_("updates.downloaded_manual", { values: { path: r.path } }),
          );
        }
        onClose();
      } catch (e) {
        // `apply_desktop_update` returns `Result<_, AppError>` since 1.5.3 —
        // the invoke bridge serialises that to `{ title, body, detail }`.
        // `showError` also accepts raw strings as a retrocompat fallback for
        // any pre-migration code path that still throws plain text.
        showError(e);
        onClose();
      } finally {
        installing = false;
      }
    } else if (isServer) {
      if (canRunServerUpgrade) {
        await runServerUpgrade();
      } else {
        await copyServerCommand();
      }
    } else {
      onClose();
    }
  }

  /**
   * Local Linux server: drive the in-app `pkexec hoard-server upgrade`
   * flow. Mirrors `handleServerInAppUpgrade` in Settings.svelte but
   * routes failures through the structured `showError` dialog (which is
   * what `apply_server_update` returns since P-D153-1).
   */
  async function runServerUpgrade() {
    if (busy) return;
    busy = true;
    try {
      const outcome = await applyServerUpdate();
      if (outcome.kind === "upgraded_and_restarted") {
        toastSuccess($_("updates.server_upgrade_success"));
      } else if (outcome.kind === "upgraded") {
        toastInfo($_("updates.server_upgrade_partial"));
      }
      onClose();
    } catch (e) {
      showError(e);
      onClose();
    } finally {
      busy = false;
    }
  }

  /** Remote server (or non-Linux client): keep the copy-command fallback. */
  async function copyServerCommand() {
    try {
      await navigator.clipboard.writeText("sudo hoard-server upgrade");
      toastSuccess($_("updates.server_command_copied"));
    } catch {
      toastInfo($_("updates.server_command_manual"));
    }
    onClose();
  }
</script>

<Modal
  {open}
  title={$_("updates.confirm_title")}
  dismissible={!installing}
  {onClose}
>
  <div class="space-y-3 text-sm text-zinc-300">
    <p>{$_("updates.needs_update")}</p>
    {#if isClient}
      <p class="text-zinc-400">
        {$_("updates.client_body", {
          values: { current: currentVersion, latest: targetVersion },
        })}
      </p>
    {:else if isServer}
      <p class="text-zinc-400">
        {$_("updates.server_body", {
          values: { current: currentVersion, latest: targetVersion },
        })}
      </p>
      <pre
        class="overflow-x-auto rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-emerald-300">sudo hoard-server upgrade</pre>
    {/if}
  </div>

  {#snippet footer()}
    <button
      type="button"
      onclick={onClose}
      disabled={installing || busy}
      class="inline-flex items-center justify-center gap-2 rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-red-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {$_("updates.no")}
    </button>
    {#if isServer && canRunServerUpgrade}
      <button
        type="button"
        onclick={onYes}
        disabled={installing || busy}
        class="inline-flex items-center justify-center gap-2 rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-emerald-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-emerald-600/60"
      >
        {#if busy}
          <Loader2 size={14} class="animate-spin" />
          <span>{$_("updates.installing")}</span>
        {:else}
          <span>{$_("updates.server_upgrade_button")}</span>
        {/if}
      </button>
    {:else}
      <button
        type="button"
        onclick={onYes}
        disabled={installing || busy}
        class="inline-flex items-center justify-center gap-2 rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-emerald-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-emerald-600/60"
      >
        {#if installing}
          <Loader2 size={14} class="animate-spin" />
          <span>{$_("updates.installing")}</span>
        {:else}
          <span>{$_("updates.yes")}</span>
        {/if}
      </button>
    {/if}
  {/snippet}
</Modal>
