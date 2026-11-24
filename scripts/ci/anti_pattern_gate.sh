#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# anti_pattern_gate.sh - what the compiler and clippy cannot refuse.
#
# 1. Lint exceptions. No `#[allow(...)]` or `#[expect(...)]` in any form
#    (`#!`, `cfg_attr`, on a test module) in any Rust file of the
#    repository, tests included. A finding is fixed at its cause
#    (AGENTS.md, docs/policies/lint-posture.md). The one exception the
#    owner has allowed is listed in APPROVED below, by file and lint.
# 2. Bytes read as text. No lossy UTF-8 or UTF-16 decoding, and no
#    `to_string_lossy` of a name, in the production code of any crate of
#    the workspace, md2html and its tooling alike
#    (docs/policies/hostile_input_policy.md): decode strictly and handle
#    the failure; decide on the `OsStr` or `Path` itself; show a name to a
#    person with `Path::display`, which says what it is for.
# 3. Discarded writes to a stream or a pipe.
#
# There is no marker that silences a rule. Exit 1 on a finding, 2 when
# the check itself cannot run.

set -euo pipefail

# The one exception, decided by the owner on 2026-11-23: a drawing's
# geometry is floating-point, and this file is where its arithmetic lives.
APPROVED_FILE="crates/md2html/src/mermaid/geom.rs"
APPROVED_LINT="clippy::float_arithmetic"

status=0

# Report a finding and flip the exit status.
report() {
  printf '❌ %s\n   %s\n' "$1" "$2"
  status=1
}

if ! RUST_FILES="$(git ls-files -- '*.rs')"; then
  printf 'ERROR: git ls-files failed\n' >&2
  exit 2
fi
if [[ -z "$RUST_FILES" ]]; then
  printf 'ERROR: no Rust files tracked\n' >&2
  exit 2
fi
mapfile -t ALL_RS <<< "$RUST_FILES"

# ── 1. Lint exceptions ───────────────────────────────────────────────────
# An attribute opens with `#[allow(`, `#![expect(`, ... on one line
# (Rust allows blanks between `#`, `!`, `[` and the name: `# [ allow (`
# is the same attribute), or is a `cfg_attr(` whose arguments (on the
# same line or the next few) name `allow(` or `expect(`. A method call
# `.expect(` is not an attribute and does not count.
exceptions="$(awk '
  function flag(text) { printf "%s:%d: %s\n", FILENAME, FNR, text }
  FNR == 1 { pending = 0 }
  {
    line = $0
    if (line ~ /^[[:space:]]*\/\//) { next }
    if (line ~ /#[[:space:]]*!?[[:space:]]*\[[[:space:]]*(allow|expect)[[:space:]]*\(/) { flag(line); pending = 0; next }
    if (line ~ /cfg_attr[[:space:]]*\(/) {
      if (line ~ /(^|[^.[:alnum:]_])(allow|expect)[[:space:]]*\(/) { flag(line); next }
      pending = 4; opened = FNR; text = line; next
    }
    if (pending > 0) {
      pending--
      if (line ~ /(^|[^.[:alnum:]_])(allow|expect)[[:space:]]*\(/) {
        printf "%s:%d: %s\n", FILENAME, opened, text; pending = 0
      }
    }
  }
' "${ALL_RS[@]}")"
while IFS= read -r finding; do
  [[ -n "$finding" ]] || continue
  file="${finding%%:*}"
  rest="${finding#*:}"
  lineno="${rest%%:*}"
  # The approved attribute is an `expect` (it fails the day the lint no
  # longer fires; an `allow` would outlive its cause) naming exactly one
  # lint, the approved one.
  if [[ "$file" == "$APPROVED_FILE" && "$rest" =~ \[[[:space:]]*expect[[:space:]]*\( ]]; then
    window="$(sed -n "${lineno},$(( lineno + 4 ))p" "$file")"
    lints="$(printf '%s\n' "$window" | grep -Eo 'clippy::[a-z_]+' | sort -u)"
    if [[ "$lints" == "$APPROVED_LINT" ]]; then
      continue
    fi
  fi
  report "lint exception: fix the cause, never #[allow]/#[expect]" "$finding"
done <<< "$exceptions"

# The approved exception must still be there and needed: clippy's
# `unfulfilled_lint_expectations` (denied by -D warnings) reports it the
# day the arithmetic leaves the file. Nothing to check here.

# ── 2 and 3. Production code of every crate (tests excluded) ────────────
# The product's crates under `crates/`, the tooling's under `scripts/`
# (a `case` `*` also matches `/`, so `scripts/*/src/*` covers the crates
# under `scripts/ci/` too).
PROD_RS=()
for file in "${ALL_RS[@]}"; do
  case "$file" in
    crates/*/src/* | scripts/*/src/*) ;;
    *) continue ;;
  esac
  case "$file" in
    */tests.rs | */tests/* | */spec_tests.rs) continue ;;
  esac
  PROD_RS+=("$file")
done

# Run one rule. $1=label $2=extended regex; the files are PROD_RS.
run_rule() {
  local label="$1" pattern="$2" matches rc=0 file lineno content
  matches="$(grep -En -- "$pattern" "${PROD_RS[@]}")" || rc=$?
  if (( rc > 1 )); then
    printf 'ERROR: grep failed (exit %d) on rule: %s\n' "$rc" "$label" >&2
    exit 2
  fi
  while IFS=: read -r file lineno content; do
    [[ -n "$file" ]] || continue
    # A comment only mentions the name.
    if printf '%s\n' "$content" | grep -Eq '^[[:space:]]*//'; then
      continue
    fi
    report "$label" "$file:$lineno: ${content#"${content%%[![:space:]]*}"}"
  done <<< "$matches"
}

run_rule "bytes: lossy UTF-8 decoding; decode strictly and handle the failure" \
  'from_utf8_lossy'
run_rule "bytes: lossy UTF-16 decoding; decode strictly and handle the failure" \
  'from_utf16_lossy'
run_rule "names: a name read with U+FFFD for its bytes; decide on the OsStr or Path, show it with Path::display" \
  'to_string_lossy'
run_rule "errors: a discarded write to a stream or pipe; return or report it" \
  'drop\((stream|pipe)\.(write_all|flush)'

if [[ $status -eq 0 ]]; then
  printf '✅ anti-pattern gate: no lint exception beyond the approved one, no lossy decoding or names, no discarded write\n'
fi
exit "$status"
