<script lang="ts">
  /**
   * The one button that sells Pro. It lives in the sidebar, where it is always
   * on screen, and on the account page, and both have to be the same button:
   * two different-looking upgrade buttons in one window read as two different
   * things.
   *
   * What it costs hangs off the hover instead of sitting under the button as a
   * paragraph. The price is the argument, so it has to be somewhere; a wall of
   * small print next to a button is not that place.
   */
  import { _ } from "svelte-i18n";
  import { Sparkles } from "@lucide/svelte";
  import { openUpgradePage } from "../stores/cloud";

  type Props = {
    /** Which side the price opens on. The sidebar sits at the bottom of the
     *  window, so its tooltip has nowhere to go but up. */
    placement?: "top" | "bottom";
  };

  let { placement = "bottom" }: Props = $props();
</script>

<div class="group relative shrink-0">
  <button
    type="button"
    onclick={() => openUpgradePage("pro")}
    class="keep-emerald flex shrink-0 items-center gap-1 rounded-md bg-gradient-to-r from-emerald-400 to-teal-400 px-2.5 py-2 text-[11px] font-semibold text-emerald-950 shadow-sm shadow-emerald-500/30 transition-all hover:from-emerald-300 hover:to-teal-300 hover:shadow-emerald-500/50"
  >
    <Sparkles size={12} data-anim="pop" />
    {$_("sidebar.upgrade")}
  </button>
  <span
    class="pointer-events-none absolute right-0 z-30 w-56 rounded-lg border border-zinc-800 bg-zinc-950 p-2.5 text-[11px] leading-relaxed text-zinc-300 opacity-0 shadow-lg transition-opacity group-hover:opacity-100 {placement ===
    'top'
      ? 'bottom-full mb-2'
      : 'top-full mt-2'}"
  >
    {$_("pro_trust.tooltip")}
  </span>
</div>
