#!/bin/sh
# Container entrypoint:
#   - Take the mounted directories over (as root) and drop straight back to an
#     unprivileged user. A NAS bind mount arrives owned by root, and the server
#     is not root; without this the first write fails and the container loops.
#   - Bootstrap config from the image's example if there is none at $CFG.
#   - Create the first admin (and print a device token) when the database is
#     empty and HOARD_ADMIN_USERNAME/PASSWORD are set, the one-click path for
#     app-store installs, where nobody is going to open a shell.
#   - Run pending migrations before exec'ing the server.
#   - exec the requested command (CMD).

set -eu

CFG="${HOARD_CONFIG_PATH:-/etc/hoard/config.toml}"
# The example lives outside /etc/hoard on purpose: that directory is where
# people mount their own config, and a mount HIDES whatever the image shipped
# underneath it. Keeping the copy elsewhere is what lets an empty mounted
# directory bootstrap itself.
EXAMPLE="/usr/share/hoard/config.toml.example"
DATA_DIR="${HOARD__STORAGE__DATA_DIR:-/var/lib/hoard}"
# The image's own user, and the knobs NAS front-ends expect to set instead.
PUID="${PUID:-10001}"
PGID="${PGID:-10001}"

# ---- 1. Ownership, then drop root -----------------------------------------
# Only reachable when the container was started as root (the default). Running
# it with `--user`/compose `user:` skips this block entirely and nothing here
# ever gets the chance to touch ownership.
if [ "$(id -u)" = 0 ] && [ "$PUID" != 0 ]; then
    mkdir -p "$DATA_DIR" "$(dirname "$CFG")"

    for dir in "$DATA_DIR" "$(dirname "$CFG")"; do
        # Recursing over a large snapshot store on every restart would be a
        # slow way to change nothing, so the top directory's owner is taken as
        # the answer for everything below it.
        if [ "$(stat -c %u "$dir")" != "$PUID" ]; then
            echo "entrypoint: taking ownership of $dir ($PUID:$PGID)…" >&2
            chown -R "$PUID:$PGID" "$dir" || {
                echo "entrypoint: cannot chown $dir — if it lives on a share that" >&2
                echo "entrypoint: forbids it, make it writable for $PUID:$PGID by hand." >&2
            }
        else
            # The directory is already ours, but single files in it may not be:
            # a `docker exec … hoard-admin …` runs as root, and if it recreates
            # the SQLite WAL the server cannot write it on the next start. Only
            # the top level, blobs are never touched by hand, and walking a
            # snapshot store on every boot would cost more than it fixes.
            find "$dir" -maxdepth 1 ! -uid "$PUID" -exec chown "$PUID:$PGID" {} + 2>/dev/null || true
        fi
    done

    exec setpriv --reuid "$PUID" --regid "$PGID" --clear-groups "$0" "$@"
fi

# ---- 2. Configuration ------------------------------------------------------
if [ ! -f "$CFG" ]; then
    if [ -f "$EXAMPLE" ] && cp "$EXAMPLE" "$CFG" 2>/dev/null; then
        echo "entrypoint: no config at $CFG, copied the bundled example (review it for production!)" >&2
    else
        echo "entrypoint: no config at $CFG and none could be written there." >&2
        echo "entrypoint: mount the directory read-write, or put a config in it yourself:" >&2
        echo "entrypoint:   mkdir -p deploy/docker/config" >&2
        echo "entrypoint:   cp deploy/config.toml.example deploy/docker/config/config.toml" >&2
        echo "entrypoint: the defaults work as-is; edit it if you want, then start again." >&2
        exit 1
    fi
fi

# ---- 3. Migrations + first-run bootstrap -----------------------------------
# Only when we are actually starting the server. This keeps one-off commands
# (`docker exec … hoard-admin …`) fast and avoids a recursive admin call.
case "${1:-}" in
hoard-server | /usr/local/bin/hoard-server)
    echo "entrypoint: running database migrations…" >&2
    hoard-admin --config "$CFG" db migrate

    # An empty database is the only moment this can run: after that the
    # variables are ignored, so leaving them set (or clearing them) changes
    # nothing. The password is never used to log in again by itself, it is
    # what the panel asks for.
    if [ -n "${HOARD_ADMIN_USERNAME:-}" ] && [ -n "${HOARD_ADMIN_PASSWORD:-}" ] &&
        hoard-admin --config "$CFG" user list | grep -q '^No users\.$'; then
        echo "entrypoint: empty database, creating the first admin…" >&2
        hoard-admin --config "$CFG" user create "$HOARD_ADMIN_USERNAME" \
            --admin --password "$HOARD_ADMIN_PASSWORD"
        # The desktop app is set up with a token, not a password, and there is
        # no way to mint one from the panel, so the install that never opens a
        # shell gets its token here, once, in the log.
        token=$(hoard-admin --config "$CFG" token create "$HOARD_ADMIN_USERNAME" \
            --device "${HOARD_ADMIN_DEVICE:-first device}" |
            sed -n 's/^Token: //p')
        echo "" >&2
        echo "  ┌───────────────────────────────────────────────────────────────" >&2
        echo "  │ Hoard is ready. Copy this token — it is shown ONCE:" >&2
        echo "  │" >&2
        echo "  │   $token" >&2
        echo "  │" >&2
        echo "  │ In the desktop app pick \"Self-Host\", give it this server's" >&2
        echo "  │ address and paste the token. The web panel at /panel takes the" >&2
        echo "  │ username and password instead." >&2
        echo "  └───────────────────────────────────────────────────────────────" >&2
        echo "" >&2
    fi
    ;;
esac

exec "$@"
