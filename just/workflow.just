# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Phase 1: safe-by-default validation.
phase1-test:
    @printf "\033[0;34m🧪 PHASE 1: Safe-by-Default Validation\033[0m\n"
    @printf "\033[1;33mNo version bump, deploy, commit, or push in this lane.\033[0m\n"
    @echo "========================================================"
    @echo ""
    @printf "\033[0;34mStep 1: Compile validation without applying fixes...\033[0m\n"
    @printf "\033[0;36m  → cargo check --workspace --all-targets --all-features\033[0m\n"
    cargo check --workspace --all-targets --all-features
    @printf "\033[0;34mStep 1b: File size policy (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → bash scripts/ci/check_file_size_policy.sh\033[0m\n"
    bash scripts/ci/check_file_size_policy.sh
    @printf "\033[0;34mStep 2: Running workspace tests (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → cargo nextest run --workspace --all-features --lib --tests\033[0m\n"
    cargo nextest run --workspace --all-features --lib --tests
    @printf "\033[0;32m✅ All workspace tests passed\033[0m\n"
    @printf "\033[0;34mStep 3: Documentation tests validation (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → just test-doc\033[0m\n"
    just test-doc
    @printf "\033[0;34mStep 3b: Rustdoc link validation (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → just rustdoc\033[0m\n"
    just rustdoc
    @printf "\033[0;34mStep 4: Ultra-strict production code linting (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → just lint-prod\033[0m\n"
    just lint-prod
    @printf "\033[0;34mStep 5: Pragmatic test code linting (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → just lint-tests\033[0m\n"
    just lint-tests
    @printf "\033[0;34mStep 6: Dependency security validation (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → cargo deny check\033[0m\n"
    cargo deny check
    @printf "\033[0;34mStep 7: CI lint gating (FAST-FAIL)...\033[0m\n"
    @printf "\033[0;36m  → just lint-ci\033[0m\n"
    just lint-ci
    @echo ""
    @printf "\033[0;32m✅ PHASE 1 COMPLETE: Validation passed with no release-side effects.\033[0m\n"
    @printf "\033[0;36m📝 Note: Linux-specific code paths (cfg target_os = linux) are validated by CI on push.\033[0m\n"
    @printf "\033[0;34m💡 Next: Run 'just phase2-ship' only when ready to commit/push\033[0m\n"

# Phase 2: commit and push (`just ship` is the full, resumable lane with the version bump).
phase2-ship:
    @printf "\033[0;34m🚀 PHASE 2: Explicit Ship Lane\033[0m\n"
    @printf "\033[1;33mThis lane performs commit and push; the version bump lives in 'just ship'.\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- phase2

# Backward compatibility alias for phase2-ship.
phase2-commit: phase2-ship

# Safe-by-default validation workflow (Rust optimized).
# Lints macOS code paths locally; Linux paths validated by CI on push.
go:
    @printf "\033[0;34m🚀 Safe-by-Default Validation Workflow (RUST OPTIMIZED)\033[0m\n"
    @printf "\033[1;33mNo version bump, deploy, commit, or push in this default lane.\033[0m\n"
    @printf "\033[0;36m📝 Lints macOS code paths - Linux cfg() paths validated by CI on push.\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- go

# Full ship pipeline: Phase 1 validation + Phase 2 deploy (resumable).
# Re-runs automatically skip already-completed steps.
# Lints macOS code paths locally; Linux paths validated by CI on push.
# Pass flags like: just ship -v, just ship --coverage-report
ship *ARGS:
    @printf "\033[0;34m🚢 Full Ship Pipeline (Phase 1 + Phase 2, Resumable)\033[0m\n"
    @printf "\033[1;33mValidates and ships. Re-runs skip completed steps.\033[0m\n"
    @printf "\033[0;36m📝 Lints macOS code paths - Linux cfg() paths validated by CI on push.\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- ship {{ ARGS }}

# Full ship pipeline from scratch (ignores previous progress).
# Pass flags like: just ship-fresh -v
ship-fresh *ARGS:
    @printf "\033[0;34m🚢 Full Ship Pipeline (Fresh Start)\033[0m\n"
    @printf "\033[1;33mIgnoring previous progress, starting from scratch.\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- ship --fresh {{ ARGS }}

