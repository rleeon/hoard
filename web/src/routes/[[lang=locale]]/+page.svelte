<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import HowItWorks from '$lib/components/HowItWorks.svelte';
  import WhatsInTheBox from '$lib/components/WhatsInTheBox.svelte';
  import GamesSection from '$lib/components/GamesSection.svelte';
  import SelfHostSection from '$lib/components/SelfHostSection.svelte';
  import SupportSection from '$lib/components/SupportSection.svelte';
  import SyncDiagram from '$lib/components/SyncDiagram.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { marquee } from '$lib/actions/marquee';
  import { tilt } from '$lib/actions/tilt';
  import { localeHref } from '$lib/i18n/href';
  import { SITE_URL } from '$lib/i18n/locales';
  import { PLANS } from '$lib/plans';
  import { version } from '$lib/version';
  import { Check, Star } from 'lucide-svelte';


  // Twelve facts, all of them checkable in the repo, a marquee with four
  // items announces its own loop every few seconds.
  const facts = [
    'sha',
    'agpl',
    'privacy',
    'os',
    'rust',
    'proton',
    'saves',
    'history',
    'export',
    'noads',
    'solo'
  ];
  // Facts that are a whole sentence on their own, with no quiet half. Kept as
  // an explicit set: svelte-i18n echoes the key name back for an empty string,
  // so "does this have a label?" cannot be asked of the translation itself.
  const SENTENCE_FACTS = new Set(['privacy']);
  // Three copies, not two: the wrap needs one copy to be at least as wide as
  // the viewport, and one copy of this list is ~3.7k px, short of a 4K panel
  // at 100%. The action derives the wrap point from the copy count.
  const FACT_COPIES = [0, 1, 2];

  // Structured data for rich results: the product + its two pricing tiers and
  // the operating organization. Description tracks the page locale.
  // The platform line is centred on its middle item (macOS today), so the list
  // is split around it instead of being centred as one string.
  const platforms = $derived.by(() => {
    const parts = $_('hero.subnote_platforms', { values: { v: $version } }).split(' · ');
    const i = Math.floor(parts.length / 2);
    const left = parts.slice(0, i).join(' · ');
    const right = parts.slice(i + 1).join(' · ');
    return {
      left: left ? `${left} ·` : '',
      mid: parts[i] ?? '',
      right: right ? `· ${right}` : ''
    };
  });

  const jsonLd = $derived(
    `<script type="application/ld+json">${JSON.stringify({
      '@context': 'https://schema.org',
      '@graph': [
        {
          '@type': 'Organization',
          '@id': `${SITE_URL}/#org`,
          name: 'Hoard',
          url: SITE_URL,
          logo: `${SITE_URL}/icon.png`,
          // Ties the site to the repo as one entity instead of two loose names.
          sameAs: ['https://github.com/rleeon/hoard'],
          founder: { '@type': 'Person', name: 'Raimundo León Oliva' }
        },
        {
          '@type': 'SoftwareApplication',
          name: 'Hoard',
          applicationCategory: 'UtilitiesApplication',
          operatingSystem: 'Windows, macOS, Linux',
          url: SITE_URL,
          // The long form, not the meta description: nothing truncates this one,
          // and it is where "Hoard the software" and "Hoard Cloud the service"
          // get told apart in so many words.
          description: $_('seo.home.desc_long'),
          license: 'https://www.gnu.org/licenses/agpl-3.0.html',
          isAccessibleForFree: true,
          codeRepository: 'https://github.com/rleeon/hoard',
          publisher: { '@id': `${SITE_URL}/#org` },
          offers: [
            {
              '@type': 'Offer',
              name: $_('plan.free'),
              price: '0',
              priceCurrency: 'EUR'
            },
            {
              '@type': 'Offer',
              name: $_('plan.pro'),
              price: PLANS.pro.priceMonthly.toFixed(2),
              priceCurrency: 'EUR'
            }
          ]
        }
      ]
    })}<\/script>`
  );
</script>

<Seo path="/" key="home" />
<svelte:head>
  <link rel="preload" as="image" href="/WEB.webp" fetchpriority="high" />
  <link rel="preload" as="image" href="/CLI.webp" fetchpriority="high" />
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html jsonLd}
</svelte:head>

<!-- ───────── HERO ───────── -->
<section class="relative">
  <div class="mx-auto max-w-6xl px-4 pt-16 sm:px-6 sm:pt-24 2xl:pt-8">
    <div class="flex flex-col items-center text-center">
      <h1
        class="max-w-4xl text-balance text-[2.4rem] font-semibold leading-[1.05] text-ink sm:text-6xl lg:text-[4.4rem] animate-fade-up"
        style="animation-delay:0.05s"
      >
        {$_('hero.title_1')}<br />
        <span class="text-accent">{$_('hero.title_2')}</span>
      </h1>

      <p
        class="mt-6 max-w-2xl text-pretty text-[1.05rem] leading-relaxed text-ink-soft sm:text-lg animate-fade-up"
        style="animation-delay:0.12s"
      >
        {$_('hero.subtitle')}
      </p>

      <div
        class="mt-9 grid w-full max-w-lg grid-cols-1 gap-4 sm:grid-cols-2 animate-fade-up"
        style="animation-delay:0.2s"
      >
        <Button href={$localeHref('/download')} size="xl" variant="primary" full>
          {$_('hero.cta_start')}
        </Button>
        <Button
          href="https://github.com/rleeon/hoard"
          target="_blank"
          size="xl"
          variant="star"
          full
        >
          <Star class="h-4 w-4 fill-current" />
          {$_('hero.cta_star')}
        </Button>
      </div>

      <div
        class="mt-4 w-full text-center font-mono text-xs tracking-wide text-ink-faint animate-fade-up sm:grid sm:grid-cols-[1fr_auto_1fr] sm:items-baseline sm:gap-x-2"
        style="animation-delay:0.26s"
      >
        <span class="sm:text-right">{$_('hero.subnote_selfhost')}</span>
        <span aria-hidden="true">&amp;</span>
        <span class="sm:text-left">{$_('hero.subnote_cloud')}</span>
      </div>
      <div
        class="mt-1 w-full text-center font-mono text-xs tracking-wide text-ink-faint animate-fade-up sm:grid sm:grid-cols-[1fr_auto_1fr] sm:items-baseline sm:gap-x-2"
        style="animation-delay:0.26s"
      >
        <span class="sm:text-right">{platforms.left}</span>
        <span>{platforms.mid}</span>
        <span class="sm:text-left">{platforms.right}</span>
      </div>
    </div>
  </div>
</section>

<!-- ───────── SCREENSHOT ───────── -->
<section class="relative">
  <div class="relative mx-auto max-w-[84rem] px-4 pt-14 sm:px-6 2xl:pb-7 2xl:pt-4">
    <div class="flex flex-col items-center gap-8 lg:flex-row lg:items-center lg:justify-center lg:gap-7">
      <figure class="tilt relative w-full lg:w-auto" use:tilt>
        <img
          src="/WEB.webp"
          alt={$_('hero.screenshot_alt')}
          width="1321"
          height="913"
          fetchpriority="high"
          decoding="async"
          class="block w-full rounded-2xl border border-line-strong shadow-[0_40px_90px_-40px_rgba(0,0,0,0.9)] lg:h-[22rem] lg:w-auto xl:h-[28rem] 2xl:h-[30rem]"
        />
      </figure>
      <figure
        class="tilt relative w-full max-w-2xl lg:w-auto lg:max-w-none"
        use:tilt
      >
        <img
          src="/CLI.webp"
          alt={$_('hero.screenshot_cli_alt')}
          width="664"
          height="630"
          fetchpriority="high"
          decoding="async"
          class="block w-full rounded-2xl border border-line-strong shadow-[0_40px_90px_-40px_rgba(0,0,0,0.9)] lg:h-[22rem] lg:w-auto xl:h-[28rem] 2xl:h-[30rem]"
        />
      </figure>
    </div>
  </div>
</section>

<!-- ───────── FACT STRIP ───────── -->
<section class="border-y border-line" aria-label={$_('facts.aria')}>
  <!-- The second track is the same list again: it fills the right edge while
       the first scrolls out, and it is aria-hidden so nothing reads twice. -->
  <div class="marquee">
    <div class="marquee-inner" use:marquee={{ speed: 52 }}>
      {#each FACT_COPIES as copy (copy)}
        <dl class="flex shrink-0 items-center" aria-hidden={copy > 0 || undefined}>
          {#each facts as f (f)}
            <div class="flex shrink-0 items-center gap-2.5 whitespace-nowrap px-7 py-5 2xl:py-4">
              <dt class="font-mono text-sm font-medium tracking-tight text-ink">
                {$_(`facts.${f}.value`)}
              </dt>
              {#if !SENTENCE_FACTS.has(f)}
                <dd class="text-xs text-ink-faint">{$_(`facts.${f}.label`)}</dd>
              {/if}
            </div>
            <!-- The dot separates two items, so it sits between them rather than
                 inside one: as a child it only had the item's own gap on its left
                 and both paddings on its right, which read as off-centre. -->
            <span class="h-1 w-1 shrink-0 rounded-full bg-accent/40" aria-hidden="true"></span>
          {/each}
        </dl>
      {/each}
    </div>
  </div>
</section>

<!-- ───────── SYNC ───────── -->
<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-14 sm:px-6">
    <div class="grid items-center gap-12 lg:grid-cols-2">
      <div class="reveal" use:reveal>
        <p class="kicker">{$_('sync.kicker')}</p>
        <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
          {$_('sync.title')}
        </h2>
        <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
          {$_('sync.body')}
        </p>
        <ul class="mt-6 space-y-3 text-sm">
          {#each ['p1', 'p2', 'p3'] as p (p)}
            <li class="flex items-start gap-2.5 text-ink-soft">
              <Check class="mt-0.5 h-4 w-4 flex-none text-accent" />
              {$_(`sync.${p}`)}
            </li>
          {/each}
        </ul>
      </div>

      <div class="reveal" use:reveal={{ delay: 100 }}>
        <div
          class="tilt rounded-2xl border border-line bg-surface p-6 sm:p-8"
          use:tilt
        >
          <SyncDiagram />
        </div>
      </div>
    </div>
  </div>
</section>

<!-- ───────── HOW IT WORKS ───────── -->
<HowItWorks />

<!-- ───────── WHAT'S IN THE BOX ───────── -->
<WhatsInTheBox />

<!-- ───────── SUPPORTED GAMES ───────── -->
<GamesSection />

<!-- ───────── SELF-HOST ───────── -->
<SelfHostSection />

<!-- ───────── SUPPORT HOARD ───────── -->
<SupportSection />
