#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
#
# sign_macos.sh TAG [--prefix P] [--profile NAME] [--identifier ID] [--workdir DIR]
#
# The second stage of a release with lane:codesign (`just
# release-sign-macos vX.Y.Z`, on the owner's Mac).  release.yml built the
# release from the signed tag and left it as a DRAFT with every asset,
# the macOS binaries ad-hoc signed; this script signs those two binaries
# with the Developer ID identity in this Mac's keychain, has Apple
# notarize them, packs the two macOS archives again, writes the release's
# checksum lists again, replaces those six assets on the draft, publishes
# it, and dispatches the package managers' workflows release.yml leaves
# to it.  No Apple key or credential ever reaches GitHub
# (docs/forge/CODE-SIGNING.md, section 5).
#
# It refuses, before it downloads anything, unless: this is macOS; TAG is
# vX.Y.Z; lane:release, lane:codesign and lane:slsa are on; the tag is an
# annotated tag whose signature GitHub verifies and that names a commit;
# its release exists and is a draft; exactly one Developer ID Application
# identity qualifies (gopass P/signing-identity names it, by its name or
# its SHA-1, or the keychain holds one); and the notary profile NAME is in
# the keychain.  Then, before it signs anything: every asset of the draft
# is in CHECKSUMS.txt with its SHA-256, CHECKSUMS.txt is what
# scripts/release/checksums.sh writes for them, and every asset carries
# the build provenance of release.yml in this repository for this tag's
# commit (`gh attestation verify`), and each macOS archive is what
# scripts/release/pack_zip.sh packs from its own files (so the archive
# rebuilt after signing differs in the binary alone).
#
# Nothing is published until every step has passed: a failure leaves the
# release a draft, and running the same command again starts over from
# the verified downloads kept in DIR (default target/release-sign/TAG).
# A draft whose macOS assets this script already replaced is accepted on
# such a run, by the SHA-256 of every file it has uploaded.  Exit 0 when
# the release is published and every lane's workflow dispatched, 1 on any
# refusal or failure, 2 on a usage error.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"
pack="$here/pack_zip.sh"
sums="$here/checksums.sh"

usage() {
  printf 'usage: sign_macos.sh vX.Y.Z [--prefix P] [--profile NAME] [--identifier ID] [--workdir DIR]\n' >&2
  exit 2
}

(( $# >= 1 )) || usage
tag="$1"
shift
prefix="keys/apple"
profile="md2html-notary"
identifier="org.githubrobbi.md2html"
work=""
while (( $# > 0 )); do
  (( $# >= 2 )) || usage
  case "$1" in
    --prefix) prefix="$2" ;;
    --profile) profile="$2" ;;
    --identifier) identifier="$2" ;;
    --workdir) work="$2" ;;
    *) usage ;;
  esac
  shift 2
done
[[ -n "$work" ]] || work="$repo_root/target/release-sign/$tag"

# The two macOS targets, as release.yml names their assets.
arches=(arm64 x64)

say() { printf '%s\n' "$*"; }
step() { printf '\n── %s\n' "$*"; }
refuse() {
  printf '❌ %s\n' "$1" >&2
  exit 1
}
# After the downloads: say that the draft stays a draft and how to go on.
fail() {
  printf '❌ %s\n' "$1" >&2
  printf '   Nothing was published: %s is still a draft.  Fix the cause, then run again: just release-sign-macos %s\n' "$tag" "$tag" >&2
  exit 1
}

sha256_of() {
  local digest rest
  if command -v sha256sum >/dev/null; then
    read -r digest rest < <(sha256sum "$1")
  else
    read -r digest rest < <(shasum -a 256 "$1")
  fi
  printf '%s\n' "$digest"
}

# ─── 1. Refusals that need nothing downloaded ─────────────────────────
step "What must hold before anything is downloaded"

os="$(uname -s)"
[[ "$os" == "Darwin" ]] || refuse "this runs on macOS only (this is $os): codesign, notarytool and the keychain that holds the Developer ID are Apple's"
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || refuse "'$tag' is not a release tag vX.Y.Z"
for tool in gh codesign xcrun security spctl ditto zip unzip python3; do
  command -v "$tool" >/dev/null || refuse "$tool is not on PATH"
done

cd "$repo_root"
repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)" || refuse "gh cannot read this repository (gh auth status)"

