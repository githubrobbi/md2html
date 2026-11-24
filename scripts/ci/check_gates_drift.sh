#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# MD2HTML gate-manifest drift detector.
#
# Verifies that every gate listed in `scripts/ci/gates.toml` is present
# in the corresponding consumer file, and that every gate present in
# the consumers has a matching manifest entry.
#
# Called by:
#   - pre-push hook (Bucket 1)
#   - `pr-fast.yml::gates-drift` job (PR-time hard gate)
#   - `just gates-drift` (manual invocation)
#
# Exit codes:
#   0  manifest and consumers agree on the gate set
#   1  drift detected - one or more gates missing on either side
#   2  manifest schema error (unparseable, missing required fields) or a
#      consumer file that cannot be read
#
# There is no bypass: drift is a "fix-me-now" signal on every tier.

set -euo pipefail

# ── Paths ──────────────────────────────────────────────────────────────
if ! REPO_ROOT="$(git rev-parse --show-toplevel)"; then
    printf 'gates-drift: run inside the md2html git checkout\n' >&2
    exit 2
fi
MANIFEST="$REPO_ROOT/scripts/ci/gates.toml"
HOOK_FAST="$REPO_ROOT/scripts/hooks/_lint_fast.sh"
HOOK_PUSH="$REPO_ROOT/scripts/hooks/_lint_pre_push.sh"
WORKFLOW="$REPO_ROOT/.github/workflows/pr-fast.yml"

# ── Colours ────────────────────────────────────────────────────────────
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
    C_BLUE=$'\033[0;34m'
    C_CYAN=$'\033[0;36m'
    C_GREEN=$'\033[0;32m'
    C_YELLOW=$'\033[1;33m'
    C_RED=$'\033[0;31m'
    C_RESET=$'\033[0m'
else
    C_BLUE=''
    C_CYAN=''
    C_GREEN=''
    C_YELLOW=''
    C_RED=''
    C_RESET=''
fi

printf '%s🔎 gates-drift - manifest vs consumers%s\n' "$C_BLUE" "$C_RESET"

# ── Sanity: required files exist ───────────────────────────────────────
for f in "$MANIFEST" "$HOOK_FAST" "$HOOK_PUSH" "$WORKFLOW"; do
    if [[ ! -f "$f" ]]; then
        printf '%s❌ missing required file: %s%s\n' "$C_RED" "$f" "$C_RESET" >&2
        exit 2
    fi
done

