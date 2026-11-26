#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of scripts/release/release_notes.py (a release's notes), run by
# the gate `gate-tests`: the downloads open the page, before what changed;
# each archive links its file in the release, and the installer its own;
# the package managers appear
# with their lanes only; what changed is the summary with every change
# folded; a release without a changelog section, or without its version,
# has no notes. The changelog read is the repository's own (0.8.0's
# section). Exit 1 when a case fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/release/release_notes.py"
failed=0
count=0

# Runs the script with `VERSION=v0.8.0`, the source repository and a
# commit, plus the given NAME=value pairs; its output goes to $out, its
# exit code to $status.
notes() {
  status=0
  out="$(env -u RELEASE_REPO -u CODESIGN -u BREW -u WINGET VERSION=v0.8.0 GITHUB_REPOSITORY=o/md2html \
    COMMIT=abc1234 "$@" python3 "$script" 2>&1)" || status=$?
}

# One case: the exit code, then every `+text` must be in the notes and
# every `-text` must not.
expect() {
  local name="$1" code="$2" want problem=""
  shift 2
  count=$((count + 1))
  [[ "$status" == "$code" ]] || problem="exit $status, not $code"
  for want in "$@"; do
    case "$want" in
      +*) grep -qF -- "${want#+}" <<< "$out" || problem="${problem:+$problem; }missing: ${want#+}" ;;
      -*) if grep -qF -- "${want#-}" <<< "$out"; then problem="${problem:+$problem; }unwanted: ${want#-}"; fi ;;
    esac
  done
  if [[ -n "$problem" ]]; then
    printf '  ✗ %s: %s\n' "$name" "$problem"
    failed=$((failed + 1))
  else
    printf '  ✓ %s\n' "$name"
  fi
}

# The line a text first stands on in the notes (0 when it is not there).
line_of() {
  grep -nF -- "$1" <<< "$out" | head -1 | cut -d: -f1
}

notes
expect "every archive links its file in the release" 0 \
  "+[md2html-windows-x64.zip](https://github.com/o/md2html/releases/download/v0.8.0/md2html-windows-x64.zip)" \
  "+[md2html-macos-arm64.zip](https://github.com/o/md2html/releases/download/v0.8.0/md2html-macos-arm64.zip)" \
  "+[md2html-macos-x64.zip](https://github.com/o/md2html/releases/download/v0.8.0/md2html-macos-x64.zip)" \
  "+[md2html-linux-x64.zip](https://github.com/o/md2html/releases/download/v0.8.0/md2html-linux-x64.zip)" \
  "+\`cargo install --git https://github.com/o/md2html --tag v0.8.0 md2html --locked\`" \
  "+\`curl -fsSL https://github.com/o/md2html/releases/download/v0.8.0/install.sh | MD2HTML_VERSION=v0.8.0 sh\`" \
  "+\`install.sh\` is the installer"
count=$((count + 1))
download="$(line_of '## ⬇️ Download')"
changed="$(line_of "## What's new in v0.8.0")"
verify="$(line_of '## Verify a download')"
if [[ -n "$download" && -n "$changed" && -n "$verify" && "$download" -lt "$changed" && "$changed" -lt "$verify" ]]; then
  printf '  ✓ the downloads open the page, what changed follows, the checks come last\n'
else
  printf '  ✗ the order: download at %s, what is new at %s, verify at %s\n' "$download" "$changed" "$verify"
  failed=$((failed + 1))
fi
expect "every change is folded under the summary" 0 \
  "+<details><summary>Every change in v0.8.0</summary>" "+### Security" \
  "+<details><summary>Checksums, build provenance, SBOM and the build</summary>" \
  "+Built from abc1234"
count=$((count + 1))
if [[ "$(line_of '<details><summary>Every change')" -lt "$(line_of '### Security')" ]]; then
  printf '  ✓ the changelog groups stand inside the fold\n'
else
  printf '  ✗ a changelog group stands outside the fold\n'
  failed=$((failed + 1))
fi
expect "without its lanes, no package manager but the source" 0 \
  "-- **Homebrew** (macOS, Linux)" "-- **winget** (Windows)" "+not notarized" "+ad-hoc signed"

notes BREW=true WINGET=true
expect "Homebrew and winget with their lanes" 0 \
  "+- **Homebrew** (macOS, Linux): \`brew install o/md2html/md2html\`" \
  "+- **winget** (Windows): \`winget install SkyLLC.md2html\`"

notes CODESIGN=true
expect "signed macOS binaries are said to be signed, with no Gatekeeper note" 0 \
  "+notarized by Apple" "-first launch of a downloaded copy asks"

notes VERSION=0.8.0 RELEASE_REPO=o/md2html-releases
expect "a version without its v, and the files in a releases repository" 0 \
  "+https://github.com/o/md2html-releases/releases/download/v0.8.0/md2html-linux-x64.zip" \
  "+https://github.com/o/md2html-releases/releases/download/v0.8.0/install.sh" \
  "+--git https://github.com/o/md2html --tag v0.8.0" "+## What's new in v0.8.0"

notes VERSION=v9.9.9
expect "a version the changelog does not hold has no notes" 1 "+no changelog section for 9.9.9" "-## ⬇️ Download"

notes VERSION=
expect "no version, no notes" 1 "+VERSION is not set"

if (( failed > 0 )); then
  printf '❌ %d of %d release-notes cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ %d release-notes cases passed\n' "$count"
