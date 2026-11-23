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

# Format code (nightly rustfmt with edition 2024 + style_edition 2024).
fmt:
    @printf "\033[0;34m📝 Formatting code (nightly rustfmt, style_edition 2024)...\033[0m\n"
    cargo fmt --all
    @printf "\033[0;32m✅ Formatting complete\033[0m\n"

# Check code formatting without applying fixes.
fmt-check:
    @printf "\033[0;34m📝 Checking code formatting (nightly rustfmt, style_edition 2024)...\033[0m\n"
    cargo fmt --all -- --check

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
    cargo clippy --workspace --lib --bins --all-features --no-deps -- {{ prod_flags }}

# Pragmatic test linting (allows unwrap/expect in test code).
lint-tests:
    @printf "\033[0;34m🧪 Pragmatic test linting (FAST-FAIL)...\033[0m\n"
    cargo clippy --workspace --tests --all-features --no-deps -- {{ test_flags }}

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
    cargo xwin clippy --workspace --lib --bins --all-features --target x86_64-pc-windows-msvc --no-deps -- {{ prod_flags }}

# Pragmatic Windows test linting (allows unwrap/expect in test code).
lint-tests-windows: _require-xwin
    @printf "\033[0;34m🪟 Windows strict test lint (cargo xwin clippy)...\033[0m\n"
    cargo xwin clippy --workspace --tests --all-features --target x86_64-pc-windows-msvc --no-deps -- {{ test_flags }}

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
# only the rustup target must be installed (rust-toolchain.toml lists it,
# so `rustup show` brings it in).
#
# `target-cpu` is pinned to the `x86-64-v2` baseline for the same reason
# `lint-ci-linux-zig` pins `v3`: user-level `target-cpu=native` tuning
# (see .cargo/config.toml notes) would leak the arm64 host CPU name into
# an x86_64 build and corrupt codegen for C/asm in transitive deps.
#
# Host-guarded: cross-compiling TO darwin FROM Linux/Windows requires an
# Apple SDK those hosts do not have, so there the recipe says plainly
# that the check did not run.  On macOS a missing rustup target is an
# error with the install command (rust-toolchain.toml lists it, so
# `rustup show` normally brings it in).

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
# crates/md2html/Cargo.toml, the one place the number is written): a
# finding there is caught before the push instead of by CI's `msrv` job.
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
    RUSTFLAGS="-D warnings" cargo "+$version" check --locked -p md2html --all-targets
    printf "\033[0;32m✅ md2html builds on Rust %s\033[0m\n" "$version"

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
    @cargo run -q --release -p md2html-gen-hooks

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
    @cargo run -q --release -p md2html-gen-hooks -- --check

# Validates four structural properties of `.github/workflows/pr-fast.yml`
# against the canonical `scripts/ci/gates.toml`:
#   1. every pr-fast-tier gate has a corresponding job;
#   2. each job's `if:` predicate is at least as permissive as the
#      least-upper-bound of its folded gates' `gate_when` classes;
#   3. every gate's resolved job-id is present in `required.needs:`
#      and the bash `declare -A R=(...)` aggregator;
#   4. the `required` job's `name:` is exactly `PR Fast CI / required`
#      - the literal string in the repo's branch-protection rule.
#
# Read-only: never mutates the workflow file.  Wired into the pre-push
# hook and the `pr-fast.yml::workflow-drift` job so any drift
# hard-blocks merge.

# Workflow drift: pr-fast.yml (and every workflow's shared pins) agree with gates.toml.
workflow-drift:
    @cargo run -q --release -p md2html-gen-workflow -- --check

# Same `md2html-gen-hooks` binary as the pre-push generator; `--target
# pre-commit` selects the `_lint_fast.sh` template + dispatch shape.
# Use this recipe whenever you change a `tier="pre-commit"` gate in
# the manifest; the on-disk hook carries an AUTO-GENERATED banner and
# any manual edits will be overwritten on the next regen.
#
# Idempotent; the `fast-drift` recipe below enforces the same property.

# Regenerate the pre-commit hook from scripts/ci/gates.toml.
gen-fast:
    @cargo run -q --release -p md2html-gen-hooks -- --target pre-commit

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
    @cargo run -q --release -p md2html-gen-hooks -- --target pre-commit --check

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
#   cargo-vet                                 the version CI pins, built with stable
#   cargo-nextest, cargo-deny                 cargo install --locked
#   zig 0.17.1 + cargo-zigbuild               the official ziglang.org tarball,
#                                             SHA-256 checked (macOS and Linux)
#   reuse                                     pipx
#   the x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu and
#   x86_64-apple-darwin rustup targets

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
    cargo_tool cargo-deny cargo-deny
    cargo_tool cargo-zigbuild cargo-zigbuild
    # cargo-vet at the version the CI jobs pin (`[toolchain] cargo-vet` in
    # scripts/ci/gates.toml), built with stable as CI builds it.
    vet_pin="$(sed -n 's/^cargo-vet = "\(.*\)"$/\1/p' scripts/ci/gates.toml)"
    [[ -n "$vet_pin" ]] || { printf "\033[0;31m❌ no cargo-vet pin in scripts/ci/gates.toml\033[0m\n"; exit 1; }
    vet_have=""
    if command -v cargo-vet >/dev/null 2>&1; then
        vet_have="$(cargo vet --version)"
    fi
    if [[ "$vet_have" == "cargo-vet ${vet_pin}" ]]; then
        printf "   \033[0;32m✓\033[0m cargo-vet %s already installed\n" "$vet_pin"
    else
        rustup toolchain install stable --profile minimal --no-self-update
        cargo +stable install --locked --force "cargo-vet@${vet_pin}"
    fi
    # The cross-lint targets (rust-toolchain.toml lists them; `rustup
    # target add` is idempotent and fails loudly on a real problem).
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

