<script lang="ts">
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { ArrowRight, ShieldCheck, Sparkles } from "lucide-svelte";
  import Button from "../lib/components/Button.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import { auth } from "../lib/stores/auth";
  import { clearOnboarding } from "../lib/stores/onboarding";

  async function finish() {
    await clearOnboarding();
    push("/dashboard");
  }
</script>

<WizardShell step="done">
  <div in:fly={{ x: 24, duration: 220 }}>
    <div class="flex justify-center">
      <span
        class="flex h-14 w-14 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-400 ring-1 ring-emerald-500/30"
      >
        <ShieldCheck size={26} />
      </span>
    </div>

    <h1
      class="mt-5 text-center text-2xl font-semibold tracking-tight text-zinc-50"
    >
      {#if $auth.user}
        You're in, {$auth.user.username}!
      {:else}
        You're all set!
      {/if}
    </h1>
    <p class="mt-2 text-center text-sm text-zinc-400">
      Hoard now has everything it needs to keep your saves in sync.
    </p>

    {#if $auth.user}
      <dl
        class="mt-6 space-y-2 rounded-lg border border-zinc-800 bg-zinc-950/40 p-4 text-sm"
      >
        <div class="flex items-center justify-between gap-3">
          <dt class="text-zinc-500">Account</dt>
          <dd class="text-zinc-200">{$auth.user.username}</dd>
        </div>
        <div class="flex items-center justify-between gap-3">
          <dt class="text-zinc-500">Server</dt>
          <dd class="break-all text-right font-mono text-xs text-zinc-300">
            {$auth.user.server_url}
          </dd>
        </div>
        {#if $auth.user.is_admin}
          <div class="flex items-center justify-between gap-3">
            <dt class="text-zinc-500">Role</dt>
            <dd
              class="inline-flex items-center gap-1 rounded-full bg-amber-500/10 px-2 py-0.5 text-xs text-amber-300"
            >
              <Sparkles size={12} /> Administrator
            </dd>
          </div>
        {/if}
      </dl>
    {/if}

    <div class="mt-7 flex justify-center">
      <Button variant="primary" size="lg" onclick={finish}>
        Continue to dashboard
        <ArrowRight size={16} />
      </Button>
    </div>
  </div>
</WizardShell>
