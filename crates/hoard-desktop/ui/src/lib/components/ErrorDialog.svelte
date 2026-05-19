<script lang="ts">
  /**
   * Single-instance error dialog wired to the `errorDialog` store.
   *
   * We render `<Modal>` with `open` driven by `error != null` so the dialog
   * mounts/unmounts cleanly. `title` and `body` are i18n keys — when they
   * don't resolve (e.g. an `AppError::plain(raw_msg)` wrapping a verbatim
   * Rust error) svelte-i18n returns the key as-is, which still produces a
   * readable line for the user. The optional `detail` is hidden behind a
   * "Show details" toggle to keep the modal calm by default.
   */
  import { _ } from "svelte-i18n";
  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";

  type AppErrorShape = {
    title: string;
    body: string;
    detail?: string | null;
  };

  let {
    error,
    onClose,
  }: {
    error: AppErrorShape | null;
    onClose: () => void;
  } = $props();

  let showDetail = $state(false);

  // Reset the toggle each time a new error pops so we don't surface the raw
  // detail by default after the user expanded a previous one.
  $effect(() => {
    if (error) showDetail = false;
  });
</script>

{#if error}
  <Modal
    open={true}
    title={$_(error.title)}
    {onClose}
    dismissible
  >
    <p class="whitespace-pre-line text-sm text-zinc-300">{$_(error.body)}</p>

    {#if error.detail}
      <button
        type="button"
        class="mt-3 text-xs text-zinc-400 underline-offset-2 transition-colors hover:text-zinc-200 hover:underline"
        onclick={() => (showDetail = !showDetail)}
      >
        {showDetail ? $_("common.hide_details") : $_("common.show_details")}
      </button>
      {#if showDetail}
        <pre
          class="mt-2 max-h-60 overflow-auto whitespace-pre-wrap break-all rounded border border-zinc-800 bg-zinc-950 p-2 text-xs text-zinc-400">{error.detail}</pre>
      {/if}
    {/if}

    {#snippet footer()}
      <Button variant="secondary" onclick={onClose}>
        {$_("common.close")}
      </Button>
    {/snippet}
  </Modal>
{/if}
