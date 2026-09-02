<script lang="ts">
  /**
   * Notifications dropdown, anchored below the bell button (top-right).
   *
   * Shows server + app notifications from the `notifications` store. When
   * empty, displays a clean placeholder. Each notification renders its `body`
   * as safe mini-markdown (see `renderMarkdown` in the store).
   *
   * See `src/lib/stores/notifications.ts` for how to push notifications and
   * how the server sends messages to users.
   */
  import { _ } from "svelte-i18n";
  import { fly } from "svelte/transition";
  import {
    Bell,
    X,
    ExternalLink,
    ShieldAlert,
    Info,
    BellOff,
    Star,
    Heart,
  } from "@lucide/svelte";

  import {
    notifications,
    dismissNotification,
    clearNotifications,
    renderMarkdown,
    type AppNotification,
  } from "../stores/notifications";

  function relativeTime(at: number): string {
    const seconds = Math.round((Date.now() - at) / 1000);
    if (seconds < 60) return $_("activity.time_just_now");
    const minutes = Math.round(seconds / 60);
    if (minutes < 60)
      return $_("activity.time_minutes_ago", { values: { count: minutes } });
    const hours = Math.round(minutes / 60);
    if (hours < 24)
      return $_("activity.time_hours_ago", { values: { count: hours } });
    const days = Math.floor(hours / 24);
    return $_("history.relative_days", { values: { count: days } });
  }

  const PRIORITY_ICON = {
    high: ShieldAlert,
    normal: Bell,
    low: Info,
  } as const;

  const PRIORITY_TINT = {
    high: "text-rose-400",
    normal: "text-emerald-400",
    low: "text-zinc-400",
  } as const;

  const PRIORITY_BORDER = {
    high: "border-l-rose-500/60",
    normal: "border-l-emerald-500/40",
    low: "border-l-zinc-700",
  } as const;

  // Button styling by icon name. `heart` gets the same solid-red treatment as
  // the support button on the website, so someone who has seen the site
  // recognises it; `star` stays quiet, because asking for a star is a small
  // favour and shouldn't shout. Anything else, including an icon name this
  // build doesn't know, renders as the neutral button.
  function actionClass(icon?: string): string {
    const base =
      "inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium transition-colors";
    if (icon === "heart") {
      return `${base} bg-red-600 text-white hover:bg-red-500`;
    }
    return `${base} bg-white/[0.06] text-zinc-300 ring-1 ring-white/10 hover:bg-white/[0.1] hover:text-zinc-100`;
  }

  // Tick once a second so relative timestamps stay honest.
  let tick = $state(0);
  $effect(() => {
    const id = setInterval(() => {
      if (!document.hidden) tick = tick + 1;
    }, 1000);
    return () => clearInterval(id);
  });
</script>

<div
  class="fixed right-3 top-14 z-[61] w-80 overflow-hidden rounded-xl border border-white/[0.08] bg-zinc-950/95 shadow-xl backdrop-blur-xl"
  transition:fly={{ y: -8, duration: 180 }}
>
  <!-- Header -->
  <div class="flex items-center justify-between border-b border-white/[0.06] px-4 py-3">
    <div class="flex items-center gap-2">
      <Bell size={15} class="text-emerald-400" />
      <span class="text-sm font-medium text-zinc-200">
        {$_("notifications.title")}
      </span>
      {#if $notifications.length > 0}
        <span class="rounded-full bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-emerald-300">
          {$notifications.length}
        </span>
      {/if}
    </div>
    {#if $notifications.length > 0}
      <button
        type="button"
        onclick={clearNotifications}
        class="text-[11px] text-zinc-500 transition-colors hover:text-zinc-300"
      >
        {$_("notifications.clear_all")}
      </button>
    {/if}
  </div>

  <!-- List or placeholder -->
  {#if $notifications.length === 0}
    <!-- Placeholder: no notifications. This is the empty state where server
         messages and important app notifications will appear. See
         stores/notifications.ts for how to push one. -->
    <div class="flex flex-col items-center gap-3 px-6 py-10 text-center">
      <div class="flex h-10 w-10 items-center justify-center rounded-full bg-zinc-800/60 text-zinc-500">
        <BellOff size={18} />
      </div>
      <p class="text-sm text-zinc-400">{$_("notifications.empty")}</p>
      <p class="max-w-[220px] text-[11px] leading-relaxed text-zinc-600">
        {$_("notifications.empty_hint")}
      </p>
    </div>
  {:else}
    <ul class="max-h-[min(50vh,24rem)] divide-y divide-white/[0.04] overflow-y-auto">
      {#each $notifications as n (n.id)}
        {@const Icon = PRIORITY_ICON[n.priority]}
        {@const _t = tick}
        <li class="flex items-start gap-3 border-l-2 px-4 py-3 {PRIORITY_BORDER[n.priority]}">
          <span class="mt-0.5 shrink-0 {PRIORITY_TINT[n.priority]}">
            <Icon size={15} />
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-start justify-between gap-2">
              <p class="text-sm font-medium text-zinc-100">{n.title}</p>
              <button
                type="button"
                onclick={() => dismissNotification(n.id)}
                class="-mr-1 -mt-1 shrink-0 rounded p-0.5 text-zinc-600 transition-colors hover:text-zinc-300"
                aria-label={$_("notifications.dismiss")}
              >
                <X size={13} />
              </button>
            </div>
            <!-- Inline mini-markdown body. {@html} is safe here: renderMarkdown
                 escapes all input before applying formatting, so no raw HTML
                 from the server can execute. -->
            <p class="mt-1 text-xs leading-relaxed text-zinc-400">
              {@html renderMarkdown(n.body)}
            </p>
            <div class="mt-2 flex items-center gap-2">
              <span class="text-[10px] text-zinc-600">
                {n.source === "server" ? $_("notifications.from_server") : $_("notifications.from_app")}
                · {relativeTime(n.at)}
              </span>
              {#if n.actions?.length ? false : n.action_url}
                <a
                  href={n.action_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="inline-flex items-center gap-1 text-[11px] font-medium text-emerald-400 hover:text-emerald-300"
                >
                  {n.action_label ?? $_("notifications.open")}
                  <ExternalLink size={11} />
                </a>
              {/if}
            </div>
            <!-- Multi-button form. The panel is 288px wide, so two buttons is
                 the practical ceiling; they wrap rather than overflow if a
                 message ever sends more. The icon comes across as a NAME and
                 is resolved here: the server never sends markup. -->
            {#if n.actions?.length}
              <div class="mt-2 flex flex-wrap gap-2">
                {#each n.actions as action (action.url)}
                  <a
                    href={action.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    class={actionClass(action.icon)}
                  >
                    {#if action.icon === "star"}
                      <Star size={12} class="shrink-0 fill-current text-amber-400" />
                    {:else if action.icon === "heart"}
                      <Heart size={12} class="shrink-0 fill-current" />
                    {/if}
                    {action.label}
                  </a>
                {/each}
              </div>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>
