<script lang="ts">
  /**
   * "Vincular a esta máquina" for a cloud orphan, a save that lives in the
   * cloud from ANOTHER machine, with no local folder here.
   *
   * The folder picker used to be the only way through, even when detection
   * already knew where the game saves on this machine. So the offer is now, in
   * order:
   *
   * 1. Folders detected for THIS slug, one click, no thinking.
   * 2. Any other game detected here, pickable by NAME. The slug match is exact,
   *    and two machines routinely slug one game differently (a Steam copy on
   *    one side, a loose install on the other): before this, that difference
   *    dumped the user into the file manager to hand-find a folder Hoard had
   *    already found. Best name match comes first and is badged.
   * 3. The folder picker, still there for what detection genuinely missed.
   *
   * Cold cache (never scanned, the case for anyone who never turned on Modo
   * Automático) is deliberately NOT rendered as "nothing found": we don't
   * know, so we offer the scan.
   *
   * Adopting is the parent's job (`onPick`): Library.svelte owns the toast and
   * the tracked-list refresh, and this modal shouldn't fork that.
   */
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Radar, Link, Search, Gamepad2 } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import {
    detectedPathsForGame,
    scanLibrary,
    type Confidence,
    type DetectionReport,
    type LinkCandidate,
    type LocalDetection,
    type TrackedSave,
  } from "../api";
  import { toastError } from "../stores/toasts";

  type Props = {
    open: boolean;
    orphan: TrackedSave | null;
    /** Folders this machine already tracks, dropped from the candidate list so
     *  two saves can't end up backing up one folder. */
    trackedPaths?: string[];
    onClose: () => void;
    onPick: (path: string) => void;
    /** Lets Library.svelte adopt the report from an in-modal scan instead of
     *  leaving its own cards stale. */
    onScanned?: (report: DetectionReport) => void;
  };
  let {
    open,
    orphan,
    trackedPaths = [],
    onClose,
    onPick,
    onScanned,
  }: Props = $props();

  let detection = $state<LocalDetection | null>(null);
  let loading = $state(false);
  let scanning = $state(false);
  let search = $state("");

  // `null` scanned_at = no cache at all. Distinct from "scanned and found
  // nothing", which is a real answer and only earns the picker.
  const neverScanned = $derived(detection !== null && detection.scanned_at === null);
  const paths = $derived(detection?.paths ?? []);
  const candidates = $derived(detection?.candidates ?? []);

  // Already sorted by the agent (affinity, then name); search only filters.
  const shown = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return candidates;
    return candidates.filter(
      (c) =>
        c.display_name.toLowerCase().includes(q) ||
        c.game_slug.toLowerCase().includes(q),
    );
  });

  // Re-query whenever a different orphan opens the modal. Cheap: the command
  // is an in-memory cache lookup, no scan.
  $effect(() => {
    if (open && orphan) {
      void load(orphan.game_slug);
    } else if (!open) {
      detection = null;
      search = "";
    }
  });

  async function load(slug: string) {
    loading = true;
    try {
      detection = await detectedPathsForGame(slug, trackedPaths);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
      detection = null;
    } finally {
      loading = false;
    }
  }

  /** Cold cache: run the normal sweep, then re-read what it found for us. */
  async function scan() {
    if (!orphan || scanning) return;
    scanning = true;
    try {
      const report = await scanLibrary();
      onScanned?.(report);
      detection = await detectedPathsForGame(orphan.game_slug, trackedPaths);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanning = false;
    }
  }

  /** Picking a game links its strongest save folder, the same one automatic
   *  tracking would have chosen. */
  function pickGame(c: LinkCandidate) {
    if (c.paths.length > 0) onPick(c.paths[0].path);
  }

  async function pickOther() {
    if (!orphan) return;
    try {
      const result = await openDialog({
        directory: true,
        multiple: false,
        // Land on the strongest detected folder when we have one, so "otra
        // carpeta" starts next door instead of at Documents.
        defaultPath: paths[0]?.path || undefined,
        title: $_("library.pick_folder_title", {
          values: { name: orphan.game_slug },
        }),
      });
      if (typeof result === "string" && result.length > 0) onPick(result);
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  function confidenceLabel(c: Confidence): string {
    return c === "high"
      ? $_("library.high")
      : c === "medium"
        ? $_("library.medium")
        : $_("library.low");
  }

  function confidenceClass(c: Confidence): string {
    return c === "high"
      ? "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30"
      : c === "medium"
        ? "bg-sky-500/10 text-sky-300 ring-sky-500/30"
        : "bg-zinc-500/10 text-zinc-400 ring-zinc-500/30";
  }
</script>

<Modal
  {open}
  title={$_("library.link_to_machine")}
  description={orphan
    ? $_("library.link_modal_desc", { values: { name: orphan.game_slug } })
    : undefined}
  {onClose}
>
  <div class="space-y-4">
    {#if loading}
      <p class="text-sm text-zinc-500">{$_("library.link_loading")}</p>
    {:else}
      {#if paths.length > 0}
        <div>
          <span class="mb-1.5 block text-xs font-medium text-zinc-400">
            {$_("library.link_detected_heading")}
          </span>
          <ul class="space-y-1.5">
            {#each paths as p (p.path)}
              <li>
                <button
                  type="button"
                  onclick={() => onPick(p.path)}
                  class="group flex w-full items-center gap-2.5 rounded-lg border border-white/[0.08] bg-zinc-950/60 px-3 py-2.5 text-left transition-colors hover:border-emerald-600/40 hover:bg-emerald-600/10"
                >
                  <Link
                    size={14}
                    class="shrink-0 text-zinc-500 group-hover:text-emerald-300"
                  />
                  <span
                    class="min-w-0 flex-1 truncate font-mono text-xs text-zinc-300 group-hover:text-zinc-100"
                    title={p.path}
                  >
                    {p.path}
                  </span>
                  <span
                    class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium ring-1 ring-inset {confidenceClass(
                      p.confidence,
                    )}"
                  >
                    {confidenceLabel(p.confidence)}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
          <p class="mt-1.5 text-xs text-zinc-500">
            {$_("library.link_detected_hint")}
          </p>
        </div>
      {:else if neverScanned}
        <div class="rounded-lg border border-white/[0.08] bg-zinc-950/60 p-3">
          <p class="text-sm text-zinc-300">{$_("library.link_never_scanned")}</p>
          <div class="mt-2.5">
            <Button variant="secondary" onclick={scan} loading={scanning}>
              <Radar size={14} />
              {scanning
                ? $_("library.link_scanning")
                : $_("library.link_scan_button")}
            </Button>
          </div>
        </div>
      {:else}
        <p class="text-sm text-zinc-500">{$_("library.link_no_detection")}</p>
      {/if}

      <!-- Pick the GAME, not the folder. Shown whenever this machine has
           anything else detected — including alongside an exact-slug hit,
           because the exact hit can still be the wrong game. -->
      {#if candidates.length > 0}
        <div>
          <span class="mb-1.5 block text-xs font-medium text-zinc-400">
            {paths.length > 0
              ? $_("library.link_pick_other_game")
              : $_("library.link_pick_game_heading")}
          </span>

          <!-- The search box only earns its space once scanning the list by eye
               stops being realistic. -->
          {#if candidates.length > 6}
            <div class="relative mb-1.5">
              <Search
                size={13}
                class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500"
              />
              <input
                type="text"
                bind:value={search}
                placeholder={$_("library.link_search_games")}
                class="w-full rounded-lg border border-white/[0.08] bg-zinc-950/60 py-1.5 pl-7 pr-2.5 text-xs text-zinc-200 placeholder:text-zinc-600 focus:border-emerald-600/40 focus:outline-none"
              />
            </div>
          {/if}

          {#if shown.length === 0}
            <p class="text-sm text-zinc-500">
              {$_("library.link_no_game_matches")}
            </p>
          {:else}
            <ul class="max-h-56 space-y-1.5 overflow-y-auto pr-1">
              {#each shown as c (c.game_slug)}
                <li>
                  <button
                    type="button"
                    onclick={() => pickGame(c)}
                    class="group flex w-full items-center gap-2.5 rounded-lg border border-white/[0.08] bg-zinc-950/60 px-3 py-2 text-left transition-colors hover:border-emerald-600/40 hover:bg-emerald-600/10"
                  >
                    <Gamepad2
                      size={14}
                      class="shrink-0 text-zinc-500 group-hover:text-emerald-300"
                    />
                    <span class="min-w-0 flex-1">
                      <span
                        class="block truncate text-xs text-zinc-200 group-hover:text-zinc-100"
                      >
                        {c.display_name}
                      </span>
                      <span
                        class="block truncate font-mono text-[10px] text-zinc-500"
                        title={c.paths[0]?.path}
                      >
                        {c.paths[0]?.path}
                      </span>
                    </span>
                    {#if c.affinity >= 2}
                      <span
                        class="shrink-0 rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-300 ring-1 ring-inset ring-emerald-500/30"
                      >
                        {$_("library.link_same_name")}
                      </span>
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          <p class="mt-1.5 text-xs text-zinc-500">
            {$_("library.link_pick_game_hint")}
          </p>
        </div>
      {/if}
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>
      {$_("common.cancel")}
    </Button>
    <!-- With something detected on offer THAT is the primary action (emerald on
         hover) and the picker is the escape hatch; with nothing, the picker is
         the only way forward and takes the emerald. -->
    <Button
      variant={paths.length > 0 || candidates.length > 0
        ? "secondary"
        : "primary"}
      onclick={pickOther}
    >
      <FolderOpen size={14} />
      {$_("library.link_other_folder")}
    </Button>
  {/snippet}
</Modal>
