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
  import { _ } from "svelte-i18n";
  import { Loader2 } from "lucide-svelte";
  import Modal from "./Modal.svelte";
  import { applyDesktopUpdate, type UpdateReport } from "../stores/updates";
  import { toastInfo, toastSuccess, toastError } from "../stores/toasts";

  type Props = {
    open: boolean;
    report: UpdateReport | null;
    onClose: () => void;
  };
  let { open, report, onClose }: Props = $props();

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

  let installing = $state(false);

  async function onYes() {
    if (installing) return;
    if (isClient) {
      installing = true;
      try {
        const r = await applyDesktopUpdate();
        if (r.kind === "installer_launched") {
          toastInfo($_("updates.installer_launched"));
        } else {
          toastInfo(
            $_("updates.downloaded_manual", { values: { path: r.path } }),
          );
        }
        onClose();
      } catch (e) {
        toastError(
          $_("updates.install_failed", { values: { error: String(e) } }),
        );
      } finally {
        installing = false;
      }
    } else if (isServer) {
      // Server upgrade is a CLI affair; we copy the command for the user to
      // paste into their server's shell.
      try {
        await navigator.clipboard.writeText("sudo hoard-server upgrade");
        toastSuccess($_("updates.server_command_copied"));
      } catch {
        toastInfo($_("updates.server_command_manual"));
      }
      onClose();
    } else {
      onClose();
    }
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
      disabled={installing}
      class="inline-flex items-center justify-center gap-2 rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-red-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {$_("updates.no")}
    </button>
    <button
      type="button"
      onclick={onYes}
      disabled={installing}
      class="inline-flex items-center justify-center gap-2 rounded-md bg-emerald-600 px-4 py-2 text-sm font-medium text-zinc-50 transition-colors hover:bg-emerald-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:bg-emerald-600/60"
    >
      {#if installing}
        <Loader2 size={14} class="animate-spin" />
        <span>{$_("updates.installing")}</span>
      {:else}
        <span>{$_("updates.yes")}</span>
      {/if}
    </button>
  {/snippet}
</Modal>
