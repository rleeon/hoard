<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Seo from '$lib/components/Seo.svelte';
  import { localeHref } from '$lib/i18n/href';
  import { reveal } from '$lib/actions/reveal';
  import { GAMES, gameSlug } from '$lib/saves';
  import { Search } from 'lucide-svelte';

  // Every game is in the prerendered HTML, so a crawler reaches all of them from
  // this one page; the search box only hides rows in the browser.
  const games = [...GAMES].sort((a, b) => a.n.localeCompare(b.n));
  const count = games.length;

  let query = $state('');
  const fold = (s: string) =>
    s
      .normalize('NFKD')
      .replace(/[̀-ͯ]/g, '')
      .toLowerCase();
  const shown = $derived(query.trim() ? games.filter((g) => fold(g.n).includes(fold(query.trim()))) : games);
</script>

<Seo path="/saves" key="saves" />

<section class="mx-auto max-w-3xl px-4 pb-16 pt-16 sm:px-6 sm:pb-24 sm:pt-24 2xl:pt-12">
  <header use:reveal class="reveal">
    <h1 class="font-display text-4xl font-semibold tracking-tight text-ink sm:text-5xl">
      {$_('saves.heading')}
    </h1>
    <p class="mt-4 max-w-2xl text-lg leading-relaxed text-ink-soft">
      {$_('saves.intro', { values: { count } })}
    </p>
  </header>

  <label class="mt-10 flex items-center gap-3 rounded-xl border border-white/[0.08] bg-black px-4 focus-within:border-white/20">
    <Search class="h-4 w-4 shrink-0 text-ink-faint" aria-hidden="true" />
    <span class="sr-only">{$_('saves.search_label')}</span>
    <input
      type="search"
      bind:value={query}
      placeholder={$_('saves.search_placeholder')}
      class="ring-focus h-12 w-full rounded-md bg-transparent text-ink placeholder:text-ink-faint"
      autocomplete="off"
      spellcheck="false"
    />
  </label>

  {#if shown.length}
    <ul class="mt-6 grid gap-2 sm:grid-cols-2">
      {#each shown as g (g.s)}
        <li>
          <a
            href={$localeHref(`/guides/${gameSlug(g)}`)}
            class="block rounded-lg border border-white/[0.06] bg-black px-4 py-3 text-sm text-ink-soft ring-focus transition-colors hover:border-white/20 hover:text-ink"
          >
            {g.n}
          </a>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="mt-8 text-sm leading-relaxed text-ink-soft">{$_('saves.empty')}</p>
  {/if}

  <p class="mt-10 text-xs leading-relaxed text-ink-faint">{$_('saves.source')}</p>
</section>
