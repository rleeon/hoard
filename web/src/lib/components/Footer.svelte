<script lang="ts">
  import { _ } from 'svelte-i18n';
  import LogoMark from './LogoMark.svelte';
  import StatusDot from './StatusDot.svelte';
  const year = new Date().getFullYear();
  const VERSION = '1.7.0';
</script>

<footer class="relative mt-auto border-t border-white/[0.06]">
  <div
    class="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-emerald-500/40 to-transparent"
    aria-hidden="true"
  ></div>
  <div
    class="pointer-events-none absolute inset-x-0 -top-6 h-12 bg-[radial-gradient(50%_60%_at_50%_100%,rgba(16,185,129,0.16),transparent_70%)]"
    aria-hidden="true"
  ></div>

  <div class="mx-auto grid max-w-6xl gap-10 px-4 py-14 sm:px-6 grid-cols-2 md:grid-cols-4">
    <div class="col-span-2 md:col-span-1">
      <div class="flex items-center gap-2.5">
        <LogoMark size={26} />
        <span class="text-base font-semibold tracking-tight text-white">Hoard</span>
        <span class="ml-1 rounded-full border border-white/10 bg-white/[0.04] px-2 py-0.5 font-mono text-[10px] tracking-wider text-zinc-400">
          {$_('footer.version_label', { values: { v: VERSION } })}
        </span>
      </div>
      <p class="mt-3 max-w-xs text-sm leading-relaxed text-zinc-400">{$_('footer.tagline')}</p>
      <div class="mt-4">
        <StatusDot />
      </div>
    </div>

    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zinc-300">
        {$_('footer.product')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li><a class="link-underline hover:text-white" href="/pricing">{$_('nav.pricing')}</a></li>
        <li>
          <a class="link-underline hover:text-white" href="/download">
            {$_('nav.download')}
          </a>
        </li>
        <li>
          <a
            class="link-underline hover:text-white"
            href="https://github.com/rleeon/hoard/blob/main/CHANGELOG.md"
            rel="noreferrer"
          >
            {$_('footer.changelog')}
          </a>
        </li>
        <li><a class="link-underline hover:text-white" href="/help">{$_('footer.status')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zinc-300">
        {$_('footer.company')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li>
          <a class="link-underline hover:text-white" href="https://github.com/rleeon/hoard" rel="noreferrer">
            {$_('footer.github')}
          </a>
        </li>
        <li><a class="link-underline hover:text-white" href="/help">{$_('footer.contact')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="text-xs font-semibold uppercase tracking-wider text-zinc-300">
        {$_('footer.legal')}
      </h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li><a class="link-underline hover:text-white" href="/legal/terms">{$_('footer.terms')}</a></li>
        <li><a class="link-underline hover:text-white" href="/legal/privacy">{$_('footer.privacy')}</a></li>
      </ul>
    </div>
  </div>
  <div class="border-t border-white/[0.05] bg-zinc-950/30">
    <p class="mx-auto max-w-6xl px-4 py-4 text-xs text-zinc-500 sm:px-6">
      {$_('footer.copyright', { values: { year } })}
    </p>
  </div>
</footer>
