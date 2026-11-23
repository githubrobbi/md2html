# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.
#
# Supply-chain / cargo-vet maintenance recipes.
#
# These recipes wrap the four-layer cargo-vet audit-discipline policy
# (docs/SUPPLY-CHAIN.md; the gate's own header in
# scripts/ci/check_vet_audit_discipline.sh).
#
#   Layer 1 - `just vet-discipline`        (the CI gate, locally invokable)
#   Layer 2 - `just vet-bump <crate>`      (the guided audit driver)
#   Layer 3 - CODEOWNERS line on supply-chain/  (no recipe)
#   Layer 4 - `Vet-Reviewed-Diff:` commit trailer (enforced by Layer 1)

# Run the cargo-vet audit-discipline gate locally against
# origin/main..HEAD (the same range the pre-push hook and pr-fast CI
# job check).  Useful for verifying a `vet-bump` workflow before push.

# The cargo-vet audit-discipline gate over origin/main..HEAD.
vet-discipline:
    bash scripts/ci/check_vet_audit_discipline.sh range origin/main..HEAD

# Run the anti-pattern gate (scripts/ci/anti_pattern_gate.sh): fails on
# any `#[allow]` or `#[expect]` in any tracked Rust file except the one
# the owner approved (clippy::float_arithmetic in
# crates/md2html/src/mermaid/geom.rs), on lossy UTF-8 or UTF-16 decoding
# in md2html's production code, and on a discarded write to a stream.
# No marker silences it.  Run by `just go`, `just pre-pr` and ci.yml on
# the Linux runner.

# Anti-pattern gate: lint exceptions, lossy decodes, discarded writes.
audit-gate:
    bash scripts/ci/anti_pattern_gate.sh

# Guided cargo-vet exemption bump - prints a step-by-step recipe to
# replace a "lazy" exemption version bump with a proper delta audit
# anchored on the existing exemption.
#
# Usage:
#   just vet-bump <crate>                       # auto-detect OLD + NEW
#   just vet-bump <crate> <new-version>         # auto-detect OLD
#   just vet-bump <crate> <old-ver> <new-ver>   # both explicit

# Print the audit recipe for bumping a cargo-vet exemption properly.
vet-bump CRATE NEW="" OLD="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "{{OLD}}" && -n "{{NEW}}" ]]; then
        bash scripts/ci/vet_bump.sh "{{CRATE}}" "{{OLD}}" "{{NEW}}"
    elif [[ -n "{{NEW}}" ]]; then
        bash scripts/ci/vet_bump.sh "{{CRATE}}" "{{NEW}}"
    else
        bash scripts/ci/vet_bump.sh "{{CRATE}}"
    fi
