#!/usr/bin/env bash
# Build Hoard as a Flatpak for local testing.
#
# Follows the "closed source" recipe from the Tauri docs
# (https://v2.tauri.app/distribute/flatpak/): build the app's own .deb the
# normal way, then have flatpak-builder unpack it into /app instead of
# re-building Rust + the frontend inside the sandbox. That keeps this script
# consistent with everything else here (scripts/build-sidecar.sh, CI) instead
# of maintaining a second, sandboxed build path with its own vendored
# cargo/node sources.
#
# Usage:
#	 scripts/build-flatpak.sh						# build + --install --user, ready to `flatpak run services.hoard.saves`
#	 scripts/build-flatpak.sh --bundle	 # also produce flatpak/hoard.flatpak (single-file, for sharing)
set -euo pipefail

HOARD="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FP="$HOARD/flatpak"
APP_ID="services.hoard.saves"
RUNTIME_VERSION="50"

want_bundle=0
for arg in "$@"; do
	case "$arg" in
		--bundle) want_bundle=1 ;;
		*) echo "unknown argument: $arg" >&2; exit 1 ;;
	esac
done

for tool in flatpak ar tar pnpm cargo; do
	command -v "$tool" >/dev/null 2>&1 || {
		echo "ERROR: '$tool' is required but not on PATH." >&2
		exit 1
	}
done

# flatpak-builder from the distro if it's there, and Flathub's own otherwise.
# The packaged one is what Flathub's docs point people at, it needs no root to
# install, and on a distro whose package lags the manifest's runtime it's the
# only one that can read it.
if command -v flatpak-builder >/dev/null 2>&1; then
	builder=(flatpak-builder)
elif flatpak info --user org.flatpak.Builder >/dev/null 2>&1; then
	builder=(flatpak run org.flatpak.Builder)
else
	echo "Installing org.flatpak.Builder (no flatpak-builder on PATH)..."
	flatpak install --user --noninteractive -y flathub org.flatpak.Builder
	builder=(flatpak run org.flatpak.Builder)
fi

if ! flatpak remote-list --user 2>/dev/null | grep -q '^flathub'; then
	echo "Adding the flathub remote (--user)..."
	flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
fi

if ! flatpak info --user "org.gnome.Platform//$RUNTIME_VERSION" >/dev/null 2>&1; then
	echo "Installing org.gnome.Platform//$RUNTIME_VERSION + org.gnome.Sdk//$RUNTIME_VERSION..."
	flatpak install --user --noninteractive -y flathub \
		"org.gnome.Platform//$RUNTIME_VERSION" "org.gnome.Sdk//$RUNTIME_VERSION"
fi

# The three externalBin sidecars every desktop bundle needs (see
# scripts/build-sidecar.sh for why: overlay, sync daemon, CLI).
echo "==> Building sidecars"
bash "$HOARD/scripts/build-sidecar.sh"

echo "==> Building frontend"
pnpm --dir "$HOARD/crates/hoard-desktop/ui" install --frozen-lockfile
pnpm --dir "$HOARD/crates/hoard-desktop/ui" build

# tauri-cli isn't a checked-in dependency (CI drives builds through the
# tauri-apps/tauri-action GitHub Action instead), so pull it via `pnpm dlx`
# rather than requiring a global install or touching package.json/lockfiles.
echo "==> Building .deb (cargo tauri build --bundles deb)"
( cd "$HOARD/crates/hoard-desktop" && pnpm dlx @tauri-apps/cli@2 build --bundles deb )

deb="$(find "$HOARD/target/release/bundle/deb" -maxdepth 1 -name '*.deb' -print -quit)"
[ -n "$deb" ] || { echo "ERROR: no .deb found under target/release/bundle/deb" >&2; exit 1; }
echo "Using $deb"
cp "$deb" "$FP/hoard.deb"

echo "==> Running flatpak-builder"
rm -rf "$FP/build-dir"
"${builder[@]}" --force-clean --user --install --repo="$FP/repo" \
	"$FP/build-dir" "$FP/$APP_ID.yml"

rm -f "$FP/hoard.deb"

if [ "$want_bundle" -eq 1 ]; then
	echo "==> Building single-file bundle"
	flatpak build-bundle "$FP/repo" "$FP/hoard.flatpak" "$APP_ID"
	echo "Bundle: $FP/hoard.flatpak"
fi

echo
echo "Done. Run it with:"
echo "	flatpak run $APP_ID"
