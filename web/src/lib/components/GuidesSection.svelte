<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { localeHref } from '$lib/i18n/href';
  import { getGuide, type GuideMeta } from '$lib/guides';
  import { DEFAULT_LOCALE, isLocale, type Locale } from '$lib/i18n/locales';
  import { ArrowRight } from 'lucide-svelte';

  // The "Guides" section closing the home. Three guides picked by hand, the
  // ones Search Console shows people already land on, so the home passes its
  // weight to them; titles and blurbs come from each guide's own frontmatter,
  // which keeps this in step with the guide in every language.
  const PICKS = ['sync-game-saves-across-pcs', 'steam-cloud-alternative', 'back-up-emulator-saves'];

  const active = $derived<Locale>(isLocale($locale) ? ($locale as Locale) : DEFAULT_LOCALE);
  const guides = $derived(
    PICKS.map((s) => getGuide(s, active)).filter((g): g is NonNullable<typeof g> => g !== null) satisfies GuideMeta[]
  );
</script>

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
    <div class="reveal flex flex-wrap items-end justify-between gap-4" use:reveal>
      <div>
        <p class="kicker">{$_('homeguides.kicker')}</p>
        <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
          {$_('homeguides.title')}
        </h2>
      </div>
      <a
        href={$localeHref('/guides')}
        class="link-underline inline-flex items-center gap-1.5 text-sm font-medium text-accent ring-focus"
      >
        {$_('guides.back')}
        <ArrowRight class="h-4 w-4" />
      </a>
    </div>

    <ul class="mt-10 grid gap-4 md:grid-cols-3">
      {#each guides as g, i (g.slug)}
        <li class="reveal" use:reveal={{ delay: i * 80 }}>
          <a
            href={$localeHref(`/guides/${g.slug}`)}
            class="group flex h-full flex-col rounded-xl border border-line bg-surface p-5 ring-focus transition-colors hover:border-line-strong"
          >
            <p class="font-display text-lg font-semibold text-ink">{g.title}</p>
            <p class="mt-2 flex-1 text-sm leading-relaxed text-ink-soft">{g.description}</p>
            <ArrowRight
              class="mt-4 h-4 w-4 text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-accent"
            />
          </a>
        </li>
      {/each}
    </ul>
  </div>
</section>
