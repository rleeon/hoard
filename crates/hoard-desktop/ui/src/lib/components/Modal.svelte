<script lang="ts">
  /**
   * Lightweight modal/dialog. We don't pull in a dialog library — a fixed
   * overlay + a focusable card covers our needs (confirm, restore, edit
   * path) and avoids one more JS dep in the bundle.
   *
   * Pressing Escape closes the modal. Clicking the backdrop also closes,
   * unless `dismissible={false}` (used for irreversible mid-action states
   * like an in-flight restore that we don't want the user to cancel by
   * accident).
   */
  import type { Snippet } from "svelte";
  import { onMount } from "svelte";
  import { X } from "lucide-svelte";

  type Props = {
    open: boolean;
    title: string;
    description?: string;
    dismissible?: boolean;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  };

  let {
    open,
    title,
    description,
    dismissible = true,
    onClose,
    children,
    footer,
  }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open && dismissible) {
      e.stopPropagation();
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown, true);
    return () => window.removeEventListener("keydown", onKeydown, true);
  });

  function backdropClick() {
    if (dismissible) onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/70 backdrop-blur-sm"
    onclick={backdropClick}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      tabindex="-1"
      class="mx-4 w-full max-w-lg rounded-xl border border-zinc-800 bg-zinc-900 shadow-xl"
      onclick={(e) => e.stopPropagation()}
    >
      <header class="flex items-start justify-between gap-4 border-b border-zinc-800 px-5 py-4">
        <div class="min-w-0">
          <h2 id="modal-title" class="text-base font-semibold tracking-tight text-zinc-100">
            {title}
          </h2>
          {#if description}
            <p class="mt-1 text-sm text-zinc-400">{description}</p>
          {/if}
        </div>
        {#if dismissible}
          <button
            type="button"
            onclick={onClose}
            aria-label="Close"
            class="-mr-1 -mt-1 rounded-md p-1 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
          >
            <X size={16} />
          </button>
        {/if}
      </header>

      <div class="px-5 py-4">
        {@render children()}
      </div>

      {#if footer}
        <footer
          class="flex items-center justify-end gap-2 border-t border-zinc-800 px-5 py-3"
        >
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}
