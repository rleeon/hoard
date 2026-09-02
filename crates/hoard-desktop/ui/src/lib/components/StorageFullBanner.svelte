<script lang="ts">
  /**
   * Persistent "the account is full, uploads are parked" card.
   *
   * The same message the ActivityFeed shows as a `backup_quota_full` row,
   * except the feed is a *scrolling log the user can hide*, so the one fact
   * that stops Hoard from doing its job used to scroll away (or never show up
   * at all if the panel was closed). This is the state version of it: it stays
   * on screen for as long as the account is over its limit, and disappears on
   * its own the moment the next account refresh says there's room again.
   *
   * Deliberately not dismissible: nothing is being backed up while it's up, so
   * a dismissed banner would just recreate the invisible-failure it exists to
   * fix. The way out (`Liberar espacio`) is right on the card.
   */
  import { _ } from "svelte-i18n";
  import { fly } from "svelte/transition";
  import { HardDrive } from "@lucide/svelte";

  import { storageBlock } from "../stores/live";
  import { openLiberate } from "../stores/liberate";
  import { formatBytes } from "../utils/format";
</script>

{#if $storageBlock}
  <div
    class="pointer-events-auto overflow-hidden rounded-lg border border-rose-500/60 bg-zinc-950/95 shadow-xl backdrop-blur"
    role="status"
    aria-live="polite"
    transition:fly={{ y: 12, duration: 180 }}
  >
    <!-- Tint layered *over* the opaque card rather than set on it: a single
         `bg-rose-500/10` would let whatever is behind the window bleed through. -->
    <div class="flex items-start gap-2.5 bg-rose-500/10 p-3">
      <span class="mt-0.5 shrink-0 text-rose-400" aria-hidden="true">
        <HardDrive size={16} />
      </span>
      <div class="min-w-0 flex-1">
        <p class="text-xs leading-snug text-zinc-100">
          {$_("activity.backup_quota_full", {
            values: {
              used: formatBytes($storageBlock.used),
              limit: formatBytes($storageBlock.limit),
            },
          })}
        </p>
        <button
          type="button"
          class="mt-2 rounded-md border border-rose-500/50 bg-rose-500/20 px-2.5 py-1 text-[11px] font-medium text-rose-100 transition-colors hover:bg-rose-500/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-400"
          onclick={openLiberate}
        >
          {$_("liberate.cta")}
        </button>
      </div>
    </div>
  </div>
{/if}
