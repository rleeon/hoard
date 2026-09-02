<script lang="ts">
  /**
   * Onboarding step 3 (Cloud branch), accept the legal terms, then kick off
   * the OAuth sign-in. Self-hosted skips this screen entirely (it has no
   * account / hosted ToS). The sign-in only fires once the user has ticked
   * the acceptance box.
   */
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { ExternalLink } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import Button from "../lib/components/Button.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import { startCloudLogin, openExternal } from "../lib/stores/cloud";
  import { toastError } from "../lib/stores/toasts";

  const TERMS_URL = "https://hoard.services/legal/terms";
  const PRIVACY_URL = "https://hoard.services/legal/privacy";

  let accepted = $state(false);
  let loading = $state(false);

  async function continueLogin() {
    if (!accepted || loading) return;
    loading = true;
    try {
      await startCloudLogin();
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      toastError(msg);
    } finally {
      loading = false;
    }
  }

  function back() {
    push("/onboarding/choose");
  }
</script>

<WizardShell step="terms" onBack={back}>
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-xl font-semibold tracking-tight text-zinc-50">
      {$_("onboarding.terms_title")}
    </h1>
    <p class="mt-2 text-sm leading-relaxed text-zinc-400">
      {$_("onboarding.terms_intro")}
    </p>

    <div class="mt-5 flex flex-col gap-2">
      <button
        type="button"
        onclick={() => openExternal(TERMS_URL)}
        class="inline-flex items-center gap-2 text-sm text-emerald-400 hover:text-emerald-300"
      >
        <ExternalLink size={14} />
        {$_("onboarding.terms_link_terms")}
      </button>
      <button
        type="button"
        onclick={() => openExternal(PRIVACY_URL)}
        class="inline-flex items-center gap-2 text-sm text-emerald-400 hover:text-emerald-300"
      >
        <ExternalLink size={14} />
        {$_("onboarding.terms_link_privacy")}
      </button>
    </div>

    <label
      class="mt-6 flex cursor-pointer items-start gap-3 rounded-xl border border-white/[0.08] bg-zinc-950/40 p-4 text-sm text-zinc-200"
    >
      <input
        type="checkbox"
        bind:checked={accepted}
        class="mt-0.5 h-4 w-4 shrink-0 rounded border-zinc-600 bg-zinc-800 text-[var(--color-accent)] focus:ring-[var(--color-accent)]/50"
      />
      <span>{$_("onboarding.terms_accept")}</span>
    </label>

    <div class="mt-8 flex justify-end">
      <Button
        variant="primary"
        size="lg"
        disabled={!accepted || loading}
        onclick={continueLogin}
      >
        {$_("onboarding.terms_continue")}
      </Button>
    </div>
  </div>
</WizardShell>
