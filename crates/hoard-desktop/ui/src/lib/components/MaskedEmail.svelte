<script lang="ts">
  /**
   * An email address, masked until asked: every character but the `@` and the
   * dots becomes a bullet, the same shape the Wrapped card uses, so the address
   * keeps its outline and nothing else. The eye beside it flips every masked
   * address in the app at once (`stores/privacy.ts`).
   */
  import { Eye, EyeOff } from "@lucide/svelte";
  import { _ } from "svelte-i18n";

  import { emailRevealed } from "../stores/privacy";

  /** `toggle={false}` drops the eye, for when the address sits inside something
   *  that is already a button (the sidebar's account link): a button inside a
   *  button is invalid HTML and the click would go to the wrong one. It still
   *  follows the shared flag, so revealing it elsewhere reveals it here. */
  type Props = { email: string; toggle?: boolean; class?: string };
  let { email, toggle: withToggle = true, class: klass = "" }: Props = $props();

  const masked = $derived(email.replace(/[^@.]/g, "\u2022"));

  function toggle(e: MouseEvent) {
    // It often sits inside something clickable (a card, a link to the account),
    // and revealing the address must not also navigate.
    e.stopPropagation();
    e.preventDefault();
    emailRevealed.update((v) => !v);
  }
</script>

<span class="inline-flex min-w-0 max-w-full items-center gap-1.5 align-middle {klass}">
  <span class="truncate {$emailRevealed ? '' : 'select-none'}">
    {$emailRevealed ? email : masked}
  </span>
  {#if withToggle}
  <button
    type="button"
    onclick={toggle}
    class="grid h-4 w-4 shrink-0 place-items-center rounded text-zinc-500 transition hover:text-zinc-200"
    aria-label={$emailRevealed ? $_("common.hide_email") : $_("common.show_email")}
    title={$emailRevealed ? $_("common.hide_email") : $_("common.show_email")}
  >
    {#if $emailRevealed}
      <EyeOff size={11} data-anim="pop" />
    {:else}
      <Eye size={11} data-anim="pop" />
    {/if}
  </button>
  {/if}
</span>
