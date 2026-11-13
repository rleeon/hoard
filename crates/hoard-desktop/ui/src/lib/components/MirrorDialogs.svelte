<script lang="ts">
  /**
   * The three questions about a save's second folder: add it, stop keeping it
   * equal, and which of the two to keep when they differ. One instance per
   * page; `mirrors.svelte.ts` says which is open.
   */
  import { FolderOpen, FolderSync } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "./Button.svelte";
  import Modal from "./Modal.svelte";
  import { toastError, toastSuccess } from "../stores/toasts";
  import { fmtDateTime } from "../utils/format";
  import {
    addMirror,
    closeMirrorDialog,
    keepMirror,
    mirrorDialog,
    removeMirror,
    type MirrorKeep,
  } from "../stores/mirrors.svelte";

  const dialog = $derived(mirrorDialog());
  let busy = $state(false);

  async function run(work: () => Promise<void>, done?: string) {
    busy = true;
    try {
      await work();
      if (done) toastSuccess(done);
      closeMirrorDialog();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busy = false;
    }
  }

  function close() {
    if (!busy) closeMirrorDialog();
  }

  function keep(side: MirrorKeep) {
    if (dialog?.kind !== "resolve") return;
    const { save, folder } = dialog;
    void run(() => keepMirror(save.save_id, folder.path, side));
  }
</script>

<Modal
  open={dialog?.kind === "add"}
  title={$_("second_folder.add_modal_title")}
  onClose={close}
  dismissible={!busy}
>
  {#if dialog?.kind === "add"}
    {@const target = dialog}
    <div class="flex flex-col gap-3 text-sm text-zinc-300">
      <p>{$_("second_folder.add_body")}</p>
      <div class="flex flex-col gap-1.5 rounded-lg border border-white/[0.08] bg-layer-2 p-3 text-xs">
        <span class="flex items-center gap-1.5 text-zinc-400" title={target.save.local_path}>
          <FolderOpen size={12} class="shrink-0" />
          <span class="truncate">{target.save.local_path}</span>
        </span>
        <span class="flex items-center gap-1.5 text-emerald-300" title={target.path}>
          <FolderSync size={12} class="shrink-0" />
          <span class="truncate">{target.path}</span>
        </span>
      </div>
      <p class="text-xs text-zinc-400">{$_("second_folder.add_note")}</p>
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={close} disabled={busy}>{$_("common.cancel")}</Button>
    <Button
      loading={busy}
      onclick={() => {
        if (dialog?.kind !== "add") return;
        const { save, path } = dialog;
        void run(() => addMirror(save.save_id, path), $_("second_folder.added_toast"));
      }}
    >
      {$_("second_folder.add_confirm")}
    </Button>
  {/snippet}
</Modal>

<Modal
  open={dialog?.kind === "remove"}
  title={$_("second_folder.remove_modal_title")}
  onClose={close}
  dismissible={!busy}
>
  {#if dialog?.kind === "remove"}
    <div class="flex flex-col gap-3 text-sm text-zinc-300">
      <p class="break-all text-xs text-zinc-400">{dialog.folder.path}</p>
      <p>{$_("second_folder.remove_body")}</p>
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={close} disabled={busy}>{$_("common.cancel")}</Button>
    <Button
      variant="secondary"
      loading={busy}
      onclick={() => {
        if (dialog?.kind !== "remove") return;
        const { save, folder } = dialog;
        void run(() => removeMirror(save.save_id, folder.path), $_("second_folder.removed_toast"));
      }}
    >
      {$_("second_folder.remove_confirm")}
    </Button>
  {/snippet}
</Modal>

<Modal
  open={dialog?.kind === "resolve"}
  title={$_("second_folder.resolve_title")}
  onClose={close}
  dismissible={!busy}
>
  {#if dialog?.kind === "resolve"}
    {@const target = dialog}
    {@const sides = target.folder.sides}
    <div class="flex flex-col gap-3 text-sm text-zinc-300">
      <p>{$_("second_folder.resolve_body")}</p>
      {#each [{ side: "primary" as const, path: target.save.local_path, files: sides?.primary_files, newest: sides?.primary_newest }, { side: "mirror" as const, path: target.folder.path, files: sides?.mirror_files, newest: sides?.mirror_newest }] as row (row.side)}
        <div class="flex flex-col gap-2 rounded-lg border border-white/[0.08] bg-layer-2 p-3">
          <span class="text-xs font-medium text-zinc-200">
            {$_(row.side === "primary" ? "second_folder.side_primary" : "second_folder.side_mirror")}
          </span>
          <span class="break-all text-xs text-zinc-400">{row.path}</span>
          {#if row.files !== undefined}
            <span class="text-xs tabular-nums text-zinc-400">
              {$_("second_folder.side_files", { values: { count: row.files } })}
              {#if row.newest}
                · {$_("second_folder.side_newest", { values: { date: $fmtDateTime(row.newest) } })}
              {/if}
            </span>
          {/if}
          <div>
            <Button variant="outline" disabled={busy} onclick={() => keep(row.side)}>
              {$_("second_folder.keep_this")}
            </Button>
          </div>
        </div>
      {/each}
      <p class="text-xs text-zinc-400">{$_("second_folder.resolve_note")}</p>
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={close} disabled={busy}>{$_("common.cancel")}</Button>
  {/snippet}
</Modal>
