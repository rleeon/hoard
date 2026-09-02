<script lang="ts">
  /**
   * The body of the *free up space* dialog: the list of games with their weight,
   * the checkboxes for picking which get archived, and the meter showing where that
   * leaves the account.
   *
   * It lives apart from {@link LiberateStorageModal} because two different places
   * show it, for different reasons:
   *
   *   - the *free up space* dialog, when the account is **already** over the limit
   *     and the sync is stopped;
   *   - the Pro farewell, when it is not over yet but is going to be: there the
   *     meter measures against the limit the account is **falling to**
   *     (`limitOverride`), not the one it has today, which stays the larger one
   *     during the grace window. Without that override the panel would say "plenty
   *     of room" on the day you cancel and "nothing fits" a month later, with
   *     nothing having changed in between.
   *
   * The selection belongs to the parent (`bind:selected`) because the button that
   * acts on it is in each dialog's footer, and each places it its own way.
   */
  import { _ } from "svelte-i18n";
  import { Link2 } from "@lucide/svelte";

  import {
    storageGamesCloud,
    type StorageGame,
    type SharedGroup,
  } from "../stores/cloud";
  import { formatBytes } from "../utils/format";

  type Props = {
    /** Carga los datos al pasar a `true`. */
    open: boolean;
    /** Bloquea las casillas mientras el padre archiva. */
    busy?: boolean;
    /** The limit to measure against, instead of the one the server applies today. */
    limitOverride?: number | null;
    /** Change it to re-read from the server (after archiving, say). */
    reloadKey?: number;
    /** The ticked saves' ids. The parent's, since the parent is what archives. */
    selected?: Set<string>;
  };

  let {
    open,
    busy = false,
    limitOverride = null,
    reloadKey = 0,
    selected = $bindable(new Set<string>()),
  }: Props = $props();

  /** When a shared group is worth talking about.
   *
   *  The notice exists to prevent an expensive mistake: archiving half a pair and
   *  freeing nothing. Under a megabyte that mistake does not exist, there is nothing
   *  to free, and the row ended up saying "shares 0 B with Factorio: archive both to
   *  free it", which also dragged the partner into the selection without gaining a
   *  byte.
   *
   *  It is a **presentation** threshold: the group's bytes still count in the meter
   *  whatever happens. Hiding bytes the account pays for would be exactly the
   *  failure `shared_groups` came to fix. */
  const SHARED_HINT_FLOOR_BYTES = 1024 * 1024;

  let loading = $state(false);
  let usedBytes = $state(0);
  let serverLimitBytes = $state(0);
  let games = $state<StorageGame[]>([]);
  let sharedGroups = $state<SharedGroup[]>([]);
  let loadError = $state<string | null>(null);

  /** The groups that do get talked about (a notice on the row, plus dragging the
   *  partner into the suggested selection). A subset of `sharedGroups`, never a
   *  replacement: the calculation uses the whole list. */
  const notableGroups = $derived(
    sharedGroups.filter((g) => g.bytes >= SHARED_HINT_FLOOR_BYTES),
  );

  /** The limit that rules: the one the parent asks for, or the one the server applies. */
  const limitBytes = $derived(
    limitOverride != null && limitOverride > 0 ? limitOverride : serverLimitBytes,
  );
  /** Recomputed here rather than read from `over_bytes`: with `limitOverride` the
   *  server's number measures against a different bar. */
  const overBytes = $derived(
    limitBytes > 0 ? Math.max(0, usedBytes - limitBytes) : 0,
  );

  $effect(() => {
    // Leer `reloadKey` dentro del efecto es lo que hace que cambiarlo relea.
    void reloadKey;
    if (open) void load();
  });

  async function load() {
    loading = true;
    loadError = null;
    try {
      const data = await storageGamesCloud();
      usedBytes = data.used_bytes;
      serverLimitBytes = data.limit_bytes;
      // Every group, unfiltered: these are bytes the account pays for and the meter
      // has to count them. The floor below only decides which get talked about,
      // never which count.
      sharedGroups = data.shared_groups ?? [];
      // Archived saves are already out of the quota; everything else is a
      // candidate, including games whose own bytes are all shared
      // (`freeable_bytes === 0`), which the old filter dropped and which are
      // precisely the ones holding a duplicate hostage.
      games = data.games.filter((g) => !g.archived);
      selected = suggestion();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }

  /** Bytes reclaimed by archiving `ids`: every selected game's exclusive bytes,
   *  plus each shared group whose *whole* set is selected. */
  function freedBy(ids: Set<string>): number {
    let total = 0;
    for (const g of games) {
      if (ids.has(g.save_id)) total += g.freeable_bytes;
    }
    for (const grp of sharedGroups) {
      if (grp.save_ids.every((id) => ids.has(id))) total += grp.bytes;
    }
    return total;
  }

  /** Opening selection: heaviest-first until the overage is covered, counting
   *  shared bytes so a duplicate pair gets picked together instead of one half
   *  being ticked for no gain. */
  function suggestion(): Set<string> {
    const sel = new Set<string>();
    if (overBytes <= 0) return sel;
    // Weight = own bytes + a share of every group it belongs to, so a game
    // whose bytes are all shared isn't ranked as worthless.
    const weight = (g: StorageGame) =>
      g.freeable_bytes +
      sharedGroups
        .filter((grp) => grp.save_ids.includes(g.save_id))
        .reduce((n, grp) => n + grp.bytes / grp.save_ids.length, 0);
    const ordered = [...games].sort((a, b) => weight(b) - weight(a));
    for (const g of ordered) {
      if (freedBy(sel) >= overBytes) break;
      sel.add(g.save_id);
      // Pull in the rest of any group this game belongs to: half a duplicate
      // frees nothing, so ticking it alone would be a lie in the meter. Only the
      // notable ones: dragging a whole game in to recover 52 bytes is ticking
      // something for the user that they never asked for, in exchange for nothing.
      for (const grp of notableGroups) {
        if (grp.save_ids.includes(g.save_id)) {
          for (const id of grp.save_ids) sel.add(id);
        }
      }
    }
    return sel;
  }

  function toggle(saveId: string) {
    const next = new Set(selected);
    if (next.has(saveId)) next.delete(saveId);
    else next.add(saveId);
    selected = next;
  }

  const freed = $derived(freedBy(selected));
  const remaining = $derived(Math.max(0, usedBytes - freed));
  const fits = $derived(limitBytes > 0 && remaining <= limitBytes);
  /** Everything selectable, for the "not even archiving it all fits" check. */
  const maxFreeable = $derived(freedBy(new Set(games.map((g) => g.save_id))));
  const hopeless = $derived(
    !loading &&
      games.length > 0 &&
      limitBytes > 0 &&
      usedBytes - maxFreeable > limitBytes,
  );

  /** Games sharing bytes with the given one, the "this is the same save twice"
   *  hint. */
  function partners(saveId: string): StorageGame[] {
    const ids = new Set<string>();
    for (const grp of notableGroups) {
      if (!grp.save_ids.includes(saveId)) continue;
      for (const id of grp.save_ids) if (id !== saveId) ids.add(id);
    }
    return games.filter((g) => ids.has(g.save_id));
  }

  /** Bytes of this game that only come back if its partners go too. */
  function sharedBytesOf(saveId: string): number {
    return notableGroups
      .filter((grp) => grp.save_ids.includes(saveId))
      .reduce((n, grp) => n + grp.bytes, 0);
  }

  // The list is one row per save; `game_slug` is the game and `label` is the
  // save slot (almost always the default "main"). Show the game name, turn the
  // slug into a title, and only surface the label when it disambiguates.
  const ROMAN = new Set([
    "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix",
    "x", "xi", "xii", "xiii", "xiv", "xv",
  ]);
  function prettifyGame(slug: string): string {
    return slug
      .split("-")
      .map((w) =>
        !w
          ? w
          : ROMAN.has(w)
            ? w.toUpperCase()
            : w[0].toUpperCase() + w.slice(1),
      )
      .join(" ");
  }
  const gameName = (g: StorageGame) => prettifyGame(g.game_slug);
  const saveLabel = (g: StorageGame) =>
    g.label && g.label.toLowerCase() !== "main" ? g.label : null;
</script>

{#if hopeless}
  <p
    class="mb-3 rounded-lg border border-rose-500/40 bg-rose-500/10 p-2.5 text-xs text-rose-200/90"
  >
    {$_("liberate.hopeless", { values: { limit: formatBytes(limitBytes) } })}
  </p>
{/if}

<div>
  <p class="mb-2 text-xs font-medium uppercase tracking-wide text-zinc-500">
    {$_("liberate.pick")}
  </p>

  {#if loading}
    <p class="text-sm text-zinc-500">{$_("liberate.loading")}</p>
  {:else if loadError}
    <div class="flex items-center justify-between gap-3">
      <p class="text-sm text-rose-400">{$_("liberate.load_error")}</p>
      <button
        type="button"
        onclick={() => void load()}
        class="shrink-0 rounded-lg border border-white/10 bg-zinc-900 px-3 py-1.5 text-xs font-medium text-zinc-200 transition-colors hover:bg-zinc-800"
      >
        {$_("liberate.retry")}
      </button>
    </div>
  {:else if games.length === 0}
    <p class="text-sm text-zinc-500">{$_("liberate.nothing")}</p>
  {:else}
    <ul class="max-h-64 space-y-1.5 overflow-y-auto pr-1">
      {#each games as g (g.save_id)}
        {@const willArchive = selected.has(g.save_id)}
        {@const twins = partners(g.save_id)}
        {@const shared = sharedBytesOf(g.save_id)}
        <li>
          <label
            class="flex cursor-pointer items-center justify-between gap-3 rounded-lg border px-3 py-2 text-sm {willArchive
              ? 'border-rose-500/40 bg-rose-500/10'
              : 'border-white/[0.08] bg-zinc-950/40'}"
          >
            <span class="flex min-w-0 items-center gap-2.5">
              <input
                type="checkbox"
                checked={willArchive}
                disabled={busy}
                onchange={() => toggle(g.save_id)}
                class="size-3.5 shrink-0 accent-rose-500"
              />
              <span class="flex min-w-0 flex-col">
                <span class="truncate text-zinc-200">{gameName(g)}</span>
                {#if saveLabel(g)}
                  <span class="truncate text-xs text-zinc-500">{saveLabel(g)}</span>
                {/if}
                {#if twins.length > 0}
                  <!-- Casi siempre no son dos juegos: es el mismo save
                       trackeado dos veces. Decirlo aquí evita que el usuario
                       archive uno, no libere nada y crea que esto no va. -->
                  <span
                    class="mt-0.5 flex items-center gap-1 text-[11px] text-amber-300/90"
                  >
                    <Link2 size={11} />
                    {$_("liberate.shared_with", {
                      values: {
                        names: twins.map(gameName).join(", "),
                        size: formatBytes(shared),
                      },
                    })}
                  </span>
                {/if}
              </span>
            </span>
            <span class="shrink-0 font-mono text-xs text-zinc-400">
              {formatBytes(g.freeable_bytes)}
            </span>
          </label>
        </li>
      {/each}
    </ul>

    <!-- Medidor: dónde deja la selección a la cuenta. -->
    <div class="mt-3 rounded-lg border border-white/[0.08] bg-zinc-950/40 p-2.5">
      <div class="flex items-baseline justify-between text-xs">
        <span class="text-zinc-400">{$_("liberate.after")}</span>
        <span class="font-mono {fits ? 'text-emerald-300' : 'text-rose-300'}">
          {formatBytes(remaining)} / {formatBytes(limitBytes)}
        </span>
      </div>
      <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
        <div
          class="h-full rounded-full {fits ? 'bg-emerald-500' : 'bg-rose-500'}"
          style="width: {limitBytes > 0
            ? Math.min(100, (remaining / limitBytes) * 100)
            : 0}%"
        ></div>
      </div>
      <p class="mt-1.5 text-[11px] {fits ? 'text-emerald-300/90' : 'text-rose-300/90'}">
        {fits
          ? $_("liberate.fits")
          : $_("liberate.still_over", {
              values: { size: formatBytes(Math.max(0, remaining - limitBytes)) },
            })}
      </p>
    </div>
  {/if}
</div>
