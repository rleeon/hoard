<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { reveal } from '$lib/actions/reveal';
  import { tilt } from '$lib/actions/tilt';
  import { RefreshCw, History, Fingerprint, ScanSearch, MonitorSmartphone, FileDown } from 'lucide-svelte';

  // The "What's in the box" section.
  //
  // Content, as the user specified it: no photos at all, the placeholders
  // were dropped; only the six features (icon, title, body) remain.
  //
  // Design (the user's pick out of ten layouts): one bordered panel with a
  // 3x2 grid of cells divided by internal borders, the icon inline with the
  // title, a soft 3D tilt on the cells and a hover background change.

  const features = [
    { key: 'sync', icon: RefreshCw },
    { key: 'versioned', icon: History },
    { key: 'verified', icon: Fingerprint },
    { key: 'detect', icon: ScanSearch },
    { key: 'crossplat', icon: MonitorSmartphone },
    { key: 'export', icon: FileDown }
  ];
</script>

<section class="border-t border-line">
  <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6 sm:py-24">
    <div class="reveal max-w-2xl" use:reveal>
      <p class="kicker">{$_('features.kicker')}</p>
      <h2 class="mt-3 text-balance text-3xl font-semibold text-ink sm:text-4xl">
        {$_('features.title')}
      </h2>
      <p class="mt-3 text-pretty text-ink-soft">{$_('features.subtitle')}</p>
    </div>

    <div class="mt-12 grid overflow-hidden rounded-2xl border border-line bg-surface sm:grid-cols-2 lg:grid-cols-3">
      {#each features as f, i (f.key)}
        <article
          class="reveal tilt group relative border-t border-line p-7 transition-colors hover:bg-bg max-sm:[&:nth-child(2n)]:border-l sm:[&:nth-child(-n+2)]:border-t-0 sm:[&:nth-child(2n)]:border-l lg:[&:nth-child(-n+3)]:border-t-0 lg:[&:nth-child(3n)]:border-l-0 lg:[&:nth-child(3n+1)]:border-l-0"
          use:reveal={{ delay: i * 60 }}
          use:tilt={{ max: 4 }}
        >
          <h3 class="flex items-center gap-2 font-semibold text-ink">
            <f.icon class="h-4 w-4 text-accent" />
            {$_(`features.${f.key}.title`)}
          </h3>
          <p class="mt-1.5 text-sm leading-relaxed text-ink-soft">
            {$_(`features.${f.key}.body`)}
          </p>
        </article>
      {/each}
    </div>
  </div>
</section>
