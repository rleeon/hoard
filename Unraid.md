# Unraid

Hoard's Community Applications template. It installs the **self-hosted server**
on your NAS; the machines you play on run the desktop app or the CLI and sync to
it.

## Installing

**From Community Applications:** *Apps* → search **Hoard** → *Install*. Fill in
an admin username and a password, press *Apply*.

**By hand,** if it is not in the Apps tab yet. From the Unraid terminal:

```sh
wget -O /boot/config/plugins/dockerMan/templates-user/hoard.xml https://raw.githubusercontent.com/rleeon/hoard/main/templates/hoard.xml
```

Then **Docker** → *Add Container* → pick **Hoard** from the *Template* dropdown.

Either way, the first start writes its own config, creates the admin account you
named, and prints a device token in the container log **once**. Copy it, open
the desktop app, pick *Self-Host*, and give it `http://TOWER-IP:12421` plus
that token. The *WebUI* button opens the web panel, where the username and
password you typed get you in.

To add another PC later, or to give someone else an account, open the *WebUI*
and use the **Users** tab: *New token* issues one for a machine, *New user*
creates an account and offers its first token straight afterwards. A token is
shown once, so copy it before closing.

The same jobs from the container's *Console*, if you prefer a terminal:

```sh
hoard-admin token create YOUR-USER --device 'living-room PC'
hoard-admin user create SOMEONE
```

## What ends up on your disk

Two folders under `/mnt/user/appdata/hoard/`:

- `data/` holds the database and every version of every save. **This is the one
  to back up.**
- `config/` holds `config.toml`, for raising limits or pointing storage at an
  S3-compatible bucket. It is written for you on first start, and the
  [self-hosting guide](SELF-HOST_GUIDE.md) explains what is in it.

The container pulls `ghcr.io/rleeon/hoard:latest` (amd64 and arm64), built on
every release.

## Updating

Unraid does it for you. The *Docker* tab marks the container once a new image is
on the registry, and *Apply update* pulls it and recreates the container. Both
folders above are bind mounts, so nothing of yours lives in the layer that gets
thrown away, and the server applies any pending database migration on start.

`hoard-server upgrade` is not the command here, and neither is the desktop app's
*Upgrade server* button. Both are built around swapping the binary in place,
which is what a bare-metal systemd install needs and what the next container
recreate would discard; run inside the container, `upgrade` says so and points
at the image instead.

## Maintaining the template

`templates/hoard.xml` describes the image built from [`deploy/docker/`](deploy/docker),
so the two move together: a field added here that the container does not read is
a field that silently does nothing.

The template points at `:latest`, so a change that depends on new container
behaviour can only be published after a release carries that behaviour into the
image. To test one before then, run the *Publish container image* workflow by
hand (it pushes `:edge`) and point a scratch copy of the template at that tag.

The install path can be tried without a NAS. Docker creates missing bind mounts
owned by root, which is the part that used to break. The host port is 12431 so
the test does not collide with a real server already on 12421 — the panel is at
<http://127.0.0.1:12431/panel>:

```sh
docker run -d --name hoard-unraid -p 12431:12421 -v /tmp/hoard-unraid/data:/var/lib/hoard -v /tmp/hoard-unraid/config:/etc/hoard -e PUID=99 -e PGID=100 -e HOARD_ADMIN_USERNAME=alice -e HOARD_ADMIN_PASSWORD=hunter2hunter2 ghcr.io/rleeon/hoard:latest
docker logs hoard-unraid
```

`ca_profile.xml` must stay in the repository root, and templates in
`templates/`; Community Applications requires both. Submissions and re-scans go
through <https://ca.unraid.net/submit/new>.
