# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Show current version.
version:
    @printf "\033[0;34m📋 Current version:\033[0m\n"
    @cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "md2html") | .version'

# ── Binary release (lane:release, dormant) - `just ship` cuts it ────
#
# md2html ships its BINARY on your command, never on an ordinary merge,
# and only once LANE_RELEASE=true (docs/forge/COMPONENTS.md):
#
#   1.  just ship            # validate (Phase 1) → bump → commit →
#                            #   open the release/vX.Y.Z PR (Phase 2)
#   2.  just merge <PR>      # watches its checks, merges (merge commit);
#                            #   the merge fires release-auto-trigger.yml
#                            #   → release.yml builds the binaries for
#                            #   Windows x64, macOS ARM64 and x64, Linux
#                            #   x64 (static)
#
# `just ship` Phase 2 bumps `[workspace.package].version` (patch) ONLY
# after Phase 1 validation passes.  `release-tag` is the manual fallback
# for re-cuts or a failed dispatch.  `just release-local` builds the
# same archives on this machine without any of it.

# Manual FALLBACK after a `just ship` release PR merge - the normal
# path is automatic (release-auto-trigger.yml dispatches release.yml on
# merge).  Use for re-cuts or if the auto-dispatch failed:
# tag main → build binaries.

# Tag main with the workspace version and push the tag (lane:release fallback).
release-tag:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git branch --show-current)
    [ "$branch" = "main" ] || { printf "\033[0;31m❌ run from main (you are on %s)\033[0m\n" "$branch"; exit 1; }
    git fetch origin main --quiet
    git pull --ff-only origin main --quiet
    ver=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.name=="md2html") | .version')
    tag="v${ver}"
    if git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null; then
        printf "\033[0;31m❌ tag %s already exists\033[0m\n" "$tag"; exit 1
    fi
    git tag -s "$tag" -m "Release $tag"
    git push origin "$tag" --quiet
    printf "\n\033[0;32m✅ Tagged %s and pushed - release.yml builds the binaries when LANE_RELEASE=true.\033[0m\n" "$tag"
    printf "\033[0;36m   Watch: just release-watch   ·   Status: just release-status\033[0m\n"

# Build dev binary.
build:
    @printf "\033[0;34m🔨 Building dev binary (FAST-FAIL)...\033[0m\n"
    cargo build --workspace

# Build release binary.
build-release:
    @printf "\033[0;34m🔨 Building optimized release binary...\033[0m\n"
    @printf "\033[1;33m⚠️  This will take significantly longer than dev builds\033[0m\n"
    cargo build --release --workspace
