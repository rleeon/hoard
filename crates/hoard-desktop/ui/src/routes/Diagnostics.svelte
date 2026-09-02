<script lang="ts">
  /**
   * Hidden detection-diagnostics panel.
   *
   * Unlocked via the 5-click gesture on the sidebar version button (the same
   * gesture that reveals the Agent Diagnostics card in Settings). Lets the
   * user (or a triager) type a slug and replay the detection pipeline,
   * surfacing every step's input/output. The Tauri command is read-only,
   * nothing here writes to the detection cache or `state.json`.
   */
  import { ArrowLeft, Search } from "@lucide/svelte";
  import { push } from "svelte-spa-router";
  import { _ } from "svelte-i18n";

  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import { toastError } from "../lib/stores/toasts";

  let slug = $state("");
  let running = $state(false);
  let trace = $state<api.DetectionTrace | null>(null);

  async function run() {
    if (!slug.trim()) return;
    running = true;
    try {
      trace = await api.detectionDiagnostics(slug.trim());
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      trace = null;
    } finally {
      running = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") run();
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

  <header class="mb-4">
    <h1 class="text-2xl font-semibold tracking-tight">
      {$_("diagnostics.title")}
    </h1>
  </header>

  <Card>
    <div class="flex flex-wrap items-end gap-3">
      <div class="min-w-64 flex-1">
        <label
          for="diagnostics-slug"
          class="mb-1 block text-xs font-medium text-zinc-400"
        >
          {$_("diagnostics.slug_label")}
        </label>
        <Input
          id="diagnostics-slug"
          bind:value={slug}
          onkeydown={onKey}
          placeholder="stardew-valley"
          icon={Search}
        />
      </div>
      <Button onclick={run} loading={running} disabled={!slug.trim()}>
        {$_("diagnostics.run_button")}
      </Button>
    </div>
  </Card>

  <div class="mt-4">
    {#if trace == null}
      <Card>
        <p class="py-8 text-center text-sm text-zinc-500">
          {$_("diagnostics.no_trace")}
        </p>
      </Card>
    {:else}
      <Card>
        <pre
          class="max-h-[70vh] overflow-auto rounded-md border border-zinc-800 bg-zinc-950/60 p-3 font-mono text-xs leading-relaxed text-zinc-200">{JSON.stringify(
            trace,
            null,
            2,
          )}</pre>
      </Card>
    {/if}
  </div>
</div>
