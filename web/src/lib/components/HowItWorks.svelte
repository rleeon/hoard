<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { Download } from 'lucide-svelte';
  import { reveal } from '$lib/actions/reveal';
  import { localeHref } from '$lib/i18n/href';
  import Button from './Button.svelte';

  // The "How it works" section.
  //
  // Content, as the user specified it:
  //   - Step 01 "Install & sign in": a download button, no image.
  //   - Step 02 "Detect your library": the real Dashboard screenshot.
  //   - Step 03 "Sync & history": the real History screenshot, in the same
  //     16:10 frame as step 02. This one is object-fill, so the smaller
  //     screenshot fills the box exactly, no bars, nothing cropped, while
  //     step 02 keeps object-cover.
  //
  // The ?v=1 on the screenshots is a cache key, not a version: the CDN in
  // front of the site cached a 404 for these paths in the minutes between the
  // HTML going live and the assets landing, and it holds that for four hours.
  // A fresh query string sidesteps it; it can go once the cache is purged.
  //
  // Design (the user's pick out of ten layouts): step 01 as a full-width CTA
  // banner, accent gradient, centered, download button, and steps 02/03 as
  // two cards below, both screenshots forced into the same 16:10 box so the
  // history one no longer reads as small next to the dashboard one.

  const steps = [
    { n: '01', title: 'how.s1_title', body: 'how.s1_body' },
    { n: '02', title: 'how.s2_title', body: 'how.s2_body', img: '/dashboard.webp?v=1', alt: 'slot.how_library', fit: 'cover' },
    { n: '03', title: 'how.s3_title', body: 'how.s3_body', img: '/history.webp?v=1', alt: 'slot.how_history', fit: 'fill' }
  ];
</script>

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal max-w-2xl" use:reveal>
      <p class="kicker">{$_('how.kicker')}</p>
      <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
        {$_('how.title')}
      </h2>
      <p class="mt-3 text-pretty text-ink-soft">{$_('how.subtitle')}</p>
    </div>

    <div class="mt-12 grid gap-5 lg:grid-cols-2">
      {#each steps as s, i (s.n)}
        {#if !s.img}
          <article
            class="reveal flex flex-col items-center justify-center gap-6 rounded-2xl border border-accent/30 bg-gradient-to-br from-accent/15 to-transparent p-10 text-center lg:col-span-2"
            use:reveal
          >
            <span class="font-mono text-[11px] font-medium tracking-[0.16em] text-accent">{s.n}</span>
            <h3 class="text-2xl font-semibold text-ink">{$_(s.title)}</h3>
            <p class="max-w-md text-sm leading-relaxed text-ink-soft">{$_(s.body)}</p>
            <div class="w-full max-w-sm">
              <Button href={$localeHref('/download')} variant="primary" size="lg" full>
                {$_('hero.cta_start')}
                <Download class="h-4 w-4" aria-hidden="true" />
              </Button>
            </div>
          </article>
        {:else}
          <article class="reveal rounded-2xl border border-line bg-surface p-7" use:reveal={{ delay: i * 80 }}>
            <span class="font-mono text-[11px] font-medium tracking-[0.16em] text-accent">{s.n}</span>
            <div class="mt-4 overflow-hidden rounded-xl border border-line">
              <div class="aspect-[16/10] overflow-hidden">
                <img
                  src={s.img}
                  alt={$_(s.alt)}
                  loading="lazy"
                  decoding="async"
                  class="block h-full w-full {s.fit === 'cover' ? 'object-cover object-top' : 'object-fill'}"
                />
              </div>
            </div>
            <h3 class="mt-5 text-lg font-semibold text-ink">{$_(s.title)}</h3>
            <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_(s.body)}</p>
          </article>
        {/if}
      {/each}
    </div>
  </div>
</section>
