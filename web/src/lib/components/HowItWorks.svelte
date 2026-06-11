<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { Download, ScanSearch, RefreshCw } from 'lucide-svelte';

  const steps = [
    { n: '01', icon: Download, title: 'how.s1_title', body: 'how.s1_body' },
    { n: '02', icon: ScanSearch, title: 'how.s2_title', body: 'how.s2_body' },
    { n: '03', icon: RefreshCw, title: 'how.s3_title', body: 'how.s3_body' }
  ];
</script>

<section class="border-t border-line bg-surface">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal max-w-2xl" use:reveal>
      <p class="kicker">{$_('how.kicker')}</p>
      <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
        {$_('how.title')}
      </h2>
      <p class="mt-3 text-pretty text-ink-soft">{$_('how.subtitle')}</p>
    </div>

    <div
      class="mt-12 grid overflow-hidden rounded-2xl border border-line bg-bg sm:grid-cols-3"
    >
      {#each steps as s, i (s.n)}
        <article
          class="reveal p-7 {i > 0 ? 'border-t border-line sm:border-l sm:border-t-0' : ''}"
          use:reveal={{ delay: i * 80 }}
        >
          <div class="flex items-center justify-between">
            <span class="font-mono text-[11px] font-medium tracking-[0.16em] text-accent">
              {s.n}
            </span>
            <s.icon class="h-5 w-5 text-ink-faint" />
          </div>
          <h3 class="mt-5 text-lg font-semibold text-ink">{$_(s.title)}</h3>
          <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_(s.body)}</p>
        </article>
      {/each}
    </div>
  </div>
</section>
