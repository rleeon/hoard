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
  import { customNames } from "../stores/gameNames";
  import { prettifySlug } from "../utils/format";
  import { dismiss, isDismissed } from "../stores/dismissedWarnings.svelte";

  import Button from "./Button.svelte";
  import { glow } from "../actions/glow";
  import * as api from "../api";
  import type { FolderWarning } from "../api";
  import { toastError, toastSuccess } from "../stores/toasts";

  type Props = {
    warnings: FolderWarning[];
    onFixed?: () => void;
    /** How many are on screen, for the tray's count. */
    shown?: number;
  };

  let { warnings, onFixed, shown = $bindable(0) }: Props = $props();

  let busy = $state<string | null>(null);

  const visible = $derived(
    warnings.filter((w) => !isDismissed("folders", w.save_id)),
  );
  $effect(() => {
    shown = visible.length;
  });

  function leaf(p: string): string {
    const parts = p.split(/[\\/]+/).filter(Boolean);
    return parts[parts.length - 1] ?? p;
  }


  async function repoint(w: FolderWarning) {
    if (!w.suggested_path) return;
    busy = w.save_id;
    try {
      await api.setSaveLocalPath(w.save_id, w.suggested_path);
      toastSuccess($_("folderkind.used", { values: { folder: leaf(w.suggested_path) } }));
      dismiss("folders", w.save_id);
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
    use:glow
    class="glow tray-card mb-2 flex items-start gap-2.5 rounded-md border border-amber-500/25 bg-amber-500/[0.08] px-3.5 py-2.5 text-[13px] text-amber-200 last:mb-0 hover:-translate-y-0.5 hover:border-amber-500/55 hover:bg-amber-500/[0.15] hover:shadow-[0_10px_26px_-14px_rgb(245_158_11/0.55)]"
  >
    <FolderX size={15} class="mt-0.5 shrink-0 text-amber-400" data-anim="pop" />
    <div class="min-w-0 flex-1">
      <p class="font-medium">
        {$_("folderkind.title", {
          values: {
            game: $customNames[w.game_slug] ?? prettifySlug(w.game_slug),
          },
        })}
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
      <div class="mt-1.5 flex flex-wrap items-center gap-2">
        {#if w.suggested_path}
          <Button
            variant="ghost"
            onclick={() => repoint(w)}
            loading={busy === w.save_id}
          >
            <ArrowRight size={13} data-anim="pop" />
            {$_("folderkind.use", { values: { folder: leaf(w.suggested_path) } })}
          </Button>
        {/if}
        <Button
          variant="ghost"
          onclick={() => dismiss("folders", w.save_id)}
        >
          {$_("folderkind.dismiss")}
        </Button>
      </div>
    </div>
  </div>
{/each}
