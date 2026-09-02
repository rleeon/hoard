<script lang="ts">
  /**
   * Blocking screen shown when the signed-in cloud account is scheduled for
   * deletion (`$cloud.account.deleted_at` is set). The account is frozen
   * server-side during its 30-day grace, every data route 403s, so we replace
   * the whole app with this until the user either reactivates or signs out.
   *
   * Reactivate (green) cancels the pending delete; "Cerrar sesión" (neutral)
   * just drops the local session. There is no dismiss: the app behind is
   * non-functional while frozen.
   */
  import { _ } from "svelte-i18n";
  import { AlertTriangle } from "@lucide/svelte";
  import Button from "./Button.svelte";
  import { cloud, reactivateCloudAccount, logoutCloud } from "../stores/cloud";
  import { toastError } from "../stores/toasts";

  let busy = $state(false);

  const purgesAt = $derived($cloud.account?.purges_at ?? null);
  const purgeDate = $derived(
    purgesAt
      ? new Date(purgesAt).toLocaleDateString(undefined, {
          year: "numeric",
          month: "long",
          day: "numeric",
        })
      : null,
  );

  async function reactivate() {
    if (busy) return;
    busy = true;
    try {
      await reactivateCloudAccount();
    } catch (e) {
      toastError($_("account_deleted.reactivate_error", { values: { error: String(e) } }));
    } finally {
      busy = false;
    }
  }

  async function signOut() {
    if (busy) return;
    busy = true;
    try {
      await logoutCloud();
    } catch (e) {
      toastError(String(e));
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-[120] flex items-center justify-center bg-zinc-950/85 p-6 backdrop-blur-sm"
  role="dialog"
  aria-modal="true"
  aria-label={$_("account_deleted.title")}
>
  <div
    class="w-full max-w-md rounded-2xl border border-amber-500/40 bg-zinc-900/95 p-8 shadow-2xl"
  >
    <div class="flex items-center gap-3">
      <span
        class="inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-amber-500/15 text-amber-400"
      >
        <AlertTriangle size={24} />
      </span>
      <h2 class="text-lg font-semibold tracking-tight text-zinc-50">
        {$_("account_deleted.title")}
      </h2>
    </div>

    <p class="mt-4 text-sm leading-relaxed text-zinc-300">
      {#if purgeDate}
        {$_("account_deleted.body", { values: { date: purgeDate } })}
      {:else}
        {$_("account_deleted.body_no_date")}
      {/if}
    </p>
    <p class="mt-3 text-sm leading-relaxed text-zinc-400">
      {$_("account_deleted.hint")}
    </p>

    <div class="mt-8 flex items-center justify-between gap-3">
      <button
        type="button"
        onclick={signOut}
        disabled={busy}
        class="rounded-lg px-3 py-2 text-sm font-medium text-zinc-400 transition hover:bg-zinc-800 hover:text-zinc-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500/50 disabled:opacity-50"
      >
        {$_("account_deleted.sign_out")}
      </button>
      <Button variant="primary" size="lg" onclick={reactivate} disabled={busy}>
        {busy ? $_("account_deleted.reactivating") : $_("account_deleted.reactivate")}
      </Button>
    </div>
  </div>
</div>
