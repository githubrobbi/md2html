#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# The tests of scripts/release/sign_macos.sh (`just release-sign-macos`),
# pack_zip.sh and checksums.sh, run by the gate `gate-tests`.  Each case
# builds a draft release the way release.yml builds one (pack_zip.sh for
# the archives, checksums.sh for the lists, every asset attested), puts
# its own `gh`, `codesign`, `xcrun`, `security`, `spctl`, `ditto`,
# `gopass` and `uname` first on the PATH (they answer from the case's
# files and write down every call), runs the script, and checks what it
# did: every refusal stops it before anything is signed or uploaded, a
# failed notarization leaves the draft a draft and a second run finishes
# it, and a run that passes replaces exactly the six macOS assets and
# lists, publishes, and dispatches the lanes that are on.  No Apple tool,
# key or keychain and nothing on GitHub is touched.  Exit 1 when a case
# fails.

set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
script="$repo/scripts/release/sign_macos.sh"
pack="$repo/scripts/release/pack_zip.sh"
sums="$repo/scripts/release/checksums.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

failed=0
count=0
tag=v0.8.0
commit=1111111111111111111111111111111111111111
commit_date=2026-11-24T10:00:01Z
epoch="$(python3 -c 'import sys, datetime; print(int(datetime.datetime.strptime(sys.argv[1], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc).timestamp()))' "$commit_date")"
hash1=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
hash2=BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB
ident="Developer ID Application: Sky, LLC (TEAM123456)"

sha() {
  if command -v sha256sum >/dev/null; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# ─── The stand-ins ────────────────────────────────────────────────────
bin="$scratch/bin"
mkdir -p "$bin"

cat > "$bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
printf 'gh %s\n' "$*" >> "$CALLS"
sub="$1 $2"
shift 2
# The flags every call may carry, and the rest in order.
args=()
patterns=()
dir=""
source_ref=""
source_digest=""
signer=""
attest_repo=""
while (( $# > 0 )); do
  case "$1" in
    --repo|-R) attest_repo="$2"; shift ;;
    --jq|-q|--json|-H|--ref) shift ;;
    --dir) dir="$2"; shift ;;
    --pattern) patterns+=("$2"); shift ;;
    --signer-workflow) signer="$2"; shift ;;
    --source-ref) source_ref="$2"; shift ;;
    --source-digest) source_digest="$2"; shift ;;
    -f) shift ;;
    --clobber|--draft=false) ;;
    *) args+=("$1") ;;
  esac
  shift
done
r="$(cat "$FIX/repo")"
need() { [[ -f "$FIX/$1" ]] || { printf 'HTTP 404: Not Found\n' >&2; exit 1; }; cat "$FIX/$1"; }
case "$sub" in
  "repo view")
    if [[ " ${args[*]:-} " == *" $r "* ]]; then printf 'main\n'; else printf '%s\n' "$r"; fi ;;
  "variable list") cat "$FIX/vars" ;;
  "api repos/$r/git/ref/tags/v0.8.0") need ref ;;
  "api repos/$r/git/tags/"*) need tagobj ;;
  "api repos/$r/commits/"*) need commit-date ;;
  "api repos/$r/contents/"*) need entitlements.plist ;;
  "release view") need isdraft ;;
  "release download")
    mkdir -p "$dir"
    for path in "$FIX/release"/*; do
      name="${path##*/}"
      if (( ${#patterns[@]} > 0 )); then
        keep=0
        for p in "${patterns[@]}"; do [[ "$p" == "$name" ]] && keep=1; done
        (( keep )) || continue
      fi
      cp "$path" "$dir/$name"
    done ;;
  "release upload")
    [[ ! -f "$FIX/refuse-upload" ]] || { printf 'HTTP 502\n' >&2; exit 1; }
    for path in "${args[@]:1}"; do cp "$path" "$FIX/release/${path##*/}"; done
    # An upload that stores other bytes than it was given, for the case
    # that must read the draft back.
    if [[ -f "$FIX/upload-corrupts" ]]; then printf 'x' >> "$FIX/release/CHECKSUMS.txt"; fi ;;
  "release edit")
    [[ ! -f "$FIX/refuse-publish" ]] || { printf 'HTTP 502\n' >&2; exit 1; }
    [[ -f "$FIX/publish-changes-nothing" ]] || printf 'false\n' > "$FIX/isdraft" ;;
  "attestation verify")
    file="${args[0]}"
    [[ "$attest_repo" == "$r" && "$signer" == "$r/.github/workflows/release.yml" ]] || { printf 'wrong identity\n' >&2; exit 1; }
    [[ "$source_ref" == "$(cat "$FIX/attest-ref")" ]] || { printf 'source ref differs\n' >&2; exit 1; }
    [[ -z "$source_digest" || "$source_digest" == "$(cat "$FIX/commit")" ]] || { printf 'source digest differs\n' >&2; exit 1; }
    if command -v sha256sum >/dev/null; then read -r h _ < <(sha256sum "$file"); else read -r h _ < <(shasum -a 256 "$file"); fi
    grep -qx "$h" "$FIX/attested" || { printf 'no attestation for %s\n' "$file" >&2; exit 1; } ;;
  "workflow run") ;;
  *) printf 'gh stand-in: no answer for %s\n' "$sub" >&2; exit 99 ;;
