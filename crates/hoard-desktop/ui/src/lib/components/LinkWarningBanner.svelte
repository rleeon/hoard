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

  import Button from "./Button.svelte";
  import type { LinkWarning } from "../api";

  type Props = { warnings: LinkWarning[] };

  let { warnings }: Props = $props();

  // Per session, like the mirror banner: back on the next launch if nothing
  // changed, quiet after "not now" within this one.
  let dismissed = $state<Set<string>>(new Set());

  const visible = $derived(warnings.filter((w) => !dismissed.has(w.save_id)));
</script>

{#each visible as w (w.save_id)}
  <div
    class="mb-5 flex items-start gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-200"
  >
    <Link2 size={15} class="mt-0.5 shrink-0 text-amber-400" />
    <div class="min-w-0 flex-1">
      <p class="font-medium">
        {$_("links.title", { values: { game: w.game_slug } })}
      </p>
      <p class="mt-1 text-amber-200/80">{$_("links.body")}</p>
      <ul class="mt-2 space-y-0.5 break-all font-mono text-[11px] text-amber-200/50">
        {#each w.links as l (l.link)}
          <li>{l.link} → {l.target}</li>
        {/each}
      </ul>
      <div class="mt-2.5">
        <Button
          variant="ghost"
          onclick={() => (dismissed = new Set([...dismissed, w.save_id]))}
        >
          {$_("links.dismiss")}
        </Button>
      </div>
    </div>
  </div>
{/each}
