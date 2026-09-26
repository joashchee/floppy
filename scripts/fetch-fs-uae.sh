#!/bin/sh
# Fetches the pinned FS-UAE release into src-tauri/resources/fs-uae/, where
# tauri.conf.json bundles it into Floppy.app. Run once after cloning, and
# again after bumping VERSION/SHA256 below. The binary is third-party
# (GPL-2.0, https://fs-uae.net) and gitignored. It fetches the build for
# this Mac's architecture.
#
# FS-UAE's own archive also carries the AROS replacement Kickstart
# (fs-uae.dat, AROS Public License), which FS-UAE falls back to without a
# ROM. Floppy never relies on it: it asks the user for their own Kickstart.
set -eu

VERSION=v3.2.35
# The matching source, shipped with each Floppy release (GPL): FS-UAE's own
# source tarball. scripts/fetch-sources.sh reads these two lines: bump them
# together with VERSION.
SRC_URL="https://github.com/FrodeSolheim/fs-uae/releases/download/$VERSION/fs-uae-${VERSION#v}.tar.xz"
SRC_SHA256=f3d3cb8d3df34b0b0125c45a5a3e187ff71050be5dc8455cc4505c0380269117
case "$(uname -m)" in
  arm64)
    ARCH=ARM64
    SHA256=852d96e870678020555ca5c7c622abcd34b8a7bc9429d29ff24caf10f0961fcb
    ;;
  x86_64)
    ARCH=x86-64
    SHA256=00ca0905bb20f499ccc710576bbb83788fa7eb05778a49537f1aac087f9bb8fb
    ;;
  *)
    echo "No FS-UAE macOS build for $(uname -m)." >&2
    exit 1
    ;;
esac
FILE="FS-UAE_${VERSION#v}_macOS_$ARCH.tar.xz"
URL="https://github.com/FrodeSolheim/fs-uae/releases/download/$VERSION/$FILE"

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/fs-uae"
APP="FS-UAE.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION $ARCH" ]; then
  echo "FS-UAE $VERSION ($ARCH) already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-fs-uae.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

echo "Downloading FS-UAE $VERSION ($ARCH)..."
curl -sSfL -o "$WORK/fs-uae.tar.xz" "$URL"
echo "$SHA256  $WORK/fs-uae.tar.xz" | shasum -a 256 -c -

# The vendor's bundle signature doesn't verify as shipped in this tarball
# (`codesign -v` reports modified resources; the binary still runs).
# Release signing re-signs the nested FS-UAE.app anyway, as for DOSBox.
tar -xJf "$WORK/fs-uae.tar.xz" -C "$WORK"
rm -rf "$DEST"
mkdir -p "$DEST"
ditto "$WORK/FS-UAE/macOS/$ARCH/$APP" "$DEST/$APP"

echo "$VERSION $ARCH" > "$DEST/VERSION"
echo "Installed $DEST/$APP"
