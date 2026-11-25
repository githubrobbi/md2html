#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# The tests of scripts/release/winget_manifests.sh, run by the gate
# `gate-tests`.  Each case gives a `gh` of its own (first on the PATH,
# answering from the case's files) a published release with a Windows
# archive and its SHA256SUMS, runs the script, and checks the three
# manifests it writes for the first winget-pkgs submission, or that it
# refuses: a draft, a digest that does not match, an archive without the
# binary.  Nothing reaches GitHub.  Exit 1 when a case fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/release/winget_manifests.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

failed=0
count=0

mkdir -p "$scratch/bin"
cat > "$scratch/bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  "release view") [[ -f "$FIX/state" ]] || { printf 'release not found\n' >&2; exit 1; }; cat "$FIX/state" ;;
  "release download")
    dir=""
    while (( $# > 0 )); do [[ "$1" == "--dir" ]] && dir="$2"; shift; done
    cp "$FIX/release"/* "$dir/" ;;
  *) printf 'gh stand-in: no answer for %s %s\n' "$1" "$2" >&2; exit 99 ;;
esac
GH
chmod +x "$scratch/bin/gh"

sha() {
  if command -v sha256sum >/dev/null; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

case_dir=""
out=""
code=0
name=""
start() {
  count=$((count + 1))
  name="$1"
  case_dir="$scratch/case-$count"
  mkdir -p "$case_dir/release" "$case_dir/stage/md2html-windows-x64"
  printf 'MZ a windows binary\n' > "$case_dir/stage/md2html-windows-x64/md2html.exe"
  printf 'notice\n' > "$case_dir/stage/md2html-windows-x64/NOTICE"
  (cd "$case_dir/stage" && zip -qr "$case_dir/release/md2html-windows-x64.zip" md2html-windows-x64)
  printf '%s  ./md2html-windows-x64.zip\n%s  ./other\n' "$(sha "$case_dir/release/md2html-windows-x64.zip")" "$(printf '0%.0s' {1..64})" > "$case_dir/release/SHA256SUMS"
  printf 'false 2026-12-01T17:04:05Z\n' > "$case_dir/state"
}
run() {
  code=0
  out="$(FIX="$case_dir" PATH="$scratch/bin:$PATH" "$BASH" "$script" "$@" 2>&1)" || code=$?
}
bad() {
  printf '❌ %s: %s\n' "$name" "$1"
  printf '%s\n' "$out" | sed 's/^/   | /'
  failed=1
}
expect_code() { [[ "$code" == "$1" ]] || bad "exit $code, expected $1"; }
expect_out() { [[ "$out" == *"$1"* ]] || bad "the output lacks: $1"; }
expect_line() { grep -qxF -- "$2" "$1" || bad "${1##*/} lacks the line: $2"; }

start "the three manifests of a published release"
run v0.8.0 "$case_dir/out"
expect_code 0
dest="$case_dir/out/manifests/s/SkyLLC/md2html/0.8.0"
digest="$(sha "$case_dir/release/md2html-windows-x64.zip" | tr 'a-f' 'A-F')"
for file in SkyLLC.md2html.yaml SkyLLC.md2html.installer.yaml SkyLLC.md2html.locale.en-US.yaml; do
  [[ -f "$dest/$file" ]] || { bad "no $file"; continue; }
  expect_line "$dest/$file" "PackageIdentifier: SkyLLC.md2html"
  expect_line "$dest/$file" "PackageVersion: 0.8.0"
  [[ "$(head -n1 "$dest/$file")" == "# yaml-language-server: "* ]] || bad "$file does not start with its schema line"
  if grep -q 'SPDX\|Copyright (c)\|@[A-Z0-9_]*@' "$dest/$file"; then bad "$file keeps a licence line or a placeholder"; fi
done
expect_line "$dest/SkyLLC.md2html.installer.yaml" "  InstallerSha256: $digest"
expect_line "$dest/SkyLLC.md2html.installer.yaml" "  InstallerUrl: https://github.com/githubrobbi/md2html/releases/download/v0.8.0/md2html-windows-x64.zip"
expect_line "$dest/SkyLLC.md2html.installer.yaml" '- RelativeFilePath: md2html-windows-x64\md2html.exe'
expect_line "$dest/SkyLLC.md2html.installer.yaml" "  PortableCommandAlias: md2html"
expect_line "$dest/SkyLLC.md2html.installer.yaml" "ReleaseDate: 2026-12-01"
expect_line "$dest/SkyLLC.md2html.locale.en-US.yaml" "License: (MIT OR Apache-2.0) AND BSD-3-Clause"
expect_line "$dest/SkyLLC.md2html.locale.en-US.yaml" "Publisher: Sky, LLC"

start "a draft is refused"
printf 'true 0001-01-01T00:00:00Z\n' > "$case_dir/state"
run v0.8.0 "$case_dir/out"
expect_code 1
expect_out "is still a draft"

start "no release is refused"
rm "$case_dir/state"
run v0.8.0 "$case_dir/out"
expect_code 1
expect_out "has no release v0.8.0"

start "an archive that is not the one SHA256SUMS lists"
printf 'changed\n' >> "$case_dir/release/md2html-windows-x64.zip"
run v0.8.0 "$case_dir/out"
expect_code 1
expect_out "its SHA256SUMS says"

start "SHA256SUMS without the archive"
printf '%s  ./something-else.zip\n' "$(sha "$case_dir/release/md2html-windows-x64.zip")" > "$case_dir/release/SHA256SUMS"
run v0.8.0 "$case_dir/out"
expect_code 1
expect_out "has no line for md2html-windows-x64.zip"

start "an archive without the binary the manifest names"
rm "$case_dir/release/md2html-windows-x64.zip" "$case_dir/stage/md2html-windows-x64/md2html.exe"
(cd "$case_dir/stage" && zip -qr "$case_dir/release/md2html-windows-x64.zip" md2html-windows-x64)
printf '%s  ./md2html-windows-x64.zip\n' "$(sha "$case_dir/release/md2html-windows-x64.zip")" > "$case_dir/release/SHA256SUMS"
run v0.8.0 "$case_dir/out"
expect_code 1
expect_out "holds no md2html-windows-x64/md2html.exe"

start "not a release tag"
run 0.8.0 "$case_dir/out"
expect_code 1
expect_out "is not a release tag"

if (( failed )); then
  printf '❌ winget_manifests.sh: a case failed (%d cases)\n' "$count"
  exit 1
fi
printf '✅ winget_manifests.sh: %d cases\n' "$count"
