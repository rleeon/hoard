#!/bin/sh
# Hoard CLI installer, Linux & macOS
#
#   curl -fsSL https://hoard.services/install.sh | sh
#
# Detects your OS/arch, downloads the matching core tarball from the latest
# GitHub release, verifies its SHA-256, and installs to ~/.local/bin (no sudo).
#
# It installs the CORE: `hoardd` (the sync engine, which runs as a background
# service) and `hoard` (the terminal face). They ship and update together
# always, `hoard` is a thin client of `hoardd` since 1.1.0, so either one on its
# own is a program that cannot do anything.
#
# Then it hands off to `hoard install`, which decides whether this machine also
# wants the desktop app and fetches it. A NAS or a server stops at the core; a
# desktop or a Steam Deck gets the app too, in the same pass and at the same
# version. That decision lives in Rust (`hoard_agent::install`) so the installer,
# `hoard upgrade` and the in-app updater cannot drift apart.
#
# Overridable with env vars:
#
#   HOARD_VERSION=1.0.2            pin a version instead of "latest"
#   HOARD_INSTALL_DIR=/opt/bin     install somewhere else
#   HOARD_HEADLESS=1               core only, never the desktop app
#   HOARD_WITH_DESKTOP=1           force the desktop app even if undetected
#
# After install:  hoard login && hoard sync start
set -eu

REPO="rleeon/hoard"

# ---- pretty output (only when stdout is a tty) -----------------------------
if [ -t 1 ]; then
  BOLD=$(printf '\033[1m'); DIM=$(printf '\033[2m'); GREEN=$(printf '\033[32m')
  YELLOW=$(printf '\033[33m'); RED=$(printf '\033[31m'); RESET=$(printf '\033[0m')
else
  BOLD=''; DIM=''; GREEN=''; YELLOW=''; RED=''; RESET=''
fi
say()  { printf '%s\n' "$*"; }
info() { printf '%s==>%s %s\n' "$GREEN" "$RESET" "$*"; }
warn() { printf '%swarning:%s %s\n' "$YELLOW" "$RESET" "$*" >&2; }
fail() { printf '%serror:%s %s\n' "$RED" "$RESET" "$*" >&2; exit 1; }

command -v tar >/dev/null 2>&1 || fail "tar is required but not found."

# ---- fetch helper (curl or wget) -------------------------------------------
if command -v curl >/dev/null 2>&1; then
  dl() { curl -fsSL "$1" -o "$2"; }
  dl_stdout() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  dl() { wget -qO "$2" "$1"; }
  dl_stdout() { wget -qO- "$1"; }
else
  fail "need curl or wget to download."
fi

# ---- detect platform -------------------------------------------------------
os=$(uname -s)
arch=$(uname -m)
case "$os" in
  Linux)  os=linux ;;
  Darwin) os=macos ;;
  *) fail "unsupported OS: $os (Linux and macOS only; on Windows use install.ps1)." ;;
esac
case "$arch" in
  x86_64|amd64)   arch=x86_64 ;;
  aarch64|arm64)  arch=aarch64 ;;
  *) fail "unsupported architecture: $arch" ;;
esac
if [ "$os" = macos ] && [ "$arch" = x86_64 ]; then
  fail "no Intel-macOS CLI build. Build from source, or self-host the server on Linux."
fi
platform="${os}-${arch}"

# ---- resolve version -------------------------------------------------------
ver="${HOARD_VERSION:-}"
if [ -z "$ver" ]; then
  info "Looking up the latest release…"
  tag=$(dl_stdout "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' | head -1 \
        | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
  [ -n "$tag" ] || fail "could not determine the latest version (GitHub API rate limit?). Set HOARD_VERSION."
  ver="${tag#v}"
fi
ver="${ver#v}"

base="https://github.com/$REPO/releases/download/v${ver}"
asset="hoard-${ver}-${platform}.tar.gz"
url="$base/$asset"

# ---- download --------------------------------------------------------------
tmp=$(mktemp -d 2>/dev/null || mktemp -d -t hoard)
trap 'rm -rf "$tmp"' EXIT INT TERM

info "Downloading ${BOLD}${asset}${RESET}"
dl "$url" "$tmp/pkg.tar.gz" || fail "download failed: $url"

# ---- verify sha256 ---------------------------------------------------------
if dl "$url.sha256" "$tmp/pkg.sha256" 2>/dev/null; then
  expected=$(awk '{print $1}' "$tmp/pkg.sha256")
  if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$tmp/pkg.tar.gz" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$tmp/pkg.tar.gz" | awk '{print $1}')
  else
    actual=''
  fi
  if [ -z "$actual" ]; then
    warn "no sha256 tool found — skipping checksum verification."
  elif [ "$actual" != "$expected" ]; then
    fail "checksum mismatch! expected $expected, got $actual. Aborting."
  else
    info "Checksum verified."
  fi
