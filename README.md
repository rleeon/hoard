# Hoard

> Self-hosted cloud sync for game saves — keep your own server, your own data,
> your own version history.

[![CI](https://github.com/USER/hoard/actions/workflows/ci.yml/badge.svg)](https://github.com/USER/hoard/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

Hoard runs on your own box (Linux, Docker, or whatever) and gives you a small
HTTP API + a CLI that you point at your save folders. Every backup is a
versioned snapshot — restore any past state, undo a bad save, or pull your
saves down on a fresh machine.

**Status: v0.1.0 — usable end-to-end, but pre-1.0. Self-hosted use only; no
public registration on the demo path. Expect breaking changes before 1.0.**

## Why

Cloud save services (Steam Cloud, GoG Galaxy, etc.) work great until they
don't: they overwrite your good save with a corrupted one from another
machine, the publisher shuts down the service, or the game just isn't on a
platform that has cloud saves at all. Hoard solves the same problem but you
own the server.

- **Versioned**: every `hoard backup` writes a new snapshot. Old versions
  stick around (configurable retention). Restore any version with one
  command.
- **Verified**: every file in a snapshot has its SHA256 stored. The CLI
  re-verifies on restore.
- **Compact**: snapshots are streamed to the server as zstd-compressed tar.
- **Multi-game / multi-save**: per-game catalog with per-save labels
  (`speedrun-attempt-3`, `before-final-boss`, …).
- **Quota-aware**: per-user storage quotas, soft delete + trash retention.
- **Cross-platform clients**: Linux today, Windows on the roadmap.

## Quickstart (Docker)

```sh
git clone https://github.com/USER/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml   # set public_url at minimum

cd deploy/docker
docker compose up -d --build
docker compose logs -f                     # wait for "listening"

# create your user + token
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'laptop'
# ⚠ save the printed token NOW — it cannot be retrieved later
```

Then on whatever machine has your saves:

```sh
cargo build --release -p hoard-cli
sudo install -m 0755 target/release/hoard /usr/local/bin/

hoard config init --server http://YOUR_SERVER:8080
hoard login --token hoard_v1_...
hoard save create --game stardew-valley --label main
hoard save list                            # note the save id
hoard backup <SAVE_ID> --from ~/.config/StardewValley/Saves --remember
hoard snapshots list <SAVE_ID>
hoard restore <SAVE_ID>                    # uses --remember'd path
```

## Quickstart (bare metal Linux + systemd)

```sh
git clone https://github.com/USER/hoard.git && cd hoard
sudo ./deploy/scripts/install.sh
sudo $EDITOR /etc/hoard/config.toml
sudo -u hoard hoard-admin --config /etc/hoard/config.toml db migrate
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
sudo systemctl start hoard-server
sudo journalctl -u hoard-server -f
```

Full instructions: [docs/install.md](docs/install.md).

## Documentation

- [docs/quickstart.md](docs/quickstart.md) — CLI walkthrough end-to-end
- [docs/install.md](docs/install.md) — bare-metal + Docker install
- [docs/api.md](docs/api.md) — HTTP API reference (curl examples)
- [docs/decisions/](docs/decisions/) — architecture decision records (ADRs)

## Architecture in one paragraph

A Rust workspace with four crates: `hoard-core` (shared types, hashing),
`hoard-server` (Axum HTTP server + SQLite via SQLx, atomic snapshot commits
via `fs::rename` + DB transaction), `hoard-admin` (server-side CLI for
users/tokens/games/db), and `hoard-cli` (the `hoard` binary that users run on
their machines). Snapshots stream up as multipart with per-file SHA256, and
stream down as tar.zst built on the fly. Soft-delete moves directories into
a `trash/` tier; a periodic task purges them after a configurable retention.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bugs and feature requests via
GitHub Issues.

## License

AGPL-3.0 — see [LICENSE](LICENSE). If you run a modified version as a network
service, the AGPL requires you to publish your changes under the same license.
