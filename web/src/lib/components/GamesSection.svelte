<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { marquee } from '$lib/actions/marquee';
  import { GAMES } from '$lib/data/games';

  // The "Supported games" section.
  //
  // Design (the user's pick, the zoom stack merged with the full-bleed one):
  // three infinite marquee lanes, same direction, each a step deeper than the
  // one before, smaller chips, dimmer, slower as they recede, so the library
  // reads as a deep conveyor, running edge to edge at full viewport width.
  //
  // The text is the original one, unchanged (kicker, title, body, the
  // hand-verified game list and the footnote). All motion is rAF-driven (no
  // CSS transitions/SMIL), and the strip wraps modulo one copy so the seam
  // never shows, the library grows to thousands of games and the section
  // still works, because the strip never ends.

  // Two copies of the list per strip: the marquee wraps modulo one copy, so
  // the seam never shows.
  const GAME_COPIES = [0, 1];
</script>

{#snippet chip(g: (typeof GAMES)[number])}
  <!-- Steam links removed on the user's call: a full-width marquee with
       71 games × 3 lanes × 2 copies produced 426 anchors — a link farm that
       grows with the game list and is useless in a decorative strip. To
       restore the links, replace the span below with:
       {#if g.steamId !== null}
         <a href={`https://store.steampowered.com/app/${g.steamId}/`} target="_blank" rel="noopener noreferrer" title={$_('games.steam_link')} class="game-chip ring-focus">{g.name}</a>
       {:else}
         <span class="game-chip">{g.name}</span>
       {/if}
       g.steamId in the data and the games.steam_link label are kept so the
       links can come back without any other change. -->
  <span class="game-chip">{g.name}</span>
{/snippet}

{#snippet lane(speed: number, o: { dim: number; size: 'md' | 'sm' | 'xs' })}
  <div class="marquee {o.size === 'sm' ? 'lane-sm' : o.size === 'xs' ? 'lane-xs' : ''}" style={`opacity:${o.dim}`}>
    <div class="marquee-inner gap-4" use:marquee={{ speed }}>
      {#each GAME_COPIES as copy (copy)}
        <ul class="flex shrink-0 items-center" aria-hidden={copy > 0 || undefined}>
          {#each GAMES as g (g.name)}
            <li class="flex shrink-0 items-center">{@render chip(g)}</li>
          {/each}
        </ul>
      {/each}
    </div>
  </div>
{/snippet}

<section class="border-t border-line" aria-label={$_('games.aria')}>
  <div class="mx-auto max-w-6xl px-4 pt-20 sm:px-6 sm:pt-24">
    <div class="reveal max-w-2xl" use:reveal>
      <p class="kicker">{$_('games.kicker')}</p>
      <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
        {$_('games.title')}
      </h2>
      <p class="mt-3 text-pretty text-ink-soft">{$_('games.body')}</p>
    </div>
  </div>

  <!-- The three zoom lanes run full width, outside the content column:
       far lane small/dim/slow, near lane big/bright/fast. -->
  <div class="mt-12 flex flex-col gap-4">
    {@render lane(18, { size: 'xs', dim: 0.55 })}
    {@render lane(32, { size: 'sm', dim: 0.8 })}
    {@render lane(52, { size: 'md', dim: 1 })}
  </div>

  <div class="mx-auto max-w-6xl px-4 pb-20 sm:px-6 sm:pb-24">
    <p class="mt-8 text-center text-xs text-ink-faint">{$_('games.footnote')}</p>
  </div>
</section>

<style>
  /* Depth steps: receding lanes use smaller chips. */
  .lane-sm :global(.game-chip) {
    font-size: 11px;
    padding: 0.35rem 0.75rem;
  }
  .lane-xs :global(.game-chip) {
    font-size: 10px;
    padding: 0.3rem 0.65rem;
  }
</style>
