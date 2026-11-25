# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Watch mode development.
dev:
    @printf "\033[0;34m🔄 Starting watch mode...\033[0m\n"
    just _install-if-missing cargo-watch cargo-watch
    cargo watch -x "nextest run --workspace --jobs 16" -x "clippy --workspace --all-targets --all-features --no-deps -- {{ lint_flags }}"

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

# Install the cross-compilation targets (`toolchain_targets` in just/shared.just).
setup-cross:
    @printf "\033[0;34m Installing cross-compilation targets...\033[0m\n"
    rustup target add {{ toolchain_targets }}
    @printf "\033[0;32m✅ Cross-compilation targets installed\033[0m\n"

# Cross-compile check for GitHub Actions compatibility.
check-cross:
    @printf "\033[0;34m Cross-compilation validation using CI pipeline...\033[0m\n"
    cargo run -q --profile tooling -p md2html-ci-pipeline -- cross-check

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
# validates last, it never debugs. The files mutated are those of
# scripts/ci/mutants-scope.txt, the list tier-2.yml reads. Prerequisite once:
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
    scope=()
    while IFS= read -r glob; do
        [[ -z "$glob" || "$glob" == \#* ]] || scope+=(-f "$glob")
    done < scripts/ci/mutants-scope.txt
    cargo mutants --package md2html "${scope[@]}" --jobs 4 --no-shuffle || status=$?
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
# (crates/md2html/src/text/entity/names*), written from the WHATWG's list
# vendored in scripts/entities/entities.json (Python 3.8 or later, standard
# library only).  The gate `entities-drift` runs the check.

# Write md2html's table of HTML's named character references.
entities:
    python3 scripts/gen_entities.py

# Fail when md2html's table of named character references is out of date.
entities-check:
    python3 scripts/gen_entities.py --check

# Capture a workspace baseline snapshot under docs/dev/baseline/<date>/:
# the measured starting point of the code-quality programme (Phase 0 of
# docs/dev/architecture/code_clean/world_class_rust_workspace_refactor_playbook.md,
# local only like everything under docs/dev/).  Re-run it after each
# phase and compare the folders.
#
# What it records, per crate as `cargo metadata` lists them (no crate
# list is written here): commit and environment, workspace metadata,
# risk markers (production code lines, unsafe, async, threads and
# locks, platform cfg, FFI, environment and process sites, pub items at
# the root, unwrap / expect / panic family in production code, lint
# exceptions, build.rs, crate docs), the largest files, the pub and
# pub(crate) surface, test counts (unit, integration, doc, the
# CommonMark examples), features, the dependency graph and its
# duplicates, the CI workflows and gates, and the supply chain
# (`cargo vet --locked`, `cargo deny check`).
#
# It stays cheap (under a minute warm): it lists tests but does not run
# them, and leaves check, clippy, rustdoc and the test run to `just go`
# and CI.  "Production code" is a crate's src/ without its test files
# (tests.rs, tests/, spec_tests.rs) and without the `#[cfg(test)] mod`
# block that closes a file; comment lines are not counted.
#
# Every measurement either succeeds or stops the recipe.  A supply-chain
# check that fails is a finding: its output is kept, the snapshot is
# finished, and the recipe then exits 1 naming it.
#
#   just baseline-snapshot              # today's date (UTC)
#   just baseline-snapshot 2026-11-23   # a given date (re-running overwrites)

# Capture a baseline snapshot of the workspace under docs/dev/baseline/<date>/.
[unix]
baseline-snapshot DATE=`date -u +%Y-%m-%d`:
    #!/usr/bin/env bash
    set -euo pipefail
    day="{{ DATE }}"
    if [[ ! "$day" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
        printf "\033[0;31m❌ DATE must be YYYY-MM-DD, got '%s'\033[0m\n" "$day" >&2
        exit 2
    fi
    for tool in jq cargo-nextest cargo-vet cargo-deny; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            printf "\033[0;31m❌ %s not found - run: \033[0;36mjust install-dev-tools\033[0m\n" "$tool" >&2
            exit 2
        fi
    done
    B="docs/dev/baseline/{{ DATE }}"
    mkdir -p "$B"
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    started=$(date +%s)
    sha="$(git rev-parse --short HEAD)"
    stamp="Captured $(date -u +%FT%TZ) at commit \`$sha\` by \`just baseline-snapshot {{ DATE }}\`."
    printf "\033[0;34m📸 Baseline snapshot -> %s\033[0m\n" "$B"

    # Occurrences of an extended regex on stdin (0 when there is none).
    # awk, not grep: a pattern with no match is an answer, not an error.
    occurrences() {
        PAT="$1" awk '{ n += gsub(ENVIRON["PAT"], "") } END { print n + 0 }'
    }
    # Lines of stdin (0 for none).
    lines() {
        awk 'END { print NR + 0 }'
    }
    # The code lines of the files named on stdin: blank and comment lines
    # dropped. MODE=prod stops a file at its closing `#[cfg(test)] mod`
    # block, MODE=test keeps only that block, MODE=all keeps every line.
    code_lines() {
        local mode="$1" list
        list="$(cat)"
        if [[ -z "$list" ]]; then
            return 0
        fi
        local files=()
        mapfile -t files <<< "$list"
        MODE="$mode" awk '
            FNR == 1 { intest = 0; armed = 0 }
            /^[[:space:]]*$/ || /^[[:space:]]*\/\// { next }
            armed && /^mod [A-Za-z_][A-Za-z0-9_]*[[:space:]]*\{/ { intest = 1 }
            { armed = ($0 ~ /^#\[cfg\(test\)\][[:space:]]*$/) }
            armed { next }
            ENVIRON["MODE"] == "all" || (ENVIRON["MODE"] == "prod" && !intest) || (ENVIRON["MODE"] == "test" && intest) { print }
        ' "${files[@]}"
    }
    # grep -nE that tells "no line matched" (exit 1, an answer) from a
    # failure (exit 2 and above, which stops the recipe).
    grep_lines() {
        local rc=0
        grep -nE "$@" || rc=$?
        if (( rc > 1 )); then
            printf "grep failed (exit %d): %s\n" "$rc" "$*" >&2
            exit 2
        fi
    }
    # The lines of stdin that do (keep) or do not (drop) match a regex.
    keep() { PAT="$1" awk '$0 ~ ENVIRON["PAT"]'; }
    drop() { PAT="$1" awk '$0 !~ ENVIRON["PAT"]'; }
    test_path='(/tests\.rs|/tests/|/spec_tests\.rs|_tests\.rs)$|/(tests|benches|examples)/'

    # ── Commit and environment ────────────────────────────────────────────
    { git rev-parse HEAD; git log -1 --format='%h %s (%cs)'; } > "$B/_commit.txt"
    {
        echo "Captured: $(date -u +%FT%TZ)"
        echo
        echo "## Host"
        uname -mrs
        if command -v sw_vers >/dev/null 2>&1; then sw_vers; fi
        echo
        echo "## Toolchain"
        rustc --version --verbose
        cargo --version
        echo
        echo "## Tools"
        cargo nextest --version | head -1
        cargo vet --version
        cargo deny --version
        just --version
        jq --version
    } > "$B/_env.txt"

    # ── Workspace metadata ────────────────────────────────────────────────
    cargo metadata --format-version 1 --no-deps --locked > "$B/_metadata.json"
    root="$(jq -r '.workspace_root' "$B/_metadata.json")"
    # name, directory (relative), kinds, root source file, edition,
    # rust-version, publish, feature count: one tab-separated line per crate.
    jq -r --arg root "$root/" '
        .packages | sort_by(.manifest_path)[] |
        [ .name,
          (.manifest_path | sub("/Cargo.toml$"; "") | ltrimstr($root)),
          ([.targets[].kind[]] | map(select(. != "custom-build")) | unique | join("+")),
          ((.targets | map(select(any(.kind[]; . == "lib"))) + map(select(any(.kind[]; . == "bin"))))[0].src_path | ltrimstr($root)),
          .edition,
          (.rust_version // "-"),
          (if .publish == [] then "no" else "yes" end),
          (.features | keys | map(select(. != "default")) | length)
        ] | @tsv' "$B/_metadata.json" > "$work/crates.tsv"
    printf "  • %s crates from cargo metadata\n" "$(lines < "$work/crates.tsv")"

    # ── Per-crate measurements ────────────────────────────────────────────
    printf "  • workspace_inventory.md, risk_markers.md, api_surface.md "
    inv="$work/inventory.rows"; risk="$work/risk.rows"; api="$work/api.rows"
    : > "$inv"; : > "$risk"; : > "$api"
    while IFS=$'\t' read -r name dir kinds rootfile edition msrv publish nfeat; do
        git ls-files -- "$dir/*.rs" > "$work/all"
        keep "^$dir/src/" < "$work/all" | drop "$test_path" > "$work/prodfiles"
        keep "$test_path" < "$work/all" > "$work/testfiles"
        code_lines prod < "$work/prodfiles" > "$work/prod"
        { code_lines test < "$work/prodfiles"; code_lines all < "$work/testfiles"; } > "$work/test"
        prod_n=$(lines < "$work/prod"); test_n=$(lines < "$work/test")
        unsafe_n=$(occurrences 'unsafe[[:space:]]*\{|unsafe[[:space:]]+(fn|impl|trait|extern)' < "$work/prod")
        async_n=$(occurrences 'async[[:space:]]+(fn|move|\{)|\.await' < "$work/prod")
        sync_n=$(occurrences 'thread::(spawn|scope|Builder)|Mutex|RwLock|Condvar|mpsc|Atomic[A-Z][a-z]+|Arc<|Arc::' < "$work/prod")
        cfg_n=$(occurrences 'cfg(_attr)?!?\([^)]*(windows|unix|target_)' < "$work/prod")
        ffi_n=$(occurrences 'extern[[:space:]]+"|#\[link|no_mangle|libc::' < "$work/prod")
        env_n=$(occurrences 'env::var(_os)?\(' < "$work/prod")
        proc_n=$(occurrences 'Command::new' < "$work/prod")
        unwrap_n=$(occurrences '\.unwrap\(\)' < "$work/prod")
        expect_n=$(occurrences '\.expect\(' < "$work/prod")
        panic_n=$(occurrences '(panic|todo|unimplemented|unreachable)!\(' < "$work/prod")
        assert_n=$(occurrences '(^|[^_a-z])assert(_eq|_ne)?!\(' < "$work/prod")
        # Lint exceptions anywhere in the crate, tests included (the gate's rule).
        mapfile -t crate_rs < "$work/all"
        code_all="$(cat "${crate_rs[@]}" | drop '^[[:space:]]*//')"
        exc_n=$(occurrences '#!?\[[[:space:]]*(allow|expect)[[:space:]]*\(|cfg_attr\(.*[^.a-z_](allow|expect)\(' <<< "$code_all")
        tests_n=$(occurrences '#\[test\]' <<< "$code_all")
        build=no
        if [[ -f "$dir/build.rs" ]]; then build=yes; fi
        docs=$(head -20 "$rootfile" | keep '^//!' | lines)
        if (( docs > 0 )); then docs=yes; else docs=no; fi
        lints=$(awk '/^\[lints\]/ { l = 1; next } /^\[/ { l = 0 } l && /^workspace[[:space:]]*=[[:space:]]*true/ { print "yes"; exit }' "$dir/Cargo.toml")
        lints=${lints:-no}
        # Root-file visibility: bare pub, and pub(crate)/pub(super).
        root_pub=$(drop '^[[:space:]]*//' < "$rootfile" | occurrences '^pub ')
        root_pubc=$(drop '^[[:space:]]*//' < "$rootfile" | occurrences '^pub\((crate|super)\) ')
        # Crate-wide visibility of production items and fields.
        vis_pub=$(occurrences '^[[:space:]]*pub[[:space:]]+(async[[:space:]]+)?(const[[:space:]]+)?(fn|struct|enum|trait|type|const|static|mod|use|macro)[[:space:]]' < "$work/prod")
        vis_crate=$(occurrences '^[[:space:]]*pub\(crate\)[[:space:]]+(async[[:space:]]+)?(const[[:space:]]+)?(fn|struct|enum|trait|type|const|static|mod|use)[[:space:]]' < "$work/prod")
        vis_super=$(occurrences '^[[:space:]]*pub\((super|in [^)]*)\)[[:space:]]+(async[[:space:]]+)?(const[[:space:]]+)?(fn|struct|enum|trait|type|const|static|mod|use)[[:space:]]' < "$work/prod")
        field_pub=$(occurrences '^[[:space:]]+pub(\((crate|super)\))?[[:space:]]+[a-z_][a-z0-9_]*[[:space:]]*:' < "$work/prod")
        mods_pub=$(occurrences '^[[:space:]]*pub(\((crate|super)\))?[[:space:]]+mod[[:space:]]' < "$work/prod")
        must_use=$(occurrences '#\[must_use' < "$work/prod")
        non_exh=$(occurrences '#\[non_exhaustive\]' < "$work/prod")
        traits=$(occurrences '^[[:space:]]*(pub(\([a-z ]*\))?[[:space:]]+)?trait[[:space:]]' < "$work/prod")
        impls=$(occurrences '^[[:space:]]*impl(<[^>]*>)?[[:space:]]+[A-Za-z_:<>]+[[:space:]]+for[[:space:]]' < "$work/prod")
        dyn_n=$(occurrences 'dyn[[:space:]]+[A-Z]' < "$work/prod")
        macros=$(occurrences 'macro_rules!' < "$work/prod")
        clones=$(occurrences '\.clone\(\)' < "$work/prod")
        owned=$(occurrences '\.to_owned\(\)|\.to_string\(\)|String::from|format!\(' < "$work/prod")
        # Declared dependencies by kind, and the workspace crates that use this one.
        deps=$(jq -r --arg n "$name" '.packages[] | select(.name == $n) | .dependencies |
            "\(map(select(.kind == null)) | length)/\(map(select(.kind == "build")) | length)/\(map(select(.kind == "dev")) | length)"' "$B/_metadata.json")
        rdeps=$(jq -r --arg n "$name" '[.packages[] | select(any(.dependencies[]; .name == $n)) | .name] | length' "$B/_metadata.json")
        printf "| \`%s\` | \`%s\` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n" \
            "$name" "$dir" "$kinds" "$edition" "$msrv" "$publish" "$lints" "$deps" "$rdeps" "$build" "$docs" "$nfeat" "$tests_n" >> "$inv"
        printf "| \`%s\` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n" \
            "$name" "$prod_n" "$test_n" "$unsafe_n" "$async_n" "$sync_n" "$cfg_n" "$ffi_n" "$env_n" "$proc_n" \
            "$unwrap_n" "$expect_n" "$panic_n" "$assert_n" "$exc_n" >> "$risk"
        printf "| \`%s\` | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n" \
            "$name" "$root_pub" "$root_pubc" "$vis_pub" "$vis_crate" "$vis_super" "$field_pub" "$mods_pub" \
            "$must_use" "$non_exh" "$traits" "$impls" "$dyn_n" "$macros" "$clones / $owned" >> "$api"
    done < "$work/crates.tsv"
    {
        echo "# Workspace inventory"
        echo
        echo "$stamp"
        echo
        echo "Deps are declared dependencies normal/build/dev; reverse deps count the workspace crates that declare this one."
        echo
        echo "| crate | path | targets | edition | rust-version | publish | lints inherited | deps n/b/d | reverse deps | build.rs | crate docs | features | #[test] |"
        echo "|---|---|---|---|---|---|---|---|---:|---|---|---:|---:|"
        cat "$inv"
    } > "$B/workspace_inventory.md"
    {
        echo "# Risk markers per crate"
        echo
        echo "$stamp"
        echo
        echo "Counts are occurrences in production code (src/ without test files and closing test modules; comments and blank lines dropped)."
        echo "Lint exceptions count #[allow]/#[expect] attributes in every Rust file of the crate, tests included."
        echo
        echo "| crate | code lines | test lines | unsafe | async | threads/locks | platform cfg | FFI | env reads | processes | unwrap | expect | panic family | assert | lint exceptions |"
        echo "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
        cat "$risk"
        echo
        echo "## Lint exceptions (file:line)"
        echo
        echo '```text'
        mapfile -t rs < <(git ls-files -- '*.rs')
        grep_lines '#!?\[[[:space:]]*(allow|expect)[[:space:]]*\(' "${rs[@]}" | drop '^[^:]*:[0-9]+:[[:space:]]*//'
        echo '```'
    } > "$B/risk_markers.md"
    {
        echo "# Visibility and API surface"
        echo
        echo "$stamp"
        echo
        echo "Root = the crate's lib.rs, or main.rs for a binary. Crate-wide counts are production items. Fields counts pub/pub(crate)/pub(super) fields."
        echo
        echo "| crate | root pub | root pub(crate/super) | pub items | pub(crate) items | pub(super/in) items | non-private fields | non-private mods | #[must_use] | #[non_exhaustive] | traits | trait impls | dyn | macro_rules! | clone() / owned |"
        echo "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|"
        cat "$api"
    } > "$B/api_surface.md"
    printf "✓\n"

    # ── File sizes ────────────────────────────────────────────────────────
    printf "  • file_sizes.md "
    git ls-files -- '*.rs' | xargs wc -l | drop ' total$' | sort -rn > "$work/sizes"
    {
        echo "# Rust file sizes (lines, limit 800)"
        echo
        echo "$stamp"
        echo
        echo "| band | files |"
        echo "|---|---:|"
        awk '{ b = $1 >= 700 ? "700-800" : $1 >= 600 ? "600-699" : $1 >= 400 ? "400-599" : $1 >= 200 ? "200-399" : "0-199"; c[b]++ }
             END { split("700-800 600-699 400-599 200-399 0-199", o, " "); for (i = 1; i <= 5; i++) printf "| %s | %d |\n", o[i], c[o[i]] + 0 }' "$work/sizes"
        echo "| all | $(lines < "$work/sizes") |"
        echo
        echo "## The 30 largest"
        echo
        echo "| lines | file |"
        echo "|---:|---|"
        head -30 "$work/sizes" | awk '{ printf "| %d | `%s` |\n", $1, $2 }'
    } > "$B/file_sizes.md"
    printf "✓\n"

    # ── Tests (listed, not run) ───────────────────────────────────────────
    printf "  • test_count.md "
    cargo nextest list --workspace --locked --message-format json > "$B/_tests_listed.json"
    cargo test --workspace --doc --locked -- --list > "$B/_doctests.txt" 2>&1
    spec="$(git ls-files -- '*spec_tests.rs' | head -1)"
    {
        echo "# Test inventory"
        echo
        echo "$stamp"
        echo
        echo "Unit and integration tests from \`cargo nextest list --workspace\`; doc tests from \`cargo test --workspace --doc -- --list\`."
        echo
        echo "| suite | kind | tests |"
        echo "|---|---|---:|"
        jq -r '."rust-suites" | to_entries[] | "| `\(.key)` | \(.value.kind) | \(.value.testcases | length) |"' "$B/_tests_listed.json"
        echo
        echo "| measure | count |"
        echo "|---|---:|"
        echo "| unit + integration (nextest) | $(jq -r '."test-count"' "$B/_tests_listed.json") |"
        echo "| in integration-test targets | $(jq -r '[."rust-suites"[] | select(.kind == "test") | .testcases | length] | add // 0' "$B/_tests_listed.json") |"
        echo "| doc tests | $(occurrences ': test$' < "$B/_doctests.txt") |"
        if [[ -n "$spec" ]]; then
            echo "| CommonMark examples in the spec (\`$spec\` EXAMPLES) | $(awk -F'= *|;' '/^const EXAMPLES: usize/ { print $2 }' "$spec") |"
            echo "| CommonMark examples that must pass (BASELINE floor) | $(awk -F'= *|;' '/^const BASELINE: usize/ { print $2 }' "$spec") |"
        fi
    } > "$B/test_count.md"
    printf "✓\n"

    # ── Features ──────────────────────────────────────────────────────────
    printf "  • feature_inventory.md, feature_tree.txt "
    {
        echo "# Feature flags"
        echo
        echo "$stamp"
        echo
        echo "| crate | feature | enables | default |"
        echo "|---|---|---|---|"
        jq -r '.packages | sort_by(.name)[] | .name as $n | (.features.default // []) as $d | .features | to_entries[] | select(.key != "default") |
            "| `\($n)` | `\(.key)` | \(if .value == [] then "-" else (.value | join(", ")) end) | \(if (.key as $k | $d | any(.[]; . == $k)) then "yes" else "no" end) |"' "$B/_metadata.json"
        echo
        echo "## Feature switches in CI and recipes"
        echo
        echo '```text'
        mapfile -t cfgs < <(git ls-files -- '.github/workflows/*.yml' 'just/*.just')
        grep_lines -- '--(all-features|no-default-features|features)' "${cfgs[@]}" | drop '^[^:]*:[0-9]+:[[:space:]]*#' | sed 's/  */ /g'
        echo '```'
    } > "$B/feature_inventory.md"
    cargo tree --workspace --locked -e features --color never > "$B/feature_tree.txt"
    printf "✓\n"

    # ── Dependencies ──────────────────────────────────────────────────────
    printf "  • crate_graph.md, duplicate_deps.md "
    cargo tree --workspace --locked -d --color never > "$B/duplicate_deps.txt" 2>&1
    members=$(jq -r '.packages | length' "$B/_metadata.json")
    versions=$(occurrences '^name = ' < Cargo.lock)
    names=$(awk -F'"' '/^name = / { print $2 }' Cargo.lock | sort -u | lines)
    awk -F'"' '/^name = / { print $2 }' Cargo.lock | sort | uniq -d > "$work/dups"
    {
        echo "# Dependency baseline"
        echo
        echo "$stamp"
        echo
        echo "| measure | value |"
        echo "|---|---:|"
        echo "| crate versions in Cargo.lock | $versions |"
        echo "| distinct names in Cargo.lock | $names |"
        echo "| workspace members | $members |"
        echo "| external crate versions | $((versions - members)) |"
        echo "| names with two or more versions | $(lines < "$work/dups") |"
        echo
        if [[ -s "$work/dups" ]]; then
            echo "Duplicated names: $(paste -sd, - < "$work/dups" | sed 's/,/, /g')"
        else
            echo "Duplicated names: none."
        fi
        echo
        echo "Paths to each duplicate on this host: \`duplicate_deps.txt\` (\`cargo tree --workspace -d\`; a workspace crate used as both a normal and a build dependency shows there too, at one version)."
    } > "$B/duplicate_deps.md"
    {
        echo "# Crate graph"
        echo
        echo "$stamp"
        echo
        echo "## Edges between workspace crates"
        echo
        echo '```text'
        jq -r '(.packages | map(.name)) as $ws | .packages | sort_by(.name)[] | .name as $n |
            .dependencies[] | select(.name as $d | $ws | any(.[]; . == $d)) |
            "\($n) -> \(.name) (\(.kind // "normal"))"' "$B/_metadata.json"
        echo '```'
        echo
        echo "## Declared external dependencies"
        echo
        echo "| crate | normal | build | dev |"
        echo "|---|---|---|---|"
        jq -r '(.packages | map(.name)) as $ws | .packages | sort_by(.name)[] |
            [.name,
             ([.dependencies[] | select(.kind == null and (.name as $d | $ws | any(.[]; . == $d) | not)) | .name] | join(", ")),
             ([.dependencies[] | select(.kind == "build" and (.name as $d | $ws | any(.[]; . == $d) | not)) | .name] | join(", ")),
             ([.dependencies[] | select(.kind == "dev" and (.name as $d | $ws | any(.[]; . == $d) | not)) | .name] | join(", "))] |
            "| `\(.[0])` | \(if .[1] == "" then "-" else .[1] end) | \(if .[2] == "" then "-" else .[2] end) | \(if .[3] == "" then "-" else .[3] end) |"' "$B/_metadata.json"
        echo
        echo "## Resolved crates each member pulls in (normal and build edges)"
        echo
        echo "| crate | crates in its tree |"
        echo "|---|---:|"
        while IFS=$'\t' read -r name _; do
            n=$(cargo tree --locked -p "$name" -e normal,build --prefix none --color never | sort -u | lines)
            echo "| \`$name\` | $n |"
        done < "$work/crates.tsv"
    } > "$B/crate_graph.md"
    printf "✓\n"

    # ── CI ────────────────────────────────────────────────────────────────
    printf "  • ci_inventory.md "
    {
        echo "# CI inventory"
        echo
        echo "$stamp"
        echo
        echo "| workflow | triggers | jobs |"
        echo "|---|---|---:|"
        for f in .github/workflows/*.yml; do
            trig=$(awk '
                /^on:[[:space:]]*[^[:space:]]/ { sub(/^on:[[:space:]]*/, ""); print; exit }
                /^on:/ { on = 1; next }
                on && /^[^[:space:]#]/ { exit }
                on && /^  [a-z_]+:/ { k = $1; sub(/:$/, "", k); out = out (out ? ", " : "") k }
                END { if (out) print out }' "$f")
            jobs=$(awk '/^jobs:/ { j = 1; next } j && /^[^[:space:]#]/ { j = 0 } j && /^  [A-Za-z0-9_-]+:/ { c++ } END { print c + 0 }' "$f")
            echo "| \`$(basename "$f")\` | $trig | $jobs |"
        done
        echo
        echo "Workflow files: $(git ls-files -- '.github/workflows/*.yml' | lines)"
        echo
        echo "## Gates (scripts/ci/gates.toml) per tier"
        echo
        echo "| tier | gates |"
        echo "|---|---:|"
        for tier in pre-commit pre-push pr-fast; do
            echo "| $tier | $(awk -v t="\"$tier\"" '/^tiers = / && index($0, t) { c++ } END { print c + 0 }' scripts/ci/gates.toml) |"
        done
        echo "| all | $(occurrences '^\[\[gate\]\]' < scripts/ci/gates.toml) |"
    } > "$B/ci_inventory.md"
    printf "✓\n"

    # ── Supply chain ──────────────────────────────────────────────────────
    printf "  • supply_chain.md "
    failed=()
    vet_rc=0
    cargo vet --locked > "$B/vet_baseline.txt" 2>&1 || vet_rc=$?
    deny_rc=0
    cargo deny --color never check --hide-inclusion-graph > "$B/deny_baseline.txt" 2>&1 || deny_rc=$?
    if (( vet_rc != 0 )); then failed+=("cargo vet --locked (exit $vet_rc)"); fi
    if (( deny_rc != 0 )); then failed+=("cargo deny check (exit $deny_rc)"); fi
    {
        echo "# Supply chain"
        echo
        echo "$stamp"
        echo
        echo "| check | exit | summary |"
        echo "|---|---:|---|"
        echo "| \`cargo vet --locked\` | $vet_rc | $(drop '^[[:space:]]*$' < "$B/vet_baseline.txt" | tail -1) |"
        echo "| \`cargo deny check\` | $deny_rc | $(keep '(advisories|bans|licenses|sources) (ok|FAILED)' < "$B/deny_baseline.txt" | sed 's/^[[:space:]]*//' | paste -sd' ' -) |"
        echo
        echo "Exemptions in supply-chain/config.toml: $(occurrences '^\[\[exemptions\.' < supply-chain/config.toml)"
        echo "Audits in supply-chain/audits.toml: $(occurrences '^\[\[audits\.' < supply-chain/audits.toml)"
    } > "$B/supply_chain.md"
    printf "✓\n"

    # ── Index ─────────────────────────────────────────────────────────────
    elapsed=$(( $(date +%s) - started ))
    {
        echo "# Baseline snapshot {{ DATE }}"
        echo
        echo "$stamp It took ${elapsed} s."
        echo
        echo "## Files"
        echo
        for f in "$B"/*; do
            n=$(basename "$f")
            if [[ "$n" != "README.md" ]]; then echo "- \`$n\`"; fi
        done
    } > "$B/_index.md"

    if (( ${#failed[@]} > 0 )); then
        printf "\033[0;31m❌ snapshot written to %s, but these checks failed: %s\033[0m\n" "$B" "${failed[*]}" >&2
        exit 1
    fi
    printf "\033[0;32m✅ snapshot complete in %d s -> %s\033[0m\n" "$elapsed" "$B"
