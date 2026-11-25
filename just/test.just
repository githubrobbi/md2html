# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Verify Rust toolchain consistency.
verify-toolchain:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔧 Verifying Rust toolchain consistency...\033[0m\n"
    rustc --version
    cargo --version
    rustfmt --version
    cargo clippy --version
    cargo metadata --format-version 1 --no-deps >/dev/null
    printf "\033[0;32m✅ Nightly toolchain verified (via rust-toolchain.toml)\033[0m\n"

# Everything a gate holds to a format, formatted by its own tool: Rust
# (rustfmt, rustfmt.toml; gate `fmt`), TOML (taplo, .taplo.toml, which
# leaves supply-chain/ out; gate `taplo`) and the supply-chain store
# (cargo vet fmt; gate `vet-fmt`).  .editorconfig says the rest (line
# endings, final newline, indentation) to editors.

# Format the Rust, the TOML and the supply-chain store, as the gates check them.
fmt:
    @printf "\033[0;34m📝 Formatting Rust, TOML and the supply-chain store...\033[0m\n"
    cargo fmt --all
    taplo fmt
    cargo vet fmt
    @printf "\033[0;32m✅ Formatting complete\033[0m\n"

# Check the same formats without changing a file.
fmt-check:
    @printf "\033[0;34m📝 Checking the formats of Rust, TOML and the supply-chain store...\033[0m\n"
    cargo fmt --all -- --check
    taplo fmt --check
    bash scripts/hooks/_check_vet_fmt.sh

# Run all tests with nextest.
test:
    @printf "\033[0;34m🧪 Running all tests with nextest (FAST-FAIL)...\033[0m\n"
    cargo llvm-cov nextest --workspace --all-features --all-targets --no-report

# Run documentation tests separately.
test-doc:
    @printf "\033[0;34m📚 Running documentation tests (FAST-FAIL)...\033[0m\n"
    RUSTDOCFLAGS='-Dwarnings -Dmissing_docs' cargo llvm-cov test --doc --workspace --all-features --no-report

# Generate coverage report.
coverage:
    just _install-if-missing cargo-llvm-cov cargo-llvm-cov
    cargo llvm-cov nextest --workspace --all-features --all-targets --html
    @printf "\033[0;32m📁 Coverage report: target/llvm-cov/html/index.html\033[0m\n"

# Ultra-strict production linting (workspace lints + justfile overrides).
lint-prod:
    @printf "\033[0;34m🔍 Ultra-strict production linting (FAST-FAIL)...\033[0m\n"
    cargo clippy --workspace --lib --bins --all-features --no-deps -- {{ lint_flags }}

# Pragmatic test linting (allows unwrap/expect in test code).
lint-tests:
    @printf "\033[0;34m🧪 Pragmatic test linting (FAST-FAIL)...\033[0m\n"
    cargo clippy --workspace --tests --all-features --no-deps -- {{ lint_flags }}

# Gate residual lints in CI (deny all warnings).
# MUST mirror .github/workflows/pr-fast.yml `clippy` job exactly so local
# runs catch everything CI catches.  Keep the two commands in lockstep.

# CI-mirror clippy: --all-targets --all-features -D warnings (the pr-fast clippy job).
lint-ci:
    @printf "\033[0;34m🚨 CI lint gating (FAST-FAIL)...\033[0m\n"
    cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings

# Feature-additivity regression guard.  Runs the
# same strict clippy flag stack as `lint-ci` but with `--no-default-features`
# instead of `--all-features`, catching items that are reachable only when
# a feature is enabled but not themselves `#[cfg(feature = "…")]`-gated.
# MUST mirror .github/workflows/pr-fast.yml `clippy-no-default` job exactly.
lint-ci-no-default:
    @printf "\033[0;34m🚨 CI lint gating (--no-default-features, FAST-FAIL)...\033[0m\n"
    cargo clippy --workspace --all-targets --no-default-features --no-deps -- -D warnings

# Cross-platform CI lint gate via Docker (Linux target): Linux-only
# clippy drift that the local host (macOS) cannot reproduce.  The
# Docker alternative to `lint-ci-linux-zig`.
#
# Uses a named Docker volume for `target/` so repeated runs reuse the Linux
# artifact cache (first run is cold, subsequent runs are incremental).
# The workspace nightly pin in `rust-toolchain.toml` is honoured via rustup.
#
# Requires: Docker (or compatible runtime such as colima, OrbStack, Podman).

# Linux clippy in Docker (rust:latest image, pinned toolchain via rustup).
lint-ci-linux:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🐧 Cross-platform CI lint gate (Linux x86_64 via Docker)...\033[0m\n"
    if ! command -v docker >/dev/null 2>&1; then
        printf "\033[0;31m❌ docker not found - install Docker Desktop / colima / OrbStack to use this recipe.\033[0m\n"
        exit 1
    fi
    if ! docker info >/dev/null 2>&1; then
        printf "\033[0;31m❌ docker daemon unreachable - start your container runtime first.\033[0m\n"
        exit 1
    fi
    docker volume create md2html-linux-target >/dev/null
    docker volume create md2html-linux-cargo-registry >/dev/null
    docker volume create md2html-linux-cargo-git >/dev/null
    docker run --rm \
        --platform linux/amd64 \
        -v "$PWD:/workspace" \
        -v md2html-linux-target:/workspace/target \
        -v md2html-linux-cargo-registry:/usr/local/cargo/registry \
        -v md2html-linux-cargo-git:/usr/local/cargo/git \
        -w /workspace \
        -e CARGO_TERM_COLOR=always \
        rust:latest \
        bash -lc "rustup show >/dev/null && cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings"
    printf "\033[0;32m✅ Linux CI lint gate passed\033[0m\n"

# Cross-platform strict lint gate for Windows (cargo-xwin clippy), the
# Windows counterpart of `lint-prod` / `lint-tests` / `lint-ci` with the
# same flag stacks (lane:cross-lint, docs/forge/COMPONENTS.md).
# `lint-ci-windows` is a hard pre-push gate; `pr-fast.yml`'s
# `windows-lint` job runs the equivalent native clippy on
# `windows-latest`.  Requires `cargo-xwin` + target
# `x86_64-pc-windows-msvc` (both from `just install-dev-tools`).

# Ultra-strict Windows production linting (workspace lints on --lib --bins).
lint-prod-windows: _require-xwin
    @printf "\033[0;34m🪟 Windows strict production lint (cargo xwin clippy)...\033[0m\n"
    cargo xwin clippy --workspace --lib --bins --all-features --target x86_64-pc-windows-msvc --no-deps -- {{ lint_flags }}

# Pragmatic Windows test linting (allows unwrap/expect in test code).
lint-tests-windows: _require-xwin
    @printf "\033[0;34m🪟 Windows strict test lint (cargo xwin clippy)...\033[0m\n"
    cargo xwin clippy --workspace --tests --all-features --target x86_64-pc-windows-msvc --no-deps -- {{ lint_flags }}

# The strict `cargo clippy --all-targets --all-features --no-deps -- -D
# warnings` stack the `pr-fast.yml::windows-lint` job runs natively on
# `windows-latest`.  Pre-push runs this via
# `scripts/hooks/_lint_pre_push.sh` so new Windows-gated code is gated on
# the same surface CI enforces.

# CI-mirror Windows clippy through cargo-xwin (a hard pre-push gate).
lint-ci-windows: _require-xwin
    @printf "\033[0;34m🚨 Windows CI lint gating (cargo xwin clippy)...\033[0m\n"
    cargo xwin clippy --workspace --all-targets --all-features --target x86_64-pc-windows-msvc --no-deps -- -D warnings

# Linux x86_64 clippy from a macOS (or other non-Linux-x86_64) host via
# cargo-zigbuild, a hard pre-push gate (lane:cross-lint).  The Docker
# path (`lint-ci-linux`) mirrors CI's image exactly; this one needs no
# container.  `cargo-zigbuild`'s `clippy` subcommand sets up a `zig cc`
# wrapper as the C cross-compiler, so any build script that compiles C
# produces Linux x86_64 objects from the host.  On an x86_64 Linux host
# the target IS the host: `lint-ci` lints it, and this recipe says so.
#
# md2html's zig is 0.17.1 (`[toolchain] ziglang` in scripts/ci/gates.toml):
# its ARM64 Linux build does not link with an older one.  `just install-dev-tools` installs it from the
# official ziglang.org tarball, checksum verified, into
# `~/.local/zig/0.17.1/`, beside whatever zig is on the PATH; every
# just-driven cargo-zigbuild here uses it (`CARGO_ZIGBUILD_ZIG_PATH`, set
# in just/local.just), so a different zig on the PATH is left alone.
#
# Requires: zig and `cargo-zigbuild` (both installed by
# `just install-dev-tools`).

# Linux x86_64 clippy via cargo-zigbuild (a hard pre-push gate).
lint-ci-linux-zig:
    #!/usr/bin/env bash
    set -euo pipefail
    host="$(rustc -vV | sed -n 's/^host: //p')"
    if [[ "$host" == "x86_64-unknown-linux-gnu" ]]; then
        printf "\033[0;36mℹ  the host is x86_64-unknown-linux-gnu: lint-ci lints this target natively; nothing to cross-compile\033[0m\n"
        exit 0
    fi
    printf "\033[0;34m🦎 Linux x86_64 clippy via cargo-zigbuild (%s → Linux)...\033[0m\n" "$host"
    # `CARGO_ZIGBUILD_ZIG_PATH` (just/local.just) names md2html's own zig
    # when it is installed; otherwise the zig on the PATH must exist.
    if [[ ! -x "${CARGO_ZIGBUILD_ZIG_PATH:-}" ]] && ! command -v zig >/dev/null 2>&1; then
        printf "\033[0;31m❌ zig not found - install with: \033[0;36mjust install-dev-tools\033[0m\n"
        exit 1
    fi
    if ! command -v cargo-zigbuild >/dev/null 2>&1; then
        printf "\033[0;31m❌ cargo-zigbuild not found - install with: \033[0;36mjust install-dev-tools\033[0m\n"
        exit 1
    fi
    # Pin `target-cpu` for the cross target: a user-level
    # `target-cpu=native` would leak the host CPU name (an arm64
    # codename on a Mac) into `zig cc -mcpu=native` for an x86_64 build
    # and break any assembly a dependency carries.  `x86-64-v3` is the
    # baseline release.yml's Linux row uses.
    #
    # Note: invoke the cargo-zigbuild binary directly (not via the `cargo
    # zigbuild` subcommand-plugin pattern).  `cargo zigbuild` always
    # routes into the `zigbuild` build subcommand and rejects further
    # subcommand args; the binary itself exposes `clippy` / `check` /
    # `test` / etc. as proper subcommands that wire up the same zig-cc
    # linker shim cargo-zigbuild's `zigbuild` does.
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C target-cpu=x86-64-v3 -C link-arg=-Wl,--gc-sections" \
        cargo-zigbuild clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --no-deps -- -D warnings
    printf "\033[0;32m✅ Linux clippy gate (zigbuild) passed\033[0m\n"

# The Intel macOS leg of lane:cross-lint (x86_64-apple-darwin, strict clippy).
#
# The fourth supported platform (macOS ARM + Intel, Linux, Windows with
# identical behavior). Cross-compiling from an arm64 host needs no extra
# toolchain: the host clang assembles x86_64 objects via `-arch x86_64`;
# only the rustup target must be installed (`just toolchain-ensure`
# installs it with the pin, `just install-dev-tools` adds it too).
#
# `target-cpu` is pinned to the `x86-64-v2` baseline for the same reason
# `lint-ci-linux-zig` pins `v3`: user-level `target-cpu=native` tuning
# (see .cargo/config.toml notes) would leak the arm64 host CPU name into
# an x86_64 build and corrupt codegen for C/asm in transitive deps.
#
# Host-guarded: cross-compiling TO darwin FROM Linux/Windows requires an
# Apple SDK those hosts do not have, so there the recipe says plainly
# that the check did not run.  On macOS a missing rustup target is an
# error with the install command (`just toolchain-ensure`, which `just go`
# runs, normally brings it in).

# Cross-target strict lint gate for Intel macOS (x86_64-apple-darwin).
lint-ci-mac-intel:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🍏 Intel macOS clippy (x86_64-apple-darwin cross from host)...\033[0m\n"
    if [ "$(uname -s)" != "Darwin" ]; then
        printf "\033[1;33m⚠  NOT CHECKED: x86_64-apple-darwin needs an Apple SDK, which only a macOS host has\033[0m\n"
        exit 0
    fi
    installed="$(rustup target list --installed)"
    if ! grep -qx 'x86_64-apple-darwin' <<< "$installed"; then
        printf "\033[0;31m❌ target x86_64-apple-darwin not installed - run: \033[0;36mrustup target add x86_64-apple-darwin\033[0m\n"
        exit 1
    fi
    CARGO_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS="-C target-cpu=x86-64-v2" \
        cargo clippy --workspace --all-targets --all-features --target x86_64-apple-darwin --no-deps -- -D warnings
    printf "\033[0;32m✅ Intel macOS clippy gate passed\033[0m\n"

# md2html on the minimum Rust it declares (`rust-version` in
# crates/md2html/Cargo.toml, the one place the number is written), built
# and its own tests run: a finding there, or a behaviour of that Rust's
# standard library a test holds, is caught before the push instead of by
# CI's `msrv` job.
msrv:
    #!/usr/bin/env bash
    set -euo pipefail
    version="$(sed -n 's/^rust-version = "\(.*\)"$/\1/p' crates/md2html/Cargo.toml)"
    if [ -z "$version" ]; then
        printf "\033[0;31m❌ crates/md2html/Cargo.toml declares no rust-version\033[0m\n"
        exit 1
    fi
    if ! rustup run "$version" rustc --version >/dev/null 2>&1; then
        printf "\033[0;31m❌ Rust %s is not installed - run: \033[0;36mrustup toolchain install %s --profile minimal\033[0m (or just install-dev-tools)\n" "$version" "$version"
        exit 1
    fi
    printf "\033[0;34m🦀 md2html on Rust %s (its declared minimum)...\033[0m\n" "$version"
    # The flags CI's `ci.yml` sets for every job, so this fails exactly
    # where its `msrv` job would.
    RUSTFLAGS="-D warnings" cargo "+$version" test --locked -p md2html --all-targets --quiet
    printf "\033[0;32m✅ md2html builds and passes its tests on Rust %s\033[0m\n" "$version"

# The repository's own documents as EPUBs (19 books) and the balancing's
# test page with its HTML trusted, each through epubcheck, failing on any
# message: what CI's `epub` job checks.  The
# checker is the `epubcheck` on the PATH (Homebrew's on macOS), else the
# release jar `just install-dev-tools` puts in ~/.local/epubcheck/, run
# with the java on the PATH.  The books and reports go to target/epubcheck.

# The 20 books through epubcheck, failing on any message (CI's `epub` job).
epubcheck:
    #!/usr/bin/env bash
    set -euo pipefail
    version="$(sed -n 's/^ *EPUBCHECK_VERSION: *//p' .github/workflows/ci.yml)"
    jar="$HOME/.local/epubcheck/${version}/epubcheck.jar"
    if command -v epubcheck >/dev/null 2>&1; then
        checker=(epubcheck)
    elif [[ -n "$version" && -f "$jar" ]] && command -v java >/dev/null 2>&1; then
        checker=(java -jar "$jar")
    else
        printf "\033[0;31m❌ epubcheck is not installed - install with: \033[0;36mjust install-dev-tools\033[0m\n"
        exit 1
    fi
    printf "\033[0;34m📚 The books through %s...\033[0m\n" "$("${checker[@]}" --version 2>&1 | head -n 1)"
    cargo build --release -p md2html
    bash scripts/ci/epubcheck_books.sh target/release/md2html target/epubcheck "${checker[@]}"

# The differential test (docs/TESTING.md): a corpus of 8,300 Markdown
# files (scripts/differential/corpus.py: the repository's, the CommonMark
# examples, seeded mutations of both) rendered in six ways by the md2html
# built from `base` and by this tree's, compared byte for byte, and no
# file may make this tree's program crash or run out of time. A change not
# meant to change output shows no difference (AGENTS.md section 11: pages
# are compared, not eyeballed); one meant to lists each, to be read. The
# base is built from its own sources in target/differential, with its own
# toolchain pin.

# Render the corpus with base's md2html and this tree's; fail on a difference or a crash.
differential base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    work="$PWD/target/differential"
    rm -rf "$work/base-src"
    mkdir -p "$work/base-src"
    git archive "{{ base }}" | tar -x -C "$work/base-src"
    printf "\033[0;34m🔁 md2html of %s, and of this tree...\033[0m\n" "{{ base }}"
    (cd "$work/base-src" && cargo build --release --locked -p md2html --target-dir "$work/base-target")
    cargo build --release --locked -p md2html
    python3 scripts/differential/corpus.py "$work/corpus"
    python3 scripts/differential/compare.py --base "$work/base-target/release/md2html" --new target/release/md2html "$work/corpus"

# Render the corpus with this tree's md2html; fail when a file makes it crash or run out of time.
generated-documents:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release --locked -p md2html
    python3 scripts/differential/corpus.py target/differential/corpus
    python3 scripts/differential/compare.py --new target/release/md2html target/differential/corpus

# Verifies that every gate listed in `scripts/ci/gates.toml` is present
# in the corresponding consumer file (`scripts/hooks/_lint_fast.sh`,
# `scripts/hooks/_lint_pre_push.sh`, `.github/workflows/pr-fast.yml`),
# that every gate present in those consumers has a matching manifest
# entry, and that the manifest's schema holds.  Hard-fails on mismatch;
# no bypass.  Wired into the pre-push hook and the
# `pr-fast.yml::gates-drift` job.

# Gate-manifest drift: the gate set of every consumer matches scripts/ci/gates.toml.
gates-drift:
    @bash scripts/ci/check_gates_drift.sh

# The generator (`scripts/ci/md2html-gen-hooks`) reads `scripts/ci/gates.toml`
# and writes `scripts/hooks/_lint_pre_push.sh`.  Use this recipe
# whenever you change the manifest; the on-disk hook carries an
# AUTO-GENERATED banner and any manual edits will be overwritten on
# the next regen.
#
# Idempotent: running this twice in a row leaves the file
# byte-identical; the `hooks-drift` recipe below is the check that
# enforces the same property.

# Regenerate the pre-push hook from scripts/ci/gates.toml.
md2html-gen-hooks:
    @cargo run -q --profile tooling --locked -p md2html-gen-hooks

# Verifies that the on-disk `scripts/hooks/_lint_pre_push.sh` matches
# what `md2html-gen-hooks` would produce from the canonical
# `scripts/ci/gates.toml`.  Catches the failure mode where someone
# manually edits the generated hook (whose header banner explicitly
# warns "MANUAL EDITS WILL BE OVERWRITTEN") instead of editing the
# manifest and regenerating.  Wired into the pre-push hook
# (Bucket 1) and the `pr-fast.yml::hooks-drift` job so manifest /
# emitted-hook divergence hard-blocks merge.

# Hook drift: the pre-push hook is byte-equal to what the generator emits.
hooks-drift:
    @cargo run -q --profile tooling --locked -p md2html-gen-hooks -- --check

# Validates `.github/workflows/pr-fast.yml` against the canonical
# `scripts/ci/gates.toml` (ten properties, listed in
# `docs/policies/lint-posture.md`): every pr-fast-tier gate has a job
# whose `if:` runs at least when its gates' `gate_when` says, the
# `required` aggregator names every gate job and keeps the name branch
# protection requires, every workflow agrees on action pins, tool
# versions, target flags and nextest profiles, and the `classify` job
# computes the change classes of `[classification]`, and each gate's job
# runs the gate's command (or the `ci_command` it declares).
#
# Read-only: never mutates the workflow file.  Wired into the pre-push
# hook and the `pr-fast.yml::workflow-drift` job so any drift
# hard-blocks merge.

# Workflow drift: pr-fast.yml (and every workflow's shared pins) agree with gates.toml.
workflow-drift:
    @cargo run -q --profile tooling --locked -p md2html-gen-workflow -- --check

# Same `md2html-gen-hooks` binary as the pre-push generator; `--target
# pre-commit` selects the `_lint_fast.sh` template + dispatch shape.
# Use this recipe whenever you change a `tier="pre-commit"` gate in
# the manifest; the on-disk hook carries an AUTO-GENERATED banner and
# any manual edits will be overwritten on the next regen.
#
# Idempotent; the `fast-drift` recipe below enforces the same property.

# Regenerate the pre-commit hook from scripts/ci/gates.toml.
gen-fast:
    @cargo run -q --profile tooling --locked -p md2html-gen-hooks -- --target pre-commit

# Verifies that the on-disk `scripts/hooks/_lint_fast.sh` matches
# what `md2html-gen-hooks --target pre-commit` would produce from the
# canonical `scripts/ci/gates.toml`.  Sibling of `hooks-drift` for
# the pre-commit tier - same binary, different `--target`.  Wired
# into the pre-push hook (Bucket 1) and the `pr-fast.yml::fast-drift`
# job so manifest / emitted-hook divergence hard-blocks merge.
#
# NOT wired into the pre-commit hook itself (would compile the
# validator on every commit and break its sub-2 s budget); the
# pre-push gate is the right place to catch the drift before it
# reaches the remote.

# Pre-commit hook drift: the pre-commit hook is byte-equal to what the generator emits.
fast-drift:
    @cargo run -q --profile tooling --locked -p md2html-gen-hooks -- --target pre-commit --check

# Host-only on purpose: the cross-target legs are `check-all-targets`
# (and the pre-push hook).

# Full lint sweep: production + tests + CI gate + formatting.
lint-all:
    @printf "\033[0;34m🔍 Full lint sweep (prod + tests + CI + fmt)...\033[0m\n"
    just lint-prod
    just lint-tests
    just lint-ci
    just fmt-check
    @printf "\033[0;32m✅ All lint checks passed\033[0m\n"

# ── Shift-left quality gates ───────────────────────────────────────────
#   IDE save   → rust-analyzer check-on-save
#   pre-commit → `just lint-fast`       (staged-scoped, sub-2 s)
#   pre-push   → `just lint-pre-push`   (workspace-parallel, ≤ 60 s)
#   PR CI      → .github/workflows/pr-fast.yml and ci.yml
# The hook runners are generated from scripts/ci/gates.toml; every gate
# there is hard: a missing tool fails it, `just install-dev-tools`
# installs them all.

# Spell-check the workspace (requires `typos-cli`).
# Pass scopes or flags, e.g. `just typos crates/md2html/src`.
typos *ARGS:
    @printf "\033[0;34m🔤 typos (spell check)...\033[0m\n"
    typos {{ ARGS }}

# REUSE / SPDX compliance lint (requires `reuse`, install via pipx).
reuse:
    @printf "\033[0;34m📜 REUSE/SPDX lint...\033[0m\n"
    reuse lint --quiet

# Check TOML formatting (requires `taplo-cli`).
taplo-check *ARGS:
    @printf "\033[0;34m📋 taplo fmt --check...\033[0m\n"
    taplo fmt --check {{ ARGS }}

# Apply TOML formatting (requires `taplo-cli`).
taplo-fmt *ARGS:
    @printf "\033[0;34m📋 taplo fmt (apply)...\033[0m\n"
    taplo fmt {{ ARGS }}

# Install every tool the git hooks' gates need (scripts/ci/gates.toml:
# every gate is hard, so a missing tool fails the push).  Idempotent:
# prints a ✓ line for each tool already present.  Fails at the end,
# naming each tool it could not install, so a partial install never
# reads as a complete one.
#
#   typos, taplo, cargo-machete, cargo-xwin   cargo install --locked
#   cargo-deny, cargo-vet                     the versions CI pins, built with stable
#   cargo-nextest                             cargo install --locked
#   zig 0.17.1 + cargo-zigbuild               the official ziglang.org tarball,
#                                             SHA-256 checked (macOS and Linux)
#   reuse                                     pipx
#   epubcheck (for `just epubcheck`)          Homebrew on macOS; elsewhere the
#                                             W3C release CI pins, SHA-256 checked
#   the x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu and
#   x86_64-apple-darwin rustup targets

# cargo-deny and cargo-vet at the versions `[toolchain]` in
# scripts/ci/gates.toml pins, the one place each is bumped (CI downloads
# the same versions' releases).  Built from crates.io with stable:
# cargo-vet's rustix 0.37 does not build with the pinned nightly on
# Linux.  A tool is installed again only when its version differs.

# Install cargo-deny and cargo-vet at the versions scripts/ci/gates.toml pins.
_install-supply-chain-tools:
    #!/usr/bin/env bash
    set -euo pipefail
    for tool in cargo-deny cargo-vet; do
        pin="$(sed -n "s/^${tool} = \"\(.*\)\"$/\1/p" scripts/ci/gates.toml)"
        if [[ -z "$pin" ]]; then
            printf "\033[0;31m❌ no %s pin in scripts/ci/gates.toml\033[0m\n" "$tool"
            exit 1
        fi
        have=""
        if command -v "$tool" >/dev/null 2>&1; then
            have="$(cargo "${tool#cargo-}" --version)"
        fi
        if [[ "$have" == "${tool} ${pin}" ]]; then
            printf "   \033[0;32m✓\033[0m %s %s already installed\n" "$tool" "$pin"
        else
            rustup toolchain install stable --profile minimal --no-self-update
            cargo +stable install --locked --force "${tool}@${pin}"
        fi
    done

# Install every tool the hooks' gates need (each gate is hard: a missing tool fails it).
install-dev-tools:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m📦 Installing the gate tools...\033[0m\n"
    missing=()
    cargo_tool() {
        local bin="$1" crate="$2"
        if command -v "$bin" >/dev/null 2>&1; then
            printf "   \033[0;32m✓\033[0m %s already installed\n" "$bin"
        else
            cargo install --locked "$crate"
        fi
    }
    cargo_tool typos typos-cli
    cargo_tool taplo taplo-cli
    cargo_tool cargo-machete cargo-machete
    cargo_tool cargo-xwin cargo-xwin
    cargo_tool cargo-nextest cargo-nextest
    cargo_tool cargo-zigbuild cargo-zigbuild
    # cargo-deny and cargo-vet at the versions CI pins.
    just _install-supply-chain-tools
    # The cross-lint targets (`just toolchain-ensure` installs them with the
    # pin; `rustup target add` is idempotent and fails loudly on a real
    # problem).
    rustup target add x86_64-pc-windows-msvc x86_64-unknown-linux-gnu x86_64-apple-darwin
    printf "   \033[0;32m✓\033[0m rustup targets for the cross-lint gates\n"
    # The minimum Rust md2html declares, for the `msrv` gate.
    msrv_version="$(sed -n 's/^rust-version = "\(.*\)"$/\1/p' crates/md2html/Cargo.toml)"
    rustup toolchain install "$msrv_version" --profile minimal --no-self-update
    printf "   \033[0;32m✓\033[0m Rust %s for the msrv gate\n" "$msrv_version"
    # zig is pinned to 0.17.1, the version `[toolchain] ziglang` names in
    # scripts/ci/gates.toml (md2html's ARM64 Linux build needs it).  It
    # is installed from the official ziglang.org tarball, its SHA-256
    # checked against the one ziglang.org publishes, into its own
    # versioned folder (`just/local.just` points cargo-zigbuild at it).
    # `~/.cargo/bin/zig` is linked only when no zig is there: another
    # repository's pin on the PATH is not taken over.
    ZIG_PIN="0.17.1"
    ZIG_DIR="$HOME/.local/zig/${ZIG_PIN}"
    ZIG_LINK="$HOME/.cargo/bin/zig"
    zig_ok() {
        local v
        [[ -x "$ZIG_DIR/zig" ]] || return 1
        v="$("$ZIG_DIR/zig" version)" || return 1
        [[ "$v" == "$ZIG_PIN" ]]
    }
    case "$(uname -s)" in
        Darwin) ZIG_OS="macos" ;;
        Linux) ZIG_OS="linux" ;;
        *) ZIG_OS="" ;;
    esac
    if zig_ok; then
        printf "   \033[0;32m✓\033[0m zig %s already installed\n" "$ZIG_PIN"
    elif [[ -z "$ZIG_OS" ]]; then
        printf "   \033[0;31m✗\033[0m zig %s: install it by hand from https://ziglang.org/download/ into %s\n" "$ZIG_PIN" "$ZIG_DIR"
        missing+=("zig ${ZIG_PIN}")
    else
        ZIG_ARCH="$(uname -m)"
        case "$ZIG_ARCH" in arm64) ZIG_ARCH="aarch64";; esac
        ZIG_TARBALL="zig-${ZIG_ARCH}-${ZIG_OS}-${ZIG_PIN}.tar.xz"
        ZIG_URL="https://ziglang.org/download/${ZIG_PIN}/${ZIG_TARBALL}"
        printf "   \033[0;36m·\033[0m installing zig %s from %s\n" "$ZIG_PIN" "$ZIG_URL"
        mkdir -p "$HOME/.local/zig"
        tmpdir="$(mktemp -d)"
        trap 'rm -rf "$tmpdir"' EXIT
        curl -fL --retry 5 --retry-delay 3 "$ZIG_URL" -o "$tmpdir/$ZIG_TARBALL"
        curl -fsSL --retry 5 --retry-delay 3 "https://ziglang.org/download/index.json" -o "$tmpdir/index.json"
        ZIG_WANT="$(python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]["shasum"])' "$tmpdir/index.json" "$ZIG_PIN" "${ZIG_ARCH}-${ZIG_OS}")"
        ZIG_GOT="$(shasum -a 256 "$tmpdir/$ZIG_TARBALL" | cut -d' ' -f1)"
        if [[ "$ZIG_WANT" != "$ZIG_GOT" ]]; then
            printf "   \033[0;31m✗\033[0m zig tarball checksum mismatch (want %s, got %s)\n" "$ZIG_WANT" "$ZIG_GOT"
            exit 1
        fi
        tar -xJf "$tmpdir/$ZIG_TARBALL" -C "$tmpdir"
        rm -rf "$ZIG_DIR"
        mv "$tmpdir/zig-${ZIG_ARCH}-${ZIG_OS}-${ZIG_PIN}" "$ZIG_DIR"
        # A download by curl carries no quarantine attribute; remove one
        # only where it exists.
        if [[ "$ZIG_OS" == "macos" ]] && xattr -p com.apple.quarantine "$ZIG_DIR/zig" >/dev/null 2>&1; then
            xattr -dr com.apple.quarantine "$ZIG_DIR"
        fi
        if [[ ! -e "$ZIG_LINK" ]]; then ln -s "$ZIG_DIR/zig" "$ZIG_LINK"; fi
        if ! zig_ok; then
            printf "   \033[0;31m✗\033[0m zig %s did not install at %s\n" "$ZIG_PIN" "$ZIG_DIR"
            exit 1
        fi
        printf "   \033[0;32m✓\033[0m zig %s installed at \033[0;36m%s\033[0m\n" "$ZIG_PIN" "$ZIG_DIR"
    fi
    if command -v reuse >/dev/null 2>&1; then
        printf "   \033[0;32m✓\033[0m reuse already installed\n"
    elif command -v pipx >/dev/null 2>&1; then
        pipx install reuse
    else
        printf "   \033[0;31m✗\033[0m reuse needs pipx: install pipx (brew install pipx / apt install pipx), then re-run\n"
        missing+=("reuse")
    fi
    # The gates `shellcheck`, `actionlint` and `zizmor` need their tools:
    # from Homebrew where there is one; elsewhere the system's packages
    # (shellcheck) or the release (actionlint, https://github.com/rhysd/actionlint;
    # zizmor, https://github.com/zizmorcore/zizmor, or `pipx install zizmor`).
    for tool in shellcheck actionlint zizmor; do
        if command -v "$tool" >/dev/null 2>&1; then
            printf "   \033[0;32m✓\033[0m %s already installed\n" "$tool"
        elif command -v brew >/dev/null 2>&1; then
            brew install "$tool"
        else
            printf "   \033[0;31m✗\033[0m %s: install it with the system's package manager\n" "$tool"
            missing+=("$tool")
        fi
    done
    # epubcheck, for `just epubcheck`: Homebrew's on macOS; elsewhere the
    # W3C release CI's `epub` job pins (its version and SHA-256 are read
    # from .github/workflows/ci.yml, the one place they are written),
    # checksum verified, into ~/.local/epubcheck/<version>/, run with the
    # java on the PATH.
    epub_version="$(sed -n 's/^ *EPUBCHECK_VERSION: *//p' .github/workflows/ci.yml)"
    epub_sha="$(sed -n 's/^ *EPUBCHECK_SHA256: *//p' .github/workflows/ci.yml)"
    [[ -n "$epub_version" && -n "$epub_sha" ]] || { printf "\033[0;31m❌ no epubcheck pin in .github/workflows/ci.yml\033[0m\n"; exit 1; }
    epub_dir="$HOME/.local/epubcheck/${epub_version}"
    if command -v epubcheck >/dev/null 2>&1; then
        printf "   \033[0;32m✓\033[0m epubcheck already installed\n"
    elif [[ -f "$epub_dir/epubcheck.jar" ]]; then
        printf "   \033[0;32m✓\033[0m epubcheck %s already installed at %s\n" "$epub_version" "$epub_dir"
    elif [[ "$(uname -s)" == "Darwin" ]] && command -v brew >/dev/null 2>&1; then
        brew install epubcheck
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        printf "   \033[0;31m✗\033[0m epubcheck needs Homebrew on macOS (https://brew.sh), then re-run\n"
        missing+=("epubcheck")
    else
        epub_zip="epubcheck-${epub_version}.zip"
        epub_tmp="$(mktemp -d)"
        curl -fL --retry 5 --retry-delay 3 -o "$epub_tmp/$epub_zip" \
            "https://github.com/w3c/epubcheck/releases/download/v${epub_version}/${epub_zip}"
        epub_got="$(sha256sum "$epub_tmp/$epub_zip" | cut -d' ' -f1)"
        if [[ "$epub_got" != "$epub_sha" ]]; then
            printf "   \033[0;31m✗\033[0m epubcheck checksum mismatch (want %s, got %s)\n" "$epub_sha" "$epub_got"
            exit 1
        fi
        unzip -q "$epub_tmp/$epub_zip" -d "$epub_tmp"
        mkdir -p "$HOME/.local/epubcheck"
        rm -rf "$epub_dir"
        mv "$epub_tmp/epubcheck-${epub_version}" "$epub_dir"
        rm -rf "$epub_tmp"
        printf "   \033[0;32m✓\033[0m epubcheck %s installed at \033[0;36m%s\033[0m\n" "$epub_version" "$epub_dir"
    fi
    if ! command -v epubcheck >/dev/null 2>&1 && ! command -v java >/dev/null 2>&1; then
        printf "   \033[0;31m✗\033[0m epubcheck needs Java 11 or newer: install a JDK (apt install default-jre), then re-run\n"
        missing+=("java")
    fi
    if (( ${#missing[@]} > 0 )); then
        printf "\033[0;31m❌ not installed: %s - the gates that need them will fail\033[0m\n" "${missing[*]}"
        exit 1
    fi
    printf "\033[0;32m✅ every gate tool is installed.\033[0m\n"

# Fail with the install command when cargo-xwin is missing (internal).
_require-xwin:
    @if ! command -v cargo-xwin >/dev/null 2>&1; then printf "\033[0;31m❌ cargo-xwin not found - install with: \033[0;36mjust install-dev-tools\033[0m\n"; exit 1; fi

# Fast staged-scoped gate (the pre-commit bundle; also runnable manually).
# Runs fmt-check (if *.rs staged), taplo-check (if *.toml staged), typos,
# reuse and the file-size-policy script in parallel.  Budget: sub-2 s.
# Every gate is hard: a missing tool fails it.

# The pre-commit gate bundle (staged-scoped, parallel).
lint-fast:
    @bash scripts/hooks/_lint_fast.sh

# Parallel pre-push gate (the pre-push bundle; also runnable manually).
# Fans out clippy (-D warnings) + fmt-check + rustdoc (-D warnings) +
# cargo deny + nextest --no-run (test-binary link check) + the smoke
# tests + file-size-policy + typos + reuse + the cross-target lints.
# Budget: ≈ 25–45 s warm, ≈ 60–90 s cold.  All failures surface in one
# pass - per-job ✅/❌ grid, then every failing job's full output.

# The pre-push gate bundle (workspace-wide, parallel then fail-fast).
lint-pre-push:
    @bash scripts/hooks/_lint_pre_push.sh

# Validate every non-merge commit subject in `origin/main..HEAD` against
# the same Conventional Commits regex CI runs on the PR title (see
# `.github/workflows/commitlint.yml`).  Same validator the pre-push
# `commit-subjects` job invokes; useful during a rebase / amend session
# before re-running `just lint-pre-push`.  No bypass.

# Check every commit subject on this branch against Conventional Commits.
check-commits:
    @printf "\033[0;34m📝 Checking commit subjects against Conventional Commits...\033[0m\n"
    @COMMIT_SUBJECT_VERBOSE=1 bash scripts/ci/check_commit_subjects.sh range origin/main..HEAD

# Install the tracked git hooks (pre-commit + commit-msg + pre-push).
# Idempotent - safe to re-run.  Sets `git config core.hooksPath` to the
# tracked `scripts/hooks/` directory so the hooks survive `git clone`
# and cannot be accidentally deleted from `.git/hooks/`.

# Install the tracked git hooks (core.hooksPath = scripts/hooks).
install-hooks:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🪝 Installing tracked git hooks...\033[0m\n"
    git config core.hooksPath scripts/hooks
    chmod +x scripts/hooks/pre-commit scripts/hooks/commit-msg scripts/hooks/pre-push \
             scripts/hooks/_lint_fast.sh scripts/hooks/_lint_pre_push.sh \
             scripts/ci/check_commit_subjects.sh
    printf "\033[0;32m✅ Git hooks installed:\033[0m\n"
    printf "   \033[0;36mpre-commit\033[0m → just lint-fast       (staged-scoped, sub-2 s)\n"
    printf "   \033[0;36mcommit-msg\033[0m → Conventional Commits  (subject-line, instant)\n"
    printf "   \033[0;36mpre-push\033[0m   → just lint-pre-push    (workspace parallel)\n"
    printf "   \033[0;33mGate failing?\033[0m Fix the cause (GETTING-STARTED has a fix-it table); bypassing is against policy.\n"
    printf "   \033[0;33mEvery gate is required\033[0m: install its tools with \`just install-dev-tools\`.\n"

# Uninstall the tracked git hooks (resets to .git/hooks/ default).
uninstall-hooks:
    @printf "\033[0;34m🪝 Uninstalling tracked git hooks...\033[0m\n"
    @# `git config --get` exits 1 for an unset key: nothing to unset then.
    @if git config --get core.hooksPath >/dev/null; then git config --unset core.hooksPath; fi
    @printf "\033[0;32m✅ Git hooks uninstalled - commits and pushes are no longer gated.\033[0m\n"

# Validate rustdoc links and warnings (catches private-intra-doc-links etc.).
#
# `--document-private-items` is REQUIRED for the "catches private-intra-doc-links"
# promise: without it rustdoc only validates links reachable from the public API
# surface, so a broken `[`crate::path::pub_crate_item`]` (or a `//!` shortcut to a
# private sibling) silently renders as dead text. Cross-platform: `#[cfg(windows)]`
# items absent on the macOS/Linux host are referenced as code spans, not links.

# Rustdoc with -D warnings and private items (broken intra-doc links fail).
rustdoc:
    @printf "\033[0;34m📚 Rustdoc link validation...\033[0m\n"
    RUSTDOCFLAGS='-Dwarnings' cargo doc --workspace --all-features --no-deps --document-private-items

# Enforce oversized Rust file policy with explicit exceptions.
file-size-policy:
    @printf "\033[0;34m📏 Checking oversized Rust file policy...\033[0m\n"
    bash scripts/ci/check_file_size_policy.sh

# Comprehensive code validation (CI/CD grade).
check:
    @printf "\033[0;34m🔍 Comprehensive validation (CI/CD grade)...\033[0m\n"
    just file-size-policy
    cargo check --workspace --all-targets --all-features
    just fmt-check
    @printf "\033[0;32m✅ Comprehensive check passed\033[0m\n"

# Quick code validation.
check-quick:
    @printf "\033[0;34m⚡ Quick validation (development speed)...\033[0m\n"
    cargo check --workspace


# Update all workspace dependencies to latest compatible versions.
update-deps:
    @printf "\033[0;34m📦 Updating all workspace dependencies...\033[0m\n"
    cargo update
    @printf "\033[0;32m✅ All dependencies updated\033[0m\n"

