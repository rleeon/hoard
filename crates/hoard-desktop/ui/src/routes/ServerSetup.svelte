<script lang="ts">
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { ArrowRight, Server, CheckCircle2 } from "lucide-svelte";
  import Button from "../lib/components/Button.svelte";
  import Input from "../lib/components/Input.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import * as api from "../lib/api";
  import { toastError } from "../lib/stores/toasts";
  import { loadUrl, saveStep, saveUrl } from "../lib/stores/onboarding";
  import { onMount } from "svelte";

  let url = $state("");
  let loading = $state(false);
  let inlineError = $state<string | null>(null);
  let healthy = $state<{ version: string } | null>(null);

  onMount(async () => {
    url = await loadUrl();
  });

  function looksLikeUrl(s: string): boolean {
    return s.startsWith("http://") || s.startsWith("https://");
  }

  async function testConnection() {
    inlineError = null;
    healthy = null;
    const trimmed = url.trim();
    if (!looksLikeUrl(trimmed)) {
      inlineError = "Address must start with http:// or https://";
      return;
    }
    loading = true;
    try {
      const info = await api.healthCheck(trimmed);
      healthy = { version: info.version };
      await saveUrl(trimmed);
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      inlineError = msg;
      toastError(msg);
    } finally {
      loading = false;
    }
  }

  async function next() {
    if (!healthy) {
      await testConnection();
      if (!healthy) return;
    }
    await saveStep("token");
    push("/onboarding/token");
  }

  function back() {
    push("/welcome");
  }

  // Reset the green badge as soon as the user edits the URL again.
  $effect(() => {
    void url;
    healthy = null;
  });
</script>

<WizardShell step="server" onBack={back}>
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-xl font-semibold tracking-tight text-zinc-50">
      Connect to your server
    </h1>
    <p class="mt-2 text-sm text-zinc-400">
      Paste the address of the Hoard server you (or your admin) set up.
    </p>

    <form
      class="mt-6 space-y-4"
      onsubmit={(e) => {
        e.preventDefault();
        void next();
      }}
    >
      <Input
        label="Server address"
        bind:value={url}
        placeholder="https://hoard.example.com:8080"
        hint="The same URL you would open in a web browser."
        icon={Server}
        autocomplete="url"
        spellcheck={false}
        autocapitalize="off"
        error={inlineError}
      />

      <div class="flex flex-wrap items-center gap-3">
        <Button
          variant="secondary"
          onclick={testConnection}
          {loading}
          disabled={!url.trim()}
        >
          Test connection
        </Button>
        {#if healthy}
          <span
            class="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
          >
            <CheckCircle2 size={14} />
            Reached server v{healthy.version}
          </span>
        {/if}
      </div>

      <div class="flex justify-end pt-2">
        <Button
          type="submit"
          variant="primary"
          size="lg"
          disabled={!healthy || loading}
        >
          Continue
          <ArrowRight size={16} />
        </Button>
      </div>
    </form>
  </div>
</WizardShell>
