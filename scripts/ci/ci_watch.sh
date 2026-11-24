#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The CI watch, run every half hour by `.github/workflows/ci-watch.yml`.
# It reads the workflow runs of the last day that ended in failure and
# answers each once:
#
#   1. A run of one of the RERUN workflows, at its first attempt, from
#      this repository, whose failed-job log shows a transient signature
#      (an HTTP 429, a reset connection, a runner that lost contact, a
#      full disk: TRANSIENT below) is run again, its failed jobs only.
#      The second attempt is never run again, so a failure that stays is
#      reported on the next pass.
#   2. Any other failure of a WATCHED workflow that a pull request did not
#      start (a pull request's author sees their own checks) gets a
#      tracking issue, labelled for its workflow and `ci-failure`, or a
#      comment on the open one of that label, and a mail to
#      <repository>failed@nios.net.
#
# Deciding before filing means a transient failure never files an issue
# that a green rerun would make stale.  The issue or comment carries the
# marker `ci-watch run=<id> attempt=<n>`; a run whose marker is already
# in an issue or comment of the window is not filed again, so a pass may
# run twice, or late, without a second report.
#
# Why a schedule, not the `workflow_run` trigger: `workflow_run` runs
# with the repository's token after any watched run, a fork's pull
# request's included, and zizmor (gate `zizmor`) refuses it for that.
# Nothing here reads a pull request's code: only the run list, job logs
# and issues, through the API.
#
# Environment: GH_REPO (owner/name), GH_TOKEN (actions: write, issues:
# write), ASSIGNEE (who the issue is assigned to), SMTP_USER and
# SMTP_PASSWORD (the mail; without them the issue is filed and the run
# fails, for a channel that does not deliver is not one), WINDOW_HOURS
# (how far back, 24 by default).
#
# Exit 0 when every failure was answered, 1 when a rerun, an issue or a
# mail failed (each attempted all the same), 2 when the environment is
# incomplete.

set -euo pipefail
shopt -s inherit_errexit

: "${GH_REPO:?GH_REPO (owner/name) is required}"
: "${ASSIGNEE:?ASSIGNEE (who a failure issue is assigned to) is required}"
window_hours="${WINDOW_HOURS:-24}"

# The workflows whose transient failure is run again once.
RERUN=(
  "PR Fast CI"
  "🔍 CodeQL (Rust SAST)"
  "🌙 MD2HTML Tier 2 Nightly CI"
)

# The workflows whose failure files an issue: name -> label|title.  A
# workflow not listed here files nothing (this watch, commitlint, the
# Dependabot workflows and the preview builds: a pull request's own).
declare -A WATCHED=(
  ["ci"]="ci-failure-ci|🔴 CI Failure: ci (tests, MSRV, supply chain, browser, EPUB)"
  ["PR Fast CI"]="ci-failure-pr-fast|🔴 CI Failure: PR Fast CI"
  ["Homebrew Publish"]="ci-failure-homebrew-publish|🔴 CI Failure: Homebrew Publish"
  ["🔄 cargo-vet: weekly import refresh"]="ci-failure-cargo-vet-weekly-import-refresh|🔴 CI Failure: cargo-vet: weekly import refresh"
  ["🔍 CodeQL (Rust SAST)"]="ci-failure-codeql-rust-sast|🔴 CI Failure: CodeQL (Rust SAST)"
  ["📦 crates.io dry-run + semver-checks"]="ci-failure-crates-io-dry-run-semver-checks|🔴 CI Failure: crates.io dry-run + semver-checks"
  ["Main merge guard"]="ci-failure-main-merge-guard|🔴 CI Failure: Main merge guard"
  ["🐤 Nightly Canary"]="ci-failure-nightly-canary|🐤 Nightly Canary: upcoming-toolchain regression"
  ["🔥 Release cache warm"]="ci-failure-release-cache-warm|🔴 CI Failure: Release cache warm"
  ["🚀 Release-plz"]="ci-failure-release-plz|🔴 CI Failure: Release-plz"
  ["🚀 MD2HTML Release Pipeline"]="ci-failure-release|🔴 Release Failure"
  ["🌙 MD2HTML Tier 2 Nightly CI"]="ci-failure-tier-2|🔴 CI Failure: Tier 2 Nightly CI"
  ["📦 WinGet Publish"]="ci-failure-winget-publish|🔴 CI Failure: WinGet Publish"
  ["🔑 WinGet Token Expiry Check"]="ci-failure-winget-token-expiry-check|🔴 CI Failure: WinGet Token Expiry Check"
)

# Transient signatures, matched against a failed job's log.  Each is an
# infrastructure error string; a compile error, a failed test or a lint
# finding prints none of them.  Extend with care: every match costs a
# rerun.  `no matching package named` after a git-source fetch is a
# fetch that landed incomplete; a rerun of the same pin heals it.  Not
# here on purpose: `GLIBC_x.y' not found` (a tool binary newer than the
# runner image), which a rerun reproduces exactly.
TRANSIENT='Too Many Requests|\b429\b|failed to download|Failed to download archive|ECONNRESET|Connection reset by peer|EAI_AGAIN|unable to resolve host|Network is unreachable|runner has lost contact|The runner was forced to stop|context deadline exceeded|No space left on device|no matching package named'

