<script lang="ts">
  /**
   * "You're syncing the game's backups, not your save."
   *
   * Detection stops looking at a folder the moment it's tracked (`run_scan`
   * skips tracked slugs), so the scoring fixes that keep new users off a
   * backup mirror do nothing for the ones already on one. This banner is the
   * only path back for them, and the reason it exists at all: an affected
   * account uploads a fresh full copy of the mirror every few minutes,
   * deduplicates against nothing, and restores folders the game won't load.
   *
   * Two acts, both explicit. Repointing is offered, never done, a silent
   * repoint is what broke pairing across machines in August. Archiving the old
   * row is offered second because repointing alone leaves the mirror's
   * versions sitting in the quota; it's Cloud-only (self-hosted has no black
   * box) and it frees space without destroying history.
   */
  import { AlertTriangle, ArrowRight, Archive } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Button from "./Button.svelte";
  import * as api from "../api";
  import type { MirrorWarning } from "../api";
  import { archiveSaveCloud } from "../stores/cloud";
  import { auth } from "../stores/auth";
  import { toastError, toastSuccess } from "../stores/toasts";
  import { formatBytes } from "../utils/format";

  type Props = {
    warnings: MirrorWarning[];
    /** Cloud footprint per save, so the banner can say what the mistake has
     *  cost so far. Missing on self-hosted and while it loads. */
    footprints?: Record<string, number>;
    /** Re-read the library after a repoint so the card shows the new folder. */
    onFixed?: () => void;
  };

  let { warnings, footprints = {}, onFixed }: Props = $props();

  // Dismissals are per-session and per-save: the warning is worth repeating
  // on the next launch if nothing was done about it, but nagging inside one
  // session after the user has said "not now" is just noise.
  let dismissed = $state<Set<string>>(new Set());
  let busy = $state<string | null>(null);

  const visible = $derived(warnings.filter((w) => !dismissed.has(w.save_id)));
  const isCloud = $derived($auth.user?.is_local_server === false);

  /** Last path segment, the whole path is in the tooltip. Windows and POSIX
   *  separators both, since the warning crosses machines in a shared cache. */
  function leaf(p: string): string {
    const parts = p.split(/[\\/]+/).filter(Boolean);
    return parts[parts.length - 1] ?? p;
  }

  function dismiss(saveId: string) {
    dismissed = new Set([...dismissed, saveId]);
  }

  async function repoint(w: MirrorWarning) {
    busy = w.save_id;
    try {
      await api.setSaveLocalPath(w.save_id, w.suggested_path);
      toastSuccess(
        $_("mirror.repointed", { values: { folder: leaf(w.suggested_path) } }),
      );
      dismiss(w.save_id);
      onFixed?.();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      busy = null;
    }
  }

  async function archive(w: MirrorWarning) {
    busy = w.save_id;
    try {
      const res = await archiveSaveCloud(w.save_id);
      toastSuccess(
        $_("mirror.archived", {
          values: { size: formatBytes(res.freed_bytes) },
        }),
      );
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
    <AlertTriangle size={15} class="mt-0.5 shrink-0 text-amber-400" />
    <div class="min-w-0 flex-1">
      <p class="font-medium">
        {$_("mirror.title", { values: { game: w.game_slug } })}
      </p>
      <p class="mt-1 text-amber-200/80">
        {$_("mirror.body", {
          values: {
            tracked: leaf(w.tracked_path),
            suggested: leaf(w.suggested_path),
          },
        })}
      </p>
      {#if footprints[w.save_id]}
        <p class="mt-1 text-amber-200/80">
          {$_("mirror.cost", {
            values: { size: formatBytes(footprints[w.save_id]) },
          })}
        </p>
      {/if}
      <!-- The full paths, monospaced: the leaf names alone are ambiguous when
           a game keeps several folders, and this is what a user pastes into a
           support thread. -->
      <p class="mt-2 break-all font-mono text-[11px] text-amber-200/50">
        {w.tracked_path} → {w.suggested_path}
      </p>
      <div class="mt-2.5 flex flex-wrap items-center gap-2">
        <Button
          variant="ghost"
          onclick={() => repoint(w)}
          loading={busy === w.save_id}
        >
          <ArrowRight size={13} />
          {$_("mirror.repoint")}
        </Button>
        {#if isCloud}
          <Button
            variant="ghost"
            onclick={() => archive(w)}
            loading={busy === w.save_id}
            title={$_("mirror.archive_title")}
          >
            <Archive size={13} />
            {$_("mirror.archive")}
          </Button>
        {/if}
        <Button variant="ghost" onclick={() => dismiss(w.save_id)}>
          {$_("mirror.dismiss")}
        </Button>
      </div>
    </div>
  </div>
{/each}
