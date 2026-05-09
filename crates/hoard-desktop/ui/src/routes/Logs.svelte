<script lang="ts">
  /**
   * Logs viewer — Settings → Advanced gateway into the agent log file.
   *
   * Reads the latest rolling log file from the cache dir and presents the
   * tail. We deliberately keep the UI minimal: a search box, a level filter,
   * a "copy all" button. People come here to copy lines into a bug report,
   * not to do real-time monitoring (the dashboard is for that).
   */
  import { onMount } from "svelte";
  import { ArrowLeft, Search, Copy, RefreshCw } from "lucide-svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import type { LogLine } from "../lib/api";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  let lines = $state<LogLine[]>([]);
  let path = $state("");
  let loading = $state(true);
  let query = $state("");
  let levelFilter = $state<"all" | "WARN" | "ERROR" | "INFO" | "DEBUG">("all");

  onMount(async () => {
    await refresh();
    try {
      path = await api.logsPath();
    } catch {
      /* non-fatal */
    }
  });

  async function refresh() {
    loading = true;
    try {
      lines = await api.tailLogs(1000);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  }

  const filtered = $derived(
    lines.filter((l) => {
      if (levelFilter !== "all" && l.level !== levelFilter) return false;
      if (!query.trim()) return true;
      const q = query.trim().toLowerCase();
      return (
        l.message.toLowerCase().includes(q) ||
        l.timestamp.toLowerCase().includes(q)
      );
    }),
  );

  async function copyAll() {
    try {
      const text = filtered
        .map((l) => `${l.timestamp} ${l.level} ${l.message}`)
        .join("\n");
      await navigator.clipboard.writeText(text);
      toastSuccess(
        $_("logs.copied_toast", { values: { count: filtered.length } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  function levelClass(level: string): string {
    switch (level) {
      case "ERROR":
        return "text-red-400";
      case "WARN":
        return "text-amber-400";
      case "INFO":
        return "text-sky-400";
      case "DEBUG":
        return "text-zinc-500";
      case "TRACE":
        return "text-zinc-600";
      default:
        return "text-zinc-400";
    }
  }
</script>

<div class="mx-auto max-w-4xl px-8 py-8">
  <button
    type="button"
    onclick={() => push("/settings")}
    class="mb-4 inline-flex items-center gap-2 text-sm text-zinc-400 transition-colors hover:text-zinc-100"
  >
    <ArrowLeft size={14} /> {$_("logs.back_to_settings")}
  </button>

  <header class="mb-4 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">{$_("logs.title")}</h1>
      <p class="mt-1 text-sm text-zinc-400">{$_("logs.subtitle")}</p>
      {#if path}
        <p class="mt-2 truncate font-mono text-xs text-zinc-500" title={path}>
          {path}
        </p>
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-2">
      <Button variant="secondary" onclick={refresh} loading={loading}>
        <RefreshCw size={14} /> {$_("logs.reload")}
      </Button>
      <Button
        variant="secondary"
        onclick={copyAll}
        disabled={filtered.length === 0}
      >
        <Copy size={14} /> {$_("logs.copy")}
      </Button>
    </div>
  </header>

  <div class="mb-3 flex flex-wrap items-center gap-3">
    <div class="flex-1 min-w-64">
      <Input
        bind:value={query}
        placeholder={$_("logs.filter_placeholder")}
        icon={Search}
      />
    </div>
    <select
      bind:value={levelFilter}
      class="rounded-md border border-zinc-700 bg-zinc-950/40 px-3 py-2 text-sm text-zinc-200 outline-none focus:border-amber-500"
    >
      <option value="all">{$_("logs.level_all")}</option>
      <option value="ERROR">{$_("logs.level_error")}</option>
      <option value="WARN">{$_("logs.level_warn")}</option>
      <option value="INFO">{$_("logs.level_info")}</option>
      <option value="DEBUG">{$_("logs.level_debug")}</option>
    </select>
  </div>

  {#if loading && lines.length === 0}
    <Card>
      <div class="py-12 text-center text-sm text-zinc-400">
        {$_("common.loading")}
      </div>
    </Card>
  {:else if filtered.length === 0}
    <Card>
      <div class="py-12 text-center">
        <p class="text-sm text-zinc-300">{$_("logs.empty_title")}</p>
        <p class="mt-1 text-xs text-zinc-500">
          {lines.length === 0
            ? $_("logs.empty_no_logs")
            : $_("logs.empty_no_match")}
        </p>
      </div>
    </Card>
  {:else}
    <div
      class="max-h-[60vh] overflow-y-auto rounded-lg border border-zinc-800 bg-zinc-950/60 font-mono text-xs"
    >
      <ul class="divide-y divide-zinc-900">
        {#each filtered as line, i (i)}
          <li class="flex gap-3 px-3 py-1.5 hover:bg-zinc-900/40">
            <span class="shrink-0 text-zinc-600">{line.timestamp}</span>
            <span class="w-12 shrink-0 font-medium {levelClass(line.level)}">
              {line.level || "·"}
            </span>
            <span class="flex-1 break-all text-zinc-300">{line.message}</span>
          </li>
        {/each}
      </ul>
    </div>
    <p class="mt-2 text-right text-xs text-zinc-500">
      {$_("logs.summary", {
        values: { shown: filtered.length, total: lines.length },
      })}
    </p>
  {/if}
</div>