# The lanes, read once.  A variable that is not set is off, as in a
# workflow's `vars.X == 'true'`.
lanes="$(gh variable list --repo "$repo" --json name,value --jq '.[] | "\(.name)=\(.value)"')" \
  || refuse "gh cannot read the repository's variables (it needs the owner's rights)"
lane_on() { grep -qx "$1=true" <<< "$lanes"; }
for lane in LANE_RELEASE LANE_CODESIGN LANE_SLSA; do
  lane_on "$lane" || refuse "$lane is not true: a release waits for this step only with lane:release, lane:codesign and lane:slsa on (docs/forge/COMPONENTS.md, lane:codesign)"
done
release_repo="$(sed -n 's/^RELEASE_REPO=//p' <<< "$lanes")"
if [[ -n "$release_repo" && "$release_repo" != "$repo" ]]; then
  refuse "RELEASE_REPO is $release_repo: this step signs the releases of $repo only"
fi
say "✅ $repo: LANE_RELEASE, LANE_CODESIGN and LANE_SLSA are on"

# The tag, read from GitHub as release.yml reads it.
ref="$(gh api "repos/$repo/git/ref/tags/$tag" --jq '"\(.object.type) \(.object.sha)"')" \
  || refuse "there is no tag $tag on GitHub: just release-tag signs and pushes it"
read -r ref_type tag_object <<< "$ref"
[[ "$ref_type" == "tag" ]] || refuse "$tag is a lightweight tag (it points at a $ref_type): a release tag is annotated and signed"
fields="$(gh api "repos/$repo/git/tags/$tag_object" --jq '"\(.tag) \(.verification.verified) \(.verification.reason) \(.object.type) \(.object.sha)"')" \
  || refuse "gh cannot read the tag object $tag_object"
read -r tag_name verified reason target_type commit <<< "$fields"
[[ "$tag_name" == "$tag" ]] || refuse "the ref $tag points at a tag object named $tag_name"
[[ "$verified" == "true" ]] || refuse "GitHub does not verify the signature of $tag (reason: $reason)"
[[ "$target_type" == "commit" ]] || refuse "$tag points at a $target_type, not a commit"
say "✅ $tag is signed ($reason) and names $commit"

# The release: there, and a draft.
if ! is_draft="$(gh release view "$tag" --repo "$repo" --json isDraft --jq .isDraft)"; then
  refuse "$repo has no release $tag: release.yml leaves it as a draft when lane:codesign is on (Actions tab: did the run finish?)"
fi
if [[ "$is_draft" != "true" ]]; then
  printf '❌ %s is published already: a published release is never changed.\n' "$tag" >&2
  printf '   If its package managers were not told yet, dispatch them by hand:\n' >&2
  lane_on LANE_BREW && printf '     gh workflow run brew-publish.yml --repo %s -f release-tag=%s\n' "$repo" "$tag" >&2
  lane_on LANE_WINGET && printf '     gh workflow run winget-publish.yml --repo %s -f release-tag=%s\n' "$repo" "$tag" >&2
  lane_on LANE_CRATES && printf '     gh workflow run release-plz.yml --repo %s -f publish_tag=%s\n' "$repo" "$tag" >&2
  exit 1
fi
say "✅ the release $tag is a draft"

# The signing identity: named in gopass (by its name or its SHA-1), or
# the one Developer ID Application identity of the keychain.  It is used
# by its SHA-1, so two certificates of the same name are told apart.
wanted=""
if command -v gopass >/dev/null; then
  entries="$(gopass ls --flat "$prefix")" || refuse "gopass cannot list $prefix (unlock the store, or pass another prefix)"
  if grep -qxF "$prefix/signing-identity" <<< "$entries"; then
    wanted="$(gopass show -o "$prefix/signing-identity")" || refuse "gopass cannot read $prefix/signing-identity"
    [[ -n "$wanted" ]] || refuse "$prefix/signing-identity is empty in gopass"
  fi
