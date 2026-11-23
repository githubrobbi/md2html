# shellcheck shell=bash
#
# Called by:
#   - `scripts/hooks/pre-commit` (git hook)
#   - `just lint-fast`           (manual runs)
#
# Architecture:
#
#   Single-bucket parallel gate.  Per-gate routing (which file
#   classes each gate fires on) is driven by `gates.toml` and
#   rendered into the dispatch section below.
#
# Budget:
#   * docs / config-only commits:        sub-2 s
#   * Rust commits (warm sccache):       ~8-15 s (three clippy passes;
#                                        cargo's target-dir lock serialises
#                                        them so the 2nd and 3rd are
#                                        incremental-cheap)
#
# The Windows cross-lint (cargo-xwin) is not in this gate: its 40-90 s
# cold cost does not fit the budget above.  It runs at pre-push
# (`lint-ci-windows`, required) and in `pr-fast.yml` (the native
# `windows-lint` job).
#
# Every gate is required, and so is its tool: a missing tool (typos,
# taplo, reuse, ...) fails the gate, and the footer names the tools in
# REQUIRED_TOOLS that are missing and `just install-dev-tools`.
#
# Per-gate documentation (label, command, rationale, expected runtime,
# CI counterpart) lives in `scripts/ci/gates.toml`'s `[[gate]]` tables
# - that is the single source of truth, and the generator preserves
# it on every regen.

set -euo pipefail

# ── Colours ────────────────────────────────────────────────────────────
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
    C_BLUE=$'\033[0;34m'
    C_CYAN=$'\033[0;36m'
    C_GREEN=$'\033[0;32m'
    C_RED=$'\033[0;31m'
    C_RESET=$'\033[0m'
else
    # Explicit empty-string form.  The `VAR=` shorthand works but
    # trips SC1007 because a stray trailing space would turn it into
    # `VAR= other_cmd` (a single-line env override in front of a
    # command invocation).  Using `VAR=''` makes the intent
    # unambiguous and keeps the linter quiet.
    C_BLUE=''
    C_CYAN=''
    C_GREEN=''
    C_RED=''
    C_RESET=''
fi

# ── Staged-file inventory ──────────────────────────────────────────────
# Tool-routing rationale (see also .taplo.toml):
# `supply-chain/*.toml` is cargo-vet's data store, formatted by
# `cargo vet fmt` (which has opinions taplo does not share - e.g.
# column-aligned trailing comments).  Two formatters fighting over the
# same file is a pre-push dead-end, so we split ownership at the
# pre-commit hook: taplo handles every other TOML; cargo-vet handles
# the store.
if ! STAGED_ALL=$(git diff --cached --name-only --diff-filter=ACMRD); then
    printf '%s❌ lint-fast: git diff --cached failed - cannot tell what is staged%s\n' \
        "$C_RED" "$C_RESET" >&2
    exit 2
fi
# The lines of stdin that match (`grep "$@"`); no match (grep's exit 1)
# is an empty result, any other grep failure stops the hook.
select_lines() {
    local status=0
    grep "$@" || status=$?
    if (( status > 1 )); then
        printf 'lint-fast: grep %s failed (exit %d)\n' "$*" "$status" >&2
        exit 2
    fi
}
STAGED_TOML=$(printf '%s\n' "$STAGED_ALL" | select_lines '\.toml$')
STAGED_TOML_NONVET=$(printf '%s\n' "$STAGED_TOML" | select_lines -v '^supply-chain/')
STAGED_VET=$(printf '%s\n' "$STAGED_TOML" | select_lines '^supply-chain/')
has_staged_rs()          { printf '%s\n' "$STAGED_ALL" | grep -q '\.rs$';   }
has_staged_toml_nonvet() { [[ -n "${STAGED_TOML_NONVET//[[:space:]]/}" ]]; }
has_staged_vet()         { [[ -n "${STAGED_VET//[[:space:]]/}" ]]; }
has_any_staged()         { [[ -n "${STAGED_ALL//[[:space:]]/}" ]]; }

printf '%s🚦 lint-fast - staged-scoped parallel gate%s\n' "$C_BLUE" "$C_RESET"
START=$(date +%s)

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

NAMES=()
PIDS=()
spawn() {
    local name="$1"
    shift
    NAMES+=("$name")
    ( "$@" ) >"$TMP/$name.out" 2>&1 &
    PIDS+=($!)
}

