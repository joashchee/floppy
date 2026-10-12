#!/bin/sh
# Fetches the pinned FS-UAE release into src-tauri/resources/fs-uae/, where
# tauri.conf.json bundles it into Floppy.app. Run once after cloning, and
# again after bumping VERSION/SHA256 below. The binary is third-party
# (GPL-2.0, https://fs-uae.net) and gitignored. It fetches the build for
# the machine it runs on: on macOS both the Apple Silicon and Intel builds,
# merged with lipo into one universal FS-UAE.app (so Floppy.app can be
# universal too); on Linux the x86-64 build.
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
MAC_ARM64_SHA256=852d96e870678020555ca5c7c622abcd34b8a7bc9429d29ff24caf10f0961fcb
MAC_X86_64_SHA256=00ca0905bb20f499ccc710576bbb83788fa7eb05778a49537f1aac087f9bb8fb
LINUX_X86_64_SHA256=dbecec21bba0d66c4b3d1e280c7dba472511c1c04ff07c34757c46d596cd07ca
case "$(uname -s)-$(uname -m)" in
  Darwin-*) OS=macOS ARCH=universal ;;
  Linux-x86_64) OS=Linux ARCH=x86-64 ;;
  *)
    echo "No FS-UAE build pinned for $(uname -s) $(uname -m)." >&2
    exit 1
    ;;
esac
BASE="https://github.com/FrodeSolheim/fs-uae/releases/download/$VERSION"

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/fs-uae"
APP="FS-UAE.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION $OS $ARCH" ]; then
  echo "FS-UAE $VERSION ($OS $ARCH) already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-fs-uae.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

# get <arch> <sha256>: downloads and unpacks one build into $WORK/<arch>/.
get() {
  file="FS-UAE_${VERSION#v}_${OS}_$1.tar.xz"
  echo "Downloading FS-UAE $VERSION ($OS $1)..."
  curl -sSfL -o "$WORK/$file" "$BASE/$file"
  echo "$2  $WORK/$file" | shasum -a 256 -c -
  mkdir -p "$WORK/$1"
  tar -xJf "$WORK/$file" -C "$WORK/$1"
}

if [ "$OS" = Linux ]; then
  get x86-64 "$LINUX_X86_64_SHA256"
  rm -rf "$DEST"
  mkdir -p "$DEST"
  # Kept as shipped: the binary, its own libraries beside it in
  # Linux/x86-64/, and the shared Locale/ it finds from there
  # (FSUAE_BIN in emulator.rs).
  cp -R "$WORK/x86-64/FS-UAE/." "$DEST/"
  echo "$VERSION $OS $ARCH" > "$DEST/VERSION"
  echo "Installed $DEST/Linux/x86-64/fs-uae"
  exit 0
fi

get ARM64 "$MAC_ARM64_SHA256"
get x86-64 "$MAC_X86_64_SHA256"
ARM="$WORK/ARM64/FS-UAE/macOS/ARM64/$APP"
INTEL="$WORK/x86-64/FS-UAE/macOS/x86-64/$APP"

# The two apps hold the same files. Every Mach-O file (the program, its
# device helper, its dylibs) becomes one with both slices; everything
# else is taken from the Apple Silicon app. fs-uae.dat differs only in
# its zip's timestamps (same entries, same CRCs, checked for v3.2.35).
# Any other difference means the builds no longer pair up, and stops.
( cd "$ARM" && find . -type f ) | while IFS= read -r f; do
  [ -f "$INTEL/$f" ] || { echo "FS-UAE's Intel build has no $f." >&2; exit 1; }
  if file -b "$ARM/$f" | grep -q Mach-O; then
    lipo -create "$ARM/$f" "$INTEL/$f" -output "$ARM/$f.universal" 2>/dev/null ||
      lipo -create "$ARM/$f" "$INTEL/$f" -output "$ARM/$f.universal"
    mv -f "$ARM/$f.universal" "$ARM/$f"
  else
    case "$f" in
      */_CodeSignature/* | */CodeResources | */fs-uae.dat) ;;
      *) cmp -s "$ARM/$f" "$INTEL/$f" || { echo "FS-UAE's builds differ in $f." >&2; exit 1; } ;;
    esac
  fi
done
# Merging broke the vendor's signature (which didn't verify as shipped
# anyway). Sign it ad hoc so it runs in development; release signing
# (scripts/sign-macos.sh) re-signs it with Floppy's Developer ID.
codesign --force --deep --sign - "$ARM" >/dev/null 2>&1 ||
  echo "Couldn't sign the merged FS-UAE.app ad hoc; release signing will." >&2

rm -rf "$DEST"
mkdir -p "$DEST"
ditto "$ARM" "$DEST/$APP"
echo "$VERSION $OS $ARCH" > "$DEST/VERSION"
echo "Installed $DEST/$APP ($(lipo -archs "$DEST/$APP/Contents/MacOS/fs-uae" 2>/dev/null))"
