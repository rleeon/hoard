#!/bin/sh
# Hoard installer, Linux & macOS
#
#   curl -fsSL https://hoard.services/install.sh | sh
#
# Downloads the core for this machine (`hoardd`, the sync engine, and `hoard`,
# the terminal), checks its SHA-256 and hands over to `hoard install`, which
# does everything else: puts the core in ~/.local/bin, takes over from any older
# Hoard on this machine, starts the sync service, and on a desktop installs the
# app too. That logic lives in Rust so this script, Hoard Setup and the app's
# own updater can't drift apart; this file only fetches and checks.
#
# Overridable with env vars:
#
#   HOARD_VERSION=1.2.2            pin a version instead of the latest
#   HOARD_INSTALL_DIR=/opt/bin     put the core somewhere else
#   HOARD_HEADLESS=1               core only, never the desktop app
#   HOARD_WITH_DESKTOP=1           the desktop app even if nothing looks graphical
set -eu

REPO="rleeon/hoard"
RELEASES="https://github.com/$REPO/releases"

if [ -t 1 ]; then
  BOLD=$(printf '\033[1m'); GREEN=$(printf '\033[32m'); RED=$(printf '\033[31m'); RESET=$(printf '\033[0m')
else
  BOLD=''; GREEN=''; RED=''; RESET=''
fi
info() { printf '%s==>%s %s\n' "$GREEN" "$RESET" "$*"; }
fail() { printf '%serror:%s %s\n' "$RED" "$RESET" "$*" >&2; exit 1; }

command -v tar >/dev/null 2>&1 || fail "tar is required but not found."
if command -v curl >/dev/null 2>&1; then
  dl() { curl -fsSL "$1" -o "$2"; }
  # Where a URL ends up after redirects, without downloading it.
  final_url() { curl -fsSLI -o /dev/null -w '%{url_effective}' "$1"; }
elif command -v wget >/dev/null 2>&1; then
  dl() { wget -qO "$2" "$1"; }
  final_url() { wget -S --spider "$1" 2>&1 | awk '/^  Location: /{u=$2} END{print u}'; }
else
  fail "need curl or wget to download."
fi
if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  fail "need sha256sum or shasum to check the download."
fi

case "$(uname -s)" in
  Linux)  os=linux ;;
  Darwin) os=macos ;;
  *) fail "unsupported OS: $(uname -s) (on Windows use install.ps1)." ;;
esac
case "$(uname -m)" in
  x86_64|amd64)  arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) fail "unsupported architecture: $(uname -m)" ;;
esac
[ "$os-$arch" = macos-x86_64 ] && fail "there is no Intel macOS build."

# The latest version is read off where `releases/latest` redirects, not from
# GitHub's API: the API allows 60 requests an hour per address, and an office
# behind one address used to run out of installs before lunch.
ver="${HOARD_VERSION:-}"
if [ -z "$ver" ]; then
  tag=$(final_url "$RELEASES/latest" | sed 's|.*/tag/||')
  case "$tag" in
    v[0-9]*|[0-9]*) ;;
    *) fail "couldn't find out the latest version. Set HOARD_VERSION and retry." ;;
  esac
  ver="$tag"
fi
ver="${ver#v}"

asset="hoard-$ver-$os-$arch.tar.gz"
url="$RELEASES/download/v$ver/$asset"
tmp=$(mktemp -d 2>/dev/null || mktemp -d -t hoard)
trap 'rm -rf "$tmp"' EXIT INT TERM

info "Downloading ${BOLD}$asset${RESET}"
dl "$url" "$tmp/core.tar.gz" || fail "download failed: $url"
dl "$url.sha256" "$tmp/core.sha256" || fail "no checksum published for $asset; not installing it."
expected=$(awk '{print $1}' "$tmp/core.sha256")
[ "$(sha256 "$tmp/core.tar.gz")" = "$expected" ] || fail "checksum mismatch for $asset. Not installing it."

tar -xzf "$tmp/core.tar.gz" -C "$tmp"
bin="$tmp/hoard-$ver-$os-$arch/hoard"
[ -x "$bin" ] || fail "the archive has no hoard binary."

set --
[ "${HOARD_HEADLESS:-}" = "1" ] && set -- "$@" --headless
[ "${HOARD_WITH_DESKTOP:-}" = "1" ] && set -- "$@" --with-desktop

# Up to 1.2.1, `hoard install` expected to be run from where it lives already,
# so a pinned older version gets put in place first, the way this script used
# to do it.
older_than() { # older_than A B: is version A older than B?
  [ "$1" = "$2" ] && return 1
  [ "$(printf '%s\n%s\n' "$1" "$2" | awk -F. '{printf "%05d%05d%05d %s\n", $1, $2, $3, $0}' | sort | head -1 | cut -d' ' -f2)" = "$1" ]
}
if older_than "$ver" 1.2.2; then
  dir="${HOARD_INSTALL_DIR:-$HOME/.local/bin}"
  mkdir -p "$dir"
  for b in hoard hoardd; do cp "$tmp/hoard-$ver-$os-$arch/$b" "$dir/.$b.new" && mv -f "$dir/.$b.new" "$dir/$b"; done
  bin="$dir/hoard"
fi

# stdin is the script itself inside `curl … | sh`: nobody can answer a prompt.
info "Installing Hoard $ver"
HOARD_NONINTERACTIVE=1 "$bin" install "$@" </dev/null

printf '\n'
info "Done. Next: ${BOLD}hoard login${RESET}"
