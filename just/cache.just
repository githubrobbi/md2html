# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Install sccache.  just/shared.just detects a working sccache and routes
# every just-driven cargo through it (RUSTC_WRAPPER), so there is nothing
# to export by hand.

# Install sccache (just-driven cargo uses it automatically once it works).
setup-sccache:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🚀 Setting up sccache for faster compilation...\033[0m\n"
    just _install-if-missing sccache sccache
    if ! sccache rustc -vV >/dev/null; then
        printf "\033[0;31m❌ sccache is installed but cannot run rustc (a sandbox or a broken install); builds stay uncached\033[0m\n"
        exit 1
    fi
    printf "\033[0;34m  → Current cache statistics:\033[0m\n"
    sccache --show-stats
    printf "\033[0;32m✅ sccache works - every just-driven cargo uses it (see just/shared.just)\033[0m\n"

# Show sccache statistics and the cache size.
sccache-stats:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m📊 sccache compilation cache statistics...\033[0m\n"
    if ! command -v sccache >/dev/null 2>&1; then
        printf "\033[0;31m❌ sccache not installed - run: \033[0;36mjust setup-sccache\033[0m\n"
        exit 1
    fi
    # The statistics name the cache location and its size.
    sccache --show-stats

# Clear the sccache cache (stops the server, empties its directory).
sccache-clear:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🧹 Clearing sccache compilation cache...\033[0m\n"
    if ! command -v sccache >/dev/null 2>&1; then
        printf "\033[0;32m✅ sccache not installed - there is no cache to clear\033[0m\n"
        exit 0
    fi
    cache_dir="${SCCACHE_DIR:-}"
    if [[ -z "$cache_dir" ]]; then
        case "$(uname -s)" in
            Darwin) cache_dir="$HOME/Library/Caches/Mozilla.sccache" ;;
            *) cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/sccache" ;;
        esac
    fi
    # --zero-stats starts the server when none runs, so --stop-server
    # always has one to stop.
    sccache --zero-stats
    sccache --stop-server
    if [[ -d "$cache_dir" ]]; then
        du -sh "$cache_dir"
        rm -rf "$cache_dir"
    fi
    printf "\033[0;32m✅ sccache cache cleared (%s)\033[0m\n" "$cache_dir"

# Prune the `cargo-llvm-cov` target tree.
#
# `just test` runs `cargo llvm-cov nextest`, which writes an instrumented
# build tree to `$CARGO_TARGET_DIR/llvm-cov-target/`.  That tree grows
# over a long session (every profile bump, every `.profraw` from a failed
# run, every compiled test binary stays there).
#
# What it prunes:
#   - `$CARGO_TARGET_DIR/llvm-cov-target/` (instrumented build tree)
#   - `$CARGO_TARGET_DIR/llvm-cov/`        (HTML coverage report)
#   - `$CARGO_TARGET_DIR/**/*.profraw`     (instrumentation output)
#
# Safe to re-run; leaves regular `cargo build` artifacts and sccache
# untouched.  Reports total freed space.

# Prune the llvm-cov target tree and stray .profraw files.
clean-cov:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🧹 Pruning the cargo-llvm-cov target tree...\033[0m\n"
    TARGET_DIR="${CARGO_TARGET_DIR:-target}"
    printf "   target dir: %s\n" "$TARGET_DIR"
    FREED_KB=0
    for sub in llvm-cov-target llvm-cov; do
        if [ -d "$TARGET_DIR/$sub" ]; then
            SIZE_KB=$(du -sk "$TARGET_DIR/$sub" | cut -f1)
            printf "   \033[0;34m•\033[0m %s/%s - %s KB\n" "$TARGET_DIR" "$sub" "$SIZE_KB"
            rm -rf "${TARGET_DIR:?}/$sub"
            FREED_KB=$((FREED_KB + SIZE_KB))
        fi
    done
    if [ -d "$TARGET_DIR" ]; then
        PROFRAW_COUNT=0
        PROFRAW_KB=0
        while IFS= read -r -d '' f; do
            kb=$(du -sk "$f" | cut -f1)
            PROFRAW_KB=$((PROFRAW_KB + kb))
            PROFRAW_COUNT=$((PROFRAW_COUNT + 1))
            rm -f "$f"
        done < <(find "$TARGET_DIR" -name '*.profraw' -type f -print0)
        if [ "$PROFRAW_COUNT" -gt 0 ]; then
            printf "   \033[0;34m•\033[0m %s leftover *.profraw file(s) - %s KB\n" "$PROFRAW_COUNT" "$PROFRAW_KB"
            FREED_KB=$((FREED_KB + PROFRAW_KB))
        fi
    fi
    if [ "$FREED_KB" -ge 1048576 ]; then
        printf "\033[0;32m✅ Freed ~%s GB\033[0m\n" "$((FREED_KB / 1048576))"
    elif [ "$FREED_KB" -ge 1024 ]; then
        printf "\033[0;32m✅ Freed ~%s MB\033[0m\n" "$((FREED_KB / 1024))"
    elif [ "$FREED_KB" -gt 0 ]; then
        printf "\033[0;32m✅ Freed %s KB\033[0m\n" "$FREED_KB"
    else
        printf "\033[0;32m✅ Nothing to clean - no llvm-cov tree, no .profraw.\033[0m\n"
    fi
