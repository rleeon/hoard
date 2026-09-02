<script lang="ts">
  import { push } from "svelte-spa-router";
  import { fly } from "svelte/transition";
  import { ArrowRight, Server, CheckCircle2 } from "@lucide/svelte";
  import { _ } from "svelte-i18n";
  import Button from "../lib/components/Button.svelte";
  import Input from "../lib/components/Input.svelte";
  import WizardShell from "../lib/components/WizardShell.svelte";
  import * as api from "../lib/api";
  import { toastError } from "../lib/stores/toasts";
  import { loadUrl, saveStep, saveUrl } from "../lib/stores/onboarding";
  import { auth } from "../lib/stores/auth";
  import { onMount } from "svelte";

  let url = $state("");
  let loading = $state(false);
  let inlineError = $state<string | null>(null);
  let healthy = $state<{ version: string } | null>(null);

  onMount(async () => {
    url = await loadUrl();
  });

  // Accept a bare hostname ("ubserver", "192.168.1.20:12421") by prepending a
  // scheme so the user doesn't have to remember it. Pick the scheme from the
  // host: a local/private address (a self-hosted server with no reverse proxy)
  // speaks plain HTTP, while a public domain is assumed to sit behind TLS.
  // This avoids the classic trap of turning "myserver:12421" into an https://
  // URL that a no-TLS LAN box can't answer, the #1 "can't reach" cause.
  // Returns `null` when the string can't be a host at all (contains spaces,
  // e.g. someone typed a friendly name like "mi servidor").
  function normalizeUrl(raw: string): string | null {
    const s = raw.trim();
    if (!s) return null;
    let withScheme: string;
    if (s.startsWith("http://") || s.startsWith("https://")) {
      withScheme = s;
    } else {
      const host = s.replace(/[:/].*$/, "");
      withScheme = `${isLocalHost(host) ? "http" : "https"}://${s}`;
    }
    // A real host has no whitespace. Reject early; the address field is for a
    // URL, not a label.
    if (/\s/.test(withScheme.replace(/^https?:\/\//, ""))) return null;
    return withScheme;
  }

  // localhost, an *.local / single-label LAN name, or a loopback / RFC1918 /
  // Tailscale-CGNAT IP → a no-TLS server we should reach over http://.
  function isLocalHost(host: string): boolean {
    const h = host.toLowerCase();
    if (h === "localhost" || h.endsWith(".local")) return true;
    if (!h.includes(".")) return true; // bare hostname, e.g. "ubserver"
    const m = h.match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.\d{1,3}$/);
    if (m) {
      const a = Number(m[1]);
      const b = Number(m[2]);
      if (a === 127 || a === 10) return true;
      if (a === 192 && b === 168) return true;
      if (a === 172 && b >= 16 && b <= 31) return true;
      if (a === 100 && b >= 64 && b <= 127) return true; // Tailscale CGNAT
    }
    return false;
  }

  async function testConnection() {
    inlineError = null;
    healthy = null;
    const normalized = normalizeUrl(url);
    if (!normalized) {
      inlineError = $_("server.invalid_url");
      return;
    }
    // Reflect the normalized form back into the field so the user sees the
    // scheme we'll actually use.
    url = normalized;
    loading = true;
    try {
      const info = await api.healthCheck(normalized);
      healthy = { version: info.version };
      await saveUrl(normalized);
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error).message;
      inlineError = msg;
      toastError(msg);
    } finally {
      loading = false;
    }
  }

  async function next() {
    if (!healthy) {
      await testConnection();
      if (!healthy) return;
    }
    await saveStep("token");
    push("/onboarding/token");
  }

  // Back navigation. A user who already has an active session (they came here
  // from "connect to your server" inside the app) must not get trapped in the
  // wizard, send them back to the account page. A brand-new user goes to the
  // welcome step as before.
  function back() {
    if ($auth.user) {
      push("/account");
    } else {
      push("/onboarding/choose");
    }
  }

  // Reset the green badge as soon as the user edits the URL again.
  $effect(() => {
    void url;
    healthy = null;
  });
</script>

<WizardShell step="server" onBack={back}>
  <div in:fly={{ x: 24, duration: 220 }}>
    <h1 class="text-xl font-semibold tracking-tight text-zinc-50">
      {$_("server.title")}
    </h1>
    <p class="mt-2 text-sm text-zinc-400">
      {$_("server.subtitle")}
    </p>

    <form
      class="mt-6 space-y-4"
      onsubmit={(e) => {
        e.preventDefault();
        void next();
      }}
    >
      <Input
        label={$_("server.address_label")}
        bind:value={url}
        placeholder="http://192.168.1.20:12421"
        hint={$_("server.address_hint")}
        icon={Server}
        autocomplete="url"
        spellcheck={false}
        autocapitalize="off"
        error={inlineError}
      />

      <div class="flex flex-wrap items-center gap-3">
        <Button
          variant="secondary"
          onclick={testConnection}
          {loading}
          disabled={!url.trim()}
        >
          {$_("server.test_connection")}
        </Button>
        {#if healthy}
          <span
            class="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300"
          >
            <CheckCircle2 size={14} />
            {$_("server.reached_server", { values: { version: healthy.version } })}
          </span>
        {/if}
      </div>

      <div class="flex justify-end pt-2">
        <Button
          type="submit"
          variant="primary"
          size="lg"
          disabled={!healthy || loading}
        >
          {$_("common.continue")}
          <ArrowRight size={16} />
        </Button>
      </div>
    </form>
  </div>
</WizardShell>
