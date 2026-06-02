<script lang="ts">
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { Server, Cloud } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import { saveStep } from "../lib/stores/onboarding";
  import { startCloudLogin } from "../lib/stores/cloud";
  import { toastError } from "../lib/stores/toasts";

  let cloudLoading = $state(false);

  async function chooseSelfHost() {
    await saveStep("server");
    push("/onboarding/server");
  }

  async function chooseCloud() {
    cloudLoading = true;
    try {
      await startCloudLogin();
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      toastError(msg);
    } finally {
      cloudLoading = false;
    }
  }

  function back() {
    push("/welcome");
  }
</script>

<WizardShell step="choose" onBack={back}>
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-xl font-semibold tracking-tight text-zinc-50">
      {$_("choose.title")}
    </h1>
    <p class="mt-2 text-sm text-zinc-400">
      {$_("choose.subtitle")}
    </p>

    <div class="mt-6 grid grid-cols-2 gap-3">
      <!-- Autohost — black -->
      <button
        type="button"
        onclick={chooseSelfHost}
        class="flex flex-col items-start gap-3 rounded-xl border border-zinc-700 bg-black p-5 text-left transition hover:border-zinc-500 hover:bg-zinc-950 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400/50"
      >
        <span
          class="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-zinc-800 text-zinc-100"
        >
          <Server size={20} />
        </span>
        <span class="text-sm font-semibold text-zinc-50">
          {$_("choose.selfhost_title")}
        </span>
        <span class="text-xs leading-relaxed text-zinc-400">
          {$_("choose.selfhost_desc")}
        </span>
      </button>

      <!-- Gmail / Cloud — green -->
      <button
        type="button"
        onclick={chooseCloud}
        disabled={cloudLoading}
        class="flex flex-col items-start gap-3 rounded-xl border border-emerald-500/40 bg-emerald-600 p-5 text-left transition hover:bg-emerald-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400/50 disabled:opacity-60"
      >
        <span
          class="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-700/60 text-emerald-50"
        >
          <Cloud size={20} />
        </span>
        <span class="text-sm font-semibold text-emerald-50">
          {$_("choose.cloud_title")}
        </span>
        <span class="text-xs leading-relaxed text-emerald-100/80">
          {$_("choose.cloud_desc")}
        </span>
      </button>
    </div>
  </div>
</WizardShell>