# Draft CHANGELOG [Unreleased] from the Conventional-Commit subjects since the
# last release tag - a placeholder to polish before shipping. Never overwrites
# hand-authored notes (the ship flow also auto-drafts if [Unreleased] is empty).

# Draft CHANGELOG [Unreleased] from the commit subjects since the last release.
changelog-draft:
    @printf "\033[0;34m📝 Drafting CHANGELOG [Unreleased] from commits since the last release\033[0m\n"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- changelog-draft

# Safe-by-default validation workflow (justfile fallback).
go-justfile:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🚀 Safe-by-Default Validation Workflow (JUSTFILE)\033[0m\n"
    printf "\033[1;33mThis fallback validates only; ship actions stay in 'just phase2-ship'.\033[0m\n"
    echo "========================================================"
    echo ""
    START_TIME=$(date +%s)
    printf "\033[0;36m⏱️  Workflow started at: %s\033[0m\n" "$(date +%H:%M:%S)"
    echo ""
    printf "\033[0;34m🧪 PHASE 1: Comprehensive Fast-Fail Testing & Validation\033[0m\n"
    just phase1-test
    echo ""
    printf "\033[0;32m✅ PHASE 1 COMPLETE - All validation passed!\033[0m\n"
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    echo ""
    printf "\033[0;32m🎉 SAFE VALIDATION WORKFLOW FINISHED!\033[0m\n"
    printf "\033[0;32m✅ Validation completed without release-side effects\033[0m\n"
    printf "\033[0;34m💡 Run 'just phase2-ship' only when ready to ship\033[0m\n"
    printf "\033[0;36m⏱️  Total execution time: %ss\033[0m\n" "$DURATION"

# ─────────────────────────────────────────────────────────────────────
# Signed-merge flow helpers (see scripts/ci/check_commit_signatures.sh)
# ─────────────────────────────────────────────────────────────────────

# Merge a PR - the ONLY sanctioned way to merge (AGENTS.md rule 7 binds it).
#
# WHY THIS RECIPE IS THE GUARD, not GitHub alone: on a plan where
# rulesets are unavailable (private repo, free plan), NO check is
# "required" - which makes `gh pr merge --auto` merge INSTANTLY, before
# a single check has run. md2html's ruleset requires checks today
# (`scripts/ci/main-ruleset.json`), and this recipe still waits HERE,
# so a ruleset that is ever off cannot turn a merge into an unchecked one:
#
#   1. `gh pr checks --watch --fail-fast` blocks until every check on
#      the PR's CURRENT head completes, failing on the first red one.
#   2. The head SHA is re-read after the watch: a push that landed
#      mid-watch restarts the wait instead of merging unverified code.
#   3. ABSENCE is not success: the `PR Fast CI / required` aggregate
#      must be PRESENT and green (a dropped Actions trigger shows a
#      green page with the aggregate simply missing).
#   4. Only then does `--auto` run - which merges immediately when the
#      checks are already green, and the day the merge queue exists
#      enrolls into it (the queue-refusal fallback below), so this
#      recipe stays correct on both plans.
#
# There is deliberately NO bypass flag on this recipe. A PR lands as a
# MERGE COMMIT, its signed commits kept one by one: the history is the
# record. Squash collapses every commit message into one, and a rebase
# rewrites (and unsigns) the signed commits, so both are refused here;
# the repo settings (`bootstrap-github.sh`) and the ruleset
# (`main-ruleset.json`) allow merge commits only, to match. The commits
# are signed locally (`just setup-signing`), so the signed-commit rule
# needs no squash.

# Merge a PR: watches EVERY check first, then merges (the only sanctioned way).
merge PR METHOD="merge":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{ METHOD }}" != "merge" ]]; then
        printf "\033[0;31m❌ '{{ METHOD }}' refused: merge commits only (never squash, never rebase).\033[0m\n" >&2
        exit 1
    fi
    for _attempt in 1 2 3; do
        head_before=$(gh pr view {{ PR }} --json headRefOid -q .headRefOid)
        printf "\033[0;34m⏳ PR #{{ PR }}: waiting for ALL checks on %s...\033[0m\n" "${head_before:0:10}"
        # Right after a push GitHub has registered no check yet, and
        # `gh pr checks --watch` then fails with "no checks reported":
        # wait for the checks to appear (five minutes at most) before
        # watching them, so that is not mistaken for a failed check.
        registered=0
        for _wait in $(seq 60); do
            if count=$(gh pr checks {{ PR }} --json name --jq length 2>/dev/null) && [[ "$count" -gt 0 ]]; then
                registered=1
                break
            fi
            sleep 5
        done
        if [[ "$registered" -ne 1 ]]; then
            printf "\033[0;31m❌ no check was reported for PR #{{ PR }} within five minutes - look at the Actions tab.\033[0m\n" >&2
            exit 1
        fi
        if ! gh pr checks {{ PR }} --watch --fail-fast; then
            printf "\033[0;31m❌ a check FAILED - fix it and re-run; this recipe never merges red.\033[0m\n" >&2
            exit 1
        fi
        head_after=$(gh pr view {{ PR }} --json headRefOid -q .headRefOid)
        if [[ "$head_after" == "$head_before" ]]; then
            agg=$(gh pr checks {{ PR }} --json name,state \
                --jq '[.[] | select(.name == "PR Fast CI / required")] | first | .state // "MISSING"')
            if [[ "$agg" != "SUCCESS" ]]; then
                printf "\033[0;31m❌ the 'PR Fast CI / required' aggregate is %s on this head - not merging (the main guard would revert). Wait for PR Fast CI to run, or re-push to retrigger it.\033[0m\n" "$agg" >&2
                exit 1
            fi
            printf "\033[0;32m✅ all checks green on %s (required aggregate SUCCESS) - merging.\033[0m\n" "${head_after:0:10}"
            # Retries with the error VISIBLE: right after a watch
            # completes, GitHub's mergeability can lag a few seconds. The
            # bare --auto fallback fires ONLY on the merge-queue refusal
            # it exists for ("the merge strategy for main is set by the
            # merge queue").
            for _try in 1 2 3; do
                if output=$(gh pr merge {{ PR }} --auto --{{ METHOD }} 2>&1); then
                    exit 0
                fi
                printf "\033[0;33m⚠️  merge attempt failed:\033[0m %s\n" "$output" >&2
                if [[ "$output" == *"merge strategy"*"merge queue"* ]]; then
                    gh pr merge {{ PR }} --auto
                    exit 0
                fi
                sleep 5
            done
            printf "\033[0;31m❌ merge refused three times - the error above is the real one.\033[0m\n" >&2
            exit 1
        fi
        printf "\033[0;33m⚠️  head moved during the watch (%s -> %s) - re-watching the new head.\033[0m\n" \
            "${head_before:0:10}" "${head_after:0:10}"
    done
    printf "\033[0;31m❌ head kept moving across 3 watches - merge it when the branch settles.\033[0m\n" >&2
    exit 1

