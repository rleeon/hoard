<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import StatusDot from '$lib/components/StatusDot.svelte';
  import { ChevronDown, Mail, Github } from 'lucide-svelte';

  const faqs = [
    { q: 'help.faq_q1', a: 'help.faq_a1' },
    { q: 'help.faq_q2', a: 'help.faq_a2' },
    { q: 'help.faq_q3', a: 'help.faq_a3' },
    { q: 'help.faq_q4', a: 'help.faq_a4' },
    { q: 'help.faq_q5', a: 'help.faq_a5' }
  ];

  let open = $state<number | null>(0);
</script>

<section class="mx-auto max-w-3xl px-4 py-20 sm:px-6">
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
      <div class="overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/40">
        <button
          class="flex w-full items-center justify-between gap-4 px-5 py-4 text-left transition-colors hover:bg-zinc-900/80"
          onclick={() => (open = open === i ? null : i)}
          aria-expanded={open === i}
        >
          <span class="font-medium text-zinc-100">{$_(f.q)}</span>
          <ChevronDown
            class="h-4 w-4 flex-none text-zinc-400 transition-transform {open === i ? 'rotate-180' : ''}"
          />
        </button>
        {#if open === i}
          <div class="border-t border-zinc-800 px-5 py-4 text-sm leading-relaxed text-zinc-400">
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
