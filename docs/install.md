# Installing the Hoard server

Two supported deployment paths: **systemd on bare metal** (recommended for a
home server) and **Docker** (recommended if you already run other services
in compose). Pick one — they're mutually exclusive on the same host.

Either way, you need:

- Linux x86_64 (other targets work but aren't tested in CI).
- A persistent disk for `data_dir` (lots of small files; ext4 is fine).
- A reverse proxy if you expose this to the internet (Caddy / nginx /
  Traefik). Hoard does not terminate TLS itself.

---

## Path A — systemd

```sh
git clone https://github.com/rleeon/hoard.git
cd hoard
sudo ./deploy/scripts/install.sh
```

The script is idempotent. It will:

1. Build release binaries with `cargo build --release` (set
   `HOARD_SKIP_BUILD=1` if you've already built them).
2. Create a system user `hoard` with `/var/lib/hoard` as its home dir and
   no shell.
3. Install `hoard-server` and `hoard-admin` to `/usr/local/bin`.
4. Lay down a starter config at `/etc/hoard/config.toml` (mode `0640`,
   owned by `root:hoard`). Existing configs are not overwritten.
5. Create `/var/lib/hoard/{data,tmp,trash}` as `hoard:hoard 0750`.
6. Install the systemd unit and `systemctl enable` it.

### Configure

```sh
sudo $EDITOR /etc/hoard/config.toml
```

At minimum review:

- `[server].public_url` — your externally visible URL.
- `[storage].max_snapshot_size_mb` — caps a single upload.
- `[retention]` — how long soft-deleted and abandoned tmp uploads stick
  around.
- `[logging].format = "json"` for journald + structured log shippers.

Optional env-var overrides go in `/etc/hoard/hoard.env` (not committed):

```sh
HOARD__SERVER__PORT=8080
HOARD__LOGGING__LEVEL=debug
```

The unit reads it via `EnvironmentFile=-/etc/hoard/hoard.env` (the leading
dash makes it optional).

### Initialize and start

```sh
sudo -u hoard hoard-admin --config /etc/hoard/config.toml db migrate
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'first-laptop'
# ⚠ token printed once — save it now

sudo systemctl start hoard-server
sudo systemctl status hoard-server
sudo journalctl -u hoard-server -f
```

### Uninstall

```sh
sudo ./deploy/scripts/uninstall.sh           # keeps /var/lib/hoard
sudo ./deploy/scripts/uninstall.sh --purge   # also wipes data + user
```

### Hardening notes

The shipped unit applies a strict set of systemd directives
(`ProtectSystem=strict`, `NoNewPrivileges`, `MemoryDenyWriteExecute`,
syscall filter to `@system-service`, etc.). To loosen any of them without
editing the upstream unit, drop a file under
`/etc/systemd/system/hoard-server.service.d/override.conf`.

---

## Path B — Docker / Compose

```sh
git clone https://github.com/rleeon/hoard.git
cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml

cd deploy/docker
docker compose up -d --build
```

The image is built multi-stage (`rust:slim` → `debian:bookworm-slim`),
runs as a non-root UID `10001`, and uses `tini` as PID 1.

### Initialize

The entrypoint runs migrations automatically the first time the server
starts. Then create your admin user and token:

```sh
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    user create alice --admin --password 'CHANGE_ME'
docker compose exec server hoard-admin --config /etc/hoard/config.toml \
    token create alice --device 'laptop'
```

### Volumes

- `hoard-data` (named) — `/var/lib/hoard` inside the container, holds the
  SQLite DB, snapshots, tmp uploads, and trash. Back this up.
- `./config` (bind, read-only) — your `config.toml`.

### Healthcheck

Compose's healthcheck hits `/v1/health` via `wget` every 30s. If the
container starts to flap, check `docker compose logs server`.

### Behind a reverse proxy

Set `HOARD_PORT=127.0.0.1:8080` in `.env` to bind only to localhost, then
front it with Caddy:

```caddyfile
hoard.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

`max_snapshot_size_mb` should match the proxy's body-size limit
(`client_max_body_size` in nginx).

---

## Upgrading

The server does **not** self-update on a timer — an in-flight upload
shouldn't be killed by the upgrader's choice of restart moment. Instead
there's an explicit subcommand:

```sh
sudo hoard-server upgrade
```

What it does:

1. Hits the GitHub releases API and finds the latest tagged version.
2. Bails if you're already on the latest, or on a newer build.
3. Downloads `hoard-{version}-linux-x86_64.tar.gz`, extracts the
   `hoard-server` binary to a sibling tmpfile, chmods it `0755`.
4. Atomically renames it over the running binary's path. The
   in-flight process keeps its file descriptor; the next exec picks
   up the new binary.
5. Prints the restart hint:

   ```
   sudo systemctl restart hoard-server
   ```

It deliberately doesn't:

- Load `/etc/hoard/config.toml`. Broken configs still upgrade cleanly.
- Touch the SQLite database or run migrations. Migrations run on the
  next `hoard-server` start.
- Restart the systemd unit itself — init systems vary, and you may
  want to schedule the restart yourself.

Docker users: the in-container path is the same, but the cleaner
upgrade is to bump the image tag and recreate the container:

```sh
cd deploy/docker
docker compose pull
docker compose up -d
```

Either way, **back up `data_dir` before upgrading across more than a
patch version** — see the next section. The 1.x line is committed to
backwards-compatible schema changes, but a backup is cheap insurance.

---

## Backup of the server data

The server itself stores everything under `data_dir`:

```
/var/lib/hoard/
├── hoard.db                # SQLite (use sqlite3 .backup for a hot copy)
├── hoard.db-wal
├── hoard.db-shm
├── data/<user_id>/<game>/<label>/v<N>/...
├── tmp/<upload_id>/        # in-flight uploads, safe to ignore
└── trash/<snapshot_id>/    # soft-deleted snapshots
```

A safe backup procedure:

```sh
sudo -u hoard sqlite3 /var/lib/hoard/hoard.db ".backup /var/backups/hoard-$(date +%F).db"
sudo tar -C /var/lib/hoard -czf /var/backups/hoard-data-$(date +%F).tgz data trash
```

The `tmp/` directory does not need to be backed up — it's reaped by the
server's cleanup task on a `tmp_cleanup_hours` interval.