# Rehearse the whole PR tier locally before pushing a branch for a PR:
# the phase-1 validation (check, file-size, the full test suite, doc
# tests, rustdoc, the clippy trio, deny), the cross-target passes (every
# leg runs or the recipe fails), the workflow drift and consistency
# check, the anti-pattern gate, `md2html --check` over every document CI
# checks, and the conventional subjects of every commit the PR would
# carry. Minutes locally instead of paid minutes on a red PR.

# Rehearse the whole PR tier locally before pushing a branch for a PR.
pre-pr:
    just phase1-test
    just check-all-targets
    cargo run -q --release -p md2html-gen-workflow -- --check
    just audit-gate
    cargo build -q --release -p md2html
    ./target/release/md2html --check README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md
    bash scripts/ci/check_commit_subjects.sh range origin/main..HEAD
    @printf "\033[0;32m✅ pre-pr: the branch is ready for 'just pr'\033[0m\n"

# Open the PR for the current branch with a title the server-side title
# check will accept: the title is validated locally against the same
# conventional-commit regex first, so a rejected title never costs a CI
# run. The body lists the branch's commits.

# Open the PR for this branch with a locally validated conventional title.
pr TITLE:
    #!/usr/bin/env bash
    set -euo pipefail
    title_file="$(mktemp)"
    printf '%s\n' "{{ TITLE }}" > "$title_file"
    bash scripts/ci/check_commit_subjects.sh file "$title_file"
    rm -f "$title_file"
    body="$(git log --reverse --format='- %s' origin/main..HEAD)"
    gh pr create --base main --title "{{ TITLE }}" --body "$body"

