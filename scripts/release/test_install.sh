#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# The tests of packaging/install/install.sh (the curl installer), run by
# the gate `gate-tests`, against stand-ins for `uname`, `sysctl`, `curl`
# and `wget` that serve a release from a folder: each system gets its
# binary (Apple silicon, Intel, a Rosetta shell, Linux x64) from the
# newest release or from the one MD2HTML_VERSION names; a system without
# a build, a version that is not one, a binary the list does not hold or
# holds with another SHA-256, a failed download and a binary that does
# not run are refused with nothing installed; an install replaces the
# one before it; wget serves when there is no curl; the PATH advice is
# given only when the folder is not on it. Each case runs under /bin/sh,
# and under dash too where there is one. Exit 1 when a case fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/packaging/install/install.sh"
failed=0
count=0
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

shells=(/bin/sh)
if dash_path="$(command -v dash)"; then
  shells+=("$dash_path")
fi

# The real tools the installer uses, alone on its PATH beside the
# stand-ins, so a curl or an md2html of this machine never answers.
tools="$work/tools"
mkdir -p "$tools"
for tool in awk cut mktemp cp chmod mv mkdir rm cat; do
  ln -s "$(command -v "$tool")" "$tools/$tool"
done
if hasher="$(command -v sha256sum)"; then
  ln -s "$hasher" "$tools/sha256sum"
else
  ln -s "$(command -v shasum)" "$tools/shasum"
fi

# The stand-ins. uname answers $FAKE_SYSTEM and $FAKE_MACHINE; sysctl
# answers $FAKE_TRANSLATED, or fails when it is empty (an Intel Mac);
# curl and wget write the served file for an address under
# https://github.com/githubrobbi/md2html/releases/ and note the address.
stubs="$work/stubs"
mkdir -p "$stubs" "$work/wget-only"
cat > "$stubs/uname" <<'EOF'
#!/bin/sh
case "$1" in
  -s) printf '%s\n' "$FAKE_SYSTEM" ;;
  -m) printf '%s\n' "$FAKE_MACHINE" ;;
