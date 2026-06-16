<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import {
    LOCALES,
    HREFLANG,
    OG_LOCALE,
    DEFAULT_LOCALE,
    SITE_URL,
    withLocale,
    isLocale,
    type Locale
  } from '$lib/i18n/locales';

  interface Props {
    /** App path without locale prefix, e.g. '/pricing' or '/' for the home. */
    path: string;
    /** i18n key base under `seo.` — reads `seo.<key>.title` / `seo.<key>.desc`. */
    key?: string;
    /** Explicit title; overrides `key`. Used by content pages (guides). */
    title?: string;
    /** Explicit description; overrides `key`. */
    description?: string;
    /** og:type — 'website' (default) or 'article' for guides/blog. */
    type?: string;
    /** OG/Twitter image, absolute path under the site root. */
    image?: string;
  }
  let {
    path,
    key,
    title: titleProp,
    description: descProp,
    type = 'website',
    image = '/app.webp'
  }: Props = $props();

  const active = $derived<Locale>(isLocale($locale) ? ($locale as Locale) : DEFAULT_LOCALE);
  const title = $derived(titleProp ?? (key ? $_(`seo.${key}.title`) : ''));
  const desc = $derived(descProp ?? (key ? $_(`seo.${key}.desc`) : ''));
  const canonical = $derived(SITE_URL + withLocale(path, active));
  const imageUrl = $derived(SITE_URL + image);
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={desc} />
  <link rel="canonical" href={canonical} />

  {#each LOCALES as l (l)}
    <link rel="alternate" hreflang={HREFLANG[l]} href={SITE_URL + withLocale(path, l)} />
  {/each}
  <link rel="alternate" hreflang="x-default" href={SITE_URL + withLocale(path, DEFAULT_LOCALE)} />

  <meta property="og:type" content={type} />
  <meta property="og:site_name" content="Hoard" />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={desc} />
  <meta property="og:url" content={canonical} />
  <meta property="og:image" content={imageUrl} />
  <meta property="og:locale" content={OG_LOCALE[active]} />
  {#each LOCALES.filter((l) => l !== active) as l (l)}
    <meta property="og:locale:alternate" content={OG_LOCALE[l]} />
  {/each}

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={desc} />
  <meta name="twitter:image" content={imageUrl} />
</svelte:head>
