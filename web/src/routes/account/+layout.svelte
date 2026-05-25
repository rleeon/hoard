<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/stores/session';

  $effect(() => {
    if ($session === null) {
      goto('/login?next=/account', { replaceState: true });
    }
  });

  interface Props {
    children: import('svelte').Snippet;
  }
  let { children }: Props = $props();
</script>

{#if $session === undefined}
  <div class="grid min-h-[60vh] place-items-center">
    <div class="h-10 w-10 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent"></div>
  </div>
{:else if $session}
  {@render children()}
{/if}
