#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# md2html's installer for macOS and Linux:
#
#   curl -fsSL https://github.com/githubrobbi/md2html/releases/latest/download/install.sh | sh
#
# It picks the release's binary for this system (macOS on Apple silicon
# or Intel, Linux on x64), downloads it with the release's SHA256SUMS,
# refuses it unless its SHA-256 is the one listed, and puts it in
# ~/.local/bin as `md2html`, without sudo. Running it again installs the
# newest release over the old one.
#
#   MD2HTML_VERSION      the release to install, `vX.Y.Z` or `X.Y.Z`
#                        (default: the newest)
#   MD2HTML_INSTALL_DIR  the folder to install into (default:
#                        ~/.local/bin)
#
#   curl -fsSL https://github.com/githubrobbi/md2html/releases/latest/download/install.sh | MD2HTML_VERSION=v0.8.1 sh
#
# Every release carries this script among its assets, with the build
# provenance of every other asset (`gh attestation verify install.sh
# --owner githubrobbi`). To read it before running it: download it, read
# it, then `sh install.sh`. Windows: the ZIP of the release, or winget.
#
# Exit 0 when md2html is installed and runs, 1 on any refusal.
#
# The whole script is one function called on the last line, so a
# download cut short runs nothing.

set -eu

repo="githubrobbi/md2html"

say() {
  printf 'md2html install: %s\n' "$1"
}

refuse() {
  printf 'md2html install: %s\n' "$1" >&2
  exit 1
}

# The asset name of the binary for this system, on standard output.
asset_for_system() {
  system="$(uname -s)"
  machine="$(uname -m)"
  case "$system" in
    Darwin)
      # A shell under Rosetta reports x86_64 on an Apple silicon Mac:
      # the native binary is the one to install there.
      # sysctl has no such name on an Intel Mac: that is no translation.
      translated="$(sysctl -n sysctl.proc_translated 2>/dev/null)" || translated=0
      if [ "$machine" = "x86_64" ] && [ "$translated" = "1" ]; then
        machine="arm64"
      fi
      case "$machine" in
        arm64 | aarch64) printf 'md2html-macos-arm64\n' ;;
        x86_64 | amd64) printf 'md2html-macos-x64\n' ;;
        *) refuse "no md2html build for macOS on $machine" ;;
      esac
      ;;
    Linux)
      case "$machine" in
        x86_64 | amd64) printf 'md2html-linux-x64\n' ;;
        *) refuse "no md2html build for Linux on $machine; with Rust, build it: cargo install --git https://github.com/$repo md2html --locked" ;;
      esac
      ;;
    MINGW* | MSYS* | CYGWIN* | Windows_NT)
      refuse "on Windows, download md2html-windows-x64.zip from https://github.com/$repo/releases/latest"
      ;;
    *)
      refuse "no md2html build for $system; with Rust, build it: cargo install --git https://github.com/$repo md2html --locked"
      ;;
  esac
}

# The release's download address: the newest, or the one MD2HTML_VERSION
# names (checked to be a version, so nothing else reaches the address).
release_base() {
  wanted="${MD2HTML_VERSION:-}"
  if [ -z "$wanted" ]; then
    printf 'https://github.com/%s/releases/latest/download\n' "$repo"
    return
  fi
  number="${wanted#v}"
  case "$number" in
    *[!0-9.]* | .* | *. | *..*) refuse "MD2HTML_VERSION=$wanted is not a version like v0.8.1" ;;
  esac
  case "$number" in
    *.*.*.*) refuse "MD2HTML_VERSION=$wanted is not a version like v0.8.1" ;;
    *.*.*) ;;
    *) refuse "MD2HTML_VERSION=$wanted is not a version like v0.8.1" ;;
  esac
  printf 'https://github.com/%s/releases/download/v%s\n' "$repo" "$number"
}

# fetch URL FILE: the file at URL into FILE, with curl or wget.
fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    refuse "curl or wget is needed to download md2html"
  fi
}

# The SHA-256 of a file, on standard output.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    refuse "sha256sum or shasum is needed to check the download"
  fi
}

main() {
  [ -n "${HOME:-}" ] || [ -n "${MD2HTML_INSTALL_DIR:-}" ] \
    || refuse "HOME is not set: name the folder with MD2HTML_INSTALL_DIR"
  dir="${MD2HTML_INSTALL_DIR:-$HOME/.local/bin}"
  asset="$(asset_for_system)"
  base="$(release_base)"

  work="$(mktemp -d 2>/dev/null || mktemp -d -t md2html)"
  # The staged copy beside the install folder goes too when a run stops
  # before it is renamed into place.
  staged="$work/nothing-staged"
  trap 'rm -rf "$work" "$staged"' EXIT
  trap 'exit 1' HUP INT TERM

  say "downloading $asset from $base"
  fetch "$base/$asset" "$work/$asset" || refuse "could not download $base/$asset"
  fetch "$base/SHA256SUMS" "$work/SHA256SUMS" || refuse "could not download $base/SHA256SUMS"

  # The list's lines read `<sha256>  ./<name>`.
  listed="$(awk -v name="./$asset" '$2 == name { print $1; exit }' "$work/SHA256SUMS")"
  [ -n "$listed" ] || refuse "the release's SHA256SUMS does not list $asset"
  actual="$(sha256_of "$work/$asset")"
  [ "$actual" = "$listed" ] \
    || refuse "$asset has SHA-256 $actual, but the release lists $listed: nothing was installed"
  say "SHA-256 checked: $actual"

  mkdir -p "$dir" || refuse "could not make the folder $dir"
  # Copied beside its place, then renamed over it: a running md2html, or
  # an install stopped half-way, never sees half a binary.
  staged="$dir/.md2html.install.$$"
  cp "$work/$asset" "$staged" || refuse "could not write into $dir"
  chmod 755 "$staged"
  if ! version="$("$staged" --version 2>&1)"; then
    rm -f "$staged"
    refuse "the downloaded md2html does not run here: $version"
  fi
  mv -f "$staged" "$dir/md2html" || refuse "could not put md2html into $dir"
  say "installed $version as $dir/md2html"

  case ":${PATH:-}:" in
    *":$dir:"*)
      if found="$(command -v md2html)" && [ "$found" != "$dir/md2html" ]; then
        say "note: $found comes first on your PATH, so \`md2html\` still runs that one"
      fi
      ;;
    *)
      say "$dir is not on your PATH: add the line"
      say "  export PATH=\"$dir:\$PATH\""
      say "to your shell's startup file (~/.zshrc, ~/.bashrc), then open a new terminal"
      ;;
  esac
}

main "$@"
