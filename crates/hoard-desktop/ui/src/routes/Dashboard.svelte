<script lang="ts">
  import { Sparkles, Send, LogOut } from "lucide-svelte";
  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Input from "../lib/components/Input.svelte";
  import * as api from "../lib/api";
  import { auth, signOut } from "../lib/stores/auth";
  import { toastError, toastSuccess } from "../lib/stores/toasts";

  let name = $state("alice");
  let greeting = $state<string | null>(null);
  let loading = $state(false);
  let signingOut = $state(false);

  async function sayHi() {
    loading = true;
    try {
      greeting = await api.greet(name);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loading = false;
    }
  }

  async function handleLogout() {
    signingOut = true;
    try {
      await signOut();
      toastSuccess("Signed out.");
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      signingOut = false;
    }
  }
</script>

<div class="mx-auto max-w-3xl px-8 py-10">
  <header class="mb-8 flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight">
        {#if $auth.user}
          Welcome back, {$auth.user.username}
        {:else}
          Dashboard
        {/if}
      </h1>
      <p class="mt-1 text-sm text-zinc-400">
        Phase&nbsp;1 placeholder. Auto-detection lands in phase&nbsp;3.
      </p>
    </div>
    <Button
      variant="ghost"
      onclick={handleLogout}
      loading={signingOut}
      aria-label="Sign out"
    >
      <LogOut size={16} />
      Sign out
    </Button>
  </header>

  <Card>
    <div class="mb-4 flex items-center gap-2 text-amber-500">
      <Sparkles size={18} />
      <h2 class="text-sm font-medium uppercase tracking-wider">
        Tauri ↔ Svelte bridge check
      </h2>
    </div>

    <p class="mb-4 text-sm text-zinc-400">
      Type a name and press the button. The frontend will call the Rust
      <code class="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-xs">
        greet
      </code>
      command and show the response below.
    </p>

    <div class="flex items-end gap-2">
      <Input bind:value={name} placeholder="your name" class="flex-1" />
      <Button onclick={sayHi} {loading} disabled={!name.trim()}>
        <Send size={16} />
        {loading ? "Sending…" : "Greet"}
      </Button>
    </div>

    {#if greeting}
      <div
        class="mt-5 rounded-md border border-amber-500/30 bg-amber-500/5 px-4 py-3 text-sm text-amber-200"
      >
        {greeting}
      </div>
    {/if}
  </Card>
</div>
