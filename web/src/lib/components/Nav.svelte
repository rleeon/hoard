<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { session } from '$lib/stores/session';
  import { localeHref } from '$lib/i18n/href';
  import {
    LOCALES,
    LOCALE_NAMES,
    DEFAULT_LOCALE,
    isLocale,
    stripLocale,
    withLocale,
    type Locale
  } from '$lib/i18n/locales';
  import LogoMark from './LogoMark.svelte';
  import { Menu, X, Globe, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';

  let open = $state(false);
  let langOpen = $state(false);
  let scrolled = $state(false);

  // Current path with the locale prefix stripped, so the switcher can re-point
  // it at any other language and `isActive` compares language-independently.
  const barePath = $derived(stripLocale($page.url.pathname));
  const active = $derived<Locale>(isLocale($locale) ? ($locale as Locale) : DEFAULT_LOCALE);

  // Functional routes (login, account, checkout, auth) live outside the
  // `[[lang=locale]]` tree and have no localized URL. On those pages the
  // language switcher points at the home in the chosen language instead of a
  // non-existent prefixed path (which the prerender crawler would 404 on).
  const FUNCTIONAL = ['/login', '/account', '/checkout', '/auth'];
  const localizable = $derived(
    !FUNCTIONAL.some((p) => barePath === p || barePath.startsWith(p + '/'))
  );
  const langTarget = (l: Locale) => (localizable ? withLocale(barePath, l) : withLocale('/', l));

  async function signOut() {
    const { auth } = await import('$lib/auth');
    await auth.signOut();
    open = false;
    goto($localeHref('/'));
  }

  function isActive(href: string) {
    return barePath === href || barePath.startsWith(href + '/');
  }

  onMount(() => {
    function onScroll() {
      scrolled = window.scrollY > 8;
    }
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });
</script>

<header
  class="sticky top-0 z-40 w-full border-b transition-[background-color,border-color] duration-300 {scrolled
    ? 'border-line bg-bg/90 backdrop-blur-md'
    : 'border-transparent bg-bg/75 backdrop-blur-sm'}"
