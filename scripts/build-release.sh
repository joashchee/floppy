#!/bin/sh
# Builds a release Floppy.app and DMG on macOS, or .deb, .rpm and AppImage
# packages on Linux, without the build machine's paths in
# it. rustc bakes source file paths into panic messages, including those of
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
# DMG or Linux packages it publishes on ansiapps.com. Finally, on macOS, the app is installed into
# ~/Applications, replacing any older copy. Always build releases with this
# script.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=~ --remap-path-prefix=$ROOT=."

npm run tauri build -- "$@"

# The DMG and Linux packages are compressed, but they're made from the
# bundle folders checked here (the .app; the .deb's and AppImage's unpacked
# trees), so checking those and the bare binary covers them all.
TARGET="$ROOT/src-tauri/target/release"
case "$(uname -s)" in
  Darwin) BUNDLES="$TARGET/bundle/macos" ;;
  *) BUNDLES="$TARGET/bundle/deb $TARGET/bundle/appimage" ;;
esac
# shellcheck disable=SC2086 # BUNDLES is a list of folders.
LEAKS=$(grep -r -l -a -F "$HOME" "$TARGET/floppy" $BUNDLES 2>/dev/null || true)
if [ -n "$LEAKS" ]; then
  echo "The build still contains $HOME in:" >&2
  echo "$LEAKS" >&2
  exit 1
fi
echo "Checked: no $HOME paths in the release build."

# macOS: restore the emulators' symlinks, and with FLOPPY_SIGN_IDENTITY
# set, sign, notarize and staple the app and a rebuilt DMG.
if [ "$(uname -s)" = Darwin ]; then
  "$ROOT/scripts/sign-macos.sh" "$TARGET/bundle/macos/Floppy.app"
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
