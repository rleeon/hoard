<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import Button from '$lib/components/Button.svelte';
  import { Heart } from 'lucide-svelte';

  // The "Support Hoard" section, as the user picked it (the dense variant):
  // a thick field of tiny watermark hearts across the card, with the copy and
  // the two buttons. The heart inside the red button is white, or it would be
  // invisible against the button. The hearts are always red, deliberately off
  // the emerald palette.

  const sponsors = 'https://github.com/sponsors/rleeon';
  const funding = 'https://github.com/rleeon/hoard/blob/main/FUNDING.md';
</script>

<!-- A watermark heart: big, faint, sitting behind the card content. -->
{#snippet water(extra: string)}
  <Heart class="pointer-events-none absolute {extra} text-red-600/10" aria-hidden="true" />
{/snippet}

<!-- The small solid heart, next to the title and inside the red button.
     The size comes in as a whole class pair: passing an extra h-4 w-4 on top
     of a fixed h-5 w-5 does nothing, since the stylesheet order decides. -->
{#snippet heart(size: string = 'h-5 w-5', white: boolean = false)}
  <Heart class="{size} shrink-0 {white ? 'text-white' : 'text-red-600'}" aria-hidden="true" />
{/snippet}

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-16 sm:px-6">
    <div class="reveal mx-auto max-w-4xl" use:reveal>
      <div
        class="relative overflow-hidden rounded-2xl border border-red-600/50 bg-rose-50 px-6 py-12 sm:px-12"
      >
        <!-- A dense field of tiny hearts scattered across the card -->
        {@render water('-top-4 left-[8%] h-12 w-12')}
        {@render water('top-10 left-[30%] h-10 w-10')}
        {@render water('top-6 right-[25%] h-12 w-12')}
        {@render water('-top-2 right-[8%] h-10 w-10')}
        {@render water('top-1/2 left-[10%] h-10 w-10 -translate-y-1/2')}
        {@render water('top-1/2 right-[12%] h-12 w-12 -translate-y-1/2')}
        {@render water('top-1/2 left-[45%] h-8 w-8 -translate-y-1/2')}
        {@render water('top-1/2 left-[65%] h-8 w-8 -translate-y-1/2')}
        {@render water('bottom-1/4 left-[22%] h-10 w-10')}
        {@render water('bottom-10 left-[55%] h-8 w-8')}
        {@render water('bottom-6 right-[30%] h-10 w-10')}
        {@render water('-bottom-3 right-[10%] h-12 w-12')}
        {@render water('-bottom-4 left-1/3 h-10 w-10')}

        <div class="relative">
          <div class="flex flex-col items-center gap-6 text-center lg:flex-row lg:justify-between lg:text-left">
            <div class="flex max-w-xl flex-col items-center gap-3 lg:items-start">
              <div class="flex items-center justify-center gap-3">
                {@render heart()}
                <h2 class="font-display text-2xl font-semibold text-rose-950">
                  {$_('support.title')}
                </h2>
              </div>
              <p class="whitespace-pre-line text-pretty text-sm text-rose-900">{$_('support.body')}</p>
            </div>
            <div class="shrink-0">
              <div class="flex flex-col items-stretch gap-3 sm:flex-row sm:items-center">
                <Button href={sponsors} target="_blank" variant="support">
                  {#snippet children()}
                    {@render heart('h-4 w-4', true)}
                    {$_('support.cta')}
                  {/snippet}
                </Button>
                <Button href={funding} target="_blank" variant="support-quiet">
                  {#snippet children()}
                    {$_('support.funding')}
                  {/snippet}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</section>