esac
GH

cat > "$bin/codesign" <<'CS'
#!/usr/bin/env bash
set -euo pipefail
printf 'codesign %s\n' "$*" >> "$CALLS"
file="${*: -1}"
case "$1" in
  --force)
    identity=""
    identifier=""
    while (( $# > 1 )); do
      case "$1" in --sign) identity="$2" ;; --identifier) identifier="$2" ;; esac
      shift
    done
    # A codesign that answers success and changes nothing, for the case
    # that must notice a binary left as it was.
    [[ -f "$FIX/codesign-changes-nothing" ]] || printf '\nSIGNED %s %s\n' "$identity" "$identifier" >> "$file" ;;
  --verify) [[ -f "$FIX/codesign-changes-nothing" ]] || grep -q '^SIGNED ' "$file" ;;
  -dvv)
    identifier=org.githubrobbi.md2html
    if [[ ! -f "$FIX/codesign-changes-nothing" ]]; then
      grep -q '^SIGNED ' "$file" || { printf '%s: code object is not signed at all\n' "$file" >&2; exit 1; }
      identifier="$(sed -n 's/^SIGNED [^ ]* //p' "$file")"
    fi
    runtime="flags=0x10000(runtime)"
    [[ ! -f "$FIX/no-runtime" ]] || runtime="flags=0x2(adhoc)"
    printf 'Identifier=%s\nCodeDirectory v=20500 size=1 %s hashes=1\nAuthority=%s\nTimestamp=24 Nov 2026\n' "$identifier" "$runtime" "$(cat "$FIX/ident-name")" >&2 ;;
  *) exit 99 ;;
esac
CS

cat > "$bin/xcrun" <<'XC'
#!/usr/bin/env bash
set -euo pipefail
printf 'xcrun %s\n' "$*" >> "$CALLS"
case "$2" in
  history) [[ -f "$FIX/profile-ok" ]] || { printf 'No Keychain password item found for profile\n' >&2; exit 69; } ;;
  submit) cat "$FIX/notary-answer" ;;
  log) printf '{"issues": [{"message": "The binary is not signed."}]}\n' ;;
  *) exit 99 ;;
esac
XC

cat > "$bin/spctl" <<'SP'
#!/usr/bin/env bash
set -euo pipefail
printf 'spctl %s\n' "$*" >> "$CALLS"
if [[ -f "$FIX/gatekeeper-unnotarized" ]]; then
  printf '%s: accepted\nsource=Developer ID\n' "${*: -1}" >&2
  exit 0
fi
if [[ -f "$FIX/gatekeeper-refuses" ]]; then
  printf '%s: rejected\nsource=Unnotarized Developer ID\n' "${*: -1}" >&2
  exit 3
