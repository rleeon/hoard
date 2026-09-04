---
title: "How to self-host Hoard with Docker"
description: "Run your own Hoard server with Docker Compose in minutes. Open source, free, on your hardware — a fully self-hosted cloud for your game saves, no account or quota."
order: 0
featured: true
updated: 2026-09-03
---

Hoard is open source and self-hostable. Instead of using Hoard Cloud, you can run the same `hoard-server` on your own machine and point every device at it — no account, no storage quota beyond the disk you give it. This guide gets a server running with Docker in a few minutes.

## Why self-host Hoard

- **Full ownership.** Your game saves live on hardware you control, not someone else's cloud.
- **No quota.** Storage is limited only by your own disk.
- **Same app, same features.** Versioned history and background sync work exactly as they do with Hoard Cloud — only the backend changes.
- **Open source.** You can read, audit and modify the server.

This is the key difference from tools like [Ludusavi](/guides/ludusavi-alternative): Ludusavi is great for local backups and bring-your-own-cloud via Rclone, but you wire up the sync yourself. Hoard gives you a managed sync server you run once and every device connects to.

## What self-hosting means for your data

Worth stating plainly, because it's the thing most comparisons get wrong about Hoard.

**Hoard Cloud** is the managed option: you sign in, and your saves sit on our servers, in the EU.

**A self-hosted Hoard is entirely yours.** Your devices talk to your server and to nothing else. There is **no account with us, no telemetry to us, no quota and no relay** — nothing passes through our servers, because there is nothing of ours in the path. We can't see a save, a game name or an email address, for the simple reason that none of it ever reaches us. If Hoard Cloud shut down tomorrow, your setup would carry on unchanged.

To be exact about one thing: your server does have logins of its own — the user you create below, and a token per device. Those are yours, on your machine, in your database. What doesn't exist is an account with us.

## What you need

- A machine that stays on (a home server, NAS that runs Docker, or a small VPS).
- Docker and Docker Compose installed.
- Optionally a domain name and a reverse proxy for HTTPS (recommended for anything beyond your LAN).

## Install with Docker Compose

Clone the repo, create a config from the example, and start the stack:

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml      # Use nano or vim or something lol

cd deploy/docker
docker compose up -d
docker compose logs -f                         # wait for "listening"
```

Wait until the logs show that the server is listening. Data lives in a named Docker volume (`hoard-data`) — back it up like any other volume. The container listens on port `12421` internally; map a different host port with `HOARD_PORT=9000 docker compose up -d`.

## Create your user and a device token

The server has no signup screen — you create users from the command line:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'desktop'
```

The token is printed once and **cannot be retrieved later**, so copy it now.

## Connect the desktop app

Install the [Hoard desktop app](/download) on each machine. In the onboarding flow, pick **Self-Host**, then paste your server URL and the token you just created. From there it behaves exactly like Hoard Cloud: it detects your games, backs up saves automatically, and keeps versioned history. See [syncing saves across PCs](/guides/sync-game-saves-across-pcs) for the day-to-day flow.

## Keep your server up to date

How you update depends on how you installed it, and the wrong command is a no-op rather than an error — so it is worth knowing which one is yours.

**Docker Compose.** Pull the new image and recreate the container. Both halves, in order:

```sh
docker compose pull
docker compose up -d
```

Stop after the first and the old container keeps running untouched: `/v1/health` goes on reporting the old version and the update looks as if it silently failed. `git pull` updates neither — what runs is the published image, not your checkout. Pin a version (`ghcr.io/rleeon/hoard:1.1`) instead of `:latest` if you would rather choose when a new one lands.

**Unraid.** *Docker* tab → Hoard → *Apply update* when one is offered. Nothing to type.

**Bare metal (systemd).** `sudo hoard-server upgrade`, then `sudo systemctl restart hoard-server`. It swaps the binary atomically and deliberately does not restart the service itself, so an in-flight sync is not killed.

`hoard-server upgrade` is for the bare-metal install only. Inside a container it refuses on purpose — the binary swap would not survive the next `docker compose up -d` — and prints the two commands above instead; run `docker compose exec server hoard-server upgrade` if you want to see it say so. Database migrations are applied by the server when it starts, so there is never a separate step for them.

## Run it in production

For anything exposed beyond your local network, terminate TLS at a reverse proxy (Caddy, nginx or Traefik). Prefer bare metal? The repo also ships a `systemd` install script and a `hoard-server upgrade` command that swaps the binary atomically without killing an in-flight sync.

## Self-host or Hoard Cloud?

Self-hosting is ideal if you already run a server and want full control with no quota. If you'd rather not maintain infrastructure, [Hoard Cloud](/pricing) gives you the same sync managed for you, with a free tier to start. Either way the app and your saves stay portable — you can switch later.

<!-- faq -->

## Frequently asked questions

### Does a self-hosted Hoard phone home?

No. The desktop app talks to the server address you give it. Your saves, your users and your logs stay on your machine, and nothing about them reaches us.

### Is the self-hosted server the same code as Hoard Cloud?

Yes, the same `hoard-server` binary, under AGPL-3.0. There is no cut-down community edition and no feature held back for the hosted version.

### Where are the saves actually stored?

By default in the Docker volume you gave the container, on your own disk. If you already run object storage, the server also speaks S3, so MinIO, Garage or Backblaze B2 work as the backing store. Either way, your devices only ever talk to your server.

### Can I run it on a NAS?

Yes, on any NAS that runs Docker. The repository ships an Unraid template, and the image drops to the `PUID`/`PGID` you give it, so bind-mounted folders end up owned by the right user instead of root.

### Do I need a domain and HTTPS?

Not on your own LAN. The moment the server is reachable from outside it, put a reverse proxy in front of it and terminate TLS there — Caddy, nginx or Traefik all work.

### What if my server is down when I finish playing?

The snapshot is taken locally, so nothing is lost. It uploads on its own once the server answers again.

### Can I start on Hoard Cloud and move later?

Yes, in both directions. You can export everything from your account page, and the app can be pointed at a different server without reinstalling.
