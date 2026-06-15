<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { ChevronDown, Mail, Github, Search } from 'lucide-svelte';

  type Faq = { q: string; a: string; group: 'billing' | 'privacy' | 'selfhost' | 'limits' };

  const faqs: Faq[] = [
    { q: 'help.faq_q1', a: 'help.faq_a1', group: 'billing' },
    { q: 'help.faq_q2', a: 'help.faq_a2', group: 'billing' },
    { q: 'help.faq_q3', a: 'help.faq_a3', group: 'selfhost' },
    { q: 'help.faq_q4', a: 'help.faq_a4', group: 'privacy' },
    { q: 'help.faq_q5', a: 'help.faq_a5', group: 'privacy' },
    { q: 'help.faq_q6', a: 'help.faq_a6', group: 'limits' },
    { q: 'help.faq_q7', a: 'help.faq_a7', group: 'limits' }
  ];

  const groupOrder: Faq['group'][] = ['billing', 'privacy', 'selfhost', 'limits'];

  // FAQPage structured data for rich results, in the page's locale.
  const faqLd = $derived(
    `<script type="application/ld+json">${JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'FAQPage',
      mainEntity: faqs.map((f) => ({
        '@type': 'Question',
        name: $_(f.q),
        acceptedAnswer: { '@type': 'Answer', text: $_(f.a) }
      }))
    })}<\/script>`
  );

  let query = $state('');
  let openKey = $state<string | null>(faqs[0].q);

  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return faqs;
    return faqs.filter((f) => {
      const text = (`${$_(f.q)} ${$_(f.a)}`).toLowerCase();
      return text.includes(q);
    });
  });

  let grouped = $derived.by(() => {
    const by: Record<string, Faq[]> = {};
    for (const f of filtered) {
      (by[f.group] ??= []).push(f);
    }
    return groupOrder
      .map((g) => ({ group: g, items: by[g] ?? [] }))
      .filter((x) => x.items.length > 0);
  });
</script>

<Seo path="/help" key="help" />
<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html faqLd}
</svelte:head>

<section class="mx-auto max-w-3xl px-4 py-16 sm:px-6 sm:py-24">
  <div class="text-center">
    <p class="kicker justify-center">Support</p>
    <h1 class="mt-3 text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('help.title')}
    </h1>
    <p class="mx-auto mt-4 max-w-xl text-pretty leading-relaxed text-ink-soft">
      {$_('help.subtitle')}
    </p>
  </div>

  <!-- Search -->
  <div class="reveal relative mt-12" use:reveal>
    <Search
      class="pointer-events-none absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint"
    />
    <input
      type="search"
      bind:value={query}
      placeholder={$_('help.search_placeholder')}
      class="ring-focus w-full rounded-xl border border-line bg-surface py-3 pl-11 pr-4 text-sm text-ink placeholder:text-ink-faint transition-colors focus:border-accent focus:outline-none"
      aria-label={$_('help.search_placeholder')}
    />
  </div>

  {#if grouped.length === 0}
    <p class="reveal mt-10 text-center text-sm text-ink-soft" use:reveal>
      {$_('help.no_results')}
    </p>
  {:else}
    {#each grouped as g (g.group)}
      <h2 class="kicker mt-12 first:mt-10">
        {$_(`help.group_${g.group}`)}
      </h2>
      <div class="mt-4 space-y-2">
        {#each g.items as f, i (f.q)}
          <div
            class="reveal overflow-hidden rounded-xl border border-line bg-surface transition-colors hover:border-line-strong"
            use:reveal={{ delay: i * 40 }}
          >
            <button
              class="ring-focus flex w-full items-center justify-between gap-4 px-5 py-4 text-left transition-colors hover:bg-bg"
              onclick={() => (openKey = openKey === f.q ? null : f.q)}
              aria-expanded={openKey === f.q}
              aria-controls={`faq-${f.q}`}
            >
              <span class="font-medium text-ink">{$_(f.q)}</span>
              <ChevronDown
                class="h-4 w-4 flex-none text-ink-faint transition-transform duration-300 {openKey ===
                f.q
                  ? 'rotate-180 text-accent'
                  : ''}"
              />
            </button>
            {#if openKey === f.q}
              <div
                id={`faq-${f.q}`}
                class="border-t border-line px-5 py-4 text-sm leading-relaxed text-ink-soft"
                transition:slide={{ duration: 220, easing: cubicOut }}
              >
                {$_(f.a)}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  {/if}

  <h2 class="mt-16 text-xl font-semibold text-ink">{$_('help.contact_title')}</h2>
  <p class="mt-2 text-sm text-ink-soft">{$_('help.contact_body')}</p>

  <div class="mt-5 flex flex-col gap-3 sm:flex-row">
    <Button href="mailto:support@hoard.services" variant="primary" full>
      <Mail class="h-4 w-4" />
      {$_('help.contact_cta_email')}
    </Button>
    <Button
      href="https://github.com/rleeon/hoard/issues/new"
      target="_blank"
      variant="secondary"
      full
    >
      <Github class="h-4 w-4" />
      {$_('help.contact_cta_github')}
    </Button>
  </div>
</section>
