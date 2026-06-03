# Hoard

> Versioned cloud sync for game saves. Use the hosted service, or run the
> exact same server yourself — your box, your data, your version history.

[![CI](https://github.com/rleeon/hoard/actions/workflows/ci.yml/badge.svg)](https://github.com/rleeon/hoard/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/rleeon/hoard?label=release)](https://github.com/rleeon/hoard/releases/latest)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Hoard snapshots your game saves every time you stop playing, hashes every
file, and lets you roll back to any earlier version or pull your saves down on
a fresh machine. Every backup is a compressed, verified, versioned snapshot —
nothing is ever silently overwritten.

It comes in two flavours from one codebase:

- **Hoard Cloud** — the hosted service at [hoard.services](https://hoard.services).
  Sign in with Google, install the app, done. Free tier (1 GB, 3 devices);
  paid tier for bigger libraries.
- **Self-hosted** — run the same `hoard-server` binary on your own box and
  point the app at it. No account, no quota but the disk you give it.

The desktop app (Windows · Linux · macOS) auto-detects installed games,
watches the save folders, and syncs in the background. A headless `hoard` CLI
does the same on servers and Steam Decks.

## Why

Steam Cloud, GOG Galaxy and friends work until they don't: they overwrite a
good save with a corrupted one from another machine, the publisher kills the
service, or the game simply isn't covered. Hoard fixes the same problem while
keeping the data under your control.

- **Versioned** — every backup is a new snapshot; old versions don't expire.
- **Verified** — every file's SHA256 is stored and re-checked on restore.
- **Compact** — snapshots stream as zstd-compressed tar; the server dedups by
  content hash.
- **Auto-detect** — thousands of games via the Ludusavi manifest, plus
  filesystem, Steam library and running-process detection.
- **Cross-platform** — pre-built installers; no compiler needed.
- **In-app updates** — when a newer release ships, the app offers to install
  the right asset for your OS.

## Get the app

You don't need to compile anything. Grab an installer from the
[**latest release**](https://github.com/rleeon/hoard/releases/latest) or from
[hoard.services/download](https://hoard.services/download):

| Platform | File |
| --- | --- |
| Windows 10 / 11 | `Hoard_<version>_x64-setup.exe` or `…_x64_en-US.msi` |
| Linux (Debian / Ubuntu) | `Hoard_<version>_amd64.deb` |
| Linux (universal) | `Hoard_<version>_amd64.AppImage` |
| macOS (Intel / Apple Silicon) | `Hoard_<version>_x64.dmg` / `…_aarch64.dmg` |

First launch warns on Windows SmartScreen and macOS Gatekeeper — the app
isn't code-signed yet. Click through.

## Self-host

Run the server once; every machine you install the app on connects to it.

### Docker

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # set public_url at minimum

cd deploy/docker
docker compose up -d --build
docker compose logs -f                        # wait for "listening"

# create your user + a token for the desktop app
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
# save the printed token now — it cannot be retrieved later
```

In the app's onboarding, pick **Autohost**, paste the server URL and token.

### Bare metal + systemd

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
sudo ./deploy/scripts/install.sh
sudo $EDITOR /etc/hoard/config.toml
sudo -u hoard hoard-admin --config /etc/hoard/config.toml db migrate
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
sudo systemctl start hoard-server
```

Upgrade later with `sudo hoard-server upgrade`: it swaps the binary
atomically and prints the `systemctl restart` step (it won't restart the
service itself, so an in-flight sync isn't killed).

## Headless CLI

```sh
hoard config init --server http://YOUR_SERVER:8080
hoard login --token hoard_v1_...
hoard save create --game stardew-valley --label main
hoard backup <SAVE_ID> --from ~/.config/StardewValley/Saves --remember
hoard snapshots list <SAVE_ID>
hoard restore <SAVE_ID>
```

## Architecture

A Rust workspace of 8 crates:

| Crate | Role |
| --- | --- |
| `hoard-core` | shared types, hashing, file-walk primitives |
| `hoard-manifest` | Ludusavi manifest parser |
| `hoard-watcher` | filesystem + process watchers (reusable lib) |
| `hoard-agent` | sync engine: backup, restore, detection; talks to the server |
| `hoard-cli` | the headless `hoard` binary |
| `hoard-admin` | server-side admin CLI (users, tokens, db) |
| `hoard-server` | Axum HTTP server; owns the DB and storage |
| `hoard-desktop` | Tauri 2 + Svelte 5 desktop app wrapping `hoard-agent` |

`hoard-server` runs in two modes from one binary. Self-hosted uses SQLite +
on-disk snapshots under `data_dir`, with bearer-token auth and the
`/v1/saves` API. Built with `--features cloud` it becomes **Hoard Cloud**:
Postgres + Cloudflare R2 object storage, Supabase JWT auth, and a
presigned-upload `/v1/cloud/*` API. The client picks the right protocol from
`/v1/health`'s `mode` field. The marketing + account site lives in `web/`
(SvelteKit, deployed to GitHub Pages).

## Building from source

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

## Releasing

The version lives in three files (`Cargo.toml`, `tauri.conf.json`,
`ui/package.json`). Don't edit them by hand — stamp all three from one number:

```sh
node scripts/stamp-version.mjs 1.8.6   # or no arg to use the latest git tag
git commit -am "release: 1.8.6" && git tag v1.8.6 && git push --tags
```

The version shown on the website and in the app's update check is read live
from [`releases/latest`](https://github.com/rleeon/hoard/releases/latest), so
once the tagged release is published everything reports the same number with
no further edits.

## License

AGPL-3.0 — see [LICENSE](LICENSE). Run a modified version as a network service
and the AGPL requires you to publish your changes under the same license.