fi
listing="$(security find-identity -v -p codesigning)" || refuse "security cannot list this Mac's code-signing identities"
pairs="$(sed -n 's/^ *[0-9][0-9]*) \([0-9A-F]\{40\}\) "\(.*\)"$/\1 \2/p' <<< "$listing" | LC_ALL=C sort -u)"
if [[ "$wanted" =~ ^[0-9A-Fa-f]{40}$ ]]; then
  upper="$(tr 'a-f' 'A-F' <<< "$wanted")"
  candidates="$(awk -v h="$upper" '$1 == h' <<< "$pairs")"
  origin="the SHA-1 $wanted in gopass $prefix/signing-identity"
elif [[ -n "$wanted" ]]; then
  candidates="$(awk -v n="$wanted" '{ hash = $1; sub(/^[^ ]+ /, ""); if ($0 == n) print hash " " $0 }' <<< "$pairs")"
  origin="the name '$wanted' in gopass $prefix/signing-identity"
else
  candidates="$(awk '{ hash = $1; sub(/^[^ ]+ /, ""); if (index($0, "Developer ID Application: ") == 1) print hash " " $0 }' <<< "$pairs")"
  origin="the keychain's Developer ID Application identities"
fi
count="$(awk 'NF { n++ } END { print n + 0 }' <<< "$candidates")"
if [[ "$count" != 1 ]]; then
  printf '❌ %s match %s valid code-signing identities in the keychain, not one:\n' "$origin" "$count" >&2
  [[ -n "$candidates" ]] && printf '     %s\n' "$candidates" >&2
  printf '   Store the SHA-1 of the md2html release certificate in gopass: gopass insert %s/signing-identity\n' "$prefix" >&2
  exit 1
fi
ident_hash="${candidates%% *}"
ident_name="${candidates#* }"
[[ "$ident_name" == "Developer ID Application: "* ]] || refuse "$ident_name is no Developer ID Application identity"
say "✅ signing with $ident_name ($ident_hash)"

# The notary credentials: a keychain profile `xcrun notarytool
# store-credentials` made; the key stays in this keychain.
if ! notary_check="$(xcrun notarytool history --keychain-profile "$profile" 2>&1)"; then
  printf '%s\n' "$notary_check" >&2
  refuse "the notary profile '$profile' is not in the keychain or does not sign in: make it once with xcrun notarytool store-credentials $profile (docs/forge/CODE-SIGNING.md, section 5)"
fi
say "✅ the notary profile $profile signs in"

# What the rebuilt archives need: the commit's time (their entries'
# time) and the entitlements at that commit.
commit_date="$(gh api "repos/$repo/commits/$commit" --jq .commit.committer.date)" || refuse "gh cannot read the commit $commit"
epoch="$(python3 -c 'import sys, datetime; print(int(datetime.datetime.strptime(sys.argv[1], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc).timestamp()))' "$commit_date")" \
  || refuse "the commit's date '$commit_date' is not a UTC time"

# ─── 2. The draft's assets, verified ──────────────────────────────────
step "The draft's assets, verified before anything is signed"

mkdir -p "$work"
orig="$work/orig"
fresh="$work/fresh"
uploaded="$work/uploaded"
mkdir -p "$uploaded"
gh api "repos/$repo/contents/packaging/macos/md2html.entitlements?ref=$commit" -H 'Accept: application/vnd.github.raw' > "$work/entitlements.plist" \
  || refuse "gh cannot read packaging/macos/md2html.entitlements at $commit"
grep -q '<plist' "$work/entitlements.plist" || refuse "packaging/macos/md2html.entitlements at $commit is no property list"

rm -rf "$fresh"
gh release download "$tag" --repo "$repo" --dir "$fresh" || refuse "the assets of the draft $tag could not be downloaded"

macos_assets=()
for arch in "${arches[@]}"; do
  macos_assets+=("md2html-macos-$arch" "md2html-macos-$arch.zip")
done

