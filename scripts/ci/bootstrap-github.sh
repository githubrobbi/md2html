#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# GitHub-side bootstrap for md2html (adopted from rust-forge-template).
#
# A repository copies FILES only. This script (re)creates the server-side
# state the machinery expects:
#   * the label `ci-failure` the CI watch files under (scripts/ci/ci_watch.sh;
#     the per-workflow labels are made by the first issue that uses each)
#   * the dormant-lane repo variables (all "false" - flip via docs/forge/COMPONENTS.md)
#   * branch ruleset for main (required PR + required status checks +
#     signatures; pr-fast.yml subscribes to merge_group too, for the day
#     the repository moves to an organization and gains a merge queue)
#   * merge-commit-only + auto-merge repo settings
#
# Requires: gh CLI authenticated with admin on the repo.
# Idempotent: safe to re-run.  Every step is required: a step GitHub
# refuses stops the script with GitHub's own error, so a partial
# bootstrap never reads as a complete one.

set -euo pipefail

REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
echo "🔧 Bootstrapping GitHub-side state for ${REPO}"

# ── 1. Labels (tiered CI-failure triage) ─────────────────────────────
echo "── labels"
for spec in \
  "ci-failure|D93F0B|A failure the CI watch filed (scripts/ci/ci_watch.sh)" \
  "ci-failure-pr-fast|D93F0B|PR Fast CI failure outside a pull request" \
  "ci-failure-tier-2|E99695|Tier-2 (nightly suite) failure" \
  "ci-failure-release|B60205|Release pipeline failure" \
  "preview-artifacts|0E8A16|Build preview artifacts for this PR (lane:preview)" \
  "preview-macos|0E8A16|Also build the macOS preview artifacts (lane:preview)"; do
  IFS='|' read -r name color desc <<<"${spec}"
  gh label create "${name}" --color "${color}" --description "${desc}" --force >/dev/null
  echo "   ✅ ${name}"
done

# ── 1b. Attribution topic: the public credit for the scaffolding.
#        Visible on the repo homepage; remove it if you prefer not to
#        credit the template (the machinery does not depend on it).
echo "── attribution topic"
gh repo edit "${REPO}" --add-topic rust-forge-template >/dev/null
echo "   ✅ topic rust-forge-template (public credit - remove any time)"

# ── 2. Dormant-lane repo variables (all off; see docs/forge/COMPONENTS.md) ──────
echo "── lane variables (all false - activation is a conscious flip)"
for lane in LANE_RELEASE LANE_RELEASE_PLZ LANE_CRATES LANE_WINGET LANE_BREW LANE_CODECOV LANE_CODEQL LANE_SLSA LANE_CODESIGN; do
  gh variable set "${lane}" --body "false"
  echo "   ✅ ${lane}=false"
done

# ── 3. Repo merge settings ───────────────────────────────────────────
echo "── merge settings (merge-commit-only + auto-merge + delete-branch-on-merge)"
gh repo edit \
  --enable-merge-commit \
  --enable-squash-merge=false \
  --enable-rebase-merge=false \
  --enable-auto-merge \
  --delete-branch-on-merge >/dev/null
echo "   ✅ done"

# ── 4. Branch ruleset for main ───────────────────────────────────────
# The ruleset is DATA: `scripts/ci/main-ruleset.json` (deletion,
# force-push protection, required signatures, a PR
# with every thread resolved, and the required checks; no merge queue,
# which GitHub offers only to repositories an organization owns).
# `apply_main_ruleset.sh` creates it the first time and updates it in
# place afterwards, so re-running this script converges the live
# ruleset on the file instead of skipping it. `just protect-main` is
# the same call for a later re-apply.
#
# Set RULESET_FLAGS=--without-signatures until `just doctor-signing` is
# green for every committer (docs/forge/ADOPTING.md step 5): the
# signature rule blocks every unsigned merge the moment it is active.
# A refusal stops the script (GitHub's error is printed above it): a
# private repository needs GitHub Pro/Team for rulesets.
echo "── ruleset: main-protection"
# RULESET_FLAGS is a list of flags, read into an array word by word.
read -r -a ruleset_flags <<< "${RULESET_FLAGS:-}"
bash "$(dirname "$0")/apply_main_ruleset.sh" "${REPO}" "${ruleset_flags[@]}"
echo "   ✅ applied from scripts/ci/main-ruleset.json"

echo "── ruleset: tag-protection-v-prefix"
TAG_RULESET_JSON=$(cat <<'JSON'
{
  "name": "tag-protection-v-prefix",
  "target": "tag",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["refs/tags/v*"], "exclude": [] } },
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" }
  ]
}
JSON
)
ruleset_names="$(gh api "repos/${REPO}/rulesets" --jq '.[].name')"
if printf '%s\n' "${ruleset_names}" | grep -qx "tag-protection-v-prefix"; then
  echo "   ↷ already exists - skipping"
else
  printf '%s' "${TAG_RULESET_JSON}" | gh api -X POST "repos/${REPO}/rulesets" --input - >/dev/null
  echo "   ✅ created"
fi

# ── 6. Manual steps gh cannot do ─────────────────────────────────────
cat <<'EOF'

📋 Manual follow-ups (GitHub UI / conscious decisions):
   1. Signed commits: if the ruleset went on with RULESET_FLAGS=--without-signatures,
      re-run `just protect-main` without the flag once every committer's
      `just doctor-signing` is green.
   2. Secrets (only when the matching lane goes live; docs/forge/COMPONENTS.md):
      CODECOV_TOKEN (lane:codecov), CARGO_REGISTRY_TOKEN (lane:crates-publish),
      WINGET_TOKEN (lane:winget), HOMEBREW_TAP_TOKEN (lane:brew).
      lane:codesign needs none: the Apple keys stay in the owner's
      keychain (docs/forge/CODE-SIGNING.md, section 5).
   3. Dependabot: Settings → Code security - enable Dependabot alerts +
      security updates (dependabot.yml in-repo handles version updates).
EOF

echo
echo "🎉 bootstrap complete"
