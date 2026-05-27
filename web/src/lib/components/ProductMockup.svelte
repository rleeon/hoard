<script lang="ts">
  // Static SVG mockup of the Hoard desktop UI. Pure markup so it
  // never JS-renders or fetches anything, but reads as an actual
  // product screenshot at a glance.
</script>

<div
  class="relative overflow-hidden rounded-2xl border border-white/[0.06] bg-zinc-950/80 p-1 shadow-[0_40px_100px_-30px_rgba(0,0,0,0.7),0_0_0_1px_rgba(16,185,129,0.06)]"
>
  <div
    class="pointer-events-none absolute -inset-px rounded-2xl bg-gradient-to-br from-emerald-500/20 via-transparent to-teal-500/10 opacity-50"
    aria-hidden="true"
  ></div>

  <div class="relative overflow-hidden rounded-xl bg-[#06080a]">
    <!-- title bar -->
    <div class="flex items-center justify-between border-b border-white/[0.06] px-4 py-2.5">
      <div class="flex items-center gap-1.5">
        <span class="h-2.5 w-2.5 rounded-full bg-red-500/70"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-amber-500/70"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-emerald-500/70"></span>
      </div>
      <div class="text-[11px] font-medium tracking-wider text-zinc-500">
        Hoard · Library
      </div>
      <div class="flex items-center gap-1.5">
        <span class="h-1.5 w-1.5 animate-pulse-glow rounded-full bg-emerald-400"></span>
        <span class="text-[10px] uppercase tracking-wider text-emerald-400/80">synced</span>
      </div>
    </div>

    <div class="grid grid-cols-[176px_1fr] gap-0">
      <!-- sidebar -->
      <aside class="border-r border-white/[0.06] bg-zinc-950/50 p-3 text-xs">
        <div class="mb-3 px-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
          Hoard
        </div>
        {#each ['Library', 'Dashboard', 'History', 'Settings'] as item, i (item)}
          <div
            class={`mb-1 flex items-center gap-2 rounded-md px-2 py-1.5 transition-colors ${
              i === 0
                ? 'bg-emerald-500/10 text-emerald-200 ring-1 ring-inset ring-emerald-400/20'
                : 'text-zinc-400'
            }`}
          >
            <span
              class={`h-1.5 w-1.5 rounded-full ${i === 0 ? 'bg-emerald-400' : 'bg-zinc-600'}`}
            ></span>
            {item}
          </div>
        {/each}

        <div class="mt-6 rounded-lg border border-emerald-500/15 bg-emerald-500/[0.04] p-2.5">
          <div class="text-[10px] uppercase tracking-wider text-emerald-300/80">Storage</div>
          <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-zinc-900">
            <div class="h-full w-[34%] rounded-full bg-gradient-to-r from-emerald-500 to-teal-400"></div>
          </div>
          <div class="mt-1.5 text-[10px] text-zinc-400">347 MB / 1 GB</div>
        </div>
      </aside>

      <!-- content -->
      <div class="p-4">
        <div class="mb-4 flex items-center justify-between">
          <div>
            <div class="text-sm font-semibold text-white">Library</div>
            <div class="text-[11px] text-zinc-500">12 games tracked · 47 snapshots</div>
          </div>
          <div class="flex items-center gap-2">
            <span class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[10px] font-medium text-zinc-300">
              Auto-sync ON
            </span>
            <span
              class="rounded-md bg-emerald-600 px-2.5 py-1 text-[10px] font-medium text-white shadow-[0_4px_12px_-4px_rgba(16,185,129,0.7)]"
            >
              + Add game
            </span>
          </div>
        </div>

        <!-- save rows -->
        <div class="space-y-1.5">
          {#each [
            { game: 'Stardew Valley', label: 'default', v: 48, size: '8.34 MiB', when: '2m ago', status: 'ok' },
            { game: 'Hollow Knight', label: 'profile-1', v: 22, size: '1.18 MiB', when: '14m ago', status: 'ok' },
            { game: 'Disco Elysium', label: 'autosave', v: 12, size: '74.2 MiB', when: '1h ago', status: 'syncing' },
            { game: 'Outer Wilds', label: 'expedition', v: 6, size: '2.91 MiB', when: '3h ago', status: 'ok' },
            { game: 'Hades II', label: 'main', v: 31, size: '12.8 MiB', when: 'just now', status: 'ok' }
          ] as row, i (row.game)}
            <div
              class={`flex items-center gap-3 rounded-md border border-white/[0.04] bg-white/[0.015] px-3 py-2 transition-colors hover:border-emerald-500/20 ${
                i === 4 ? 'border-emerald-500/30 bg-emerald-500/[0.04]' : ''
              }`}
            >
              <div class="grid h-7 w-7 place-items-center rounded-md bg-gradient-to-br from-emerald-500/15 to-emerald-500/[0.02] text-[10px] font-bold text-emerald-300 ring-1 ring-inset ring-emerald-400/20">
                {row.game[0]}
              </div>
              <div class="flex-1 min-w-0">
                <div class="text-xs font-medium text-zinc-100">{row.game}</div>
                <div class="font-mono text-[10px] text-zinc-500">{row.label} · v{row.v}</div>
              </div>
              <div class="font-mono text-[10px] tabular-nums text-zinc-400">{row.size}</div>
              <div class="font-mono text-[10px] text-zinc-500">{row.when}</div>
              <span
                class={`h-1.5 w-1.5 rounded-full ${
                  row.status === 'syncing'
                    ? 'bg-amber-400 animate-pulse-glow'
                    : 'bg-emerald-400'
                }`}
              ></span>
            </div>
          {/each}
        </div>

        <!-- mini chart -->
        <div class="mt-4 rounded-lg border border-white/[0.06] bg-zinc-950/40 p-3">
          <div class="mb-2 flex items-center justify-between">
            <div class="text-[10px] uppercase tracking-wider text-zinc-500">
              Snapshots this week
            </div>
            <div class="font-mono text-[10px] text-emerald-300">+18%</div>
          </div>
          <svg viewBox="0 0 280 50" class="block h-12 w-full" aria-hidden="true">
            <defs>
              <linearGradient id="mock-spark" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0" stop-color="#34d399" stop-opacity="0.5" />
                <stop offset="1" stop-color="#34d399" stop-opacity="0" />
              </linearGradient>
            </defs>
            <path
              d="M0 40 L 20 32 L 40 36 L 60 28 L 80 30 L 100 18 L 120 22 L 140 14 L 160 20 L 180 10 L 200 16 L 220 8 L 240 12 L 260 6 L 280 10"
              fill="none"
              stroke="#34d399"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <path
              d="M0 40 L 20 32 L 40 36 L 60 28 L 80 30 L 100 18 L 120 22 L 140 14 L 160 20 L 180 10 L 200 16 L 220 8 L 240 12 L 260 6 L 280 10 L 280 50 L 0 50 Z"
              fill="url(#mock-spark)"
            />
          </svg>
        </div>
      </div>
    </div>
  </div>
</div>
