<script lang="ts">
  /**
   * Compact storage bar for the sidebar footer (replaces the old
   * watcher/cloud LiveStatus line).
   *
   * Same reading as the dashboard's QuotaBar, used / quota with the bar
   * escalating emerald → amber (≥80%) → red (≥95%), just shrunk to fit
   * the rail. Two data sources: a self-hosted session (`auth.user`) or a
   * Hoard Cloud account (`cloud.account`), a cloud-only user has no
   * `auth.user`, so reading only that store left the footer blank for
   * every Gmail login. Prefer self-hosted when both exist. Uncapped
   * sources (self-hosted "at home", or a `-1` cloud limit) get the plain
   * "X usado" line with no bar.
   *
   * The sidebar outlives every route, so this component owns its own 30s
   * refresh; the dashboard's poll writes to the same store, whoever fires
   * first wins.
   */
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { _ } from "svelte-i18n";
  import { auth, refreshQuota } from "../stores/auth";
  import { cloud, refreshCloud } from "../stores/cloud";
  import CountUp from "./CountUp.svelte";
  import { glow } from "../actions/glow";
  import { tilt } from "../actions/tilt";

  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  onMount(() => {
    const timer = setInterval(() => {
      if (document.hidden) return;
      // Refresh whichever session is live; self-hosted wins if both are.
      if (get(auth).user) refreshQuota().catch(() => {});
      else if (get(cloud).account) refreshCloud().catch(() => {});
    }, 30_000);
    return () => clearInterval(timer);
  });

  // Unify the two sources. Self-hosted `storage_quota_bytes` and cloud
  // `storage_limit_bytes` both mean "cap in bytes"; a cloud limit of `-1`
  // is unlimited (no bar), matching a self-hosted "at home" server.
  const src = $derived.by(() => {
    const u = $auth.user;
    if (u) {
      return {
        show: true,
        used: u.storage_used_bytes,
        limit: u.storage_quota_bytes,
        capped: !u.is_local_server && u.storage_quota_bytes > 0,
        // Self-hosted UserInfo carries no pressure signal → always "ok".
        status: "ok" as "ok" | "purging" | "full" | "grace",
      };
    }
    const acc = $cloud.account;
    if (acc) {
      return {
        show: true,
        used: acc.storage_used_bytes,
        limit: acc.storage_limit_bytes,
        capped: acc.storage_limit_bytes > 0,
        status: acc.storage_status ?? "ok",
      };
    }
    return {
      show: false,
      used: 0,
      limit: 0,
      capped: false,
      status: "ok" as "ok" | "purging" | "full" | "grace",
    };
  });

  const used = $derived(src.used);
  const quota = $derived(src.limit);
  const capped = $derived(src.capped);

  const pct = $derived(
    capped ? Math.min(100, Math.max(0, (used / quota) * 100)) : 0,
  );

  // Colour tracks the *consequence*, not an arbitrary % threshold: amber only
  // once the server is actually purging old versions to make room, red only
  // when it's full and rejecting uploads. Below that it stays emerald however
  // high the bar climbs, a plan at 85% that isn't purging anything is fine.
  //
  // `grace` is sky: a downgrade is scheduled, so the figures are still those of
  // the old, larger limit and nothing is being deleted, but the rail is the
  // one surface visible from every screen, so it's where a shrink that hasn't
  // happened yet should be noticeable. Account has the date.
  const barClass = $derived(
    src.status === "full"
      ? "bg-red-500"
      : src.status === "purging"
        ? "bg-amber-400"
        : src.status === "grace"
          ? "bg-sky-500"
          : "bg-emerald-500",
  );
  const pctClass = $derived(
    src.status === "full"
      ? "text-red-400"
      : src.status === "purging"
        ? "text-amber-400"
        : src.status === "grace"
          ? "text-sky-300"
          : "text-zinc-300",
  );
</script>

{#if src.show}
  <div class="glow tilt space-y-1.5 rounded-md px-1 py-0.5" title={$_("quota.label")} use:glow use:tilt>
    <div class="flex items-center justify-between gap-2 text-[11px] text-zinc-400">
      <span class="truncate">
        {#if capped}
          {$_("quota.used_of", {
            values: { used: fmtBytes(used), quota: fmtBytes(quota) },
          })}
        {:else}
          {$_("quota.used", { values: { size: fmtBytes(used) } })}
        {/if}
      </span>
      {#if capped}
        <span class="shrink-0 font-semibold tabular-nums {pctClass}">
          <CountUp value={pct} decimals={1} suffix="%" />
        </span>
      {/if}
    </div>
    {#if capped}
      <div class="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
        <div
          class="h-full rounded-full transition-[width] duration-500 ease-out {barClass}"
          style:width="{pct}%"
        ></div>
      </div>
    {/if}
  </div>
{/if}