else
  warn "no .sha256 published for this asset — skipping verification."
fi

# ---- extract ---------------------------------------------------------------
tar -xzf "$tmp/pkg.tar.gz" -C "$tmp"
root="$tmp/hoard-${ver}-${platform}"
# Both halves of the core, checked before anything is written. Installing one
# without the other is what this whole layout exists to prevent: `hoard` with no
# `hoardd` is a client with nothing to talk to, and it fails at the point of use
# rather than here.
for want in hoard hoardd; do
  [ -f "$root/$want" ] || fail "the archive did not contain '$want' — refusing to install half of the core."
done

# ---- install ---------------------------------------------------------------
dir="${HOARD_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$dir"
put() {
  if command -v install >/dev/null 2>&1; then
    install -m 0755 "$root/$1" "$dir/$1"
  else
    cp "$root/$1" "$dir/$1" && chmod 0755 "$dir/$1"
  fi
}
put hoard
put hoardd
info "Installed ${BOLD}hoard ${ver}${RESET} (engine + CLI) → ${dir}"

# ---- PATH check ------------------------------------------------------------
on_path=no
case ":$PATH:" in
  *":$dir:"*) on_path=yes ;;
esac

if [ "$on_path" = no ]; then
  # Pick the rc file for the user's login shell.
  case "${SHELL:-}" in
    */zsh)  rc="$HOME/.zshrc" ;;
    */bash) rc="$HOME/.bashrc" ;;
    *)      rc="$HOME/.profile" ;;
  esac
  line="export PATH=\"$dir:\$PATH\""
  if [ -f "$rc" ] && grep -Fq "$line" "$rc" 2>/dev/null; then
    :
  else
    printf '\n# Added by the Hoard CLI installer\n%s\n' "$line" >> "$rc"
  fi
  say ""
  warn "$dir is not on your PATH yet."
  say "  Added it to ${BOLD}$rc${RESET}. Open a new terminal, or run:"
  say "    ${BOLD}$line${RESET}"
fi

# ---- the rest of Hoard -----------------------------------------------------
# The core is in; `hoard install` takes it from here. It decides what else this
# machine wants and fetches it at the SAME version, so the pieces never drift.
# Everything below is best-effort on purpose: a failure here leaves a working
# core behind, and the user can re-run `hoard install` once the cause is fixed.
say ""
# Both flags accumulate rather than overwrite, so setting HOARD_HEADLESS=1 and
# HOARD_WITH_DESKTOP=1 together reaches `hoard install` as the contradiction it
# is and clap rejects it (`conflicts_with`). Assigning instead of appending,
# which this did, silently dropped --headless and pulled a desktop app onto a
# machine that asked not to have one. install.ps1 has always accumulated; these
# two must not disagree about what the same env vars mean.
rest_args=""
[ "${HOARD_HEADLESS:-}" = "1" ]     && rest_args="$rest_args --headless"
[ "${HOARD_WITH_DESKTOP:-}" = "1" ] && rest_args="$rest_args --with-desktop"

# stdin is the script itself inside `curl … | sh`, so anything that might prompt
# has to be told it can't. `hoard install` picks a non-interactive delivery when
# it sees this.
# The version is passed explicitly even though the binary just placed is already
# that one: it writes the contract down, and if somebody reorders this and a `hoard`
# of another version gets installed, it fails instead of mixing pieces of two
# releases.
HOARD_NONINTERACTIVE=1 "$dir/hoard" install --version "$ver" $rest_args </dev/null || {
  warn "the core is installed, but setting up the rest didn't finish."
  say  "  Re-run it when you're ready:  ${BOLD}hoard install${RESET}"
}

say ""
info "Done. Next steps:"
say "  ${BOLD}hoard login${RESET}       ${DIM}# sign in (Cloud or self-hosted)${RESET}"
say "  ${BOLD}hoard sync start${RESET}  ${DIM}# run the background sync service${RESET}"
say ""
say "${DIM}Docs: https://hoard.services/cli${RESET}"
