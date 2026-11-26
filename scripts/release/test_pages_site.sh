#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of scripts/release/pages_site.py (the site on GitHub Pages),
# run by the gate `gate-tests` against a stand-in for md2html that writes
# a page, the README one with the links $LINKS names: the README is the
# front page and keeps its name, the root documents and the handbook are
# there, a file a page links to is copied beside it, and a link that leads
# nowhere, or out of the repository, fails the build with its page named;
# a folder that exists and a failed render are refused. Exit 1 when a case
# fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/release/pages_site.py"
work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT
failed=0
count=0

# md2html's stand-in: `SOURCE -o TARGET` writes one page at TARGET (the
# handbook's start page inside it for `docs`); the README's links to each
# path of $LINKS; $FAIL_ON names a source it fails on.
cat > "$work/md2html" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
source="$1" target="$3"
[[ "$source" != "${FAIL_ON:-}" ]] || exit 3
if [[ "$source" == docs ]]; then
  mkdir -p "$target"
  target="$target/README.html"
fi
{
  printf '<!doctype html><title>%s</title>' "$source"
  printf '<a href="https://example.org/">out</a><a href="#top">here</a>'
  [[ "$source" == README.md ]] || LINKS=""
  for link in ${LINKS:-}; do
    printf '<a href="%s">x</a>' "$link"
  done
} > "$target"
EOF
chmod +x "$work/md2html"

# Runs the script into a new folder, with the given NAME=value pairs; its
# output goes to $out, its exit code to $status, the site to $site.
site_of() {
  count=$((count + 1))
  site="$work/site-$count"
  status=0
  out="$(env -u LINKS -u FAIL_ON "$@" python3 "$script" "$work/md2html" "$repo" "$site" 2>&1)" || status=$?
}

# One case: the exit code, then every `+text` must be in the output and
# every `-text` must not, and every `=path` must be a file of the site.
expect() {
  local name="$1" code="$2" want problem=""
  shift 2
  [[ "$status" == "$code" ]] || problem="exit $status, not $code"
  for want in "$@"; do
    case "$want" in
      +*) grep -qF -- "${want#+}" <<< "$out" || problem="${problem:+$problem; }missing: ${want#+}" ;;
      -*) if grep -qF -- "${want#-}" <<< "$out"; then problem="${problem:+$problem; }unwanted: ${want#-}"; fi ;;
      =*) [[ -f "$site/${want#=}" ]] || problem="${problem:+$problem; }no file: ${want#=}" ;;
    esac
  done
  if [[ -n "$problem" ]]; then
    printf '  ✗ %s: %s\n' "$name" "$problem"
    failed=$((failed + 1))
  else
    printf '  ✓ %s\n' "$name"
  fi
}

site_of
expect "the README is the front page, under its name too, beside the root documents and the handbook" 0 \
  "+pages in" =index.html =README.html =CHANGELOG.html =SECURITY.html =AGENTS.html =GOTO.html =docs/README.html

site_of LINKS="docs/capabilities/light.jpg CHANGELOG.html docs/README.html"
expect "a file a page links to is copied beside it" 0 =docs/capabilities/light.jpg
if cmp -s "$repo/docs/capabilities/light.jpg" "$site/docs/capabilities/light.jpg"; then
  printf '  ✓ the copy is the file\n'
else
  printf '  ✗ the copy is the file\n'
  failed=$((failed + 1))
fi

site_of LINKS="nowhere.html"
expect "a link to no file fails, with its page named" 1 "+README.html links to nowhere.html" "-pages in"

site_of LINKS="docs/command-line.md"
expect "a link to Markdown that is no page of the site fails" 1 "+links to docs/command-line.md"

site_of LINKS="../outside.txt"
expect "a link out of the repository fails" 1 "+links to ../outside.txt"

site_of FAIL_ON="SECURITY.md"
expect "a failed render fails the build" 1 "+md2html failed on SECURITY.md"

mkdir -p "$work/taken"
status=0
out="$(python3 "$script" "$work/md2html" "$repo" "$work/taken" 2>&1)" || status=$?
site="$work/taken"
expect "a folder that exists is refused" 2 "+exists"

status=0
out="$(python3 "$script" "$work/md2html" "$work" "$work/new" 2>&1)" || status=$?
site="$work/new"
expect "sources with no README are refused" 2 "+holds no README.md"

if (( failed > 0 )); then
  printf '❌ %d of %d pages_site cases failed\n' "$failed" "$((count + 3))"
  exit 1
fi
printf '✅ pages_site: %d cases passed\n' "$((count + 3))"
