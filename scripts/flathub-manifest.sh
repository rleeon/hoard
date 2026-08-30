#!/usr/bin/env bash
# Turn the local Flatpak manifest into one Flathub can actually build.
#
# The manifest this reads feeds `scripts/build-flatpak.sh`, which builds the
# .deb first and leaves it in `flatpak/` for flatpak-builder to unpack.
# Flathub builds nothing of ours: it clones the `flathub/<app-id>` repo and
# runs flatpak-builder there, so every source has to be remote and carry a
# checksum. A `path:` source pointing at a file we build locally is the one
# thing that recipe cannot be.
#
# So this rewrites that single source into the release asset plus its hash and
# leaves the rest of the manifest alone — one file to maintain, not two that
# drift.
#
# Usage:
#	 scripts/flathub-manifest.sh v1.1.5	 # writes flatpak/flathub/<id>.yml
#	 scripts/flathub-manifest.sh v1.1.5 -	 # writes to stdout
set -euo pipefail

HOARD="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="services.hoard.saves"
REPO="rleeon/hoard"

tag="${1:-}"
[ -n "$tag" ] || { echo "usage: $(basename "$0") <tag> [-]" >&2; exit 1; }
version="${tag#v}"
asset="Hoard_${version}_amd64.deb"
url="https://github.com/$REPO/releases/download/$tag/$asset"

command -v gh >/dev/null 2>&1 || { echo "ERROR: 'gh' is required but not on PATH." >&2; exit 1; }

# The release publishes a `.sha256` beside every asset, so the hash comes from
# the same place the file does and nothing here has to download tens of MB to
# work it out.
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
gh release download "$tag" --repo "$REPO" --pattern "$asset.sha256" --dir "$tmp" >/dev/null 2>&1 || {
	echo "ERROR: $tag has no $asset.sha256 — is that a released tag?" >&2
	exit 1
}
sha="$(awk '{print $1; exit}' "$tmp/$asset.sha256")"
[ "${#sha}" -eq 64 ] || { echo "ERROR: '$sha' doesn't look like a sha256." >&2; exit 1; }

# Swap the local source for the release one. If the manifest stops carrying
# that source this stops rather than emitting something that builds the wrong
# thing.
src="$HOARD/flatpak/$APP_ID.yml"
grep -q '^        path: hoard\.deb$' "$src" || {
	echo "ERROR: $src no longer carries the local .deb source this rewrites." >&2
	exit 1
}
generated="$(awk -v url="$url" -v sha="$sha" '
	/^        path: hoard\.deb$/ {
		print "        url: " url
		print "        sha256: " sha
		print "        dest-filename: hoard.deb"
		next
	}
	{ print }
' "$src")"

if [ "${2:-}" = "-" ]; then
	printf '%s\n' "$generated"
	exit 0
fi

out="$HOARD/flatpak/flathub"
mkdir -p "$out"
printf '%s\n' "$generated" > "$out/$APP_ID.yml"
echo "Wrote $out/$APP_ID.yml for $tag"
echo
echo "The Flathub repo also needs, beside it:"
echo "	flatpak/$APP_ID.metainfo.xml"
echo "	flatpak/shared-modules/"
