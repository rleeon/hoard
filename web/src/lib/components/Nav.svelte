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
  import DiscordIcon from './DiscordIcon.svelte';
  const DISCORD_URL = 'https://discord.gg/BYpXT8v4rh';
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
  class="sticky top-0 z-40 w-full bg-bg/80 backdrop-blur-md"
>
  <nav
    class="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6 md:grid md:grid-cols-[1fr_auto_1fr]"
  >
    <a
      href={$localeHref('/')}
      class="nav-pill flex items-center gap-2.5 ring-focus md:justify-self-start"
      aria-label="Hoard home"
    >
      <LogoMark size={28} />
      <span class="font-display text-base font-semibold tracking-tight text-ink">Hoard</span>
    </a>

    <div class="hidden items-center gap-1 md:flex md:justify-self-center">
      <a
        href={$localeHref('/download')}
        class="nav-pill ring-focus text-sm {isActive('/download')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.download')}
      </a>
      <a
        href={$localeHref('/pricing')}
        class="nav-pill ring-focus text-sm {isActive('/pricing')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.pricing')}
      </a>
      <a
        href={$localeHref('/cli')}
        class="nav-pill ring-focus text-sm {isActive('/cli')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.cli')}
      </a>
      <a
        href={$localeHref('/guides')}
        class="nav-pill ring-focus text-sm {isActive('/guides')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.guides')}
      </a>
      <a
        href={$localeHref('/help')}
        class="nav-pill ring-focus text-sm {isActive('/help')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.help')}
      </a>
    </div>

    <div class="hidden items-center gap-3 md:flex md:justify-self-end">
      <!-- Language switcher: <details> keeps every locale link in the static
           HTML so the prerender crawler discovers all 8 URLs of each page. -->
      <details
        bind:open={langOpen}
        class="group relative"
      >
        <summary
          class="nav-pill flex cursor-pointer list-none items-center gap-1.5 text-sm text-ink-soft ring-focus [&::-webkit-details-marker]:hidden"
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
          href="/account"
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
          href="/login"
          class="inline-flex h-9 items-center rounded-lg bg-accent px-4 text-sm font-medium text-pine ring-focus transition-colors hover:bg-emerald-300"
        >
          {$_('nav.signin')}
        </a>
      {/if}
      <!-- Icon-only in Discord's blurple: it reads as a second call to action
           without fighting the emerald one next to it. -->
      <a
        href={DISCORD_URL}
        target="_blank"
        rel="noopener noreferrer"
        aria-label="Discord"
        title="Discord"
        class="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-[#5865F2] text-white ring-focus transition-colors hover:bg-[#4752C4]"
      >
        <DiscordIcon class="h-[18px] w-[18px]" />
      </a>
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
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/download')} onclick={() => (open = false)}>
          {$_('nav.download')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/pricing')} onclick={() => (open = false)}>
          {$_('nav.pricing')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/cli')} onclick={() => (open = false)}>
          {$_('nav.cli')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/guides')} onclick={() => (open = false)}>
          {$_('nav.guides')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/help')} onclick={() => (open = false)}>
          {$_('nav.help')}
        </a>
        {#if $session}
          <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href="/account" onclick={() => (open = false)}>
            {$_('nav.account')}
          </a>
          <button class="rounded-md px-3 py-2.5 text-left text-ink hover:bg-ink/5" onclick={signOut}>
            {$_('nav.signout')}
          </button>
        {:else}
          <a
            class="mt-1 inline-flex items-center justify-center rounded-md bg-accent px-3 py-2.5 font-medium text-pine hover:bg-emerald-300"
            href="/login"
            onclick={() => (open = false)}
          >
            {$_('nav.signin')}
          </a>
        {/if}
        <a
          class="mt-1 inline-flex items-center justify-center gap-2 rounded-md bg-[#5865F2] px-3 py-2.5 font-medium text-white hover:bg-[#4752C4]"
          href={DISCORD_URL}
          target="_blank"
          rel="noopener noreferrer"
          onclick={() => (open = false)}
        >
          <DiscordIcon class="h-[18px] w-[18px]" />
          Discord
        </a>

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

  <!-- Bottom hairline: a dedicated line that only fades its opacity as you
       scroll — background and blur stay constant, so appearing/disappearing
       reads as a clean fade rather than a jump in fill + blur. -->
  <div
    class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-line transition-opacity duration-500 ease-out"
    style="opacity: {scrolled ? 1 : 0};"
    aria-hidden="true"
  ></div>
</header>

<style>
  /* The locale links live inside a closed <details> so the prerender crawler
     discovers all 8 URLs in the static HTML. But the panel is positioned
     absolute, and an absolutely-positioned child escapes the native
     "hide when closed" behaviour of <details>, so without this it stayed
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
