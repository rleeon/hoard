<script lang="ts">
  import { _ } from 'svelte-i18n';
  import LogoH from './LogoH.svelte';
  import { localeHref } from '$lib/i18n/href';
  import Button from './Button.svelte';
  import { Download } from 'lucide-svelte';
  import { version } from '$lib/version';
  const year = new Date().getFullYear();
</script>

<!-- preload-code="viewport" warms the route chunks of every footer link as the
     footer scrolls into view — code only, so no `load` runs and no locale is
     switched (the layout's preload-data="hover" still decides when data is
     fetched). Cheap here because the footer is always below the fold; the nav
     is not, and "viewport" there would just mean "eager". -->
<footer class="mt-auto border-t border-line" data-sveltekit-preload-code="viewport">
  <div class="mx-auto grid max-w-6xl gap-10 px-4 py-14 sm:px-6 grid-cols-2 md:grid-cols-4">
    <div class="col-span-2 md:col-span-1">
      <div class="flex items-center gap-2.5">
        <span class="flex items-baseline text-[23px] font-semibold tracking-[-0.02em] text-ink">
          <LogoH class="mr-[3px] h-[0.78em] w-auto" />oard
        </span>
        <span
          class="ml-1 rounded-full border border-line bg-surface px-2 py-0.5 font-mono text-[10px] tracking-wider text-ink-soft"
        >
          {$_('footer.version_label', { values: { v: $version } })}
        </span>
      </div>
      <p class="mt-3 max-w-xs text-sm leading-relaxed text-ink-soft">{$_('footer.tagline')}</p>
      <!-- The home lost its closing call to action, so the download lives here:
           the brand column is the last thing read on every page, not just the
           front one. -->
      <div class="mt-5 max-w-[13rem]">
        <Button href={$localeHref('/download')} variant="primary" size="md" full>
          <Download class="h-4 w-4" aria-hidden="true" />
          {$_('cta_section.cta')}
        </Button>
      </div>
    </div>

    <div>
      <h3 class="font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">
        {$_('footer.product')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-ink-soft">
        <li><a class="link-underline hover:text-ink" href={$localeHref('/pricing')}>{$_('nav.pricing')}</a></li>
        <li>
          <a class="link-underline hover:text-ink" href={$localeHref('/download')}>
            {$_('nav.download')}
          </a>
        </li>
        <li>
          <a
            class="link-underline hover:text-ink"
            href="https://github.com/rleeon/hoard/blob/main/CHANGELOG.md"
            rel="noreferrer"
          >
            {$_('footer.changelog')}
          </a>
        </li>
        <li>
          <a class="link-underline hover:text-ink" href={$localeHref('/guides')}>{$_('footer.guides')}</a>
        </li>
        <li><a class="link-underline hover:text-ink" href={$localeHref('/help')}>{$_('footer.status')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">
        {$_('footer.company')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-ink-soft">
        <li>
          <a class="link-underline hover:text-ink" href="https://github.com/rleeon/hoard" rel="noreferrer">
            {$_('footer.github')}
          </a>
        </li>
        <li>
          <a
            class="link-underline hover:text-ink"
            href="https://discord.gg/BYpXT8v4rh"
            target="_blank"
            rel="noopener noreferrer"
          >
            Discord
          </a>
        </li>
        <li><a class="link-underline hover:text-ink" href={$localeHref('/help')}>{$_('footer.contact')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint">
        {$_('footer.legal')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-ink-soft">
        <li><a class="link-underline hover:text-ink" href={$localeHref('/legal/terms')}>{$_('footer.terms')}</a></li>
        <li><a class="link-underline hover:text-ink" href={$localeHref('/legal/privacy')}>{$_('footer.privacy')}</a></li>
        <li><a class="link-underline hover:text-ink" href={$localeHref('/legal/subprocessors')}>{$_('footer.subprocessors')}</a></li>
        <li><a class="link-underline hover:text-ink" href={$localeHref('/legal/notice')}>{$_('footer.notice')}</a></li>
      </ul>
    </div>
  </div>
  <div class="border-t border-line">
    <p class="mx-auto max-w-6xl px-4 py-4 text-xs text-ink-faint sm:px-6">
      {$_('footer.copyright', { values: { year } })}
    </p>
  </div>
</footer>
