<script lang="ts">
  // Hoard-help: a bug or an idea, with whatever files help, sent to us. Same
  // destination from a Cloud install and a self-hosted one; the line under
  // the title says so, and says what happens to it.
  //
  // Files come in three ways: the picker, dropped on the window (Tauri hands
  // over real paths, which the webview's own drop event does not), and a
  // screenshot pasted with Ctrl+V, which is written to a temp file first so
  // it travels like the rest.
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    Image as ImageIcon,
    Film,
    FileArchive,
    FileText,
    File as FileIcon,
    X,
    Send,
    Check,
  } from "@lucide/svelte";
  import {
    feedbackSend,
    feedbackDescribeFiles,
    feedbackStashImage,
    feedbackLogsPreview,
    FEEDBACK_MAX_FILE_BYTES,
    FEEDBACK_MAX_REPORT_BYTES,
    FEEDBACK_MAX_FILES,
    type FeedbackKind,
    type PickedFile,
  } from "../lib/api";
  import { fmtBytes } from "../lib/pro/lib";
  import { cloud } from "../lib/stores/cloud";
  import { auth } from "../lib/stores/auth";
  import { toastError } from "../lib/stores/toasts";
  import { formatCloudError } from "../lib/utils/cloudErrors";

  let kind = $state<FeedbackKind>("bug");
  let message = $state("");
  let contact = $state("");
  let files = $state<PickedFile[]>([]);
  let attachLogs = $state(true);

  let logsOpen = $state(false);
  let logsText = $state<string | null>(null);

  let dragging = $state(false);
  let sending = $state(false);
  let progress = $state<{ sent: number; total: number } | null>(null);
  let error = $state<string | null>(null);
  let sentId = $state<string | null>(null);
  let pasted = 0;

  const mode = $derived(
    $cloud.account ? "cloud" : $auth.user ? "selfhosted" : "none",
  ) as "cloud" | "selfhosted" | "none";
  const totalBytes = $derived(files.reduce((a, f) => a + f.size, 0));
  const pct = $derived(
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.sent / progress.total) * 100))
      : 0,
  );

  // The limits are round numbers of MiB; `fmtBytes` would print "90.0 MB".
  const mb = (n: number) => `${Math.round(n / 1024 / 1024)} MB`;

  function setKind(k: FeedbackKind) {
    kind = k;
    // Logs help with a bug and are noise on an idea; the switch is still theirs.
    attachLogs = k === "bug";
  }

  function iconFor(name: string) {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    if (["png", "jpg", "jpeg", "gif", "webp", "bmp"].includes(ext)) return ImageIcon;
    if (["mp4", "mov", "mkv", "webm", "avi"].includes(ext)) return Film;
    if (["zip", "7z", "rar", "gz", "tar", "xz", "zst"].includes(ext)) return FileArchive;
    if (["txt", "log", "json", "md", "toml", "ini", "cfg"].includes(ext)) return FileText;
    return FileIcon;
  }

  /** Adds what fits and says why the rest did not. Checked here so a 2 GB
   *  video is turned away at once, not after a minute of uploading. */
  function accept(incoming: PickedFile[]) {
    let next = [...files];
    for (const f of incoming) {
      if (next.some((x) => x.path === f.path)) continue;
      if (f.size > FEEDBACK_MAX_FILE_BYTES) {
        toastError(
          $_("help.file_too_big", {
            values: { name: f.name, max: mb(FEEDBACK_MAX_FILE_BYTES) },
          }),
        );
        continue;
      }
      if (next.length >= FEEDBACK_MAX_FILES) {
        toastError($_("help.err_too_many"));
        break;
      }
      if (next.reduce((a, x) => a + x.size, 0) + f.size > FEEDBACK_MAX_REPORT_BYTES) {
        toastError($_("help.err_total_large"));
        continue;
      }
      next.push(f);
    }
    files = next;
  }

  async function addPaths(paths: string[]) {
    if (paths.length === 0) return;
    accept(await feedbackDescribeFiles(paths));
  }

  async function pick() {
    const picked = await openDialog({ multiple: true, directory: false });
    if (!picked) return;
    await addPaths(Array.isArray(picked) ? picked : [picked]);
  }

  function remove(path: string) {
    files = files.filter((f) => f.path !== path);
  }

  function toBase64(buf: ArrayBuffer): string {
    const bytes = new Uint8Array(buf);
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) {
      bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    }
    return btoa(bin);
  }

  async function onPaste(e: ClipboardEvent) {
    if (sentId) return;
    const items = [...(e.clipboardData?.items ?? [])].filter((i) =>
      i.type.startsWith("image/"),
    );
    if (items.length === 0) return;
    e.preventDefault();
    for (const item of items) {
      const blob = item.getAsFile();
      if (!blob) continue;
      const ext = item.type.split("/")[1]?.replace("jpeg", "jpg") || "png";
      pasted += 1;
      try {
        const f = await feedbackStashImage(
          `screenshot-${pasted}.${ext}`,
          toBase64(await blob.arrayBuffer()),
        );
        accept([f]);
      } catch (err) {
        toastError(formatCloudError(err));
      }
    }
  }

  async function toggleLogs() {
    logsOpen = !logsOpen;
    if (logsOpen && logsText === null) {
      try {
        logsText = await feedbackLogsPreview();
      } catch {
        logsText = "";
      }
    }
  }

  async function send() {
    if (sending) return;
    error = null;
    if (!message.trim()) {
      error = $_("help.err_empty");
      return;
    }
    sending = true;
    progress = null;
    try {
      sentId = await feedbackSend({
        kind,
        message,
        contact: contact.trim() || null,
        mode,
        files: files.map((f) => f.path),
        attach_logs: attachLogs,
      });
    } catch (err) {
      error = formatCloudError(err);
    } finally {
      sending = false;
    }
  }

  function reset() {
    kind = "bug";
    message = "";
    contact = "";
    files = [];
    attachLogs = true;
    logsOpen = false;
    logsText = null;
    progress = null;
    error = null;
    sentId = null;
  }

  onMount(() => {
    const unlisteners: Promise<UnlistenFn>[] = [
      listen<{ sent: number; total: number }>("feedback://progress", (e) => {
        progress = e.payload;
      }),
      getCurrentWebview().onDragDropEvent((e) => {
        if (sentId || sending) return;
        const p = e.payload;
        if (p.type === "enter" || p.type === "over") dragging = true;
        else if (p.type === "leave") dragging = false;
        else if (p.type === "drop") {
          dragging = false;
          addPaths(p.paths).catch((err) => toastError(formatCloudError(err)));
        }
      }),
    ];
    return () => {
      for (const u of unlisteners) u.then((f) => f()).catch(() => {});
    };
  });
