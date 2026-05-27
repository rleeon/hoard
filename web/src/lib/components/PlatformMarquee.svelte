<script lang="ts">
  import { _ } from 'svelte-i18n';

  const items = [
    'platforms.steam',
    'platforms.epic',
    'platforms.gog',
    'platforms.xbox',
    'platforms.proton',
    'platforms.wine',
    'platforms.lutris',
    'platforms.bottles',
    'platforms.heroic',
    'platforms.itch'
  ];

  // Duplicated so the translateX(-50%) loop is seamless.
  const track = [...items, ...items];
</script>

<div class="relative">
  <div class="mb-4 text-center text-[11px] font-medium uppercase tracking-[0.18em] text-zinc-500">
    {$_('platforms.title')}
  </div>
  <div class="marquee-mask overflow-hidden">
    <div class="flex w-max items-center gap-10 animate-marquee whitespace-nowrap py-2">
      {#each track as key, i (i)}
        <span
          class="inline-flex items-center gap-2 font-mono text-sm font-medium uppercase tracking-wider text-zinc-400"
        >
          <span class="h-1.5 w-1.5 rounded-full bg-emerald-500/60"></span>
          {$_(key)}
        </span>
      {/each}
    </div>
  </div>
</div>