# Apply the main-branch ruleset on GitHub from scripts/ci/main-ruleset.json
# (no deletion, no force push, signatures, merge commits only, a PR with every
# thread resolved, and the required checks: the PR Fast CI aggregate, the
# PR title, and md2html's own tests on three systems, its oldest supported
# Rust, the supply chain and the browser suite). No merge queue: GitHub
# offers one only to repositories an organization owns. The pre-push hook
# is the local mirror and main-merge-guard.yml the server-side backstop.
# `just protect-main --without-signatures` defers the signature rule
# until every committer's `just doctor-signing` is green.

# Apply the main-branch ruleset on GitHub from scripts/ci/main-ruleset.json.
protect-main *FLAGS:
    bash scripts/ci/apply_main_ruleset.sh {{ FLAGS }}

# Self-heal: re-sign every commit on the current branch, then re-push.
#
# `main`'s ruleset requires every commit to carry a verified signature.
# If one slipped through unsigned (e.g. committed with
# `-c commit.gpgsign=false`), the `commit-signatures` pre-push gate blocks
# the push.  This rebases the branch in place, re-signing each commit with
# your configured GPG key, then force-pushes with lease.  `--amend` leaves
# the staged set empty so the per-commit pre-commit hook no-ops; the heavy
# gates run once on the final push.

# Re-sign every commit on this branch, then force-push with lease.
sign-branch:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [[ "$branch" == "main" ]]; then
        printf "\033[0;31m❌ refusing to rewrite main - run this on a feature branch.\033[0m\n" >&2
        exit 1
    fi
    base=$(git merge-base origin/main HEAD)
    n=$(git rev-list --count "$base"..HEAD)
    printf "\033[0;34m🔏 Re-signing %s commit(s) on '%s' (base %s)...\033[0m\n" "$n" "$branch" "${base:0:10}"
    git rebase --exec 'git commit --amend --no-edit -S' "$base"
    printf "\033[0;34m⬆️  Force-pushing (with lease)...\033[0m\n"
    git push --force-with-lease
    printf "\033[0;32m✅ Branch re-signed and pushed - every commit now carries a signature.\033[0m\n"

# One-command commit-signing setup (SSH format, the low-friction path).
#
# SCOPE: everything this writes is LOCAL to this repository. Your global
# git config and every other repository on this machine stay untouched;
# run it once per repo that wants signing. The only shared artifacts are
# inherently user-level: the SSH key file itself and its GitHub
# registration.
#
# Does everything that CAN be automated: creates an ed25519 key if you have
# none (you choose the passphrase), points THIS repo's git config at it
# (SSH signing format, sign commits + tags), registers the key on GitHub
# as a SIGNING key via the API, and finishes with the doctor. Idempotent;
# safe to re-run; never overwrites an existing key. GPG users: configure
# manually and run `just doctor-signing` instead.
#
# Fleet / unattended machines: this recipe never GENERATES a key without a
# human (key issuance is an identity decision - pre-provision
# ~/.ssh/id_ed25519 via your fleet tooling instead; with a key already in
# place, this recipe is fully non-interactive: config + registration + doctor).

