<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { ChevronDown, Mail, Github } from 'lucide-svelte';

  const faqs = [
    { q: 'help.faq_q1', a: 'help.faq_a1' },
    { q: 'help.faq_q2', a: 'help.faq_a2' },
    { q: 'help.faq_q3', a: 'help.faq_a3' },
    { q: 'help.faq_q4', a: 'help.faq_a4' },
    { q: 'help.faq_q5', a: 'help.faq_a5' },
    { q: 'help.faq_q6', a: 'help.faq_a6' },
    { q: 'help.faq_q7', a: 'help.faq_a7' }
  ];

  let open = $state<number | null>(0);
</script>

<svelte:head>
  <title>{`${$_('help.title')} — Hoard`}</title>
  <link rel="canonical" href="https://hoard.services/help" />
</svelte:head>

<section class="mx-auto max-w-3xl px-4 py-20 sm:px-6 sm:py-24">
  <div class="text-center">
    <h1 class="text-balance text-4xl font-bold tracking-tight text-white sm:text-5xl">
      {$_('help.title')}
    </h1>
    <p class="mt-4 text-pretty text-lg text-zinc-400">{$_('help.subtitle')}</p>
    <div class="mt-6 flex justify-center"><StatusDot /></div>
  </div>

  <h2 class="mt-16 text-xl font-semibold text-white">{$_('help.faq_title')}</h2>

  <div class="mt-5 space-y-2">
    {#each faqs as f, i (f.q)}
      <div
        class="reveal overflow-hidden rounded-xl border border-white/[0.06] bg-white/[0.02] transition-colors hover:border-white/15"
        use:reveal={{ delay: i * 40 }}
      >
        <button
          class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left ring-focus transition-colors hover:bg-white/[0.04]"
          onclick={() => (open = open === i ? null : i)}
          aria-expanded={open === i}
          aria-controls={`faq-${i}`}
        >
          <span class="font-medium text-zinc-100">{$_(f.q)}</span>
          <ChevronDown
            class="h-4 w-4 flex-none text-zinc-400 transition-transform duration-300 {open === i
              ? 'rotate-180 text-emerald-400'
              : ''}"
          />
        </button>
        {#if open === i}
          <div
            id={`faq-${i}`}
            class="border-t border-white/[0.06] px-5 py-4 text-sm leading-relaxed text-zinc-400"
            transition:slide={{ duration: 220, easing: cubicOut }}
          >
            {$_(f.a)}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <h2 class="mt-16 text-xl font-semibold text-white">{$_('help.contact_title')}</h2>
  <p class="mt-2 text-sm text-zinc-400">{$_('help.contact_body')}</p>

  <div class="mt-5 flex flex-col gap-3 sm:flex-row">
    <Button href="mailto:support@hoard.services" variant="primary">
      <Mail class="h-4 w-4" />
      {$_('help.contact_cta_email')}
    </Button>
    <Button
      href="https://github.com/rleeon/hoard/issues/new"
      target="_blank"
      variant="secondary"
    >
      <Github class="h-4 w-4" />
      {$_('help.contact_cta_github')}
    </Button>
  </div>
</section>