fi
printf '%s: accepted\nsource=Notarized Developer ID\n' "${*: -1}" >&2
SP

cat > "$bin/security" <<'SE'
#!/usr/bin/env bash
set -euo pipefail
printf 'security %s\n' "$*" >> "$CALLS"
cat "$FIX/identities"
SE

cat > "$bin/gopass" <<'GO'
#!/usr/bin/env bash
set -euo pipefail
printf 'gopass %s\n' "$*" >> "$CALLS"
case "$1" in
  ls) if [[ -f "$FIX/gopass-identity" ]]; then printf 'keys/apple/signing-identity\n'; fi ;;
  show) cat "$FIX/gopass-identity" ;;
  *) exit 99 ;;
esac
GO

cat > "$bin/ditto" <<'DI'
#!/usr/bin/env bash
set -euo pipefail
printf 'ditto %s\n' "$*" >> "$CALLS"
src="${*: -2:1}"
dst="${*: -1}"
(cd "$(dirname "$src")" && zip -qr "$dst" "$(basename "$src")")
DI

cat > "$bin/uname" <<'UN'
#!/usr/bin/env bash
cat "$FIX/uname"
UN
chmod +x "$bin"/*

# ─── The draft release, as release.yml makes it ───────────────────────
base="$scratch/base"
mkdir -p "$base/release" "$base/stage"
for platform in macos-arm64 macos-x64 linux-x64; do
  folder="$base/stage/md2html-$platform"
  mkdir -p "$folder"
  printf 'the md2html binary for %s\n' "$platform" > "$folder/md2html"
  chmod +x "$folder/md2html"
  for doc in LICENSE-MIT LICENSE-APACHE NOTICE README.md CHANGELOG.md; do
    printf '%s of md2html\n' "$doc" > "$folder/$doc"
  done
  cp "$folder/md2html" "$base/release/md2html-$platform"
  bash "$pack" "$folder" "$base/release/md2html-$platform.zip" "$epoch"
done
printf 'a windows zip\n' > "$base/release/md2html-windows-x64.zip"
printf 'a windows binary\n' > "$base/release/md2html-windows-x64.exe"
printf '{"bomFormat": "CycloneDX"}\n' > "$base/release/sbom-md2html.cdx.json"
bash "$sums" "$base/release"
for path in "$base/release"/*; do sha "$path"; done > "$base/attested"
printf 'githubrobbi/md2html\n' > "$base/repo"
printf 'LANE_RELEASE=true\nLANE_CODESIGN=true\nLANE_SLSA=true\nLANE_BREW=true\nLANE_WINGET=true\nLANE_CRATES=false\n' > "$base/vars"
printf 'tag 2222222222222222222222222222222222222222\n' > "$base/ref"
printf '%s true valid commit %s\n' "$tag" "$commit" > "$base/tagobj"
printf '%s\n' "$commit_date" > "$base/commit-date"
printf '%s\n' "$commit" > "$base/commit"
printf 'refs/tags/%s\n' "$tag" > "$base/attest-ref"
printf '<?xml version="1.0"?>\n<plist version="1.0"><dict/></plist>\n' > "$base/entitlements.plist"
printf 'true\n' > "$base/isdraft"
printf '  1) %s "%s"\n     1 valid identities found\n' "$hash1" "$ident" > "$base/identities"
printf '%s\n' "$ident" > "$base/ident-name"
touch "$base/profile-ok"
printf '{"id": "sub-1", "status": "Accepted", "message": "Processing complete"}\n' > "$base/notary-answer"
printf 'Darwin\n' > "$base/uname"
rm -rf "$base/stage"

# ─── The harness ──────────────────────────────────────────────────────
case_dir=""
out=""
code=0
start() {
  count=$((count + 1))
  name="$1"
  case_dir="$scratch/case-$count"
  cp -R "$base" "$case_dir"
  : > "$case_dir/calls"
}
run() {
  code=0
  : > "$case_dir/calls"
  out="$(FIX="$case_dir" CALLS="$case_dir/calls" PATH="$bin:$PATH" "$BASH" "$script" "$@" --workdir "$case_dir/work" 2>&1)" || code=$?
}
bad() {
  printf '❌ %s: %s\n' "$name" "$1"
  printf '%s\n' "$out" | sed 's/^/   | /'
  failed=1
}
expect_code() { [[ "$code" == "$1" ]] || bad "exit $code, expected $1"; }
expect_out() { [[ "$out" == *"$1"* ]] || bad "the output lacks: $1"; }
expect_no_out() { if [[ "$out" == *"$1"* ]]; then bad "the output has: $1"; fi; }
expect_call() { grep -qF -- "$1" "$case_dir/calls" || bad "no call: $1"; }
expect_no_call() { if grep -qF -- "$1" "$case_dir/calls"; then bad "an unwanted call: $1"; fi; }
# A refusal: exit 1 with the reason, and nothing signed, sent, published.
expect_refused() {
  expect_code 1
  expect_out "$1"
  expect_no_call "codesign --force"
  expect_no_call "notarytool submit"
  expect_no_call "gh release upload"
  expect_no_call "gh release edit"
  expect_no_call "gh workflow run"
  if [[ -f "$case_dir/isdraft" && "$(cat "$case_dir/isdraft")" == "false" ]]; then
    bad "the release was published"
  fi
}
# A run that passed: the six files replaced, the rest untouched, the
# lists right, the release published, the lanes that are on dispatched.
expect_published() {
  expect_code 0
  expect_out "is published"
  local asset
  for asset in md2html-linux-x64 md2html-linux-x64.zip md2html-windows-x64.exe md2html-windows-x64.zip sbom-md2html.cdx.json; do
    cmp -s "$case_dir/release/$asset" "$base/release/$asset" || bad "$asset was changed"
  done
  for asset in md2html-macos-arm64 md2html-macos-x64; do
    grep -q "^SIGNED $hash1 org.githubrobbi.md2html\$" "$case_dir/release/$asset" || bad "$asset is not the signed binary"
  done
  local check="$case_dir/verify"
  rm -rf "$check"
  mkdir -p "$check"
  for arch in arm64 x64; do
    (cd "$check" && unzip -q "$case_dir/release/md2html-macos-$arch.zip")
    cmp -s "$check/md2html-macos-$arch/md2html" "$case_dir/release/md2html-macos-$arch" || bad "the archive of $arch holds another binary than its raw asset"
    for doc in LICENSE-MIT LICENSE-APACHE NOTICE README.md CHANGELOG.md; do
      cmp -s "$check/md2html-macos-$arch/$doc" <(printf '%s of md2html\n' "$doc") || bad "the archive of $arch lost $doc"
    done
    [[ "$(unzip -Z1 "$case_dir/release/md2html-macos-$arch.zip" | wc -l | tr -d ' ')" == 7 ]] || bad "the archive of $arch holds other entries"
  done
  if command -v sha256sum > /dev/null; then
    (cd "$case_dir/release" && sha256sum -c --status CHECKSUMS.txt) || bad "CHECKSUMS.txt does not match the published files"
  else
    (cd "$case_dir/release" && shasum -a 256 -c --status CHECKSUMS.txt) || bad "CHECKSUMS.txt does not match the published files"
  fi
  cmp -s "$case_dir/release/CHECKSUMS.txt" "$case_dir/release/SHA256SUMS" || bad "SHA256SUMS is not CHECKSUMS.txt"
  local changed
  changed="$(comm -13 <(LC_ALL=C sort "$base/release/CHECKSUMS.txt") <(LC_ALL=C sort "$case_dir/release/CHECKSUMS.txt") | sed 's/^[0-9a-f]*  //' | LC_ALL=C sort | tr '\n' ' ')"
  [[ "$changed" == "./md2html-macos-arm64 ./md2html-macos-arm64.zip ./md2html-macos-x64 ./md2html-macos-x64.zip " ]] || bad "CHECKSUMS.txt changed in: $changed"
  [[ "$(cat "$case_dir/isdraft")" == "false" ]] || bad "the release is still a draft"
  # The order: upload, then publish, then the dispatches.
  local upload publish brew
  upload="$(awk '/^gh release upload/ { n = NR } END { print n }' "$case_dir/calls")"
  publish="$(awk '/^gh release edit/ { n = NR } END { print n }' "$case_dir/calls")"
  brew="$(awk '/^gh workflow run brew-publish.yml/ { n = NR } END { print n }' "$case_dir/calls")"
  if [[ -z "$upload" || -z "$publish" || -z "$brew" ]] || (( upload > publish || publish > brew )); then
    bad "not upload, publish, dispatch in that order"
  fi
  expect_call "gh workflow run winget-publish.yml --repo githubrobbi/md2html --ref main -f release-tag=$tag"
  expect_no_call "release-plz.yml"
  expect_call "xcrun notarytool submit"
}

# ─── pack_zip.sh and checksums.sh ─────────────────────────────────────
start "pack_zip.sh packs the same bytes from the same files"
mkdir -p "$case_dir/a/md2html-macos-arm64" "$case_dir/b"
printf 'bin\n' > "$case_dir/a/md2html-macos-arm64/md2html"
chmod 0700 "$case_dir/a/md2html-macos-arm64/md2html"
printf 'notice\n' > "$case_dir/a/md2html-macos-arm64/NOTICE"
bash "$pack" "$case_dir/a/md2html-macos-arm64" "$case_dir/one.zip" "$epoch"
(cd "$case_dir/b" && TZ=Pacific/Kiritimati unzip -q "$case_dir/one.zip")
touch "$case_dir/b/md2html-macos-arm64/NOTICE"
chmod 0600 "$case_dir/b/md2html-macos-arm64/NOTICE"
bash "$pack" "$case_dir/b/md2html-macos-arm64" "$case_dir/two.zip" "$epoch"
cmp -s "$case_dir/one.zip" "$case_dir/two.zip" || { out="$(unzip -Z "$case_dir/one.zip"; unzip -Z "$case_dir/two.zip")"; bad "unpacked and packed again, the archive differs"; }
for doc in README.md LICENSE-MIT CHANGELOG.md LICENSE-APACHE a-z z-a; do printf '%s\n' "$doc" > "$case_dir/a/md2html-macos-arm64/$doc"; done
bash "$pack" "$case_dir/a/md2html-macos-arm64" "$case_dir/four.zip" "$epoch"
out="$(unzip -Z1 "$case_dir/four.zip")"
[[ "$out" == "md2html-macos-arm64/"$'\n'"$(cd "$case_dir/a" && find md2html-macos-arm64 -type f | LC_ALL=C sort)" ]] || bad "the entries are not the folder, then its files in byte order"
# No extra field in any entry: no owner, no second time stamp.
out="$(python3 -c 'import sys, zipfile; print(" ".join(i.filename for i in zipfile.ZipFile(sys.argv[1]).infolist() if i.extra))' "$case_dir/four.zip")"
[[ -z "$out" ]] || bad "entries carry extra fields: $out"
mkdir -p "$case_dir/a/md2html-macos-arm64/sub"
code=0
out="$(bash "$pack" "$case_dir/a/md2html-macos-arm64" "$case_dir/three.zip" "$epoch" 2>&1)" || code=$?
expect_code 1
expect_out "is not a regular file"

start "checksums.sh lists every file but the two lists, as sha256sum ./* does"
mkdir -p "$case_dir/d"
printf 'x\n' > "$case_dir/d/b"
printf 'y\n' > "$case_dir/d/a.zip"
printf 'old\n' > "$case_dir/d/CHECKSUMS.txt"
bash "$sums" "$case_dir/d"
out="$(cat "$case_dir/d/CHECKSUMS.txt")"
[[ "$out" == "$(sha "$case_dir/d/a.zip")  ./a.zip"$'\n'"$(sha "$case_dir/d/b")  ./b" ]] || bad "the list is not sha256sum's, sorted"
cmp -s "$case_dir/d/CHECKSUMS.txt" "$case_dir/d/SHA256SUMS" || bad "SHA256SUMS is not CHECKSUMS.txt"

# ─── The refusals ─────────────────────────────────────────────────────
start "not macOS"
printf 'Linux\n' > "$case_dir/uname"
run "$tag"
expect_refused "runs on macOS only"
[[ ! -s "$case_dir/calls" ]] || bad "it called a tool before refusing"

start "not a release tag"
run 0.8.0
expect_refused "is not a release tag"

start "lane:codesign off"
sed -i.old 's/^LANE_CODESIGN=true$/LANE_CODESIGN=false/' "$case_dir/vars"
run "$tag"
expect_refused "LANE_CODESIGN is not true"
expect_no_call "gh release download"

start "lane:slsa off (no provenance to check)"
grep -v '^LANE_SLSA=' "$case_dir/vars" > "$case_dir/vars.new" && mv "$case_dir/vars.new" "$case_dir/vars"
run "$tag"
expect_refused "LANE_SLSA is not true"

start "a lightweight tag"
printf 'commit %s\n' "$commit" > "$case_dir/ref"
run "$tag"
expect_refused "is a lightweight tag"

start "a tag GitHub does not verify"
printf '%s false unsigned commit %s\n' "$tag" "$commit" > "$case_dir/tagobj"
run "$tag"
expect_refused "does not verify the signature"

start "no release for the tag"
rm "$case_dir/isdraft"
run "$tag"
expect_refused "has no release $tag"

start "the release is published already"
printf 'false\n' > "$case_dir/isdraft"
run "$tag"
expect_code 1
expect_out "is published already"
expect_out "gh workflow run brew-publish.yml"
expect_no_call "gh release download"
expect_no_call "codesign --force"
expect_no_call "gh workflow run"

start "no Developer ID identity in the keychain"
printf '     0 valid identities found\n' > "$case_dir/identities"
run "$tag"
expect_refused "not one"

start "two Developer ID identities and none named"
printf '  1) %s "%s"\n  2) %s "%s"\n' "$hash1" "$ident" "$hash2" "$ident" > "$case_dir/identities"
run "$tag"
expect_refused "match 2 valid code-signing identities"

start "two of the same name, the one named by its SHA-1 in gopass"
printf '  1) %s "%s"\n  2) %s "%s"\n' "$hash2" "$ident" "$hash1" "$ident" > "$case_dir/identities"
printf '%s\n' "$hash1" > "$case_dir/gopass-identity"
rm "$case_dir/profile-ok" # stop at the next check: the identity is what this case is about
run "$tag"
expect_refused "notary profile"
expect_out "signing with $ident ($hash1)"

start "no notary profile"
rm "$case_dir/profile-ok"
run "$tag"
expect_refused "notary profile 'md2html-notary'"

start "an asset that is not what CHECKSUMS.txt lists"
printf 'tampered\n' >> "$case_dir/release/md2html-linux-x64"
run "$tag"
expect_refused "are not the ones its CHECKSUMS.txt lists"
expect_out "still a draft"

start "an asset without release.yml's provenance"
grep -vx "$(sha "$case_dir/release/md2html-macos-x64.zip")" "$case_dir/attested" > "$case_dir/attested.new"
mv "$case_dir/attested.new" "$case_dir/attested"
run "$tag"
expect_refused "md2html-macos-x64.zip carries no build provenance"

start "provenance of another commit"
printf '3333333333333333333333333333333333333333\n' > "$case_dir/commit"
run "$tag"
expect_refused "carries no build provenance"

start "an archive release.yml did not pack with pack_zip.sh"
stage="$case_dir/oldzip"
mkdir -p "$stage"
(cd "$stage" && unzip -q "$case_dir/release/md2html-macos-arm64.zip")
rm "$case_dir/release/md2html-macos-arm64.zip"
(cd "$stage" && zip -rq "$case_dir/release/md2html-macos-arm64.zip" md2html-macos-arm64)
bash "$sums" "$case_dir/release"
for path in "$case_dir/release"/*; do sha "$path"; done > "$case_dir/attested"
run "$tag"
expect_refused "does not write the md2html-macos-arm64.zip release.yml wrote"

# ─── Runs that pass, and runs again ───────────────────────────────────
start "a draft signed, notarized, packed again and published"
run "$tag"
expect_published
expect_no_out "built by a dispatch"
expect_call "codesign --force --options runtime --timestamp --identifier org.githubrobbi.md2html"
expect_call "spctl --assess --type open --context context:primary-signature"

start "a release built by a dispatch of release.yml on main"
printf 'refs/heads/main\n' > "$case_dir/attest-ref"
run "$tag"
expect_published
expect_out "built by a dispatch of release.yml on main"

start "a signature without the hardened runtime"
touch "$case_dir/no-runtime"
run "$tag"
expect_code 1
expect_out "lacks (runtime)"
expect_no_call "notarytool submit"
expect_no_call "gh release upload"

start "Gatekeeper does not accept the notarized binary"
touch "$case_dir/gatekeeper-refuses"
run "$tag"
expect_code 1
expect_out "Gatekeeper refuses md2html-macos-arm64"
expect_no_call "gh release upload"
expect_no_call "gh release edit"

start "Gatekeeper accepts it, but not as notarized"
touch "$case_dir/gatekeeper-unnotarized"
run "$tag"
expect_code 1
expect_out "does not see md2html-macos-arm64 as notarized"
expect_no_call "gh release upload"

start "a codesign that left the binaries as they were"
touch "$case_dir/codesign-changes-nothing"
run "$tag"
expect_code 1
expect_out "instead of the four macOS assets"
expect_no_call "gh release upload"

start "GitHub still shows a draft after publishing"
touch "$case_dir/publish-changes-nothing"
run "$tag"
expect_code 1
expect_out "still shows $tag as a draft"
expect_no_call "gh workflow run"

start "notarization refused, then the same command again"
printf '{"id": "sub-1", "status": "Invalid", "message": "Processing complete"}\n' > "$case_dir/notary-answer"
run "$tag"
expect_code 1
expect_out "answered 'Invalid'"
expect_out "still a draft"
expect_no_call "gh release upload"
expect_no_call "gh release edit"
printf '{"id": "sub-2", "status": "Accepted", "message": "Processing complete"}\n' > "$case_dir/notary-answer"
run "$tag"
expect_published

start "publishing failed after the upload, then the same command again"
touch "$case_dir/refuse-publish"
run "$tag"
expect_code 1
expect_out "could not be published"
[[ "$(cat "$case_dir/isdraft")" == "true" ]] || bad "the release is not a draft any more"
rm "$case_dir/refuse-publish"
run "$tag"
expect_published

start "a draft that gained an asset between two runs"
touch "$case_dir/refuse-upload"
run "$tag"
expect_code 1
rm "$case_dir/refuse-upload"
printf 'new\n' > "$case_dir/release/md2html-extra.zip"
run "$tag"
expect_refused "are not the ones kept in"

start "the draft does not hold what was uploaded"
touch "$case_dir/upload-corrupts"
run "$tag"
expect_code 1
expect_out "the draft's CHECKSUMS.txt is not the file uploaded"
expect_no_call "gh release edit"

start "a fork publishes, and dispatches nothing"
printf 'someone/md2html\n' > "$case_dir/repo"
run "$tag"
expect_code 0
expect_out "someone/md2html is no canonical repository: brew-publish.yml not dispatched"
expect_no_call "gh workflow run"

start "a draft changed by someone else between two runs"
touch "$case_dir/refuse-upload"
run "$tag"
expect_code 1
rm "$case_dir/refuse-upload"
printf 'someone else\n' >> "$case_dir/release/md2html-macos-x64"
run "$tag"
expect_refused "is neither the verified download"

if (( failed )); then
  printf '❌ sign_macos.sh: a case failed (%d cases)\n' "$count"
  exit 1
fi
printf '✅ sign_macos.sh, pack_zip.sh, checksums.sh: %d cases\n' "$count"
