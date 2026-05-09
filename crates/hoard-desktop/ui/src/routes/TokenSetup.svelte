<script lang="ts">
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { KeyRound, ArrowRight } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import Button from "../lib/components/Button.svelte";
  import Input from "../lib/components/Input.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import { signIn } from "../lib/stores/auth";
  import { loadUrl, saveStep } from "../lib/stores/onboarding";
  import { toastError } from "../lib/stores/toasts";
  import { onMount } from "svelte";

  let token = $state("");
  let url = $state("");
  let loading = $state(false);
  let inlineError = $state<string | null>(null);

  const TOKEN_RE = /^hoard_v1_[0-9a-f]{64}$/;

  onMount(async () => {
    url = await loadUrl();
    if (!url) {
      // Should not happen — defensively send the user back to the URL step.
      push("/onboarding/server");
    }
  });

  function back() {
    push("/onboarding/server");
  }

  async function submit() {
    inlineError = null;
    const trimmed = token.trim();
    if (!TOKEN_RE.test(trimmed)) {
      inlineError = $_("token.invalid_key");
      return;
    }
    loading = true;
    try {
      await signIn(url, trimmed);
      await saveStep("done");
      push("/onboarding/done");
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      inlineError = msg;
      toastError(msg);
    } finally {
      loading = false;
    }
  }
</script>

<WizardShell step="token" onBack={back}>
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-xl font-semibold tracking-tight text-zinc-50">
      {$_("token.title")}
    </h1>
    <p class="mt-2 text-sm text-zinc-400">
      {$_("token.subtitle")}
      <span
        class="break-all rounded bg-zinc-800/80 px-1.5 py-0.5 font-mono text-xs text-zinc-200"
      >
        {url || $_("token.your_server")}
      </span>.
    </p>

    <form
      class="mt-6 space-y-4"
      onsubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <Input
        label={$_("token.access_key_label")}
        bind:value={token}
        type="password"
        placeholder="hoard_v1_…"
        hint={$_("token.access_key_hint")}
        icon={KeyRound}
        autocomplete="off"
        spellcheck={false}
        autocapitalize="off"
        error={inlineError}
      />

      <div class="flex justify-end pt-2">
        <Button
          type="submit"
          variant="primary"
          size="lg"
          {loading}
          disabled={!token.trim()}
        >
          {$_("token.sign_in")}
          <ArrowRight size={16} />
        </Button>
      </div>
    </form>
  </div>
</WizardShell>