# One-command SSH commit-signing setup for this repository.
setup-signing:
    #!/usr/bin/env bash
    set -euo pipefail
    KEYFILE="$HOME/.ssh/id_ed25519"
    printf "\033[0;34m🔏 Commit-signing setup (SSH format)\033[0m\n"
    # 0. NEVER clobber an existing setup. Working signing (any format):
    #    nothing to do. Configured-but-broken GPG: that is the user's
    #    setup to repair (or consciously replace) - refuse to silently
    #    rewrite their global git config to SSH.
    if just doctor-signing >/dev/null 2>&1; then
        printf "  \033[0;32m✅ commit signing already configured and working - nothing to change\033[0m\n"
        exit 0
    fi
    # A broken setup in THIS repo's local config is yours to resolve; we
    # refuse to overwrite it. A broken GLOBAL config is different: we never
    # touch global anyway, and a repo-local override is exactly the fix.
    # `git config --get` exits 1 for an unset key (an answer, not an
    # error) and higher for a broken config (an error).
    cfg() {
        local rc=0 value
        value="$(git config "$@")" || rc=$?
        if (( rc > 1 )); then
            printf "\033[0;31m❌ git config %s failed (exit %d)\033[0m\n" "$*" "$rc" >&2
            exit 1
        fi
        printf '%s' "$value"
    }
    local_key=$(cfg --local --get user.signingkey)
    local_format=$(cfg --local --get gpg.format)
    local_format="${local_format:-openpgp}"
    if [[ -n "$local_key" && "$local_format" != "ssh" ]]; then
        printf "  \033[1;33m⚠  this repo has a LOCAL GPG signing setup (key %s) that is not working.\033[0m\n" "$local_key"
        printf "  \033[1;33m   Not overwriting it. Repair GPG ('just doctor-signing' shows what is\033[0m\n"
        printf "  \033[1;33m   broken) or remove the local override first:\033[0m\n"
        printf "  \033[1;33m     git config --unset user.signingkey && just setup-signing\033[0m\n"
        exit 1
    fi
    global_key=$(cfg --global --get user.signingkey)
    if [[ -n "$global_key" ]]; then
        printf "  \033[0;36mℹ️  your GLOBAL git config has a signing setup (key %s) that is not\033[0m\n" "$global_key"
        printf "  \033[0;36m   working here. Leaving it alone; configuring THIS repo locally with\033[0m\n"
        printf "  \033[0;36m   SSH signing instead (local config overrides global for this repo).\033[0m\n"
    fi
    # 1. Key material (interactive: you pick the passphrase)
    if [[ -f "$KEYFILE" ]]; then
        printf "  \033[0;32m✅ SSH key exists: %s\033[0m\n" "$KEYFILE"
    else
        printf "  \033[0;36m→ No SSH key found - generating one (choose a passphrase when asked)\033[0m\n"
        ssh-keygen -t ed25519 -f "$KEYFILE"
    fi
    # 2. Point git at it (global: every repo on this machine signs)
    git config gpg.format ssh
    git config user.signingkey "$KEYFILE.pub"
    git config commit.gpgsign true
    git config tag.gpgsign true
    printf "  \033[0;32m✅ THIS repo signs all commits + tags with %s.pub\033[0m\n" "$KEYFILE"
    printf "  \033[0;36m   (repo-local config; your other repositories are untouched)\033[0m\n"
    # 3. Register on GitHub as a SIGNING key (separate registration from the
    #    authentication use of the same key). Already registered is fine;
    #    any other refusal (most often a token without the
    #    admin:ssh_signing_key scope) stops here: an unregistered key signs
    #    commits GitHub shows as Unverified, which the ruleset refuses.
    if gh_out="$(gh ssh-key add "$KEYFILE.pub" --type signing --title "$(hostname) signing" 2>&1)"; then
        printf "  \033[0;32m✅ key registered on GitHub as a signing key\033[0m\n"
    elif [[ "$gh_out" == *"already"* ]]; then
        printf "  \033[0;32m✅ key already registered on GitHub as a signing key\033[0m\n"
    else
        printf '%s\n' "$gh_out" >&2
        printf "  \033[0;31m❌ GitHub refused the signing key. If the token lacks the scope, run:\033[0m\n"
        printf "  \033[0;36m   gh auth refresh -h github.com -s admin:ssh_signing_key   then re-run this recipe\033[0m\n"
        exit 1
    fi
    # 4. Prove it
    just doctor-signing

