#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# pack_zip.sh BUNDLE OUT EPOCH
#
# Packs the folder BUNDLE into the ZIP archive OUT, with one root entry
# named as BUNDLE is (`md2html-macos-arm64/`) and its files below it, so
# that the same files always give the same bytes.  release.yml packs the
# macOS and Linux archives with it, and `just release-sign-macos` packs
# the macOS archives again with it after signing their binary
# (scripts/release/sign_macos.sh): the rebuilt archive differs from the
# one release.yml made in the binary alone.
#
# What makes the bytes repeatable:
#   - the entries in one order: the root folder, then its files sorted by
#     their bytes (LC_ALL=C), never the order a file system lists them in;
#   - one time for every entry, EPOCH (seconds since 1970, UTC: the
#     released commit's time), written in UTC whatever the machine's zone;
#   - one mode per kind: 0755 for the folder and an executable file, 0644
#     for every other file, whatever the umask or the checkout left;
#   - `zip -X`: no extra fields (no owner, no second time stamp);
#   - ZIP and ZIPOPT, which Info-ZIP reads options from, are cleared.
#
# BUNDLE holds regular files only (release.yml's bundles do); a folder or
# a link inside it is refused, as is an empty one.  OUT is replaced.
# Exit 0 when OUT is written, 1 when the input is refused, 2 on a usage
# error.

set -euo pipefail

if (( $# != 3 )); then
  printf 'usage: pack_zip.sh BUNDLE OUT EPOCH\n' >&2
  exit 2
fi
bundle="$1"
out="$2"
epoch="$3"

fail() {
  printf 'pack_zip.sh: %s\n' "$1" >&2
  exit 1
}

[[ -d "$bundle" ]] || fail "$bundle is not a folder"
[[ "$epoch" =~ ^[0-9]+$ ]] || fail "EPOCH must be whole seconds since 1970, not '$epoch'"

parent="$(cd "$(dirname "$bundle")" && pwd)"
root="$(basename "$bundle")"
case "$out" in
  /*) ;;
  *) out="$PWD/$out" ;;
esac

# The time as `touch -t` reads it, computed in UTC (GNU and BSD `date`
# spell "from these seconds" differently; Python says it one way).
stamp="$(python3 -c 'import sys, time; print(time.strftime("%Y%m%d%H%M.%S", time.gmtime(int(sys.argv[1]))))' "$epoch")"

files=()
while IFS= read -r name; do
  path="$parent/$root/$name"
  if [[ -L "$path" || ! -f "$path" ]]; then
    fail "$root/$name is not a regular file: a bundle holds files only"
  fi
  if [[ -x "$path" ]]; then
    chmod 0755 "$path"
  else
    chmod 0644 "$path"
  fi
  TZ=UTC touch -t "$stamp" "$path"
  files+=("$root/$name")
done < <(cd "$parent/$root" && find . -mindepth 1 -print | sed 's|^\./||' | LC_ALL=C sort)

(( ${#files[@]} > 0 )) || fail "$bundle is empty"
chmod 0755 "$parent/$root"
TZ=UTC touch -t "$stamp" "$parent/$root"

rm -f "$out"
(cd "$parent" && TZ=UTC env -u ZIP -u ZIPOPT zip -X -q "$out" "$root/" "${files[@]}")
