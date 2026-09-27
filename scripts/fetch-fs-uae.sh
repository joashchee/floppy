#!/bin/sh
# Fetches the pinned FS-UAE release into src-tauri/resources/fs-uae/, where
# tauri.conf.json bundles it into Floppy.app. Run once after cloning, and
# again after bumping VERSION/SHA256 below. The binary is third-party
# (GPL-2.0, https://fs-uae.net) and gitignored. It fetches the build for
# the machine it runs on: macOS (this Mac's architecture) or Linux x86-64.
#
# FS-UAE's own archive also carries the AROS replacement Kickstart
# (fs-uae.dat, AROS Public License). Keep it: Floppy offers it as an
# opt-in fallback until the user has a real Kickstart (kickstart_file =
# internal, amiga.rs).
set -eu

VERSION=v3.2.35
# The matching source, shipped with each Floppy release (GPL): FS-UAE's own
# source tarball. scripts/fetch-sources.sh reads these two lines: bump them
# together with VERSION.
SRC_URL="https://github.com/FrodeSolheim/fs-uae/releases/download/$VERSION/fs-uae-${VERSION#v}.tar.xz"
SRC_SHA256=f3d3cb8d3df34b0b0125c45a5a3e187ff71050be5dc8455cc4505c0380269117
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    OS=macOS ARCH=ARM64
    SHA256=852d96e870678020555ca5c7c622abcd34b8a7bc9429d29ff24caf10f0961fcb
    ;;
  Darwin-x86_64)
    OS=macOS ARCH=x86-64
    SHA256=00ca0905bb20f499ccc710576bbb83788fa7eb05778a49537f1aac087f9bb8fb
    ;;
  Linux-x86_64)
    OS=Linux ARCH=x86-64
    SHA256=dbecec21bba0d66c4b3d1e280c7dba472511c1c04ff07c34757c46d596cd07ca
    ;;
  *)
    echo "No FS-UAE build pinned for $(uname -s) $(uname -m)." >&2
    exit 1
    ;;
esac
FILE="FS-UAE_${VERSION#v}_${OS}_$ARCH.tar.xz"
URL="https://github.com/FrodeSolheim/fs-uae/releases/download/$VERSION/$FILE"

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/fs-uae"
APP="FS-UAE.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION $OS $ARCH" ]; then
  echo "FS-UAE $VERSION ($OS $ARCH) already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-fs-uae.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

echo "Downloading FS-UAE $VERSION ($OS $ARCH)..."
curl -sSfL -o "$WORK/fs-uae.tar.xz" "$URL"
echo "$SHA256  $WORK/fs-uae.tar.xz" | shasum -a 256 -c -

tar -xJf "$WORK/fs-uae.tar.xz" -C "$WORK"
rm -rf "$DEST"
mkdir -p "$DEST"

if [ "$OS" = Linux ]; then
  # Kept as shipped: the binary, its own libraries beside it in
  # Linux/x86-64/, and the shared Locale/ it finds from there
  # (FSUAE_BIN in emulator.rs).
  cp -R "$WORK/FS-UAE/." "$DEST/"
  echo "$VERSION $OS $ARCH" > "$DEST/VERSION"
  echo "Installed $DEST/Linux/$ARCH/fs-uae"
  exit 0
fi

# The vendor's bundle signature doesn't verify as shipped in this tarball
# (`codesign -v` reports modified resources; the binary still runs).
# Release signing re-signs the nested FS-UAE.app anyway, as for DOSBox.
ditto "$WORK/FS-UAE/macOS/$ARCH/$APP" "$DEST/$APP"

echo "$VERSION $OS $ARCH" > "$DEST/VERSION"
echo "Installed $DEST/$APP"
