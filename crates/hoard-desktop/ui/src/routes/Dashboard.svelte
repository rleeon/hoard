<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Sparkles, Send } from "lucide-svelte";

  let name = $state("alice");
  let greeting = $state<string | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function sayHi() {
    loading = true;
    error = null;
    try {
      greeting = await invoke<string>("greet", { name });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="mx-auto max-w-3xl px-8 py-10">
  <header class="mb-8">
    <h1 class="text-2xl font-semibold tracking-tight">Welcome to Hoard</h1>
    <p class="mt-1 text-sm text-zinc-400">
      Phase&nbsp;0 scaffold. Real onboarding lands in phase&nbsp;1.
    </p>
  </header>

  <section
    class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 shadow-sm"
  >
    <div class="mb-4 flex items-center gap-2 text-amber-500">
      <Sparkles size={18} />
      <h2 class="text-sm font-medium uppercase tracking-wider">
        Tauri ↔ Svelte bridge check
      </h2>
    </div>

    <p class="mb-4 text-sm text-zinc-400">
      Type a name and click the button. The frontend will call the Rust
      <code class="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-xs"
        >greet</code
      > command and show the response below.
    </p>

    <div class="flex items-center gap-2">
      <input
        bind:value={name}
        type="text"
        class="flex-1 rounded-md border border-zinc-700 bg-zinc-950/40 px-3 py-2 text-sm placeholder-zinc-500 outline-none focus:border-amber-500 focus:ring-1 focus:ring-amber-500"
        placeholder="your name"
      />
      <button
        type="button"
        onclick={sayHi}
        disabled={loading || !name.trim()}
        class="inline-flex items-center gap-2 rounded-md bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition-colors hover:bg-amber-400 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
      >
        <Send size={16} />
        {loading ? "Sending…" : "Greet"}
      </button>
    </div>

    {#if greeting}
      <div
        class="mt-5 rounded-md border border-amber-500/30 bg-amber-500/5 px-4 py-3 text-sm text-amber-200"
      >
        {greeting}
      </div>
    {/if}

    {#if error}
      <div
        class="mt-5 rounded-md border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-200"
      >
        Error: {error}
      </div>
    {/if}
  </section>
</div>
