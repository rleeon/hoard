<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { session } from '$lib/stores/session';
  import LogoMark from './LogoMark.svelte';
  import StatusDot from './StatusDot.svelte';
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
  class="sticky top-0 z-40 w-full border-b transition-[background-color,border-color,box-shadow] duration-300 {scrolled
    ? 'border-white/[0.08] bg-zinc-950/85 shadow-[0_8px_24px_-12px_rgba(0,0,0,0.5)] backdrop-blur-xl'
    : 'border-transparent bg-zinc-950/55 backdrop-blur-lg'}"
>
  <nav class="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6">
    <a
      href="/"
      class="group flex items-center gap-2.5 rounded-md ring-focus"
      aria-label="Hoard home"
    >
      <span class="transition-transform duration-500 group-hover:rotate-[8deg] group-hover:scale-110">
        <LogoMark size={28} animated />
      </span>
      <span class="text-base font-semibold tracking-tight text-white">Hoard</span>
    </a>

    <div class="hidden items-center gap-7 md:flex">
      <a
        href="/pricing"
        class="link-underline ring-focus text-sm transition-colors {isActive('/pricing')
          ? 'text-white'
          : 'text-zinc-400 hover:text-white'}"
      >
        {$_('nav.pricing')}
      </a>
      <a
        href="/help"
        class="link-underline ring-focus text-sm transition-colors {isActive('/help')
          ? 'text-white'
          : 'text-zinc-400 hover:text-white'}"
      >
        {$_('nav.help')}
      </a>
      <a
        href="/download"
        class="link-underline ring-focus text-sm transition-colors {isActive('/download')
          ? 'text-white'
          : 'text-zinc-400 hover:text-white'}"
      >
        {$_('nav.download')}
      </a>
    </div>

    <div class="hidden items-center gap-3 md:flex">
      {#if $session === undefined}
        <span class="h-9 w-24 rounded-md shimmer bg-zinc-900/60"></span>
      {:else if $session}
        <a
          href="/account"
          class="group flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.03] py-1 pl-1 pr-3 ring-focus transition-colors hover:border-emerald-400/40 hover:bg-emerald-400/[0.06]"
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
              class="grid h-7 w-7 place-items-center rounded-full bg-gradient-to-br from-emerald-500 to-emerald-700 text-xs font-semibold text-white"
            >
              {($session.email[0] ?? '?').toUpperCase()}
            </span>
          {/if}
          <span class="text-sm text-zinc-200 transition-colors group-hover:text-white">
            {$_('nav.account')}
          </span>
        </a>
      {:else}
        <a
          href="/login"
          class="group relative inline-flex h-9 items-center overflow-hidden rounded-lg bg-gradient-to-b from-emerald-500 to-emerald-600 px-4 text-sm font-medium text-white shadow-[0_6px_24px_-8px_rgba(16,185,129,0.65),inset_0_1px_0_rgba(255,255,255,0.18)] ring-focus transition-all hover:from-emerald-400 hover:to-emerald-500 hover:shadow-[0_12px_32px_-10px_rgba(16,185,129,0.85),inset_0_1px_0_rgba(255,255,255,0.22)]"
        >
          <span class="relative z-10">{$_('nav.signin')}</span>
          <span
            aria-hidden="true"
            class="pointer-events-none absolute inset-y-0 left-0 w-1/2 -translate-x-full bg-gradient-to-r from-transparent via-white/35 to-transparent transition-transform duration-[800ms] ease-out group-hover:translate-x-[220%]"
          ></span>
        </a>
      {/if}
    </div>

    <button
      class="md:hidden grid h-10 w-10 place-items-center rounded-lg text-zinc-300 ring-focus hover:bg-white/5"
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
      class="overflow-hidden border-t border-white/[0.06] bg-zinc-950 md:hidden"
      transition:slide={{ duration: 220, easing: cubicOut }}
    >
      <div class="mx-auto flex max-w-6xl flex-col gap-1 px-4 py-3">
        <a class="rounded-md px-3 py-2.5 text-zinc-200 hover:bg-white/5" href="/pricing" onclick={() => (open = false)}>
          {$_('nav.pricing')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-zinc-200 hover:bg-white/5" href="/help" onclick={() => (open = false)}>
          {$_('nav.help')}
        </a>
        <a class="rounded-md px-3 py-2.5 text-zinc-200 hover:bg-white/5" href="/download" onclick={() => (open = false)}>
          {$_('nav.download')}
        </a>
        {#if $session}
          <a class="rounded-md px-3 py-2.5 text-zinc-200 hover:bg-white/5" href="/account" onclick={() => (open = false)}>
            {$_('nav.account')}
          </a>
          <button class="rounded-md px-3 py-2.5 text-left text-zinc-200 hover:bg-white/5" onclick={signOut}>
            {$_('nav.signout')}
          </button>
        {:else}
          <a
            class="mt-1 inline-flex items-center justify-center rounded-md bg-emerald-600 px-3 py-2.5 font-medium text-white hover:bg-emerald-500"
            href="/login"
            onclick={() => (open = false)}
          >
            {$_('nav.signin')}
          </a>
        {/if}
        <div class="mt-3 border-t border-white/[0.06] pt-3">
          <StatusDot />
        </div>
      </div>
    </div>
  {/if}
</header>
