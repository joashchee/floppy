#!/bin/sh
# Builds a release Floppy.app (and DMG) without the build machine's paths in
# it. rustc bakes source file paths into panic messages, including those of
# every dependency under ~/.cargo/registry, which would publish the builder's
# home folder (and user name). --remap-path-prefix rewrites them. When
# several prefixes match, rustc applies the last one, so the more specific
# project path comes after $HOME.
#
# Arguments are passed on to `tauri build`, e.g. `--bundles app`.
# Afterwards the built app is searched for $HOME, and the script fails if
# it's still there. Always build releases with this script.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME=~ --remap-path-prefix=$ROOT=."

npm run tauri build -- "$@"

# The DMG is compressed, but it's made from this .app, so checking the app
# and the bare binary covers both.
TARGET="$ROOT/src-tauri/target/release"
LEAKS=$(grep -r -l -a -F "$HOME" "$TARGET/floppy" "$TARGET/bundle/macos" 2>/dev/null || true)
if [ -n "$LEAKS" ]; then
  echo "The build still contains $HOME in:" >&2
  echo "$LEAKS" >&2
  exit 1
fi
echo "Checked: no $HOME paths in the release build."
