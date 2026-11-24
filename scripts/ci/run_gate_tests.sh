#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# Gate `gate-tests`: the tests of the shell scripts that guard this
# repository, every `test_*.sh` git tracks under `scripts/` (the
# anti-pattern gate's, the no-bypass guard's, and any added later: a new
# one is run without being listed here).  This runner is not named
# `test_*.sh`, or it would run itself.  Each test exits non-zero on a
# failed case.  Exit 1 when one fails, 2 when there is nothing to run.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

if ! listed="$(git ls-files -- 'scripts/*test_*.sh')"; then
  printf 'ERROR: git ls-files failed\n' >&2
  exit 2
fi
if [[ -z "$listed" ]]; then
  printf 'ERROR: no scripts/**/test_*.sh tracked - the gates have no tests to run\n' >&2
  exit 2
fi

failed=()
count=0
while IFS= read -r test; do
  count=$((count + 1))
  printf '── %s\n' "$test"
  if ! bash "$test"; then
    failed+=("$test")
  fi
done <<< "$listed"

if (( ${#failed[@]} > 0 )); then
  printf '❌ %d of %d gate tests failed: %s\n' "${#failed[@]}" "$count" "${failed[*]}"
  exit 1
fi
printf '✅ %d gate tests passed\n' "$count"
