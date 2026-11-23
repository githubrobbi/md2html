# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Watch mode development.
dev:
    @printf "\033[0;34m🔄 Starting watch mode...\033[0m\n"
    just _install-if-missing cargo-watch cargo-watch
    cargo watch -x "nextest run --workspace --jobs 16" -x "clippy --workspace --all-targets --all-features --no-deps -- {{ test_flags }}"

# Clean build artifacts.
clean:
    @printf "\033[0;34m🧹 Cleaning build artifacts...\033[0m\n"
    cargo clean

# Stop the cargo and rustc processes of THIS checkout: every cargo whose
# working directory is inside it, and everything those cargos started
# (rustc, build scripts, test binaries). Builds of other repositories on
# the machine are left alone.

# Stop this checkout's cargo builds and everything they started (other repositories untouched).
[unix]
kill:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(pwd -P)"
    # pgrep exits 1 when nothing matches: that is the "nothing running"
    # answer, not an error.  Anything above 1 is a real failure.
    pgrep_or_none() {
        local rc=0
        pgrep "$@" || rc=$?
        if (( rc > 1 )); then
            printf "\033[0;31m❌ pgrep %s failed (exit %d)\033[0m\n" "$*" "$rc" >&2
            exit 1
        fi
    }
    # The working directory of a process; nothing when the process has
    # exited since it was listed (a race, not an error).  A lookup that
    # fails while the process still exists IS an error.
    cwd_of() {
        local pid="$1" dir
        if [[ -d "/proc/$pid" ]]; then
            if dir="$(readlink "/proc/$pid/cwd")"; then
                printf '%s\n' "$dir"
            elif [[ -d "/proc/$pid" ]]; then
                return 1
            fi
        elif kill -0 "$pid" 2>/dev/null; then
            if dir="$(lsof -a -p "$pid" -d cwd -Fn)"; then
                printf '%s\n' "$dir" | sed -n 's/^n//p'
            elif kill -0 "$pid" 2>/dev/null; then
                return 1
            fi
        fi
    }
    targets=()
    # Assigned first, so a pgrep failure stops the recipe instead of
    # reading as "no process".
    cargo_pids="$(pgrep_or_none -x cargo)"
    for pid in $cargo_pids; do
        dir="$(cwd_of "$pid")"
        if [[ "$dir" == "$root" || "$dir" == "$root"/* ]]; then
            # The cargo and its whole process tree, breadth first.
            queue=("$pid")
            while (( ${#queue[@]} > 0 )); do
                parent="${queue[0]}"
                queue=("${queue[@]:1}")
                targets+=("$parent")
                children="$(pgrep_or_none -P "$parent")"
                for child in $children; do
                    queue+=("$child")
                done
            done
        fi
    done
    if (( ${#targets[@]} == 0 )); then
        printf "\033[0;32m✅ no cargo process of this checkout is running\033[0m\n"
        exit 0
    fi
    printf "\033[0;31m🚨 stopping %d process(es) of this checkout: %s\033[0m\n" "${#targets[@]}" "${targets[*]}"
    failed=0
    for pid in "${targets[@]}"; do
        # A process that ended between the listing and now is fine; one
        # that is still there after a refused kill (permissions) is not.
        if ! kill "$pid" 2>/dev/null && ps -p "$pid" >/dev/null; then
            printf "\033[0;31m❌ could not stop process %s\033[0m\n" "$pid" >&2
            failed=1
        fi
    done
    if (( failed )); then
        exit 1
    fi
    printf "\033[0;32m✅ stopped\033[0m\n"

# Install the cross-compilation targets rust-toolchain.toml lists.
setup-cross:
    @printf "\033[0;34m Installing cross-compilation targets...\033[0m\n"
    rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-msvc x86_64-apple-darwin aarch64-apple-darwin
    @printf "\033[0;32m✅ Cross-compilation targets installed\033[0m\n"

# Cross-compile check for GitHub Actions compatibility.
check-cross:
    @printf "\033[0;34m Cross-compilation validation using CI pipeline...\033[0m\n"
    cargo run -q --release -p md2html-ci-pipeline -- cross-check

# Type-check all #[cfg(windows)] code paths from a macOS or Linux host
# using cargo-xwin: missing imports, wrong types and dead code in
# Windows-only blocks that a host `cargo check` skips.  `--all-targets
# --all-features` checks tests alongside libs and bins.  pr-fast.yml's
# `windows-lint` job runs the strict clippy natively on windows-latest;
# this recipe is the local mirror, ~2-5 s warm once xwin has cached the
# MSVC SDK.

# Type-check the #[cfg(windows)] code paths from this host with cargo-xwin.
check-windows:
    @if ! command -v cargo-xwin >/dev/null 2>&1; then printf "\033[0;31m❌ cargo-xwin not found - install with: \033[0;36mjust install-dev-tools\033[0m\n"; exit 1; fi
    @printf "\033[0;34m🪟 Windows cross-check (cargo xwin check, all-targets all-features)...\033[0m\n"
    cargo xwin check --workspace --all-targets --all-features --target x86_64-pc-windows-msvc
    @printf "\033[0;32m✅ Windows cross-check passed\033[0m\n"

# Cross-platform compile check across all four supported platforms
# (macOS ARM native, macOS Intel cross, Linux, Windows), the manual
# "pre-PR sweep" of lane:cross-lint (docs/forge/COMPONENTS.md).
#
# Every leg runs or the recipe fails: a missing tool is an error with
# the install command, never a skipped leg.  The one leg that cannot
# exist on a host is Intel macOS off macOS (no Apple SDK); the
# lint-ci-mac-intel recipe says so plainly.  Linux runs through
# cargo-zigbuild (`just lint-ci-linux-zig`); `just lint-ci-linux` is the
# Docker alternative that mirrors CI's image exactly.

# Compile and lint for macOS ARM + Intel, Linux and Windows from this host.
check-all-targets:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🌐 Cross-platform compile check (macOS ARM + Intel, Linux, Windows)...\033[0m\n"
    printf "\033[0;36m   ── native host ──\033[0m\n"
    cargo check --workspace --all-targets --all-features
    printf "\033[0;32m   ✅ native host\033[0m\n\n"
    printf "\033[0;36m   ── macOS Intel x86_64-apple-darwin ──\033[0m\n"
    just lint-ci-mac-intel
    printf "\n\033[0;36m   ── Windows x86_64-pc-windows-msvc (xwin) ──\033[0m\n"
    just check-windows
    printf "\n\033[0;36m   ── Linux x86_64-unknown-linux-gnu (zigbuild) ──\033[0m\n"
    just lint-ci-linux-zig
    printf "\033[0;32m✅ Cross-platform check complete\033[0m\n"


# Update everything: Rust toolchain, cargo tools, system packages.
# Uses cargo-binstall (pre-built binaries) when available for ~100× faster installs.
# Run before `just go` or `just ship` to stay cutting-edge.
[unix]
update-tools:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[1;34m🔄 Updating all development tools\033[0m\n"
    echo "========================================================"

    # ── 1. Rust toolchain ──────────────────────────────────────
    printf "\n\033[0;34m🦀 Step 1/3: Rust toolchain\033[0m\n"
    if command -v brew >/dev/null 2>&1 && brew list --formula rustup >/dev/null 2>&1; then
        printf "  → rustup managed by Homebrew\n"
        brew upgrade rustup
    elif ! out="$(rustup self update 2>&1)"; then
        # A distribution's rustup disables self-update on purpose; that
        # is the package manager's job, not an error.  Anything else is.
        if [[ "$out" == *"self-update is disabled"* ]]; then
            printf "  → rustup is managed by the system package manager (self-update disabled)\n"
        else
            printf '%s\n' "$out" >&2
            exit 1
        fi
    fi
    rustup update
    just toolchain-sync
    printf "\033[0;32m  ✅ Rust toolchain up to date\033[0m\n"

    # ── 2. Cargo tools ─────────────────────────────────────────
    printf "\n\033[0;34m📦 Step 2/3: Cargo tools\033[0m\n"
    TOOLS=(
        cargo-nextest
        cargo-llvm-cov
        cargo-deny
        cargo-xwin
        cargo-zigbuild
        sccache
        cargo-machete
        cargo-outdated
        cargo-mutants
        cargo-watch
        typos-cli
        taplo-cli
        flamegraph
    )
    if command -v cargo-binstall >/dev/null 2>&1; then
        printf "  → Using cargo-binstall (fast, pre-built binaries)\n"
        cargo binstall --no-confirm "${TOOLS[@]}"
    else
        printf "  → Using cargo install (compiling from source - slower)\n"
        printf "  💡 Install cargo-binstall for ~100× faster updates: cargo install cargo-binstall\n"
        cargo install --locked "${TOOLS[@]}"
    fi
    # cargo-vet is built with stable, at the version the CI jobs pin
    # (`[toolchain] cargo-vet` in scripts/ci/gates.toml).
    vet_pin="$(sed -n 's/^cargo-vet = "\(.*\)"$/\1/p' scripts/ci/gates.toml)"
    [[ -n "$vet_pin" ]] || { printf "\033[0;31m❌ no cargo-vet pin in scripts/ci/gates.toml\033[0m\n"; exit 1; }
    rustup toolchain install stable --profile minimal --no-self-update
    cargo +stable install --locked "cargo-vet@${vet_pin}"
    printf "\033[0;32m  ✅ Cargo tools up to date\033[0m\n"

    # ── 3. System packages ─────────────────────────────────────
    printf "\n\033[0;34m🍺 Step 3/3: System packages\033[0m\n"
    if command -v brew >/dev/null 2>&1 && brew list --formula just >/dev/null 2>&1; then
        brew upgrade just
        printf "\033[0;32m  ✅ just up to date\033[0m\n"
    else
        printf "  → just is not a Homebrew formula here; update it the way it was installed\n"
    fi

    echo ""
    printf "\033[1;32m🎉 All tools updated!\033[0m\n"
    rustc --version
    cargo nextest --version
    cargo llvm-cov --version

[windows]
update-tools:
    #!powershell
    $ErrorActionPreference = 'Stop'
    # PowerShell does not stop on a failing native command: every call
    # below goes through Invoke-Checked, which throws on a non-zero exit.
    function Invoke-Checked {
        param([string]$Exe, [string[]]$Arguments)
        & $Exe @Arguments
        if ($LASTEXITCODE -ne 0) { throw "$Exe $($Arguments -join ' ') failed (exit $LASTEXITCODE)" }
    }
    Write-Host "`n[update-tools] Updating all development tools" -ForegroundColor Blue
    Write-Host "========================================================"

    # --- 1. Rust toolchain ------------------------------------
    Write-Host "`n[Rust] Step 1/3: Rust toolchain" -ForegroundColor Blue
    Invoke-Checked rustup @('self', 'update')
    Invoke-Checked rustup @('update')
    Invoke-Checked just @('toolchain-sync')
    Write-Host "  [OK] Rust toolchain up to date" -ForegroundColor Green

    # --- 2. Cargo tools ---------------------------------------
    Write-Host "`n[Pkg] Step 2/3: Cargo tools" -ForegroundColor Blue
    $tools = @(
        "cargo-nextest"
        "cargo-llvm-cov"
        "cargo-deny"
        "sccache"
        "cargo-machete"
        "cargo-outdated"
        "cargo-mutants"
        "cargo-watch"
        "typos-cli"
        "taplo-cli"
    )
    if (Get-Command cargo-binstall -ErrorAction SilentlyContinue) {
        Write-Host "  -> Using cargo-binstall (fast, pre-built binaries)"
        Invoke-Checked cargo (@('binstall', '--no-confirm') + $tools)
    } else {
        Write-Host "  -> Using cargo install (compiling from source - slower)"
        Write-Host "  [TIP] Install cargo-binstall for faster updates: cargo install cargo-binstall" -ForegroundColor Yellow
        Invoke-Checked cargo (@('install', '--locked') + $tools)
    }
    Write-Host "  [OK] Cargo tools up to date" -ForegroundColor Green

    # --- 3. System packages -----------------------------------
    Write-Host "`n[Pkg] Step 3/3: System packages" -ForegroundColor Blue
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Invoke-Checked winget @('upgrade', '--id', 'Casey.Just', '--exact')
    } elseif (Get-Command scoop -ErrorAction SilentlyContinue) {
        Invoke-Checked scoop @('update', 'just')
    } else {
        Write-Host "  -> No winget or scoop: update just the way it was installed"
    }
    Write-Host "  [OK] System packages up to date" -ForegroundColor Green

    Write-Host "`n[DONE] All tools updated!" -ForegroundColor Green
    Invoke-Checked rustc @('--version')
    Invoke-Checked cargo @('nextest', '--version')

# Replicate Tier 2's failure-prone legs LOCALLY, before any dispatch: CI
# validates last, it never debugs. Prerequisite once:
# `cargo install cargo-udeps --version 0.1.61 --locked` - the SAME
# version tier-2.yml pins; "latest" is how a lagging release bites. The
# mutants leg is the honest cost of local validation; scope it with
# `-f crates/md2html/src/<file>.rs` while iterating and run the full
# leg before the dispatch.
#
# cargo-mutants exits 0 (all caught), 2 (a mutant was missed), 3 (some
# timed out - a timeout is a kill: the mutant could not ship unnoticed)
# or anything else for a broken run (no baseline, bad arguments).  The
# verdict reads both the exit code and mutants.out/missed.txt, exactly
# as tier-2.yml does.

# Replicate the Tier 2 udeps + mutants legs locally before dispatching CI.
tier2-local:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v cargo-udeps >/dev/null 2>&1; then
        printf "\033[0;31m❌ cargo-udeps not found - install with: \033[0;36mcargo install cargo-udeps --version 0.1.61 --locked\033[0m\n"
        exit 1
    fi
    just _install-if-missing cargo-mutants cargo-mutants
    cargo fetch --locked
    cargo udeps --locked --workspace --all-targets --all-features
    status=0
    cargo mutants --package md2html -f crates/md2html/src/sanitize.rs -f crates/md2html/src/text.rs --jobs 4 --no-shuffle || status=$?
    if [[ -s mutants.out/missed.txt ]]; then
        echo "MISSED MUTANTS:"
        cat mutants.out/missed.txt
        exit 1
    fi
    if [[ "$status" -ne 0 && "$status" -ne 3 ]]; then
        printf "\033[0;31m❌ cargo mutants failed (exit %d) - read its output above\033[0m\n" "$status"
        exit "$status"
    fi
    echo "mutants: no escapes (timeouts are kills)"

# HTML's named character references, md2html's table of them
# (crates/md2html/src/inline/entity/names*), written from the WHATWG list
# that Python's standard library carries (`html.entities.html5`).

# Write md2html's table of HTML's named character references.
entities:
    python3 scripts/gen_entities.py

# Fail when md2html's table of named character references is out of date.
entities-check:
    python3 scripts/gen_entities.py --check
