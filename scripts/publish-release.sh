#!/bin/sh
# Publishes a release build (scripts/build-release.sh) for download from
# ansiapps.com, the only place Floppy is downloaded from (CLAUDE.md rule 8).
# Run it on each platform's build machine after building there:
#
#   scripts/publish-release.sh [--dry-run] [--allow-unsigned]
#
# 1. Checks the packages are this version's (package.json), and writes
#    SHA256SUMS-macOS or SHA256SUMS-Linux beside them.
# 2. Uploads them to the R2 bucket behind downloads.ansiapps.com, under
#    floppy/<version>/, with a year-long immutable Cache-Control. It never
#    replaces a file: if any is already there it stops before uploading
#    anything. A mistake is fixed with a new version.
# 3. Puts the GPL sources and AROS notice (bundle/source/, from
#    build-release.sh) on the v<version> GitHub release, creating it at
#    HEAD if needed and adding only files it lacks. GitHub keeps source
#    only, never the app.
# 4. Prints the site's _redirects lines for this version.
#
# A macOS DMG must be signed and notarized (scripts/sign-macos.sh), or
# Gatekeeper won't open it; --allow-unsigned publishes one anyway.
# --dry-run checks and prints everything without uploading.
#
# Credentials, for an R2 API token scoped to Object Read & Write on this one
# bucket: R2_ACCOUNT_ID, R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY from the
# environment, else from the login keychain on macOS (`security
# add-generic-password -s ansiapps-r2 -a R2_ACCESS_KEY_ID -w`) or the secret
# service on Linux (`secret-tool store --label … service ansiapps-r2
# account R2_ACCESS_KEY_ID`). Never in a repo. R2_BUCKET defaults to
# ansiapps-downloads. Uploads use curl's own AWS signing, so nothing else
# needs installing; the GitHub step needs `gh`, signed in.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
BUNDLE="$ROOT/src-tauri/target/release/bundle"
BUCKET=${R2_BUCKET:-ansiapps-downloads}
PUBLIC=https://downloads.ansiapps.com
REPO=https://github.com/joashchee/floppy

DRY=0 UNSIGNED=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY=1 ;;
    --allow-unsigned) UNSIGNED=1 ;;
    *) echo "Unknown argument: $arg" >&2; exit 2 ;;
  esac
done

fail() { echo "$*" >&2; exit 1; }

VERSION=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' package.json)
CONF_VERSION=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' src-tauri/tauri.conf.json)
[ -n "$VERSION" ] || fail "Couldn't read the version from package.json."
[ "$VERSION" = "$CONF_VERSION" ] ||
  fail "package.json says $VERSION but tauri.conf.json says $CONF_VERSION."
TAG="v$VERSION"

if [ -n "$(git status --porcelain)" ]; then
  [ "$DRY" = 1 ] || fail "The working tree has uncommitted changes. Publish from the release commit."
  echo "Warning: uncommitted changes (a real publish would stop here)." >&2
fi

# 1. The packages, and their checksums.
case "$(uname -s)" in
  Darwin)
    OS=macOS
    PACKAGES=$(ls "$BUNDLE"/dmg/Floppy_"$VERSION"_*.dmg 2>/dev/null || true)
    sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
    ;;
  *)
    OS=Linux
    PACKAGES=$(ls "$BUNDLE"/deb/Floppy_"$VERSION"_*.deb \
      "$BUNDLE"/rpm/Floppy-"$VERSION"-*.rpm \
      "$BUNDLE"/appimage/Floppy_"$VERSION"_*.AppImage 2>/dev/null || true)
    sha256() { sha256sum "$1" | cut -d' ' -f1; }
    ;;
esac
[ -n "$PACKAGES" ] ||
  fail "No $VERSION packages in $BUNDLE. Build with scripts/build-release.sh first."

if [ "$OS" = macOS ] && [ "$UNSIGNED" = 0 ]; then
  for p in $PACKAGES; do
    xcrun stapler validate "$p" >/dev/null 2>&1 &&
      spctl --assess --type open --context context:primary-signature "$p" 2>/dev/null ||
      fail "$(basename "$p") isn't signed and notarized, so Gatekeeper won't open it.
Build with FLOPPY_SIGN_IDENTITY set (scripts/sign-macos.sh), or pass --allow-unsigned."
  done
fi

SUMS="$BUNDLE/SHA256SUMS-$OS"
: >"$SUMS"
for p in $PACKAGES; do
  printf '%s  %s\n' "$(sha256 "$p")" "$(basename "$p")" >>"$SUMS"
done
FILES="$PACKAGES
$SUMS"
echo "Publishing Floppy $VERSION for $OS:"
sed 's/^/  /' "$SUMS"

content_type() {
  case "$1" in
    *.dmg) echo application/x-apple-diskimage ;;
    *.deb) echo application/vnd.debian.binary-package ;;
    *.rpm) echo application/x-rpm ;;
    SHA256SUMS*) echo "text/plain; charset=utf-8" ;;
    *) echo application/octet-stream ;;
  esac
}

