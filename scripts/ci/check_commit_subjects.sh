#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# Conventional Commits subject validator.
#
# **Why this exists** - `.github/workflows/commitlint.yml` validates the
# **PR title** at PR-open / synchronize time.  That catch is post-push:
# the contributor finds out about a malformed scope like
# `fix(sanitize, text)` only after the workflow has already failed
# upstream.  This script encodes the SAME regex as the CI workflow
# (single source of truth - see "REGEX" below) so the pre-push hook
# (`scripts/hooks/_lint_pre_push.sh`), the commit-msg hook
# (`scripts/hooks/commit-msg`) and `just pr` fail the offending subject
# locally.
#
# **Regex**: mirrors the `PATTERN=` line of `.github/workflows/commitlint.yml`.
# Updates to either MUST land in the same commit.
#
# **Modes**:
#   range RANGE              -- validate every non-merge commit subject
#                              in the git revision range RANGE (e.g.
#                              `origin/main..HEAD` or
#                              `BASE_OID..LOCAL_OID`).  An empty or
#                              merges-only range succeeds; a range git
#                              cannot resolve FAILS.
#   file PATH                -- validate the first non-comment, non-blank
#                              line of PATH (the format `git commit`
#                              passes to `commit-msg` hooks).
#   subject "<subject line>" -- validate a single literal subject string.
#                              Useful for ad-hoc shell checks and tests.
#
# **Exit codes**: 0 conformant, 1 non-conformant (or input error).
#
# There is deliberately no bypass: the server-side title check applies the
# same regex, so skipping the local check only defers the same failure.
#
# **Allowed types** (mirrors `.github/workflows/commitlint.yml` and
# `docs/CONTRIBUTING.md` -> "Commit message conventions"): feat, fix, perf,
# refactor, docs, test, build, ci, chore, style, revert.
#
# **Scope grammar**: `[a-z0-9-]+`.  Single token -- no spaces, no
# commas, no slashes.  Use a single canonical scope per commit
# (e.g. `fix(sanitize)` rather than `fix(sanitize, text)`); work that
# spans several areas goes in the body.
#
# **Breaking-change marker**: an optional `!` between the scope and
# the colon (`feat(cli)!: rename --out to --output`).

set -euo pipefail

# ── Single source of truth for the Conventional Commits regex ───────
# Synchronised with the `PATTERN=` line of `.github/workflows/commitlint.yml`
# and the type list of `docs/CONTRIBUTING.md`.  Update all three together.
readonly REGEX='^(feat|fix|perf|refactor|docs|test|build|ci|chore|style|revert)(\([a-z0-9-]+\))?!?: .{1,}$'

# ── Colours ─────────────────────────────────────────────────────────
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
    C_GREEN=$'\033[0;32m'
    C_RED=$'\033[0;31m'
    C_YELLOW=$'\033[1;33m'
    C_CYAN=$'\033[0;36m'
    C_RESET=$'\033[0m'
else
    C_GREEN=''
    C_RED=''
    C_YELLOW=''
    C_CYAN=''
    C_RESET=''
fi

usage() {
    cat <<USAGE >&2
Usage: $(basename "$0") <mode> <arg>

Modes:
  range RANGE        Validate non-merge commit subjects in RANGE
                     (e.g. 'origin/main..HEAD').
  file PATH          Validate the first non-comment line of PATH
                     (the commit-msg hook input format).
  subject "TEXT"     Validate a single literal subject string.

Examples:
  $(basename "$0") range origin/main..HEAD
  $(basename "$0") file .git/COMMIT_EDITMSG
  $(basename "$0") subject 'fix(sanitize): drop javascript: links in reference definitions'
USAGE
    exit 2
}

# ── Helper: validate one literal subject ───────────────────────────
# Prints the failing subject + a hint when non-conformant; returns
# the appropriate exit code.  Caller aggregates failures across a
# batch.
validate_one() {
    local subject="$1"
    local oid="${2:-}"
    if [[ -z "$subject" ]]; then
        printf '%s[FAIL]%s empty subject%s\n' "$C_RED" "$C_RESET" \
            "${oid:+ (commit ${oid:0:10})}" >&2
        return 1
    fi
    if [[ "$subject" =~ $REGEX ]]; then
        return 0
    fi
    printf '%s[FAIL]%s non-conventional commit subject%s\n' \
        "$C_RED" "$C_RESET" "${oid:+ (commit ${oid:0:10})}" >&2
    printf '   %ssubject:%s %s\n' "$C_CYAN" "$C_RESET" "$subject" >&2
    return 1
}