# Every asset is listed with its SHA-256, the lists are what checksums.sh
# writes, and every asset carries release.yml's provenance for this
# commit.
verify_release() {
  local dir="$1" name check="$work/recheck"
  for name in "${macos_assets[@]}" CHECKSUMS.txt SHA256SUMS; do
    [[ -f "$dir/$name" ]] || fail "the draft has no asset $name"
  done
  cmp -s "$dir/CHECKSUMS.txt" "$dir/SHA256SUMS" || fail "the draft's SHA256SUMS is not its CHECKSUMS.txt"
  rm -rf "$check"
  cp -R "$dir" "$check"
  bash "$sums" "$check"
  if ! cmp -s "$dir/CHECKSUMS.txt" "$check/CHECKSUMS.txt"; then
    printf '   in the one list, not the other:\n' >&2
    comm -3 <(LC_ALL=C sort "$dir/CHECKSUMS.txt") <(LC_ALL=C sort "$check/CHECKSUMS.txt") >&2
    fail "the draft's assets are not the ones its CHECKSUMS.txt lists (or checksums.sh writes another list): none is signed"
  fi
  rm -rf "$check"
  for path in "$dir"/*; do
    name="${path##*/}"
    attest_ok "$path" || fail "$name carries no build provenance of release.yml in $repo for $tag at $commit: none is signed"
  done
}

# One asset's provenance: signed by release.yml of this repository, from
# the push of this tag at its commit, or (a run started again by
# dispatch) from the default branch, whose release.yml checked the tag
# and built the commit it names.
attest_ok() {
  local path="$1" pushed dispatched
  if pushed="$(gh attestation verify "$path" --repo "$repo" \
        --signer-workflow "$repo/.github/workflows/release.yml" \
        --source-ref "refs/tags/$tag" --source-digest "$commit" 2>&1)"; then
    return 0
  fi
  if dispatched="$(gh attestation verify "$path" --repo "$repo" \
        --signer-workflow "$repo/.github/workflows/release.yml" \
        --source-ref "refs/heads/$default_branch" 2>&1)"; then
    say "   ${path##*/}: built by a dispatch of release.yml on $default_branch"
    return 0
  fi
  printf '%s\n%s\n' "$pushed" "$dispatched" >&2
  return 1
}

default_branch="$(gh repo view "$repo" --json defaultBranchRef --jq .defaultBranchRef.name)" || refuse "gh cannot read the default branch of $repo"

if [[ ! -d "$orig" ]]; then
  verify_release "$fresh"
  mv "$fresh" "$orig"
  say "✅ every asset of the draft is listed, attested and kept in $orig"
else
  # A run after one that stopped: the kept downloads are verified again,
  # and the draft may hold only them or what an earlier run uploaded.
  verify_release "$orig"
  (cd "$orig" && find . -mindepth 1 -print | LC_ALL=C sort) > "$work/orig.names"
  (cd "$fresh" && find . -mindepth 1 -print | LC_ALL=C sort) > "$work/fresh.names"
  cmp -s "$work/orig.names" "$work/fresh.names" || fail "the draft's assets are not the ones kept in $orig: delete $work and the draft (gh release delete $tag --yes), then dispatch release.yml for $tag again"
  for path in "$fresh"/*; do
    name="${path##*/}"
    if ! cmp -s "$path" "$orig/$name" && [[ ! -f "$uploaded/$(sha256_of "$path")" ]]; then
      fail "the draft's $name is neither the verified download kept in $orig nor a file this script uploaded: delete $work and the draft (gh release delete $tag --yes), then dispatch release.yml for $tag again"
    fi
  done
  rm -rf "$fresh"
  say "✅ the draft holds the verified downloads kept in $orig, or files this script uploaded"
fi

# ─── 3. The archives, unpacked and checked against pack_zip.sh ────────
step "The macOS archives, unpacked"