# ── Manifest parsing (minimal TOML) ────────────────────────────────────
# We deliberately do NOT shell out to `taplo`/`dasel` - the manifest
# uses a small, regular subset of TOML and a grep-based parser keeps
# this script self-contained.  Schema enforced by the awk below; any
# manifest that doesn't fit gets a schema error (exit 2).
#
# AWK pass: emits one line per gate of the form
#     <id>|<tiers-csv>|<gate_when>|<consumer_names>
# (`|`, not a tab: a tab is IFS whitespace, and `read` would collapse an
# empty field and shift every field after it.)
# where:
#   id       = the kebab-case gate identifier
#   tiers    = comma-separated subset of {pre-commit,pre-push,pr-fast}
#   gate_when = always|rust_changed|dep_changed|infra_changed|code_changed
# Every gate is required: the schema has no switch that lets one pass
# without running (a `hard` key is refused below).
parse_manifest() {
    awk '
        # State machine: track whether we are inside a [[gate]] table.
        /^\[\[gate\]\]$/ { in_gate=1; id=""; tiers=""; gw=""; cn=""; next }
        /^\[/             { in_gate=0; next }
        !in_gate          { next }

        /^id[ \t]*=[ \t]*"/ {
            sub(/^id[ \t]*=[ \t]*"/, ""); sub(/".*$/, ""); id=$0
        }
        /^tiers[ \t]*=[ \t]*\[/ {
            line=$0
            sub(/^tiers[ \t]*=[ \t]*\[/, "", line)
            sub(/\].*$/, "", line)
            gsub(/[ "]/, "", line)
            tiers=line
        }
        /^gate_when[ \t]*=[ \t]*"/ {
            sub(/^gate_when[ \t]*=[ \t]*"/, ""); sub(/".*$/, ""); gw=$0
        }
        /^consumer_names[ \t]*=[ \t]*\{/ {
            # Inline TOML table: { "pre-commit" = "fmt-check", ... }
            # Keep the body as-is for downstream parsing.
            line=$0
            sub(/^consumer_names[ \t]*=[ \t]*\{/, "", line)
            sub(/\}.*$/, "", line)
            gsub(/ /, "", line)
            cn=line
        }
        in_gate && id != "" {
            ids[id]=1
            if (tiers != "") tier_of[id]=tiers
            if (gw    != "") when_of[id]=gw
            if (cn    != "") cn_of[id]=cn
        }
        END {
            for (i in ids) {
                printf "%s|%s|%s|%s\n", \
                    i, \
                    (i in tier_of ? tier_of[i] : ""), \
                    (i in when_of ? when_of[i] : ""), \
                    (i in cn_of ? cn_of[i] : "")
            }
        }
    ' "$MANIFEST"
}

# Look up the per-tier consumer-side name for a gate id.  Falls back
# to the gate id itself when no override is declared.
#
# Args: <consumer_names_csv> <tier>
#       where consumer_names_csv looks like `"pre-commit"="fmt-check","pr-fast"="fmt"`
consumer_name_for() {
    local cn="$1" tier="$2" default="$3"
    if [[ -z "$cn" ]]; then
        printf '%s' "$default"
        return
    fi
    # Extract the value after `"$tier"="..."` if present.  The entries
    # are few and short, so a plain loop does it without a pipeline
    # whose "no match" exit would need swallowing.
    local match="" entry
    local -a entries
    IFS=',' read -ra entries <<< "$cn"
    for entry in "${entries[@]}"; do
        if [[ "$entry" =~ ^\"$tier\"=\"([^\"]+)\"$ ]]; then
            match="${BASH_REMATCH[1]}"
            break
        fi
    done
    if [[ -n "$match" ]]; then
        printf '%s' "$match"
    else
        printf '%s' "$default"
    fi
}

# ── Consumer scraping ──────────────────────────────────────────────────
# Each consumer exposes its gate IDs via a unique, greppable pattern:
#   _lint_fast.sh:      `spawn "<id>" ...`        (line starts with optional whitespace)
#   _lint_pre_push.sh:  `spawn_bg "<id>" ...` or `run_seq "<id>" ...`
#   pr-fast.yml:        `^  <id>:$` for top-level job blocks (2-space indent).
#
# Infrastructure jobs in pr-fast.yml are NOT gates and MUST be excluded
# from the reverse-direction check (see WORKFLOW_INFRA_JOBS below).

# Print every `<call> "<id>"` id found in a consumer file, skipping
# comment lines.  One awk pass: no grep pipeline whose "no match" exit
# status would have to be told apart from a real read error.
#   $1 = file   $2 = ERE alternation of the dispatch calls
scrape_calls() {
    awk -v calls="$2" '
        /^[[:space:]]*#/ { next }
        {
            line = $0
            re = "(" calls ")[[:space:]]+\"[a-z][a-z-]*\""
            while (match(line, re)) {
                hit = substr(line, RSTART, RLENGTH)
                sub(/^[^"]*"/, "", hit); sub(/"$/, "", hit)
                print hit
                line = substr(line, RSTART + RLENGTH)
            }
        }
    ' "$1" | sort -u
}

scrape_pre_commit() {
    # Per-match extraction so multi-spawn lines yield every id.
    scrape_calls "$HOOK_FAST" 'spawn'
}

scrape_pre_push() {
    # `spawn_bg "<id>"` / `run_seq "<id>"` anywhere on the line.
    scrape_calls "$HOOK_PUSH" 'spawn_bg|run_seq'
}

scrape_pr_fast() {
    # YAML jobs live under the `^jobs:$` header.  YAML triggers
    # (`push:`, `pull_request:`, `merge_group:`, etc.) live under
    # `on:` and would also match the 2-space-indent regex; restrict
    # to lines below `jobs:` to avoid catching them.
    awk '/^jobs:/{in_jobs=1; next} in_jobs && /^[a-z][a-zA-Z_-]+:/{exit} in_jobs && /^  [a-z][a-z-]+:$/{print}' "$WORKFLOW" \
        | sed -E 's/^  ([a-z][a-z-]+):$/\1/' \
        | sort -u
}

# Jobs in pr-fast.yml that exist for orchestration / branch protection,
# NOT for an individual gate.  These are EXEMPT from the manifest -
# they have no matching `[[gate]]` entry.  (Failure notification lives
# in the CI watch (`.github/workflows/ci-watch.yml`), so no `notify-failure`
# job is listed here.)
WORKFLOW_INFRA_JOBS=(
    "classify"
    "required"
)

# ── Job names ──────────────────────────────────────────────────────────
# A gate's pr-fast job is its id, or the `consumer_names."pr-fast"` the
# manifest gives it (`lint-ci` runs in job `clippy`); the manifest is the
# one place that says so.

# ── Diff machinery ─────────────────────────────────────────────────────
ERRORS=0

emit_missing() {
    local where="$1" id="$2" expected="$3"
    printf "  %s❌%s [%s] gate \`%s\` listed in manifest but not found in %s\n" \
        "$C_RED" "$C_RESET" "$where" "$id" "$expected" >&2
    ERRORS=$(( ERRORS + 1 ))
}

emit_orphan() {
    local where="$1" id="$2"
    printf "  %s❌%s [%s] gate \`%s\` defined in consumer but missing from manifest\n" \
        "$C_RED" "$C_RESET" "$where" "$id" >&2
    ERRORS=$(( ERRORS + 1 ))
}

# ── Parse once, validate the schema ───────────────────────────────────
# Parsing into a variable (not a process substitution) makes a parser
# failure abort the script instead of reading as an empty manifest.
# `hard` was a switch that could only let a gate pass without its tool;
# it is gone, and a manifest that still carries it is refused.
hard_status=0
hard_lines=$(grep -n '^hard[[:space:]]*=' "$MANIFEST") || hard_status=$?
case "$hard_status" in
    0)
        printf "%s❌ schema: \`hard\` is not a gate field (every gate is required); remove:%s\n%s\n" \
            "$C_RED" "$C_RESET" "$hard_lines" >&2
        exit 2
        ;;
    1) ;;
    *)
        printf '%s❌ cannot read %s (grep exit %d)%s\n' "$C_RED" "$MANIFEST" "$hard_status" "$C_RESET" >&2
        exit 2
        ;;
esac

MANIFEST_ROWS=$(parse_manifest)
if [[ -z "$MANIFEST_ROWS" ]]; then
    printf '%s❌ schema: no [[gate]] table parsed from %s%s\n' "$C_RED" "$MANIFEST" "$C_RESET" >&2
    exit 2
fi
SCHEMA_ERRORS=0
while IFS='|' read -r id tiers when _cn; do
    if [[ -z "$tiers" ]]; then
        printf "%s❌ schema: gate '%s' has no tiers%s\n" "$C_RED" "$id" "$C_RESET" >&2
        SCHEMA_ERRORS=$(( SCHEMA_ERRORS + 1 ))
    fi
    case "$when" in
        always|rust_changed|dep_changed|infra_changed|code_changed) ;;
        *)
            printf "%s❌ schema: gate '%s' has gate_when '%s' (always|rust_changed|dep_changed|infra_changed|code_changed)%s\n" \
                "$C_RED" "$id" "$when" "$C_RESET" >&2
            SCHEMA_ERRORS=$(( SCHEMA_ERRORS + 1 ))
            ;;
    esac
done <<< "$MANIFEST_ROWS"
if (( SCHEMA_ERRORS > 0 )); then
    exit 2
fi

# ── Forward check: manifest → consumers ────────────────────────────────
# For every [[gate]] in the manifest, every tier it claims must
# actually contain it.  The scraped consumer sets are O(n) so we
# build them once and look up via grep -Fxq.
PC_SET=$(scrape_pre_commit)
PP_SET=$(scrape_pre_push)
PR_SET=$(scrape_pr_fast)
for consumer in PC_SET:"$HOOK_FAST" PP_SET:"$HOOK_PUSH" PR_SET:"$WORKFLOW"; do
    var="${consumer%%:*}"
    if [[ -z "${!var}" ]]; then
        printf '%s❌ no gate found in %s - the consumer format changed or the file is broken%s\n' \
            "$C_RED" "${consumer#*:}" "$C_RESET" >&2
        exit 2
    fi
done

while IFS='|' read -r id tiers _when cn; do
    [[ -z "$id" ]] && continue
    IFS=',' read -ra TIER_LIST <<< "$tiers"
    for t in "${TIER_LIST[@]}"; do
        case "$t" in
            pre-commit)
                expected=$(consumer_name_for "$cn" "pre-commit" "$id")
                grep -Fxq "$expected" <<< "$PC_SET" \
                    || emit_missing "pre-commit" "$id" "_lint_fast.sh ('$expected')"
                ;;
            pre-push)
                expected=$(consumer_name_for "$cn" "pre-push" "$id")
                grep -Fxq "$expected" <<< "$PP_SET" \
                    || emit_missing "pre-push" "$id" "_lint_pre_push.sh ('$expected')"
                ;;
            pr-fast)
                expected_job=$(consumer_name_for "$cn" "pr-fast" "$id")
                grep -Fxq "$expected_job" <<< "$PR_SET" \
                    || emit_missing "pr-fast" "$id" "pr-fast.yml job '$expected_job'"
                ;;
            tier-2)
                # Not checked here: tier-2.yml has its own uniquely
                # shaped jobs that don't fit the gate manifest schema.
                ;;
            *)
                printf "  %s❌%s schema: gate \`%s\` has unknown tier \`%s\`\n" \
                    "$C_RED" "$C_RESET" "$id" "$t" >&2
                ERRORS=$(( ERRORS + 1 ))
                ;;
        esac
    done