esac
EOF
cat > "$stubs/sysctl" <<'EOF'
#!/bin/sh
[ -n "${FAKE_TRANSLATED:-}" ] || exit 1
printf '%s\n' "$FAKE_TRANSLATED"
EOF
cat > "$stubs/curl" <<'EOF'
#!/bin/sh
out=""
while [ $# -gt 1 ]; do
  [ "$1" = "-o" ] && out="$2"
  shift
done
printf '%s\n' "$1" >> "$FAKE_LOG"
served="$FAKE_SERVED/${1#https://github.com/githubrobbi/md2html/releases/}"
[ -f "$served" ] || exit 22
cat "$served" > "$out"
EOF
cat > "$work/wget-only/wget" <<'EOF'
#!/bin/sh
printf 'wget %s\n' "$4" >> "$FAKE_LOG"
served="$FAKE_SERVED/${4#https://github.com/githubrobbi/md2html/releases/}"
[ -f "$served" ] || exit 8
cat "$served" > "$3"
EOF
cp "$stubs/uname" "$stubs/sysctl" "$work/wget-only/"
chmod +x "$stubs"/* "$work/wget-only"/*

sha() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# serve DIR VERSION: a release at DIR whose binaries print
# `md2html VERSION`, with its SHA256SUMS.
serve() {
  local dir="$1" version="$2" name
  mkdir -p "$dir"
  for name in md2html-macos-arm64 md2html-macos-x64 md2html-linux-x64; do
    printf '#!/bin/sh\nprintf "md2html %s (%s)\\n"\n' "$version" "$name" > "$dir/$name"
  done
  (cd "$dir" && for name in md2html-*; do printf '%s  ./%s\n' "$(sha "$name")" "$name"; done) > "$dir/SHA256SUMS"
}

served="$work/served"
serve "$served/latest/download" 0.9.0
serve "$served/download/v0.8.1" 0.8.1

# run SHELL [NAME=value ...]: the installer under SHELL, its PATH the
# stand-ins and the tools; output in $out, exit code in $status, the
# addresses asked for in $asked. The install folder is $dest, fresh.
run() {
  local shell="$1"
  shift
  dest="$work/dest"
  : > "$work/log"
  status=0
  out="$(env -i HOME="$work/home" PATH="$stubs:$tools" FAKE_LOG="$work/log" FAKE_SERVED="$served" \
    FAKE_SYSTEM=Darwin FAKE_MACHINE=arm64 MD2HTML_INSTALL_DIR="$dest" "$@" \
    "$shell" "$script" 2>&1)" || status=$?
  asked="$(cat "$work/log")"
}

# expect NAME CODE [+text|-text|=installed-version|!]...: the exit code,
# then each `+text` in the output (or the addresses asked), each `-text`
# in neither, `=text` what the installed md2html prints, `!` nothing
# installed and nothing left beside it.
expect() {
  local name="$1" code="$2" want problem="" seen
  shift 2
  count=$((count + 1))
  seen="$out"$'\n'"$asked"
  [[ "$status" == "$code" ]] || problem="exit $status, not $code"
  for want in "$@"; do
    case "$want" in
      +*) grep -qF -- "${want#+}" <<< "$seen" || problem="${problem:+$problem; }missing: ${want#+}" ;;
      -*) if grep -qF -- "${want#-}" <<< "$seen"; then problem="${problem:+$problem; }unwanted: ${want#-}"; fi ;;
      =*)
        if [[ ! -x "$dest/md2html" ]]; then
          problem="${problem:+$problem; }nothing installed"
        elif [[ "$("$dest/md2html")" != "${want#=}" ]]; then
          problem="${problem:+$problem; }installed: $("$dest/md2html")"
        fi
        ;;
      !)
        if [[ -e "$dest/md2html" ]]; then problem="${problem:+$problem; }an md2html was installed"; fi
        if compgen -G "$dest/.md2html.install.*" > /dev/null; then problem="${problem:+$problem; }a staged copy was left"; fi
        ;;
    esac
  done
  if [[ -n "$problem" ]]; then
    printf '  ✗ %s: %s\n' "$name" "$problem"
    printf '%s\n' "$out" | sed 's/^/      /'
    failed=$((failed + 1))
  else
    printf '  ✓ %s\n' "$name"
  fi
}

for shell in "${shells[@]}"; do
  printf '  under %s\n' "$shell"

  rm -rf "$work/dest"
  run "$shell"
  expect "Apple silicon gets the newest arm64 binary" 0 \
    "+https://github.com/githubrobbi/md2html/releases/latest/download/md2html-macos-arm64" \
    "+releases/latest/download/SHA256SUMS" "+SHA-256 checked" \
    "=md2html 0.9.0 (md2html-macos-arm64)" "+is not on your PATH" "+export PATH=\"$work/dest:\$PATH\""

  run "$shell" FAKE_MACHINE=x86_64 FAKE_TRANSLATED=1
  expect "a Rosetta shell on Apple silicon gets the arm64 binary" 0 "=md2html 0.9.0 (md2html-macos-arm64)"

  run "$shell" FAKE_MACHINE=x86_64
  expect "an Intel Mac gets the x64 binary" 0 "=md2html 0.9.0 (md2html-macos-x64)"

  run "$shell" FAKE_SYSTEM=Linux FAKE_MACHINE=x86_64 MD2HTML_VERSION=0.8.1
  expect "Linux x64 gets the version asked for, with or without its v" 0 \
    "+https://github.com/githubrobbi/md2html/releases/download/v0.8.1/md2html-linux-x64" \
    "=md2html 0.8.1 (md2html-linux-x64)" "-latest"

  run "$shell" FAKE_SYSTEM=Linux FAKE_MACHINE=x86_64 MD2HTML_VERSION=v0.8.1 PATH="$stubs:$tools:$work/dest"
  expect "an install replaces the one before it, and a folder on PATH needs no advice" 0 \
    "=md2html 0.8.1 (md2html-linux-x64)" "-is not on your PATH"

  rm -rf "$work/dest"
  run "$shell" FAKE_SYSTEM=Linux FAKE_MACHINE=aarch64
  expect "Linux on ARM has no build: refused, nothing asked for" 1 \
    "+no md2html build for Linux on aarch64" "+cargo install --git" "-downloading" "!"

  run "$shell" FAKE_SYSTEM=MINGW64_NT-10.0
  expect "Windows is sent to its ZIP" 1 "+md2html-windows-x64.zip" "-releases/latest/download/" "!"

  for bad in v1.2 1.2.3.4 '1.2.3;x' ../1.2.3 1..2 .1.2.3; do
    run "$shell" MD2HTML_VERSION="$bad"
    expect "MD2HTML_VERSION=$bad is refused before any download" 1 "+is not a version like v0.8.1" "-downloading" "!"
  done

  run "$shell" MD2HTML_VERSION=v7.7.7
  expect "a release that is not there is refused" 1 "+could not download" "!"

  cp -R "$served/latest" "$work/good"
  printf '#!/bin/sh\necho tampered\n' > "$served/latest/download/md2html-macos-arm64"
  run "$shell"
  expect "a binary with another SHA-256 than the list is refused" 1 "+but the release lists" "!"

  mkdir -p "$work/dest"
  printf '#!/bin/sh\necho old\n' > "$work/dest/md2html"
  chmod +x "$work/dest/md2html"
  run "$shell"
  expect "a refused download leaves the installed md2html as it was" 1 "=old"
  rm -rf "$work/dest"

  grep -v macos-arm64 "$work/good/download/SHA256SUMS" > "$served/latest/download/SHA256SUMS"
  cp "$work/good/download/md2html-macos-arm64" "$served/latest/download/"
  run "$shell"
  expect "a binary the list does not hold is refused" 1 "+does not list md2html-macos-arm64" "!"

  printf '#!/bin/sh\necho "cannot execute binary file" >&2\nexit 126\n' > "$served/latest/download/md2html-macos-arm64"
  (cd "$served/latest/download" && printf '%s  ./md2html-macos-arm64\n' "$(sha md2html-macos-arm64)") >> "$served/latest/download/SHA256SUMS"
  run "$shell"
  expect "a binary that does not run here is refused and removed" 1 \
    "+does not run here: cannot execute binary file" "!"
  rm -rf "$served/latest" "$work/good"
  serve "$served/latest/download" 0.9.0

  run "$shell" PATH="$work/wget-only:$tools"
  expect "wget serves when there is no curl" 0 \
    "+wget https://github.com/githubrobbi/md2html/releases/latest/download/md2html-macos-arm64" \
    "=md2html 0.9.0 (md2html-macos-arm64)"

  rm -rf "${work:?}/dest" "${work:?}/home"
  run "$shell" MD2HTML_INSTALL_DIR=
  expect "the folder is ~/.local/bin unless one is named" 0 "+as $work/home/.local/bin/md2html"
  rm -rf "${work:?}/home"
done

if (( failed > 0 )); then
  printf '❌ %d of %d install cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ %d install cases passed\n' "$count"
