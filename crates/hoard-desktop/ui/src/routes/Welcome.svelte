<script lang="ts">
  import { push } from "svelte-spa-router";
  import { ArrowRight } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { _ } from "svelte-i18n";
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
      {$_("welcome.title")}
    </h1>
    <p class="mt-2 text-sm leading-relaxed text-zinc-400">
      {$_("welcome.tagline_1")}
      <strong class="text-zinc-200">{$_("welcome.tagline_your")}</strong>
      {$_("welcome.tagline_2")}
      <strong class="text-zinc-200">{$_("welcome.tagline_your")}</strong>
      {$_("welcome.tagline_3")}
    </p>

    <ul class="mt-6 space-y-2 text-sm text-zinc-300">
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        {$_("welcome.bullet_1")}
      </li>
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        {$_("welcome.bullet_2")}
      </li>
      <li class="flex items-start gap-2">
        <span class="mt-1 inline-block h-1.5 w-1.5 rounded-full bg-amber-500"
        ></span>
        {$_("welcome.bullet_3")}
      </li>
    </ul>

    <div class="mt-8 flex justify-end">
      <Button variant="primary" size="lg" onclick={start}>
        {$_("welcome.get_started")}
        <ArrowRight size={16} />
      </Button>
    </div>
  </div>
</WizardShell>
