<script lang="ts">
  import { _, locale } from 'svelte-i18n';
  import { page } from '$app/stores';
  import { afterNavigate, goto } from '$app/navigation';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { session } from '$lib/stores/session';
  import { preloadLocale } from '$lib/i18n';
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
  import LogoH from './LogoH.svelte';
  import { scrollLag } from '$lib/actions/scrollLag';
  import DiscordIcon from './DiscordIcon.svelte';
  const DISCORD_URL = 'https://discord.gg/BYpXT8v4rh';
  import { Menu, X, Globe, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';

  let open = $state(false);

  // The mobile panel covers the whole viewport, so navigating from inside it
  // (the wordmark, the browser's back button, anything that isn't one of the
  // links below) used to leave it open on top of the new page: the route had
  // changed but nothing looked like it had. Closing on every navigation lets
  // the slide-out play as the page changes, which is the only motion that says
  // "you moved".
  afterNavigate(() => {
    open = false;
    langMobile = false;
  });
  let langOpen = $state(false);
  let langMobile = $state(false);
  // Picking a language re-renders every `$_` on the page at the same moment the
  // panel would be animating its height shut, and a height animation inside a
  // `backdrop-blur` header is the expensive kind: the two together drop a frame
  // or two. The panel closes instantly in that case, the page changing language
  // is signal enough; the toggle keeps its slide.
  let langInstant = $state(false);
  let scrolled = $state(false);
  // Set by the first docked/floating change the reader causes. Until then the
  // capsule only shows its state, so a reload halfway down the page doesn't
  // open with a drip nobody asked for.
  let armed = $state(false);

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
    let ready = false;
    function onScroll() {
      const next = window.scrollY > 8;
      if (ready && next !== scrolled) armed = true;
      scrolled = next;
    }
    onScroll();
    ready = true;
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });
</script>

<!-- A capsule, inset from the sides; the header around it is see-through.
     Opaque and unblurred: a blur was the priciest thing to repaint on every
     scrolled frame, and an 80% fill without one let the content show straight
     through. At the top of the page it hangs from the top edge like a drop
     (see `.capsule` below); once the page scrolls it lets go and floats. -->
