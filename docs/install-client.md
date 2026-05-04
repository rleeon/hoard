# Installing the Hoard desktop app

The Hoard desktop app is a small Tauri-based agent that watches your save
folders, uploads them to your Hoard server, and lets you restore previous
versions from a friendly UI. It runs on Linux, Windows, and macOS.

> Looking for the headless CLI / server packages? See
> [`docs/install.md`](install.md).

---

## Download

Grab the latest installer for your platform from the
[**Releases page**](https://github.com/hoarddev/hoard/releases/latest).

| Platform | File you want |
| --- | --- |
| **Linux (Debian / Ubuntu / Mint / Pop!_OS)** | `Hoard_<version>_amd64.deb` |
| **Linux (Fedora / RHEL / openSUSE)** | `hoard-<version>-1.x86_64.rpm` |
| **Linux (any other distro, including Arch / NixOS)** | `Hoard_<version>_amd64.AppImage` |
| **Windows 10 / 11** | `Hoard_<version>_x64-setup.exe` (NSIS) or `Hoard_<version>_x64_en-US.msi` |
| **macOS 11+ (Intel)** | `Hoard_<version>_x64.dmg` |
| **macOS 11+ (Apple Silicon)** | `Hoard_<version>_aarch64.dmg` |

Each artifact ships with a matching `.sha256` file. Verify before
installing:

```sh
sha256sum -c Hoard_*.sha256
```

---

## Install

### Linux — `.deb`

```sh
sudo apt install ./Hoard_*.deb
```

`apt` will pull in `libwebkit2gtk-4.1-0` and `libgtk-3-0` if you don't
have them. The launcher entry shows up under "Utilities". Hoard will
also install a `hoard-desktop` binary on your `$PATH`.

### Linux — `.rpm`

```sh
sudo dnf install ./hoard-*.rpm
# or
sudo rpm -i hoard-*.rpm
```

### Linux — AppImage

```sh
chmod +x Hoard_*.AppImage
./Hoard_*.AppImage
```

To get a launcher entry, drop it under `~/Applications/` and use a tool
like [AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher).

### Windows — NSIS (`.exe`) or MSI (`.msi`)

Double-click the installer.

> **First-run warning.** Hoard isn't yet code-signed, so SmartScreen
> will flag it as "unrecognised publisher". Click **More info →
> Run anyway**. Once we have a signing cert this warning goes away.

### macOS — `.dmg`

Mount the DMG and drag **Hoard.app** into `/Applications`.

> **First-run warning.** Hoard isn't yet notarised. Right-click the
> app and pick **Open**, then **Open** again at the prompt. From then
> on it launches normally. (`xattr -dr com.apple.quarantine
> /Applications/Hoard.app` works too.)

---

## First run

On first launch you'll go through a tiny onboarding wizard:

1. **Server URL** — paste the URL of your Hoard server, e.g.
   `https://hoard.your-domain.tld`. Hoard probes `/health` to confirm
   it can reach the server. (See [docs/install.md](install.md) if you
   haven't deployed a server yet.)
2. **Token** — paste a bearer token. Generate one with the admin CLI
   on the server:

   ```sh
   hoard-admin token create --user <your-username> --label desktop
   ```

   The token starts with `hoard_v1_` and is shown exactly once. Hoard
   stores it in your OS keyring (Secret Service / DPAPI / Keychain),
   not on disk in plaintext.
3. **Done** — Hoard scans your machine for known games and lands you
   on the Library, where you can pick which saves to track.

That's it. Close the window — the agent keeps running in the tray.

---

## Uninstall

| Platform | How |
| --- | --- |
| Linux (`.deb`) | `sudo apt remove hoard` |
| Linux (`.rpm`) | `sudo dnf remove Hoard` |
| Linux (AppImage) | Just delete the `.AppImage` file. |
| Windows | Settings → Apps → Hoard → Uninstall. |
| macOS | Drag `/Applications/Hoard.app` to the Trash. |

To wipe local data as well:

| Platform | Path |
| --- | --- |
| Linux | `~/.config/hoard/` (state) and `~/.cache/hoard/` (logs) |
| Windows | `%APPDATA%\hoard\` and `%LOCALAPPDATA%\hoard\` |
| macOS | `~/Library/Application Support/hoard/` and `~/Library/Caches/hoard/` |

The OS keyring entry is named `hoard.dev/desktop` — you can remove it
from your platform's credential manager after uninstalling.

---

## Updating

For v0.2 there's no auto-updater yet. Watch the
[Releases page](https://github.com/hoarddev/hoard/releases) for new
versions and reinstall over the top — Hoard's local state survives
upgrades.

Auto-updates are on the roadmap for v0.3. They'll require code-signing
keys to be rolled out first, so we're holding the feature until we can
ship signed releases on every platform.

---

## Troubleshooting

### "Couldn't reach the server"

The Server URL screen probes `/health` on whatever URL you typed.
Common fixes:

- Use the full URL, including scheme: `https://hoard.example.com`,
  not `hoard.example.com`.
- If you're on HTTPS with a self-signed cert, install the CA on your
  machine first — Hoard does not bypass certificate validation.
- Check your firewall allows outbound 443 (or whatever port your
  server uses).

### "Invalid token"

Tokens are tied to a user. If you regenerated the token on the server,
or revoked it via `hoard-admin token revoke`, you'll need to issue a
fresh one and paste it on the Token screen. Hoard stores one token per
machine.

### The tray icon doesn't show up (Linux)

You're probably on a desktop without an `AppIndicator` host. Install:

- **GNOME**: the
  [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/)
  extension.
- **KDE / XFCE / Cinnamon**: works out of the box.
- **Pure Wayland sessions** (sway, Hyprland): use `waybar` with the
  tray module, or rely on the main window — Hoard will keep working
  even if the tray fails to register.

### The agent keeps showing "Failed — retrying"

Open **Settings → Advanced → View logs**, click **Copy**, and file an
issue with the lines pasted in. The most common cause is a server-side
quota; the second-most-common is a save folder you no longer have
write access to.

### Where are my logs?

`Settings → Advanced → View logs` shows you the tail. The actual files
live under your platform's cache dir (`~/.cache/hoard/logs/` on Linux),
rotated daily. They're plain text and safe to copy into a bug report —
we never log save contents, only file counts and sizes.