print_help_block() {
    cat >&2 <<HELP
${C_YELLOW}-> Expected:${C_RESET} ${C_CYAN}type(scope): subject${C_RESET}
   where ${C_CYAN}type${C_RESET} is one of: feat, fix, perf, refactor, docs,
   test, build, ci, chore, style, revert; and ${C_CYAN}scope${C_RESET} is a
   SINGLE [a-z0-9-]+ token (e.g. ${C_CYAN}md2html${C_RESET},
   ${C_CYAN}sanitize${C_RESET}, ${C_CYAN}pipeline${C_RESET} -- no commas,
   no spaces, no slashes).

${C_YELLOW}Examples${C_RESET}:
   ${C_GREEN}fix(sanitize): drop javascript: links in reference definitions${C_RESET}
   ${C_GREEN}refactor(md2html): numeric types say what they mean${C_RESET}
   ${C_GREEN}ci: the anti-pattern gate runs on the Linux runner only${C_RESET}
   ${C_GREEN}feat(cli)!: rename --out to --output${C_RESET}  (the trailing ${C_CYAN}!${C_RESET} marks a breaking change)

${C_YELLOW}Fix${C_RESET}: reword the offending commit's subject:
   ${C_CYAN}git commit --amend -m "<type>(<scope>): <subject>"${C_RESET}   the most recent commit
   ${C_CYAN}git rebase --exec ... / a fresh branch${C_RESET}            an older one

See ${C_CYAN}docs/CONTRIBUTING.md${C_RESET} -> "Commit message conventions" and
\`.github/workflows/commitlint.yml\` for the canonical reference.
HELP
}

# ── Main ────────────────────────────────────────────────────────────
[[ $# -ge 2 ]] || usage
mode="$1"
arg="$2"
fail=0

case "$mode" in
    range)
        # Walk every non-merge commit in RANGE.  --no-merges drops
        # merge commits (Conventional Commits doesn't apply to merges
        # -- they get auto-generated subjects like "Merge branch ...").
        # An empty range exits 0 (e.g. push of a branch already at the
        # remote head).
        if ! git rev-parse --git-dir >/dev/null 2>&1; then
            printf '%s[FAIL]%s not in a git repository\n' "$C_RED" "$C_RESET" >&2
            exit 1
        fi
        # Read the log first: a range git cannot resolve (a missing
        # `origin/main`, a typo) is an error, never an empty range.
        if ! log_out=$(git log --no-merges --format='%H%x09%s' "$arg"); then
            printf '%s[FAIL]%s git cannot read the range %s - run: git fetch origin (or fix the range)\n' \
                "$C_RED" "$C_RESET" "$arg" >&2
            exit 1
        fi
        count=0
        while IFS=$'\t' read -r oid subject; do
            [[ -n "$oid" ]] || continue
            count=$((count + 1))
            validate_one "$subject" "$oid" || fail=1
        done <<< "$log_out"
        if (( fail )); then
            printf '\n' >&2
            print_help_block
            exit 1
        fi
        # Quiet success unless the user explicitly asked for verbosity.
        if [[ "${COMMIT_SUBJECT_VERBOSE:-0}" == "1" ]]; then
            printf '%s[OK]%s %d commit subject(s) in %s pass Conventional Commits\n' \
                "$C_GREEN" "$C_RESET" "$count" "$arg"
        fi
        ;;

    file)
        # `commit-msg` hook input: the first non-blank, non-comment
        # line is the subject.  `git commit` strips comments before
        # finalising the message so we mirror that here.
        if [[ ! -f "$arg" ]]; then
            printf '%s[FAIL]%s file not found: %s\n' "$C_RED" "$C_RESET" "$arg" >&2
            exit 1
        fi
        # Read the whole file (commit messages are tiny) and pull the
        # first qualifying line.
        subject=$(awk '!/^#/ && NF { print; exit }' "$arg")
        # An empty subject is not conventional either: git aborts an
        # empty message on its own, but not one made with
        # --allow-empty-message.  validate_one reports it.
        validate_one "$subject" || { print_help_block; exit 1; }
        ;;

    subject)
        validate_one "$arg" || { print_help_block; exit 1; }
        ;;

    *)
        usage
        ;;
esac
