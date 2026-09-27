<script lang="ts">
  /**
   * "This isn't a save folder."
   *
   * A tracked folder that turned out to be a game's installation, an installer
   * or a whole Wine prefix. The backup already keeps only the saves in it, so
   * nothing is broken any more; this says so, and offers the folder detection
   * found instead when there is one. Moving it is the user's call, never done
   * behind their back.
   */
  import { FolderX, ArrowRight } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "./Button.svelte";
  import * as api from "../api";
  import type { FolderWarning } from "../api";
  import { toastError, toastSuccess } from "../stores/toasts";

  type Props = { warnings: FolderWarning[]; onFixed?: () => void };

  let { warnings, onFixed }: Props = $props();

  let dismissed = $state<Set<string>>(new Set());
  let busy = $state<string | null>(null);

  const visible = $derived(warnings.filter((w) => !dismissed.has(w.save_id)));

  function leaf(p: string): string {
    const parts = p.split(/[\\/]+/).filter(Boolean);
    return parts[parts.length - 1] ?? p;
  }

  function dismiss(id: string) {
    dismissed = new Set([...dismissed, id]);
  }

  async function repoint(w: FolderWarning) {
    if (!w.suggested_path) return;
    busy = w.save_id;
    try {
      await api.setSaveLocalPath(w.save_id, w.suggested_path);
      toastSuccess($_("folderkind.used", { values: { folder: leaf(w.suggested_path) } }));
      dismiss(w.save_id);
      onFixed?.();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busy = null;
    }
  }
</script>

{#each visible as w (w.save_id)}
  <div
    class="mb-5 flex items-start gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-200"
  >
    <FolderX size={15} class="mt-0.5 shrink-0 text-amber-400" />
    <div class="min-w-0 flex-1">
      <p class="font-medium">
        {$_("folderkind.title", { values: { game: w.game_slug } })}
      </p>
      <p class="mt-1 text-amber-200/80">
        {$_(
          w.kind === "install" && w.keeps_catalog_saves
            ? "folderkind.install_catalog"
            : `folderkind.${w.kind}`,
        )}
      </p>
      <p class="mt-2 break-all font-mono text-[11px] text-amber-200/50">
        {w.tracked_path}{#if w.suggested_path}&nbsp;→ {w.suggested_path}{/if}
      </p>
      <div class="mt-2.5 flex flex-wrap items-center gap-2">
        {#if w.suggested_path}
          <Button
            variant="ghost"
            onclick={() => repoint(w)}
            loading={busy === w.save_id}
          >
            <ArrowRight size={13} />
            {$_("folderkind.use", { values: { folder: leaf(w.suggested_path) } })}
          </Button>
        {/if}
        <Button variant="ghost" onclick={() => dismiss(w.save_id)}>
          {$_("folderkind.dismiss")}
        </Button>
      </div>
    </div>
  </div>
{/each}
