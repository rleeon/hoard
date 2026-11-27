<script lang="ts">
  /**
   * The sign-in link, to finish by hand. Shown when the system could not open a
   * browser, or when one was "opened" and the sign-in never came back: on
   * DroidDeck (#115) `xdg-open` worked from a terminal and not from the app, and
   * the button simply did nothing.
   *
   * The link carries the loopback port and the nonce of the attempt in flight,
   * so it only completes on this device and while that listener is up (30 min).
   */
  import { _ } from "svelte-i18n";
  import { Globe, Copy, Terminal } from "@lucide/svelte";

  import Modal from "./Modal.svelte";
  import Button from "./Button.svelte";
  import { toastError, toastSuccess } from "../stores/toasts";

  type Props = {
    url: string | null;
    failed: boolean;
    detail: string | null;
    onClose: () => void;
  };

  let { url, failed, detail, onClose }: Props = $props();

  async function copy() {
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      toastSuccess($_("signin_manual.copied"));
    } catch (e) {
      toastError(typeof e === "string" ? e : (e as Error).message);
    }
  }
</script>

<Modal
  open={url !== null}
  title={$_(failed ? "signin_manual.title_failed" : "signin_manual.title_slow")}
  {onClose}
>
  <div class="flex justify-center">
    <span
      class="flex h-14 w-14 items-center justify-center rounded-full ring-1 {failed
        ? 'bg-amber-500/10 text-amber-400 ring-amber-500/30'
        : 'bg-emerald-500/10 text-emerald-400 ring-emerald-500/30'}"
    >
      <Globe size={26} />
    </span>
  </div>
  <p class="mt-4 text-center text-sm leading-relaxed text-zinc-200">
    {$_(failed ? "signin_manual.body_failed" : "signin_manual.body_slow")}
  </p>

  <ol class="mt-4 list-decimal space-y-1.5 pl-5 text-sm leading-relaxed text-zinc-300">
    <li>{$_("signin_manual.step_copy")}</li>
    <li>{$_("signin_manual.step_paste")}</li>
    <li>{$_("signin_manual.step_back")}</li>
  </ol>

  <div
    class="mt-3 select-all break-all rounded-xl border border-white/[0.08] bg-layer-2 p-3 font-mono text-xs leading-relaxed text-zinc-200"
  >
    {url}
  </div>
  <p class="mt-2 text-xs text-zinc-500">{$_("signin_manual.expiry")}</p>

  <p class="mt-4 flex flex-wrap items-center gap-2 text-xs text-zinc-400">
    <Terminal size={14} class="shrink-0" />
    {$_("signin_manual.cli")}
    <code class="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-zinc-200">hoard login</code>
  </p>

  {#if failed && detail}
    <p class="mt-3 break-words text-xs text-zinc-500">
      {$_("signin_manual.detail", { values: { detail } })}
    </p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>
      {$_("signin_manual.close")}
    </Button>
    <Button onclick={copy}>
      <Copy size={16} />
      {$_("signin_manual.copy")}
    </Button>
  {/snippet}
</Modal>
