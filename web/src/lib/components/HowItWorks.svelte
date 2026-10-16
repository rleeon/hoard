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
  //     16:10 frame as step 02. It is wider than the frame (773x444 against
  //     16:10), so it is `object-contain` over the screenshot's own near-black:
  //     `fill` stretched the rows 9% and `cover` would eat the Restore buttons
  //     at the right edge. The letterbox is invisible because the tile behind
  //     it is the same colour as the shot. Step 02 keeps object-cover.
  //
  // The ?v=1 on the screenshots is a cache key, not a version: the CDN in
  // front of the site cached a 404 for these paths in the minutes between the
  // HTML going live and the assets landing, and it holds that for four hours.
  // A fresh query string sidesteps it; it can go once the cache is purged.
  //
  // Design: black and white only, on the user's call ("no greens"); the hero
  // above is the reference. Step 01 is a full-width row with the download on
  // its right (the one green allowed: the app's black, green-edged button), steps 02/03 two cards below with both screenshots forced into
  // the same 16:10 box.

  const steps = [
    { n: '01', title: 'how.s1_title', body: 'how.s1_body' },
    { n: '02', title: 'how.s2_title', body: 'how.s2_body', img: '/dashboard.webp?v=1', alt: 'slot.how_library', fit: 'cover' },
    { n: '03', title: 'how.s3_title', body: 'how.s3_body', img: '/history.webp?v=2', alt: 'slot.how_history', fit: 'contain' }
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
            class="reveal flex flex-col gap-8 rounded-3xl border border-white/[0.08] bg-black p-8 sm:p-10 lg:col-span-2 lg:flex-row lg:items-center lg:justify-between"
            use:reveal
          >
            <div class="max-w-xl">
              <span class="font-mono text-xs tracking-[0.16em] text-ink-faint">{s.n}</span>
              <h3 class="mt-3 text-2xl font-semibold text-ink sm:text-3xl">{$_(s.title)}</h3>
              <p class="mt-3 text-pretty leading-relaxed text-ink-soft">{$_(s.body)}</p>
            </div>
            <div class="w-full flex-none lg:w-80">
              <Button href={$localeHref('/download')} variant="primary" size="xl" full>
                {$_('hero.cta_start')}
                <Download class="h-4 w-4" aria-hidden="true" />
              </Button>
            </div>
          </article>
        {:else}
          <article
            class="reveal flex flex-col rounded-3xl border border-white/[0.08] bg-black p-4 sm:p-5"
            use:reveal={{ delay: i * 80 }}
          >
            <div class="overflow-hidden rounded-2xl border border-white/[0.08]">
              <div
                class="aspect-[16/10] overflow-hidden {s.fit === 'contain' ? 'bg-[#030303]' : ''}"
              >
                <img
                  src={s.img}
                  alt={$_(s.alt)}
                  loading="lazy"
                  decoding="async"
                  class="block h-full w-full {s.fit === 'cover'
                    ? 'object-cover object-top'
                    : 'object-contain'}"
                />
              </div>
            </div>
            <div class="px-3 pb-3 pt-6 sm:px-4">
              <span class="font-mono text-xs tracking-[0.16em] text-ink-faint">{s.n}</span>
              <h3 class="mt-2 text-xl font-semibold text-ink">{$_(s.title)}</h3>
              <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_(s.body)}</p>
            </div>
          </article>
        {/if}
      {/each}
    </div>
  </div>
</section>
