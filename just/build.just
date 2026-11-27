# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Show current version.
version:
    @printf "\033[0;34m📋 Current version:\033[0m\n"
    @cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "md2html") | .version'

# ── Binary release (lane:release, dormant) - `just ship` cuts it ────
#
# md2html ships its BINARY on your command, never on an ordinary merge,
# and only once LANE_RELEASE=true (docs/forge/COMPONENTS.md).  Every
# release tag is signed by the owner; no workflow makes one:
#
#   1.  just ship            # validate (Phase 1) → bump → commit →
#                            #   open the release/vX.Y.Z PR (Phase 2)
#   2.  just merge <PR>      # watches its checks, merges (merge commit)
#   3.  just release-tag     # on an up-to-date main: signs vX.Y.Z and
#                            #   pushes it; the push starts release.yml,
#                            #   which builds the binaries for Windows
#                            #   x64 and ARM64, macOS ARM64 and x64,
#                            #   Linux x64 and ARM64 (static) and
#                            #   publishes the release
#
# `just ship` Phase 2 bumps `[workspace.package].version` ONLY after
# Phase 1 validation passes: the patch number, or the one `--bump minor`
# or `--bump major` names (docs/policies/release_policy.md says which),
# then rolls the changelog and stamps the capabilities pages, so the
# version agrees in its three places (gate `versions`).  `just
# release-local` builds the same archives on this machine without any
# of it.

# The step after the release PR merges: `release-tag` signs `vX.Y.Z` on
# an up-to-date `main` and pushes it.  It refuses a commit that is not
# release-ready - the three places disagree, the changelog has no
# release heading for the version, or changes wait under Unreleased (main
# moved on after the release PR) - a tag that exists here or on origin,
# and a signing setup GitHub would not verify (release.yml refuses an
# unverified tag).

# Sign the release tag vX.Y.Z on main and push it; the push starts release.yml (lane:release).
release-tag:
    #!/usr/bin/env bash
    set -euo pipefail
    fail() { printf "\033[0;31m❌ %s\033[0m\n" "$1"; exit 1; }
    branch=$(git branch --show-current)
    [ "$branch" = "main" ] || fail "run from main (you are on ${branch:-a detached HEAD})"
    [ -z "$(git status --porcelain)" ] || fail "the working tree has changes: the tag names a commit, so commit or stash them first"
    git fetch origin main --quiet
    git pull --ff-only origin main --quiet
    [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
        || fail "main has commits origin/main does not: a release tag names a commit on origin/main"
    ver=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="md2html") | .version')
    tag="v${ver}"
    if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
        fail "tag $tag already exists here"
    fi
    remote_tag=$(git ls-remote --tags origin "refs/tags/$tag")
    [ -z "$remote_tag" ] || fail "tag $tag already exists on origin"
    # Release-ready: the version said the same in its three places, with
    # its own release heading in CHANGELOG.md (scripts/ci/versions.py).
    if ! python3 scripts/ci/versions.py check --tag "$tag"; then
        fail "main is not release-ready for $tag: merge the release PR of 'just ship' first"
    fi
    pending=$(awk '/^## \[Unreleased\]/ { open = 1; next } /^## / { open = 0 } open && NF' CHANGELOG.md)
    if [ -n "$pending" ]; then
        printf "\033[0;31m❌ CHANGELOG.md has entries under Unreleased: main moved on after the release PR merged,\033[0m\n"
        printf "\033[0;31m   and a tag here would release changes the notes of %s do not name.\033[0m\n" "$tag"
        printf "\033[0;36m   Sign the release PR's merge commit instead: git tag -s %s <merge-commit> -m \"Release %s\" && git push origin %s\033[0m\n" "$tag" "$tag" "$tag"
        exit 1
    fi
    # A tag GitHub cannot verify is refused by release.yml after the push.
    just doctor-signing || fail "fix the signing setup first: release.yml refuses a tag GitHub does not verify"
    git tag -s "$tag" -m "Release $tag"
    git push origin "refs/tags/$tag" --quiet
    printf "\n\033[0;32m✅ Signed %s and pushed it - release.yml builds and publishes the release when LANE_RELEASE=true.\033[0m\n" "$tag"
    printf "\033[0;36m   Watch: just release-watch   ·   Status: just release-status\033[0m\n"

# The winget manifests of `SkyLLC.md2html` for the published release
# vX.Y.Z, for the package's first submission to microsoft/winget-pkgs,
# made by hand (packaging/winget/README.md); every later version is
# komac's (winget-publish.yml).  The SHA-256 comes from the release's
# SHA256SUMS, checked against the downloaded Windows archive.

# Write the winget manifests of the published release vX.Y.Z into a folder (first submission).
winget-manifests tag outdir="target/winget":
    bash scripts/release/winget_manifests.sh {{ quote(tag) }} {{ quote(outdir) }}

# Build dev binary.
build:
    @printf "\033[0;34m🔨 Building dev binary (FAST-FAIL)...\033[0m\n"
    cargo build --workspace

# Build release binary.
build-release:
    @printf "\033[0;34m🔨 Building optimized release binary...\033[0m\n"
    @printf "\033[1;33m⚠️  This will take significantly longer than dev builds\033[0m\n"
    cargo build --release --workspace
