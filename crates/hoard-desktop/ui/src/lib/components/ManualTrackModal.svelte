<script lang="ts">
  /**
   * "Manual track" dialog (Hoard free).
   *
   * Two things Hoard can't auto-detect, tracked by hand through one modal:
   *
   *  - **Game**, a game the catalog/Steam scan misses (indie, DRM-free, odd
   *    install). The user names it and points at its save folder; play-detection
   *    falls back to the slug, or to pinned processes if they add them.
   *  - **Emulator**, no storefront/manifest entry at all, so we also collect
   *    the emulator's executable(s) to know when the user is "playing". Presets
   *    pre-fill folder + processes for the common ones.
   *
   * Both track through the normal `addGameToTracking`. Emulator saves carry an
   * `emu-<id>` slug and pin `backup_only` so Hoard never restores the cloud copy
   * over an in-progress local save; games carry a plain slug. Kept as its own
   * component so none of this touches the detection flow in Library.svelte.
   */
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { FolderOpen, Plus, X, Gamepad2, Cpu } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import Input from "./Input.svelte";
  import {
    listEmulatorPresets,
    listEmulatorTitles,
    listRunningProcesses,
    addGameToTracking,
    type EmulatorPreset,
    type EmulatorTitle,
    type RunningProcess,
    type TrackedSave,
  } from "../api";
  import { toastError, toastSuccess } from "../stores/toasts";

  type Props = {
    open: boolean;
    onClose: () => void;
    onAdded: (save: TrackedSave) => void;
  };
  let { open, onClose, onAdded }: Props = $props();

  type Mode = "game" | "emulator";
  let mode = $state<Mode>("game");

  // --- Game mode ---------------------------------------------------------
  let gameName = $state("");

  // --- Emulator mode -----------------------------------------------------
  let presets = $state<EmulatorPreset[]>([]);
  let loadingPresets = $state(false);
  // "" = nothing chosen yet, "custom" = an emulator not in the catalog.
  let selectedId = $state("");
  let customName = $state("");

  // Games found inside the emulator's folder, when its tree can be split per title.
  // Picking them is offered because the intermediate level carries a profile id
  // generated per install: copying the whole tree leaves the save hanging off a
  // profile the other machine's emulator has never seen, and it rejects it. The
  // title's folder is the part both installs do agree on.
  //
  // An empty list after searching means the tree does not have the expected shape;
  // the root is then added as it is, as always. A guess that missed would leave the
  // user with nothing, which is worse than the problem this solves.
  let titles = $state<EmulatorTitle[]>([]);
  let chosenTitles = $state<Set<string>>(new Set());
  let scanningTitles = $state(false);
  let scannedTitles = $state(false);
  // The name the user gives each title, by path. The folder is named with the
  // console's id (`0100152000022000`) and there is no way to translate it: the
  // manifest Hoard uses is Steam's and knows nothing about the Switch. So it is
  // asked, once, at the only moment the user has the list in front of them and knows
  // which is which.
  //
  // **The name does not go into the slug.** The slug is formed from the id, which is
  // identical on both machines; if it depended on what somebody types, the same game
  // would be tracked twice the moment it was spelled differently on the other PC.
  let titleNames = $state<Record<string, string>>({});

  // --- Shared ------------------------------------------------------------
  let folder = $state("");
  let procs = $state<string[]>([]);
  let manualProc = $state("");
  // Emulators default to backup-only (never restore over an in-progress save);
  // plain games sync normally unless the user opts in.
  let backupOnly = $state(true);

  let running = $state<RunningProcess[]>([]);
  let detected = $state(false);
  let detecting = $state(false);
  let submitting = $state(false);

  const isEmulator = $derived(mode === "emulator");
  const selectedPreset = $derived(
    presets.find((p) => p.id === selectedId) ?? null,
  );
  const isCustom = $derived(selectedId === "custom");
  const splitsPerTitle = $derived(selectedPreset?.splits_per_title ?? false);
  // Game mode needs a name + folder; emulator mode also needs a chosen
  // emulator and at least one process (no catalog to derive it from).
  const canSubmit = $derived(
    isEmulator
      ? selectedId.length > 0 &&
          folder.trim().length > 0 &&
          procs.length > 0 &&
          (!isCustom || customName.trim().length > 0)
      : gameName.trim().length > 0 && folder.trim().length > 0,
  );

  // Lazy-load the emulator catalog the first time it's needed.
  $effect(() => {
    if (open && isEmulator && presets.length === 0 && !loadingPresets) {
      void loadPresets();
    }
  });

  function pickMode(m: Mode) {
    if (m === mode) return;
    mode = m;
    // Switching intent shouldn't carry the other mode's fields over.
    folder = "";
    procs = [];
    running = [];
    detected = false;
    selectedId = "";
    customName = "";
    backupOnly = m === "emulator";
    clearTitles();
  }

  function clearTitles() {
    titles = [];
    chosenTitles = new Set();
    titleNames = {};
    scanningTitles = false;
    scannedTitles = false;
  }

  async function scanTitles() {
    if (!selectedId || !folder.trim() || scanningTitles) return;
    scanningTitles = true;
    try {
      titles = await listEmulatorTitles(selectedId, folder.trim());
      // All ticked: somebody looking for their games wants them, and unticking is
      // faster than ticking ten.
      chosenTitles = new Set(titles.map((t) => t.path));
      scannedTitles = true;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      scanningTitles = false;
    }
  }

  function toggleTitle(path: string) {
    const next = new Set(chosenTitles);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    chosenTitles = next;
  }

  async function loadPresets() {
    loadingPresets = true;
    try {
      presets = await listEmulatorPresets();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      loadingPresets = false;
    }
  }

  /** Pre-fill folder + processes from the chosen catalog entry. */
  function onSelect() {
    if (selectedPreset) {
      procs = [...selectedPreset.processes];
      folder = selectedPreset.save_paths[0] ?? "";
    } else {
      // "Otro…", start blank, let the user fill everything in.
      procs = [];
      folder = "";
    }
    running = [];
    detected = false;
    clearTitles();
  }

  async function pickFolder() {
    try {
      const result = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: folder || undefined,
        title: $_("emulators.folder_label"),
      });
      if (typeof result === "string" && result.length > 0) folder = result;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }

  async function detect() {
    detecting = true;
    try {
      running = await listRunningProcesses();
      detected = true;
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      detecting = false;
    }
  }

  function addProc(name: string) {
    const n = name.trim();
    if (n.length === 0) return;
    if (procs.some((p) => p.toLowerCase() === n.toLowerCase())) return;
    procs = [...procs, n];
  }

  function addManualProc() {
    addProc(manualProc);
    manualProc = "";
  }

  function removeProc(name: string) {
    procs = procs.filter((p) => p !== name);
  }

  function slugify(s: string): string {
    return s
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function reset() {
    mode = "game";
    gameName = "";
    selectedId = "";
    customName = "";
    folder = "";
    procs = [];
    manualProc = "";
    backupOnly = true;
    running = [];
    detected = false;
    clearTitles();
  }

  function close() {
    reset();
    onClose();
  }

  async function submit() {
    if (!canSubmit || submitting) return;
    submitting = true;
    try {
      let slug: string;
      let display: string;
      if (isEmulator) {
        slug = isCustom ? `emu-${slugify(customName)}` : `emu-${selectedId}`;
        display = isCustom
          ? customName.trim()
          : (selectedPreset?.display_name ?? selectedId);
      } else {
        display = gameName.trim();
        slug = slugify(display);
      }

      // The chosen titles: one entry per game rather than one for the whole tree.
      // They all share the emulator's executable, so they are marked as a shared
      // process: otherwise starting the emulator would count as playing all ten at
      // once, inventing hours on nine saves and vetoing their sync with nobody
      // having touched them.
      const picked = titles.filter((t) => chosenTitles.has(t.path));
      if (isEmulator && picked.length > 0) {
        let last = null;
        for (const t of picked) {
          const named = (titleNames[t.path] ?? "").trim();
          last = await addGameToTracking({
            // The id, not the name: it is what both machines call the same.
            game_slug: `${slug}-${t.title_id.toLowerCase()}`,
            local_path: t.path,
            display_name: named ? named : `${display} — ${t.title_id}`,
            preset: backupOnly ? "backup_only" : undefined,
            processes: procs,
            shared_processes: true,
          });
          onAdded(last);
        }
        toastSuccess(
          $_("emulators.titles_selected", { values: { count: picked.length } }),
        );
        reset();
        onClose();
        return;
      }

      const saved = await addGameToTracking({
        game_slug: slug,
        local_path: folder.trim(),
        display_name: display,
        preset: backupOnly ? "backup_only" : undefined,
        // Empty process list ⇒ derive play-detection from the slug (games);
        // emulators always carry at least one (enforced by canSubmit).
        processes: procs.length > 0 ? procs : undefined,
      });
      onAdded(saved);
      toastSuccess(
        isEmulator
          ? $_("emulators.added", { values: { name: display } })
          : $_("manual.added_game", { values: { name: display } }),
      );
      reset();
      onClose();
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    } finally {
      submitting = false;
    }
  }
</script>

<Modal
  {open}
  title={$_("manual.title")}
  description={isEmulator
    ? $_("emulators.description")
    : $_("manual.description_game")}
  onClose={close}
>
  <div class="space-y-4">
    <!-- Mode toggle: game vs emulator -->
    <div
      class="grid grid-cols-2 gap-1 rounded-lg border border-zinc-800 bg-zinc-950/60 p-1"
    >
      <button
        type="button"
        onclick={() => pickMode("game")}
        class="rounded-md px-3 py-1.5 text-sm font-medium transition {mode ===
        'game'
          ? 'bg-emerald-600/20 text-emerald-300 ring-1 ring-inset ring-emerald-600/40'
          : 'text-zinc-400 hover:text-zinc-200'}"
      >
        {$_("manual.mode_game")}
      </button>
      <button
        type="button"
        onclick={() => pickMode("emulator")}
        class="rounded-md px-3 py-1.5 text-sm font-medium transition {mode ===
        'emulator'
          ? 'bg-emerald-600/20 text-emerald-300 ring-1 ring-inset ring-emerald-600/40'
          : 'text-zinc-400 hover:text-zinc-200'}"
      >
        {$_("manual.mode_emulator")}
      </button>
    </div>

    {#if isEmulator}
      <!-- Emulator picker -->
      <div>
        <label
          for="emu-select"
          class="mb-1.5 block text-xs font-medium text-zinc-400"
        >
          {$_("emulators.choose")}
        </label>
        <select
          id="emu-select"
          bind:value={selectedId}
          onchange={onSelect}
          disabled={loadingPresets}
          class="w-full rounded-md border border-zinc-800 bg-zinc-950/60 px-3 py-2 text-sm text-zinc-100 focus:border-emerald-600 focus:outline-none focus:ring-1 focus:ring-emerald-600 disabled:opacity-50"
        >
          <option value="" disabled>{$_("emulators.choose_placeholder")}</option
          >
          {#each presets as p (p.id)}
            <option value={p.id}>{p.display_name} — {p.system}</option>
          {/each}
          <option value="custom">{$_("emulators.custom")}</option>
        </select>
      </div>

      {#if isCustom}
        <div>
          <label
            for="emu-custom-name"
            class="mb-1.5 block text-xs font-medium text-zinc-400"
          >
            {$_("emulators.custom_name_label")}
          </label>
          <Input
            id="emu-custom-name"
            bind:value={customName}
            placeholder={$_("emulators.custom_name_placeholder")}
          />
        </div>
      {/if}
    {:else}
      <!-- Game name -->
      <div>
        <label
          for="manual-game-name"
          class="mb-1.5 block text-xs font-medium text-zinc-400"
        >
          {$_("manual.game_name_label")}
        </label>
        <Input
          id="manual-game-name"
          bind:value={gameName}
          placeholder={$_("manual.game_name_placeholder")}
        />
      </div>
    {/if}

    {#if isEmulator ? selectedId.length > 0 : true}
      <!-- Save folder -->
      <div>
        <label
          for="manual-folder"
          class="mb-1.5 block text-xs font-medium text-zinc-400"
        >
          {$_("emulators.folder_label")}
        </label>
        <div class="flex flex-wrap gap-2">
          <Input
            id="manual-folder"
            class="flex-1"
            bind:value={folder}
            placeholder={$_("emulators.folder_placeholder")}
          />
          <Button variant="secondary" onclick={pickFolder}>
            <FolderOpen size={14} />
            {$_("emulators.browse")}
          </Button>
        </div>
        <p class="mt-1.5 text-xs text-zinc-500">
          {isEmulator
            ? $_("emulators.folder_hint")
            : $_("manual.game_folder_hint")}
        </p>
      </div>

      <!-- Juegos dentro del árbol de la consola. Sólo para los emuladores
           cuyo nivel intermedio lleva un identificador por instalación: ahí
           copiar la raíz entera rompe la partida en la otra máquina. -->
      {#if isEmulator && splitsPerTitle && folder.trim().length > 0}
        <div>
          <span class="mb-1.5 block text-xs font-medium text-zinc-400">
            {$_("emulators.titles_label")}
          </span>
          <p class="mb-2 text-xs text-zinc-500">
            {$_("emulators.titles_hint")}
          </p>
          {#if !scannedTitles}
            <Button
              variant="secondary"
              onclick={scanTitles}
              loading={scanningTitles}
            >
              {$_("emulators.titles_scan")}
            </Button>
          {:else if titles.length === 0}
            <p class="text-xs text-amber-200/80">
              {$_("emulators.titles_none")}
            </p>
          {:else}
            <div
              class="max-h-40 space-y-1 overflow-y-auto rounded-md border border-white/[0.08] bg-zinc-900/60 p-2"
            >
              {#each titles as t (t.path)}
                <div
                  class="flex items-center gap-2 rounded px-1.5 py-1 text-xs hover:bg-white/[0.04]"
                >
                  <input
                    type="checkbox"
                    class="h-3.5 w-3.5 shrink-0 rounded border-zinc-700 bg-zinc-900 text-emerald-500"
                    checked={chosenTitles.has(t.path)}
                    onchange={() => toggleTitle(t.path)}
                    aria-label={t.title_id}
                  />
                  <input
                    type="text"
                    class="min-w-0 flex-1 rounded border border-white/[0.08] bg-zinc-900 px-2 py-1 text-xs text-zinc-200 placeholder:text-zinc-600 focus:border-emerald-500/40 focus:outline-none"
                    placeholder={$_("emulators.titles_name_placeholder")}
                    disabled={!chosenTitles.has(t.path)}
                    bind:value={titleNames[t.path]}
                  />
                  <span class="shrink-0 font-mono text-[10px] text-zinc-600">
                    {t.title_id}
                  </span>
                </div>
              {/each}
            </div>
            <p class="mt-1.5 text-xs text-zinc-500">
              {$_("emulators.titles_selected", {
                values: { count: chosenTitles.size },
              })}
              · {$_("emulators.titles_name_hint")}
            </p>
          {/if}
        </div>
      {/if}

      <!-- Process(es) -->
      <div>
        <span class="mb-1.5 block text-xs font-medium text-zinc-400">
          {isEmulator
            ? $_("emulators.processes_label")
            : $_("manual.processes_label_game")}
        </span>

        {#if procs.length > 0}
          <div class="mb-2 flex flex-wrap gap-1.5">
            {#each procs as p (p)}
              <span
                class="inline-flex items-center gap-1 rounded-md bg-emerald-600/15 px-2 py-1 font-mono text-xs text-emerald-300 ring-1 ring-inset ring-emerald-600/30"
              >
                {p}
                <button
                  type="button"
                  onclick={() => removeProc(p)}
                  aria-label={$_("emulators.remove_proc")}
                  class="rounded-sm text-emerald-400/70 hover:text-emerald-200"
                >
                  <X size={12} />
                </button>
              </span>
            {/each}
          </div>
        {/if}

        <div class="flex flex-wrap gap-2">
          <Input
            class="flex-1"
            bind:value={manualProc}
            placeholder={$_("emulators.process_placeholder")}
            onkeydown={(e: KeyboardEvent) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addManualProc();
              }
            }}
          />
          <Button
            variant="secondary"
            onclick={addManualProc}
            disabled={manualProc.trim().length === 0}
          >
            <Plus size={14} />
            {$_("emulators.add_proc")}
          </Button>
        </div>

        <div class="mt-2">
          <Button variant="ghost" onclick={detect} loading={detecting}>
            <Cpu size={14} />
            {detecting ? $_("emulators.detecting") : $_("emulators.detect")}
          </Button>
        </div>

        {#if detected && running.length === 0}
          <p class="mt-2 text-xs text-zinc-500">
            {$_("emulators.no_processes_found")}
          </p>
        {/if}
        {#if running.length > 0}
          <ul
            class="mt-2 max-h-40 divide-y divide-zinc-800 overflow-y-auto rounded-md border border-zinc-800 bg-zinc-950/40"
          >
            {#each running as proc (proc.name)}
              <li class="flex items-center justify-between gap-2 px-3 py-1.5">
                <span class="flex min-w-0 items-center gap-2">
                  <Gamepad2 size={13} class="shrink-0 text-zinc-500" />
                  <span class="truncate font-mono text-xs text-zinc-300"
                    >{proc.name}</span
                  >
                  <span class="shrink-0 tabular-nums text-[10px] text-zinc-600"
                    >{proc.cpu.toFixed(0)}%</span
                  >
                </span>
                <button
                  type="button"
                  onclick={() => addProc(proc.name)}
                  disabled={procs.some(
                    (p) => p.toLowerCase() === proc.name.toLowerCase(),
                  )}
                  class="shrink-0 rounded-md px-2 py-0.5 text-xs text-emerald-400 hover:bg-emerald-600/10 disabled:text-zinc-600 disabled:hover:bg-transparent"
                >
                  <Plus size={13} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <p class="mt-1.5 text-xs text-zinc-500">
          {isEmulator
            ? $_("emulators.processes_hint")
            : $_("manual.processes_hint_game")}
        </p>
      </div>

      <!-- Backup-only toggle -->
      <label class="flex items-start gap-2.5 text-sm text-zinc-300">
        <input
          type="checkbox"
          bind:checked={backupOnly}
          class="mt-0.5 h-4 w-4 rounded border-zinc-700 bg-zinc-950 text-emerald-600 focus:ring-emerald-600"
        />
        <span>
          {$_("emulators.backup_only_label")}
          <span class="mt-0.5 block text-xs text-zinc-500">
            {$_("emulators.backup_only_hint")}
          </span>
        </span>
      </label>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={close}>
      {$_("common.cancel")}
    </Button>
    <Button onclick={submit} loading={submitting} disabled={!canSubmit}>
      <Plus size={14} />
      {submitting ? $_("emulators.submitting") : $_("manual.submit")}
    </Button>
  {/snippet}
</Modal>
