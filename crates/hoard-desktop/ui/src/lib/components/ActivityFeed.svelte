<script lang="ts">
  /**
   * Floating bottom-right panel listing recent `agent://*` activity.
   *
   * Toggled from the sidebar (📜 button) and persisted via
   * `prefs.live_activity_visible`. Read-only, the entries come from
   * `live.ts`'s `activityFeed` circular buffer.
   *
   * We render at most 50 visible rows. The buffer holds more (80) so
   * scrolling history isn't truncated mid-session; the cap above is a
   * cheap CSS overflow cutoff.
   */
  import { _ } from "svelte-i18n";
  import { feedRelativeTime, feedSummary } from "../utils/feedText";
  import { fly } from "svelte/transition";
  import {
    Eye,
    Play,
    Square,
    Clock,
    UploadCloud,
    CheckCircle2,
    XCircle,
    Download,
    RefreshCcw,
    Ban,
    WifiOff,
    Scissors,
    FileWarning,
    Trash2,
    HardDrive,
    AlertTriangle,
    Lock,
    LockOpen,
    X,
  } from "@lucide/svelte";

  import { activityFeed, type FeedEntry } from "../stores/live";
  import { openLiberate } from "../stores/liberate";
  import * as api from "../api";
  import { formatBytes } from "../utils/format";

  /** Hide affordance. Persisted via prefs so the next boot remembers. */
  let { onClose }: { onClose: () => void } = $props();

  const ICONS = {
    watcher_armed: Eye,
    game_started: Play,
    game_stopped: Square,
    throttled: Clock,
    upload_started: UploadCloud,
    upload_completed: CheckCircle2,
    upload_failed: XCircle,
    bandwidth_throttled: Clock,
    auto_restored: Download,
    cloud_pull: RefreshCcw,
    quota_reached: Ban,
    offline: WifiOff,
    online: CheckCircle2,
    backup_too_large: XCircle,
    backup_quota_full: HardDrive,
    backup_trimmed: Scissors,
    backup_files_unreadable: FileWarning,
    auto_restore_failed: XCircle,
    auto_restore_stuck: AlertTriangle,
    auto_restore_recovered: CheckCircle2,
    backup_blocked: AlertTriangle,
    backup_unblocked: CheckCircle2,
    storage_purging: Trash2,
    storage_full: AlertTriangle,
    storage_grace: Clock,
    gate_locked: Lock,
    gate_unlocked: LockOpen,
  } as const;

  const TINTS = {
    watcher_armed: "text-emerald-400",
    game_started: "text-emerald-300",
    game_stopped: "text-zinc-400",
    throttled: "text-amber-300",
    upload_started: "text-emerald-300",
    upload_completed: "text-emerald-400",
    upload_failed: "text-rose-400",
    bandwidth_throttled: "text-amber-300",
    auto_restored: "text-sky-300",
    cloud_pull: "text-emerald-300",
    quota_reached: "text-amber-400",
    offline: "text-rose-400",
    online: "text-emerald-400",
    backup_too_large: "text-rose-400",
    backup_quota_full: "text-rose-400",
    backup_trimmed: "text-amber-300",
    backup_files_unreadable: "text-amber-300",
    auto_restore_failed: "text-rose-400",
    auto_restore_stuck: "text-amber-400",
    auto_restore_recovered: "text-emerald-400",
    backup_blocked: "text-rose-400",
    backup_unblocked: "text-emerald-400",
    storage_purging: "text-amber-400",
    storage_full: "text-rose-400",
    storage_grace: "text-sky-300",
    gate_locked: "text-rose-400",
    gate_unlocked: "text-emerald-400",
  } as const;

  // Alert rows get a tinted "card" so plan-limit / storage-pressure events
  // read at a glance: amber for reversible pressure (trimming / purging),
  // red for a hard stop (over-cap upload, restore failure, storage full).
  const ROW_ACCENT: Partial<Record<FeedEntry["kind"], string>> = {
    backup_too_large: "my-1 rounded-md border border-rose-500/60 bg-rose-500/10",
    auto_restore_failed:
      "my-1 rounded-md border border-rose-500/60 bg-rose-500/10",
    // Amber, not red: the save still syncs once the cause clears, and the
    // agent keeps retrying on the escalating backoff.
    auto_restore_stuck:
      "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_full: "my-1 rounded-md border border-rose-500/60 bg-rose-500/10",
    // Red, not amber: unlike `auto_restore_stuck`, there is no retry waiting here.
    // Without a person, this save never uploads again.
    backup_blocked: "my-1 rounded-md border border-rose-500/60 bg-rose-500/10",
    backup_quota_full:
      "my-1 rounded-md border border-rose-500/60 bg-rose-500/10",
    backup_trimmed: "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    // Amber: the backup is usable, it is just missing a piece. The day nothing
    // uploads, `upload_failed`'s red row appears next to it and that one rules.
    backup_files_unreadable:
      "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_purging: "my-1 rounded-md border border-amber-500/60 bg-amber-500/10",
    storage_grace: "my-1 rounded-md border border-sky-500/60 bg-sky-500/10",
  };

  /** Rows whose problem the user can actually do something about from here. */
  const ACTIONABLE = new Set<FeedEntry["kind"]>([
    "backup_quota_full",
    "storage_full",
  ]);

  const relativeTime = (at: number) => feedRelativeTime(at, $_);
  const summary = (e: FeedEntry) => feedSummary(e, $_);

  async function hidePanel() {
    try {
      await api.setLiveActivityVisible(false);
    } catch (err) {
      console.warn("setLiveActivityVisible failed:", err);
    }
    onClose();
  }

  // Tick once a second so relative timestamps stay honest. Wrapped in a
  // counter so Svelte 5 notices the change. Skipped while the window is hidden
  //, no visible timestamps to keep honest, so don't force re-renders.
  let tick = $state(0);
  let timer: ReturnType<typeof setInterval>;
  $effect(() => {
    timer = setInterval(() => {
      if (!document.hidden) tick = tick + 1;
    }, 1000);
    return () => clearInterval(timer);
  });
  const _tickRef = $derived(tick); // touched in summary() via relativeTime call below
