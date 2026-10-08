<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import DiscordIcon from '$lib/components/DiscordIcon.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { localeHref } from '$lib/i18n/href';
  import { PLANS, formatBytes, formatPlanQuota } from '$lib/plans';
  import { release, CHANGELOG_URL } from '$lib/version';
  import { Download, Copy, Check, Github, Mail, ArrowDown } from 'lucide-svelte';

  const press = __HOARD_PRESS__;
  const main = press.assets.find((a) => a.kind === 'main');
  const others = press.assets.filter((a) => a !== main);
  const zipBytes = press.assets.reduce((n, a) => n + a.bytes, 0);
  const latest = press.release;
  const CONTACT = 'support@hoard.services';

  const fileName = (zipName: string) => zipName.split('/').pop() ?? zipName;
  const isVideo = (file: string) => /\.(mp4|webm)$/i.test(file);
  const dims = (a: { width: number | null; height: number | null }) =>
    a.width && a.height ? `${a.width} × ${a.height}` : '';

  const abouts = ['line', 'short', 'long'] as const;

  const facts = $derived([
    ['press.fact.what', $_('press.fact.what_value')],
    ['press.fact.platforms', $_('press.fact.platforms_value')],
    [
      'press.fact.price',
      $_('press.fact.price_value', {
        values: { price: formatPrice(PLANS.pro.priceMonthly), storage: formatPlanQuota('pro') }
      })
    ],
    ['press.fact.selfhost', $_('press.fact.selfhost_value')],
    ['press.fact.cloud', $_('press.fact.cloud_value')],
    ['press.fact.license', $_('press.fact.license_value')],
    ['press.fact.built', $_('press.fact.built_value')],
    ['press.fact.languages', $_('press.fact.languages_value')],
    ['press.fact.developer', $_('press.fact.developer_value')],
    [
      'press.fact.version',
      $_('press.fact.version_value', { values: { v: $release.v, date: formatDate($release.date) } })
    ]
  ]);

  function formatPrice(eur: number): string {
    try {
      return new Intl.NumberFormat($locale ?? 'en', { style: 'currency', currency: 'EUR' }).format(eur);
    } catch {
      return `€${eur}`;
    }
  }

  function formatDate(iso: string): string {
    if (!iso) return '';
    try {
      return new Intl.DateTimeFormat($locale ?? 'en', { dateStyle: 'long', timeZone: 'UTC' }).format(
        new Date(`${iso}T12:00:00Z`)
      );
    } catch {
      return iso;
    }
  }

  // CHANGELOG headings are English; the known ones get a translated label and
  // anything else is shown as written.
  const SECTION_KEYS = new Set(['added', 'changed', 'removed', 'fixed', 'security', 'deprecated']);
  const sectionLabel = (name: string) =>
    SECTION_KEYS.has(name.toLowerCase()) ? $_(`press.section.${name.toLowerCase()}`) : name;

  // Headlines keep their `code` spans; split on backticks rather than render
  // markdown, so nothing from the CHANGELOG ever goes through {@html}.
  const segments = (text: string) =>
    text.split('`').map((part, i) => ({ part, code: i % 2 === 1 }));

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = id;
      setTimeout(() => {
        if (copied === id) copied = null;
      }, 2000);
    } catch {
      /* clipboard blocked, the text is still there to select by hand */
    }
  }
</script>

<Seo path="/press" key="press" />

