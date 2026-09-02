<script lang="ts">
  /**
   * Storage usage indicator for the dashboard.
   *
   * Two display modes (driven by `is_local_server`, classified by Rust on
   * login, see `commands/auth.rs::classify_server`):
   *
   *  - Self-hosted (localhost / RFC1918 / .local): show MB used.
   *    The user owns the disk; a quota bar would be nonsense.
   *  - External SaaS: show percent + a coloured bar so heavy users see
   *    how close they are to hitting their limit.
   *
   * The component is purely presentational, the parent owns refresh
   * cadence (typically a 30s interval calling `refreshQuota` from the
   * auth store).
   */
  import { HardDrive } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import type { UserInfo } from "../api";
  import CountUp from "./CountUp.svelte";

  type Props = {
    user: UserInfo;
  };

  let { user }: Props = $props();

  // Format bytes with one decimal place once we cross MB. We don't try to
  // pluralize "bytes"/"KB" because saves are virtually always at least a
  // few KB, and humanizing all the edge cases adds noise.
  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  let used = $derived(user.storage_used_bytes ?? 0);
  let quota = $derived(user.storage_quota_bytes ?? 0);

  // Percent only makes sense when there's an actual cap; clamp to [0, 100]
  // so a server that overshot its own quota during a migration doesn't
  // produce a bar wider than the parent.
  let pct = $derived(
    quota > 0 ? Math.min(100, Math.max(0, (used / quota) * 100)) : 0,
  );

  // Bar colour escalates as the user fills up. We use the same palette as
  // the agent activity pills so the dashboard feels consistent.
  let barClass = $derived(
    pct >= 95
      ? "bg-red-500"
      : pct >= 80
        ? "bg-amber-400"
        : "bg-emerald-500",
  );
</script>

<div
  class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 shadow-sm"
>
  <div class="flex items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <span
        class="flex h-9 w-9 items-center justify-center rounded-lg bg-zinc-800/80 text-zinc-300"
      >
        <HardDrive size={16} />
      </span>
      <div>
        <p class="text-xs uppercase tracking-wide text-zinc-500">
          {$_("quota.label")}
        </p>
        {#if user.is_local_server}
          <p class="text-sm font-medium text-zinc-100">
            {$_("quota.used", { values: { size: fmtBytes(used) } })}
          </p>
        {:else if quota > 0}
          <p class="text-sm font-medium text-zinc-100">
            {$_("quota.used_of", { values: { used: fmtBytes(used), quota: fmtBytes(quota) } })}
          </p>
        {:else}
          <p class="text-sm font-medium text-zinc-100">
            {$_("quota.used", { values: { size: fmtBytes(used) } })}
          </p>
        {/if}
      </div>
    </div>
    {#if !user.is_local_server && quota > 0}
      <span
        class="text-sm font-semibold tabular-nums {pct >= 95
          ? 'text-red-400'
          : pct >= 80
            ? 'text-amber-400'
            : 'text-zinc-300'}"
      >
        <CountUp value={pct} decimals={1} suffix="%" />
      </span>
    {:else if user.is_local_server}
      <span class="text-xs text-zinc-500">{$_("quota.self_hosted")}</span>
    {/if}
  </div>

  {#if !user.is_local_server && quota > 0}
    <!-- Track + fill. Width is bound to the derived `pct` so the bar
         animates smoothly as polls land. -->
    <div class="mt-3 h-2 w-full overflow-hidden rounded-full bg-zinc-800">
      <div
        class="h-full rounded-full transition-[width] duration-500 ease-out {barClass}"
        style:width="{pct}%"
      ></div>
    </div>
  {/if}
</div>
