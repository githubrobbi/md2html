#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# winget_manifests.sh TAG OUTDIR
#
# Writes the three winget manifests of `SkyLLC.md2html` for the published
# release TAG (vX.Y.Z) from the templates in packaging/winget/, into
# OUTDIR/manifests/s/SkyLLC/md2html/X.Y.Z/, the folder they take in
# microsoft/winget-pkgs: the package's FIRST submission, which is made by
# hand (packaging/winget/README.md, `just winget-manifests vX.Y.Z`).
# Every later version is komac's (winget-publish.yml).
#
# The values come from the release itself: the version from TAG, the
# installer's SHA-256 from the release's SHA256SUMS, checked against the
# downloaded md2html-windows-x64.zip (which must hold
# md2html-windows-x64/md2html.exe, the file the manifest names), and the
# release date from its publication.  It refuses a release that is a
# draft or missing, a SHA256SUMS without the archive's line or with
# another digest, an archive without the binary, and a template it would
# leave a placeholder in.  The templates' own licence lines are not
# written to the manifests.  Exit 0 when the manifests are written, 1 on
# a refusal, 2 on a usage error.

set -euo pipefail

if (( $# != 2 )); then
  printf 'usage: winget_manifests.sh vX.Y.Z OUTDIR\n' >&2
  exit 2
fi
tag="$1"
outdir="$2"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
templates="$(cd "$here/../../packaging/winget" && pwd)"
asset="md2html-windows-x64.zip"

refuse() {
  printf '❌ %s\n' "$1" >&2
  exit 1
}

[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]] || refuse "'$tag' is not a release tag vX.Y.Z"
version="${BASH_REMATCH[1]}"

state="$(gh release view "$tag" --repo githubrobbi/md2html --json isDraft,publishedAt --jq '"\(.isDraft) \(.publishedAt)"')" \
  || refuse "githubrobbi/md2html has no release $tag"
read -r is_draft published <<< "$state"
[[ "$is_draft" == "false" ]] || refuse "$tag is still a draft: winget installs from the published release (just release-sign-macos $tag publishes it)"
[[ "$published" =~ ^([0-9]{4}-[0-9]{2}-[0-9]{2})T ]] || refuse "the release's publication time '$published' is no date"
release_date="${BASH_REMATCH[1]}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
gh release download "$tag" --repo githubrobbi/md2html --dir "$work" --pattern "$asset" --pattern SHA256SUMS \
  || refuse "the release's $asset and SHA256SUMS could not be downloaded"

listed="$(awk -v name="./$asset" '$2 == name { print $1 }' "$work/SHA256SUMS")"
[[ "$listed" =~ ^[0-9a-f]{64}$ ]] || refuse "SHA256SUMS of $tag has no line for $asset"
if command -v sha256sum >/dev/null; then
  read -r actual rest < <(sha256sum "$work/$asset")
else
  read -r actual rest < <(shasum -a 256 "$work/$asset")
fi
[[ "$actual" == "$listed" ]] || refuse "$asset of $tag has the SHA-256 $actual, its SHA256SUMS says $listed"
entries="$(unzip -Z1 "$work/$asset")" || refuse "$asset of $tag cannot be read as a ZIP"
grep -qx 'md2html-windows-x64/md2html.exe' <<< "$entries" || refuse "$asset of $tag holds no md2html-windows-x64/md2html.exe"
sha_upper="$(tr 'a-f' 'A-F' <<< "$actual")"

dest="$outdir/manifests/s/SkyLLC/md2html/$version"
mkdir -p "$dest"
for template in "$templates"/SkyLLC.md2html*.yaml.in; do
  name="${template##*/}"
  name="${name%.in}"
  sed -e '/^# SPDX-License-Identifier: /d' -e '/^# Copyright (c) /d' \
      -e "s|@VERSION@|$version|g" -e "s|@TAG@|$tag|g" \
      -e "s|@SHA256@|$sha_upper|g" -e "s|@RELEASE_DATE@|$release_date|g" \
      "$template" > "$dest/$name"
  if grep -n '@[A-Z0-9_]*@' "$dest/$name" >&2; then
    refuse "$name keeps a placeholder (above)"
  fi
done
printf '✅ the manifests of SkyLLC.md2html %s, in %s:\n' "$version" "$dest"
printf '   %s\n' "$dest"/*
