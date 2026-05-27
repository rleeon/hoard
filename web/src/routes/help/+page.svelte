<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
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

<svelte:head>
  <title>{`${$_('help.title')} — Hoard`}</title>
  <link rel="canonical" href="https://hoard.services/help" />
</svelte:head>

<section class="relative mx-auto max-w-3xl px-4 py-20 sm:px-6 sm:py-24">
  <div
    class="pointer-events-none absolute -top-24 left-1/2 -z-10 h-72 w-[34rem] -translate-x-1/2 rounded-full bg-emerald-500/[0.08] blur-3xl"
  ></div>

  <div class="text-center">
    <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80">
      Support
    </div>
    <h1 class="mt-3 text-balance text-4xl font-extrabold tracking-tight text-white sm:text-5xl">
      {$_('help.title')}
    </h1>
    <p class="mx-auto mt-4 max-w-xl text-pretty text-lg text-zinc-400">
      {$_('help.subtitle')}
    </p>
    <div class="mt-6 flex justify-center"><StatusDot /></div>
  </div>

  <!-- Search -->
  <div class="reveal relative mt-12" use:reveal>
    <Search
      class="pointer-events-none absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-500"
    />
    <input
      type="search"
      bind:value={query}
      placeholder={$_('help.search_placeholder')}
      class="w-full rounded-xl border border-white/[0.06] bg-white/[0.025] py-3 pl-11 pr-4 text-sm text-zinc-100 placeholder:text-zinc-500 ring-focus transition-colors focus:border-emerald-500/50 focus:bg-white/[0.04] focus:outline-none"
      aria-label={$_('help.search_placeholder')}
    />
  </div>

  {#if grouped.length === 0}
    <p class="reveal mt-10 text-center text-sm text-zinc-400" use:reveal>
      {$_('help.no_results')}
    </p>
  {:else}
    {#each grouped as g (g.group)}
      <h2
        class="mt-12 text-[11px] font-semibold uppercase tracking-[0.18em] text-emerald-400/80 first:mt-10"
      >
        {$_(`help.group_${g.group}`)}
      </h2>
      <div class="mt-4 space-y-2">
        {#each g.items as f, i (f.q)}
          <div
            class="reveal card-edge overflow-hidden rounded-xl border border-white/[0.06] bg-white/[0.02] transition-colors hover:border-white/15"
            use:reveal={{ delay: i * 40 }}
          >
            <button
              class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left ring-focus transition-colors hover:bg-white/[0.04]"
              onclick={() => (openKey = openKey === f.q ? null : f.q)}
              aria-expanded={openKey === f.q}
              aria-controls={`faq-${f.q}`}
            >
              <span class="font-medium text-zinc-100">{$_(f.q)}</span>
              <ChevronDown
                class="h-4 w-4 flex-none text-zinc-400 transition-transform duration-300 {openKey ===
                f.q
                  ? 'rotate-180 text-emerald-400'
                  : ''}"
              />
            </button>
            {#if openKey === f.q}
              <div
                id={`faq-${f.q}`}
                class="border-t border-white/[0.06] px-5 py-4 text-sm leading-relaxed text-zinc-300"
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

  <h2 class="mt-16 text-xl font-semibold text-white">{$_('help.contact_title')}</h2>
  <p class="mt-2 text-sm text-zinc-400">{$_('help.contact_body')}</p>

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
