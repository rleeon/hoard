<script lang="ts">
  /**
   * The login the server turned away: this account has all the devices its plan
   * covers and this machine is not one of them.
   *
   * It is a dialog and not a toast because the sign-in did not happen. Someone
   * who just watched a browser round-trip finish needs to be told why they are
   * still signed out, and given the two ways forward: leave, or go Pro. The
   * third way, unlinking a machine, lives on the website (the app cannot list
   * the devices of an account it is not signed into).
   */
  import { _ } from "svelte-i18n";
  import { MonitorSmartphone } from "@lucide/svelte";

  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import { openUpgradePage, openWebAccount } from "../stores/cloud";

  type Props = {
    open: boolean;
    /** Devices the account has, counting the one that was refused. */
    used: number;
    /** What the plan covers. */
    limit: number;
    /** `"free"` / `"pro"`, to word the plan line. */
    plan: string;
    onClose: () => void;
  };

  let { open, used, limit, plan, onClose }: Props = $props();

  // `used` arrives counting the machine the server registered while answering,
  // which we then handed back. What the user has is one less.
  const linked = $derived(Math.max(used - 1, limit));
</script>

<Modal open={open} title={$_("devices.limit_title")} onClose={onClose}>
  <!-- Centred icon, then the text, which is the shape every onboarding screen
       uses (`OnboardingDone`): this dialog lands on top of the wizard, so it
       has to look like it belongs there and not like a web alert. Amber, since
       it is a warning and not a failure. -->
  <div class="flex justify-center">
    <span
      class="flex h-14 w-14 items-center justify-center rounded-full bg-amber-500/10 text-amber-400 ring-1 ring-amber-500/30"
    >
      <MonitorSmartphone size={26} />
    </span>
  </div>
  <div class="mt-4 space-y-3 text-center">
    <p class="text-sm leading-relaxed text-zinc-200">
      {$_("devices.limit_body", {
        values: { plan: plan === "pro" ? "Pro" : "Free", limit, linked },
      })}
    </p>
    <p class="text-sm leading-relaxed text-zinc-400">
      {$_("devices.limit_why")}
    </p>
    <button
      type="button"
      class="text-sm text-emerald-400 underline-offset-2 hover:underline"
      onclick={() => openWebAccount()}
    >
      {$_("devices.limit_manage")}
    </button>
  </div>
  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>
      {$_("devices.limit_back")}
    </Button>
    <Button
      onclick={() => {
        onClose();
        void openUpgradePage("pro");
      }}
    >
      {$_("devices.limit_upgrade")}
    </Button>
  {/snippet}
</Modal>
