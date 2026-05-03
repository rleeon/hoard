#!/bin/sh
# Container entrypoint:
#   - Bootstrap config from the example if /etc/hoard/config.toml is missing
#     (handy when you forgot to mount one).
#   - Run pending migrations before exec'ing the server.
#   - exec the requested command (CMD).

set -eu

CFG="${HOARD_CONFIG_PATH:-/etc/hoard/config.toml}"
EXAMPLE="/etc/hoard/config.toml.example"

if [ ! -f "$CFG" ]; then
  if [ -f "$EXAMPLE" ]; then
    echo "entrypoint: no config at $CFG, copying from example (review for production!)" >&2
    cp "$EXAMPLE" "$CFG" 2>/dev/null || {
      echo "entrypoint: cannot write $CFG (read-only mount?). Mount one explicitly." >&2
      exit 1
    }
  else
    echo "entrypoint: missing $CFG and no example available" >&2
    exit 1
  fi
fi

# Only run migrations when we're actually starting the server. This keeps
# `docker compose exec server hoard-admin ...` (and other one-off commands)
# fast and avoids a recursive admin call.
case "${1:-}" in
  hoard-server|/usr/local/bin/hoard-server)
    echo "entrypoint: running database migrations…" >&2
    hoard-admin --config "$CFG" db migrate
    ;;
esac

exec "$@"
