<script lang="ts">
  import { ChevronLeft, ChevronRight } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  let {
    page,
    pages,
    onPage,
    compact = false,
  }: {
    /** Zero-based. */
    page: number;
    pages: number;
    onPage: (page: number) => void;
    /** Among the small tiles (the cloud saves), so it doesn't make its row
     *  taller than theirs. */
    compact?: boolean;
  } = $props();

  const first = $derived(page <= 0);
  const last = $derived(page >= pages - 1);

  // aria-disabled and not disabled: a button that goes disabled under the
  // focus drops it to the body, and on the last page the arrows lost their
  // place.
  const half =
    "flex flex-1 items-center justify-center text-zinc-400 transition-colors hover:bg-[var(--surface-2)] hover:text-zinc-100 aria-disabled:cursor-default aria-disabled:text-zinc-700 aria-disabled:hover:bg-transparent";
</script>

<div
  role="group"
  aria-label={$_("common.page_of", { values: { page: page + 1, total: pages } })}
  class="panel relative flex overflow-hidden {compact ? 'min-h-16' : 'min-h-24'}"
>
  <button
    type="button"
    aria-label={$_("common.page_previous")}
    title={$_("common.page_previous")}
    aria-disabled={first}
    onclick={() => {
      if (!first) onPage(page - 1);
    }}
    class={half}
  >
    <ChevronLeft size={compact ? 20 : 28} />
  </button>
  <span class="w-px self-stretch bg-[var(--edge)]"></span>
  <button
    type="button"
    aria-label={$_("common.page_next")}
    title={$_("common.page_next")}
    aria-disabled={last}
    onclick={() => {
      if (!last) onPage(page + 1);
    }}
    class={half}
  >
    <ChevronRight size={compact ? 20 : 28} />
  </button>
  <span
    class="pointer-events-none absolute inset-x-0 text-center tabular-nums text-zinc-500 {compact
      ? 'bottom-1 text-[10px]'
      : 'bottom-2 text-[11px]'}"
  >
    {page + 1} / {pages}
  </span>
</div>
