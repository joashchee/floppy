#!/bin/sh
# Fetches the pinned DOSBox Staging release into src-tauri/resources/dosbox/,
# where tauri.conf.json bundles it into Floppy.app. Run once after cloning,
# and again after bumping VERSION/SHA256 below. The binary is third-party
# (GPL-2.0-or-later, https://www.dosbox-staging.org) and gitignored.
set -eu

VERSION=v0.83.0
SHA256=d8a771adfb8010fa6b5f7fb5351abfba659273ad01c89f03675a92bdbdae8167
URL="https://github.com/dosbox-staging/dosbox-staging/releases/download/$VERSION/dosbox-staging-macOS-$VERSION.dmg"
# The matching source, shipped with each Floppy release (GPL). DOSBox
# Staging publishes no source tarball, so this is GitHub's archive of the
# tag. scripts/fetch-sources.sh reads these two lines: bump them together
# with VERSION.
SRC_URL="https://github.com/dosbox-staging/dosbox-staging/archive/refs/tags/$VERSION.tar.gz"
SRC_SHA256=9b36be5a666784adaeffa560bd0950691f851a76bdb97e7ae3c989561e91caf3

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DEST="$ROOT/src-tauri/resources/dosbox"
APP="DOSBox Staging.app"

if [ -f "$DEST/VERSION" ] && [ "$(cat "$DEST/VERSION")" = "$VERSION" ]; then
  echo "DOSBox Staging $VERSION already present."
  exit 0
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/floppy-dosbox.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

echo "Downloading DOSBox Staging $VERSION..."
curl -sSfL -o "$WORK/dosbox.dmg" "$URL"
echo "$SHA256  $WORK/dosbox.dmg" | shasum -a 256 -c -

# hdiutil keeps the vendor's code signature intact. 7-Zip is the fallback
# for environments where mounting is unavailable. It loses the signature, which
# release signing replaces anyway, and writes extended attributes as
# "name:com.apple.*" files, which are removed afterwards.
rm -rf "$DEST"
mkdir -p "$DEST"
if hdiutil attach -nobrowse -readonly -mountpoint "$WORK/mnt" "$WORK/dosbox.dmg" >/dev/null 2>&1; then
  ditto "$WORK/mnt/$APP" "$DEST/$APP"
  hdiutil detach "$WORK/mnt" >/dev/null
elif command -v 7zz >/dev/null 2>&1; then
  7zz x -y -o"$WORK/x" "$WORK/dosbox.dmg" "$APP" >/dev/null
  find "$WORK/x/$APP" -type f -name '*:com.apple.*' -exec rm {} +
  mv "$WORK/x/$APP" "$DEST/$APP"
else
  echo "Need hdiutil or 7zz (brew install sevenzip) to unpack the DMG." >&2
  exit 1
fi

echo "$VERSION" > "$DEST/VERSION"
echo "Installed $DEST/$APP"
