<script lang="ts">
  /**
   * The HUD's debug panel. **TEMPORARY**, see `debug.svelte.ts`.
   *
   * Deliberately untranslated and unpolished: it is scaffolding for picking the
   * final opacity and size values while looking at the HUD over a real game, not a
   * surface any user will ever see. It gets deleted wholesale along with the store
   * once the numbers are settled.
   */
  import { overlayDebug, saveOverlayDebug, resetOverlayDebug } from "./debug.svelte";

  const knobs = [
    { key: "bgOpacity" as const, label: "Fondo", min: 0, max: 1, step: 0.02, fmt: (v: number) => v.toFixed(2) },
    { key: "textOpacity" as const, label: "Contenido", min: 0.1, max: 1, step: 0.02, fmt: (v: number) => v.toFixed(2) },
    { key: "fontSize" as const, label: "Letra", min: 10, max: 24, step: 1, fmt: (v: number) => `${v}px` },
  ];
</script>

<!-- Fijo abajo a la derecha para no tapar el registro. -->
<div class="pointer-events-auto fixed bottom-3 right-3 z-50 select-none">
  {#if !overlayDebug.panelOpen}
    <button
      type="button"
      onclick={() => {
        overlayDebug.panelOpen = true;
        saveOverlayDebug();
      }}
      class="rounded-md border border-amber-500/50 bg-amber-500/15 px-2 py-1 text-[11px] font-medium text-amber-200"
      >debug</button
    >
  {:else}
    <div
      class="w-64 space-y-2 rounded-lg border border-amber-500/50 bg-zinc-950/95 p-3 text-[11px] text-zinc-300"
    >
      <div class="flex items-center justify-between">
        <span class="font-semibold uppercase tracking-wide text-amber-300">debug · temporal</span>
        <button
          type="button"
          onclick={() => {
            overlayDebug.panelOpen = false;
            saveOverlayDebug();
          }}
          class="rounded px-1 text-zinc-400 hover:text-zinc-100">–</button
        >
      </div>

      {#each knobs as k (k.key)}
        <label class="flex items-center gap-2">
          <span class="w-16 shrink-0 text-zinc-400">{k.label}</span>
          <input
            type="range"
            min={k.min}
            max={k.max}
            step={k.step}
            value={overlayDebug[k.key]}
            oninput={(e) => {
              overlayDebug[k.key] = +e.currentTarget.value;
              saveOverlayDebug();
            }}
            class="flex-1 accent-amber-400"
          />
          <span class="w-10 shrink-0 text-right tabular-nums text-zinc-500"
            >{k.fmt(overlayDebug[k.key])}</span
          >
        </label>
      {/each}

      <div class="flex gap-2 pt-1">
        <button
          type="button"
          onclick={resetOverlayDebug}
          class="flex-1 rounded border border-zinc-700 py-1 text-zinc-300 hover:bg-zinc-800"
          >Reiniciar</button
        >
        <!-- Copia los valores actuales: es lo que hay que pegarme para fijarlos. -->
        <button
          type="button"
          onclick={() =>
            navigator.clipboard?.writeText(
              `bg ${overlayDebug.bgOpacity} · texto ${overlayDebug.textOpacity} · letra ${overlayDebug.fontSize}px`,
            )}
          class="flex-1 rounded border border-zinc-700 py-1 text-zinc-300 hover:bg-zinc-800"
          >Copiar</button
        >
      </div>
    </div>
  {/if}
</div>
