<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import { localeHref } from '$lib/i18n/href';
  import { DEFAULT_LOCALE, isLocale, SITE_URL, withLocale, localePrefix, HREFLANG, type Locale } from '$lib/i18n/locales';
  import { ArrowLeft } from 'lucide-svelte';

  let { data } = $props();
  const guide = $derived(data.guide);
  const active = $derived<Locale>(isLocale($locale) ? ($locale as Locale) : DEFAULT_LOCALE);
  const url = $derived(SITE_URL + withLocale(`/guides/${guide.slug}`, active));

  // Guide bodies use root-relative links (e.g. /guides/ludusavi-alternative).
  // The HTML is parsed once per locale-file, so prefix in-app links with the
  // active locale here — otherwise a Spanish page would link to bare English.
  // English prefix is '' (no change); external/anchor/mailto links don't match.
  const localizedHtml = $derived(
    localePrefix(active)
      ? guide.html.replace(/href="\/(?!\/)/g, `href="${localePrefix(active)}/`)
      : guide.html
  );

  // JSON-LD is emitted as a raw script block, so a closing script tag coming
  // from guide content would end it early. Escaping `<` keeps the payload inert
  // and is still valid JSON.
  const ld = (data: unknown) =>
    `<script type="application/ld+json">${JSON.stringify(data).replace(/</g, '\\u003c')}<\/script>`;

  const articleLd = $derived(
    ld({
      '@context': 'https://schema.org',
      '@type': 'TechArticle',
      headline: guide.title,
      description: guide.description,
      inLanguage: HREFLANG[active],
      ...(guide.updated ? { datePublished: guide.updated, dateModified: guide.updated } : {}),
      mainEntityOfPage: url,
      author: { '@type': 'Organization', name: 'Hoard', url: SITE_URL },
      publisher: {
        '@type': 'Organization',
        name: 'Hoard',
        logo: { '@type': 'ImageObject', url: `${SITE_URL}/icon-512.png` }
      }
    })
  );

  // A guide that carries an FAQ section also answers as a FAQPage: the pairs
  // reach a model already split into question and answer, which is the form it
  // quotes. Empty when the guide has no `<!-- faq -->` block.
  const faqLd = $derived(
    guide.faq.length
      ? ld({
          '@context': 'https://schema.org',
          '@type': 'FAQPage',
          inLanguage: HREFLANG[active],
          mainEntity: guide.faq.map((f) => ({
            '@type': 'Question',
            name: f.question,
            acceptedAnswer: { '@type': 'Answer', text: f.answer }
          }))
        })
      : ''
  );
</script>

<Seo
  path={`/guides/${guide.slug}`}
  title={`${guide.title} — Hoard`}
  description={guide.description}
  type="article"
/>
<svelte:head>
  {@html articleLd}
  {@html faqLd}
</svelte:head>

<article class="mx-auto max-w-2xl px-4 py-16 sm:px-6 sm:py-24">
  <a
    href={$localeHref('/guides')}
    class="inline-flex items-center gap-1.5 rounded-md text-sm text-ink-soft ring-focus transition-colors hover:text-ink"
  >
    <ArrowLeft class="h-4 w-4" />
    {$_('guides.back')}
  </a>

  <h1 class="mt-6 font-display text-3xl font-semibold tracking-tight text-ink sm:text-4xl">
    {guide.title}
  </h1>
  {#if guide.updated}
    <p class="mt-3 font-mono text-xs uppercase tracking-[0.14em] text-ink-faint">
      {$_('guides.updated')}
      {guide.updated}
    </p>
  {/if}

  <div class="prose mt-8">
    {@html localizedHtml}
  </div>

  <div class="mt-14 rounded-xl border border-line bg-surface p-6">
    <h2 class="font-display text-lg font-semibold text-ink">{$_('guides.cta_title')}</h2>
    <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">{$_('guides.cta_text')}</p>
    <a
      href={$localeHref('/download')}
      class="mt-4 inline-flex h-10 items-center rounded-lg bg-accent px-4 text-sm font-medium text-pine ring-focus transition-colors hover:bg-emerald-300"
    >
      {$_('nav.download')}
    </a>
  </div>
</article>

<style>
  .prose {
    color: var(--color-ink-soft);
    line-height: 1.7;
    font-size: 1rem;
  }
  .prose :global(h2) {
    color: var(--color-ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.4rem;
    letter-spacing: -0.01em;
    margin-top: 2.5rem;
    margin-bottom: 0.75rem;
  }
  .prose :global(h3) {
    color: var(--color-ink);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 1.15rem;
    margin-top: 2rem;
    margin-bottom: 0.5rem;
  }
  .prose :global(p) {
    margin: 1rem 0;
  }
  .prose :global(a) {
    color: var(--color-accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .prose :global(strong) {
    color: var(--color-ink);
    font-weight: 600;
  }
  .prose :global(ul),
  .prose :global(ol) {
    margin: 1rem 0;
    padding-left: 1.5rem;
  }
  .prose :global(ul) {
    list-style: disc;
  }
  .prose :global(ol) {
    list-style: decimal;
  }
  .prose :global(li) {
    margin: 0.4rem 0;
  }
  .prose :global(li)::marker {
    color: var(--color-ink-faint);
  }
  .prose :global(code) {
    font-family: var(--font-mono);
    font-size: 0.85em;
    background: var(--color-pine-elev);
    border: 1px solid var(--color-pine-line);
    border-radius: 0.35rem;
    padding: 0.1rem 0.35rem;
    color: var(--color-ink);
  }
  .prose :global(pre) {
    background: var(--color-pine-elev);
    border: 1px solid var(--color-pine-line);
    border-radius: 0.6rem;
    padding: 1rem 1.2rem;
    overflow-x: auto;
    margin: 1.25rem 0;
  }
  .prose :global(pre code) {
    background: none;
    border: none;
    padding: 0;
    font-size: 0.85rem;
  }
  /* First guide to carry a Markdown table is the tool comparison, and a
     six-column table never fits a phone. `display: block` + `width: max-content`
     is the only way to make it scroll on its own without a wrapper element —
     the Markdown pipeline strips raw HTML, so there is no div to hang it on. */
  .prose :global(table) {
    display: block;
    width: max-content;
    max-width: 100%;
    overflow-x: auto;
    border-collapse: collapse;
    margin: 1.5rem 0;
    font-size: 0.92rem;
  }
  .prose :global(th),
  .prose :global(td) {
    border: 1px solid var(--color-pine-line);
    padding: 0.55rem 0.8rem;
    text-align: left;
    vertical-align: top;
  }
  .prose :global(th) {
    background: var(--color-pine-elev);
    color: var(--color-ink);
    font-weight: 600;
    white-space: nowrap;
  }
  .prose :global(tbody tr:nth-child(even)) {
    background: var(--color-pine);
  }
  .prose :global(blockquote) {
    border-left: 3px solid var(--color-accent-deep);
    padding-left: 1rem;
    margin: 1.25rem 0;
    color: var(--color-ink-soft);
  }
  .prose :global(h2:first-child),
  .prose :global(h3:first-child),
  .prose :global(p:first-child) {
    margin-top: 0;
  }
</style>
