#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The anti-pattern gate's own test (run by scripts/ci/run_gate_tests.sh, gate
# `gate-tests`).  Each case builds a scratch git repository holding a few
# Rust files, runs scripts/ci/anti_pattern_gate.sh in it and checks the
# exit status: 0 nothing to refuse, 1 a finding, 2 the check cannot run.
# A planted violation of each rule must be refused, the one approved
# exception must pass, and nothing else may.

set -euo pipefail

# Every case is a repository of its own.  A git hook exports GIT_DIR (and,
# in a worktree, more), and while it is set `git init` and `git add` act on
# the repository the hook runs for, not on the scratch one: `git init`
# made this repository bare and `git add` staged the scratch files into its
# index.  The variables that point git at a repository are cleared first,
# and every case checks that its repository is the one it made.
while IFS= read -r name; do
  unset "$name"
done < <(git rev-parse --local-env-vars)

gate="$(cd "$(dirname "$0")" && pwd)/anti_pattern_gate.sh"
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
failures=0
cases=0

# The approved attribute, as geom.rs carries it.
approved='#![expect(clippy::float_arithmetic, reason = "the geometry kernel")]'

# scratch_repository <folder>: a repository made in <folder>, refused
# loudly when git made or used one anywhere else.
scratch_repository() {
  git -C "$1" init -q
  if [[ ! -d "$1/.git" ]]; then
    printf 'ERROR: git init did not make %s/.git; git is pointed at another repository\n' "$1" >&2
    exit 2
  fi
}