since="$(jq -nr --argjson hours "$window_hours" 'now - $hours * 3600 | strftime("%Y-%m-%dT%H:%M:%SZ")')"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
failures=0

is_rerun_workflow() {
  local name="$1" listed
  for listed in "${RERUN[@]}"; do
    [[ "$listed" == "$name" ]] && return 0
  done
  return 1
}

# The markers already filed in the window: every issue of the label
# `ci-failure` and every comment in the repository updated since then.
gh api --paginate -X GET "/repos/$GH_REPO/issues" \
  -f labels=ci-failure -f state=all -f since="$since" -f per_page=100 \
  | jq -r '.[].body // empty' > "$scratch/filed"
gh api --paginate -X GET "/repos/$GH_REPO/issues/comments" \
  -f since="$since" -f per_page=100 \
  | jq -r '.[].body // empty' >> "$scratch/filed"

# Whether `run` was run again: the first failed job whose log shows a
# transient signature asks for a rerun of the run's failed jobs.  A log
# that cannot be read is not retried: the failure is reported instead.
rerun_if_transient() {
  local run="$1" jobs job_id job_name log signature status
  jobs="$(gh api -X GET "/repos/$GH_REPO/actions/runs/$run/jobs" -f filter=latest -f per_page=100 \
    | jq -r '.jobs[] | select(.conclusion == "failure") | "\(.id) \(.name)"')"
  while IFS=' ' read -r job_id job_name; do
    [[ -n "$job_id" ]] || continue
    if ! log="$(gh api "/repos/$GH_REPO/actions/jobs/$job_id/logs" 2> "$scratch/log-error")"; then
      printf '::warning::run %s: the log of job %s (%s) cannot be read, so it is not retried: %s\n' \
        "$run" "$job_id" "$job_name" "$(cat "$scratch/log-error")"
      continue
    fi
    # grep ends with 1 when no line matches (a genuine failure) and with
    # 2 on a bad pattern, which stops the watch.
    status=0
    signature="$(grep -E -m1 "$TRANSIENT" <<< "$log")" || status=$?
    if (( status > 1 )); then
      printf '::error::grep failed (exit %s) on the transient pattern\n' "$status"
      exit 1
    fi
    if (( status == 0 )); then
      printf '   transient in job %s (%s): %s\n' "$job_id" "$job_name" "${signature:0:200}"
      if gh api --method POST "/repos/$GH_REPO/actions/runs/$run/rerun-failed-jobs" > /dev/null; then
        printf '   run %s: failed jobs run again\n' "$run"
        return 0
      fi
      printf '::error::run %s: the rerun was refused; the failure is reported instead\n' "$run"
      failures=$((failures + 1))
      return 1
    fi
  done <<< "$jobs"
  return 1
}

# The mail for one failure: from the account that owns the app password
# (nios.net's DMARC admits only its own relay as a sender) to
# <repository>failed@nios.net, by plain SMTP.
mail_failure() {
  local subject="$1" text="$2" repo to message
  repo="${GH_REPO#*/}"
  to="${repo,,}failed@nios.net"
  if [[ -z "${SMTP_USER:-}" || -z "${SMTP_PASSWORD:-}" ]]; then
    printf '::error::NOTIFY_SMTP_USER / NOTIFY_SMTP_PASSWORD are not set; no mail was sent for: %s\n' "$subject"
    failures=$((failures + 1))
    return
  fi
  message="$scratch/mail"
  {
    printf 'From: %s CI <%s>\r\n' "$repo" "$SMTP_USER"
    printf 'To: %s\r\n' "$to"
    printf 'Subject: [%s] %s\r\n' "$repo" "$subject"
    printf 'Date: %s\r\n' "$(date -R)"
    printf 'MIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n'
    printf '%s\r\n' "$text"
  } > "$message"
  if curl --silent --show-error --fail --ssl-reqd \
      --url 'smtp://smtp.gmail.com:587' \
      --user "$SMTP_USER:$SMTP_PASSWORD" \
      --mail-from "$SMTP_USER" --mail-rcpt "$to" \
      --upload-file "$message"; then
    printf '   mailed %s\n' "$to"
  else
    printf '::error::the mail to %s failed\n' "$to"
    failures=$((failures + 1))
  fi
}

