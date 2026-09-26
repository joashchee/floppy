#!/bin/sh
# Fetches the pinned Basilisk II build into src-tauri/resources/basilisk/,
# where tauri.conf.json bundles it into Floppy.app. Run once after cloning,
# and again after bumping the pins below. The binary is third-party
# (GPL-2.0-or-later, github.com/kanjitalk755/macemu) and gitignored.
#
# Upstream publishes no binaries, so this is Floppy's own build of a pinned
# macemu commit, made by .github/workflows/basilisk.yml and published as
# the basilisk-ii-$VERSION release of this repo. One universal app covers
# both architectures. Rebuilding never gives the same bytes, so after
# publishing a new build, copy its hashes from the release's SHA256SUMS.
#
# Before the release is up, point BASILISK_TARBALL at a local copy of the
# app tarball (the workflow's artifact). Its hash is still checked.
set -eu

VERSION=2026-08-30-892eeb7
SHA256=795a4ab20e4aa06658eb4d01d9c112c2dd66d5fef95c00063f7cd45b0fc90c26
URL="https://github.com/joashchee/floppy/releases/download/basilisk-ii-$VERSION/BasiliskII-$VERSION-macOS-universal.tar.xz"
# The matching source, shipped with each Floppy release (GPL): the exact
# macemu commit, plus GMP and MPFR (LGPL-3.0-or-later), which are linked
# statically into the arm64 slice. scripts/fetch-sources.sh reads these
# lines: bump them together with VERSION.
SRC_URL="https://github.com/joashchee/floppy/releases/download/basilisk-ii-$VERSION/BasiliskII-$VERSION-source.tar.gz"
SRC_SHA256=1e812a3de44d89418054dd6fd3e9612111a83556a06e5104adb89d15c977321a
GMP_SRC_URL="https://github.com/joashchee/floppy/releases/download/basilisk-ii-$VERSION/gmp-6.3.0.tar.xz"
GMP_SRC_SHA256=a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898
MPFR_SRC_URL="https://github.com/joashchee/floppy/releases/download/basilisk-ii-$VERSION/mpfr-4.2.2.tar.xz"
MPFR_SRC_SHA256=b67ba0383ef7e8a8563734e2e889ef5ec3c3b898a01d00fa0a6869ad81c6ce01

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/basilisk"
APP="BasiliskII.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION" ]; then
  echo "Basilisk II $VERSION already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-basilisk.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

if [ -n "${BASILISK_TARBALL:-}" ]; then
  echo "Using $BASILISK_TARBALL..."
  cp "$BASILISK_TARBALL" "$WORK/basilisk.tar.xz"
else
  echo "Downloading Basilisk II $VERSION..."
  curl -sSfL -o "$WORK/basilisk.tar.xz" "$URL"
fi
echo "$SHA256  $WORK/basilisk.tar.xz" | shasum -a 256 -c -

# Signed ad hoc by the workflow. Release signing re-signs the nested
# BasiliskII.app, as for DOSBox and FS-UAE.
tar -xJf "$WORK/basilisk.tar.xz" -C "$WORK"
rm -rf "$DEST"
mkdir -p "$DEST"
ditto "$WORK/$APP" "$DEST/$APP"

echo "$VERSION" > "$DEST/VERSION"
echo "Installed $DEST/$APP"