# check <want> <description> <path> <content> [<path> <content>]...
# A fresh repository with a clean file beside the planted ones.
check() {
  local want="$1" desc="$2" got=0 repo
  shift 2
  cases=$((cases + 1))
  repo="$scratch/case$cases"
  mkdir -p "$repo/crates/clean/src"
  printf 'fn main() {}\n' > "$repo/crates/clean/src/main.rs"
  while (( $# >= 2 )); do
    mkdir -p "$repo/$(dirname "$1")"
    printf '%s\n' "$2" > "$repo/$1"
    shift 2
  done
  scratch_repository "$repo"
  git -C "$repo" add -A
  (cd "$repo" && bash "$gate") > "$repo.out" 2>&1 || got=$?
  if [[ "$got" == "$want" ]]; then
    printf 'OK   (exit %s) %s\n' "$got" "$desc"
  else
    printf 'FAIL (exit %s, want %s) %s\n' "$got" "$want" "$desc"
    sed 's/^/     | /' "$repo.out"
    failures=$((failures + 1))
  fi
}

# ── Nothing to refuse ────────────────────────────────────────────────────
check 0 "a clean tree" \
  crates/a/src/lib.rs 'pub fn f() -> u8 { 1 }'
check 0 "the approved exception in its file" \
  crates/md2html/src/mermaid/geom.rs "$approved"
check 0 "the approved exception over two lines" \
  crates/md2html/src/mermaid/geom.rs $'#![expect(\n    clippy::float_arithmetic,\n    reason = "the geometry kernel"\n)]'
check 0 "a method named expect is not an attribute" \
  crates/a/src/lib.rs 'fn f(v: Option<u8>) -> u8 { v.expect("set") }'
check 0 "an attribute in a comment is only mentioned" \
  crates/a/src/lib.rs '// never write #[allow(dead_code)] here'
check 0 "lossy decoding in a test is a test's business" \
  crates/a/src/tests.rs 'fn t(b: &[u8]) { let _s = String::from_utf8_lossy(b); }' \
  crates/a/tests/cli.rs 'fn t(p: &std::path::Path) { let _s = p.to_string_lossy(); }'
check 0 "a lossy call named in a comment" \
  crates/a/src/lib.rs '// String::from_utf8_lossy would hide the bad bytes'

# ── Rule 1: lint exceptions, in any form, anywhere ──────────────────────
check 1 "#[allow] in production code" \
  crates/a/src/lib.rs $'#[allow(dead_code)]\nfn f() {}'
check 1 "#![allow] at a crate root" \
  crates/a/src/main.rs '#![allow(clippy::pedantic)]'
check 1 "#[expect] with a reason" \
  crates/a/src/lib.rs $'#[expect(clippy::cast_lossless, reason = "x")]\nfn f() {}'
check 1 "#[allow] on a test module" \
  crates/a/src/lib.rs $'#[cfg(test)]\n#[allow(clippy::unwrap_used)]\nmod tests {}'
check 1 "#[allow] in an integration test" \
  crates/a/tests/cli.rs $'#[allow(unused)]\nfn t() {}'
check 1 "#[allow] in the tooling" \
  scripts/ci/tool/src/main.rs '#[allow(clippy::print_stdout)]'
check 1 "cfg_attr with allow on one line" \
  crates/a/src/lib.rs $'#[cfg_attr(test, allow(dead_code))]\nfn f() {}'
check 1 "cfg_attr with expect on a following line" \
  crates/a/src/lib.rs $'#[cfg_attr(\n    test,\n    expect(dead_code, reason = "x")\n)]\nfn f() {}'
check 1 "spaces inside the attribute" \
  crates/a/src/lib.rs $'# [ allow (dead_code)]\nfn f() {}'
check 1 "the approved lint in another file" \
  crates/md2html/src/mermaid/plane.rs "$approved"
check 1 "the approved file naming a second lint" \
  crates/md2html/src/mermaid/geom.rs '#![expect(clippy::float_arithmetic, clippy::cast_lossless, reason = "x")]'
check 1 "the approved lint allowed instead of expected" \
  crates/md2html/src/mermaid/geom.rs '#![allow(clippy::float_arithmetic, reason = "x")]'
check 1 "the approved file with another lint" \
  crates/md2html/src/mermaid/geom.rs '#![expect(clippy::float_cmp, reason = "x")]'
check 1 "the approved file with a second attribute" \
  crates/md2html/src/mermaid/geom.rs "$approved"$'\n\n\n\n\n#[allow(dead_code)]\nfn f() {}'

# ── Rule 2: bytes read as text, in production code ──────────────────────
check 1 "from_utf8_lossy in production code" \
  crates/a/src/lib.rs 'fn f(b: &[u8]) -> String { String::from_utf8_lossy(b).into_owned() }'
check 1 "from_utf16_lossy in production code" \
  crates/a/src/lib.rs 'fn f(b: &[u16]) -> String { String::from_utf16_lossy(b) }'
check 1 "to_string_lossy of a name" \
  crates/a/src/lib.rs 'fn f(p: &std::path::Path) -> String { p.to_string_lossy().into_owned() }'
check 1 "lossy decoding in the tooling" \
  scripts/ci/tool/src/main.rs 'fn f(b: &[u8]) { let _s = String::from_utf8_lossy(b); }'

# ── Rule 3: discarded writes ─────────────────────────────────────────────
check 1 "a discarded write to a stream" \
  crates/a/src/lib.rs 'fn f(stream: &mut std::net::TcpStream) { drop(stream.write_all(b"x")); }'
check 1 "a discarded flush of a pipe" \
  crates/a/src/lib.rs 'fn f(pipe: &mut std::process::ChildStdin) { drop(pipe.flush()); }'

# ── The check cannot run: an error, never a pass ────────────────────────
cases=$((cases + 1))
empty="$scratch/case$cases"
mkdir -p "$empty"
scratch_repository "$empty"
got=0
(cd "$empty" && bash "$gate") > "$empty.out" 2>&1 || got=$?
if [[ "$got" == 2 ]]; then
  printf 'OK   (exit 2) a repository with no Rust file is an error\n'
else
  printf 'FAIL (exit %s, want 2) a repository with no Rust file is an error\n' "$got"
  failures=$((failures + 1))
fi

if (( failures > 0 )); then
  printf '%d of %d anti-pattern gate cases failed\n' "$failures" "$cases"
  exit 1
fi
printf 'all %d anti-pattern gate cases passed\n' "$cases"