stage="$work/stage"
rm -rf "$stage"
for arch in "${arches[@]}"; do
  base="md2html-macos-$arch"
  mkdir -p "$stage/$arch"
  entries="$(unzip -Z1 "$orig/$base.zip")" || fail "$base.zip cannot be read"
  bad="$(awk -v b="$base/" '$0 == b { next } index($0, b) == 1 && length($0) > length(b) && index(substr($0, length(b) + 1), "/") == 0 { next } { print }' <<< "$entries")"
  [[ -z "$bad" ]] || fail "$base.zip holds entries outside one flat folder $base/: $bad"
  (cd "$stage/$arch" && TZ=UTC env -u UNZIP -u UNZIPOPT unzip -q "$orig/$base.zip")
  [[ -f "$stage/$arch/$base/md2html" ]] || fail "$base.zip holds no $base/md2html"
  [[ -f "$stage/$arch/$base/NOTICE" ]] || fail "$base.zip holds no NOTICE"
  cmp -s "$stage/$arch/$base/md2html" "$orig/$base" || fail "the md2html in $base.zip is not the raw asset $base"
  bash "$pack" "$stage/$arch/$base" "$work/repacked-$arch.zip" "$epoch"
  cmp -s "$work/repacked-$arch.zip" "$orig/$base.zip" \
    || fail "pack_zip.sh on this Mac does not write the $base.zip release.yml wrote from the same files (another zip?): nothing was signed"
  rm -f "$work/repacked-$arch.zip"
done
say "✅ both archives are what pack_zip.sh packs from their files"

# ─── 4. Sign, verify, notarize ────────────────────────────────────────
step "Signing with the Developer ID"

signed="$work/signed"
rm -rf "$signed"
mkdir -p "$signed"
for arch in "${arches[@]}"; do
  bin="$signed/md2html-macos-$arch"
  cp "$orig/md2html-macos-$arch" "$bin"
  chmod 0755 "$bin"
  codesign --force --options runtime --timestamp --identifier "$identifier" \
    --entitlements "$work/entitlements.plist" --sign "$ident_hash" "$bin" \
    || fail "codesign could not sign md2html-macos-$arch"
  codesign --verify --strict --verbose=2 "$bin" || fail "the signature of md2html-macos-$arch does not verify"
  details="$(codesign -dvv "$bin" 2>&1)" || fail "codesign cannot read the signature of md2html-macos-$arch"
  for want in "Authority=$ident_name" "Identifier=$identifier" "Timestamp=" "(runtime)"; do
    [[ "$details" == *"$want"* ]] || fail "the signature of md2html-macos-$arch lacks $want"
  done
  say "✅ md2html-macos-$arch signed, hardened runtime, Apple time stamp"
done

step "Notarizing (Apple's notary service; usually minutes)"
notary="$work/notarize"
rm -rf "$notary"
mkdir -p "$notary/md2html"
for arch in "${arches[@]}"; do
  cp "$signed/md2html-macos-$arch" "$notary/md2html/"
done
ditto -c -k --keepParent "$notary/md2html" "$notary/md2html-notarize.zip"
if ! submitted="$(xcrun notarytool submit "$notary/md2html-notarize.zip" --keychain-profile "$profile" \
      --wait --timeout 30m --output-format json)"; then
  printf '%s\n' "$submitted" >&2
  fail "the notary service did not take the submission"
fi
parsed="$(python3 -c 'import json, sys; answer = json.loads(sys.argv[1]); print(answer.get("id", "-"), answer.get("status", "-"))' "$submitted")" \
  || fail "the notary service's answer is no JSON: $submitted"
read -r submission status <<< "$parsed"
if [[ "$status" != "Accepted" ]]; then
  xcrun notarytool log "$submission" --keychain-profile "$profile" >&2 || printf '   (the notary log of %s could not be read)\n' "$submission" >&2
  fail "the notary service answered '$status' for submission $submission"
fi
say "✅ notarized: submission $submission accepted"
for arch in "${arches[@]}"; do
  verdict="$(spctl --assess --type open --context context:primary-signature --verbose=2 "$signed/md2html-macos-$arch" 2>&1)" \
    || { printf '%s\n' "$verdict" >&2; fail "Gatekeeper refuses md2html-macos-$arch"; }
  [[ "$verdict" == *"source=Notarized Developer ID"* ]] || { printf '%s\n' "$verdict" >&2; fail "Gatekeeper does not see md2html-macos-$arch as notarized"; }
