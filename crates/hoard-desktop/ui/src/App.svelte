<script lang="ts">
  import { Archive, Library, History, Settings as SettingsIcon } from "lucide-svelte";
  import Dashboard from "./routes/Dashboard.svelte";

  // Phase 0 has only one route; later phases swap in svelte-spa-router.
  const nav = [
    { label: "Library", icon: Library, active: false },
    { label: "Dashboard", icon: Archive, active: true },
    { label: "History", icon: History, active: false },
    { label: "Settings", icon: SettingsIcon, active: false },
  ];
</script>

<div class="flex h-full">
  <!-- Sidebar -->
  <aside
    class="flex w-60 shrink-0 flex-col border-r border-zinc-800 bg-zinc-950"
  >
    <div class="flex items-center gap-2 px-5 py-5">
      <div
        class="flex h-9 w-9 items-center justify-center rounded-lg bg-amber-500/10 text-amber-500 ring-1 ring-amber-500/40"
      >
        <Archive size={20} />
      </div>
      <div>
        <div class="text-base font-semibold tracking-tight">Hoard</div>
        <div class="text-xs text-zinc-500">v0.2.0-dev</div>
      </div>
    </div>

    <nav class="flex-1 space-y-1 px-3 py-2">
      {#each nav as item (item.label)}
        <button
          type="button"
          class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors {item.active
            ? 'bg-zinc-800 text-zinc-50'
            : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-100'}"
        >
          <item.icon size={18} />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="border-t border-zinc-800 px-5 py-4 text-xs text-zinc-500">
      Self-hosted save sync.<br />
      Your server, your data.
    </div>
  </aside>

  <!-- Main content -->
  <main class="flex-1 overflow-y-auto">
    <Dashboard />
  </main>
</div>
