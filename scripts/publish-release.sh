#!/usr/bin/env bash
# Publishes Floppy's public beta builds to a release of its GitHub
# repository, which ansiapps.com's download buttons link to (CLAUDE.md
# rule 8; the site picks the newest file per platform at each deploy).
#
#   scripts/publish-release.sh [--sources DIR] [files...]
#
# CI runs it from .github/workflows/windows.yml and linux.yml (their
# `publish` job, with GH_TOKEN, GITHUB_REPOSITORY and GITHUB_SHA set),
# with the installers to upload and, from Linux, the GPL sources.
# On the maintainer's Mac, after scripts/build-release.sh, run it with no
# files: it uploads the notarized Floppy_<version>_universal.app.zip and
# the sources that build gathered. That needs `gh` signed in, a clean tree
# and the commit pushed.
#
# Which release:
# - A `v*` tag, or a commit that is (or will become) the tag of the
#   version in tauri.conf.json: that version's release, "Floppy 0.6.0",
#   made if it's missing (tagging this commit), its notes the version's
#   CHANGELOG.md section.
# - Any later commit, while the version's tag already marks an earlier
#   one: the `dev` prerelease, moved to this commit. A build in between
#   versions never replaces a version's files, and the site never offers
#   it (it skips prereleases).
#
# Every platform publishes to the same release, so any may make it first:
# a failed make is tolerated, and the upload that follows says whether the
# release is really there. Installers of the same name are replaced
# (--clobber). Sources (--sources: the emulators' GPL source and
# AROS-SOURCE.txt, from scripts/fetch-sources.sh) are only added when the
# release lacks them: they're the same for every platform.
set -euo pipefail
cd "$(dirname "$0")/.."

SOURCES=
files=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sources) SOURCES=$2; shift 2 ;;
    -*) echo "Unknown option: $1" >&2; exit 2 ;;
    *) files+=("$1"); shift ;;
  esac
done

version=$(node -p "require('./src-tauri/tauri.conf.json').version")
[[ "$version" == "$(node -p "require('./package.json').version")" ]] ||
  { echo "package.json and tauri.conf.json disagree on the version." >&2; exit 1; }

if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
  repo="$GITHUB_REPOSITORY"
  sha="$GITHUB_SHA"
else
  # The maintainer's Mac: the notarized app zip and its sources.
  repo=joashchee/floppy
  [[ -z "$(git status --porcelain)" ]] ||
    { echo "The working tree has uncommitted changes. Publish from the release commit." >&2; exit 1; }
  sha=$(git rev-parse HEAD)
  git fetch -q origin
  git merge-base --is-ancestor "$sha" origin/main ||
    { echo "Push this commit to main first: the release is made at it." >&2; exit 1; }
  if [[ ${#files[@]} -eq 0 ]]; then
    bundle=src-tauri/target/universal-apple-darwin/release/bundle
    zip="$bundle/macos/Floppy_${version}_universal.app.zip"
    [[ -f "$zip" ]] || { echo "No $zip. Build it with scripts/build-release.sh --bundles app." >&2; exit 1; }
    app="$bundle/macos/Floppy.app"
    spctl --assess --type execute "$app" 2>/dev/null ||
      { echo "Gatekeeper doesn't accept $app: it must be signed and notarized." >&2; exit 1; }
    files=("$zip")
    [[ -n "$SOURCES" ]] || SOURCES="$bundle/source"
  fi
fi
[[ ${#files[@]} -gt 0 || -n "$SOURCES" ]] || { echo "Nothing to publish." >&2; exit 1; }

# The commit a tag marks, or nothing when there's no such tag
# (an annotated tag's ^{} line names its commit).
tagged() {
  git ls-remote --tags origin "refs/tags/$1" "refs/tags/$1^{}" | sort -k2 | tail -n 1 | cut -f1
}

# A version's section of CHANGELOG.md, without its heading.
notes() {
  awk -v v="$1" '
    index($0, "## " v " ") == 1 || $0 == "## " v { on = 1; next }
    on && /^## / { exit }
    on { print }
  ' CHANGELOG.md
}

BETA="**Public beta.** Floppy is still in development: use it at your own risk, and keep copies of anything you bring to it.

- **macOS** (Apple Silicon and Intel): the app, signed and notarized. Unzip it and move Floppy.app to Applications.
- **Windows** (x64): the installer or MSI. DOS and Amiga apps run; classic Mac apps don't yet (Basilisk II isn't bundled for Windows). Unsigned, so SmartScreen warns: More info, then Run anyway.
- **Linux** (x86-64): the .deb or .rpm, on Ubuntu 22.04 and newer, or a matching Fedora.

Floppy bundles DOSBox Staging, FS-UAE and Basilisk II (GPL). Their exact source, and a notice for the AROS ROM inside FS-UAE, are attached here. Floppy never includes ROMs, operating systems or other guest software: you bring your own."

if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
  tag="${GITHUB_REF#refs/tags/}"
  version="${tag#v}"
else
  tag="v$version"
  at=$(tagged "$tag")
  [[ -n "$at" && "$at" != "$sha" ]] && tag=dev
fi

if [[ "$tag" == dev ]]; then
  # Move (or make) the dev tag to this commit, then its release.
  gh api -X PATCH "repos/$repo/git/refs/tags/dev" -f sha="$sha" -F force=true >/dev/null 2>&1 ||
    gh api -X POST "repos/$repo/git/refs" -f ref=refs/tags/dev -f sha="$sha" >/dev/null 2>&1 || true
  body="A build between versions, from ${sha:0:7}: everything since Floppy $version (CHANGELOG.md's Unreleased). Untested, and not offered on ansiapps.com."
  gh release create dev --repo "$repo" --prerelease --title "Floppy development build" --notes "$body" >/dev/null 2>&1 ||
    gh release edit dev --repo "$repo" --prerelease --notes "$body" >/dev/null
else
  body=$(notes "$version")
  [[ -n "$body" ]] || body="Floppy $version."
  body="$BETA"$'\n\n'"$body"
  gh release create "$tag" --repo "$repo" --target "$sha" --title "Floppy $version (public beta)" --notes "$body" >/dev/null 2>&1 || true
fi

if [[ ${#files[@]} -gt 0 ]]; then
  gh release upload "$tag" --repo "$repo" --clobber "${files[@]}"
  echo "Published to $tag: ${files[*]}"
fi

if [[ -n "$SOURCES" ]]; then
  [[ -d "$SOURCES" ]] || { echo "No sources folder at $SOURCES." >&2; exit 1; }
  have=$(gh release view "$tag" --repo "$repo" --json assets --jq '.assets[].name')
  missing=()
  for f in "$SOURCES"/*; do
    [[ -f "$f" ]] || continue
    grep -qxF "$(basename "$f")" <<<"$have" || missing+=("$f")
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    gh release upload "$tag" --repo "$repo" "${missing[@]}"
    echo "Added the sources to $tag: ${missing[*]}"
  else
    echo "$tag already has every source file."
  fi
fi
