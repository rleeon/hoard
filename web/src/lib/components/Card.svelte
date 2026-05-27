<script lang="ts">
  interface Props {
    title?: string;
    subtitle?: string;
    children: import('svelte').Snippet;
    actions?: import('svelte').Snippet;
    class?: string;
  }
  let { title, subtitle, children, actions, class: extra = '' }: Props = $props();
</script>

<section
  class="card-edge rounded-2xl border border-white/[0.06] bg-white/[0.025] p-6 backdrop-blur-sm transition-colors hover:border-white/[0.10] {extra}"
>
  {#if title || actions}
    <header class="mb-4 flex items-start justify-between gap-4">
      <div>
        {#if title}
          <h2 class="text-lg font-semibold tracking-tight text-zinc-100">{title}</h2>
        {/if}
        {#if subtitle}
          <p class="mt-0.5 text-sm text-zinc-400">{subtitle}</p>
        {/if}
      </div>
      {#if actions}
        <div class="flex flex-wrap items-center gap-2">{@render actions()}</div>
      {/if}
    </header>
  {/if}
  {@render children()}
</section>
