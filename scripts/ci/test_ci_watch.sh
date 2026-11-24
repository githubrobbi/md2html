#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of scripts/ci/ci_watch.sh (the CI watch), run by the gate
# `gate-tests`.  Each case writes the API's answers as files, puts a
# `gh` and a `curl` of its own first on the PATH (they answer from the
# files and write down every call), runs the watch, and checks what it
# asked of GitHub and of the mail: a transient failure is run again and
# never filed, a genuine one is filed once and mailed, a pull request's
# run is left to its author.  Nothing reaches GitHub or a mail server.
# Exit 1 when a case fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/ci/ci_watch.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

failed=0
count=0

# The stand-ins.  `gh api` answers GETs from "$FIX" and writes every
# call, with its fields, to "$CALLS"; a body given as `body=@file` is
# copied there too.  `curl` writes the mail's recipient.
mkdir -p "$scratch/bin"
cat > "$scratch/bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
method=GET
path=""
fields=()
while (( $# > 0 )); do
  case "$1" in
    -X|--method) method="$2"; shift ;;
    -f|-F) fields+=("$2"); shift ;;
    /*) path="$1" ;;
  esac
  shift
done
printf '%s %s %s\n' "$method" "$path" "${fields[*]}" >> "$CALLS"
for field in "${fields[@]}"; do
  if [[ "$field" == body=@* ]]; then
    cat "${field#body=@}" >> "$CALLS"
  fi
done
answer() {
  if [[ -f "$FIX/$1" ]]; then
    cat "$FIX/$1"
  else
    printf '%s\n' "$2"
  fi
}
case "$method $path" in
  "GET /repos/o/r/actions/runs") answer runs.json '{"workflow_runs":[]}' ;;
  "GET /repos/o/r/issues/comments") answer comments.json '[]' ;;
  "GET /repos/o/r/issues")
    if [[ " ${fields[*]} " == *" state=open "* ]]; then
      answer open-issue.json '[]'
    else
      answer issues.json '[]'
    fi
    ;;
  "GET /repos/o/r/actions/runs/"*"/jobs")
    id="${path#/repos/o/r/actions/runs/}"
    answer "jobs-${id%/jobs}.json" '{"jobs":[]}'
    ;;
  "GET /repos/o/r/actions/jobs/"*"/logs")
    id="${path#/repos/o/r/actions/jobs/}"
    id="${id%/logs}"
    if [[ ! -f "$FIX/log-$id.txt" ]]; then
      printf 'HTTP 410: the log is gone\n' >&2
      exit 1
    fi
    cat "$FIX/log-$id.txt"
    ;;
  "POST /repos/o/r/actions/runs/"*"/rerun-failed-jobs")
    if [[ -f "$FIX/refuse-rerun" ]]; then
      printf 'HTTP 403: refused\n' >&2
      exit 1
    fi
    printf '{}\n'
    ;;
  "POST /repos/o/r/issues") printf '{"html_url":"https://github.com/o/r/issues/99"}\n' ;;
  "POST /repos/o/r/issues/"*"/comments") printf '{}\n' ;;
  *) printf 'the stand-in gh has no answer for: %s %s\n' "$method" "$path" >&2; exit 1 ;;
esac
GH
cat > "$scratch/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
while (( $# > 0 )); do
  if [[ "$1" == --mail-rcpt ]]; then
    printf 'MAIL %s\n' "$2" >> "$CALLS"
  fi
  shift
done
CURL
chmod +x "$scratch/bin/gh" "$scratch/bin/curl"

# A failed run as the API lists it: id, workflow, attempt, event, and
# whether it comes from this repository (a fork's pull request does not).
run() {
  local head="o/r"
  [[ "$5" == ours ]] || head="someone/r"
  jq -n --argjson id "$1" --arg name "$2" --argjson attempt "$3" --arg event "$4" --arg head "$head" \
    '{id: $id, name: $name, run_attempt: $attempt, event: $event, head_branch: "main",
      head_sha: "0123456789abcdef", run_number: 7, html_url: "https://github.com/o/r/actions/runs/\($id)",
      actor: {login: "someone"}, head_repository: {full_name: $head}, repository: {full_name: "o/r"}}'
}

# The runs of a case, as the one page the API answers.
runs() {
  jq -s '{workflow_runs: .}' > "$FIX/runs.json"
}

# One failed job of a run, and its log.
failed_job() {
  printf '{"jobs":[{"id":%s,"name":"build","conclusion":"failure"},{"id":1,"name":"fine","conclusion":"success"}]}\n' \
    "$2" > "$FIX/jobs-$1.json"
  printf '%s\n' "$3" > "$FIX/log-$2.txt"
}

# A fresh case: empty answers, no calls yet.
fresh() {
  FIX="$scratch/fix"
  CALLS="$scratch/calls"
  rm -rf "$FIX"
  mkdir -p "$FIX"
  : > "$CALLS"
  export FIX CALLS
}

# Runs the watch; its exit code goes to $status, its output to $out.
watch() {
  status=0
  out="$(env PATH="$scratch/bin:$PATH" GH_REPO=o/r ASSIGNEE=owner \
    SMTP_USER="${smtp_user-mailer}" SMTP_PASSWORD="${smtp_password-secret}" \
    bash "$script" 2>&1)" || status=$?
}

# Checks one case: the exit code, then every `+text` must be among the
# calls and every `-text` must not.
expect() {
  local name="$1" code="$2" want problem=""
  shift 2
  count=$((count + 1))
  if [[ "$status" != "$code" ]]; then
    problem="exit $status, not $code"
  fi
  for want in "$@"; do
    case "$want" in
      +*) grep -qF -- "${want#+}" "$CALLS" || problem="${problem:+$problem; }missing: ${want#+}" ;;
      -*) if grep -qF -- "${want#-}" "$CALLS"; then problem="${problem:+$problem; }unwanted: ${want#-}"; fi ;;
    esac
  done
  if [[ -n "$problem" ]]; then
    printf '  ✗ %s: %s\n%s\n--- calls\n%s\n' "$name" "$problem" "$out" "$(cat "$CALLS")"
    failed=$((failed + 1))
  else
    printf '  ✓ %s\n' "$name"
  fi
}

fresh
watch
expect "no failed run asks for nothing" 0 "-POST"

fresh
run 11 "PR Fast CI" 1 pull_request ours | runs
failed_job 11 501 "Error: Too Many Requests (429) fetching the action"
watch
expect "a transient failure is run again, never filed" 0 \
  "+POST /repos/o/r/actions/runs/11/rerun-failed-jobs" "-POST /repos/o/r/issues" "-MAIL"

fresh
run 12 "PR Fast CI" 1 push ours | runs
failed_job 12 502 "##[error]Process completed with exit code 1."
printf '%s\n' "test parse::tables ... FAILED" >> "$FIX/log-502.txt"
watch
expect "a genuine failure on main is filed and mailed" 0 \
  "-rerun-failed-jobs" "+POST /repos/o/r/issues title=🔴 CI Failure: PR Fast CI - 0123456" \
  "+labels[]=ci-failure-pr-fast labels[]=ci-failure assignees[]=owner" \
  "+<!-- ci-watch run=12 attempt=1 -->" "+MAIL rfailed@nios.net" \
  "+| **Workflow** | \`PR Fast CI\` |" "+| **Commit** | [\`0123456\`](https://github.com/o/r/commit/0123456789abcdef) |"

fresh
run 13 "PR Fast CI" 1 pull_request ours | runs
failed_job 13 503 "error[E0308]: mismatched types"
watch
expect "a pull request's genuine failure is left to its author" 0 "-POST" "-MAIL"

fresh
run 14 "PR Fast CI" 1 pull_request fork | runs
failed_job 14 504 "Connection reset by peer"
watch
expect "a fork's run is never run again" 0 "-POST" "-/jobs" "-MAIL"

fresh
run 15 "ci" 1 push ours | runs
printf '[{"body":"earlier\\n<!-- ci-watch run=15 attempt=1 -->"}]\n' > "$FIX/comments.json"
watch
expect "a failure already reported is not filed again" 0 "-POST" "-MAIL"

fresh
run 16 "ci" 1 schedule ours | runs
printf '[{"number":7,"html_url":"https://github.com/o/r/issues/7"}]\n' > "$FIX/open-issue.json"
watch
expect "a repeat failure comments on the open issue" 0 \
  "+POST /repos/o/r/issues/7/comments" "-POST /repos/o/r/issues title=" "-/jobs" "+MAIL rfailed@nios.net"

fresh
run 17 "PR Fast CI" 2 push ours | runs
failed_job 17 507 "Too Many Requests"
watch
expect "a second attempt is filed, not run again" 0 \
  "-rerun-failed-jobs" "+<!-- ci-watch run=17 attempt=2 -->" "+attempt 2 failed too"

fresh
run 18 "Commit lint" 1 push ours | runs
watch
expect "a workflow not watched files nothing" 0 "-POST" "-MAIL"

fresh
run 19 "PR Fast CI" 1 push ours | runs
failed_job 19 509 "No space left on device"
touch "$FIX/refuse-rerun"
watch
expect "a refused rerun is filed instead, and the watch fails" 1 \
  "+rerun-failed-jobs" "+<!-- ci-watch run=19 attempt=1 -->" "+MAIL"

fresh
run 20 "🌙 MD2HTML Tier 2 Nightly CI" 1 schedule ours | runs
printf '{"jobs":[{"id":510,"name":"miri","conclusion":"failure"}]}\n' > "$FIX/jobs-20.json"
watch
expect "a log that cannot be read is reported, not retried" 0 \
  "-rerun-failed-jobs" "+POST /repos/o/r/issues title=🔴 CI Failure: Tier 2 Nightly CI" "+MAIL"

fresh
run 21 "ci" 1 push ours | runs
smtp_user="" smtp_password="" watch
expect "without the mail's secrets the issue is filed and the watch fails" 1 \
  "+POST /repos/o/r/issues" "-MAIL"

fresh
run 22 "🐤 Nightly Canary" 1 schedule ours | runs
watch
expect "the canary's issue says it is an early warning" 0 \
  "+title=🐤 Nightly Canary: upcoming-toolchain regression - 0123456" "+early warning"

if (( failed > 0 )); then
  printf '❌ %d of %d CI-watch cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ %d CI-watch cases passed\n' "$count"