# Verify commit signing is set up so commits can land on main.
#
# Supports BOTH signing formats: SSH (`gpg.format = ssh`, the low-friction
# modern path) and OpenPGP (classic gpg). Checks, in order: commit.gpgsign
# is on; a signing key is configured; the signer can actually produce a
# signature with it; and the key is registered on GitHub (so commits render
# as "verified").  Every check runs and reports with its fix; the recipe
# exits non-zero if any fails.  A check gh cannot answer (a token
# without the scope to list keys or emails) is reported as NOT CHECKED
# with the command that grants the scope.

# Check that commits will be signed and verified on GitHub.
doctor-signing:
    #!/usr/bin/env bash
    set -uo pipefail
    fail=0
    ok()   { printf "  \033[0;32m✅ %s\033[0m\n" "$1"; }
    bad()  { printf "  \033[0;31m❌ %s\033[0m\n" "$1"; fail=1; }
    # `git config --get` exits 1 for an unset key (an answer) and higher
    # for a broken config (an error that stops the doctor).
    cfg() {
        local rc=0 value
        value="$(git config --get "$1")" || rc=$?
        if (( rc > 1 )); then
            printf "  \033[0;31m❌ git config --get %s failed (exit %d)\033[0m\n" "$1" "$rc" >&2
            exit 1
        fi
        printf '%s' "$value"
    }
    printf "\033[0;34m🩺 Commit-signing doctor\033[0m\n"

    if [[ "$(cfg commit.gpgsign)" == "true" ]]; then
        ok "commit.gpgsign = true"
    else
        bad "commit.gpgsign is not true - run: git config commit.gpgsign true"
    fi

    key=$(cfg user.signingkey)
    if [[ -n "$key" ]]; then
        ok "user.signingkey = $key"
    else
        bad "no user.signingkey - set it: git config user.signingkey <KEYID>"
    fi

    format=$(cfg gpg.format)
    format="${format:-openpgp}"
    if [[ "$format" == "ssh" ]]; then
        keyfile="${key/#\~/$HOME}"
        if [[ -f "$keyfile" || -f "${keyfile%.pub}" ]]; then
            sigkey="$keyfile"; [[ -f "${keyfile%.pub}" ]] && sigkey="${keyfile%.pub}"
            if printf 'test' | ssh-keygen -Y sign -n git -f "$sigkey" >/dev/null 2>&1; then
                ok "SSH signing works with the configured key"
            else
                bad "ssh-keygen cannot sign with '$key' - is the private key present and unlocked?"
            fi
        else
            bad "gpg.format=ssh but user.signingkey does not point at a key file: $key"
        fi
    elif printf 'test' | gpg ${key:+--local-user "$key"} --clearsign >/dev/null 2>&1; then
        ok "GPG can sign with the configured key"
    else
        bad "GPG cannot sign (broken agent / locked key) - check 'gpg --list-secret-keys'"
    fi

    # GitHub registration: an unregistered key signs commits GitHub shows
    # as Unverified, which the ruleset refuses.  Listing keys needs a token
    # scope the default `gh` login lacks; without it the check is reported
    # as NOT CHECKED, with the command that grants the scope.
    notchecked() { printf "  \033[1;33m⚠  NOT CHECKED: %s\033[0m\n" "$1"; }
    if [[ -n "$key" && "$format" == "ssh" ]]; then
        pubfile="${keyfile%.pub}.pub"
        if [[ ! -f "$pubfile" ]]; then
            bad "no public key at $pubfile to compare with GitHub's signing keys"
        elif keys=$(gh api user/ssh_signing_keys --jq '.[].key' 2>/dev/null); then
            pub="$(cut -d' ' -f1-2 "$pubfile")"
            if grep -qxF "$pub" <<< "$keys"; then
                ok "SSH signing key is registered on GitHub (commits will verify)"
            else
                bad "the SSH key is not registered on GitHub as a SIGNING key - run: just setup-signing"
            fi
        else
            notchecked "GitHub SSH signing keys (gh token lacks read:ssh_signing_key) - run: gh auth refresh -h github.com -s read:ssh_signing_key"
        fi
    elif [[ -n "$key" ]]; then
        # Branch on gh's exit status, not its output: on a 404 (missing
        # scope) gh prints error JSON that would otherwise leak into the
        # match and read as a false "key not found".
        if keys=$(gh api user/gpg_keys --jq '.[].key_id, .[].subkeys[]?.key_id' 2>/dev/null); then
            if grep -qiF "${key: -16}" <<< "$keys"; then
                ok "signing key is registered on GitHub (commits will verify)"
            else
                bad "the configured key is not among your GitHub GPG keys - add it: https://github.com/settings/keys"
            fi
        else
            notchecked "GitHub GPG keys (gh token lacks read:gpg_key) - run: gh auth refresh -h github.com -s read:gpg_key"
        fi
    fi

    # Identity check: a key that signs fine and IS registered on GitHub
    # still renders "Unverified" (reason: no_user) if the commit's
    # user.email isn't a verified email on the SAME GitHub account the key
    # is registered to - GitHub can't attribute the commit to anyone. This
    # is a silent failure mode: every check above passes, the signature is
    # cryptographically valid, and the commit still shows Unverified.
    email=$(cfg user.email)
    if [[ -n "$email" ]]; then
        if gh_emails=$(gh api user/emails --jq '.[].email' 2>/dev/null); then
            if grep -qxF "$email" <<< "$gh_emails"; then
                ok "user.email ($email) matches a verified GitHub email"
            else
                bad "user.email ($email) is not a verified email on your GitHub account - the signature will be valid but the commit shows 'Unverified' (GitHub can't attribute it to anyone). Fix: git config user.email <a verified address> (see https://github.com/settings/emails), matching whichever account this signing key is registered to"
            fi
        else
            notchecked "GitHub account emails (gh token lacks user:email) - run: gh auth refresh -h github.com -s user:email (or check '$email' at https://github.com/settings/emails)"
        fi
    else
        bad "no user.email configured - git config user.email <you>@users.noreply.github.com"
    fi

    if (( fail )); then
        printf "\033[0;31m❌ signing setup incomplete - commits may be rejected by main's ruleset.\033[0m\n" >&2
        exit 1
    fi
    printf "\033[0;32m✅ local signing is set up - commits will be signed.\033[0m\n"

