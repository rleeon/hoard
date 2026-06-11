<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { session } from '$lib/stores/session';
  import LogoMark from './LogoMark.svelte';
  import { Menu, X } from 'lucide-svelte';
  import { onMount } from 'svelte';

  let open = $state(false);
  let scrolled = $state(false);

  async function signOut() {
    const { auth } = await import('$lib/auth');
    await auth.signOut();
    open = false;
    goto('/');
  }

  function isActive(href: string) {
    return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
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
    <a href="/" class="flex items-center gap-2.5 rounded-md ring-focus" aria-label="Hoard home">
      <LogoMark size={28} />
      <span class="font-display text-base font-semibold tracking-tight text-ink">Hoard</span>
    </a>

    <div class="hidden items-center gap-7 md:flex">
      <a
        href="/pricing"
        class="link-underline ring-focus text-sm transition-colors {isActive('/pricing')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.pricing')}
      </a>
      <a
        href="/help"
        class="link-underline ring-focus text-sm transition-colors {isActive('/help')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.help')}
      </a>
      <a
        href="/download"
        class="link-underline ring-focus text-sm transition-colors {isActive('/download')
          ? 'text-ink'
          : 'text-ink-soft hover:text-ink'}"
      >
        {$_('nav.download')}
      </a>
    </div>

    <div class="hidden items-center gap-3 md:flex">
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
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href="/pricing" onclick={() => (open = false)}>
          {$_('nav.pricing')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href="/help" onclick={() => (open = false)}>
          {$_('nav.help')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-ink hover:bg-ink/5" href="/download" onclick={() => (open = false)}>
          {$_('nav.download')}
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
      </div>
    </div>
  {/if}
</header>
