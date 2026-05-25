<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { session } from '$lib/stores/session';
  import { auth } from '$lib/auth';
  import LogoMark from './LogoMark.svelte';
  import { Menu, X } from 'lucide-svelte';

  let open = $state(false);

  async function signOut() {
    await auth.signOut();
    open = false;
    goto('/');
  }

  function linkClass(href: string) {
    const active = $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
    return active
      ? 'text-white'
      : 'text-zinc-400 hover:text-white transition-colors';
  }
</script>

<header
  class="sticky top-0 z-40 w-full border-b border-zinc-900/80 bg-zinc-950/80 backdrop-blur"
>
  <nav class="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6">
    <a href="/" class="flex items-center gap-2.5">
      <LogoMark size={28} />
      <span class="text-base font-semibold tracking-tight text-white">Hoard</span>
    </a>

    <div class="hidden items-center gap-7 md:flex">
      <a href="/pricing" class={linkClass('/pricing')}>{$_('nav.pricing')}</a>
      <a href="/help" class={linkClass('/help')}>{$_('nav.help')}</a>
      <a
        href="https://github.com/rleeon/hoard/releases/latest"
        target="_blank"
        rel="noreferrer"
        class="text-zinc-400 hover:text-white transition-colors"
      >
        {$_('nav.get_app')}
      </a>
    </div>

    <div class="hidden items-center gap-3 md:flex">
      {#if $session === undefined}
        <span class="h-9 w-20 rounded-md shimmer bg-zinc-900/60"></span>
      {:else if $session}
        <a
          href="/account"
          class="flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/60 py-1 pl-1 pr-3 transition-colors hover:border-zinc-700"
        >
          {#if $session.avatarUrl}
            <img
              src={$session.avatarUrl}
              alt=""
              class="h-7 w-7 rounded-full"
              referrerpolicy="no-referrer"
            />
          {:else}
            <span
              class="grid h-7 w-7 place-items-center rounded-full bg-emerald-700 text-xs font-semibold text-white"
            >
              {($session.email[0] ?? '?').toUpperCase()}
            </span>
          {/if}
          <span class="text-sm text-zinc-200">{$_('nav.account')}</span>
        </a>
      {:else}
        <a
          href="/login"
          class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow-md shadow-emerald-900/30 transition-all hover:-translate-y-px hover:bg-emerald-500"
        >
          {$_('nav.signin')}
        </a>
      {/if}
    </div>

    <button
      class="md:hidden grid h-10 w-10 place-items-center rounded-lg text-zinc-300 hover:bg-zinc-900"
      onclick={() => (open = !open)}
      aria-label="Toggle menu"
    >
      {#if open}
        <X class="h-5 w-5" />
      {:else}
        <Menu class="h-5 w-5" />
      {/if}
    </button>
  </nav>

  {#if open}
    <div class="border-t border-zinc-900 bg-zinc-950 md:hidden">
      <div class="mx-auto flex max-w-6xl flex-col gap-1 px-4 py-3">
        <a class="px-2 py-2 text-zinc-200" href="/pricing" onclick={() => (open = false)}
          >{$_('nav.pricing')}</a
        >
        <a class="px-2 py-2 text-zinc-200" href="/help" onclick={() => (open = false)}
          >{$_('nav.help')}</a
        >
        {#if $session}
          <a class="px-2 py-2 text-zinc-200" href="/account" onclick={() => (open = false)}
            >{$_('nav.account')}</a
          >
          <button class="px-2 py-2 text-left text-zinc-200" onclick={signOut}
            >{$_('nav.signout')}</button
          >
        {:else}
          <a class="px-2 py-2 text-zinc-200" href="/login" onclick={() => (open = false)}
            >{$_('nav.signin')}</a
          >
        {/if}
      </div>
    </div>
  {/if}
</header>