<section class="mx-auto max-w-5xl px-4 pb-16 pt-16 sm:px-6 sm:pb-24 sm:pt-24 2xl:pt-12">
  <div class="mx-auto max-w-2xl text-center">
    <p class="kicker">{$_('press.kicker')}</p>
    <h1 class="mt-3 text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('press.title')}
    </h1>
    <p class="mt-4 text-pretty leading-relaxed text-ink-soft">{$_('press.subtitle')}</p>
    <div class="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
      <!-- A plain anchor with `download`: the router leaves it alone and the
           browser saves the file instead of trying to navigate to it. -->
      <a
        href="/hoard-press-kit.zip"
        download
        class="glow pop-self anim-host inline-flex h-12 items-center justify-center gap-2 rounded-lg bg-accent px-6 text-base font-medium text-pine ring-focus transition-colors duration-200 hover:bg-emerald-300 active:brightness-95"
      >
        <Download data-anim="pop" class="h-4 w-4" aria-hidden="true" />
        {$_('press.zip_cta')}
        <span class="font-mono text-xs text-pine/70">.zip · {formatBytes(zipBytes)}</span>
      </a>
      {#if latest}
        <Button href="#latest-release" variant="secondary" size="lg">
          <ArrowDown data-anim="pop" class="h-4 w-4" aria-hidden="true" />
          {$_('press.release_cta', { values: { v: latest.version } })}
        </Button>
      {/if}
    </div>
  </div>

  {#if main}
    <figure class="reveal mx-auto mt-14 max-w-4xl" use:reveal>
      <a href="/{main.file}" download={fileName(main.zipName)} class="block rounded-2xl ring-focus">
        <img
          src="/{main.file}"
          alt={$_('hero.screenshot_alt')}
          width={main.width ?? undefined}
          height={main.height ?? undefined}
          decoding="async"
          class="block w-full rounded-2xl border border-line-strong shadow-[0_40px_90px_-40px_rgba(0,0,0,0.9)]"
        />
      </a>
      <figcaption class="mt-3 flex flex-wrap items-center justify-center gap-x-3 gap-y-1 text-xs text-ink-faint">
        <span class="text-ink-soft">{$_(main.label)}</span>
        <span class="font-mono">{dims(main)} · {formatBytes(main.bytes)}</span>
        <a
          href="/{main.file}"
          download={fileName(main.zipName)}
          class="link-underline text-accent ring-focus hover:text-emerald-300"
        >
          {$_('press.download')}
        </a>
      </figcaption>
    </figure>
  {/if}

  <!-- Ready-to-paste descriptions -->
  <div class="reveal mt-20" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('press.about_title')}</h2>
    <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">{$_('press.about_body')}</p>
    <div class="mx-auto mt-8 max-w-3xl space-y-4">
      {#each abouts as id (id)}
        <article class="rounded-2xl border border-line bg-surface p-6">
          <div class="flex items-center justify-between gap-3">
            <h3 class="font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">
              {$_(`press.about.${id}_label`)}
            </h3>
            <button
              onclick={() => copy($_(`press.about.${id}`), id)}
              class="glow pop-self anim-host inline-flex h-8 items-center gap-1.5 rounded-lg border border-line bg-bg px-3 text-xs text-ink-soft ring-focus transition-colors hover:border-accent hover:text-accent"
            >
              {#if copied === id}
                <Check data-anim="pop" class="h-3.5 w-3.5 text-accent" aria-hidden="true" />
                {$_('press.copied')}
              {:else}
                <Copy data-anim="pop" class="h-3.5 w-3.5" aria-hidden="true" />
                {$_('press.copy')}
              {/if}
            </button>
          </div>
          <p class="mt-3 text-pretty text-sm leading-relaxed text-ink">{$_(`press.about.${id}`)}</p>
        </article>
      {/each}
    </div>
  </div>

  <!-- Fact sheet -->
  <div class="reveal mt-20" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('press.facts_title')}</h2>
    <dl class="mx-auto mt-8 max-w-3xl divide-y divide-line overflow-hidden rounded-2xl border border-line bg-surface">
      {#each facts as [label, value] (label)}
        <div class="flex flex-col gap-1 p-4 sm:flex-row sm:gap-6">
          <dt class="flex-none text-sm font-medium text-ink sm:w-44">{$_(label)}</dt>
          <dd class="text-sm leading-relaxed text-ink-soft">{value}</dd>
        </div>
      {/each}
      <div class="flex flex-col gap-1 p-4 sm:flex-row sm:gap-6">
        <dt class="flex-none text-sm font-medium text-ink sm:w-44">{$_('press.fact.links')}</dt>
        <dd class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
          <a class="link-underline text-accent ring-focus hover:text-emerald-300" href={$localeHref('/')}>hoard.services</a>
          <a class="link-underline text-accent ring-focus hover:text-emerald-300" href={$localeHref('/download')}>
            {$_('nav.download')}
          </a>
          <a
            class="link-underline text-accent ring-focus hover:text-emerald-300"
            href="https://github.com/rleeon/hoard"
            target="_blank"
            rel="noopener noreferrer">GitHub</a
          >
        </dd>
      </div>
    </dl>
  </div>

  <!-- Every other file in the kit -->
  {#if others.length}
    <div class="reveal mt-20" use:reveal>
      <h2 class="text-center text-2xl font-semibold text-ink">{$_('press.media_title')}</h2>
      <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">{$_('press.media_body')}</p>
      <div class="mt-8 grid gap-4 sm:grid-cols-2">
        {#each others as a (a.file)}
          <figure
            class="flex flex-col overflow-hidden rounded-2xl border border-line bg-surface {a.kind === 'diagram'
              ? 'sm:col-span-2'
              : ''}"
          >
            <div class="grid flex-1 place-items-center bg-bg p-6">
              {#if isVideo(a.file)}
                <video
                  src="/{a.file}"
                  controls
                  muted
                  playsinline
                  preload="metadata"
                  class="max-h-80 w-auto max-w-full rounded-lg"
                ></video>
              {:else}
                <img
                  src="/{a.file}"
                  alt={$_(a.label)}
                  width={a.width ?? undefined}
                  height={a.height ?? undefined}
                  loading="lazy"
                  decoding="async"
                  class="h-auto max-h-80 w-auto max-w-full rounded-lg {a.kind === 'icon'
                    ? 'max-h-32'
                    : 'border border-line'}"
                />
              {/if}
            </div>
            <figcaption class="flex flex-wrap items-center justify-between gap-2 border-t border-line p-4">
              <span>
                <span class="block text-sm font-medium text-ink">{$_(a.label)}</span>
                <span class="mt-0.5 block font-mono text-xs text-ink-faint">
                  {[dims(a), formatBytes(a.bytes)].filter(Boolean).join(' · ')}
                </span>
              </span>
              <a
                href="/{a.file}"
                download={fileName(a.zipName)}
                class="glow inline-flex h-8 items-center gap-1.5 rounded-lg border border-line bg-bg px-3 text-xs text-ink-soft ring-focus transition-colors hover:border-accent hover:text-accent"
              >
                <Download class="h-3.5 w-3.5" aria-hidden="true" />
                {$_('press.download')}
              </a>
            </figcaption>
          </figure>
        {/each}
      </div>
    </div>
  {/if}

  <!-- What changed last: read from the CHANGELOG at build time, so a release
       commit updates this section on its own. -->
  {#if latest}
    <div id="latest-release" class="reveal mt-20 scroll-mt-24" use:reveal>
      <h2 class="text-center text-2xl font-semibold text-ink">{$_('press.release_title')}</h2>
      <p class="mt-2 text-center font-mono text-xs text-ink-faint">
        v{latest.version} · {formatDate(latest.date)}
      </p>
      <div class="mx-auto mt-8 max-w-3xl rounded-2xl border border-line bg-surface p-6">
        {#if $locale && $locale !== 'en'}
          <p class="mb-4 text-xs text-ink-faint">{$_('press.release_lang_note')}</p>
        {/if}
        {#if latest.summary}
          <p class="text-pretty leading-relaxed text-ink" lang="en">{latest.summary}</p>
        {/if}
        {#each latest.sections as section (section.name)}
          {#if section.name.toLowerCase() === 'fixed'}
            <details class="group mt-6">
              <summary class="cursor-pointer text-sm font-semibold text-ink ring-focus">
                {sectionLabel(section.name)} ({section.items.length})
              </summary>
              <ul class="mt-3 space-y-1.5 text-sm leading-relaxed text-ink-soft" lang="en">
                {#each section.items as item, i (i)}
                  <li class="flex gap-2">
                    <span class="text-accent" aria-hidden="true">·</span>
                    <span
                      >{#each segments(item) as s, j (j)}{#if s.code}<code
                            class="rounded bg-pine px-1 font-mono text-[0.85em] text-white/90">{s.part}</code
                          >{:else}{s.part}{/if}{/each}</span
                    >
                  </li>
                {/each}
              </ul>
            </details>
          {:else}
            <h3 class="mt-6 text-sm font-semibold text-ink">{sectionLabel(section.name)}</h3>
            <ul class="mt-3 space-y-1.5 text-sm leading-relaxed text-ink-soft" lang="en">
              {#each section.items as item, i (i)}
                <li class="flex gap-2">
                  <span class="text-accent" aria-hidden="true">·</span>
                  <span
                    >{#each segments(item) as s, j (j)}{#if s.code}<code
                          class="rounded bg-pine px-1 font-mono text-[0.85em] text-white/90">{s.part}</code
                        >{:else}{s.part}{/if}{/each}</span
                  >
                </li>
              {/each}
            </ul>
          {/if}
        {/each}
        <p class="mt-6 text-sm">
          <a
            href={CHANGELOG_URL}
            target="_blank"
            rel="noopener noreferrer"
            class="link-underline text-accent ring-focus hover:text-emerald-300"
          >
            {$_('press.release_full')}
          </a>
        </p>
      </div>
    </div>
  {/if}

  <!-- Terms of use and who to write to -->
  <div class="reveal mt-20 grid gap-4 sm:grid-cols-2" use:reveal>
    <article class="rounded-2xl border border-line bg-surface p-6">
      <h2 class="text-lg font-semibold text-ink">{$_('press.usage_title')}</h2>
      <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_('press.usage_body')}</p>
    </article>
    <article class="rounded-2xl border border-line bg-surface p-6">
      <h2 class="text-lg font-semibold text-ink">{$_('press.contact_title')}</h2>
      <p class="mt-2 text-sm leading-relaxed text-ink-soft">{$_('press.contact_body')}</p>
      <div class="mt-4 flex flex-wrap gap-3">
        <Button href="mailto:{CONTACT}" variant="outline" size="md">
          <Mail data-anim="pop" class="h-4 w-4" aria-hidden="true" />
          {CONTACT}
        </Button>
        <Button href="https://discord.gg/BYpXT8v4rh" target="_blank" variant="secondary" size="md">
          <DiscordIcon data-anim="pop" class="h-4 w-4" />
          Discord
        </Button>
        <Button href="https://github.com/rleeon/hoard" target="_blank" variant="secondary" size="md">
          <Github data-anim="pop" class="h-4 w-4" aria-hidden="true" />
          GitHub
        </Button>
      </div>
    </article>
  </div>
</section>
