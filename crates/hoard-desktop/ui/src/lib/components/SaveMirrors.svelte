<script lang="ts">
  /**
   * The second folders of one tracked save, under its own folder in the
   * Library card: where each stands, and the way to drop it or to settle a
   * conflict. The questions themselves are `MirrorDialogs`.
   */
  import { AlertTriangle, FolderSync, X } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import type { TrackedSave } from "../api";
  import {
    askRemoveMirror,
    askResolveMirror,
    mirrorsOf,
    type MirrorFolder,
  } from "../stores/mirrors.svelte";

  let { save }: { save: TrackedSave } = $props();

  const folders = $derived(mirrorsOf(save.save_id));

  function label(folder: MirrorFolder): string {
    if (folder.keep) return $_("second_folder.status_copying");
    return $_(`second_folder.status_${folder.status}`);
  }

  function tone(folder: MirrorFolder): string {
    if (folder.keep || folder.status === "pending") return "bg-zinc-700/40 text-zinc-400";
    if (folder.status === "in_sync") return "bg-emerald-500/15 text-emerald-300";
    return "bg-amber-500/10 text-amber-400";
  }
</script>

{#each folders as folder (folder.path)}
  <div class="flex flex-col gap-1">
    <div class="flex items-center gap-1 text-[10px] text-zinc-500" title={folder.path}>
      <FolderSync size={10} class="shrink-0" />
      <span class="min-w-0 flex-1 truncate">{folder.path}</span>
      <span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium {tone(folder)}">
        {label(folder)}
      </span>
      <button
        type="button"
        onclick={() => askRemoveMirror(save, folder)}
        aria-label={$_("second_folder.remove_title")}
        title={$_("second_folder.remove_title")}
        class="shrink-0 rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-700/40 hover:text-zinc-200"
      >
        <X size={11} />
      </button>
    </div>
    {#if folder.status === "conflict" && !folder.keep}
      <div class="flex items-start gap-1 text-[10px] text-amber-400/90">
        <AlertTriangle size={10} class="mt-px shrink-0" />
        <span class="min-w-0 flex-1">{$_("second_folder.conflict_hint")}</span>
        <button
          type="button"
          onclick={() => askResolveMirror(save, folder)}
          class="shrink-0 rounded bg-amber-500/10 px-1.5 py-0.5 font-medium text-amber-300 ring-1 ring-inset ring-amber-500/40 transition-colors hover:bg-amber-500/20"
        >
          {$_("second_folder.resolve_button")}
        </button>
      </div>
    {/if}
  </div>
{/each}
