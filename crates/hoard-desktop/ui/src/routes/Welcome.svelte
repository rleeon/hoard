<script lang="ts">
  import { push } from "svelte-spa-router";
  import { ArrowRight } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import Button from "../lib/components/Button.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import { saveStep } from "../lib/stores/onboarding";

  async function start() {
    await saveStep("server");
    push("/onboarding/server");
  }
</script>

<WizardShell step="welcome">
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-2xl font-semibold tracking-tight text-zinc-50">
      Welcome to Hoard
    </h1>
    <p class="mt-2 text-sm leading-relaxed text-zinc-400">
      Cloud sync for your game saves &mdash; on
      <strong class="text-zinc-200">your</strong> server, with
      <strong class="text-zinc-200">your</strong> data. Hoard runs quietly in
      the background and uploads new saves whenever you finish playing.
    </p>

    <ul class="mt-6 space-y-2 text-sm text-zinc-300">
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        Detects games you have installed automatically.
      </li>
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        Backs up after each session, never while a game is running.
      </li>
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        Restores any past version with two clicks.
      </li>
    </ul>

    <div class="mt-8 flex justify-end">
      <Button variant="primary" size="lg" onclick={start}>
        Get started
        <ArrowRight size={16} />
      </Button>
    </div>
  </div>
</WizardShell>