<header class="sticky top-0 z-40 w-full px-3 pt-3 sm:px-6" data-motion="keep">
  <div
    class="capsule relative mx-auto max-w-5xl border border-white/[0.09] bg-capsule {scrolled
      ? 'is-floating'
      : 'is-docked'} {armed ? 'is-armed' : ''}"
    use:scrollLag={{ active: scrolled }}
  >
  <nav
    class="flex h-14 items-center justify-between px-2 lg:grid lg:grid-cols-[1fr_auto_1fr]"
  >
    <a
      href={$localeHref('/')}
      class="nav-pill glow pop-hold group flex items-center ring-focus lg:justify-self-start"
      aria-label="Hoard home"
    >
      <span class="flex items-baseline text-[23px] font-semibold tracking-[-0.02em] text-ink">
        <LogoH class="mr-[3px] h-[0.78em] w-auto" />oard
      </span>
    </a>

    <div class="hidden items-center gap-1 lg:flex lg:justify-self-center">
      <a
        href={$localeHref('/download')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/download')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.download')}
      </a>
      <a
        href={$localeHref('/pricing')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/pricing')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.pricing')}
      </a>
      <a
        href={$localeHref('/cli')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/cli')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.cli')}
      </a>
      <a
        href={$localeHref('/ai')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/ai')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.ai')}
      </a>
      <a
        href={$localeHref('/guides')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/guides')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.guides')}
      </a>
      <a
        href={$localeHref('/help')}
        class="nav-pill glow pop-hold ring-focus text-sm {isActive('/help')
          ? 'nav-pill-active'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.help')}
      </a>
    </div>

    <div class="hidden items-center gap-3 lg:flex lg:justify-self-end">
      <!-- Language switcher: <details> keeps every locale link in the static
           HTML so the prerender crawler discovers all 8 URLs of each page. -->
      <details
        bind:open={langOpen}
        class="group relative"
      >
        <summary
          class="nav-pill glow pop-hold flex cursor-pointer list-none items-center gap-1.5 text-sm text-ink-soft ring-focus [&::-webkit-details-marker]:hidden"
          aria-label="Language"
        >
          <!-- Same spin as the phone's globe: one language of motion for the
               same control on both sizes. -->
          <Globe
            class="lang-globe h-4 w-4 transition-transform duration-300 ease-out {langOpen
              ? 'rotate-180'
              : 'rotate-0'}"
          />
          <!-- The code, as on the phone: the full name gave the links in the
               middle less room on medium screens; the list below still names
               every language in full. -->
          <span
            class="font-mono text-xs uppercase transition-colors duration-300 {langOpen
              ? 'text-ink'
              : ''}">{active}</span
          >
        </summary>
        <!-- preload-data="tap" overrides the layout's "hover" default: the
             locale `load` calls `locale.set(lang)`, so a hover-preload would
             switch the whole page's language just by mousing over an item.
             Restrict it to the actual click. -->
        <div
          data-sveltekit-preload-data="tap"
          class="lang-menu absolute right-0 z-50 mt-3 min-w-40 overflow-hidden rounded-2xl border border-white/[0.09] bg-capsule p-1 shadow-lg"
        >
          {#each LOCALES as l (l)}
            <a
              href={langTarget(l)}
              hreflang={l}
              onclick={() => (langOpen = false)}
              onpointerdown={() => preloadLocale(l)}
              onfocus={() => preloadLocale(l)}
              class="glow flex items-center justify-between gap-3 rounded-xl px-3 py-2 text-sm transition-colors hover:bg-ink/5 {l === active
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
        <span class="h-9 w-24 animate-pulse rounded-full bg-ink/5"></span>
      {:else if $session}
        <a
          href="/account"
          class="glow pop-hold group flex items-center gap-2 rounded-full border border-white/25 bg-black py-1 pl-1 pr-3 ring-focus transition-colors hover:border-white/50"
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
          class="glow pop-hold inline-flex h-9 items-center rounded-full border border-white/25 bg-black px-4 text-sm font-medium text-ink ring-focus transition-colors hover:border-white/50"
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
        class="glow pop-hold grid h-9 w-9 shrink-0 place-items-center rounded-full bg-[#5865F2] text-white ring-focus transition-colors hover:bg-[#4752C4]"
      >
        <DiscordIcon class="h-[18px] w-[18px]" />
      </a>
    </div>

    <div class="flex items-center gap-1 lg:hidden">
      <!-- Language gets its own toggle next to the burger: it used to live at
           the bottom of the burger panel, past every link, which is a long way
           down for the one control a visitor in the wrong language needs first.
           The desktop <details> still carries all eight links in the static
           HTML, so the prerender crawler keeps finding every locale URL. -->
      <button
        class="glow anim-host pop-hold grid h-10 w-auto min-w-10 place-items-center gap-1 rounded-full px-3 text-ink-soft ring-focus hover:bg-ink/5"
        onclick={() => {
          langInstant = false;
          langMobile = !langMobile;
          open = false;
        }}
        aria-label="Language"
        aria-expanded={langMobile}
      >
        <span class="flex items-center gap-1.5">
          <!-- The globe spins on its own axis when the panel opens and unwinds
               when it closes, the same 300ms cross-fade the burger uses to
               swap its icon: two different controls, one language of motion. -->
          <Globe
            class="lang-globe h-5 w-5 transition-transform duration-300 ease-out {langMobile
              ? 'rotate-180'
              : 'rotate-0'}"
          />
          <span
            class="font-mono text-[11px] uppercase transition-colors duration-300 {langMobile
              ? 'text-ink'
              : ''}">{active}</span
          >
        </span>
      </button>

      <button
        class="glow pop-hold grid h-10 w-10 place-items-center rounded-full text-ink-soft ring-focus hover:bg-ink/5"
        onclick={() => {
          open = !open;
          langMobile = false;
        }}
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
    </div>
  </nav>

  {#if open}
    <div
      class="overflow-hidden border-t border-white/10 lg:hidden"
      transition:slide={{ duration: 220, easing: cubicOut }}
    >
      <div class="flex flex-col gap-1 p-3">
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/download')} onclick={() => (open = false)}>
          {$_('nav.download')}
        </a>
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/pricing')} onclick={() => (open = false)}>
          {$_('nav.pricing')}
        </a>
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/cli')} onclick={() => (open = false)}>
          {$_('nav.cli')}
        </a>
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/ai')} onclick={() => (open = false)}>
          {$_('nav.ai')}
        </a>
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/guides')} onclick={() => (open = false)}>
          {$_('nav.guides')}
        </a>
        <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href={$localeHref('/help')} onclick={() => (open = false)}>
          {$_('nav.help')}
        </a>
        {#if $session}
          <a class="glow rounded-xl px-3 py-2.5 text-ink hover:bg-ink/5" href="/account" onclick={() => (open = false)}>
            {$_('nav.account')}
          </a>
          <button class="glow rounded-xl px-3 py-2.5 text-left text-ink hover:bg-ink/5" onclick={signOut}>
            {$_('nav.signout')}
          </button>
        {:else}
          <a
            class="glow mt-1 inline-flex items-center justify-center rounded-xl border border-white/25 bg-black px-3 py-2.5 font-medium text-ink hover:border-white/50"
            href="/login"
            onclick={() => (open = false)}
          >
            {$_('nav.signin')}
          </a>
        {/if}
        <a
          class="glow anim-host mt-1 inline-flex items-center justify-center gap-2 rounded-xl bg-[#5865F2] px-3 py-2.5 font-medium text-white hover:bg-[#4752C4]"
          href={DISCORD_URL}
          target="_blank"
          rel="noopener noreferrer"
          onclick={() => (open = false)}
        >
          <DiscordIcon data-anim="pop" class="h-[18px] w-[18px]" />
          Discord
        </a>

      </div>
    </div>
  {/if}

  {#if langMobile}
    <div
      class="overflow-hidden border-t border-white/10 lg:hidden"
      transition:slide={{ duration: langInstant ? 0 : 220, easing: cubicOut }}
    >
      <div class="grid grid-cols-2 gap-1 p-3" data-sveltekit-preload-data="tap">
        {#each LOCALES as l (l)}
          <a
            class="glow flex items-center justify-between rounded-xl px-3 py-2.5 text-sm hover:bg-ink/5 {l === active
              ? 'text-ink'
              : 'text-ink-soft'}"
            href={langTarget(l)}
            hreflang={l}
            onpointerdown={() => preloadLocale(l)}
            onclick={() => {
              langInstant = true;
              langMobile = false;
            }}
          >
            {LOCALE_NAMES[l]}
            {#if l === active}<Check class="h-3.5 w-3.5 text-accent" />{/if}
          </a>
        {/each}
      </div>
    </div>
  {/if}

  </div>
</header>

<style>
  /* Every control in the bar grows by the same amount, on the owner's call, and
     only by that: the globe and the Discord mark used to add their own 30% icon
     pop on top, which made those two jump far more than the rest. The rest of
     the site keeps its buttons' 6% pop.
     The controls carry `.pop-hold` instead of `.pop-hold`: the pop always plays
     through, and stops at the top for as long as the pointer stays on the
     control (hoverMotion.ts). `--nav-grow` is the size it stops at. */
  header {
    --nav-grow: 1.3;
  }

  /* The owner's call: the floating capsule only once the page has scrolled.
     At the very top it docks to the top edge, same width, never full bleed,
     and the two top corners turn into concave shoulders that pull it up into
     the edge, the way a drop hangs from a surface. Letting go, it stretches,
     the shoulders shrink back into it, and it falls past its place and bounces
     back; docking, it rises into the edge, squashes and the shoulders grab.
     The owner asked for it stronger than a plain spring, hence keyframes. The
     whole bar keeps its motion under reduced-motion (`data-motion="keep"`, see
     app.css), on the owner's call. */
  .capsule {
    --r: 1.75rem;
    --shoulder: 1.5rem;
    /* The hairline as it actually renders: white at 9% over the capsule fill.
       Opaque on purpose, a translucent copy doubled up where it crosses the
       capsule's own border and left a brighter dash at the joint. */
    --rim: color-mix(in srgb, var(--color-capsule), #fff 9%);
    border-radius: var(--r);
    transform-origin: 50% 0;
    transition:
      border-radius 900ms cubic-bezier(0.22, 1.4, 0.36, 1),
      border-top-color 250ms ease,
      box-shadow 400ms ease;
  }
  .capsule.is-floating {
    /* The top corners round off and the top hairline comes back only once the
       shoulders are gone (about 200ms in): rounding them straight away left
       each shoulder beside a corner that had already pulled away from it. */
    transition:
      border-radius 800ms cubic-bezier(0.22, 1.4, 0.36, 1) 200ms,
      border-top-color 250ms ease 200ms,
      box-shadow 400ms ease;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      0 18px 50px -18px rgba(0, 0, 0, 1);
  }
  .capsule.is-docked {
    /* Up by the header's own top padding, flush with the viewport edge. */
    transform: translateY(-0.75rem);
    border-top-left-radius: 0;
    border-top-right-radius: 0;
    border-top-color: transparent;
  }
  /* The shoulders: a capsule-coloured corner with a quarter circle cut out of
     it, hairline included, sitting just outside each top corner. Each one is a
     pixel wider than the curve so it covers the capsule's own side border down
     to where the curve meets it: left uncovered, that stretch of border showed
     as a short vertical stick, on one side only, depending on how the capsule
     landed on the pixel grid. The hairline ends exactly on that border column. */
  .capsule::before,
  .capsule::after {
    content: '';
    position: absolute;
    top: -1px;
    width: calc(var(--shoulder) + 1px);
    height: calc(var(--shoulder) + 1px);
    pointer-events: none;
    opacity: 0;
    transform: scale(0);
  }
  .capsule::before {
    left: calc(-1 * var(--shoulder) - 1px);
    transform-origin: 100% 0;
    background: radial-gradient(
      circle at 0 100%,
      transparent var(--shoulder),
      var(--rim) var(--shoulder),
      var(--rim) calc(var(--shoulder) + 1px),
      var(--color-capsule) calc(var(--shoulder) + 1px)
    );
  }
  .capsule::after {
    right: calc(-1 * var(--shoulder) - 1px);
    transform-origin: 0 0;
    background: radial-gradient(
      circle at 100% 100%,
      transparent var(--shoulder),
      var(--rim) var(--shoulder),
      var(--rim) calc(var(--shoulder) + 1px),
      var(--color-capsule) calc(var(--shoulder) + 1px)
    );
  }
  .capsule.is-docked::before,
  .capsule.is-docked::after {
    opacity: 1;
    transform: scale(1);
  }

  /* The shoulders run on the capsule's clock (same duration). Letting go they
     shrink into the capsule's own corners within the first 200ms, before it
     has fallen far: they used to reach up to keep hold of the edge and were
     left hanging apart from the bar, a loose triangle, for most of a second. */
  .capsule.is-armed.is-floating {
    animation: capsule-drip 1.1s cubic-bezier(0.3, 0.6, 0.35, 1) both;
  }
  .capsule.is-armed.is-floating::before,
  .capsule.is-armed.is-floating::after {
    animation: shoulder-let-go 1.1s linear both;
  }
  .capsule.is-armed.is-docked {
    animation: capsule-dock 0.9s cubic-bezier(0.3, 0.6, 0.35, 1) both;
  }
  .capsule.is-armed.is-docked::before,
  .capsule.is-armed.is-docked::after {
    animation: shoulder-grip 0.9s cubic-bezier(0.3, 0.6, 0.35, 1) both;
  }
  @keyframes capsule-drip {
    0% {
      transform: translateY(-0.75rem) scale(1, 1);
    }
    22% {
      transform: translateY(-0.45rem) scale(0.99, 1.06);
    }
    42% {
      transform: translateY(0.5rem) scale(1.012, 0.96);
    }
    62% {
      transform: translateY(-0.2rem) scale(0.997, 1.02);
    }
    80% {
      transform: translateY(0.08rem) scale(1, 0.995);
    }
    100% {
      transform: translateY(0) scale(1, 1);
    }
  }
  @keyframes shoulder-let-go {
    0% {
      opacity: 1;
      transform: scale(1, 1);
    }
    9% {
      opacity: 1;
      transform: scale(0.7, 1.08);
    }
    17%,
    100% {
      opacity: 0;
      transform: scale(0, 0.5);
    }
  }
  @keyframes capsule-dock {
    0% {
      transform: translateY(0) scale(1, 1);
    }
    35% {
      transform: translateY(-0.95rem) scale(1.012, 0.955);
    }
    58% {
      transform: translateY(-0.65rem) scale(0.995, 1.025);
    }
    78% {
      transform: translateY(-0.8rem) scale(1, 0.995);
    }
    100% {
      transform: translateY(-0.75rem) scale(1, 1);
    }
  }
  @keyframes shoulder-grip {
    0% {
      opacity: 0;
      transform: scale(0, 0);
    }
    30% {
      opacity: 1;
    }
    45% {
      transform: scale(1.25, 1.15);
    }
    68% {
      transform: scale(0.93, 0.97);
    }
    85% {
      transform: scale(1.03, 1.01);
    }
    100% {
      opacity: 1;
      transform: scale(1, 1);
    }
  }

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
