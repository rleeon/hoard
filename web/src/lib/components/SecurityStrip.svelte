<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { Globe2, Fingerprint, Code2, FileDown } from 'lucide-svelte';

  const items = [
    { key: 'eu', icon: Globe2 },
    { key: 'sha', icon: Fingerprint },
    { key: 'agpl', icon: Code2 },
    { key: 'export', icon: FileDown }
  ];
</script>

<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal mx-auto max-w-2xl text-center" use:reveal>
      <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
        Trust & data
      </div>
      <h2 class="mt-3 text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {$_('security.title')}
      </h2>
      <p class="mt-3 text-pretty text-zinc-400">{$_('security.subtitle')}</p>
    </div>

    <div class="mt-12 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each items as it, i (it.key)}
        <div
          class="reveal card-edge group flex flex-col rounded-xl border border-white/[0.06] bg-white/[0.02] p-5 transition-colors hover:border-emerald-500/30"
          use:reveal={{ delay: i * 60 }}
        >
          <div class="flex items-center gap-3">
            <span
              class="grid h-9 w-9 place-items-center rounded-lg bg-emerald-500/10 text-emerald-300 ring-1 ring-inset ring-emerald-400/15 transition-transform duration-500 group-hover:scale-110"
            >
              <it.icon class="h-4 w-4" />
            </span>
            <h3 class="text-sm font-semibold text-white">{$_(`security.${it.key}_title`)}</h3>
          </div>
          <p class="mt-3 text-xs leading-relaxed text-zinc-400">{$_(`security.${it.key}_body`)}</p>
        </div>
      {/each}
    </div>
  </div>
</section>
