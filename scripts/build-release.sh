#!/bin/sh
# Builds a release Floppy.app (and DMG) on macOS, or .deb, .rpm and
# AppImage packages on Linux (CI builds the .deb and .rpm only:
# .github/workflows/linux.yml), without the build machine's paths in
# it. Windows has its own, scripts/build-windows.ps1. rustc bakes source
# file paths into panic messages, including those of
# every dependency under ~/.cargo/registry, which would publish the builder's
# home folder (and user name). --remap-path-prefix rewrites them. When
# several prefixes match, rustc applies the last one, so the more specific
# project path comes after $HOME.
#
# Arguments are passed on to `tauri build`, e.g. `--bundles app`.
# Afterwards the built app is searched for $HOME, and the script fails if
# it's still there. On macOS, scripts/sign-macos.sh then restores the
# emulators' symlinks and, when FLOPPY_SIGN_IDENTITY is set, signs and
# notarizes the app and DMG. It then downloads the source of the bundled GPL
# emulators, plus a notice for the AROS ROM inside FS-UAE
# (scripts/fetch-sources.sh), which every release must offer alongside the
# builds every release carries (CLAUDE.md rule 8). Finally, on macOS, the
# app is installed into ~/Applications, replacing any older copy. Always
# build releases with this script.
#
# On macOS the app is universal (Apple Silicon and Intel), signed with the
# maintainer's Developer ID and the hardened runtime, notarized and
# stapled, then zipped for the release
# (bundle/macos/Floppy_<version>_universal.app.zip, which
# scripts/publish-release.sh uploads). That needs, once:
#   1. A "Developer ID Application" certificate in the login keychain.
#      Found by itself; with more than one, name it in
#      FLOPPY_SIGN_IDENTITY.
#   2. notarytool's credentials in the keychain, under a profile:
#        xcrun notarytool store-credentials floppy-notary \
#          --apple-id you@example.com --team-id TEAMID
#      (it asks for an app-specific password from account.apple.com).
#      Another profile, e.g. one another ansiapps app already made for
#      the same Apple account: FLOPPY_NOTARY_PROFILE.
# FLOPPY_UNSIGNED=1 builds without them (ad hoc, not notarized, no zip):
# for trying the build only, never for a release, as Gatekeeper blocks it
# on any other Mac. No Apple credential is ever in the repo or the
# environment.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

TARGET="$ROOT/src-tauri/target/release"
if [ "$(uname -s)" = Darwin ]; then
  TARGET="$ROOT/src-tauri/target/universal-apple-darwin/release"
  PROFILE=${FLOPPY_NOTARY_PROFILE:-floppy-notary}
  # The signing identity and notarytool profile, checked before the long
  # build.
  if [ -n "${FLOPPY_UNSIGNED:-}" ]; then
    FLOPPY_SIGN_IDENTITY=-
    echo "FLOPPY_UNSIGNED: signed ad hoc and not notarized; not for release."
  elif [ -z "${FLOPPY_SIGN_IDENTITY:-}" ]; then
    FLOPPY_SIGN_IDENTITY=$(security find-identity -v -p codesigning |
      sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -n 1)
    [ -n "$FLOPPY_SIGN_IDENTITY" ] || {
      echo "No Developer ID Application certificate in the keychain. See this script's header, or set FLOPPY_UNSIGNED=1 to try the build unsigned." >&2
      exit 1
    }
  fi
  if [ "$FLOPPY_SIGN_IDENTITY" != - ] && [ "${FLOPPY_NOTARIZE:-1}" != 0 ] &&
    ! xcrun notarytool history --keychain-profile "$PROFILE" >/dev/null 2>&1; then
    echo "No notarytool credentials under the profile \"$PROFILE\". See this script's header." >&2
    exit 1
  fi
  FLOPPY_NOTARY_PROFILE=$PROFILE
  export FLOPPY_SIGN_IDENTITY FLOPPY_NOTARY_PROFILE
  echo "Signing as: $FLOPPY_SIGN_IDENTITY"
  # sign-macos.sh signs and notarizes; these would make Tauri do it too.
  unset APPLE_SIGNING_IDENTITY APPLE_CERTIFICATE APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID \
    APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH 2>/dev/null || true
  # Universal: both slices of Floppy, around emulators that are universal
  # themselves (fetch-fs-uae.sh merges FS-UAE's two builds).
  set -- --target universal-apple-darwin "$@"
