<script lang="ts">
  /**
   * Lightweight modal/dialog. We don't pull in a dialog library, a fixed
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
  import { X } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  type Props = {
    open: boolean;
    title: string;
    description?: string;
    dismissible?: boolean;
    /** Keeps the header and the footer still and scrolls the body alone. For tall
     *  dialogs where the footer's ways out have to stay visible: by default the
     *  whole card scrolls and the footer ends up right at the bottom, off
     *  screen. */
    scrollBody?: boolean;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  };

  let {
    open,
    title,
    description,
    dismissible = true,
    scrollBody = false,
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
      class="pop mx-4 max-h-[85vh] w-full max-w-lg {scrollBody
        ? 'flex flex-col overflow-hidden'
        : 'overflow-y-auto'}"
      onclick={(e) => e.stopPropagation()}
    >
      <header
        class="flex shrink-0 items-start justify-between gap-4 border-b border-[var(--edge)] px-5 py-4"
      >
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
            aria-label={$_("common.close")}
            class="-mr-1 -mt-1 rounded-md p-1 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
          >
            <X size={16} />
          </button>
        {/if}
      </header>

      <div class="px-5 py-4 {scrollBody ? 'min-h-0 flex-1 overflow-y-auto' : ''}">
        {@render children()}
      </div>

      {#if footer}
        <footer
          class="flex shrink-0 items-center justify-end gap-2 border-t border-[var(--edge)] px-5 py-3"
        >
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}
