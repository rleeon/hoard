<script lang="ts">
  /**
   * Common chrome for every onboarding screen: vertically-centred card,
   * progress dots, optional back button, and a slot-rendered step name.
   *
   * Keeps each individual route (Welcome / ServerSetup / TokenSetup /
   * OnboardingDone) focused on its own form rather than re-implementing
   * navigation wiring.
   */
  import type { Snippet } from "svelte";
  import { ArrowLeft } from "lucide-svelte";
  import { _ } from "svelte-i18n";
  import Logo from "./Logo.svelte";
  import type { OnboardingStep } from "../stores/onboarding";

  type Props = {
    step: OnboardingStep;
    onBack?: () => void;
    children: Snippet;
  };

  let { step, onBack, children }: Props = $props();

  const ORDER: OnboardingStep[] = ["welcome", "server", "token", "done"];
  const currentIndex = $derived(ORDER.indexOf(step));
</script>

<div
  class="flex min-h-full flex-col items-center justify-center bg-zinc-950 px-6 py-10"
>
  <div class="w-full max-w-md">
    <!-- Logo header -->
    <div class="mb-8 flex justify-center text-emerald-500">
      <Logo size={48} />
    </div>

    <!-- Card -->
    <div
      class="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-8 shadow-xl backdrop-blur"
    >
      {#if onBack}
        <button
          type="button"
          onclick={onBack}
          class="mb-4 inline-flex items-center gap-1 text-sm text-zinc-400 hover:text-zinc-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/50 rounded"
        >
          <ArrowLeft size={14} />
          {$_("common.back")}
        </button>
      {/if}
      {@render children()}
    </div>

    <!-- Progress dots -->
    <div
      class="mt-6 flex items-center justify-center gap-2"
      aria-label={$_("wizard.step_aria", { values: { current: currentIndex + 1, total: ORDER.length } })}
    >
      {#each ORDER as s, i (s)}
        <span
          class="h-1.5 rounded-full transition-all duration-300 {i ===
          currentIndex
            ? 'w-6 bg-emerald-500'
            : i < currentIndex
              ? 'w-1.5 bg-emerald-500/70'
              : 'w-1.5 bg-zinc-700'}"
          aria-hidden="true"
        ></span>
      {/each}
    </div>
  </div>
</div>