</script>

<!-- Positioning belongs to the bottom-right stack in App.svelte, not here: the
     storage banner shares that corner and the two must not sit on top of each
     other. This is just the card. -->
<aside
  class="pointer-events-auto flex w-full flex-col rounded-lg border border-zinc-800 bg-zinc-950/95 shadow-xl backdrop-blur"
  aria-label={$_("activity.panel_label")}
  transition:fly={{ y: 12, duration: 180 }}
>
  <header
    class="flex items-center justify-between border-b border-zinc-800 px-3 py-2"
  >
    <div class="flex items-center gap-2 text-xs font-medium text-zinc-200">
      <span aria-hidden="true">
        <RefreshCcw size={12} />
      </span>
      <span>{$_("activity.panel_title")}</span>
    </div>
    <button
      type="button"
      class="rounded p-1 text-zinc-400 hover:text-zinc-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500"
      aria-label={$_("activity.hide_panel")}
      title={$_("activity.hide_panel")}
      onclick={hidePanel}
    >
      <X size={14} />
    </button>
  </header>

  {#if $activityFeed.length === 0}
    <p class="px-3 py-6 text-center text-xs text-zinc-500">
      {$_("activity.empty")}
    </p>
  {:else}
    <ul class="max-h-[min(60vh,28rem)] divide-y divide-zinc-800/60 overflow-y-auto">
      {#each $activityFeed.slice(0, 50) as entry (entry.id)}
        {@const Icon = ICONS[entry.kind]}
        <li class="flex items-start gap-3 px-3 py-2 {ROW_ACCENT[entry.kind] ?? ''}">
          <span
            class="mt-0.5 shrink-0 {TINTS[entry.kind]}"
            aria-hidden="true"
          >
            <Icon size={14} />
          </span>
          <div class="min-w-0 flex-1 text-xs leading-snug">
            <p class="text-zinc-200 {ACTIONABLE.has(entry.kind) ? '' : 'truncate'}">
              {summary(entry)}
            </p>
            <!-- Storage rows carry the way out. A feed that only *reports* a
                 full account leaves the user hunting through Settings for the
                 escape hatch at the worst possible moment. -->
            {#if ACTIONABLE.has(entry.kind)}
              <button
                type="button"
                class="mt-1 rounded-md border border-rose-500/50 bg-rose-500/10 px-2 py-1 text-[11px] font-medium text-rose-200 transition-colors hover:bg-rose-500/20"
                onclick={openLiberate}
              >
                {$_("liberate.cta")}
              </button>
            {/if}
            <p class="text-[10px] text-zinc-500">
              {_tickRef >= 0 ? relativeTime(entry.at) : ""}
            </p>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</aside>
