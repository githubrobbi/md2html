#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# File-size gate: no Rust file of the workspace is longer than 800 lines.
#
# Every `.rs` file git tracks (the index, so a staged new file counts) is
# measured; a file the index lists but the working tree no longer has is
# being deleted and is not measured.  There is no exception list: a file
# past the limit is split into a module with children.

set -euo pipefail

limit=800

if ! files=$(git ls-files -- '*.rs'); then
  printf 'ERROR: git ls-files failed - cannot list the Rust files to measure\n' >&2
  exit 2
fi

status=0
checked=0
while IFS= read -r path; do
  [[ -n "$path" && -f "$path" ]] || continue
  checked=$((checked + 1))
  lines=$(wc -l < "$path")
  lines=$((lines + 0))
  if (( lines > limit )); then
    printf 'OVERSIZED: %s (%d lines, limit %d) - split it into a module with children\n' \
      "$path" "$lines" "$limit"
    status=1
  fi
done <<< "$files"

if (( status != 0 )); then
  exit "$status"
fi
printf 'File size policy OK: %d Rust files, none over %d lines\n' "$checked" "$limit"
