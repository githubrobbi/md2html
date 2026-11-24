# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# md2html itself has no dependencies; these recipes look at the tooling
# crates' supply chain and at md2html's own speed.

# Security audit: RustSec advisories against Cargo.lock (the tooling crates).
audit:
    @printf "\033[0;34m🔒 Security audit...\033[0m\n"
    just _install-if-missing cargo-audit cargo-audit
    cargo audit

# Advisories (cargo-audit) and the whole cargo-deny policy, both or nothing.
audit-comprehensive:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔒 Comprehensive security audit...\033[0m\n"
    just _install-if-missing cargo-audit cargo-audit
    just _install-if-missing cargo-deny cargo-deny
    printf "\033[0;34m  → cargo-audit (vulnerability scan)...\033[0m\n"
    cargo audit
    printf "\033[0;34m  → cargo-deny (advisories, bans, licences, sources)...\033[0m\n"
    cargo deny check
    printf "\033[0;32m✅ audit clean\033[0m\n"

# Unused dependencies (cargo-udeps, the Tier 2 pin) and outdated ones
# (cargo-outdated), both or nothing.

# Unused and outdated dependencies of the workspace.
deps-optimize:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔧 Dependency review...\033[0m\n"
    if ! command -v cargo-udeps >/dev/null 2>&1; then
        printf "\033[0;31m❌ cargo-udeps not found - install with: \033[0;36mcargo install cargo-udeps --version 0.1.61 --locked\033[0m\n"
        exit 1
    fi
    just _install-if-missing cargo-outdated cargo-outdated
    printf "\033[0;34m  → Finding unused dependencies...\033[0m\n"
    cargo udeps --locked --workspace --all-targets --all-features
    printf "\033[0;34m  → Checking for outdated dependencies...\033[0m\n"
    cargo outdated --workspace

# Where a render spends its time: the repository's documents (README,
# showcase, the capabilities page and its translations, all of them as
# one, and that twice) rendered in three stages, the blocks read, the
# bare HTML of --fragment and the whole page, each timed on its own with
# the release profile (crates/md2html/src/tests/measure.rs). A
# measurement, not a gate: compare a change with main on one machine.

# Time each stage of a render on the repository's documents (release profile).
measure:
    cargo test --release -p md2html --bin md2html where_the_time_goes -- --ignored --nocapture

# A CPU flamegraph of md2html rendering one document (by default the
# capabilities page, the largest of the repository's documents).  The
# page is written to a scratch folder, never over the repository's own
# rendered pages.  macOS needs `sudo` for dtrace; Linux needs `perf`.

# CPU flamegraph of md2html rendering one document (docs/capabilities.md by default).
flamegraph document="docs/capabilities.md":
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔥 Flamegraph: md2html rendering {{ document }}\033[0m\n"
    if ! command -v cargo-flamegraph >/dev/null 2>&1; then
        printf "\033[0;31m❌ cargo-flamegraph not found - install with: \033[0;36mcargo install flamegraph\033[0m\n"
        exit 1
    fi
    [[ -f "{{ document }}" ]] || { printf "\033[0;31m❌ no such document: %s\033[0m\n" "{{ document }}"; exit 1; }
    out_dir="$(mktemp -d)"
    trap 'rm -rf "$out_dir"' EXIT
    mkdir -p target
    svg="target/flamegraph-$(date +%Y%m%d-%H%M%S).svg"
    # The profiler's own error (permissions, a missing perf) is printed as
    # it is; the hint below only adds the usual cure.
    if ! cargo flamegraph --release -p md2html --bin md2html -o "$svg" -- "{{ document }}" -o "$out_dir/page.html"; then
        printf "\033[1;33m   On macOS dtrace needs root: sudo just flamegraph; on Linux install perf.\033[0m\n"
        exit 1
    fi
    printf "\033[0;32m✅ Flamegraph: %s (open it in a browser)\033[0m\n" "$svg"

# Profile-Guided Optimization for md2html.
#
# Three phases:
#
#   1. Build an instrumented `md2html` (it writes .profraw samples on
#      every run).
#   2. Drive it through a representative corpus: the repository's own
#      documents (README, showcase, the capabilities page and its
#      translations), rendered to a scratch folder, ten times each.
#      Every run must succeed: a failing run would profile an error path.
#   3. Merge the samples with `llvm-profdata` and rebuild with
#      `-Cprofile-use`, which lets LLVM lay out the hot blocks together
#      and inline the hot call graph.
#
# Prerequisite: the `llvm-tools` rustup component (rust-toolchain.toml
# lists it).
#
# Output:
#   - `<target>/pgo-data/`                 raw samples (discarded on each run)
#   - `<target>/pgo-data/merged.profdata`  merged profile
#   - `<target>/release/md2html`           the PGO-optimized binary

# Profile-guided optimized md2html build, trained on the repository's documents.
[unix]
pgo:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[1;34m🏁 PGO pipeline for md2html\033[0m\n"
    printf "\033[0;34m   Three phases: instrumented build → corpus run → optimized rebuild\033[0m\n\n"

    # Locate llvm-profdata shipped with the Rust toolchain.
    sysroot="$(rustc --print sysroot)"
    host="$(rustc -vV | sed -n 's|host: ||p')"
    LLVM_PROFDATA="${sysroot}/lib/rustlib/${host}/bin/llvm-profdata"
    if [[ ! -x "$LLVM_PROFDATA" ]]; then
        printf "\033[1;31m❌ llvm-profdata not found at %s\033[0m\n" "$LLVM_PROFDATA"
        printf "\033[0;34m   Install with: rustup component add llvm-tools\033[0m\n"
        exit 1
    fi
    printf "\033[0;34m  llvm-profdata: %s\033[0m\n" "$LLVM_PROFDATA"

    # Honour CARGO_TARGET_DIR.
    TARGET_DIR="${CARGO_TARGET_DIR:-$(pwd)/target}"
    PGO_DIR="${TARGET_DIR}/pgo-data"
    rm -rf "$PGO_DIR"
    mkdir -p "$PGO_DIR"

    # Phase 1: instrumented build
    printf "\n\033[0;34m🔨 Phase 1/3: building instrumented md2html\033[0m\n"
    RUSTFLAGS="-Cprofile-generate=${PGO_DIR}" cargo build --release -p md2html
    INSTRUMENTED="${TARGET_DIR}/release/md2html"
    if [[ ! -x "$INSTRUMENTED" ]]; then
        printf "\033[1;31m❌ instrumented binary missing: %s\033[0m\n" "$INSTRUMENTED"
        exit 1
    fi

    # Phase 2: corpus run
    printf "\n\033[0;34m🏃 Phase 2/3: rendering the repository's documents (10× each)\033[0m\n"
    out_dir="$(mktemp -d)"
    trap 'rm -rf "$out_dir"' EXIT
    corpus=(README.md docs/showcase.md docs/capabilities.md docs/capabilities.*.md)
    for doc in "${corpus[@]}"; do
        printf "\033[0;34m    → %s\033[0m\n" "$doc"
        for _ in 1 2 3 4 5 6 7 8 9 10; do
            "$INSTRUMENTED" "$doc" -o "$out_dir/page.html"
        done
    done
    printf "\033[0;32m  ✓ corpus run complete (%d documents)\033[0m\n" "${#corpus[@]}"

    # Phase 3: merge + rebuild
    printf "\n\033[0;34m🧬 Phase 3/3: merging profraw and rebuilding\033[0m\n"
    MERGED="${PGO_DIR}/merged.profdata"
    "$LLVM_PROFDATA" merge -o "$MERGED" "$PGO_DIR"
    RUSTFLAGS="-Cprofile-use=${MERGED}" cargo build --release -p md2html
    printf "\033[0;32m✅ PGO build complete - %s/release/md2html uses the merged profile\033[0m\n" "$TARGET_DIR"
    printf "\033[0;34m   Compare it with a plain release build on the same documents before relying on it.\033[0m\n"
