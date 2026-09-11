<script lang="ts">
  /**
   * Dev-only console for the states that are otherwise a pain to reach: a quota
   * about to burst, an account at its device limit, a plan that changed. It
   * paints over the account snapshot the UI reads (`stores/debug.ts`), it does
   * not talk to the server, so nothing here changes the account or the saves.
   *
   * Ctrl+Shift+D toggles it. `import.meta.env.DEV` keeps it out of the build the
   * users get.
   */
  import { X, Bug } from "@lucide/svelte";

  import { cloud } from "../stores/cloud";
  import {
    applySurfaces,
    SURFACE_DEFAULTS,
    surfaceTune,
    type SurfaceTune,
    clearOverrides,
    cloudOverrides,
    debugDeviceLimit,
    debugPanelOpen,
    setOverrides,
  } from "../stores/debug";

  const GB = 1024 ** 3;

  const tune = $derived($surfaceTune ?? SURFACE_DEFAULTS);
  function setTune(key: keyof SurfaceTune, value: number) {
    applySurfaces({ ...tune, [key]: value });
  }
  const SURFACE_SLIDERS: { key: keyof SurfaceTune; label: string; min: number; max: number; step: number }[] = [
    { key: "floor", label: "floor (0 = pure black)", min: 0, max: 0.2, step: 0.005 },
    { key: "sidebar", label: "left rail (0 = pure black)", min: 0, max: 0.3, step: 0.005 },
    { key: "sidebarAlpha", label: "left rail opacity", min: 0, max: 1, step: 0.01 },
    { key: "panel", label: "panel", min: 0.05, max: 0.35, step: 0.005 },
    { key: "raised", label: "raised", min: 0.05, max: 0.4, step: 0.005 },
    { key: "pop", label: "floating", min: 0.05, max: 0.45, step: 0.005 },
    { key: "hue", label: "tint hue", min: 0, max: 360, step: 1 },
    { key: "chroma", label: "tint strength", min: 0, max: 0.05, step: 0.001 },
  ];

  const account = $derived($cloud.account);
  // The real limit, ignoring an override, so the percentage slider means the
  // same thing after you have already moved it.
  const limitGb = $derived(
    account && account.storage_limit_bytes > 0
      ? account.storage_limit_bytes / GB
      : 5,
  );

  let pct = $state(0);
  $effect(() => {
    if (!account || account.storage_limit_bytes <= 0) return;
    if ($cloudOverrides?.storage_used_bytes === undefined) {
      pct = Math.round(
        (account.storage_used_bytes / account.storage_limit_bytes) * 100,
      );
    }
  });

  /** A plan is not one field. The server resolves the whole set from it
   *  (`plans.rs`), so switching to Pro here has to move the device and storage
   *  ceilings with it or the panel shows a Pro account still capped at 3
   *  devices, which is a state that cannot happen. */
  function applyPlan(next: "free" | "pro") {
    const GB_ = 1024 ** 3;
    setOverrides(
      next === "pro"
        ? {
            plan: "pro",
            devices_limit: -1,
            storage_limit_bytes: 100 * GB_,
            max_save_size_bytes: 10 * GB_,
          }
        : {
            plan: "free",
            devices_limit: 3,
            storage_limit_bytes: 2 * GB_,
            max_save_size_bytes: 1 * GB_,
          },
    );
  }

  function applyQuota(next: number) {
    pct = next;
    const limit = limitGb * GB;
    setOverrides({
      storage_used_bytes: Math.round((limit * next) / 100),
      storage_limit_bytes: limit,
      storage_status: next >= 100 ? "full" : next >= 80 ? "purging" : "ok",
    });
  }
</script>

