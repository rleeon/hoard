<script lang="ts">
  /**
   * "Your year of Pro is running out." Shown on the same days as the emails,
   * from 25 days before the year ends to the end of the month of Pro after it.
   *
   * Unlike the storage card this one can be put away: nothing has stopped,
   * the account is still Pro. Putting it away lasts until the next reminder.
   */
  import { _ } from "svelte-i18n";
  import { fly } from "svelte/transition";
  import { CalendarClock, X } from "@lucide/svelte";

  import { openUpgradePage } from "../stores/cloud";
  import { dismissPassReminder, passReminder } from "../stores/pass";
  import { fmtDate } from "../utils/format";

  let opening = $state(false);

  const day = (iso: string) =>
    $fmtDate(iso, { year: "numeric", month: "long", day: "numeric" });

  // Same lock as the pricing page: creating the checkout takes a second or
  // two, and every extra click is another checkout.
  async function buy() {
    if (opening) return;
    opening = true;
    try {
      await openUpgradePage("pro", "once");
    } finally {
      opening = false;
    }
  }
</script>

{#if $passReminder}
  {@const p = $passReminder}
  <div
    class="pointer-events-auto overflow-hidden rounded-lg border border-amber-500/40 bg-layer-3 shadow-xl"
    role="status"
    aria-live="polite"
    transition:fly={{ y: 12, duration: 180 }}
  >
    <div class="flex items-start gap-2.5 bg-amber-500/10 p-3">
      <span class="mt-0.5 shrink-0 text-amber-400" aria-hidden="true">
        <CalendarClock size={16} />
      </span>
      <div class="min-w-0 flex-1">
        <p class="text-xs leading-snug text-zinc-100">
          {#if p.phase === "ending"}
            {$_("pass.ending", {
              values: { days: p.daysLeft, end: day(p.yearEnds), until: day(p.proUntil) },
            })}
          {:else}
            {$_("pass.grace", {
              values: { days: p.daysLeft, end: day(p.yearEnds), until: day(p.proUntil) },
            })}
          {/if}
        </p>
        <button
          type="button"
          class="mt-2 rounded-md border border-emerald-600/60 bg-black px-2.5 py-1 text-[11px] font-medium text-emerald-300 transition-colors hover:border-emerald-500 disabled:opacity-60 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500"
          disabled={opening}
          onclick={buy}
        >
          {$_("pass.buy")}
        </button>
      </div>
      <button
        type="button"
        class="shrink-0 text-zinc-500 transition-colors hover:text-zinc-200"
        aria-label={$_("common.close")}
        onclick={() => dismissPassReminder(p.key)}
      >
        <X size={14} />
      </button>
    </div>
  </div>
{/if}
