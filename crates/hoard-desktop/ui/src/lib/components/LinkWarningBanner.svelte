<script lang="ts">
  /**
   * "Part of this folder isn't being backed up."
   *
   * The backup never follows a symbolic link: it can lead anywhere, and in a
   * Wine prefix one leads to `/`. That used to be silent, and EmuDeck builds
   * `Emulation/saves/<emulator>` out of nothing but links, so tracking it
   * uploaded an empty folder with no hint as to why. Advisory only: the fix is
   * tracking the folder the link points at, which is the user's call.
   */
  import { Link2 } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import { customNames } from "../stores/gameNames";
  import { prettifySlug } from "../utils/format";
  import { dismiss, isDismissed } from "../stores/dismissedWarnings.svelte";

  import Button from "./Button.svelte";
  import { glow } from "../actions/glow";
  import type { LinkWarning } from "../api";

  type Props = {
    warnings: LinkWarning[];
    /** How many are on screen, for the tray's count. */
    shown?: number;
  };

  let { warnings, shown = $bindable(0) }: Props = $props();

  // Per session, like the mirror banner: back on the next launch if nothing
  // changed, quiet after "not now" within this one.

  const visible = $derived(
    warnings.filter((w) => !isDismissed("links", w.save_id)),
  );
  $effect(() => {
    shown = visible.length;
  });
</script>

{#each visible as w (w.save_id)}
  <div
    use:glow
    class="glow tray-card mb-2 flex items-start gap-2.5 rounded-md border border-amber-500/25 bg-amber-500/[0.08] px-3.5 py-2.5 text-[13px] text-amber-200 last:mb-0 hover:-translate-y-0.5 hover:border-amber-500/55 hover:bg-amber-500/[0.15] hover:shadow-[0_10px_26px_-14px_rgb(245_158_11/0.55)]"
  >
    <Link2 size={15} class="mt-0.5 shrink-0 text-amber-400" data-anim="pop" />
    <div class="min-w-0 flex-1">
      <p class="font-medium">
        {$_("links.title", {
          values: {
            game: $customNames[w.game_slug] ?? prettifySlug(w.game_slug),
          },
        })}
      </p>
      <p class="mt-1 text-amber-200/80">{$_("links.body")}</p>
      <ul class="mt-2 space-y-0.5 break-all font-mono text-[11px] text-amber-200/50">
        {#each w.links as l (l.link)}
          <li>{l.link} → {l.target}</li>
        {/each}
      </ul>
      <div class="mt-1.5">
        <Button
          variant="ghost"
          onclick={() => dismiss("links", w.save_id)}
        >
          {$_("links.dismiss")}
        </Button>
      </div>
    </div>
  </div>
{/each}
