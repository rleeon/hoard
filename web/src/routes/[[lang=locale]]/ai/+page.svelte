<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import AgentsStrip from '$lib/components/AgentsStrip.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { localeHref } from '$lib/i18n/href';
  import { Copy, Check, FileText, RotateCcw, Eye, RefreshCw } from 'lucide-svelte';

  // Word for word what `hoard agents` prints. It stays in English in every
  // locale: the agent reads it fine, and a translated copy would drift from
  // the one the binary hands out.
  const PROMPT =
    'Run `hoard agents --skill` and save the output as a skill for yourself, in whichever directory you load skills from (for example ~/.claude/skills/hoard/SKILL.md). Then read it.';
  const SKILL_URL =
    'https://github.com/rleeon/hoard/blob/main/crates/hoard-cli/src/agents/SKILL.md';

  // Each claim here is a rule in that SKILL.md; reword one only if the skill
  // still says it.
  const safety = [
    { icon: RotateCcw, title: 'ai.safe1_title', body: 'ai.safe1_body' },
    { icon: Eye, title: 'ai.safe2_title', body: 'ai.safe2_body' },
    { icon: RefreshCw, title: 'ai.safe3_title', body: 'ai.safe3_body' }
  ];

  let copied = $state(false);
  async function copyPrompt() {
    try {
      await navigator.clipboard.writeText(PROMPT);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      /* clipboard blocked, the prompt is still there to select by hand */
    }
  }
</script>

<Seo path="/ai" key="ai" />

<!-- The hero is the home's, piece for piece. -->
<section class="relative">
  <div class="mx-auto max-w-6xl px-4 pb-12 pt-16 sm:px-6 sm:pb-14 sm:pt-24 2xl:pt-8">
    <div class="flex flex-col items-center text-center">
      <h1
        class="max-w-4xl text-balance text-[2.4rem] font-semibold leading-[1.05] text-ink sm:text-6xl lg:text-[4.4rem] animate-fade-up"
        style="animation-delay:0.05s"
      >
        {$_('ai.title_1')}<br />
        <span class="text-accent">{$_('ai.title_2')}</span>
      </h1>

      <p
        class="mt-6 max-w-2xl text-pretty text-[1.05rem] leading-relaxed text-ink-soft sm:text-lg animate-fade-up"
        style="animation-delay:0.12s"
      >
        {$_('ai.subtitle')}
      </p>

      <div
        class="mt-9 grid w-full max-w-lg grid-cols-1 gap-4 sm:grid-cols-2 animate-fade-up"
        style="animation-delay:0.2s"
      >
        <Button href={$localeHref('/cli')} size="xl" variant="primary" full>
          {$_('ai.cta_cli')}
        </Button>
        <Button href={SKILL_URL} target="_blank" size="xl" variant="mono" full>
          <FileText data-anim="pop" class="h-4 w-4" />
          {$_('ai.cta_skill')}
        </Button>
      </div>

      <p
        class="mt-4 font-mono text-xs tracking-wide text-ink-faint animate-fade-up"
        style="animation-delay:0.26s"
      >
        {$_('ai.works_any')}
      </p>
    </div>
  </div>
</section>

<AgentsStrip class="border-y border-line" />

<section class="border-t border-line">
  <div class="mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal max-w-2xl" use:reveal>
      <h2 class="text-balance text-3xl font-semibold text-ink sm:text-4xl">
        {$_('ai.setup_title')}
      </h2>
      <p class="mt-3 text-pretty text-ink-soft">{$_('ai.setup_subtitle')}</p>
    </div>

    <div class="mt-12 space-y-5">
      <article
        class="reveal flex flex-col gap-8 rounded-3xl border border-white/[0.08] bg-black p-8 sm:p-10 lg:flex-row lg:items-center lg:justify-between"
        use:reveal
      >
        <div class="max-w-xl">
          <span class="font-mono text-xs tracking-[0.16em] text-ink-faint">01</span>
          <h3 class="mt-3 text-2xl font-semibold text-ink sm:text-3xl">{$_('ai.s1_title')}</h3>
          <p class="mt-3 text-pretty leading-relaxed text-ink-soft">{$_('ai.s1_body')}</p>
        </div>
        <div class="w-full flex-none lg:w-80">
          <Button href={$localeHref('/cli')} variant="primary" size="xl" full>
            {$_('ai.cta_cli')}
          </Button>
        </div>
      </article>

      <article
        class="reveal rounded-3xl border border-white/[0.08] bg-black p-8 sm:p-10"
        use:reveal={{ delay: 80 }}
      >
        <span class="font-mono text-xs tracking-[0.16em] text-ink-faint">02</span>
        <h3 class="mt-3 text-2xl font-semibold text-ink sm:text-3xl">{$_('ai.s2_title')}</h3>
        <p class="mt-3 max-w-xl text-pretty leading-relaxed text-ink-soft">{$_('ai.s2_body')}</p>
        <div class="mt-6 flex items-stretch gap-2">
          <code
            class="flex-1 whitespace-pre-wrap break-words rounded-2xl border border-white/[0.08] bg-black px-5 py-4 font-mono text-[13px] leading-relaxed text-white/90"
            >{PROMPT}</code
          >
          <button
            onclick={copyPrompt}
            class="glow pop-self anim-host grid w-12 flex-none place-items-center rounded-2xl border border-white/15 bg-black text-ink-soft ring-focus transition-colors hover:border-white/40 hover:text-ink"
            aria-label={$_('cli.copy')}
          >
            {#if copied}
              <Check data-anim="pop" class="h-4 w-4 text-ink" />
            {:else}
              <Copy data-anim="pop" class="h-4 w-4" />
            {/if}
          </button>
        </div>
      </article>

      <article
        class="reveal rounded-3xl border border-white/[0.08] bg-black p-8 sm:p-10"
        use:reveal={{ delay: 160 }}
      >
        <span class="font-mono text-xs tracking-[0.16em] text-ink-faint">03</span>
        <h3 class="mt-3 text-2xl font-semibold text-ink sm:text-3xl">{$_('ai.s3_title')}</h3>
        <p class="mt-3 max-w-2xl text-pretty leading-relaxed text-ink-soft">{$_('ai.s3_body')}</p>
      </article>
    </div>
  </div>
</section>

<section class="border-t border-line">
  <div class="mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-24">
    <h2
      class="reveal max-w-2xl text-balance text-3xl font-semibold text-ink sm:text-4xl"
      use:reveal
    >
      {$_('ai.safe_title')}
    </h2>
    <div class="mt-12 grid gap-5 md:grid-cols-3">
      {#each safety as s, i (s.title)}
        <article
          class="reveal rounded-3xl border border-white/[0.08] bg-black p-7"
          use:reveal={{ delay: i * 80 }}
        >
          <div
            class="grid h-10 w-10 place-items-center rounded-xl border border-white/15 text-ink"
          >
            <s.icon class="h-5 w-5" />
          </div>
          <h3 class="mt-5 text-lg font-semibold text-ink">{$_(s.title)}</h3>
          <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_(s.body)}</p>
        </article>
      {/each}
    </div>
  </div>
</section>
