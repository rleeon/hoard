# Hoard

> Self-hosted cloud sync for game saves — keep your own server, your own data,
> your own version history.

[![CI](https://github.com/rleeon/hoard/actions/workflows/ci.yml/badge.svg)](https://github.com/rleeon/hoard/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/rleeon/hoard?label=release)](https://github.com/rleeon/hoard/releases/latest)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Hoard is two things in one project:

1. **A small server** you run on your own box (Linux, Docker, NAS, whatever).
2. **A desktop app** for Windows, Linux, and macOS that auto-detects your
   installed games, watches the save folders, and ships versioned snapshots
   to your server in the background — and pulls down updates if a different
   machine has a newer save.

Every backup is a hashed, compressed, versioned snapshot. You can restore any
past state, undo a corrupted save, or pull your saves down on a fresh
install with one click.

**Status: v1.3.5 — stable.** API and on-disk schema are committed since
v1.0; only backwards-compatible changes within the 1.x line. Recent
1.3.x releases focused on detection coverage, the Library UX, and an
in-app updater that finally retires the "download the .deb and run
`dpkg -i`" loop.

## Why

Steam Cloud, GoG Galaxy, EA Play and friends all work great until they don't:
they overwrite your good save with a corrupted one from another machine, the
publisher kills the service, or the game just isn't on a platform that has
cloud saves at all. Hoard solves the same problem but **you own the server
and the data**.

- **Versioned**: every backup is a new snapshot. Old versions stick around
  (configurable retention). Restore any version with one click.
- **Verified**: every file in a snapshot has its SHA256 stored. The client
  re-verifies on restore.
- **Compact**: snapshots stream to the server as zstd-compressed tar.
- **Multi-game / multi-save**: per-game catalog with per-save labels
  (`speedrun-attempt-3`, `before-final-boss`, …).
- **Quota-aware**: per-user storage quotas, soft delete + trash retention.
- **Auto-detect**: thousands of games covered out of the box via the
  Ludusavi manifest, plus filesystem + Steam library + running-process
  detection.
- **Cross-platform clients**: Windows, Linux, and macOS — pre-built
  installers, no compiler needed.
- **In-app updates**: when a newer release is out, an amber alert
  button next to the sidebar version offers to install it. The
  desktop client downloads the right asset for your OS and hands it
  to the platform installer (`pkexec dpkg -i` / `msiexec` / `open`);
  the server has a separate `hoard-server upgrade` subcommand the
  operator runs by hand so a sync mid-upload never gets killed.

---

## Get the desktop app

> **You do not need to compile anything to use Hoard.** Pre-built installers
> for every platform are attached to every tagged release.

Download from the [**Releases page →**](https://github.com/rleeon/hoard/releases/latest)

| Platform | File |
| --- | --- |
| **Windows 10 / 11** | `Hoard_<version>_x64-setup.exe` (NSIS, per-user) — or `Hoard_<version>_x64_en-US.msi` |
| **Linux (Debian / Ubuntu / Mint / Pop!_OS)** | `Hoard_<version>_amd64.deb` |
| **Linux (Fedora / RHEL / openSUSE)** | `hoard-<version>-1.x86_64.rpm` |
| **Linux (Arch / NixOS / others)** | `Hoard_<version>_amd64.AppImage` |
| **macOS 11+ (Intel)** | `Hoard_<version>_x64.dmg` |
| **macOS 11+ (Apple Silicon)** | `Hoard_<version>_aarch64.dmg` |

After the first install, future updates are offered inside the app: an
amber alert button next to the sidebar version pops a confirmation
modal and runs the platform installer for you.

Full install / uninstall / troubleshooting guide:
[**docs/install-client.md**](docs/install-client.md).

> First-run note: Hoard isn't yet code-signed, so Windows SmartScreen
> ("unrecognised publisher") and macOS Gatekeeper ("unidentified developer")
> will warn on first launch. Click through; the [client install
> guide](docs/install-client.md#install) has the exact steps.

---

## Set up the server

You run the server once. After that, every machine you install the desktop
app on connects to it.

### Docker (recommended)

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml   # set public_url at minimum

cd deploy/docker
docker compose up -d --build
docker compose logs -f                     # wait for "listening"

# create your user + a token for the desktop app
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
# ⚠ save the printed token NOW — it cannot be retrieved later
```

Then open the desktop app on your gaming machine, paste the server URL and
the token, and you're done.

### Bare metal Linux + systemd

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
sudo ./deploy/scripts/install.sh
sudo $EDITOR /etc/hoard/config.toml
sudo -u hoard hoard-admin --config /etc/hoard/config.toml db migrate
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
sudo systemctl start hoard-server
sudo journalctl -u hoard-server -f
```

Or download the pre-built server tarball
(`hoard-<version>-linux-x86_64.tar.gz`) from the Releases page if you don't
want to compile.

To upgrade the server later, run `sudo hoard-server upgrade` on the
host — the subcommand fetches the latest release, swaps the binary
atomically, and prints the `systemctl restart` step. It deliberately
does *not* restart the service itself, so an in-flight sync isn't
killed by the upgrader.

Full server install instructions: [**docs/install.md**](docs/install.md).

---

## Use the headless CLI (advanced)

If you'd rather sync from a server, a Steam Deck, or a CI box without a GUI,
the same `hoard` CLI from v0.x is still shipped:

```sh
hoard config init --server http://YOUR_SERVER:8080
hoard login --token hoard_v1_...
hoard save create --game stardew-valley --label main
hoard backup <SAVE_ID> --from ~/.config/StardewValley/Saves --remember
hoard snapshots list <SAVE_ID>
hoard restore <SAVE_ID>
```

Full walkthrough: [**docs/quickstart.md**](docs/quickstart.md).

---

## Documentation

- [docs/install-client.md](docs/install-client.md) — desktop app install,
  first-run wizard, troubleshooting
- [docs/install.md](docs/install.md) — server install (bare metal + Docker)
- [docs/quickstart.md](docs/quickstart.md) — headless CLI walkthrough
- [docs/api.md](docs/api.md) — HTTP API reference (curl examples)
- [docs/privacy.md](docs/privacy.md) — every network call the desktop app
  makes, what it stores, and the opt-in telemetry contract
- [docs/decisions/](docs/decisions/) — architecture decision records (ADRs)

## Architecture in one paragraph

A Rust workspace with nine crates: `hoard-core` (shared types, hashing),
`hoard-server` (Axum HTTP server + SQLite via SQLx, atomic snapshot commits
via `fs::rename` + DB transaction), `hoard-admin` (server-side CLI for
users / tokens / games / db), `hoard-cli` (the `hoard` binary for headless
use), `hoard-agent` (the shared sync engine — talks to the server, walks
files, runs backup / restore), `hoard-manifest` (Ludusavi manifest parser),
`hoard-detect` (filesystem + Steam + process detection), `hoard-watcher`
(filesystem + process watchers exposed as a reusable library), and
`hoard-desktop` (Tauri 2 + Svelte 5 GUI that wraps `hoard-agent` with a
tray icon, autostart, dashboard, and history viewer). Snapshots stream up
as multipart with per-file SHA256, and stream down as `tar.zst` built on
the fly. Soft-delete moves directories into a `trash/` tier; a periodic
task purges them after a configurable retention.

## Building from source

You don't need to do this — the Releases page has installers — but if you
want to:

```sh
# Server / CLI / admin
cargo build --release -p hoard-server -p hoard-cli -p hoard-admin

# Desktop app (needs Node 20 + pnpm 9 + Tauri prerequisites)
pnpm --dir crates/hoard-desktop/ui install
cargo install tauri-cli --version '^2'
cargo tauri build --manifest-path crates/hoard-desktop/Cargo.toml
```

Linux build prerequisites: `libwebkit2gtk-4.1-dev`, `libgtk-3-0`,
`libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`,
`build-essential`, `libdbus-1-dev`. See `.github/workflows/release-desktop.yml`
for the canonical list.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bugs and feature requests via
GitHub Issues.

## License

AGPL-3.0 — see [LICENSE](LICENSE). If you run a modified version as a
network service, the AGPL requires you to publish your changes under the
same license.
