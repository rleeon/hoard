<script lang="ts">
  /**
   * Tiny status dot + label that sits in the sidebar header.
   *
   * Two halves: local watcher state ("escuchando saves") and cloud loop
   * state ("nube online"). Aggregated into a single colour via the
   * `liveStatus` derived store; tooltip exposes the granular reading.
   *
   * Reads from `live.ts`; doesn't subscribe to anything itself — the
   * subscription is set up once at boot from `App.svelte`.
   */
  import { _ } from "svelte-i18n";
  import {
    watcher as watcherStore,
    cloudLoop,
    liveStatus,
  } from "../stores/live";
  import { auth } from "../stores/auth";

  const w = watcherStore;
  const c = cloudLoop;

  // Self-hosted servers have no cloud-pull loop, so the cloud half would sit
  // on "Comprobando…" forever. On a local server we show watcher state only
  // and colour the dot from the watcher alone.
  const isLocal = $derived($auth.user?.is_local_server ?? false);

  const dot = $derived.by(() => {
    if (isLocal) {
      return $w.armed
        ? "bg-emerald-500 shadow-[0_0_0_3px_rgba(16,185,129,0.18)]"
        : "bg-zinc-500 shadow-[0_0_0_3px_rgba(113,113,122,0.18)]";
    }
    switch ($liveStatus) {
      case "ok":
        return "bg-emerald-500 shadow-[0_0_0_3px_rgba(16,185,129,0.18)]";
      case "warn":
        return "bg-amber-500 shadow-[0_0_0_3px_rgba(245,158,11,0.18)]";
      case "error":
        return "bg-rose-500 shadow-[0_0_0_3px_rgba(244,63,94,0.18)]";
      default:
        return "bg-zinc-500 shadow-[0_0_0_3px_rgba(113,113,122,0.18)]";
    }
  });

  // Compact label: watcher count + cloud state. We deliberately keep it
  // short so the sidebar header doesn't reflow when the count changes.
  const label = $derived.by(() => {
    const wPart = $w.armed
      ? $_("status.watching_count", { values: { count: $w.count } })
      : $_("status.watcher_off");
    if (isLocal) return wPart;
    let cPart: string;
    switch ($c.status) {
      case "online":
        cPart = $_("status.cloud_online");
        break;
      case "throttled":
        cPart = $_("status.cloud_throttled");
        break;
      case "offline":
        cPart = $_("status.cloud_offline");
        break;
      default:
        cPart = $_("status.cloud_unknown");
    }
    return `${wPart} · ${cPart}`;
  });

  const tooltip = $derived.by(() => {
    const lines: string[] = [];
    lines.push(
      $w.armed
        ? $_("status.tooltip_watcher_on", { values: { count: $w.count } })
        : $_("status.tooltip_watcher_off"),
    );
    if (isLocal) return lines.join("\n");
    switch ($c.status) {
      case "online":
        lines.push(
          $c.last_ok_at
            ? $_("status.tooltip_cloud_online", {
                values: { seconds: Math.round((Date.now() - $c.last_ok_at) / 1000) },
              })
            : $_("status.cloud_online"),
        );
        break;
      case "throttled":
        lines.push(
          $_("status.tooltip_cloud_throttled", {
            values: { seconds: $c.retry_in ?? 60 },
          }),
        );
        break;
      case "offline":
        lines.push($_("status.tooltip_cloud_offline"));
        break;
      default:
        lines.push($_("status.cloud_unknown"));
    }
    return lines.join("\n");
  });
</script>

<div
  class="flex items-center gap-2 px-1 text-[11px] text-zinc-400"
  title={tooltip}
  aria-label={tooltip}
>
  <span
    class="h-2 w-2 shrink-0 rounded-full transition-colors duration-300 {dot}"
    aria-hidden="true"
  ></span>
  <span class="truncate">{label}</span>
</div>
