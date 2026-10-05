<script lang="ts">
  import { _ } from 'svelte-i18n';
  import Button from '$lib/components/Button.svelte';
  import Seo from '$lib/components/Seo.svelte';
  import { reveal } from '$lib/actions/reveal';
  import { onMount } from 'svelte';
  import { Apple, Github, Monitor, Copy, Check } from 'lucide-svelte';
  import { version, release, ALL_RELEASES, CHANGELOG_URL, RELEASES_LATEST } from '$lib/version';

  type Platform = 'windows' | 'macos' | 'linux';
  type Arch = 'x64' | 'arm64';
  let detected = $state<Platform | null>(null);
  let detectedArch = $state<Arch | null>(null);

  type Asset = {
    label: string;
    sublabel: string;
    href: string;
    /** Left off where the platform ships a single build and there is no
     *  choice to get wrong (the macOS dmg is Apple Silicon, full stop). */
    arch?: Arch;
    /** Hoard Setup rather than a raw package. Listed first, and what the big
     *  button points at: it works out which package this machine wants, which
     *  is the question the rest of this page is asking the visitor to answer
     *  for themselves. */
    setup?: boolean;
  };

  const fileName = (url: string) => url.split('/').pop() ?? url;

  /** One row, or none at all: a release that does not list the file has no
   *  business offering a link to it. This is how the ARM bundles and Hoard
   *  Setup can be listed here before every published release carries them,
   *  they appear on the releases that have them and nowhere else. */
  const row = (
    href: string | null,
    sublabel: string,
    extra: { arch?: Arch; setup?: boolean } = {}
  ): Asset[] => (href ? [{ label: fileName(href), sublabel, href, ...extra }] : []);

  // Direct links to the installers of the latest release, clicking starts
  // the download. URLs come from the `release` store, so they follow GitHub
  // automatically when a new version ships.
  let downloads = $derived<Record<Platform, { name: string; assets: Asset[] }>>({
    windows: {
      name: 'Windows',
      assets: [
        ...row($release.assets.setupWindows, 'Installer · Windows 10/11 · x64', {
          arch: 'x64',
          setup: true
        }),
        ...row($release.assets.setupWindowsArm64, 'Installer · Windows 11 · ARM64', {
          arch: 'arm64',
          setup: true
        }),
        ...row($release.assets.windowsSetup, 'Windows 10/11 · x64', { arch: 'x64' }),
        ...row($release.assets.windowsSetupArm64, 'Windows 11 · ARM64', { arch: 'arm64' }),
        ...row($release.assets.windowsMsi, 'Windows 10/11 · x64 · MSI', { arch: 'x64' })
      ]
    },
    macos: {
      name: 'macOS',
      assets: [
        ...row($release.assets.setupMacos, 'Installer · Apple Silicon', { setup: true }),
        ...row($release.assets.macosDmg, 'macOS 12+ · Apple Silicon')
      ]
    },
    linux: {
      name: 'Linux',
      assets: [
        ...row($release.assets.setupLinux, 'Installer · x64 · chmod +x and run', {
          arch: 'x64',
          setup: true
        }),
        ...row($release.assets.setupLinuxArm64, 'Installer · ARM64 · chmod +x and run', {
          arch: 'arm64',
          setup: true
        }),
        ...row($release.assets.linuxDeb, 'Debian / Ubuntu · x64', { arch: 'x64' }),
        ...row($release.assets.linuxAppImage, 'Universal · x64', { arch: 'x64' }),
        ...row($release.assets.linuxRpm, 'Fedora / openSUSE · x64', { arch: 'x64' }),
        ...row($release.assets.linuxDebArm64, 'Debian / Ubuntu · ARM64', { arch: 'arm64' }),
        ...row($release.assets.linuxAppImageArm64, 'Universal · ARM64', { arch: 'arm64' }),
        ...row($release.assets.linuxRpmArm64, 'Fedora / openSUSE · ARM64', { arch: 'arm64' })
      ]
    }
  });

  // Same one-liner as /cli: the raw repo copy, so it still works when
  // hoard.services itself is unreachable.
  const SH_CMD = 'curl -fsSL https://raw.githubusercontent.com/rleeon/hoard/main/web/static/install.sh | sh';

  type Cmd = {
    id: string;
    title: string;
    /** i18n key of the line under the title. */
    note?: string;
    /** Left off when there is nothing to run yet; the note says why. */
    code?: string;
    recommended?: boolean;
  };

  /** The ARM build when we guessed ARM and the release has one, x64 otherwise.
   *  A wrong guess here is a package manager refusing the architecture, loud
   *  and obvious, not a broken install. */
  const forArch = (x64: string | null, arm64: string | null) =>
    (detectedArch === 'arm64' ? arm64 : null) ?? x64;

  // Hoard Setup and the Flatpak carry no version in their names, so
  // `latest/download` reaches them without asking the GitHub API: the commands
  // that matter most are in the prerendered page and survive a rate limit.
  const LATEST = `${RELEASES_LATEST}/download`;
  // `uname -m` says x86_64 or aarch64 on Linux, the same suffixes CI uses.
  const SETUP_LINUX = `${LATEST}/HoardSetup-$(uname -m)`;

  let setupWindows = $derived(
    forArch(`${LATEST}/HoardSetup-x86_64.exe`, `${LATEST}/HoardSetup-aarch64.exe`)
  );

  // Hoard Setup first and recommended everywhere: it knows which package this
  // machine wants, and it is the one way out that also removes the service.
  // The Windows commands are PowerShell, and `curl.exe` on purpose: plain
  // `curl` is an alias of Invoke-WebRequest in Windows PowerShell, whose
  // progress bar makes a large download crawl. Setup is a windowed program, so
  // `-Wait` is what holds the prompt, and its silent mode reports through the
  // log rather than the console.
  let installCmds = $derived<Record<Platform, Cmd[]>>({
    windows: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        note: 'download.m_setup_note',
        recommended: true,
        code: `curl.exe -fLo "$env:TEMP\\HoardSetup.exe" ${setupWindows}\nStart-Process "$env:TEMP\\HoardSetup.exe" -ArgumentList '--silent' -Wait\nGet-Content "$env:TEMP\\hoard-setup.log"`
      },
      {
        id: 'nsis',
        title: 'NSIS · .exe',
        note: 'download.m_nsis_note',
        code: `curl.exe -fLo "$env:TEMP\\Hoard-setup.exe" ${forArch($release.assets.windowsSetup, $release.assets.windowsSetupArm64)}\nStart-Process "$env:TEMP\\Hoard-setup.exe" -ArgumentList '/S' -Wait`
      }
    ],
    macos: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        note: 'download.m_setup_note',
        recommended: true,
        code: `curl -fLo /tmp/HoardSetup.zip ${LATEST}/HoardSetup-aarch64.zip\nditto -xk /tmp/HoardSetup.zip /tmp\n"/tmp/Hoard Setup.app/Contents/MacOS/HoardSetup" --silent`
      },
      {
        id: 'dmg',
        title: 'DMG',
        note: 'download.m_dmg_note',
        // A fixed mount point, so the commands don't depend on the volume name.
        code: `curl -fLo /tmp/Hoard.dmg ${$release.assets.macosDmg}\nhdiutil attach -nobrowse -mountpoint /tmp/hoard-dmg /tmp/Hoard.dmg\ncp -R /tmp/hoard-dmg/Hoard.app /Applications/\nhdiutil detach /tmp/hoard-dmg`
      }
    ],
    linux: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        note: 'download.m_setup_note',
        recommended: true,
        code: `curl -fLo /tmp/HoardSetup ${SETUP_LINUX}\nchmod +x /tmp/HoardSetup\n/tmp/HoardSetup --silent`
      },
      {
        id: 'apt',
        title: 'Debian / Ubuntu · apt',
        // apt cannot install from a URL. wget because a stock Ubuntu desktop
        // ships it and not curl.
        code: `wget -O /tmp/hoard.deb ${forArch($release.assets.linuxDeb, $release.assets.linuxDebArm64)}\nsudo apt install /tmp/hoard.deb`
      },
      {
        id: 'dnf',
        title: 'Fedora · dnf',
        code: `sudo dnf install ${forArch($release.assets.linuxRpm, $release.assets.linuxRpmArm64)}`
      },
      {
        id: 'zypper',
        title: 'openSUSE · zypper',
        // The rpm is signed with minisign, not GPG, and zypper stops to ask.
        code: `sudo zypper install --allow-unsigned-rpm ${forArch($release.assets.linuxRpm, $release.assets.linuxRpmArm64)}`
      },
      // Built and packaged (flatpak/), not on Flathub yet and no bundle in the
      // releases, so there is nothing a command could point at.
      { id: 'flatpak', title: 'Flatpak · Flathub', note: 'download.m_flatpak_soon' },
      {
        id: 'appimage',
        title: 'AppImage',
        note: 'download.m_appimage_note',
        code: `mkdir -p ~/Applications\ncurl -fLo ~/Applications/Hoard.AppImage ${forArch($release.assets.linuxAppImage, $release.assets.linuxAppImageArm64)}\nchmod +x ~/Applications/Hoard.AppImage`
      },
      // Setup needs libfontconfig, which a bare server or NAS may not have; the
      // script needs nothing but a shell and stops at the engine there.
      { id: 'sh', title: $_('download.m_script'), note: 'download.m_script_note', code: SH_CMD }
    ]
  });

  // Removing only the package leaves `hoardd` running, and on Windows the
  // updater inside it then installs the app again, hence `sync stop` before
  // every manual route.
  let uninstallCmds = $derived<Record<Platform, Cmd[]>>({
    windows: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        note: 'download.u_setup_note',
        recommended: true,
        code: `curl.exe -fLo "$env:TEMP\\HoardSetup.exe" ${setupWindows}\nStart-Process "$env:TEMP\\HoardSetup.exe" -ArgumentList '--uninstall' -Wait\nGet-Content "$env:TEMP\\hoard-setup.log"`
      },
      {
        id: 'nsis',
        title: 'NSIS · .exe',
        note: 'download.u_nsis_note',
        code: `& "$env:LOCALAPPDATA\\Hoard\\hoard.exe" sync stop\n& "$env:LOCALAPPDATA\\Hoard\\uninstall.exe" /S`
      }
    ],
    macos: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        note: 'download.u_setup_note',
        recommended: true,
        code: `curl -fLo /tmp/HoardSetup.zip ${LATEST}/HoardSetup-aarch64.zip\nditto -xk /tmp/HoardSetup.zip /tmp\n"/tmp/Hoard Setup.app/Contents/MacOS/HoardSetup" --uninstall`
      },
      {
        id: 'app',
        title: 'Hoard.app',
        note: 'download.u_stop_note',
        code: '/Applications/Hoard.app/Contents/MacOS/hoard sync stop\nrm -rf /Applications/Hoard.app'
      }
    ],
    linux: [
      {
        id: 'setup',
        title: 'Hoard Setup',
        // Setup refuses a Flatpak (`Delivery::Managed`): the sandbox belongs to
        // flatpak, so that one has its own row.
        note: 'download.u_setup_note_linux',
        recommended: true,
        code: `curl -fLo /tmp/HoardSetup ${SETUP_LINUX}\nchmod +x /tmp/HoardSetup\n/tmp/HoardSetup --uninstall`
      },
      { id: 'apt', title: 'Debian / Ubuntu · apt', note: 'download.u_stop_note', code: 'hoard sync stop\nsudo apt remove hoard' },
      { id: 'dnf', title: 'Fedora · dnf', note: 'download.u_stop_note', code: 'hoard sync stop\nsudo dnf remove hoard' },
      { id: 'zypper', title: 'openSUSE · zypper', note: 'download.u_stop_note', code: 'hoard sync stop\nsudo zypper remove hoard' },
      {
        id: 'flatpak',
        title: 'Flatpak',
        note: 'download.u_stop_note',
        code: 'flatpak run --command=hoard services.hoard.saves sync stop\nflatpak uninstall services.hoard.saves'
      },
      { id: 'appimage', title: 'AppImage', note: 'download.u_appimage_note', code: 'rm ~/Applications/Hoard.AppImage' }
    ]
  });

  let installOs = $state<Platform>('windows');
  let uninstallOs = $state<Platform>('windows');

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = id;
      setTimeout(() => {
        if (copied === id) copied = null;
      }, 2000);
    } catch {
      /* clipboard blocked, the command is still visible to select manually */
    }
  }

  onMount(() => {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) detected = 'windows';
    else if (ua.includes('mac')) detected = 'macos';
    else if (ua.includes('linux')) detected = 'linux';
    if (detected) installOs = uninstallOs = detected;

    // Architecture, best effort. The user-agent string is no help, Windows on
    // ARM reports x86_64 on purpose, for compatibility, so we ask Client
    // Hints, which only Chromium answers, and stay on x64 when nobody does.
    // Getting it wrong costs an ARM visitor one click, not a broken download:
    // every build stays listed below whatever we guess.
    const uaData = (navigator as unknown as { userAgentData?: UserAgentDataLike }).userAgentData;
    uaData
      ?.getHighEntropyValues(['architecture'])
      .then((v) => {
        if (v.architecture === 'arm') detectedArch = 'arm64';
        else if (v.architecture) detectedArch = 'x64';
      })
      .catch(() => {});
  });

  type UserAgentDataLike = {
    getHighEntropyValues(hints: string[]): Promise<{ architecture?: string }>;
  };

  /** The download the big button points at.
   *
   *  Hoard Setup, wherever there is one: it resolves the right package for the
   *  machine it lands on, so it is right even when our architecture guess is
   *  not. Falling back to picking a bundle by architecture keeps the old
   *  behaviour for a release published before the installer existed. */
  let primary = $derived(
    detected && downloads[detected].assets.length
      ? (downloads[detected].assets.find((a) => a.setup && a.arch === detectedArch) ??
        // Platforms with a single build (macOS) tag no architecture at all.
        downloads[detected].assets.find((a) => a.setup && !a.arch) ??
        downloads[detected].assets.find((a) => a.arch === detectedArch) ??
        downloads[detected].assets[0])
      : null
  );

  const order: Platform[] = ['windows', 'macos', 'linux'];
