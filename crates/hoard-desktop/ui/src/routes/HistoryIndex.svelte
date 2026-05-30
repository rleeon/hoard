<script lang="ts">
  /**
   * History index — landing page for the "Historial" sidebar item.
   *
   * The per-save timeline (`History.svelte`) lives at `/history/:saveId`, so
   * a bare `/history` had nothing to match and the nav button was disabled —
   * the user could never reach history at all. This page fills that gap:
   * it lists every tracked save and links each one to its timeline.
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { History as HistoryIcon, ChevronRight, PauseCircle } from "lucide-svelte";
  import { _ } from "svelte-i18n";

  import Card from "../lib/components/Card.svelte";
  import * as api from "../lib/api";
  import type { TrackedSave } from "../lib/api";
  import { formatBytes } from "../lib/utils/format";
  import { toastError } from "../lib/stores/toasts";

  let saves = $state<TrackedSave[]>([]);
  let loading = $state(true);

  onMount(async () => {
    try {
      saves = await api.listTrackedSaves();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  });

  function formatRelative(iso: string): string {
    const diff = (Date.now() - new Date(iso).getTime()) / 1000;
    if (diff < 60) return $_("history.relative_just_now");
    if (diff < 3600)
      return $_("history.relative_minutes", {
        values: { count: Math.floor(diff / 60) },
      });
    if (diff < 86400)
      return $_("history.relative_hours", {
        values: { count: Math.floor(diff / 3600) },
      });
    if (diff < 86400 * 7)
      return $_("history.relative_days", {
        values: { count: Math.floor(diff / 86400) },
      });
    return new Date(iso).toLocaleDateString();
  }
</script>

<div class="mx-auto max-w-4xl px-8 py-8">
  <header class="mb-6">
    <h1 class="flex items-center gap-2 text-2xl font-semibold tracking-tight">
      <HistoryIcon size={22} class="text-emerald-400" />
      {$_("history.index_title")}
    </h1>
    <p class="mt-1 text-sm text-zinc-400">{$_("history.index_subtitle")}</p>
  </header>

  {#if loading}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else if saves.length === 0}
    <Card>
      <div class="py-12 text-center">
        <p class="text-sm text-zinc-300">{$_("history.index_empty_title")}</p>
        <p class="mt-1 text-xs text-zinc-500">{$_("history.index_empty_body")}</p>
      </div>
    </Card>
  {:else}
    <ol class="space-y-2">
      {#each saves as save (save.save_id)}
        <li>
          <button
            type="button"
            onclick={() => push(`/history/${save.save_id}`)}
            class="flex w-full items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900/40 px-4 py-3 text-left transition-colors hover:border-zinc-700 hover:bg-zinc-800/40"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 text-sm">
                <span class="truncate font-medium text-zinc-100">
                  {save.game_slug}
                </span>
                <span class="rounded bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400">
                  {save.label}
                </span>
                {#if save.paused}
                  <span
                    class="inline-flex items-center gap-1 rounded bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300"
                  >
                    <PauseCircle size={12} /> {$_("history.paused")}
                  </span>
                {/if}
              </div>
              <div class="mt-1 flex items-center gap-3 text-xs text-zinc-500">
                {#if save.last_version_num !== null}
                  <span>{$_("history.index_version", { values: { version: save.last_version_num } })}</span>
                  <span>·</span>
                {/if}
                <span>{formatBytes(save.total_size_bytes)}</span>
                {#if save.last_backup_at}
                  <span>·</span>
                  <span>{formatRelative(save.last_backup_at)}</span>
                {/if}
              </div>
            </div>
            <ChevronRight size={16} class="shrink-0 text-zinc-500" />
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</div>