done <<< "$MANIFEST_ROWS"

# ── Reverse check: consumers → manifest ────────────────────────────────
# Every gate ID found in a consumer must have a matching manifest
# entry on the corresponding tier.  Catches the "added a gate to the
# hook but forgot the manifest" failure mode.

# Build manifest-id-set per tier for reverse lookups.
declare -A MANIFEST_PC=()
declare -A MANIFEST_PP=()
declare -A MANIFEST_PR=()
while IFS='|' read -r id tiers _when cn; do
    [[ -z "$id" ]] && continue
    IFS=',' read -ra TIER_LIST <<< "$tiers"
    for t in "${TIER_LIST[@]}"; do
        case "$t" in
            pre-commit)
                MANIFEST_PC[$(consumer_name_for "$cn" "pre-commit" "$id")]=1 ;;
            pre-push)
                MANIFEST_PP[$(consumer_name_for "$cn" "pre-push" "$id")]=1 ;;
            pr-fast)
                MANIFEST_PR[$(consumer_name_for "$cn" "pr-fast" "$id")]=1 ;;
        esac
    done
done <<< "$MANIFEST_ROWS"

while IFS= read -r id; do
    [[ -z "$id" ]] && continue
    [[ -n "${MANIFEST_PC[$id]:-}" ]] || emit_orphan "pre-commit" "$id"