</script>

<Seo path="/download" key="download" />

<section class="mx-auto max-w-5xl px-4 pb-16 pt-16 sm:px-6 sm:pb-24 sm:pt-24 2xl:pt-12">
  <div class="mx-auto max-w-2xl text-center">
    <h1 class="text-balance text-4xl font-semibold text-ink sm:text-5xl">
      {$_('download.title')}
    </h1>
    <p class="mt-4 text-pretty leading-relaxed text-ink-soft">
      {$_('download.subtitle')}
    </p>
    <p class="mt-3 font-mono text-xs text-ink-faint">
      {$_('download.version', { values: { v: $release.v, date: $release.date } })}
    </p>
  </div>

  {#if detected}
    {@const cta = primary ?? downloads[detected].assets[0]}
    <div class="reveal mt-10 flex flex-col items-center gap-3" use:reveal>
      <a
        href={cta.href}
        class="glow pop-self rounded-lg border border-accent-deep bg-bg px-8 py-3 font-medium text-ink ring-focus transition-colors hover:border-accent"
      >
        {$_('download.cta_for', { values: { platform: downloads[detected].name } })}
      </a>
      <p class="font-mono text-xs text-ink-faint">{cta.sublabel}</p>
    </div>
  {/if}

  <div class="mt-14 grid gap-5 sm:grid-cols-3">
    {#each order as p, i (p)}
      <article
        class="reveal flex flex-col rounded-2xl border border-line bg-surface p-6 transition-colors hover:border-line-strong"
        use:reveal={{ delay: i * 70 }}
      >
        <div class="flex items-center gap-3">
          <div
            class="grid h-10 w-10 place-items-center rounded-xl bg-accent-tint text-accent"
          >
            {#if p === 'macos'}
              <Apple class="h-5 w-5" />
            {:else}
              <Monitor class="h-5 w-5" />
            {/if}
          </div>
          <h2 class="text-lg font-semibold text-ink">{downloads[p].name}</h2>
        </div>

        <ul class="mt-5 space-y-2.5">
          {#each downloads[p].assets as a (a.label)}
            <li>
              <a
                href={a.href}
                class="glow block rounded-lg border border-line bg-bg px-4 py-3 ring-focus transition-colors hover:border-accent hover:bg-accent-tint"
              >
                <span class="font-mono text-sm font-medium text-ink">{a.label}</span>
                <span class="mt-0.5 block text-xs text-ink-faint">{a.sublabel}</span>
              </a>
            </li>
          {/each}
        </ul>
      </article>
    {/each}
  </div>

  {#snippet osPicker(current: Platform, pick: (p: Platform) => void)}
    <div
      class="mx-auto mt-6 flex w-fit gap-1 rounded-xl border border-line bg-surface p-1"
      role="group"
      aria-label={$_('download.os_pick')}
    >
      {#each order as p (p)}
        <button
          type="button"
          onclick={() => pick(p)}
          aria-pressed={current === p}
          class="rounded-lg px-4 py-1.5 text-sm font-medium ring-focus transition-colors {current === p
            ? 'bg-accent-tint text-accent'
            : 'text-ink-soft hover:text-ink'}"
        >
          {downloads[p].name}
        </button>
      {/each}
    </div>
  {/snippet}

  <!-- Every panel is in the HTML, only hidden, so the commands are there for
       search engines and for anyone reading without JavaScript. -->
  {#snippet commandPanels(cmds: Record<Platform, Cmd[]>, current: Platform, scope: string)}
    {#each order as p (p)}
      <div class="mt-6" hidden={current !== p}>
        {#if p === 'windows'}
          <p class="mb-3 text-center text-xs text-ink-faint">{$_('download.ps_note')}</p>
        {/if}
        <ul class="space-y-3">
          {#each cmds[p] as c (c.id)}
            <li
              class="rounded-2xl border bg-surface p-4 {c.recommended ? 'border-accent' : 'border-line'}"
            >
              <div class="flex flex-wrap items-center gap-2">
                <h3 class="text-sm font-semibold text-ink">{c.title}</h3>
                {#if c.recommended}
                  <span
                    class="rounded-full bg-accent-tint px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-accent"
                  >
                    {$_('download.recommended')}
                  </span>
                {/if}
              </div>
              {#if c.note}
                <p class="mt-1 text-xs leading-relaxed text-ink-faint">{$_(c.note)}</p>
              {/if}
              {#if c.code}
                <div class="mt-3 flex items-stretch gap-2">
                  <pre
                    class="min-w-0 flex-1 overflow-x-auto rounded-lg bg-pine px-4 py-3 font-mono text-[13px] leading-relaxed text-white/90"><code
                      >{c.code}</code
                    ></pre>
                  <button
                    type="button"
                    onclick={() => copy(c.code ?? '', `${scope}-${p}-${c.id}`)}
                    class="glow pop-self anim-host grid w-11 flex-none place-items-center rounded-lg border border-line bg-bg text-ink-soft ring-focus transition-colors hover:border-accent hover:text-accent"
                    aria-label={$_('cli.copy')}
                  >
                    {#if copied === `${scope}-${p}-${c.id}`}
                      <Check data-anim="pop" class="h-4 w-4 text-accent" />
                    {:else}
                      <Copy data-anim="pop" class="h-4 w-4" />
                    {/if}
                  </button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  {/snippet}

  <!-- install from a terminal -->
  <div id="terminal" class="reveal mx-auto mt-16 max-w-3xl scroll-mt-24" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('download.term_title')}</h2>
    <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">
      {$_('download.term_body')}
    </p>
    {@render osPicker(installOs, (p) => (installOs = p))}
    {@render commandPanels(installCmds, installOs, 'install')}
  </div>

  <!-- uninstall -->
  <div id="uninstall" class="reveal mx-auto mt-16 max-w-3xl scroll-mt-24" use:reveal>
    <h2 class="text-center text-2xl font-semibold text-ink">{$_('download.uninstall_title')}</h2>
    <p class="mx-auto mt-2 max-w-xl text-center text-sm text-ink-soft">
      {$_('download.uninstall_body')}
    </p>
    {@render osPicker(uninstallOs, (p) => (uninstallOs = p))}
    {@render commandPanels(uninstallCmds, uninstallOs, 'uninstall')}
  </div>

  <!-- changelog -->
  <div
    class="reveal mt-14 overflow-hidden rounded-2xl border border-pine-line bg-pine"
    use:reveal
  >
    <div class="grid items-center gap-6 p-8 sm:grid-cols-[1fr_auto]">
      <div>
        <p class="kicker">v{$version}</p>
        <h3 class="mt-2 text-xl font-semibold text-white">{$_('download.changelog_title')}</h3>
        <p class="mt-2 text-sm text-white/60">{$_('download.changelog_body')}</p>
      </div>
      <a
        href={CHANGELOG_URL}
        target="_blank"
        rel="noopener noreferrer"
        class="glow pop-self anim-host inline-flex items-center justify-center gap-2 rounded-lg border border-pine-line bg-white/5 px-5 py-2.5 text-sm font-medium text-white ring-focus transition-colors hover:bg-white/10"
      >
        <Github data-anim="pop" class="h-4 w-4" />
        {$_('download.changelog_cta')}
      </a>
    </div>
  </div>

  <div
    class="reveal mt-6 flex flex-col items-center gap-4 rounded-2xl border border-line bg-surface p-7 text-center sm:flex-row sm:justify-between sm:text-left"
    use:reveal
  >
    <div>
      <h3 class="text-lg font-semibold text-ink">{$_('download.all_releases_title')}</h3>
      <p class="mt-1.5 text-sm text-ink-soft">{$_('download.all_releases_body')}</p>
    </div>
    <Button href={ALL_RELEASES} target="_blank" variant="secondary" size="lg">
      <Github class="h-4 w-4" />
      {$_('download.all_releases_cta')}
    </Button>
  </div>

  <p class="mt-10 text-center text-xs text-ink-faint">
    {$_('download.selfhost_note')}
    <a
      href="https://github.com/rleeon/hoard#self-host"
      target="_blank"
      rel="noopener noreferrer"
      class="link-underline text-accent ring-focus hover:text-emerald-300"
    >
      GitHub
    </a>.
  </p>
</section>
