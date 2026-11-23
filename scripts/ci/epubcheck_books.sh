#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# The repository's own documents as EPUBs, each checked by epubcheck, the
# W3C's validator: the README, the showcase, the capabilities page and its
# translations (19 books). A book passes with no message at all: no fatal
# error, no error, no warning. CI's `epub` job and `just epubcheck` run it.
#
# Usage: epubcheck_books.sh MD2HTML OUT_DIR CHECKER...
#   MD2HTML   the md2html binary to render with
#   OUT_DIR   where the books and the checker's reports go (emptied first)
#   CHECKER   the command that runs epubcheck: `epubcheck`, or
#             `java -jar /path/to/epubcheck.jar`

set -euo pipefail

if (( $# < 3 )); then
  printf 'usage: %s MD2HTML OUT_DIR CHECKER...\n' "$0" >&2
  exit 2
fi
md2html=$1
out=$2
shift 2
checker=("$@")

if ! command -v "${checker[0]}" >/dev/null 2>&1; then
  printf 'epubcheck cannot run: %s is not installed - install it with: just install-dev-tools\n' "${checker[0]}" >&2
  exit 1
fi

rm -rf "$out"
mkdir -p "$out"

books=0
failed=()
for doc in README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md; do
  name=$(basename "$doc" .md)
  # The page's own warnings (links the book leaves out) are the author's
  # to read; the check is epubcheck's.
  "$md2html" --epub -q "$doc" -o "$out/$name.html"
  books=$((books + 1))
  report="$out/$name.epubcheck.txt"
  if "${checker[@]}" --failonwarnings "$out/$name.epub" >"$report" 2>&1 \
    && grep -qx 'Messages: 0 fatals / 0 errors / 0 warnings / 0 infos' "$report"; then
    printf 'ok      %s\n' "$doc"
  else
    printf 'FAILED  %s\n' "$doc"
    cat "$report"
    failed+=("$doc")
  fi
done

if (( ${#failed[@]} > 0 )); then
  printf '%d of %d books do not pass epubcheck: %s\n' "${#failed[@]}" "$books" "${failed[*]}" >&2
  exit 1
fi
printf '%d books pass epubcheck: 0 errors, 0 warnings\n' "$books"