done
say "✅ Gatekeeper accepts both as Notarized Developer ID"

# ─── 5. The archives and the lists, again ─────────────────────────────
step "The archives packed again, the checksum lists written again"

out="$work/out"
rm -rf "$out"
cp -R "$orig" "$out"
for arch in "${arches[@]}"; do
  base="md2html-macos-$arch"
  cp "$signed/$base" "$out/$base"
  cp "$signed/$base" "$stage/$arch/$base/md2html"
  bash "$pack" "$stage/$arch/$base" "$out/$base.zip" "$epoch"
done
bash "$sums" "$out"

# Only the four macOS lines may change, and each of them must.
changed="$(comm -13 <(LC_ALL=C sort "$orig/CHECKSUMS.txt") <(LC_ALL=C sort "$out/CHECKSUMS.txt") | sed -n 's/^[0-9a-f]*  \.\///p' | LC_ALL=C sort)"
expected="$(printf '%s\n' "${macos_assets[@]}" | LC_ALL=C sort)"
[[ "$changed" == "$expected" ]] || fail "the new CHECKSUMS.txt changes $(tr '\n' ' ' <<< "$changed")instead of the four macOS assets"
say "✅ CHECKSUMS.txt and SHA256SUMS differ in the four macOS lines alone"

# ─── 6. Upload, check, publish, dispatch ──────────────────────────────
step "Replacing the six assets on the draft"

replace=()
for name in "${macos_assets[@]}" CHECKSUMS.txt SHA256SUMS; do
  replace+=("$out/$name")
  cp "$out/$name" "$uploaded/$(sha256_of "$out/$name")"
done
gh release upload "$tag" --repo "$repo" --clobber "${replace[@]}" || fail "the upload to the draft did not finish"

check="$work/check"
rm -rf "$check"
patterns=()
for name in "${macos_assets[@]}" CHECKSUMS.txt SHA256SUMS; do
  patterns+=(--pattern "$name")
done
gh release download "$tag" --repo "$repo" --dir "$check" "${patterns[@]}" || fail "the replaced assets could not be read back"
for name in "${macos_assets[@]}" CHECKSUMS.txt SHA256SUMS; do
  cmp -s "$check/$name" "$out/$name" || fail "the draft's $name is not the file uploaded"
done
say "✅ the draft holds the signed files"

step "Publishing $tag"
gh release edit "$tag" --repo "$repo" --draft=false || fail "the release could not be published"
now="$(gh release view "$tag" --repo "$repo" --json isDraft --jq .isDraft)" || refuse "gh cannot read the release $tag after publishing it"
[[ "$now" == "false" ]] || fail "GitHub still shows $tag as a draft"
say "✅ $tag is published"

step "The package managers (as release.yml dispatches them without lane:codesign)"
missed=()
dispatch() {
  local lane="$1" workflow="$2" input="$3"
  if ! lane_on "$lane"; then
    say "   $lane is off: $workflow not dispatched"
    return 0
  fi
  if [[ "${repo%%/*}" != "githubrobbi" ]]; then
    say "   $repo is no canonical repository: $workflow not dispatched"
    return 0
  fi
  if gh workflow run "$workflow" --repo "$repo" --ref "$default_branch" -f "$input=$tag"; then
    say "✅ $workflow dispatched for $tag"
  else
    missed+=("gh workflow run $workflow --repo $repo --ref $default_branch -f $input=$tag")
  fi
}
dispatch LANE_BREW brew-publish.yml release-tag
dispatch LANE_WINGET winget-publish.yml release-tag
dispatch LANE_CRATES release-plz.yml publish_tag
if (( ${#missed[@]} > 0 )); then
  printf '❌ %s is published, but these dispatches failed; run them by hand:\n' "$tag" >&2
  printf '     %s\n' "${missed[@]}" >&2
  exit 1
fi

printf '\n✅ %s: macOS binaries signed by %s and notarized; the release is published.\n' "$tag" "$ident_name"