{#if $debugPanelOpen}
  <div
    class="fixed bottom-4 right-4 z-[200] max-h-[80vh] w-80 overflow-y-auto rounded-lg border border-violet-500/40 bg-layer-3 p-3 font-mono text-xs text-zinc-300 shadow-2xl backdrop-blur"
  >
    <header class="mb-3 flex items-center justify-between">
      <span class="flex items-center gap-1.5 font-semibold text-violet-300">
        <Bug size={13} /> debug
      </span>
      <button
        type="button"
        class="text-zinc-500 hover:text-zinc-200"
        onclick={() => debugPanelOpen.set(false)}
        aria-label="close"
      >
        <X size={14} />
      </button>
    </header>

    {#if !account}
      <p class="text-zinc-500">
        No cloud account in this session, so there is nothing to paint over.
      </p>
    {:else}
      <label class="mb-1 flex items-baseline justify-between" for="dbg-quota">
        <span>storage used</span>
        <span class="text-zinc-100">{pct}% of {limitGb.toFixed(0)} GB</span>
      </label>
      <input
        id="dbg-quota"
        type="range"
        min="0"
        max="110"
        bind:value={pct}
        oninput={() => applyQuota(pct)}
        class="mb-1 w-full accent-violet-500"
      />
      <div class="mb-3 flex gap-1">
        {#each [0, 50, 80, 95, 100] as p (p)}
          <button
            type="button"
            class="rounded border border-zinc-700 px-1.5 py-0.5 hover:border-violet-500 hover:text-zinc-100"
            onclick={() => applyQuota(p)}
          >
            {p}%
          </button>
        {/each}
      </div>

      <div class="mb-3">
        <span class="mb-1 block">plan</span>
        <div class="flex gap-1">
          {#each ["free", "pro"] as p (p)}
            <button
              type="button"
              class="flex-1 rounded border px-1.5 py-0.5 {account.plan === p
                ? 'border-violet-500 text-zinc-100'
                : 'border-zinc-700 hover:border-violet-500'}"
              onclick={() => applyPlan(p as "free" | "pro")}
            >
              {p}
            </button>
          {/each}
        </div>
      </div>

      <div class="mb-3">
        <span class="mb-1 block">
          devices {account.devices_used} / {account.devices_limit < 0
            ? "∞"
            : account.devices_limit}
        </span>
        <div class="flex gap-1">
          <button
            type="button"
            class="flex-1 rounded border border-zinc-700 px-1.5 py-0.5 hover:border-violet-500"
            onclick={() => setOverrides({ devices_used: 3, devices_limit: 3 })}
          >
            3 / 3
          </button>
          <button
            type="button"
            class="flex-1 rounded border border-zinc-700 px-1.5 py-0.5 hover:border-violet-500"
            onclick={() => setOverrides({ devices_used: 6, devices_limit: 3 })}
          >
            6 / 3
          </button>
          <button
            type="button"
            class="flex-1 rounded border border-zinc-700 px-1.5 py-0.5 hover:border-violet-500"
            onclick={() =>
              debugDeviceLimit.set({ used: 4, limit: 3, plan: "free" })}
          >
            refused
          </button>
        </div>
      </div>

      <div class="mb-3">
        <span class="mb-1 block">storage status</span>
        <div class="flex gap-1">
          {#each ["ok", "purging", "full", "grace"] as st (st)}
            <button
              type="button"
              class="flex-1 rounded border border-zinc-700 px-1 py-0.5 hover:border-violet-500"
              onclick={() =>
                setOverrides({
                  storage_status: st as "ok" | "purging" | "full" | "grace",
                })}
            >
              {st}
            </button>
          {/each}
        </div>
      </div>

      <button
        type="button"
        class="w-full rounded border border-zinc-700 px-2 py-1 hover:border-emerald-500 hover:text-emerald-300"
        onclick={clearOverrides}
      >
        show reality again
      </button>
      {#if $cloudOverrides}
        <p class="mt-2 text-[10px] leading-tight text-amber-400/80">
          The account on screen is painted over. The service is unaffected.
        </p>
      {/if}
    {/if}
    <div class="mt-4 border-t border-white/[0.08] pt-3">
      <span class="mb-2 block font-semibold text-violet-300">surfaces</span>
      {#each SURFACE_SLIDERS as sl (sl.key)}
        <label class="mb-1 flex items-baseline justify-between" for="dbg-s-{sl.key}">
          <span>{sl.label}</span>
          <span class="text-zinc-100">{tune[sl.key]}</span>
        </label>
        <input
          id="dbg-s-{sl.key}"
          type="range"
          min={sl.min}
          max={sl.max}
          step={sl.step}
          value={tune[sl.key]}
          oninput={(e) => setTune(sl.key, Number((e.currentTarget as HTMLInputElement).value))}
          class="mb-2 w-full accent-violet-500"
        />
      {/each}
      <button
        type="button"
        class="w-full rounded border border-zinc-700 px-2 py-1 hover:border-emerald-500 hover:text-emerald-300"
        onclick={() => applySurfaces(null)}
      >
        surfaces back to the stylesheet
      </button>
    </div>
  </div>
{/if}
