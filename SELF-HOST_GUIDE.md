# Self-host

[![CI](https://github.com/rleeon/hoard/actions/workflows/ci.yml/badge.svg)](https://github.com/rleeon/hoard/actions/workflows/ci.yml) [![Release](https://img.shields.io/github/v/release/rleeon/hoard?label=release)](https://github.com/rleeon/hoard/releases/latest) [![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

> Steam Cloud is not a backup strategy. Your server is.

Run `hoard-server` on your own hardware — a NAS, a VPS, or the machine
under your desk — and point every device at it. No account, no quota
beyond the disk you give it, and every byte of every save travels through
your own server, not someone else's cloud.

### Docker

The server is a prebuilt multi-arch image — `ghcr.io/rleeon/hoard` (also
mirrored to Docker Hub as `rleeon/hoard`), amd64 and arm64, published on every
release tag. It needs no compiler and no clone:

```sh
mkdir hoard && cd hoard
curl -O https://raw.githubusercontent.com/rleeon/hoard/main/deploy/docker/docker-compose.yml

# Those two variables are read only when the database is empty: on a first
# start they create that admin and print a device token in the log, once.
HOARD_ADMIN_USERNAME=myuser HOARD_ADMIN_PASSWORD='mypassword' docker compose up -d
docker compose logs -f server     # wait for "listening", then copy the token
```

The container writes itself a working `config.toml` into `./config/`, so there
is nothing to prepare beforehand. In the app's onboarding pick **Self-Host**
and give it `http://IP:12421` plus that token.

Clone the repo instead if you'd rather read the config before anything starts,
or build the image yourself:

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
mkdir -p deploy/docker/config
cp deploy/config.toml.example deploy/docker/config/config.toml
$EDITOR deploy/docker/config/config.toml    # Use nano or vim or something lol

cd deploy/docker
docker compose up -d              # pulls the prebuilt image from GHCR
# To build from source instead, uncomment `build:` in docker-compose.yml first.
docker compose logs -f server    # wait for "listening"

# create your user + a token for the desktop app
docker compose exec server hoard-admin --config /etc/hoard/config.toml user create myuser --admin --password 'mypassword'    #change "myuser" and "mypassword", dont delete ''.

docker compose exec server hoard-admin --config /etc/hoard/config.toml token create myuser --device 'desktop'    #changue "myuser" and if you want changue "desktop"
# SAVE THE TOKEN NOW — it cannot be retrieved later
```

In the app's onboarding, pick **Self-Host**, paste the server URL and token.

Update later with both halves of this, in order:

```sh
docker compose pull && docker compose up -d
```

`pull` fetches the new image; `up -d` recreates the container from it. Stop
after the first and the old container keeps running untouched, so `/v1/health`
goes on reporting the old version and the update looks like it silently failed.
`git pull` updates neither: what runs is the published image, not your
checkout — that's only the source, and it's used only if you uncommented
`build:`. Pin a version (`ghcr.io/rleeon/hoard:1.1`) in place of `:latest` if
you'd rather choose when that happens.

`hoard-server upgrade` is the bare-metal command and does not apply here: it
swaps the binary in place, and the next `up -d` would discard it. It knows that
about itself, so `docker compose exec server hoard-server upgrade` refuses and
prints the two commands above rather than downloading anything.

### Unraid

Hoard ships an Unraid template, so on a NAS none of the above is needed:

**Apps** → search **Hoard** → *Install*. Fill in the two boxes it asks for — an
admin username and a password — and press *Apply*. The container writes its own
config, creates that account, and prints a device token in its log **once**:

```

 Hoard is ready. Copy this token — it is shown ONCE:

 hoard_v1_…
```

Copy it, open the desktop app on your gaming PC, pick **Self-Host**, and give
it `http://IP:12421` plus that token. Remember changue "IP" pls --  The container's *WebUI* button 
opens the web panel, where the same username and password get you in.

Another PC later? *WebUI* → **Users** → *New token*. It is shown once, so copy
it before closing the dialog. The same tab creates accounts for other people in
the house. From a terminal instead, Container → *Console*:

```sh
hoard-admin token create myuser --device 'living-room PC'
```

Two folders under `/mnt/user/appdata/hoard/` hold everything — `data/` (the
database and every version of every save) and `config/` (`config.toml`, for when
you want to raise a limit or move storage to S3). Back up `data/` and you have
backed up the lot.

Updating is the *Docker* tab's job: it marks the container when a new image is
published, and *Apply update* pulls it and recreates it. Neither `hoard-server
upgrade` nor the app's *Upgrade server* button applies here; both swap a binary
in place, which a container recreate discards.

Not in the Apps tab yet? The template can be installed by hand — from the Unraid
terminal:

```sh
wget -O /boot/config/plugins/dockerMan/templates-user/hoard.xml https://raw.githubusercontent.com/rleeon/hoard/main/templates/hoard.xml
```

then **Docker** → *Add Container* → pick **Hoard** from the *Template* dropdown.

### Bare metal + systemd

```sh
git clone https://github.com/rleeon/hoard.git && cd hoard
sudo ./deploy/scripts/install.sh
sudo $EDITOR /etc/hoard/config.toml    # Use nano or vim or something lol
sudo -u hoard hoard-admin --config /etc/hoard/config.toml db migrate
sudo -u hoard hoard-admin --config /etc/hoard/config.toml user create myuser --admin --password 'mypassword'    #change "myuser" and "mypassword", dont delete ''.
sudo systemctl start hoard-server
```

Upgrade later with `sudo hoard-server upgrade`: it swaps the binary
atomically and prints the `systemctl restart` step (it won't restart the
service itself, so an in-flight sync isn't killed).

### Tailscale

If both the server and your machines are on a Tailnet, skip the reverse
proxy entirely: Tailscale encrypts the wire, so the server's lack of TLS
doesn't matter and none of the proxy settings below apply. The install is
the same on every machine — the server box is not special, just another
node of your tailnet — and each machine signs in once:

**Install the client.** 

On Linux — the server included — run the install
script below. On Windows or macOS, grab the app from
https://tailscale.com/download and sign in inside it instead of the
commands below.

```sh
curl -fsSL https://tailscale.com/install.sh | sh
```

**Sign in on that machine.**

```sh
sudo tailscale up
```

It prints a URL and opens your browser; authenticate once and the machine
joins your tailnet. On a headless server the URL is only printed — open it
from your laptop. One `tailscale up` per machine, all logged in to the
same account.

**Find the server's name.** From any machine in the tailnet:

```sh
tailscale status
```

lists every machine with its IPv4 (`100.x`) and its MagicDNS names: the
short one is the machine's hostname, the long one is
`<name>.<tailnet>.ts.net` — e.g. `server` and `server.<mytailnet>.ts.net`.
The names only resolve if MagicDNS is on (it is by default); if it isn't,
use the `100.x` IPv4 from the same listing.

**Check the server answers.** From a *client* machine, using the server
machine's short name from the listing above (the real address has no
brackets):

```sh
# Remember changue "ip" pls
tailscale ping IP
curl http://IP:12421/v1/health
```

`tailscale ping` reporting the path is up is the whole test — if that
works, the app will reach the server.

In the app's onboarding, pick **Self-Host** and paste `http://server:12421`
— replace `server` with your server machine's name from `tailscale status`,
no brackets — or the server's `100.x` IPv4. Both are classified by the
client as a local server, so the dashboard shows "X used" of your disk
instead of a quota percentage. Two address shapes are *not* classified as
local and you'd get the quota view: the full MagicDNS FQDN
(`<name>.<tailnet>.ts.net`) and an IPv6 literal (`fd7a:…`) — stick to
the short name or the IPv4. If you paste `http://user@host:12421` out of
habit, the client strips the `user@` itself.

### Behind a reverse proxy

`hoard-server` has no TLS, so most people put nginx, Caddy or a Cloudflare
tunnel in front of it. **Two proxy defaults will break syncing** and the
symptoms don't look like proxy problems:

- **Upload body size.** nginx allows 1 MB by default, and anything bigger gets
  a `413` before it ever reaches Hoard. The app can only report what it's told,
  so the backup shows up as rejected for size. From 1.1.3 the limit that
  matters is your biggest single *file*, not the whole save (see
  [How uploads travel](#how-uploads-travel)) — but the default is still 1 MB,
  which nothing survives.
- **Timeouts.** A big restore or upload that runs past `proxy_read_timeout`
  (60 s by default) is cut off mid-transfer and surfaces as a `502` with an
  HTML body — HTML that Hoard never emits.

nginx:

```nginx
client_max_body_size 4G;
proxy_read_timeout   600s;
proxy_send_timeout   600s;
proxy_request_buffering off;   # stream uploads instead of spooling to disk
```

Caddy needs neither (no body cap, and it streams by default). A Cloudflare
proxied hostname caps request bodies at 100 MB on the free plan regardless of
your own config — since 1.1.3 that cap applies per file rather than per save,
so it only bites if a single save file is over 100 MB; if one is, use a tunnel
to a hostname that isn't proxied, or connect over your LAN/VPN.

### How uploads travel

Hoard has always stored each unique file once (keyed by its SHA-256) and let
versions share the bytes. Until 1.1.2 that only applied to *storage*: a backup
still uploaded the whole folder every time, and the server threw away the 99%
it already had. A 3 GB save that changed 10 MB cost 3 GB of upload.

From 1.1.3 the client negotiates first. It hashes the save, tells the server
what the version contains, and the server answers which of those files it is
missing. Only those travel — each as its own request — and a final call closes
the version. In practice a second backup of the same game moves megabytes.

What this changes for you:

- **Upload time and bandwidth** drop to whatever actually changed.
- **Request bodies** are now one file each, not the whole save, so proxy body
  caps stop being the thing that breaks big saves.
- **`max_snapshot_size_mb`** still limits how big a save may be, but the client
  now learns the number before uploading instead of failing mid-transfer.
- **Nothing to configure**, and nothing to migrate: the server advertises the
  capability in `/v1/health` and older clients keep using the old upload path
  against the same server.

The client still never talks to the storage backend — every byte goes through
the server, exactly as before.

### Your machines

From 1.1.3 the server keeps a census of the machines on each account: which
exist, which are on right now, and what each is playing. The Eye panel in the
app shows it. Every version in a game's history already said which machine it
came from; this adds the live half.

It works because each machine sends a small heartbeat to **your** server every
30 seconds. A machine counts as on while its last heartbeat is under 90 seconds
old, so one that loses power goes dark on its own — nothing has to notice it
left. Machines identify themselves by a stable fingerprint, so reinstalling the
app doesn't create a duplicate, and one that goes 90 days without appearing is
forgotten. You can also forget one on the spot; its saves and their history are
untouched.

Nothing here leaves your server. The census lives in your SQLite, your machines
write to it directly, and there is no endpoint that forwards it anywhere. It's
your computers talking to each other through your own server — which is why
this belongs in self-hosted and operator broadcasts don't.

### The web panel

The server serves a small web panel from the same port it already listens on.
Point a browser at your server's address — `http://192.168.1.50:12421`, or
whatever your reverse proxy exposes — and it lands on `/panel`.

Sign in with the username and password you created with `hoard-admin user
create`. That password was written to the database from the first release and
never read by anything until now, so an account created two years ago can sign
in today. Forgot it? `hoard-admin user passwd <user>` sets a new one and closes
any browser session that was open.

If you would rather not use a password, the panel also takes a `hoard_v1_…`
token — the same one your desktop app uses. It is exchanged for a session
immediately and never stored in the browser.

What you get:

- **Your account**: every game, save and version with its real size, what
  deduplication saved, which machine each version came from, playtime for the
  last 30 days, and your quota. Any version can be downloaded, sent to the
  trash, or pulled back out of it — deleted versions stay listed, struck
  through, until the server purges them for good. The page also updates on its
  own when another machine uploads.
- **Your machines**: the same census the app's Eye panel shows — who is on and
  what they are playing.
- **Activity**: what the server recorded — versions created, restored, deleted
  and pruned. This log has been written since the first migration; the panel is
  the first thing that reads it.

Admin accounts get three more sections: server-wide storage (logical vs. real
size, orphan objects, database size), users, and the diagnostic logs your
clients upload. Non-admins cannot reach them — the check is server-side, so
hiding the tab is not what protects them.

The users tab does the whole of account administration: create an account,
rename one, set a password, change a quota, grant or remove admin, delete an
account, and issue a device token for a machine. Between them that is every
`hoard-admin user` and `hoard-admin token` subcommand, so a server on a NAS
never needs a shell for day-to-day work. Three things behave the way they do
for a reason:

- **A new token is shown once.** Only its SHA-256 is stored, here as everywhere
  else, so a token you close the dialog on is a token you mint again.
- **Deleting an account deletes its saves**, every version of them, and cannot
  be undone. The dialog asks you to type the account's name and tells you how
  much is about to go.
- **Setting someone's password closes their browser sessions but leaves their
  devices syncing.** A device token is how a PC backs up; nobody changing a
  password is asking to re-pair every machine they own.

You cannot delete the account you are signed in as. That is also what keeps a
server from ending up with no admin at all: the admin flag guards its own
route, and getting it back would need a shell.

Deliberately **not** in the panel: migrating storage backends and verifying
every object. Those stay in `hoard-admin`, where a terminal can show progress
and refuse to be closed halfway.

Two things worth knowing:

- Browser sessions are ordinary API tokens with a short life, so they show up
  in `hoard-admin token list <user>` — and in the panel's own token list —
  next to your devices' tokens, and revoking one signs that browser out. They
  are told apart by their device name, which is why the panel refuses to issue
  a device token called `web panel`.
- Five wrong passwords in a row shut that account's door for
  `panel.login_throttle_secs` (10 by default), and the reply carries a
  `Retry-After`. It is deliberately short — the point is to stop the password
  hashing from being used as a CPU lever, not to lock you out of your own
  server. Raise it if the panel faces the open internet. You cannot go below
  2 seconds: a lower value is read as 2 and the server says so at startup.
- The throttle counts per client address, and the server only takes an
  `X-Forwarded-For` header at its word when the connection comes from a proxy
  you named in `server.trusted_proxies` (default: `loopback`). **If your
  reverse proxy is not on the same machine — a container on a Docker network,
  another box on the LAN — add it there**, or every client behind it shares one
  counter. The server prints what it trusts at startup.
- Sessions last `panel.session_days` (14 by default) and the cookie is marked
  `Secure` only when the request arrives over HTTPS, so a plain-HTTP LAN
  instance still works. Put it behind TLS if you expose it to the internet.

To turn the whole thing off — panel pages and password login together — set
`enabled = false` under `[panel]` in `config.toml`.

```toml
[server]
# Only these peers may say who the client is via X-Forwarded-For.
# "loopback" | "private" | "any" | addresses | CIDRs. [] believes nobody.
trusted_proxies = ["loopback"]

[panel]
enabled = true
# How long a browser session lasts.
session_days = 14
# Seconds the login door stays shut after five wrong passwords. Minimum 2.
login_throttle_secs = 10
```

## External storage (S3-compatible)

By default the server keeps every blob on local disk under `data_dir`. You can
instead point it at any **S3-compatible** bucket — MinIO, Backblaze B2,
Cloudflare R2, Garage, Wasabi — by adding a `[storage.s3]` block. Consumer
drives that don't speak S3 at all (OneDrive, Mega, Google Drive, Dropbox,
pCloud, Proton Drive…) work through an `rclone serve s3` bridge; that's a
[section of its own](#consumer-cloud-drives-onedrive-mega-drive-dropbox-)
because it has real trade-offs.

Nothing else changes: the client never talks to the bucket, there are no
presigned URLs, and the upgrade is zero-config for existing installs (omit the
block and you stay on disk).

Important: **the server is still required.** It owns the SQLite index, the auth
tokens and the deduplication — the bucket only holds opaque zstd blob/chunk
bytes, sharded by content hash. The bucket *on its own* is not restorable: without
the server's database there is no mapping from saves/versions to those bytes.
The SQLite DB, `tmp/` upload staging and the upgrade marker always stay on local
disk regardless of backend, so `data_dir` must still be writable.

### MinIO

```sh
# bring up MinIO and create the bucket (console at :9001, or `mc mb`)
docker run -d -p 9000:9000 -p 9001:9000 \
  -e MINIO_ROOT_USER=hoard -e MINIO_ROOT_PASSWORD=change-me-please \
  -v /srv/minio:/data minio/minio server /data --console-address ":9001"
mc alias set h http://127.0.0.1:9000 hoard change-me-please
mc mb h/hoard
```

Then in `/etc/hoard/config.toml`:

```toml
[storage]
data_dir = "/var/lib/hoard"     # still holds the DB + tmp staging
backend  = "s3"

[storage.s3]
endpoint = "http://127.0.0.1:9000"
bucket = "hoard"
access_key_id = "hoard"
secret_access_key = "change-me-please"
force_path_style = true          # MinIO / Garage / rclone require this
```

On boot the server writes a probe object, reads it back, compares the bytes and
deletes it. A bad endpoint, bucket or credential — or one that doesn't store
what it was sent — fails fast with a clear message instead of erroring
mid-sync, or worse, corrupting quietly.

### Consumer cloud drives (OneDrive, Mega, Drive, Dropbox, …)

There is no native OneDrive/Mega/Drive/Dropbox integration and there won't be:
the server speaks S3 and nothing else. What bridges the gap is **`rclone serve
s3`**, a small process you run next to the server that presents an S3 endpoint
on localhost and forwards every object to any of rclone's ~70 remotes. Hoard
sees a bucket; your drive sees a folder full of files.

Read the [trade-offs](#is-this-a-good-idea) before you commit to it. If you
have a real object store available (a €5 B2 bucket, a MinIO on your NAS), use
that instead — this path exists because "I already pay for 1 TB of OneDrive"
is a perfectly good reason.

#### 1. Install rclone and connect your drive

```sh
sudo -v ; curl https://rclone.org/install.sh | sudo bash
sudo -u hoard -H rclone config          # run it as the user the server runs as
```

`rclone config` is an interactive wizard: `n` for a new remote, name it
`drive`, pick your provider from the list, accept the defaults, and say **no**
to "Use auto config" if the server is headless — it prints a URL you open on
your laptop, and you paste the token back. Provider-specific notes:

| Provider | `rclone config` type | Notes |
|---|---|---|
| OneDrive | `onedrive` | Choose `OneDrive Personal or Business`; the wizard then lists your drives. |
| Mega | `mega` | User + password, no OAuth dance. Enable 2FA-free app access. |
| Google Drive | `drive` | Ask for a full-access scope (`1`), otherwise the server can't delete during GC. |
| Dropbox | `dropbox` | Default scopes are fine. |
| pCloud / Proton / Koofr / … | see `rclone config` | Anything in [rclone's list](https://rclone.org/overview/) works the same way. |

Verify the remote and create the folder that will act as the bucket:

```sh
sudo -u hoard -H rclone lsd drive:            # should list your drive's folders
sudo -u hoard -H rclone mkdir drive:hoard     # this folder *is* the bucket
```

#### 2. Run the bridge as a service

`rclone serve s3` must be up before the server and stay up; run it under
systemd, not in a terminal. `/etc/systemd/system/rclone-hoard-s3.service`:

```ini
[Unit]
Description=rclone S3 bridge for Hoard
After=network-online.target
Wants=network-online.target

[Service]
User=hoard
Group=hoard
ExecStart=/usr/bin/rclone serve s3 drive:hoard \
  --addr 127.0.0.1:9100 \
  --auth-key hoardkey,CHANGE-ME-please \
  --force-path-style \
  --vfs-cache-mode writes \
  --vfs-cache-max-size 4G \
  --cache-dir /var/lib/hoard/rclone-cache \
  --log-level NOTICE
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```sh
sudo mkdir -p /var/lib/hoard/rclone-cache && sudo chown hoard: /var/lib/hoard/rclone-cache
sudo systemctl enable --now rclone-hoard-s3
sudo systemctl status rclone-hoard-s3
```

What the flags are doing, since each one is load-bearing:

- `--addr 127.0.0.1:9100` — **loopback only**. The bridge has no TLS and one
  static key pair; it must not be reachable from the network.
- `--auth-key id,secret` — invent both, they're only shared with the server. If
  you omit the flag rclone allows *anonymous* access to your drive. Don't.
- `--force-path-style` — the addressing Hoard uses (it's rclone's default too,
  spelled out here so a config edit can't silently flip it).
- `--vfs-cache-mode writes` + `--vfs-cache-max-size` — stage uploads on local
  disk and push them to the drive in the background. Without it every upload
  runs at your drive's write speed with the server waiting on it. The trade-off
  is honest: a blob is "stored" from Hoard's point of view when rclone has it,
  so a machine that dies with a full cache loses whatever hadn't been pushed
  yet. Drop the two flags if you'd rather be slow than sorry.

Also make the server wait for the bridge — add to
`/etc/systemd/system/hoard-server.service.d/rclone.conf`:

```ini
[Unit]
After=rclone-hoard-s3.service
Wants=rclone-hoard-s3.service
```

#### 3. Point Hoard at it

```toml
[storage]
data_dir = "/var/lib/hoard"     # still holds the DB, tmp staging and the cache
backend  = "s3"

[storage.s3]
endpoint = "http://127.0.0.1:9100"
bucket = "hoard"                 # the folder you created inside the remote
region = ""
access_key_id = "hoardkey"
secret_access_key = "CHANGE-ME-please"
force_path_style = true
```

#### 4. Check it before you trust it with saves

```sh
sudo -u hoard hoard-admin --config /etc/hoard/config.toml storage status
```

The last line must read `Reachability : ok (write+read+delete probe passed)`.
That probe writes an object, **reads it back and compares the bytes**, then
deletes it — a bridge that mangles uploads fails here instead of months later
at restore. The server runs the same probe at boot and refuses to start if it
fails, so a red status is not something to work around.

Then do a real round-trip: back up a save from the app, delete it locally,
restore it. And once you have a few versions stored:

```sh
sudo -u hoard hoard-admin --config /etc/hoard/config.toml storage verify --all
```

which re-downloads every object and checks it still hashes to its key.

#### Is this a good idea?

Sometimes. What you're getting is a drive with no object-storage semantics
pretending to be one, so:

- **Slow, and rate-limited.** Every blob is a file operation against a consumer
  API. OneDrive throttles, Drive caps uploads around 750 GB/day, Mega meters
  bandwidth. A first backup of a large library takes hours; restores stream at
  whatever the drive gives you. Keep the bridge on the same machine as the
  server — LAN, never WAN.
- **`rclone serve s3` is marked experimental by rclone itself.** It has been
  stable enough in practice, but that's the ground you're standing on.
- **Your drive alone is not a backup of your saves.** The folder holds opaque
  zstd blobs named by hash; the mapping from saves and versions to those bytes
  lives only in the server's SQLite DB. Copy `data_dir/hoard.db` somewhere
  safe on a schedule (`sqlite3 hoard.db ".backup /path/hoard.db.bak"` is
  consistent against a running server) or the blobs are unreadable bytes.
- **You still need local disk.** The DB, upload staging and the rclone cache
  all live under `data_dir`. Restores spool one blob (or one 4 MiB chunk of a
  large file) at a time, so scratch space stays small — but it isn't zero.
- **Don't let a desktop sync client near that folder.** The OneDrive/Dropbox
  app syncing the same directory the bridge writes to is how you get partial
  files and conflict copies. Give the bridge its own folder and leave it alone.

What you get in exchange: the server stops growing, and its disk holds a
database instead of every version of every save.

#### Any other S3 endpoint

The same `[storage.s3]` block points at anything that speaks S3. Hoard talks a
deliberately plain dialect — no flexible checksums, no `aws-chunked` bodies, no
presigned URLs, and one plain PUT per object (files above 128 MB are split into
chunks before they reach the backend, so multipart never comes up in practice)
— which is why older MinIO builds, Backblaze B2, Ceph RGW, Garage, SeaweedFS
and friends work without special-casing.

### Migrating between storage backends

Switching an existing install between `local` and `s3` (either direction) means
copying the blob/chunk bytes first — flipping `backend` alone would leave the
server pointing at an empty store (it refuses to boot in that case, telling you
to migrate). `hoard-admin storage` does the copy.

**Stop the server first.** Writes that arrive mid-migration would be missed.

```sh
# 1. See what you have and where.
sudo -u hoard hoard-admin --config /etc/hoard/config.toml storage status

# 2. Fill in the [storage.s3] block in config.toml (endpoint/bucket/keys),
#    but leave backend on its current value for now.

# 3. Stop the server, then copy every object to the new backend.
sudo systemctl stop hoard-server
sudo -u hoard hoard-admin --config /etc/hoard/config.toml storage migrate --to s3

# 4. Flip the switch and restart.
sudo $EDITOR /etc/hoard/config.toml        # [storage] backend = "s3"
sudo systemctl start hoard-server
```

`migrate` is idempotent and resumable — if it's interrupted, just run it again
and it skips whatever already copied. It **never deletes source data** unless
you pass `--delete-source`, and even then only after every object has been
copied and hash-verified. Going the other way is symmetric: `--to local`.

Once you've confirmed the new backend works (`storage verify --all` is green and
a restore succeeds), reclaim the old copy with a second run:

```sh
sudo -u hoard hoard-admin --config /etc/hoard/config.toml \
  storage migrate --to s3 --delete-source
```

`storage verify [--all | --sample N]` re-downloads objects and checks each one
hashes to its key — handy as a periodic bit-rot / integrity check, not just
after a migration. It exits nonzero if anything is missing or corrupt.

As with any S3 setup: the bucket only holds opaque zstd blob/chunk bytes. It is
**not restorable on its own** — without the server's SQLite database there is no
mapping from saves and versions to those bytes. Back up the DB too.

## Headless CLI

```sh
hoard config init --server http://YOUR_SERVER:12421
hoard login --token hoard_v1_...
hoard save create --game stardew-valley --label main
hoard backup <SAVE_ID> --from ~/.config/StardewValley/Saves --remember
hoard snapshots list <SAVE_ID>
hoard restore <SAVE_ID>
```
## The client side: where things live

The desktop app and the CLI are thin clients of a local service (`hoardd`, one
per user) that does the actual syncing and outlives the window. When something
looks wrong, these are the four places worth opening. Paths are Linux; on
Windows use `%APPDATA%\hoard\hoard\config\…` and
`%LOCALAPPDATA%\hoard\hoard\cache\…`, on macOS `~/Library/…`.

| What | Where |
| --- | --- |
| Service log | `~/.cache/hoard/logs/hoardd.log` |
| App log | `~/.cache/hoard/logs/agent.log` (daily) |
| CLI session | `~/.config/hoard/config.toml` (written by `hoard login`) |
| App session | `~/.config/hoard/desktop/session.toml` + the OS keyring |
| IPC socket | `$XDG_RUNTIME_DIR/hoard/hoardd.sock` |

The service starts on demand — any client spawns it if it isn't running — and
`hoard sync start` also registers it to start at login. Since 1.1.2 the 
engine is a component in its own right instead of a passenger inside 
the app bundle, so **an AppImage no longer costs you the login part**: 
As long as a `hoardd` exists somewhere stable — the one:
`curl -fsSL https://raw.githubusercontent.com/rleeon/hoard/main/web/static/install.sh | sh`
puts in, or the one a `.deb`/`.rpm` brings — the unit points at that copy and
sync starts with your session while the AppImage stays as the graphical face.
That is what makes game mode sync on SteamOS, Bazzite and the other atomic
images, where an AppImage is the only way to run the app.

It only falls back when the *sole* `hoardd` on the disk is the one inside the
AppImage: that binary lives in a temporary mount that doesn't survive a reboot,
so declaring the unit would point it at a path that won't exist next boot.
`hoard sync start` refuses with that reason instead of leaving you a unit that
dies with `203/EXEC`, and sync then runs whenever Hoard is open and no earlier.
Installing the core fixes it without giving up the AppImage.

### The service is running but "offline"

That banner means the service is up and its *engine* isn't. Since 1.1.1 the
window tells you which of these it is; on 1.1.0 it doesn't, and the log does:

```sh
tail -n 50 ~/.cache/hoard/logs/hoardd.log
```

- `no session` — the engine has no credentials. **On 1.1.0 this hits every
  self-hosted user who signed in through the app**: the service only read
  `config.toml`, which just `hoard login --token` writes. Upgrade to 1.1.1, or
  write that file by hand (`[server] url` + `[auth] token`) and restart the
  service.
- `the system keyring didn't answer` / `refused to hand over` — the keyring is
  locked, absent (headless, no D-Bus), or the item belongs to another binary.
  Signing in again rewrites it under the service.
- Anything else — the line carries the real error; that's what to put in a bug
  report.

To watch it in the foreground, stop the running one first (it owns the socket,
so a second copy just exits):

```sh
pkill -x hoardd && hoardd     # or ./squashfs-root/usr/bin/hoardd from an extracted AppImage
```