fi

export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=~ --remap-path-prefix=$ROOT=."
if [ -n "${CARGO_HOME:-}" ]; then
  RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$CARGO_HOME=~/.cargo"
fi

# Every third-party license text the app ships, into its resources
# (src-tauri/resources/licenses/), fresh for this platform's dependencies.
python3 "$ROOT/scripts/third-party-licenses.py"

npm run tauri build -- "$@"

# The DMG and Linux packages are compressed, but they're made from the
# bundle folders checked here (the .app; the .deb's, .rpm's and AppImage's
# unpacked trees), so checking those and the bare binary covers them all.
case "$(uname -s)" in
  Darwin) BUNDLES="$TARGET/bundle/macos" ;;
  *) BUNDLES="$TARGET/bundle/deb $TARGET/bundle/rpm $TARGET/bundle/appimage" ;;
esac
# The bundled emulators are skipped: they're third-party builds made on
# other machines, never this one, and DOSBox Staging's and Basilisk II's
# Linux builds do carry their own CI's /home/runner, which is this
# machine's $HOME on a GitHub runner.
# shellcheck disable=SC2086 # BUNDLES is a list of folders.
LEAKS=$(grep -r -l -a -F --exclude-dir=dosbox --exclude-dir=fs-uae --exclude-dir=basilisk \
  "$HOME" "$TARGET/floppy" $BUNDLES 2>/dev/null || true)
if [ -n "$LEAKS" ]; then
  echo "The build still contains $HOME in:" >&2
  echo "$LEAKS" >&2
  exit 1
fi
echo "Checked: no $HOME paths in the release build."

# macOS: restore the emulators' symlinks, sign, notarize and staple the
# app and a rebuilt DMG, check Gatekeeper accepts it, and zip it for the
# release.
if [ "$(uname -s)" = Darwin ]; then
  APP="$TARGET/bundle/macos/Floppy.app"
  "$ROOT/scripts/sign-macos.sh" "$APP"
  ARCHS=$(lipo -archs "$APP/Contents/MacOS/floppy" 2>/dev/null)
  case "$ARCHS" in
    *arm64*x86_64* | *x86_64*arm64*) echo "Checked: universal ($ARCHS)." ;;
    *) echo "Not universal: $ARCHS" >&2; exit 1 ;;
  esac
  if [ "$FLOPPY_SIGN_IDENTITY" != - ] && [ "${FLOPPY_NOTARIZE:-1}" != 0 ]; then
    spctl --assess --type execute -vv "$APP" 2>&1 | grep -q "source=Notarized Developer ID" ||
      { echo "Gatekeeper doesn't accept the app." >&2; exit 1; }
    echo "Checked: Gatekeeper accepts it (notarized Developer ID)."
    # The download (scripts/publish-release.sh): ditto keeps the
    # signature, the stapled ticket, symlinks and extended attributes.
    VERSION=$(defaults read "$APP/Contents/Info" CFBundleShortVersionString)
    ZIP="$TARGET/bundle/macos/Floppy_${VERSION}_universal.app.zip"
    rm -f "$ZIP"
    ditto -c -k --keepParent "$APP" "$ZIP"
    echo "Wrote $ZIP"
  fi
fi

# Shipping DOSBox Staging and FS-UAE binaries means shipping their source.
"$ROOT/scripts/fetch-sources.sh" "$TARGET/bundle/source"

# Install the new build for everyday use (macOS; on Linux, install the
# package the usual way).
[ "$(uname -s)" = Darwin ] || exit 0
APPS="$HOME/Applications"
mkdir -p "$APPS"
rm -rf "$APPS/Floppy.app"
ditto "$TARGET/bundle/macos/Floppy.app" "$APPS/Floppy.app"
echo "Installed $APPS/Floppy.app"
