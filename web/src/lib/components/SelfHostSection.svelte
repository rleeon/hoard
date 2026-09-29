<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { ArrowRight } from 'lucide-svelte';

  // The "Self-host" section, laid out as the user picked it: text left,
  // terminal right, and the real server panel screenshot (selfhost.webp)
  // running full width at the bottom of the pine band, with its shadow cast
  // upward onto the band. The screenshot must never leave the frame, which is
  // why the card clips and the image is padded, not bled.
  //
  // Both grid cells carry min-w-0: the terminal is a nowrap <pre>, and without
  // it the auto column takes the terminal's max-content width (~390px), which
  // dragged the heading and the body out of the card on any phone (they were
  // clipped, not wrapped, below a ~466px viewport).
  //
  // The curl URL is split with a backslash-newline so the terminal keeps its
  // width; the shell joins it back into one word, so pasting it still works.

  const github = 'https://github.com/rleeon/hoard#self-host';
</script>

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
    <div class="reveal overflow-hidden rounded-3xl border border-pine-line bg-pine" use:reveal>
      <div class="grid items-center gap-10 p-10 pb-0 sm:p-14 sm:pb-0 lg:grid-cols-2">
        <div class="min-w-0 pb-14">
          <p class="kicker">{$_('selfhost.kicker')}</p>
          <h2 class="mt-3 text-balance text-3xl font-semibold text-white sm:text-4xl">
            {$_('selfhost.title')}
          </h2>
          <p class="mt-4 text-pretty leading-relaxed text-white/60">
            {$_('selfhost.body')}
          </p>
          <a
            href={github}
            target="_blank"
            rel="noopener noreferrer"
            class="link-underline mt-6 inline-flex items-center gap-1.5 text-sm font-medium text-emerald-400 ring-focus hover:text-emerald-300"
          >
            {$_('selfhost.cta')}
            <ArrowRight class="h-4 w-4" />
          </a>
        </div>
        <div class="min-w-0">
          <pre
            class="overflow-x-auto rounded-xl border border-pine-line bg-black/30 p-5 font-mono text-[13px] leading-relaxed text-white/80"><code
              >mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/\
rleeon/hoard/main/deploy/docker/docker-compose.yml
docker compose up -d</code
            ></pre>
        </div>
      </div>
      <div class="px-10 sm:px-14">
        <img
          src="/selfhost.webp?v=1"
          alt={$_('slot.selfhost')}
          loading="lazy"
          decoding="async"
          class="block w-full rounded-xl border border-pine-line shadow-[0_-30px_60px_-30px_rgba(0,0,0,0.6)]"
        />
      </div>
      <div class="h-10 sm:h-14"></div>
    </div>
  </div>
</section>
