<script lang="ts">
  /**
   * Floating bottom-right panel listing recent `agent://*` activity.
   *
   * Toggled from the sidebar (📜 button) and persisted via
   * `prefs.live_activity_visible`. Read-only — the entries come from
   * `live.ts`'s `activityFeed` circular buffer.
   *
   * We render at most 50 visible rows. The buffer holds more (80) so
   * scrolling history isn't truncated mid-session; the cap above is a
   * cheap CSS overflow cutoff.
   */
  import { _ } from "svelte-i18n";
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
    X,
  } from "lucide-svelte";

  import { activityFeed, type FeedEntry } from "../stores/live";
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
  } as const;

  function relativeTime(at: number): string {
    const seconds = Math.round((Date.now() - at) / 1000);
    if (seconds < 5) return $_("activity.time_just_now");
    if (seconds < 60)
      return $_("activity.time_seconds_ago", { values: { count: seconds } });
    const minutes = Math.round(seconds / 60);
    if (minutes < 60)
      return $_("activity.time_minutes_ago", { values: { count: minutes } });
    const hours = Math.round(minutes / 60);
    return $_("activity.time_hours_ago", { values: { count: hours } });
  }

  function summary(e: FeedEntry): string {
    const name = e.game_slug ?? e.save_id?.slice(0, 8) ?? "—";
    switch (e.kind) {
      case "watcher_armed":
        return $_("activity.watcher_armed", { values: { name } });
      case "game_started":
        return $_("activity.game_started", { values: { name } });
      case "game_stopped":
        return $_("activity.game_stopped", { values: { name } });
      case "throttled":
        return $_("activity.throttled", { values: { name } });
      case "upload_started":
        return $_("activity.upload_started", { values: { name } });
      case "upload_completed":
        return $_("activity.upload_completed", {
          values: {
            name,
            version: e.version ?? 0,
            size: formatBytes(e.bytes ?? 0),
          },
        });
      case "upload_failed":
        return $_("activity.upload_failed", {
          values: { name, error: e.error ?? "" },
        });
      case "bandwidth_throttled":
        return $_("activity.bandwidth_throttled", {
          values: { name, seconds: e.retry_in ?? 60 },
        });
      case "auto_restored":
        return $_("activity.auto_restored", {
          values: { name, version: e.version ?? 0 },
        });
      case "cloud_pull":
        return $_("activity.cloud_pull", {
          values: {
            count: e.new_versions ?? 0,
            size: formatBytes(e.bytes ?? 0),
          },
        });
      case "quota_reached":
        return $_("activity.quota_reached", {
          values: { plan: e.plan ?? "free", seconds: e.retry_in ?? 60 },
        });
      case "offline":
        return $_("activity.offline");
      case "online":
        return $_("activity.online");
    }
  }

  async function hidePanel() {
    try {
      await api.setLiveActivityVisible(false);
    } catch (err) {
      console.warn("setLiveActivityVisible failed:", err);
    }
    onClose();
  }

  // Tick once a second so relative timestamps stay honest. Wrapped in a
  // counter so Svelte 5 notices the change.
  let tick = $state(0);
  let timer: ReturnType<typeof setInterval>;
  $effect(() => {
    timer = setInterval(() => (tick = tick + 1), 1000);
    return () => clearInterval(timer);
  });
  const _tickRef = $derived(tick); // touched in summary() via relativeTime call below
</script>

<aside
  class="pointer-events-auto fixed bottom-4 right-4 z-40 flex w-[min(22rem,calc(100vw-2rem))] flex-col rounded-lg border border-zinc-800 bg-zinc-950/95 shadow-xl backdrop-blur"
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
        <li class="flex items-start gap-3 px-3 py-2">
          <span
            class="mt-0.5 shrink-0 {TINTS[entry.kind]}"
            aria-hidden="true"
          >
            <Icon size={14} />
          </span>
          <div class="min-w-0 flex-1 text-xs leading-snug">
            <p class="truncate text-zinc-200">{summary(entry)}</p>
            <p class="text-[10px] text-zinc-500">
              {_tickRef >= 0 ? relativeTime(entry.at) : ""}
            </p>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</aside>
