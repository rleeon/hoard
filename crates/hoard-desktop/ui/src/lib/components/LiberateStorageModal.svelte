<script lang="ts">
  /**
   * "Libera espacio" dialog, the black-box escape hatch.
   *
   * Shown when the account's live saves exceed the plan limit and purging old
   * versions can't bring them under (the Pro→Free case). It lists the games by
   * weight and offers three ways out:
   *   - green, top     → upgrade to Pro (nothing gets archived)
   *   - black, bottom-l → download the saves first (reuses the account export)
   *   - red,   bottom-r → Continuar: archive the ticked games. Archiving frees
   *                       the quota now, freezes the cloud copy for 7 days
   *                       (downloadable), then it's purged. The LOCAL save is
   *                       never touched and it's reversible by reactivating
   *                       after upgrading.
   *
   * El cuerpo,la lista, las casillas y el medidor, vive en
   * {@link LiberateStoragePanel}, porque la despedida de Pro enseña lo mismo
   * midiendo contra otro límite. Aquí quedan sólo las salidas.
   */
  import { _ } from "svelte-i18n";
  import { push } from "svelte-spa-router";
  import Modal from "./Modal.svelte";
  import LiberateStoragePanel from "./LiberateStoragePanel.svelte";
  import { Crown, Download, Archive } from "@lucide/svelte";
  import { archiveSaveCloud } from "../stores/cloud";
  import { toastError, toastSuccess } from "../stores/toasts";

  type Props = {
    open: boolean;
    onClose: () => void;
    /** Reuse the Account page's export flow to grab a copy first. */
    onDownload: () => void;
    /** Called after a successful archive so the parent can refresh the account
     *  (storage bar / status). */
    onDone: () => void;
  };

  let { open, onClose, onDownload, onDone }: Props = $props();

  let busy = $state(false);
  let selected = $state<Set<string>>(new Set());

  async function handleContinue() {
    const ids = [...selected];
    if (ids.length === 0) {
      onClose();
      return;
    }
    busy = true;
    try {
      for (const id of ids) {
        await archiveSaveCloud(id);
      }
      toastSuccess($_("liberate.done"));
      onDone();
      onClose();
    } catch (e) {
      toastError(String(e));
    } finally {
      busy = false;
    }
  }

  // A la pantalla Pro, no al navegador. Este diálogo salta cuando la cuota se
  // llena, o sea, en mitad de otra cosa, así que abrir una pestaña encima es
  // el peor momento posible para hacerlo.
  function goPro() {
    onClose();
    push("/pro");
  }
</script>

<Modal
  {open}
  title={$_("liberate.title")}
  dismissible={!busy}
  onClose={busy ? () => {} : onClose}
>
  <div class="space-y-4">
    <p class="text-sm text-zinc-300">{$_("liberate.intro")}</p>

    <!-- Pasar a Pro — green, top, full width -->
    <button
      type="button"
      onclick={goPro}
      disabled={busy}
      class="flex w-full items-center justify-center gap-2 rounded-lg bg-emerald-600 px-4 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-emerald-500 disabled:opacity-50"
    >
      <Crown size={16} />
      {$_("liberate.pro")}
    </button>

    <LiberateStoragePanel {open} {busy} bind:selected />

    <p class="rounded-lg border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs text-amber-200/90">
      {$_("liberate.explain_archive")}
    </p>
  </div>

  {#snippet footer()}
    <div class="flex w-full items-center justify-between gap-3">
      <!-- Descargar saves — black, bottom-left -->
      <button
        type="button"
        onclick={onDownload}
        disabled={busy}
        class="flex items-center gap-2 rounded-lg border border-white/10 bg-zinc-900 px-4 py-2 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-800 disabled:opacity-50"
      >
        <Download size={15} />
        {$_("liberate.download")}
      </button>

      <!-- Continuar (= archivar) — red, bottom-right -->
      <button
        type="button"
        onclick={handleContinue}
        disabled={busy || selected.size === 0}
        class="flex items-center gap-2 rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white transition-colors hover:bg-red-500 disabled:opacity-50"
      >
        <Archive size={15} />
        {busy
          ? $_("liberate.archiving")
          : $_("liberate.continue_n", { values: { count: selected.size } })}
      </button>
    </div>
  {/snippet}
</Modal>
