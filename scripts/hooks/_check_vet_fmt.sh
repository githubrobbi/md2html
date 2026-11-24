#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# Format-drift detector for cargo-vet's `supply-chain/` store.
#
# cargo-vet owns the format of `audits.toml`, `config.toml`, and
# `imports.lock`, so taplo leaves `supply-chain/` alone (`.taplo.toml`)
# and this script checks it instead.
# `cargo vet check` hard-fails at pre-push if the store drifts; this
# script is the pre-commit counterpart.
#
# `cargo vet fmt` is mutating-only in 0.10.x (no `--check` mode), so
# we snapshot the store, run the reformatter, and compare:
#   * if the reformatter produced the same bytes, we restore the
#     pre-fmt state so the working tree is untouched and exit 0;
#   * if any file changed, we restore the pre-fmt state (so the
#     operator's working tree is untouched) and exit 1 with a hint.
#
# Revisit once upstream cargo-vet adds a `fmt --check` mode; this
# script can then shrink to a single subprocess call.

set -euo pipefail

# Resolve to the workspace root - cargo vet has to run where
# `supply-chain/` lives, and the pre-commit hook cwd is already the
# workspace root, but make it explicit for standalone invocations.
cd "$(git rev-parse --show-toplevel)"

snap_dir=$(mktemp -d)
trap 'rm -rf -- "$snap_dir"' EXIT

cp -R supply-chain "$snap_dir/"
# Quiet when it works; when it fails (cargo-vet missing, a store it cannot
# read) its own message is the diagnosis, so it is shown.
if ! fmt_out=$(cargo vet fmt 2>&1); then
    printf '%s\n' "$fmt_out"
    printf 'cargo vet fmt failed - the store could not be checked\n'
    exit 2
fi

# diff: 0 the same, 1 different, anything else a failure to compare.
same=0
diff -rq supply-chain "$snap_dir/supply-chain" >/dev/null || same=$?
case "$same" in
    0) exit 0 ;; # No drift: nothing to do; trap cleans up $snap_dir.
    1) ;;
    *)
        printf 'diff failed (exit %d) - the store could not be compared\n' "$same"
        exit 2
        ;;
esac

# Drift found - restore pre-fmt state so the operator's working tree
# is untouched, then fail with an actionable hint.
rm -rf supply-chain
mv "$snap_dir/supply-chain" .

echo "supply-chain format drift detected."
echo "Run: cargo vet fmt && git add supply-chain/ && re-commit"
exit 1