# 2. R2.
secret() { # secret NAME: the environment, else the keychain or secret service
  eval "value=\${$1:-}"
  if [ -z "$value" ]; then
    if [ "$OS" = macOS ]; then
      value=$(security find-generic-password -s ansiapps-r2 -a "$1" -w 2>/dev/null || true)
    elif command -v secret-tool >/dev/null; then
      value=$(secret-tool lookup service ansiapps-r2 account "$1" 2>/dev/null || true)
    fi
  fi
  [ -n "$value" ] || fail "No $1: set it in the environment or store it (see the top of $0)."
  printf '%s' "$value"
}

# s3 METHOD KEY [curl args…]: a signed request for bucket object KEY; prints
# the HTTP status. The credentials go to curl on stdin, never in its
# arguments, where other users' `ps` could see them.
s3() {
  method=$1 key=$2
  shift 2
  printf 'user = "%s:%s"\n' "$ACCESS_KEY" "$SECRET_KEY" |
    curl -sS -K - --aws-sigv4 "aws:amz:auto:s3" -X "$method" \
      -o /dev/null -w '%{http_code}' "$@" \
      "https://$ACCOUNT.r2.cloudflarestorage.com/$BUCKET/$key"
}

if [ "$DRY" = 1 ]; then
  echo "Dry run: would upload to r2://$BUCKET/floppy/$VERSION/:"
  for f in $FILES; do echo "  $(basename "$f") ($(content_type "$(basename "$f")"))"; done
else
  ACCOUNT=$(secret R2_ACCOUNT_ID)
  ACCESS_KEY=$(secret R2_ACCESS_KEY_ID)
  SECRET_KEY=$(secret R2_SECRET_ACCESS_KEY)
  # Every file is checked before any is sent, so a clash uploads nothing.
  for f in $FILES; do
    key="floppy/$VERSION/$(basename "$f")"
    status=$(s3 HEAD "$key" --head)
    case "$status" in
      404) ;;
      200) fail "$key is already in the bucket. Published files are never replaced: bump the version." ;;
      *) fail "Checking $key in r2://$BUCKET failed (HTTP $status). Check the R2 credentials." ;;
    esac
  done
  for f in $FILES; do
    name=$(basename "$f")
    key="floppy/$VERSION/$name"
    echo "Uploading $name…"
    status=$(s3 PUT "$key" -T "$f" \
      -H "Content-Type: $(content_type "$name")" \
      -H "Cache-Control: public, max-age=31536000, immutable" \
      -H "x-amz-content-sha256: $(sha256 "$f")")
    [ "$status" = 200 ] || fail "Uploading $key failed (HTTP $status)."
  done
  # Once the custom domain is live, each file must be served from it, with
  # no cookie (the site's no-tracking rule covers this host too).
  for f in $FILES; do
    url="$PUBLIC/floppy/$VERSION/$(basename "$f")"
    headers=$(curl -sSI "$url" 2>/dev/null || true)
    if ! printf '%s' "$headers" | head -1 | grep -q ' 200'; then
      echo "Warning: $url isn't being served yet. Is downloads.ansiapps.com connected to the bucket?" >&2
    elif printf '%s' "$headers" | grep -qi '^set-cookie:'; then
      fail "$url sets a cookie. Turn off whatever adds it in Cloudflare before linking the download."
    fi
  done
fi

# 3. The GPL sources on GitHub.
SOURCES=$(ls "$BUNDLE"/source/* 2>/dev/null || true)
[ -n "$SOURCES" ] || fail "No sources in $BUNDLE/source. build-release.sh gathers them."
if [ "$DRY" = 1 ]; then
  echo "Dry run: would make sure $TAG's GitHub release has:"
  for f in $SOURCES; do echo "  $(basename "$f")"; done
elif ! HAVE=$(gh release view "$TAG" --json assets --jq '.assets[].name' 2>/dev/null); then
  # shellcheck disable=SC2086 # SOURCES is a list of paths without spaces.
  gh release create "$TAG" --target "$(git rev-parse HEAD)" --title "Floppy $VERSION" \
    --notes "Source for the GPL emulators bundled with Floppy $VERSION, and the notice for the AROS ROM inside FS-UAE. Floppy itself is downloaded from ansiapps.com." \
    $SOURCES
  git fetch --tags --quiet
else
  for f in $SOURCES; do
    printf '%s\n' "$HAVE" | grep -qxF "$(basename "$f")" || gh release upload "$TAG" "$f"
  done
  echo "$TAG's GitHub release has every source file."
fi

# 4. The site's stable links.
echo
echo "Point these lines in the ansiapps-site repo's public/_redirects at $VERSION:"
for p in $PACKAGES; do
  name=$(basename "$p")
  case "$name" in
    *.dmg) path=mac ;;
    *.deb) path=deb ;;
    *.rpm) path=rpm ;;
    *.AppImage) path=appimage ;;
  esac
  printf '/download/floppy/%-9s %s/floppy/%s/%s 302\n' "$path" "$PUBLIC" "$VERSION" "$name"
done
printf '/download/floppy/%-9s %s/floppy/%s/SHA256SUMS-%s 302\n' "sums-$(echo "$OS" | tr '[:upper:]' '[:lower:]')" "$PUBLIC" "$VERSION" "$OS"
printf '/download/floppy/%-9s %s/releases/tag/%s 302\n' source "$REPO" "$TAG"
