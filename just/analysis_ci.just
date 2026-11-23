# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# Check for unused dependencies with cargo-machete (the `machete` gate's command).
machete:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔪 Checking for unused dependencies...\033[0m\n"
    if ! command -v cargo-machete >/dev/null 2>&1; then
        printf "\033[0;31m❌ cargo-machete not installed - install with: \033[0;36mjust install-dev-tools\033[0m\n"
        exit 1
    fi
    cargo-machete --skip-target-dir
    printf "\033[0;32m✅ Dependency check complete!\033[0m\n"
