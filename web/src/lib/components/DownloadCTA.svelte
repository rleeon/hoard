<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from './Button.svelte';
  import { onMount } from 'svelte';
  import { ArrowRight, Github } from 'lucide-svelte';

  type Platform = 'Windows' | 'macOS' | 'Linux';
  let label = $state<Platform | null>(null);

  onMount(() => {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) label = 'Windows';
    else if (ua.includes('mac')) label = 'macOS';
    else if (ua.includes('linux')) label = 'Linux';
  });

  let cta = $derived(
    label
      ? $_('cta_section.cta_os', { values: { platform: label } })
      : $_('cta_section.cta')
  );
</script>

<div class="flex flex-col items-start gap-3 sm:flex-row sm:items-center">
  <Button href="/download" size="lg" variant="primary">
    {cta}
    <ArrowRight class="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
  </Button>
  <span class="text-sm text-zinc-500">
    {$_('cta_section.subnote')}
    <a
      href="https://github.com/rleeon/hoard/releases"
      target="_blank"
      rel="noreferrer noopener"
      class="link-underline inline-flex items-center gap-1 text-emerald-400 hover:text-emerald-300"
    >
      <Github class="h-3.5 w-3.5" />
      GitHub Releases
    </a>
  </span>
</div>