done <<< "$PC_SET"

while IFS= read -r id; do
    [[ -z "$id" ]] && continue
    [[ -n "${MANIFEST_PP[$id]:-}" ]] || emit_orphan "pre-push" "$id"
done <<< "$PP_SET"

while IFS= read -r id; do
    [[ -z "$id" ]] && continue
    # Skip infra jobs.
    skip=0
    for infra in "${WORKFLOW_INFRA_JOBS[@]}"; do
        [[ "$id" == "$infra" ]] && { skip=1; break; }
    done
    (( skip )) && continue
    [[ -n "${MANIFEST_PR[$id]:-}" ]] || emit_orphan "pr-fast" "$id"
done <<< "$PR_SET"

# ── Verdict ────────────────────────────────────────────────────────────
if (( ERRORS > 0 )); then
    printf '\n%s❌ gates-drift: %d mismatch(es) detected%s\n' \
        "$C_RED" "$ERRORS" "$C_RESET" >&2
    printf '   %sFix: update either %sscripts/ci/gates.toml%s%s or the\n' \
        "$C_YELLOW" "$C_CYAN" "$C_RESET" "$C_YELLOW" >&2
    printf '   %scorresponding consumer so they agree (the hooks are generated:%s\n' \
        "$C_YELLOW" "$C_RESET" >&2
    printf '   %sedit the manifest, then run %sjust md2html-gen-hooks%s%s).%s\n' \
        "$C_YELLOW" "$C_CYAN" "$C_RESET" "$C_YELLOW" "$C_RESET" >&2
    exit 1
fi

printf '%s✅ gates-drift: manifest and consumers agree (%d gates)%s\n' \
    "$C_GREEN" "$(printf '%s\n' "$MANIFEST_ROWS" | wc -l | tr -d ' ')" "$C_RESET"
