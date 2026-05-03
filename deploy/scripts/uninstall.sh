#!/usr/bin/env bash
# Hoard server uninstaller.
#
# By default this removes the service, binaries, and config — but KEEPS the
# data directory. Pass --purge to also delete /var/lib/hoard (irreversible).
#
# Run as root: sudo ./deploy/scripts/uninstall.sh [--purge]

set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${BIN_DIR:-$PREFIX/bin}"
ETC_DIR="${ETC_DIR:-/etc/hoard}"
DATA_DIR="${DATA_DIR:-/var/lib/hoard}"
USER_NAME="${USER_NAME:-hoard}"
GROUP_NAME="${GROUP_NAME:-hoard}"
SERVICE_NAME="hoard-server.service"

PURGE=0
for arg in "$@"; do
  case "$arg" in
    --purge) PURGE=1 ;;
    -h|--help)
      sed -n '2,8p' "$0"
      exit 0 ;;
    *) printf 'unknown option: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

log() { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!! \033[0m%s\n' "$*" >&2; }

[[ $EUID -eq 0 ]] || { echo "must be run as root" >&2; exit 1; }

# Stop + disable service ---------------------------------------------------
if systemctl list-unit-files | grep -q "^$SERVICE_NAME"; then
  log "Stopping and disabling $SERVICE_NAME"
  systemctl stop "$SERVICE_NAME" 2>/dev/null || true
  systemctl disable "$SERVICE_NAME" 2>/dev/null || true
fi
rm -f "/etc/systemd/system/$SERVICE_NAME"
systemctl daemon-reload

# Binaries -----------------------------------------------------------------
log "Removing binaries"
rm -f "$BIN_DIR/hoard-server" "$BIN_DIR/hoard-admin"

# Config -------------------------------------------------------------------
log "Removing $ETC_DIR"
rm -rf "$ETC_DIR"

# Data ---------------------------------------------------------------------
if [[ "$PURGE" -eq 1 ]]; then
  warn "Purging data directory $DATA_DIR (this CANNOT be undone)"
  rm -rf "$DATA_DIR"
  if id -u "$USER_NAME" >/dev/null 2>&1; then
    log "Removing user $USER_NAME"
    userdel "$USER_NAME" 2>/dev/null || true
  fi
  if getent group "$GROUP_NAME" >/dev/null; then
    log "Removing group $GROUP_NAME"
    groupdel "$GROUP_NAME" 2>/dev/null || true
  fi
else
  warn "Keeping $DATA_DIR — pass --purge to remove it (and the hoard user)."
fi

log "Uninstall complete."