# Files or comments on the tracking issue of one failure, then mails it.
report() {
  local run_json="$1" run attempt name branch sha number url actor label title marker
  local short body existing issue_url issue_title
  run="$(jq -r .id <<< "$run_json")"
  attempt="$(jq -r .run_attempt <<< "$run_json")"
  name="$(jq -r .name <<< "$run_json")"
  branch="$(jq -r .head_branch <<< "$run_json")"
  sha="$(jq -r .head_sha <<< "$run_json")"
  number="$(jq -r .run_number <<< "$run_json")"
  url="$(jq -r .html_url <<< "$run_json")"
  actor="$(jq -r .actor <<< "$run_json")"
  label="${WATCHED[$name]%%|*}"
  title="${WATCHED[$name]#*|}"
  marker="ci-watch run=$run attempt=$attempt"
  short="${sha:0:7}"

  body="$scratch/body"
  {
    printf "## %s failed on \`%s\`\n\n" "$name" "$branch"
    printf '| Field | Value |\n|-------|-------|\n'
    printf "| **Workflow** | \`%s\` |\n" "$name"
    printf '| **Run** | [#%s](%s) |\n' "$number" "$url"
    printf '| **Attempt** | %s |\n' "$attempt"
    printf "| **Branch** | \`%s\` |\n" "$branch"
    printf "| **Commit** | [\`%s\`](https://github.com/%s/commit/%s) |\n" "$short" "$GH_REPO" "$sha"
    printf "| **Started by** | \`%s\` |\n\n" "$actor"
    if [[ "$attempt" == "1" ]]; then
      printf "_Filed by the CI watch (\`scripts/ci/ci_watch.sh\`): no transient signature in the failed jobs, so this is a genuine failure._\n"
    else
      printf "_Filed by the CI watch (\`scripts/ci/ci_watch.sh\`): attempt %s failed too, a lasting fault or a real regression._\n" "$attempt"
    fi
    if [[ "$name" == "🐤 Nightly Canary" ]]; then
      printf "\nThis is an early warning from the floating-nightly canary, not a broken pin: \`main\` still builds on the pinned toolchain. Look before the next pin bump; an upstream or dependency regression is usually fixed by a dependency bump, not a toolchain downgrade.\n"
    fi
    printf '\nMailed to the repository'"'"'s failure address; GitHub mails the assignee on this issue and on every repeat-failure comment.\n'
    printf '\n<!-- %s -->\n' "$marker"
  } > "$body"

  issue_url=""
  issue_title="$title - $short"
  if existing="$(gh api -X GET "/repos/$GH_REPO/issues" -f labels="$label" -f state=open -f per_page=1 \
      | jq -r '.[0] // empty | "\(.number) \(.html_url)"')"; then
    if [[ -n "$existing" ]]; then
      if gh api --method POST "/repos/$GH_REPO/issues/${existing%% *}/comments" \
          -F body=@"$body" > /dev/null; then
        issue_url="${existing#* }"
        issue_title="$title (repeat: $short)"
        printf '   commented on %s\n' "$issue_url"
      fi
    elif issue_url="$(gh api --method POST "/repos/$GH_REPO/issues" \
        -f title="$issue_title" -F body=@"$body" \
        -f 'labels[]'="$label" -f 'labels[]'=ci-failure -f 'assignees[]'="$ASSIGNEE" \
        | jq -r .html_url)"; then
      printf '   filed %s\n' "$issue_url"
    fi
  fi
  if [[ -z "$issue_url" ]]; then
    printf '::error::run %s: the tracking issue could not be filed; the mail still goes\n' "$run"
    failures=$((failures + 1))
  fi
  mail_failure "$issue_title" "$(printf '%s failed on %s (%s).\r\n\r\nRun:   %s\r\nIssue: %s' \
    "$name" "$branch" "$short" "$url" "${issue_url:-none, it could not be filed}")"
}

printf 'CI watch over %s: failed runs since %s\n' "$GH_REPO" "$since"
runs="$(gh api --paginate -X GET "/repos/$GH_REPO/actions/runs" \
    -f status=failure -f created=">=$since" -f per_page=100 \
  | jq -c '.workflow_runs[] | {id, name, run_attempt, event, head_branch, head_sha, run_number,
      html_url, actor: .actor.login,
      ours: (.head_repository.full_name == .repository.full_name)}')"

while IFS= read -r run_json; do
  [[ -n "$run_json" ]] || continue
  run="$(jq -r .id <<< "$run_json")"
  name="$(jq -r .name <<< "$run_json")"
  attempt="$(jq -r .run_attempt <<< "$run_json")"
  event="$(jq -r .event <<< "$run_json")"
  ours="$(jq -r .ours <<< "$run_json")"
  printf '· run %s (%s), attempt %s, %s\n' "$run" "$name" "$attempt" "$event"

  if [[ "$attempt" == "1" && "$ours" == "true" ]] && is_rerun_workflow "$name"; then
    if rerun_if_transient "$run"; then
      continue
    fi
  fi
  if [[ "$event" == "pull_request" ]]; then
    printf '   a pull request'"'"'s run: its author sees it\n'
    continue
  fi
  if [[ -z "${WATCHED[$name]+listed}" ]]; then
    printf '   not a watched workflow\n'
    continue
  fi
  if grep -qF "ci-watch run=$run attempt=$attempt" "$scratch/filed"; then
    printf '   already reported\n'
    continue
  fi
  report "$run_json"
done <<< "$runs"

if (( failures > 0 )); then
  printf '❌ %d step(s) of the watch failed (above)\n' "$failures"
  exit 1
fi
printf '✅ every failure answered\n'
