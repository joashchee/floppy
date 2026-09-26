#!/bin/sh
# Downloads the source of every GPL emulator Floppy.app bundles, at the
# exact versions the fetch scripts pin, into a folder to attach to the
# GitHub release next to the DMG. Shipping GPL binaries means offering
# their source too. build-release.sh runs this. Run it alone as
#   scripts/fetch-sources.sh [output folder]
# The URLs and SHA-256s live beside each binary's pin (SRC_URL and
# SRC_SHA256 in fetch-dosbox.sh and fetch-fs-uae.sh), so a version bump
# can't leave the source behind. A hash mismatch fails the script.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/src-tauri/target/release/bundle/source"}
mkdir -p "$OUT"

# pin <fetch script> <variable>: the variable's value, with $VERSION and
# ${VERSION#v} expanded (the only expansions the SRC_ lines use).
pin() {
  version=$(sed -n 's/^VERSION=//p' "$1")
  raw=$(sed -n "s/^$2=//p" "$1" | tr -d '"')
  bare=${version#v}
  printf '%s\n' "$raw" | sed -e "s|\${VERSION#v}|$bare|g" -e "s|\$VERSION|$version|g"
}

fetch() {
  script="$ROOT/scripts/$1"
  name=$2
  url=$(pin "$script" SRC_URL)
  sha=$(pin "$script" SRC_SHA256)
  dest="$OUT/$name"
  if [ -f "$dest" ] && echo "$sha  $dest" | shasum -a 256 -c - >/dev/null 2>&1; then
    echo "Have $name"
    return
  fi
  echo "Downloading $name..."
  curl -sSfL -o "$dest.part" "$url"
  echo "$sha  $dest.part" | shasum -a 256 -c - >/dev/null || {
    echo "$name doesn't match its pinned SHA-256 ($url)." >&2
    rm -f "$dest.part"
    exit 1
  }
  mv "$dest.part" "$dest"
}

DOSBOX_VERSION=$(sed -n 's/^VERSION=v//p' "$ROOT/scripts/fetch-dosbox.sh")
FSUAE_VERSION=$(sed -n 's/^VERSION=v//p' "$ROOT/scripts/fetch-fs-uae.sh")
fetch fetch-dosbox.sh "dosbox-staging-$DOSBOX_VERSION-source.tar.gz"
fetch fetch-fs-uae.sh "fs-uae-$FSUAE_VERSION-source.tar.xz"

TAG=v$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/package.json" | head -1)
echo "GPL sources are in $OUT. Attach them to the release with the DMG:"
echo "  gh release upload $TAG \"$OUT\"/*"
