#!/usr/bin/env bash
# Install, update or remove the Hoard plugin for Decky Loader.
#
#   ./decky-install.sh              install from this folder, or from the latest
#                                   release when the folder has no built plugin
#   ./decky-install.sh --uninstall
#
# Run it from Desktop Mode, as your normal user (not with sudo): it asks for the
# password itself, because Decky's plugins folder belongs to root.
set -euo pipefail

REPO="rleeon/hoard"
ASSET="hoard-decky.tar.gz"
NAME="hoard"
PLUGINS="$HOME/homebrew/plugins"
HERE="$(cd "$(dirname "$0")" && pwd)"

say() { printf '%s\n' "$*"; }
die() { printf 'decky-install: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -ne 0 ] || die "run it as your normal user, without sudo"
[ -d "$PLUGINS" ] || die "Decky Loader is not installed (no $PLUGINS). Install it first: https://decky.xyz"

# On a Steam Deck the default user has no password until you set one, and
# without it sudo cannot ask for anything.
if ! sudo -n true 2>/dev/null; then
    say "Decky's plugin folder belongs to root, so this needs your password."
    say "On a Steam Deck with no password yet, set one first with: passwd"
    sudo -v || die "sudo is needed to write to $PLUGINS"
fi

restart_decky() {
    sudo systemctl restart plugin_loader 2>/dev/null \
        || say "Restart Decky (or reboot) to load the change."
}

if [ "${1:-}" = "--uninstall" ]; then
    sudo rm -rf "${PLUGINS:?}/$NAME"
    restart_decky
    say "Hoard plugin removed."
    exit 0
fi

SRC="$HERE"
if [ ! -f "$SRC/plugin.json" ] || [ ! -f "$SRC/dist/index.js" ]; then
    TMP="$(mktemp -d)"
    trap 'rm -rf "$TMP"' EXIT
    say "Downloading the latest Hoard plugin..."
    curl -fsSL "https://github.com/$REPO/releases/latest/download/$ASSET" -o "$TMP/$ASSET" \
        || die "could not download $ASSET from the latest release"
    tar -xzf "$TMP/$ASSET" -C "$TMP"
    SRC="$TMP/hoard-decky"
    [ -f "$SRC/dist/index.js" ] || die "the download does not contain a built plugin"
fi

sudo rm -rf "${PLUGINS:?}/$NAME"
sudo mkdir -p "$PLUGINS/$NAME/dist"
sudo cp "$SRC/plugin.json" "$SRC/package.json" "$SRC/main.py" "$PLUGINS/$NAME/"
sudo cp "$SRC/dist/index.js" "$PLUGINS/$NAME/dist/"
[ -f "$SRC/LICENSE" ] && sudo cp "$SRC/LICENSE" "$PLUGINS/$NAME/"
restart_decky

say "Hoard plugin installed. Open the Quick Access menu (the ... button) and look under Decky."
if ! systemctl --user is-active --quiet hoard-sync 2>/dev/null; then
    say "Note: Hoard's sync service is not running here, so the plugin will show it as asleep."
    say "Install Hoard first if you have not: https://hoard.services/download"
fi
