<script lang="ts">
  import { _ } from 'svelte-i18n';
  import LogoMark from './LogoMark.svelte';
  let year = new Date().getFullYear();
</script>

<footer class="border-t border-zinc-900 bg-zinc-950">
  <div class="mx-auto grid max-w-6xl gap-10 px-4 py-12 sm:px-6 md:grid-cols-4">
    <div>
      <div class="flex items-center gap-2.5">
        <LogoMark size={26} />
        <span class="text-base font-semibold text-white">Hoard</span>
      </div>
      <p class="mt-3 max-w-xs text-sm text-zinc-400">{$_('footer.tagline')}</p>
    </div>

    <div>
      <h3 class="text-sm font-semibold text-zinc-200">{$_('footer.product')}</h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li><a class="hover:text-white" href="/pricing">{$_('nav.pricing')}</a></li>
        <li>
          <a class="hover:text-white" href="https://github.com/rleeon/hoard/releases/latest"
            >{$_('nav.get_app')}</a
          >
        </li>
        <li>
          <a
            class="hover:text-white"
            href="https://github.com/rleeon/hoard/blob/main/CHANGELOG.md">{$_('footer.changelog')}</a
          >
        </li>
        <li><a class="hover:text-white" href="/help">{$_('footer.status')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="text-sm font-semibold text-zinc-200">{$_('footer.company')}</h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li>
          <a class="hover:text-white" href="https://github.com/rleeon/hoard">{$_('footer.github')}</a>
        </li>
        <li><a class="hover:text-white" href="/help">{$_('footer.contact')}</a></li>
      </ul>
    </div>

    <div>
      <h3 class="text-sm font-semibold text-zinc-200">{$_('footer.legal')}</h3>
      <ul class="mt-3 space-y-2 text-sm text-zinc-400">
        <li><a class="hover:text-white" href="/legal/terms">{$_('footer.terms')}</a></li>
        <li><a class="hover:text-white" href="/legal/privacy">{$_('footer.privacy')}</a></li>
      </ul>
    </div>
  </div>
  <div class="border-t border-zinc-900">
    <p class="mx-auto max-w-6xl px-4 py-4 text-xs text-zinc-500 sm:px-6">
      {$_('footer.copyright', { values: { year } })}
    </p>
  </div>
</footer>
