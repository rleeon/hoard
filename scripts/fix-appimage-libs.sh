#!/usr/bin/env bash
#
# Strip host-provided graphics/Wayland libraries out of the built AppImage.
#
# Tauri's AppImage bundler copies WebKitGTK's dependency closure into the
# bundle, which drags in libwayland-client/-egl/-cursor, libEGL, libGL(X),
# libgbm and libdrm. On the build runner (Ubuntu 22.04) those match the host,
# but on a newer or immutable target, SteamOS / Bazzite / recent Fedora, the
# bundled libwayland-client no longer matches the running compositor, so
# WebKitGTK fails at `could not create default EGL display: EGL_BAD_PARAMETER`
# and the window renders solid white.
#
# These libraries are exactly the ones the AppImage project's own excludelist
# marks "must come from the host". We extract the AppImage, delete them so the
# dynamic loader falls through to the system copies, and repackage. The `.deb`
# and `.rpm` are unaffected (they depend on system libs already), and the
# in-app updater ships the `.deb` on Linux, so the repackaged AppImage needs no
# updater signature, `release.yml` minisigns it afterwards.
#
# Usage: scripts/fix-appimage-libs.sh <bundle-root>
#   <bundle-root> defaults to target/release/bundle; the script also probes
#   target/*/release/bundle so it works with or without an explicit --target.
set -euo pipefail

BUNDLE_ROOT="${1:-}"
if [ -z "$BUNDLE_ROOT" ] || [ ! -d "$BUNDLE_ROOT" ]; then
  for cand in target/release/bundle target/*/release/bundle; do
    [ -d "$cand" ] && BUNDLE_ROOT="$cand" && break
  done
fi

APPIMAGE="$(find "$BUNDLE_ROOT/appimage" -maxdepth 1 -name '*.AppImage' -type f 2>/dev/null | head -n1 || true)"
if [ -z "$APPIMAGE" ]; then
  echo "fix-appimage-libs: no .AppImage under $BUNDLE_ROOT/appimage — nothing to do"
  exit 0
fi
echo "fix-appimage-libs: patching $APPIMAGE"

# Libraries that must resolve from the host, never from the bundle.
PATTERNS=(
  'libwayland-client.so*'
  'libwayland-cursor.so*'
  'libwayland-egl.so*'
  'libwayland-server.so*'
  'libEGL.so*'
  'libGL.so*'
  'libGLX.so*'
  'libGLdispatch.so*'
  'libOpenGL.so*'
  'libgbm.so*'
  'libdrm.so*'
)

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ABS_APPIMAGE="$(readlink -f "$APPIMAGE")"

pushd "$WORK" >/dev/null
chmod +x "$ABS_APPIMAGE"
"$ABS_APPIMAGE" --appimage-extract >/dev/null

removed=0
for pat in "${PATTERNS[@]}"; do
  while IFS= read -r -d '' lib; do
    echo "  removing $(basename "$lib")"
    rm -f "$lib"
    removed=$((removed + 1))
  done < <(find squashfs-root -type f -name "$pat" -print0 2>/dev/null)
done
echo "fix-appimage-libs: removed $removed bundled lib(s)"

if [ "$removed" -eq 0 ]; then
  echo "fix-appimage-libs: nothing bundled — leaving original AppImage untouched"
  popd >/dev/null
  exit 0
fi

# Repackage with appimagetool. Fetch it once; ARCH must match the payload.
ARCH="$(uname -m)"
TOOL="appimagetool-${ARCH}.AppImage"
if [ ! -x "$TOOL" ]; then
  wget -q "https://github.com/AppImage/appimagetool/releases/download/continuous/${TOOL}" -O "$TOOL"
  chmod +x "$TOOL"
fi

export ARCH
# --appimage-extract-and-run avoids needing FUSE on the runner.
"./$TOOL" --appimage-extract-and-run squashfs-root "$ABS_APPIMAGE" >/dev/null
popd >/dev/null

echo "fix-appimage-libs: repackaged $APPIMAGE"
