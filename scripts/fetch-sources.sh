#!/bin/sh
# Downloads the source of every GPL emulator Floppy.app bundles, at the
# exact versions the fetch scripts pin, into a folder to attach to the
# GitHub release next to the DMG. Shipping GPL binaries means offering
# their source too. build-release.sh runs this. Run it alone as
#   scripts/fetch-sources.sh [output folder]
# The URLs and SHA-256s live beside each binary's pin (SRC_URL and
# SRC_SHA256 in fetch-dosbox.sh, fetch-fs-uae.sh and fetch-basilisk.sh,
# which also pins GMP_ and MPFR_ ones for the libraries linked into
# Basilisk II), so a version bump can't leave the source behind. A hash
# mismatch fails the script.
#
# It also writes AROS-SOURCE.txt: FS-UAE's fs-uae.dat carries the AROS
# replacement Kickstart (AROS Public License), which Floppy offers as a
# fallback, but FS-UAE ships neither its licence text nor its source. The
# notice names the exact build, read from the fs-uae.dat being shipped,
# and where its source is. A missing fs-uae.dat, or one with no AROS
# version in it, fails the script.
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

# fetch <fetch script> <file name> [variable prefix, e.g. GMP_]
fetch() {
  script="$ROOT/scripts/$1"
  name=$2
  url=$(pin "$script" "${3:-}SRC_URL")
  sha=$(pin "$script" "${3:-}SRC_SHA256")
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
BASILISK_VERSION=$(sed -n 's/^VERSION=//p' "$ROOT/scripts/fetch-basilisk.sh")
fetch fetch-basilisk.sh "BasiliskII-$BASILISK_VERSION-source.tar.gz"
fetch fetch-basilisk.sh "$(basename "$(pin "$ROOT/scripts/fetch-basilisk.sh" GMP_SRC_URL)")" GMP_
fetch fetch-basilisk.sh "$(basename "$(pin "$ROOT/scripts/fetch-basilisk.sh" MPFR_SRC_URL)")" MPFR_

# aros_notice: AROS-SOURCE.txt, for the AROS ROM inside the bundled FS-UAE.
aros_notice() {
  dat=$(find "$ROOT/src-tauri/resources/fs-uae" -name fs-uae.dat 2>/dev/null | head -1)
  if [ -z "$dat" ]; then
    echo "No fs-uae.dat under src-tauri/resources/fs-uae: run scripts/fetch-fs-uae.sh first." >&2
    exit 1
  fi
  ver=$(unzip -p "$dat" share/fs-uae/aros-amiga-m68k-rom.bin | LC_ALL=C grep -a -o '[$]VER: exec.library[^)]*)' | head -1)
  if [ -z "$ver" ]; then
    echo "$dat has no AROS ROM with a version in it: check what FS-UAE ships now." >&2
    exit 1
  fi
  fsuae=$(sed -n 's/^VERSION=//p' "$ROOT/scripts/fetch-fs-uae.sh")
  cat > "$OUT/AROS-SOURCE.txt" <<EOF
AROS replacement Kickstart in Floppy's bundled FS-UAE

FS-UAE $fsuae, bundled with Floppy unmodified, carries the AROS m68k
replacement Kickstart in its fs-uae.dat:

  share/fs-uae/aros-amiga-m68k-rom.bin  (main ROM)
  share/fs-uae/aros-amiga-m68k-ext.bin  (extended ROM)
  Build: ${ver#\$VER: }

AROS is free software under the AROS Public License (APL), version
1.1: https://aros.sourceforge.io/license.html

Its source is the AROS project's repository. The build above dates from
the day in its version string, so take the source as of that date:

  https://github.com/aros-development-team/AROS
  https://aros.sourceforge.io/download.php

Floppy doesn't modify, link or add AROS. It uses FS-UAE's built-in copy
only when you choose "Use AROS for Now" in the Amiga setup, until you
add a Kickstart ROM of your own.
EOF
  echo "Wrote AROS-SOURCE.txt (${ver#\$VER: })"
}
aros_notice

TAG=v$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$ROOT/package.json" | head -1)
echo "GPL sources and the AROS notice are in $OUT. Attach them to the release with the DMG or Linux packages:"
echo "  gh release upload $TAG \"$OUT\"/*"
