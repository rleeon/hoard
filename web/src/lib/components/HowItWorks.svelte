<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { Download, Search, GitBranch } from 'lucide-svelte';

  const steps = [
    { n: '01', icon: Download, title: 'how.s1_title', body: 'how.s1_body' },
    { n: '02', icon: Search, title: 'how.s2_title', body: 'how.s2_body' },
    { n: '03', icon: GitBranch, title: 'how.s3_title', body: 'how.s3_body' }
  ];
</script>

<section class="relative border-t border-white/[0.05]">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal mx-auto max-w-2xl text-center" use:reveal>
      <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
        Hoard 101
      </div>
      <h2 class="mt-3 text-balance text-3xl font-bold tracking-tight text-white sm:text-4xl">
        {$_('how.title')}
      </h2>
      <p class="mt-3 text-pretty text-zinc-400">{$_('how.subtitle')}</p>
    </div>

    <div class="relative mt-14 grid gap-5 lg:grid-cols-3">
      <span
        class="pointer-events-none absolute left-0 right-0 top-12 hidden h-px bg-gradient-to-r from-transparent via-emerald-500/30 to-transparent lg:block"
        aria-hidden="true"
      ></span>
      {#each steps as s, i (s.n)}
        <article
          class="reveal card-edge relative rounded-2xl border border-white/[0.06] bg-white/[0.025] p-7 transition-colors hover:border-emerald-500/30"
          use:reveal={{ delay: i * 90 }}
        >
          <div class="flex items-center gap-3">
            <span
              class="font-mono text-[11px] font-semibold tracking-wider text-emerald-400/80"
            >
              {s.n}
            </span>
            <span class="h-px flex-1 bg-white/[0.08]"></span>
            <span
              class="grid h-10 w-10 place-items-center rounded-xl bg-gradient-to-br from-emerald-500/15 to-emerald-500/[0.04] text-emerald-300 ring-1 ring-inset ring-emerald-400/20"
            >
              <s.icon class="h-5 w-5" />
            </span>
          </div>
          <h3 class="mt-5 text-lg font-semibold tracking-tight text-white">
            {$_(s.title)}
          </h3>
          <p class="mt-2 text-sm leading-relaxed text-zinc-400">{$_(s.body)}</p>
        </article>
      {/each}
    </div>
  </div>
</section>
