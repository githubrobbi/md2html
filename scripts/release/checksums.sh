#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# checksums.sh DIR
#
# Writes DIR/CHECKSUMS.txt, the SHA-256 of every other file in DIR, and
# DIR/SHA256SUMS, a copy of it under the name tools look for.  Each line
# is `<sha256>  ./<name>`, as `sha256sum ./*` writes it (so `sha256sum -c
# CHECKSUMS.txt` reads it), the lines sorted by name in byte order
# (LC_ALL=C).  Neither file lists itself or the other.
#
# release.yml writes a release's lists with it, and `just
# release-sign-macos` writes them again with it after the macOS files
# are signed (scripts/release/sign_macos.sh): the same files give the
# same bytes, so only the lines of the files that changed differ.
#
# DIR holds regular files only; a folder or a link in it is refused, as
# is a DIR with nothing to list.  Exit 0 when both lists are written, 1
# when DIR is refused, 2 on a usage error.

set -euo pipefail

if (( $# != 1 )); then
  printf 'usage: checksums.sh DIR\n' >&2
  exit 2
fi
dir="$1"

fail() {
  printf 'checksums.sh: %s\n' "$1" >&2
  exit 1
}

[[ -d "$dir" ]] || fail "$dir is not a folder"

# GNU's sha256sum where there is one (Linux, macOS 15 and later), the
# shasum of Perl otherwise: both write `<sha256>  <name>`.
if command -v sha256sum >/dev/null; then
  hasher=(sha256sum)
else
  hasher=(shasum -a 256)
fi

names=()
while IFS= read -r name; do
  case "$name" in
    ./CHECKSUMS.txt | ./SHA256SUMS | ./CHECKSUMS.txt.new) continue ;;
  esac
  if [[ -L "$dir/$name" || ! -f "$dir/$name" ]]; then
    fail "$dir/${name#./} is not a regular file: a release holds files only"
  fi
  names+=("$name")
done < <(cd "$dir" && find . -mindepth 1 -print | LC_ALL=C sort)

(( ${#names[@]} > 0 )) || fail "$dir holds nothing to list"

(cd "$dir" && "${hasher[@]}" "${names[@]}" > CHECKSUMS.txt.new)
mv "$dir/CHECKSUMS.txt.new" "$dir/CHECKSUMS.txt"
cp "$dir/CHECKSUMS.txt" "$dir/SHA256SUMS"
