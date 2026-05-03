#!/usr/bin/env bash
# Hoard server installer for Linux + systemd.
#
# Idempotent: running it twice is safe. It will:
#   1. Build release binaries (unless HOARD_SKIP_BUILD=1)
#   2. Create the `hoard` system user/group if missing
#   3. Install hoard-server and hoard-admin to /usr/local/bin
#   4. Place a starter config at /etc/hoard/config.toml (won't overwrite)
#   5. Create /var/lib/hoard with correct ownership
#   6. Install + enable the systemd unit
#
# Run as root: sudo ./deploy/scripts/install.sh

set -euo pipefail

# ---------------------------------------------------------------------------
# Tunables (override via env)
PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${BIN_DIR:-$PREFIX/bin}"
ETC_DIR="${ETC_DIR:-/etc/hoard}"
DATA_DIR="${DATA_DIR:-/var/lib/hoard}"
USER_NAME="${USER_NAME:-hoard}"
GROUP_NAME="${GROUP_NAME:-hoard}"
SERVICE_NAME="hoard-server.service"
SKIP_BUILD="${HOARD_SKIP_BUILD:-0}"

# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

log() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!! \033[0m%s\n' "$*" >&2; }
die() { printf '\033[1;31m!! \033[0m%s\n' "$*" >&2; exit 1; }

[[ $EUID -eq 0 ]] || die "must be run as root (try: sudo $0)"

# 1. Build (unless skipped) -------------------------------------------------
if [[ "$SKIP_BUILD" != "1" ]]; then
  log "Building release binaries (this may take a few minutes)…"
  command -v cargo >/dev/null || die "cargo not found in PATH (install Rust >= 1.75)"
  ( cd "$REPO_ROOT" && SQLX_OFFLINE=true cargo build --release -p hoard-server -p hoard-admin )
fi

SERVER_BIN="$REPO_ROOT/target/release/hoard-server"
ADMIN_BIN="$REPO_ROOT/target/release/hoard-admin"
[[ -x "$SERVER_BIN" ]] || die "missing $SERVER_BIN — build it first or unset HOARD_SKIP_BUILD"
[[ -x "$ADMIN_BIN" ]]  || die "missing $ADMIN_BIN  — build it first or unset HOARD_SKIP_BUILD"

# 2. User / group -----------------------------------------------------------
if ! getent group "$GROUP_NAME" >/dev/null; then
  log "Creating group $GROUP_NAME"
  groupadd --system "$GROUP_NAME"
fi
if ! id -u "$USER_NAME" >/dev/null 2>&1; then
  log "Creating system user $USER_NAME"
  useradd --system --gid "$GROUP_NAME" \
          --home-dir "$DATA_DIR" --no-create-home \
          --shell /usr/sbin/nologin \
          "$USER_NAME"
fi

# 3. Install binaries -------------------------------------------------------
log "Installing binaries to $BIN_DIR"
install -d -m 0755 "$BIN_DIR"
install -m 0755 "$SERVER_BIN" "$BIN_DIR/hoard-server"
install -m 0755 "$ADMIN_BIN"  "$BIN_DIR/hoard-admin"

# 4. Config -----------------------------------------------------------------
log "Setting up $ETC_DIR"
install -d -m 0755 "$ETC_DIR"
if [[ ! -f "$ETC_DIR/config.toml" ]]; then
  install -m 0644 "$REPO_ROOT/deploy/config.toml.example" "$ETC_DIR/config.toml"
  log "Wrote starter config to $ETC_DIR/config.toml — REVIEW BEFORE STARTING"
else
  log "Keeping existing $ETC_DIR/config.toml"
fi
# Permissive read for the service user; secrets (env file) get tighter perms.
chown -R root:"$GROUP_NAME" "$ETC_DIR"
chmod 0750 "$ETC_DIR"
chmod 0640 "$ETC_DIR/config.toml"

if [[ ! -f "$ETC_DIR/hoard.env" ]]; then
  cat >"$ETC_DIR/hoard.env" <<'EOF'
# Optional environment overrides for hoard-server.
# Keys use the HOARD__ prefix and double underscores to nest into TOML sections.
# Examples:
#   HOARD__SERVER__PORT=8080
#   HOARD__LOGGING__LEVEL=debug
EOF
  chown root:"$GROUP_NAME" "$ETC_DIR/hoard.env"
  chmod 0640 "$ETC_DIR/hoard.env"
fi

# 5. Data dir ---------------------------------------------------------------
log "Setting up $DATA_DIR"
install -d -m 0750 -o "$USER_NAME" -g "$GROUP_NAME" "$DATA_DIR"
for sub in data tmp trash; do
  install -d -m 0750 -o "$USER_NAME" -g "$GROUP_NAME" "$DATA_DIR/$sub"
done

# 6. systemd unit -----------------------------------------------------------
UNIT_SRC="$REPO_ROOT/deploy/systemd/$SERVICE_NAME"
UNIT_DST="/etc/systemd/system/$SERVICE_NAME"
log "Installing $UNIT_DST"
install -m 0644 "$UNIT_SRC" "$UNIT_DST"
systemctl daemon-reload
systemctl enable "$SERVICE_NAME" >/dev/null

# Done ----------------------------------------------------------------------
log "Install complete."
cat <<EOF

Next steps:

  1. Edit $ETC_DIR/config.toml (set public_url, retention, etc.)
  2. Initialize the database:
       sudo -u $USER_NAME hoard-admin --config $ETC_DIR/config.toml db migrate
  3. Create your first admin user:
       sudo -u $USER_NAME hoard-admin --config $ETC_DIR/config.toml \\
           user create youruser --admin --password 'SECRET'
  4. Start the service:
       systemctl start $SERVICE_NAME
       systemctl status $SERVICE_NAME
  5. Check logs:
       journalctl -u $SERVICE_NAME -f

EOF
