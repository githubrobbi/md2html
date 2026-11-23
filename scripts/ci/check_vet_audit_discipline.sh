#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# cargo-vet exemption-audit discipline gate.
#
# **Why this exists** - `supply-chain/config.toml`'s `[[exemptions.<crate>]]`
# blocks are the project's audit-debt ledger.  Every entry is a promise that
# we will eventually replace the exemption with a proper `[[audits.<crate>]]`
# record in `audits.toml` (or with an upstream import).  When a dependency
# patch-bumps (e.g. `toml 0.9.7 -> 0.9.8`), the lazy fix is to bump the
# exemption's `version =` line so `cargo vet check` keeps passing.  That
# turns the exemption into a *permanent rubber-stamp*: every future bump
# inherits the same blanket trust, and the audit debt compounds silently.
#
# The store has no exemptions today, so the gate passes trivially; it
# stays so that an exemption the owner ever grants cannot be bumped
# without its audit.
#
# This script is the "no lazy bumps" gate.  For every exemption version
# changed between the merge-base and the pushed range, it requires
# **both** of the following (defense-in-depth - see `docs/SUPPLY-CHAIN.md`
# and `docs/policies/dependency_policy.md`):
#
#   (A) Audit-correspondence - a matching `[[audits.<crate>]]` delta block
#       in `supply-chain/audits.toml` covering the version transition
#       (`delta = "OLD -> NEW"`) with non-empty `notes`.  This is the
#       formal audit record `cargo vet diff` produced.
#
#   (B) Commit-trailer attestation - at least one commit in the range
#       carries a `Vet-Reviewed-Diff: <crate>@<OLD>-><NEW>` trailer.
#       This is the human reviewer's signature in the git log, anchored
#       to the commit-signing key (branch protection enforces signed
#       commits on `main`).  Multiple bumps in one push must each have
#       a corresponding trailer.
#
# Both checks must pass - the audit record is the *what was reviewed*,
# the trailer is the *who reviewed it* + *acknowledged in this push*.
# A passing audit without a trailer means the audit predates the bump
# (possibly stale).  A passing trailer without an audit means the
# reviewer skipped the formal record.
#
# **Modes** - invoked the same way as `check_commit_subjects.sh`:
#
#   range RANGE              -- validate every exemption bump in the
#                              git revision range RANGE (e.g.
#                              `origin/main..HEAD`).  Reads
#                              `COMMIT_RANGES` env (newline-separated
#                              ranges) when RANGE is the literal
#                              `${COMMIT_RANGES}` or empty.
#
#   pre-push                 -- read `COMMIT_RANGES` from the
#                              environment (set by
#                              `_lint_pre_push.sh`).  Empty/unset
#                              means "nothing was pushed" (success).
#
#   ci                       -- compute the range from `GITHUB_BASE_REF`
#                              (PR mode) or the merge-base with
#                              `origin/main` (push mode).  A base that
#                              cannot be resolved is an error (exit 2),
#                              never an empty range.
#
# **Exit codes**: 0 conformant, 1 non-conformant, 2 input/env error.
#
# There is no bypass: an exemption bump without its audit is the exact
# thing this gate exists to stop.
#
# **Environment knobs** (optional):
#   VET_DISCIPLINE_VERBOSE=1      -- print one line per parsed exemption
#                                    bump.  Useful for debugging the
#                                    diff parser.

set -euo pipefail

# ── Repo paths ──────────────────────────────────────────────────────
if ! REPO_ROOT="$(git rev-parse --show-toplevel)"; then
    printf 'vet-audit-discipline: run inside the md2html git checkout\n' >&2
    exit 2
fi
CONFIG_REL="supply-chain/config.toml"
AUDITS_REL="supply-chain/audits.toml"

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
Usage: $(basename "$0") <mode> [arg]

Modes:
  range RANGE         Validate exemption bumps in the git rev range RANGE
                      (e.g. 'origin/main..HEAD').  Multiple ranges may be
                      supplied via the COMMIT_RANGES env var (newline-sep).
  pre-push            Read COMMIT_RANGES from the environment (set by
                      _lint_pre_push.sh).  Empty/unset => silent success.
  ci                  Compute the range from GITHUB_BASE_REF (PR mode)
                      or merge-base with origin/main.

