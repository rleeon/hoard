<script lang="ts">
  /**
   * "Scan folder" dialog, the single answer to "point Hoard at a folder".
   *
   * Two modes, one flow:
   *
   * * **add** (no `target`): the Library's own scan-folder button. Every
   *   save-like dir found is listed as "found <Game> here, track it?" and
   *   tracks through the normal `addGameToTracking`.
   * * **target** (`target` set): the same scan, but for ONE known game, "track
   *   this game with another folder" and "no save folder yet". Picking a row
   *   hands the path back through `onPick` instead of creating a game, and the
   *   header keeps the explanation of why Hoard has no path plus the Steam
   *   install dir, which is exactly the folder worth scanning first (so we
   *   scan it on open, no clicks).
   *
   * The backend walk (`scan_folder`) does not apply the periodic scan's
   * precision gate, the user pointing at a folder IS the evidence, so what
   * comes back is everything under there that holds data, including the folder
   * itself when that's the save folder.
   */
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, FolderSearch, Check, Plus, Search } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import Cover from "./Cover.svelte";
  import {
    scanFolder,
    addGameToTracking,
    type Confidence,
    type DetectedGame,
  } from "../api";
  import { toastError, toastSuccess } from "../stores/toasts";

  /** The game a target-mode scan is looking for a folder for. */
  export type ScanTarget = {
    slug: string;
    display_name: string;
    /** Steam's install dir, when known, scanned on open. */
    install_dir?: string | null;
    /** Folder already detected/tracked for this game; seeds the picker. */
    seed_path?: string | null;
    steam_app_id?: number | null;
    /** No save folder detected at all, the case the explanation is about. */
    no_paths?: boolean;
  };

  type Props = {
    open: boolean;
    onClose: () => void;
    onAdded: (save: import("../api").TrackedSave) => void;
    target?: ScanTarget | null;
    onPick?: (path: string) => void;
  };
  let { open, onClose, onAdded, target = null, onPick }: Props = $props();

  let folder = $state("");
  let scanning = $state(false);
  let scanned = $state(false);
  let results = $state<DetectedGame[]>([]);
  // Slugs currently being tracked / already tracked this session, so a row's
  // button shows progress and can't be double-submitted.
  let trackingSlug = $state<string | null>(null);
  let addedSlugs = $state<string[]>([]);

  // Cuando no hay ninguna carpeta detectada, abrir ya escaneando la de
  // instalación: enseñar esa ruta servía justo para que el usuario fuera a
  // mirar si el save está ahí, y hacerle dar dos clics más es la pereza que
  // sobra. Con carpeta ya detectada no se escanea nada solo, se busca OTRA,
  // así que la conocida únicamente siembra el selector.
  let autoScanned = $state(false);
  $effect(() => {
    if (!open) {
      autoScanned = false;
      return;
    }
    if (autoScanned || !target) return;
    autoScanned = true;
    if (target.no_paths && target.install_dir) {
      folder = target.install_dir;
      void runScan();
    }
  });

  async function pickFolder() {
    try {
      const result = await openDialog({
        directory: true,
        multiple: false,
        // Abre el selector DONDE ya sabemos que guarda el juego, para que se
        // navegue desde ahí en vez de desde Documentos.
        defaultPath: folder || target?.seed_path || undefined,
        title: $_("scan_folder.pick"),
      });
      if (typeof result === "string" && result.length > 0) {
        folder = result;
        await runScan();
      }
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function scanPath(path: string) {
    folder = path;
    await runScan();
  }

  async function runScan() {
    if (folder.trim().length === 0 || scanning) return;
    scanning = true;
    scanned = false;
    results = [];
    addedSlugs = [];
    try {
      results = await scanFolder(folder.trim());
      scanned = true;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanning = false;
    }
  }

  /** Row action. In target mode the path goes back to the caller; in add mode
   *  it becomes a new tracked game. */
  async function choose(game: DetectedGame) {
    const path = game.found_paths[0];
    if (!path) return;
    // Sólo el juego que se buscaba se devuelve como "su carpeta". Un resultado
    // de OTRO juego es la carpeta de ese otro: bindearla al objetivo escribía un
    // override manual permanente (`device.json`, sobrevive a todo) y dejaba al
    // dueño real sin poder rastrear lo suyo. Ago-2026: Horizon Forbidden West
    // acabó apuntando a la carpeta de Surviving Mars. Se añade como lo que es.
    if (target && game.slug === target.slug) {
      usePath(path);
      return;
    }
    if (trackingSlug !== null) return;
    trackingSlug = game.slug;
    try {
      const saved = await addGameToTracking({
        game_slug: game.slug,
        local_path: path,
        display_name: game.display_name,
        steam_app_id: game.steam_app_id,
      });
      onAdded(saved);
      addedSlugs = [...addedSlugs, game.slug];
      toastSuccess(
        $_("manual.added_game", { values: { name: game.display_name } }),
      );
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      trackingSlug = null;
    }
  }

  /** Hand a path back to the caller (target mode) and close. */
  function usePath(path: string) {
    onPick?.(path);
    close();
  }

  /** En modo dirigido, TODOS los resultados que no son el juego buscado. La
   *  salida de emergencia sigue disponible,el usuario puede saber algo que la
   *  detección no, pero con el aviso de a quién pertenece esa carpeta. Son
   *  todos y no el primero: una carpeta madre puede tener varios juegos dentro,
   *  y nombrar solo a uno haría creer que los demás no están en juego. */
  const foreignOwners = $derived(
    target ? results.filter((g) => g.slug !== target.slug) : [],
  );

  /** True when the scanned folder is itself among the results **for the game we
   *  were sent here for**, only then does its row already hand the folder over
   *  (`usePath`), and only then is the "use the folder as-is" escape hatch a
   *  duplicate.
   *
   *  A row for a DIFFERENT game is not a duplicate, and treating it as one is
   *  how a second folder ended up tracked as its own game: point the picker at
   *  `Desktop\saves` for Factorio, attribution names that exact folder after
   *  something else, the escape hatch disappears, and the only thing left to
   *  click tracks it under the foreign slug. The row and the escape hatch do
   *  opposite things, track it as its own game, or give it to the target,
   *  so one can never stand in for the other. */
  const folderInResults = $derived(
    results.some(
      (g) =>
        g.found_paths[0] === folder.trim() && (!target || g.slug === target.slug),
    ),
  );

  function confidenceClass(c: Confidence): string {
    if (c === "high") return "bg-emerald-500/10 text-emerald-300 ring-emerald-500/30";
    if (c === "medium") return "bg-amber-500/10 text-amber-300 ring-amber-500/30";
    return "bg-zinc-500/10 text-zinc-300 ring-zinc-500/30";
  }

  function confidenceLabel(c: Confidence): string {
    return c === "high"
      ? $_("library.high")
      : c === "medium"
        ? $_("library.medium")
        : $_("library.low");
  }

  function reset() {
    folder = "";
    scanning = false;
    scanned = false;
    results = [];
    trackingSlug = null;
    addedSlugs = [];
  }

  function close() {
    reset();
    onClose();
  }
</script>

<Modal
  {open}
  title={target
    ? $_("scan_folder.target_title", { values: { name: target.display_name } })
    : $_("scan_folder.title")}
  description={target ? undefined : $_("scan_folder.description")}
  onClose={close}
>
  <div class="space-y-4">
    {#if target}
      <!-- Por qué no hay carpeta + dónde está instalado. Es el texto que antes
           vivía en su propio aviso: sigue aquí porque la pista de instalación
           es lo que permite ir a mirar uno mismo. -->
      <div class="space-y-2">
        {#if target.no_paths}
          <p class="text-sm text-zinc-300">
            {$_("library.no_save_alert_body", {
              values: { name: target.display_name },
            })}
          </p>
        {/if}
        {#if target.install_dir}
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="min-w-0 flex-1 break-all rounded-md bg-zinc-950/60 px-3 py-2 font-mono text-[11px] text-zinc-400 ring-1 ring-inset ring-zinc-800"
            >
              {$_("library.no_save_alert_install_hint", {
                values: { path: target.install_dir },
              })}
            </span>
            <Button
              variant="secondary"
              onclick={() => scanPath(target.install_dir ?? "")}
              disabled={scanning}
            >
              <Search size={14} />
              {$_("scan_folder.scan_install_dir")}
            </Button>
          </div>
        {/if}
      </div>
    {/if}

    <div class="flex flex-wrap gap-2">
      <Button variant="secondary" onclick={pickFolder} loading={scanning}>
        <FolderOpen size={14} />
        {folder ? $_("scan_folder.change") : $_("scan_folder.pick")}
      </Button>
      {#if folder}
        <span
          class="flex min-w-0 flex-1 items-center truncate rounded-md bg-zinc-950/60 px-3 font-mono text-xs text-zinc-400 ring-1 ring-inset ring-zinc-800"
          title={folder}
        >
          {folder}
        </span>
      {/if}
    </div>

    {#if scanning}
      <p class="py-6 text-center text-sm text-zinc-400">
        {$_("scan_folder.scanning")}
      </p>
    {:else if scanned && results.length === 0}
      <div class="py-8 text-center">
        <FolderSearch size={28} class="mx-auto mb-2 text-zinc-600" />
        <p class="text-sm text-zinc-400">{$_("scan_folder.empty")}</p>
      </div>
    {:else if results.length > 0}
      <div>
        <p class="mb-2 text-xs uppercase tracking-wide text-zinc-500">
          {$_("scan_folder.found", { values: { count: results.length } })}
        </p>
        <ul
          class="max-h-72 divide-y divide-zinc-800 overflow-y-auto rounded-md border border-zinc-800 bg-zinc-950/40"
        >
          {#each results as game (game.slug + game.found_paths[0])}
            {@const isAdded = addedSlugs.includes(game.slug)}
            <li class="flex items-center justify-between gap-3 px-3 py-2.5">
              <Cover
                appId={game.steam_app_id ?? undefined}
                slug={game.slug}
                name={game.display_name}
                class="h-9 w-9 shrink-0 rounded-lg"
                initialClass="text-xs"
              />
              <div class="min-w-0 flex-1">
                <p class="flex items-center gap-1.5 truncate text-sm font-medium text-zinc-100">
                  <span class="truncate">{game.display_name}</span>
                  <span
                    class="shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide ring-1 ring-inset {confidenceClass(
                      game.confidence,
                    )}"
                  >
                    {confidenceLabel(game.confidence)}
                  </span>
                </p>
                <p class="truncate font-mono text-[11px] text-zinc-500">
                  {game.found_paths[0]}
                </p>
              </div>
              {#if isAdded}
                <span
                  class="flex shrink-0 items-center gap-1 text-xs text-emerald-400"
                >
                  <Check size={14} />
                  {$_("scan_folder.tracked")}
                </span>
              {:else}
                <Button
                  variant="secondary"
                  onclick={() => choose(game)}
                  loading={trackingSlug === game.slug}
                  disabled={trackingSlug !== null}
                >
                  <Plus size={13} />
                  {target && game.slug === target.slug
                    ? $_("scan_folder.use")
                    : $_("scan_folder.track")}
                </Button>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {:else}
      <p class="py-6 text-center text-sm text-zinc-500">
        {$_("scan_folder.hint")}
      </p>
    {/if}

    <!-- Salida de emergencia del modo dirigido: el usuario sabe cuál es la
         carpeta aunque el escaneo no la proponga. Sin esto, cambiar el selector
         del sistema por este diálogo quitaría capacidad en vez de darla. -->
    {#if target && folder && !folderInResults}
      <button
        type="button"
        onclick={() => usePath(folder.trim())}
        class="w-full rounded-md border border-dashed border-zinc-700 px-3 py-2.5 text-left text-xs text-zinc-400 transition-colors hover:border-emerald-500/40 hover:text-zinc-200"
      >
        {$_("scan_folder.use_as_is")}
        <span class="mt-0.5 block truncate font-mono text-[11px] text-zinc-500">
          {folder}
        </span>
        {#if foreignOwners.length > 0}
          <span class="mt-1 block text-[11px] text-amber-300">
            {$_("scan_folder.belongs_to_other", {
              values: {
                name: foreignOwners.map((g) => g.display_name).join(", "),
              },
            })}
          </span>
        {/if}
      </button>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={close}>
      {$_("common.close")}
    </Button>
  {/snippet}
</Modal>
