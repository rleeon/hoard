<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { noticeKey, NOTICE_CTA, type NoticeSlug } from '$lib/notices';

  let { data } = $props();

  const slug = $derived(data.slug as NoticeSlug);
  const base = $derived(noticeKey(slug));
  const cta = $derived(NOTICE_CTA[slug]);

  // Bodies are numbered paragraphs so a translation can be shorter or longer
  // than the English without the page caring. A missing key renders as the key
  // itself in svelte-i18n, so we stop at the first one that is not there
  // rather than printing `notices.x.p3` to a reader.
  const paragraphs = $derived(
    [1, 2, 3, 4]
      .map((n) => `${base}.p${n}`)
      .map((k) => ({ k, text: $_(k) }))
      .filter((p) => p.text !== p.k)
  );
</script>

<Seo path={`/notices/${slug}`} title={$_(`${base}.title`)} description={$_(`${base}.p1`)} />

<section class="mx-auto max-w-2xl px-4 pb-16 pt-16 sm:px-6 sm:pb-24 sm:pt-24 2xl:pt-12">
  <p class="kicker">{$_('notices.kicker')}</p>

  <h1 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
    {$_(`${base}.title`)}
  </h1>

  <div class="mt-6 space-y-4">
    {#each paragraphs as p, i (p.k)}
      <p class="reveal text-pretty leading-relaxed text-ink-soft" use:reveal={{ delay: i * 40 }}>
        {p.text}
      </p>
    {/each}
  </div>

  <div class="reveal mt-10" use:reveal>
    <Button href={cta.href}>{$_(cta.key)}</Button>
  </div>

  <p class="mt-12 border-t border-line pt-6 text-sm text-ink-faint">
    {$_('notices.footer')}
  </p>
</section>