# Comprehensive validation (Rust optimized).
check-all:
    @printf "\033[0;34m📋 Comprehensive Validation (RUST OPTIMIZED)\033[0m\n"
    @printf "\033[1;33mUsing high-performance Rust script with parallel execution...\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- check-all

# Generate coverage report (Rust optimized).
coverage-report:
    @printf "\033[0;34m📊 Coverage Report Generation (RUST OPTIMIZED)\033[0m\n"
    @printf "\033[1;33mSmart detection: generates from existing data or runs tests if needed...\033[0m\n"
    @echo "========================================================"
    cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- coverage-report

# Watch the latest release workflow run (lane:release).
release-watch:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m👀 Watching latest release workflow...\033[0m\n"
    run_id="$(gh run list --workflow=release.yml --limit=1 --json databaseId --jq '.[0].databaseId // empty')"
    if [[ -z "$run_id" ]]; then
        printf "\033[0;31m❌ release.yml has never run here (lane:release is off until LANE_RELEASE=true)\033[0m\n"
        exit 1
    fi
    gh run watch "$run_id"

# Open release workflow in browser.
release-status:
    @printf "\033[0;34m🔗 Opening release workflow in browser...\033[0m\n"
    @gh run list --workflow=release.yml --limit=1 --web

# Check current workflow status.
workflow-status:
    @printf "\033[0;34m📊 Checking workflow status...\033[0m\n"
    @cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- workflow-status

# Reset workflow state.
workflow-reset:
    @printf "\033[0;33m⚠️  Resetting workflow state...\033[0m\n"
    @cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- workflow-reset

# Resume incomplete workflow.
workflow-resume:
    @printf "\033[0;34m🔄 Resuming workflow...\033[0m\n"
    @cargo run -q --release --target-dir target/ci-bootstrap -p md2html-ci-pipeline -- workflow-resume

