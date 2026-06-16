<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import { localeHref } from '$lib/i18n/href';
  import { reveal } from '$lib/actions/reveal';
  import { listGuides } from '$lib/guides';
  import { DEFAULT_LOCALE, isLocale, SITE_URL, withLocale, type Locale } from '$lib/i18n/locales';
  import { ArrowRight } from 'lucide-svelte';

  const active = $derived<Locale>(isLocale($locale) ? ($locale as Locale) : DEFAULT_LOCALE);
  const guides = $derived(listGuides(active));

  // ItemList structured data so search engines see the guide collection.
  const listLd = $derived(
    `<script type="application/ld+json">${JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'ItemList',
      name: $_('seo.guides.title'),
      itemListElement: guides.map((g, i) => ({
        '@type': 'ListItem',
        position: i + 1,
        url: SITE_URL + withLocale(`/guides/${g.slug}`, active),
        name: g.title
      }))
    })}<\/script>`
  );
</script>

<Seo path="/guides" key="guides" />
<svelte:head>
  {@html listLd}
</svelte:head>

<section class="mx-auto max-w-3xl px-4 py-16 sm:px-6 sm:py-24">
  <header use:reveal class="reveal">
    <h1 class="font-display text-4xl font-semibold tracking-tight text-ink sm:text-5xl">
      {$_('guides.index_heading')}
    </h1>
    <p class="mt-4 max-w-2xl text-lg leading-relaxed text-ink-soft">
      {$_('guides.index_intro')}
    </p>
  </header>

  <ul class="mt-12 space-y-4">
    {#each guides as g (g.slug)}
      <li use:reveal class="reveal">
        <a
          href={$localeHref(`/guides/${g.slug}`)}
          class="group block rounded-xl border border-line bg-surface p-5 ring-focus transition-colors hover:border-line-strong"
        >
          <div class="flex items-start justify-between gap-4">
            <div>
              <h2 class="font-display text-lg font-semibold text-ink">{g.title}</h2>
              <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{g.description}</p>
            </div>
            <ArrowRight
              class="mt-1 h-5 w-5 shrink-0 text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-accent"
            />
          </div>
        </a>
      </li>
    {/each}
  </ul>
</section>
