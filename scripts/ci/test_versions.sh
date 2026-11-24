#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of scripts/ci/versions.py (gate `versions`), run by the gate
# `gate-tests`.  Each case copies the files the script reads into a fresh
# folder, plants one disagreement, and expects `check` to refuse it; the
# repository as it is must pass; `stamp` must make a bumped copy agree
# again, renew the fingerprint of a current translation and leave a stale
# one stale.  No git repository is made: the script reads files only.
# Exit 1 when a case fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/ci/versions.py"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

failed=0
count=0

# A fresh copy of every file versions.py reads, in "$scratch/tree".
fresh() {
  rm -rf "$scratch/tree"
  mkdir -p "$scratch/tree/crates/md2html" "$scratch/tree/docs"
  cp "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/CHANGELOG.md" "$repo/README.md" "$scratch/tree/"
  cp "$repo/crates/md2html/Cargo.toml" "$scratch/tree/crates/md2html/"
  mkdir -p "$scratch/tree/docs/policies"
  cp "$repo/docs/SUPPLY-CHAIN.md" "$repo"/docs/capabilities*.md "$scratch/tree/docs/"
  cp "$repo/docs/policies/release_policy.md" "$scratch/tree/docs/policies/"
}

# Replace the first `old` with `new` in a file of the copy; fail the case
# when `old` is not there (the test would prove nothing).
plant() {
  python3 - "$scratch/tree/$1" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1:4]
text = open(path, encoding="utf-8").read()
if old not in text:
    sys.exit(f"plant: {old!r} is not in {path}")
open(path, "w", encoding="utf-8", newline="\n").write(text.replace(old, new, 1))
PY
}

# The current version, from the repository.
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$repo/Cargo.toml" | head -n 1)"

# Expect `check` on the copy to pass (0) or fail (1) and, when failing,
# to name `needle`.
expect() {
  local name="$1" want="$2" needle="$3" got=0 out
  shift 3
  count=$((count + 1))
  out="$(python3 "$script" --root "$scratch/tree" check "$@" 2>&1)" || got=$?
  if [[ "$got" != "$want" ]] || { [[ -n "$needle" ]] && [[ "$out" != *"$needle"* ]]; }; then
    printf '  ❌ %s: exit %s (wanted %s)\n%s\n' "$name" "$got" "$want" "$out"
    failed=$((failed + 1))
  else
    printf '  ✅ %s\n' "$name"
  fi
}

fresh
expect "the repository as it is agrees" 0 ""
expect "a tag that names the version" 0 "" --tag "v$version"
expect "a tag that names another version" 1 "does not name the version" --tag "v9.9.9"

fresh; plant Cargo.toml "version = \"$version\"" 'version = "9.9.9"'
expect "Cargo.toml alone bumped" 1 "not 9.9.9"

fresh; plant Cargo.toml "path = \"crates/md2html-version\", version = \"$version\"" 'path = "crates/md2html-version", version = "0.0.1"'
expect "a path dependency that requires another version" 1 "lockstep"

fresh; plant Cargo.lock "name = \"md2html\"
version = \"$version\"" 'name = "md2html"
version = "0.0.1"'
expect "Cargo.lock behind" 1 "Cargo.lock: md2html"

fresh; plant docs/capabilities.de.md "version: $version" "version: 0.0.1"
expect "a translation's front matter behind" 1 "capabilities.de.md"

fresh; plant docs/capabilities.md "**md2html $version**" "**md2html 0.0.1**"
expect "a footer that names another version" 1 "names md2html 0.0.1"

fresh; plant CHANGELOG.md "## [$version] - " "## [0.0.1] - "
expect "the newest changelog heading behind" 1 "newest release is 0.0.1"

fresh; plant CHANGELOG.md "## [Unreleased]" "## Unreleased"
expect "no [Unreleased] heading for the roll" 1 "Unreleased"

fresh; plant CHANGELOG.md "compare/v$version...HEAD" "compare/v0.0.1...HEAD"
expect "the Unreleased link from another version" 1 "compares from v0.0.1"

fresh; plant README.md "or newer." "or later."
expect "the README's minimum Rust differs from rust-version" 1 "README.md"

# A release made the way the checklist says: the manifest, the lock, the
# changelog, then `stamp`.  One translation was stale before; it stays so.
fresh
next="9.8.7"
plant Cargo.toml "version = \"$version\"" "version = \"$next\""
plant Cargo.toml "path = \"crates/md2html-version\", version = \"$version\"" "path = \"crates/md2html-version\", version = \"$next\""
python3 - "$scratch/tree/Cargo.lock" "$version" "$next" <<'PY'
import re, sys
path, old, new = sys.argv[1:4]
text = open(path, encoding="utf-8").read()
blocks = text.split("[[package]]")
for i, block in enumerate(blocks):
    if i and not re.search(r"^source\s*=", block, re.M):
        blocks[i] = block.replace(f'version = "{old}"', f'version = "{new}"', 1)
open(path, "w", encoding="utf-8", newline="\n").write("[[package]]".join(blocks))
PY
plant CHANGELOG.md "## [Unreleased]" "## [Unreleased]

## [$next] - 2030-01-02"
plant CHANGELOG.md "compare/v$version...HEAD" "compare/v$next...HEAD
[$next]: https://example.invalid/compare/v$version...v$next"
plant docs/capabilities.fr.md "source_sha256: " "source_sha256: 00"
count=$((count + 1))
if out="$(python3 "$script" --root "$scratch/tree" stamp "$next" --date 2030-01-02 2>&1)" \
  && [[ "$out" == *"capabilities.fr.md: was stale before"* ]]; then
  printf '  ✅ stamp writes the pages and keeps a stale translation stale\n'
else
  printf '  ❌ stamp failed or renewed a stale translation\n%s\n' "$out"
  failed=$((failed + 1))
fi
expect "a stamped release agrees" 0 "" --tag "v$next"
fingerprint="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$scratch/tree/docs/capabilities.md")"
count=$((count + 1))
if grep -q "^source_sha256: $fingerprint$" "$scratch/tree/docs/capabilities.de.md" \
  && ! grep -q "^source_sha256: $fingerprint$" "$scratch/tree/docs/capabilities.fr.md" \
  && grep -q "^date: 2030-01-02$" "$scratch/tree/docs/capabilities.md"; then
  printf '  ✅ a current translation gets the new fingerprint, the date is set\n'
else
  printf '  ❌ fingerprints or date not as expected after stamp\n'
  failed=$((failed + 1))
fi
count=$((count + 1))
if python3 "$script" --root "$scratch/tree" notes "$next" >/dev/null 2>&1 \
  && ! python3 "$script" --root "$scratch/tree" notes 0.0.1 >/dev/null 2>&1; then
  printf '  ✅ notes finds a released section and only that\n'
else
  printf '  ❌ notes\n'
  failed=$((failed + 1))
fi

if ((failed > 0)); then
  printf '❌ versions.py: %d of %d cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ versions.py: %d cases\n' "$count"
