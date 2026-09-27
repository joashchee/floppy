#!/bin/sh
# Fetches the pinned DOSBox Staging release into src-tauri/resources/dosbox/,
# where tauri.conf.json bundles it into Floppy. Run once after cloning,
# and again after bumping VERSION and the SHA-256s below. The binary is
# third-party (GPL-2.0-or-later, https://www.dosbox-staging.org) and
# gitignored. It fetches the build for the machine it runs on: the macOS
# DMG, or the Linux x86-64 tarball.
set -eu

VERSION=v0.83.0
MAC_SHA256=d8a771adfb8010fa6b5f7fb5351abfba659273ad01c89f03675a92bdbdae8167
LINUX_SHA256=d3a94f7f1c3e68a47ec88d61145506c7904452adb0c9c5928cb8cfe2331d6c5c
# The matching source, shipped with each Floppy release (GPL). DOSBox
# Staging publishes no source tarball, so this is GitHub's archive of the
# tag. scripts/fetch-sources.sh reads these two lines: bump them together
# with VERSION.
SRC_URL="https://github.com/dosbox-staging/dosbox-staging/archive/refs/tags/$VERSION.tar.gz"
SRC_SHA256=9b36be5a666784adaeffa560bd0950691f851a76bdb97e7ae3c989561e91caf3

BASE="https://github.com/dosbox-staging/dosbox-staging/releases/download/$VERSION"
case "$(uname -s)-$(uname -m)" in
  Darwin-*)
    OS=macOS
    URL="$BASE/dosbox-staging-macOS-$VERSION.dmg"
    SHA256=$MAC_SHA256
    ;;
  Linux-x86_64)
    OS=Linux
    URL="$BASE/dosbox-staging-linux-x86_64-$VERSION.tar.xz"
    SHA256=$LINUX_SHA256
    ;;
  *)
    echo "No DOSBox Staging build pinned for $(uname -s) $(uname -m)." >&2
    exit 1
    ;;
esac

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/dosbox"
APP="DOSBox Staging.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION $OS" ]; then
  echo "DOSBox Staging $VERSION ($OS) already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-dosbox.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

echo "Downloading DOSBox Staging $VERSION ($OS)..."
curl -sSfL -o "$WORK/dosbox.pkg" "$URL"
echo "$SHA256  $WORK/dosbox.pkg" | shasum -a 256 -c -

rm -rf "$DEST"
mkdir -p "$DEST"
if [ "$OS" = Linux ]; then
  # One top-level folder: the `dosbox` binary, the libraries it loads from
  # its own lib/, and its resources/. Kept together as shipped.
  tar -xJf "$WORK/dosbox.pkg" -C "$WORK"
  cp -R "$WORK"/dosbox-staging-linux-*/. "$DEST/"
  echo "$VERSION $OS" > "$DEST/VERSION"
  echo "Installed $DEST/dosbox"
  exit 0
fi

# hdiutil keeps the vendor's code signature intact. 7-Zip is the fallback
# for environments where mounting is unavailable. It loses the signature, which
# release signing replaces anyway, and writes extended attributes as
# "name:com.apple.*" files, which are removed afterwards.
if hdiutil attach -nobrowse -readonly -mountpoint "$WORK/mnt" "$WORK/dosbox.pkg" >/dev/null 2>&1; then
  ditto "$WORK/mnt/$APP" "$DEST/$APP"
  hdiutil detach "$WORK/mnt" >/dev/null
elif command -v 7zz >/dev/null 2>&1; then
  7zz x -y -o"$WORK/x" "$WORK/dosbox.pkg" "$APP" >/dev/null
  find "$WORK/x/$APP" -type f -name '*:com.apple.*' -exec rm {} +
  mv "$WORK/x/$APP" "$DEST/$APP"
else
  echo "Need hdiutil or 7zz (brew install sevenzip) to unpack the DMG." >&2
  exit 1
fi

echo "$VERSION $OS" > "$DEST/VERSION"
echo "Installed $DEST/$APP"