>
  <nav class="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6">
    <a href={$localeHref('/')} class="flex items-center gap-2.5 rounded-md ring-focus" aria-label="Hoard home">
      <LogoMark size={28} />
      <span class="font-display text-base font-semibold tracking-tight text-ink">Hoard</span>
    </a>

    <div class="hidden items-center gap-7 md:flex">
      <a
        href={$localeHref('/pricing')}
        class="link-underline ring-focus text-sm transition-colors {isActive('/pricing')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.pricing')}
      </a>
      <a
        href={$localeHref('/help')}
        class="link-underline ring-focus text-sm transition-colors {isActive('/help')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.help')}
      </a>
      <a
        href={$localeHref('/guides')}
        class="link-underline ring-focus text-sm transition-colors {isActive('/guides')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.guides')}
      </a>
      <a
        href={$localeHref('/download')}
        class="link-underline ring-focus text-sm transition-colors {isActive('/download')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.download')}
      </a>
    </div>

    <div class="hidden items-center gap-3 md:flex">
      <!-- Language switcher: <details> keeps every locale link in the static
           HTML so the prerender crawler discovers all 8 URLs of each page. -->
      <details
        bind:open={langOpen}
        class="group relative"
      >
        <summary
          class="flex cursor-pointer list-none items-center gap-1.5 rounded-md px-2 py-1.5 text-sm text-ink-soft ring-focus transition-colors hover:text-ink [&::-webkit-details-marker]:hidden"
          aria-label="Language"
        >
          <Globe class="h-4 w-4" />
          <span class="font-medium">{LOCALE_NAMES[active]}</span>
        </summary>
        <!-- preload-data="tap" overrides the layout's "hover" default: the
             locale `load` calls `locale.set(lang)`, so a hover-preload would
             switch the whole page's language just by mousing over an item.
             Restrict it to the actual click. -->
        <div
          data-sveltekit-preload-data="tap"
          class="lang-menu absolute right-0 z-50 mt-2 min-w-40 overflow-hidden rounded-lg border border-line bg-bg/95 py-1 shadow-lg backdrop-blur-md"
        >
          {#each LOCALES as l (l)}
            <a
              href={langTarget(l)}
              hreflang={l}
              onclick={() => (langOpen = false)}
              class="flex items-center justify-between gap-3 px-3 py-2 text-sm transition-colors hover:bg-ink/5 {l === active
                ? 'text-ink'
                : 'text-ink-soft hover:text-ink'}"
            >
              {LOCALE_NAMES[l]}
              {#if l === active}<Check class="h-3.5 w-3.5 text-accent" />{/if}
            </a>
          {/each}
        </div>
      </details>
      {#if $session === undefined}
        <span class="h-9 w-24 animate-pulse rounded-md bg-ink/5"></span>
      {:else if $session}
        <a
          href={$localeHref('/account')}
          class="group flex items-center gap-2 rounded-full border border-line bg-surface py-1 pl-1 pr-3 ring-focus transition-colors hover:border-line-strong"
        >
          {#if $session.avatarUrl}
            <img
              src={$session.avatarUrl}
              alt=""
              width="28"
              height="28"
              class="h-7 w-7 rounded-full"
              referrerpolicy="no-referrer"
            />
          {:else}
            <span
              class="grid h-7 w-7 place-items-center rounded-full bg-accent-deep text-xs font-semibold text-white"
            >
              {($session.email[0] ?? '?').toUpperCase()}
            </span>
          {/if}
          <span class="text-sm text-ink-soft transition-colors group-hover:text-ink">
            {$_('nav.account')}
          </span>
        </a>
      {:else}
        <a
          href={$localeHref('/login')}
          class="inline-flex h-9 items-center rounded-lg bg-accent px-4 text-sm font-medium text-pine ring-focus transition-colors hover:bg-emerald-300"
        >
          {$_('nav.signin')}
        </a>
      {/if}
    </div>

    <button
      class="md:hidden grid h-10 w-10 place-items-center rounded-lg text-ink-soft ring-focus hover:bg-ink/5"
      onclick={() => (open = !open)}
      aria-label="Toggle menu"
      aria-expanded={open}
    >
      <span class="relative h-5 w-5">
        <Menu
          class="absolute inset-0 h-5 w-5 transition-all duration-300 {open
            ? 'opacity-0 rotate-90'
            : 'opacity-100 rotate-0'}"
        />
        <X
          class="absolute inset-0 h-5 w-5 transition-all duration-300 {open
            ? 'opacity-100 rotate-0'
            : 'opacity-0 -rotate-90'}"
        />
      </span>
    </button>
  </nav>

  {#if open}
    <div
      class="overflow-hidden border-t border-line bg-bg md:hidden"
      transition:slide={{ duration: 220, easing: cubicOut }}
    >
      <div class="mx-auto flex max-w-6xl flex-col gap-1 px-4 py-3">
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/pricing')} onclick={() => (open = false)}>
          {$_('nav.pricing')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/help')} onclick={() => (open = false)}>
          {$_('nav.help')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/guides')} onclick={() => (open = false)}>
          {$_('nav.guides')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/download')} onclick={() => (open = false)}>
          {$_('nav.download')}
        </a>
        {#if $session}
          <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/account')} onclick={() => (open = false)}>
            {$_('nav.account')}
          </a>
          <button class="rounded-md px-3 py-2.5 text-left text-ink hover:bg-ink/5" onclick={signOut}>
            {$_('nav.signout')}
          </button>
        {:else}
          <a
            class="mt-1 inline-flex items-center justify-center rounded-md bg-accent px-3 py-2.5 font-medium text-pine hover:bg-emerald-300"
            href={$localeHref('/login')}
            onclick={() => (open = false)}
          >
            {$_('nav.signin')}
          </a>
        {/if}

        <div class="mt-2 border-t border-line pt-3">
          <p class="px-3 pb-1 font-mono text-[10px] uppercase tracking-[0.16em] text-ink-faint">
            <Globe class="mr-1 inline h-3 w-3" />Language
          </p>
          <div class="grid grid-cols-2 gap-1" data-sveltekit-preload-data="tap">
            {#each LOCALES as l (l)}
              <a
                class="flex items-center justify-between rounded-md px-3 py-2 text-sm hover:bg-ink/5 {l === active
                  ? 'text-ink'
                  : 'text-ink-soft'}"
                href={langTarget(l)}
                hreflang={l}
                onclick={() => (open = false)}
              >
                {LOCALE_NAMES[l]}
                {#if l === active}<Check class="h-3.5 w-3.5 text-accent" />{/if}
              </a>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</header>

<style>
  /* The locale links live inside a closed <details> so the prerender crawler
     discovers all 8 URLs in the static HTML. But the panel is positioned
     absolute, and an absolutely-positioned child escapes the native
     "hide when closed" behaviour of <details> — so without this it stayed
     painted on top of the page even while closed (clicking the summary
     toggled `open` but nothing visibly changed). Gate the panel on the
     parent's `open` state explicitly; the links stay in the DOM for SEO. */
  details > .lang-menu {
    display: none;
  }
  details[open] > .lang-menu {
    display: block;
  }
</style>