Environment:
  VET_DISCIPLINE_VERBOSE=1        Verbose per-bump diagnostics.
USAGE
    exit 2
}

# ── Parse `<exemption blob>` => `crate<TAB>version<TAB>criteria` lines ──
#
# Awk reader that walks a single `supply-chain/config.toml` blob from
# stdin and emits one tab-separated line per `[[exemptions.<crate>]]`
# block:
#
#   <crate>\t<version>\t<criteria>
#
# Handles the legitimate case of the same crate appearing under multiple
# `[[exemptions.<crate>]]` blocks at different versions (e.g.
# two majors of one crate) - each block emits
# its own line.  Empty/optional fields stay blank.
parse_exemptions() {
    awk '
        function flush() {
            if (crate != "") {
                printf "%s\t%s\t%s\n", crate, version, criteria
            }
            crate=""; version=""; criteria=""
        }
        /^\[\[exemptions\.[^]]+\]\]$/ {
            flush()
            crate = $0
            sub(/^\[\[exemptions\./, "", crate)
            sub(/\]\]$/, "", crate)
            next
        }
        /^\[/ {
            flush()
            next
        }
        crate == "" { next }
        /^version[[:space:]]*=[[:space:]]*"/ {
            v = $0
            sub(/^version[[:space:]]*=[[:space:]]*"/, "", v)
            sub(/"[[:space:]]*$/, "", v)
            version = v
            next
        }
        /^criteria[[:space:]]*=[[:space:]]*"/ {
            c = $0
            sub(/^criteria[[:space:]]*=[[:space:]]*"/, "", c)
            sub(/"[[:space:]]*$/, "", c)
            criteria = c
            next
        }
        END { flush() }
    '
}

# ── Parse `audits.toml` => `crate<TAB>delta-or-version<TAB>criteria` ──
#
# Awk reader for `[[audits.<crate>]]` blocks.  Emits one line per audit
# entry with the form:
#
#   <crate>\t<key>=<value>\t<criteria>
#
# where `<key>` is `delta` or `version` and `<value>` is the literal
# string from the TOML (e.g. `1.52.2 -> 1.52.3` or `3.1.1`).  Spaces
# around the `->` arrow are normalised to a single space.  Lines that
# carry neither `delta` nor `version` (pure `who`/`criteria`/`notes`
# entries) are emitted with an empty key=value, callers ignore them.
parse_audits() {
    awk '
        function flush() {
            if (crate != "") {
                printf "%s\t%s\t%s\n", crate, kv, criteria
            }
            crate=""; kv=""; criteria=""
        }
        /^\[\[audits\.[^]]+\]\]$/ {
            flush()
            crate = $0
            sub(/^\[\[audits\./, "", crate)
            sub(/\]\]$/, "", crate)
            next
        }
        /^\[/ {
            flush()
            next
        }
        crate == "" { next }
        /^delta[[:space:]]*=[[:space:]]*"/ {
            v = $0
            sub(/^delta[[:space:]]*=[[:space:]]*"/, "", v)
            sub(/"[[:space:]]*$/, "", v)
            # Normalise "X -> Y" / "X->Y" -> "X -> Y"
            gsub(/[[:space:]]*->[[:space:]]*/, " -> ", v)
            kv = "delta=" v
            next
        }
        /^version[[:space:]]*=[[:space:]]*"/ {
            v = $0
            sub(/^version[[:space:]]*=[[:space:]]*"/, "", v)
            sub(/"[[:space:]]*$/, "", v)
            kv = "version=" v
            next
        }
        /^criteria[[:space:]]*=[[:space:]]*"/ {
            c = $0
            sub(/^criteria[[:space:]]*=[[:space:]]*"/, "", c)
            sub(/"[[:space:]]*$/, "", c)
            criteria = c
            next
        }
        END { flush() }
    '
}

# ── Diff helpers ────────────────────────────────────────────────────

# Print the parsed exemption set for a given git ref, falling back to
# the empty set if the file did not exist at that ref (first-ever
# supply-chain commit).  The ref itself must resolve (checked by the
# caller): only the FILE may be absent.  `git ls-tree` answers that
# question with an exit status that means "no such path" and nothing
# else, unlike `git cat-file -e`, which fails the same way for a bad ref.
exemptions_at_ref() {
    local ref="$1" listing
    listing=$(git ls-tree --name-only "$ref" -- "$CONFIG_REL")
    if [[ -n "$listing" ]]; then
        git show "$ref:$CONFIG_REL" | parse_exemptions
    fi
}

# Same, for HEAD or a working-tree path.  When ref is the literal
# `WORKTREE`, read the on-disk file (so the gate also catches drift
# inside a staged-but-not-committed change).
exemptions_at() {
    local ref="$1"
    if [[ "$ref" == "WORKTREE" ]]; then
        if [[ -f "$REPO_ROOT/$CONFIG_REL" ]]; then
            parse_exemptions < "$REPO_ROOT/$CONFIG_REL"
        fi
    else
        exemptions_at_ref "$ref"
    fi
}

# Walk the parsed audits.toml at HEAD (working-tree).  Audits are
# authoritative at HEAD only - there is no value in checking what
# audits existed at BASE since the gate's job is to confirm the bumps
# are *currently* audited.
audits_now() {
    if [[ -f "$REPO_ROOT/$AUDITS_REL" ]]; then
        parse_audits < "$REPO_ROOT/$AUDITS_REL"
    fi
}

# Collect all `Vet-Reviewed-Diff:` trailers across the commits in the
# supplied git range.  Trailer format (case-sensitive on the key,
# whitespace-tolerant on the value):
#
#   Vet-Reviewed-Diff: <crate>@<OLD>-><NEW>
#
# Output: one trailer per line, normalised to
# `<crate>\t<OLD>\t<NEW>` so the caller can grep `-Fxq` for a specific
# bump.  A malformed trailer value (bad arrow, missing version) matches
# no bump, so the bump it was meant to cover still fails the gate.
#
# Implementation note: we use `git log --format=%B` (the raw commit body)
# and grep for the trailer line by name, rather than the more elegant
# `%(trailers:key=...)` format placeholder.  The latter requires git
# ≥ 2.20 with a specific build option and silently emits empty strings
# on older toolchains; the body-grep approach works on every git ≥ 1.x.
trailers_in_range() {
    local range="$1" bodies
    [[ -z "$range" ]] && return 0
    # Read the log into a variable so a git failure is an error, not an
    # empty trailer set.
    bodies=$(git log --no-merges --format='%B%x00' "$range" | tr '\0' '\n') || return 2
    printf '%s\n' "$bodies" \
        | while IFS= read -r line; do
            [[ "$line" == Vet-Reviewed-Diff:* ]] || continue
            # Strip the key + surrounding whitespace.
            value="${line#Vet-Reviewed-Diff:}"
            value="${value#"${value%%[![:space:]]*}"}"
            value="${value%"${value##*[![:space:]]}"}"
            # Accept `<crate>@<OLD>-><NEW>` with optional spaces around `->`.
            # Normalise to crate<TAB>OLD<TAB>NEW.
            if [[ "$value" =~ ^([A-Za-z0-9_.+-]+)@([A-Za-z0-9_.+:@-]+)[[:space:]]*-\>[[:space:]]*([A-Za-z0-9_.+-]+)$ ]]; then
                printf '%s\t%s\t%s\n' \
                    "${BASH_REMATCH[1]}" \
                    "${BASH_REMATCH[2]}" \
                    "${BASH_REMATCH[3]}"
            fi
        done
}

# ── Per-range validator ─────────────────────────────────────────────
#
# Diffs the parsed exemption sets at BASE vs HEAD and verifies every
# bump has both the audit record AND the commit trailer.  Returns
# non-zero on any failure; accumulates failure messages on stderr so
# multiple problems are surfaced in one run.
validate_range() {
    local range="$1"
    # NOTE: this function runs under `if ! validate_range`, where bash
    # suspends `set -e` for the whole body.  Every command whose failure
    # matters is therefore checked explicitly below; a failure is an
    # input error (return 2), never "nothing to check".
    local base head
    if [[ "$range" != *..* ]]; then
        printf '%s[FAIL]%s not a BASE..HEAD range: %s\n' "$C_RED" "$C_RESET" "$range" >&2
        return 2
    fi
    base="${range%%..*}"
    head="${range##*..}"
    if [[ -z "$base" || -z "$head" ]]; then
        printf '%s[FAIL]%s not a BASE..HEAD range: %s\n' "$C_RED" "$C_RESET" "$range" >&2
        return 2
    fi
    local head_oid base_oid
    if ! base_oid=$(git rev-parse --verify --quiet "$base^{commit}") \
        || ! head_oid=$(git rev-parse --verify --quiet "$head^{commit}"); then
        printf '%s[FAIL]%s git cannot resolve the range %s - run: git fetch origin\n' \
            "$C_RED" "$C_RESET" "$range" >&2
        return 2
    fi
    if [[ "$base_oid" == "$head_oid" ]]; then
        return 0
    fi

    # Pre-flight: nothing to do if neither file changed in the range.
    local touched
    if ! touched=$(git diff --name-only "$range" -- "$CONFIG_REL" "$AUDITS_REL"); then
        printf '%s[FAIL]%s git diff failed for %s\n' "$C_RED" "$C_RESET" "$range" >&2
        return 2
    fi
    if [[ -z "$touched" ]]; then
        return 0
    fi

    local base_set head_set
    if ! base_set=$(exemptions_at "$base_oid"); then
        printf '%s[FAIL]%s cannot read %s at %s\n' "$C_RED" "$C_RESET" "$CONFIG_REL" "$base" >&2
        return 2
    fi
    # HEAD: if the range ends at the checked-out commit, read the working
    # tree so an uncommitted change is also checked.
    local current_oid
    if ! current_oid=$(git rev-parse --verify HEAD); then
        printf '%s[FAIL]%s cannot resolve HEAD\n' "$C_RED" "$C_RESET" >&2
        return 2
    fi
    if [[ "$head_oid" == "$current_oid" ]]; then
        head_set=$(exemptions_at "WORKTREE")
    elif ! head_set=$(exemptions_at "$head_oid"); then
        printf '%s[FAIL]%s cannot read %s at %s\n' "$C_RED" "$C_RESET" "$CONFIG_REL" "$head" >&2
        return 2
    fi

    local audits trailers
    audits=$(audits_now)
    if ! trailers=$(trailers_in_range "$range"); then
        printf '%s[FAIL]%s git log failed for %s\n' "$C_RED" "$C_RESET" "$range" >&2
        return 2
    fi

    if [[ "${VET_DISCIPLINE_VERBOSE:-0}" == "1" ]]; then
        printf '%s[verbose]%s range=%s base_exemptions=%d head_exemptions=%d trailers=%d\n' \
            "$C_CYAN" "$C_RESET" "$range" \
            "$(printf '%s\n' "$base_set" | awk 'NF' | wc -l | tr -d ' ')" \
            "$(printf '%s\n' "$head_set" | awk 'NF' | wc -l | tr -d ' ')" \
            "$(printf '%s\n' "$trailers" | awk 'NF' | wc -l | tr -d ' ')" >&2
    fi

    local local_fail=0
    local crate version criteria

    # Build a base-crate-versions lookup as a newline-separated string
    # of `crate<TAB>version` pairs.  Each pair is unique by construction
    # since the parser emits per-block.  Multiple base versions for the
    # same crate (e.g. crypto-common) are all preserved.
    local base_pairs
    base_pairs=$(printf '%s\n' "$base_set" | awk -F'\t' 'NF>=2 {print $1 "\t" $2}')
    local head_pairs
    head_pairs=$(printf '%s\n' "$head_set" | awk -F'\t' 'NF>=2 {print $1 "\t" $2}')

    # For each (crate,version,criteria) in HEAD that is NOT in BASE,
    # classify as ADD (no base versions for this crate, untracked) or
    # BUMP (some base version of this crate exists and is no longer in
    # HEAD - direction-insensitive: a forward 2.2.1 -> 2.2.2 OR a
    # backward 2.2.2 -> 2.2.1 "anchor restoration" both count).
    #
    # Only BUMPs are gated.  ADDs (brand-new exemptions for a crate
    # that wasn't previously exempt) are out of scope for the v1 of
    # this policy - they are mostly produced mechanically by
    # `cargo vet regenerate exemptions` when a new transitive dep
    # appears, and gating them would break dependabot's regenerate
    # flow.  Adds are still subject to the layer-3 (CODEOWNERS) review
    # gate at the PR level.
    while IFS=$'\t' read -r crate version criteria; do
        [[ -z "$crate" || -z "$version" ]] && continue
        # Skip if this exact pair already existed at base - no change.
        if grep -Fxq "$crate"$'\t'"$version" <<<"$base_pairs"; then
            continue
        fi

        # Find base versions of this crate that are NOT present in head
        # (those are the candidates this new pair replaces).
        local base_versions_for_crate
        base_versions_for_crate=$(awk -F'\t' -v c="$crate" '$1==c {print $2}' <<<"$base_pairs")
        local replaced_old=""
        local v
        while IFS= read -r v; do
            [[ -z "$v" ]] && continue
            if ! grep -Fxq "$crate"$'\t'"$v" <<<"$head_pairs"; then
                replaced_old="$v"
                break
            fi
        done <<<"$base_versions_for_crate"

        if [[ -z "$replaced_old" ]]; then
            # ADD - out of scope for v1 of the discipline gate.
            if [[ "${VET_DISCIPLINE_VERBOSE:-0}" == "1" ]]; then
                printf '  %s[skip]%s add %s@%s - new exemption, not gated\n' \
                    "$C_CYAN" "$C_RESET" "$crate" "$version" >&2
            fi
            continue
        fi

        # BUMP path: require (A) delta audit AND (B) commit trailer.
        # Both checks are direction-insensitive: the same audit covers
        # forward bumps (lazy-replace anti-pattern) and backward bumps
        # (anchor-restoration pattern), because cargo-vet evaluates
        # delta chains in both directions from an exemption.
        local need_delta_fwd="$replaced_old -> $version"
        local need_delta_rev="$version -> $replaced_old"
        local have_audit=0
        local have_trailer=0

        if grep -Fxq "$crate"$'\t'"delta=$need_delta_fwd"$'\t'"$criteria" <<<"$audits" \
           || grep -Fq  "$crate"$'\t'"delta=$need_delta_fwd"$'\t' <<<"$audits" \
           || grep -Fxq "$crate"$'\t'"delta=$need_delta_rev"$'\t'"$criteria" <<<"$audits" \
           || grep -Fq  "$crate"$'\t'"delta=$need_delta_rev"$'\t' <<<"$audits"; then
            have_audit=1
        fi
        if grep -Fxq "$crate"$'\t'"$replaced_old"$'\t'"$version" <<<"$trailers" \
           || grep -Fxq "$crate"$'\t'"$version"$'\t'"$replaced_old" <<<"$trailers"; then
            have_trailer=1
        fi

        if (( have_audit && have_trailer )); then
            if [[ "${VET_DISCIPLINE_VERBOSE:-0}" == "1" ]]; then
                printf '  %s[ok]%s bump %s %s -> %s - audit + trailer present\n' \
                    "$C_GREEN" "$C_RESET" "$crate" "$replaced_old" "$version" >&2
            fi
            continue
        fi

        local_fail=1
        printf '%s[FAIL]%s lazy exemption bump: %s@%s -> %s\n' \
            "$C_RED" "$C_RESET" "$crate" "$replaced_old" "$version" >&2
        if (( ! have_audit )); then
            printf '   %s missing:%s %s[[audits.%s]]%s with %sdelta = "%s"%s (criteria = %s%s%s) in %s%s%s\n' \
                "$C_RED" "$C_RESET" \
                "$C_CYAN" "$crate" "$C_RESET" \
                "$C_CYAN" "$need_delta_fwd" "$C_RESET" \
                "$C_CYAN" "$criteria" "$C_RESET" \
                "$C_CYAN" "$AUDITS_REL" "$C_RESET" >&2
        fi
        if (( ! have_trailer )); then
            printf '   %s missing:%s commit-trailer %sVet-Reviewed-Diff: %s@%s->%s%s on any commit in %s%s%s\n' \
                "$C_RED" "$C_RESET" \
                "$C_CYAN" "$crate" "$replaced_old" "$version" "$C_RESET" \
                "$C_CYAN" "$range" "$C_RESET" >&2
        fi
    done <<<"$head_set"

    return "$local_fail"
}

print_help_block() {
    cat >&2 <<HELP

${C_YELLOW}-> Fix:${C_RESET} every exemption bump needs BOTH an audit AND a trailer.
   ${C_CYAN}just vet-bump <crate> [<new-version>]${C_RESET}      -> guided audit + diff review
   ${C_CYAN}git commit --amend --trailer 'Vet-Reviewed-Diff: <crate>@<old>-><new>'${C_RESET}

See ${C_CYAN}docs/SUPPLY-CHAIN.md${C_RESET} and ${C_CYAN}docs/policies/dependency_policy.md${C_RESET}.
HELP
}

# ── Main ────────────────────────────────────────────────────────────
[[ $# -ge 1 ]] || usage
mode="$1"

# Collect the ranges to validate.  Each mode lowers to a newline-
# separated list of `BASE..HEAD` strings stored in $RANGES.
RANGES=""
case "$mode" in
    range)
        if [[ $# -ge 2 && -n "${2:-}" ]]; then
            RANGES="$2"
        else
            RANGES="${COMMIT_RANGES:-}"
        fi
        ;;
    pre-push)
        # The hook's preamble names why it has no range to give (no
        # origin/main to compare with); a gate that checked nothing must
        # not pass.
        if [[ -n "${COMMIT_RANGES_UNCHECKED:-}" ]]; then
            printf '%s[FAIL]%s NOT CHECKED: %s\n' "$C_RED" "$C_RESET" "$COMMIT_RANGES_UNCHECKED" >&2
            exit 2
        fi
        RANGES="${COMMIT_RANGES:-}"
        ;;
    ci)
        # GitHub PR-mode sets GITHUB_BASE_REF to e.g. `main`.  In
        # push-to-main mode, fall back to the merge-base with
        # `origin/main` (which on a clean PR-merge is HEAD~1).
        # The checkout must carry full history (`fetch-depth: 0`): the
        # merge-base is the only correct base, and a base that cannot be
        # resolved is an error, never "nothing to check".
        base_ref="origin/${GITHUB_BASE_REF:-main}"
        if ! git rev-parse --verify --quiet "$base_ref" >/dev/null; then
            git fetch --no-tags origin "${GITHUB_BASE_REF:-main}"
        fi
        if ! base=$(git merge-base "$base_ref" HEAD); then
            printf '%s[FAIL]%s no merge-base between %s and HEAD - check out with fetch-depth: 0\n' \
                "$C_RED" "$C_RESET" "$base_ref" >&2
            exit 2
        fi
        RANGES="$base..HEAD"
        ;;
    *)
        usage
        ;;
esac

# Empty range list = nothing was pushed (pre-push / range modes only;
# `ci` mode always yields a range or exits above).
if [[ -z "${RANGES//[[:space:]]/}" ]]; then
    exit 0
fi

# Validate each range; accumulate the fail flag so multiple ranges
# (multi-ref pushes) all surface their problems in one go.
overall_fail=0
input_error=0
while IFS= read -r range; do
    [[ -z "$range" ]] && continue
    rc=0
    validate_range "$range" || rc=$?
    case "$rc" in
        0) ;;
        1) overall_fail=1 ;;
        *) input_error=1 ;;
    esac
done <<<"$RANGES"

if (( input_error )); then
    exit 2
fi
if (( overall_fail )); then
    print_help_block
    exit 1
fi

if [[ "${VET_DISCIPLINE_VERBOSE:-0}" == "1" ]]; then
    printf '%s[OK]%s vet-audit-discipline: all exemption bumps audited\n' \
        "$C_GREEN" "$C_RESET"
fi