</script>

<svelte:window onpaste={onPaste} />

<div class="mx-auto max-w-5xl px-6 py-8">
  <!-- header -->
  <div class="mb-6">
    <h1 class="font-display text-2xl font-semibold tracking-tight text-zinc-50">
      {$_("help.title")}
    </h1>
    <p class="mt-0.5 max-w-3xl text-sm text-zinc-400">
      {$_("help.subtitle")}
    </p>
  </div>

  {#if sentId}
    <div
      class="flex flex-col items-center gap-3 rounded-2xl border border-emerald-400/20 bg-layer-1 px-6 py-10 text-center"
    >
      <Check size={32} class="icon-anim-pop text-emerald-300" data-anim="pop" />
      <h2 class="text-lg font-semibold text-zinc-50">{$_("help.sent_title")}</h2>
      <p class="max-w-md text-sm text-zinc-400">{$_("help.sent_body")}</p>
      <button
        type="button"
        onclick={reset}
        class="mt-2 rounded-2xl border border-white/[0.08] bg-layer-2 px-4 py-2 text-sm font-medium text-zinc-300 transition hover:border-emerald-400/30"
      >
        {$_("help.send_another")}
      </button>
    </div>
  {:else}
    <!-- what it is about -->
    <div class="rounded-2xl border border-white/[0.08] bg-layer-1 p-4">
      <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h2 class="text-sm font-semibold text-zinc-100">{$_("help.message_label")}</h2>
        <div class="flex gap-1 rounded-lg border border-white/[0.08] bg-layer-2 p-1">
          {#each [["bug", "help.kind_bug"], ["idea", "help.kind_idea"]] as const as [k, label] (k)}
            <button
              type="button"
              onclick={() => setKind(k)}
              aria-pressed={kind === k}
              class="rounded-md px-2.5 py-1 text-xs font-medium ring-1 ring-inset transition {kind ===
              k
                ? 'bg-emerald-600/20 text-emerald-300 ring-emerald-600/40'
                : 'text-zinc-400 ring-transparent hover:ring-white/15'}"
            >
              {$_(label)}
            </button>
          {/each}
        </div>
      </div>

      <textarea
        bind:value={message}
        disabled={sending}
        rows="7"
        maxlength="20000"
        placeholder={$_(kind === "bug" ? "help.placeholder_bug" : "help.placeholder_idea")}
        class="block w-full resize-y rounded-2xl border border-white/[0.08] bg-layer-2 px-3.5 py-3 text-sm leading-relaxed text-zinc-100 placeholder:text-zinc-500 focus:border-emerald-400/40 focus:outline-none"
      ></textarea>

      <label class="mt-4 block">
        <span class="text-xs font-medium text-zinc-300">{$_("help.contact_label")}</span>
        <input
          type="text"
          bind:value={contact}
          disabled={sending}
          maxlength="200"
          placeholder={$_("help.contact_placeholder")}
          class="mt-1.5 block w-full rounded-2xl border border-white/[0.08] bg-layer-2 px-3.5 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 focus:border-emerald-400/40 focus:outline-none"
        />
      </label>
      {#if mode === "cloud"}
        <p class="mt-1.5 text-[11px] text-zinc-500">{$_("help.contact_hint_cloud")}</p>
      {/if}
    </div>

    <!-- files -->
    <div class="mt-4 rounded-2xl border border-white/[0.08] bg-layer-1 p-4">
      <div class="mb-3 flex items-end justify-between gap-3">
        <h2 class="text-sm font-semibold text-zinc-100">{$_("help.files_title")}</h2>
        {#if files.length > 0}
          <span class="text-[11px] tabular-nums text-zinc-500">
            {fmtBytes(totalBytes)} / {mb(FEEDBACK_MAX_REPORT_BYTES)}
          </span>
        {/if}
      </div>

      <div
        class="rounded-2xl border border-dashed bg-layer-2 px-4 py-5 text-center transition {dragging
          ? 'border-emerald-400/60'
          : 'border-white/[0.12]'}"
      >
        <p class="text-xs text-zinc-400">
          {$_("help.files_hint")}
          <button
            type="button"
            onclick={pick}
            class="font-medium text-emerald-300 underline decoration-emerald-300/30 underline-offset-2 transition hover:decoration-emerald-300"
          >
            {$_("help.files_pick")}</button
          >
        </p>
        <p class="mt-1.5 text-[11px] text-zinc-500">
          {$_("help.files_limits", {
            values: {
              files: FEEDBACK_MAX_FILES,
              per: mb(FEEDBACK_MAX_FILE_BYTES),
              total: mb(FEEDBACK_MAX_REPORT_BYTES),
            },
          })}
        </p>
      </div>

      {#if files.length > 0}
        <ul class="mt-3 space-y-2">
          {#each files as f (f.path)}
            {@const Icon = iconFor(f.name)}
            <li
              class="flex items-center gap-3 rounded-2xl border border-white/[0.08] bg-layer-2 p-2 transition hover:border-emerald-400/25"
            >
              <Icon size={16} class="ml-1 shrink-0 text-zinc-500" />
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm text-zinc-100" title={f.path}>{f.name}</div>
                <div class="text-[11px] tabular-nums text-zinc-500">{fmtBytes(f.size)}</div>
              </div>
              <button
                type="button"
                onclick={() => remove(f.path)}
                disabled={sending}
                class="grid h-7 w-7 shrink-0 place-items-center rounded-lg border border-white/[0.08] text-zinc-400 transition hover:border-white/25 disabled:opacity-40"
                aria-label={$_("help.remove_file")}
                title={$_("help.remove_file")}
              >
                <X size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      <!-- logs -->
      <div class="mt-4 border-t border-white/[0.08] pt-4">
        <div class="flex items-start gap-3">
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-zinc-100">{$_("help.logs_label")}</p>
            <p class="mt-0.5 text-xs leading-relaxed text-zinc-500">
              {$_("help.logs_hint")}
              <button
                type="button"
                onclick={toggleLogs}
                class="font-medium text-emerald-300 underline decoration-emerald-300/30 underline-offset-2 transition hover:decoration-emerald-300"
              >
                {logsOpen ? $_("help.logs_hide") : $_("help.logs_show")}
              </button>
            </p>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={attachLogs}
            aria-label={$_("help.logs_label")}
            onclick={() => (attachLogs = !attachLogs)}
            class="mt-0.5 flex h-6 w-11 shrink-0 items-center rounded-full p-0.5 transition-colors {attachLogs
              ? 'bg-emerald-600'
              : 'bg-zinc-700'}"
          >
            <span
              class="h-5 w-5 rounded-full bg-white transition-transform {attachLogs
                ? 'translate-x-5'
                : ''}"
            ></span>
          </button>
        </div>
        {#if logsOpen}
          <pre
            class="logs mt-3 max-h-72 overflow-auto rounded-xl border border-white/[0.08] bg-black p-3 font-mono text-[11px] leading-relaxed text-zinc-400">{logsText ===
            null
              ? "…"
              : logsText.trim() || $_("help.logs_empty")}</pre>
        {/if}
      </div>
    </div>

    {#if error}
      <div
        class="mt-4 rounded-2xl border border-red-500/40 bg-layer-2 px-3 py-2 text-xs text-red-300"
      >
        {error}
      </div>
    {/if}

    <!-- send: the same long button as the Wrapped card's -->
    <button
      type="button"
      onclick={send}
      disabled={sending}
      class="relative mt-4 flex w-full items-center justify-center gap-2.5 overflow-hidden rounded-2xl border px-4 py-3.5 transition {sending
        ? 'border-emerald-400/40 bg-layer-2 text-zinc-300'
        : 'border-white/[0.08] bg-layer-2 text-zinc-300 hover:border-emerald-400/30'}"
    >
      {#if sending && progress && progress.total > 0}
        <span
          class="absolute inset-y-0 left-0 bg-emerald-500/15 transition-[width] duration-200"
          style="width:{pct}%"
        ></span>
      {/if}
      <Send size={17} class="relative text-emerald-300" data-anim="pop" />
      <span class="relative text-sm font-medium">
        {#if sending}
          {$_("help.sending")}
          {#if progress && progress.total > 0}<span class="tabular-nums text-zinc-500"
              >&nbsp;{pct}%</span
            >{/if}
        {:else}
          {$_("help.send")}
        {/if}
      </span>
    </button>
  {/if}
</div>

<style>
  .logs {
    white-space: pre-wrap;
    word-break: break-all;
    scrollbar-width: thin;
    scrollbar-color: rgba(113, 113, 122, 0.5) transparent;
  }
</style>
