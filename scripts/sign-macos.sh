#!/bin/sh
# Finishes a macOS release build for download: restores the symlinks
# Tauri's resource copy turned into plain files, signs the three nested
# emulator apps and Floppy.app with a Developer ID (hardened runtime,
# secure timestamp), rebuilds the DMG, signs it, and notarizes and staples
# both the app and the DMG, so Gatekeeper opens them offline too.
# build-release.sh runs it after `tauri build`.
#
#   scripts/sign-macos.sh [Floppy.app]
#
# FLOPPY_SIGN_IDENTITY   "Developer ID Application: … (TEAMID)". Unset: only
#                        the symlinks are restored (and the DMG rebuilt
#                        with them). "-" signs ad hoc, for testing the
#                        script without an Apple account (no notarizing).
# FLOPPY_NOTARY_PROFILE  notarytool keychain profile (default
#                        floppy-notary), made once with
#                        `xcrun notarytool store-credentials floppy-notary
#                        --apple-id … --team-id …`. FLOPPY_NOTARIZE=0 skips
#                        notarizing.
#
# The emulators' entitlements are in src-tauri/macos/: upstream's own for
# DOSBox Staging (allow-jit) and FS-UAE (allow-unsigned-executable-memory);
# Basilisk II's build is ad hoc upstream, so it gets FS-UAE's.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP=${1:-"$ROOT/src-tauri/target/release/bundle/macos/Floppy.app"}
IDENTITY=${FLOPPY_SIGN_IDENTITY:-}
PROFILE=${FLOPPY_NOTARY_PROFILE:-floppy-notary}
ENTS="$ROOT/src-tauri/macos"
RES="$APP/Contents/Resources"
# The bundle folder the app is in (bundle/macos/..), which holds dmg/.
BUNDLE=$(cd "$(dirname "$APP")/.." && pwd)

[ -d "$APP" ] || { echo "No app at $APP" >&2; exit 1; }

# 1. Symlinks. Every symlink in src-tauri/resources/ (BasiliskII.app's
# SDL2.framework today) is put back in the app at the same place,
# replacing the file or folder Tauri copied in its stead.
( cd "$ROOT/src-tauri/resources" && find . -type l ) | while IFS= read -r link; do
  dest="$RES/${link#./}"
  rm -rf "$dest"
  mkdir -p "$(dirname "$dest")"
  ln -s "$(readlink "$ROOT/src-tauri/resources/${link#./}")" "$dest"
done
echo "Restored the bundled emulators' symlinks."

# 2. Signing, innermost first.
sign() { # sign <path> [entitlements]
  if [ "$IDENTITY" = "-" ]; then
    # No hardened runtime: its library validation needs every file to
    # carry the same Team ID, which ad hoc signatures don't have.
    set -- "$1" ${2:+--entitlements "$2"}
    codesign --force --sign - "$@"
  else
    set -- "$1" ${2:+--entitlements "$2"}
    codesign --force --sign "$IDENTITY" --options runtime --timestamp "$@"
  fi
}

# Deepest paths first, one per line (the emulators' names have spaces,
# never newlines).
deepest_first() { awk -F/ '{ print NF "\t" $0 }' | sort -rn | cut -f2-; }

sign_nested_app() { # sign_nested_app <app> <entitlements>
  nested=$1 ents=$2
  main="$nested/Contents/MacOS/$(defaults read "$nested/Contents/Info" CFBundleExecutable)"
  # Loose Mach-O files: dylibs, helper tools, and executables other than
  # the main one (FS-UAE's device helper). Frameworks are signed whole.
  find "$nested" -type f ! -path "*.framework/*" | deepest_first |
    while IFS= read -r f; do
      [ "$f" = "$main" ] && continue
      file -b "$f" | grep -q Mach-O || continue
      if [ -x "$f" ] && ! file -b "$f" | grep -q "dynamically linked shared library"; then
        sign "$f" "$ents"
      else
        sign "$f"
      fi
    done
  find "$nested" -type d -name "*.framework" | deepest_first |
    while IFS= read -r fw; do sign "$fw"; done
  sign "$nested" "$ents"
}

if [ -n "$IDENTITY" ]; then
  sign_nested_app "$RES/dosbox/DOSBox Staging.app" "$ENTS/dosbox.entitlements"
  sign_nested_app "$RES/fs-uae/FS-UAE.app" "$ENTS/fs-uae.entitlements"
  sign_nested_app "$RES/basilisk/BasiliskII.app" "$ENTS/basilisk.entitlements"
  sign "$APP"
  codesign --verify --deep --strict "$APP"
  echo "Signed $APP ($IDENTITY) and checked it with codesign --verify --deep --strict."
fi

notarize() { # notarize <file to submit> <file to staple>
  xcrun notarytool submit "$1" --keychain-profile "$PROFILE" --wait
  xcrun stapler staple "$2"
}
NOTARIZE=0
if [ -n "$IDENTITY" ] && [ "$IDENTITY" != "-" ] && [ "${FLOPPY_NOTARIZE:-1}" != 0 ]; then
  NOTARIZE=1
fi

if [ "$NOTARIZE" = 1 ]; then
  ZIP=$(mktemp -d "${TMPDIR:-/tmp}/floppy-notarize.XXXXXX")/Floppy.zip
  ditto -c -k --keepParent "$APP" "$ZIP"
  notarize "$ZIP" "$APP"
  rm -f "$ZIP"
fi

# 3. The DMG, rebuilt from the finished app (only when Tauri made one in
# this build: a `--bundles app` build leaves an earlier build's dmg/ folder
# or DMG behind, older than the app).
DMG_DIR="$BUNDLE/dmg"
VERSION=$(defaults read "$APP/Contents/Info" CFBundleShortVersionString)
case "$(uname -m)" in arm64) ARCH=aarch64 ;; *) ARCH=x64 ;; esac
DMG="$DMG_DIR/Floppy_${VERSION}_${ARCH}.dmg"
[ -f "$DMG" ] && [ "$DMG" -nt "$APP/Contents/Info.plist" ] || exit 0
STAGE=$(mktemp -d "${TMPDIR:-/tmp}/floppy-dmg.XXXXXX")
ditto "$APP" "$STAGE/Floppy.app"
ln -s /Applications "$STAGE/Applications"
# Made beside the staging folder and moved into place once it's whole, so
# a failure leaves Tauri's DMG where it was.
hdiutil create -volname Floppy -srcfolder "$STAGE" -fs HFS+ -format UDZO "$STAGE.dmg" >/dev/null
mv -f "$STAGE.dmg" "$DMG"
rm -rf "$STAGE"
if [ -n "$IDENTITY" ]; then
  if [ "$IDENTITY" = "-" ]; then
    codesign --force --sign - "$DMG"
  else
    codesign --force --sign "$IDENTITY" --timestamp "$DMG"
  fi
fi
if [ "$NOTARIZE" = 1 ]; then
  notarize "$DMG" "$DMG"
  spctl --assess --type open --context context:primary-signature -v "$DMG"
fi
echo "Wrote $DMG"
