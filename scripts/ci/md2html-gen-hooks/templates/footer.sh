
# ── Wait on Bucket 1 ───────────────────────────────────────────────────
BG_FAILED=()
for i in "${!BG_PIDS[@]}"; do
    if ! wait "${BG_PIDS[$i]}"; then
        BG_FAILED+=("${BG_NAMES[$i]}")
    fi
done

# ── Report Bucket 1 ────────────────────────────────────────────────────
for i in "${!BG_NAMES[@]}"; do
    name="${BG_NAMES[$i]}"
    failed=0
    for f in "${BG_FAILED[@]+"${BG_FAILED[@]}"}"; do
        [[ "$f" == "$name" ]] && { failed=1; break; }
    done
    if (( failed )); then
        printf '  %s❌%s [1] %s\n' "$C_RED" "$C_RESET" "$name"
    else
        printf '  %s✅%s [1] %s\n' "$C_GREEN" "$C_RESET" "$name"
    fi
done

# ── Report Bucket 2 ────────────────────────────────────────────────────
for r in "${SEQ_RESULTS[@]+"${SEQ_RESULTS[@]}"}"; do
    IFS=':' read -r name status dt <<< "$r"
    case "$status" in
        ok)   printf '  %s✅%s [2] %s (%ss)\n' "$C_GREEN"  "$C_RESET" "$name" "${dt:-0}" ;;
        fail) printf '  %s❌%s [2] %s (%ss)\n' "$C_RED"    "$C_RESET" "$name" "${dt:-0}" ;;
        skip) printf '  %s⏭ %s [2] %s (skipped after fail-fast)\n' "$C_YELLOW" "$C_RESET" "$name" ;;
    esac
done

# If we ran Bucket 2 at all but nothing fired (pure docs), say so.
if (( ! CODE_CHANGED )); then
    printf '  %sℹ%s  Bucket 2 skipped - no rust/dep/infra files changed\n' "$C_CYAN" "$C_RESET"
fi

# Aggregate failure list for final dump.
FAILED=("${BG_FAILED[@]+"${BG_FAILED[@]}"}")
[[ -n "$SEQ_FIRST_FAIL" ]] && FAILED+=("$SEQ_FIRST_FAIL")

# ── Required tools ─────────────────────────────────────────────────────
# Every gate is required.  A gate whose tool is missing failed above
# with "command not found"; one that did not run on this push will fail
# on the push that runs it.  REQUIRED_TOOLS is generated from the gates.
missing=()
for tool in "${REQUIRED_TOOLS[@]+"${REQUIRED_TOOLS[@]}"}"; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if (( ${#missing[@]} > 0 )); then
    # NOTE: no backticks around `just install-dev-tools` - the cyan
    # ANSI codes already emphasise the command, and literal backticks
    # inside a single-quoted printf format string trip shellcheck
    # SC2016 ("expressions don't expand in single quotes") even
    # though they are harmless literal bytes in this context.
    printf '  %s❌%s required tools missing: %s - run %sjust install-dev-tools%s\n' \
        "$C_RED" "$C_RESET" "${missing[*]}" "$C_CYAN" "$C_RESET"
fi

# ── Dump failing output ────────────────────────────────────────────────
if (( ${#FAILED[@]} > 0 )); then
    for name in "${FAILED[@]}"; do
        printf '\n%s==== %s output ====%s\n' "$C_RED" "$name" "$C_RESET"
        cat "$TMP/$name.out"
    done
    DUR=$(( $(date +%s) - START ))
    printf '\n%s❌ lint-pre-push FAILED (%ss) - push aborted%s\n' "$C_RED" "$DUR" "$C_RESET" >&2
    # Same SC2016 avoidance as the install-dev-tools hint above.
    printf '%s   Fix the gate and push again; just install-dev-tools for a missing tool%s\n' "$C_YELLOW" "$C_RESET" >&2
    exit 1
fi

DUR=$(( $(date +%s) - START ))
printf '%s✅ lint-pre-push passed (%ss)%s\n' "$C_GREEN" "$DUR" "$C_RESET"
