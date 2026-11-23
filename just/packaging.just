# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025-2026 Sky, LLC.

# MD2HTML packaging recipes: platform-native bundles + installers.
#
# These recipes wrap the thin shell scripts under `packaging/` with the
# build-release step and the status output the rest of the justfile uses.
# The heavy lifting (plist templating, freedesktop install layout) lives
# in packaging/*/ so the scripts also work outside `just`.

# Build a macOS .app bundle wrapping the release md2html binary.
#
# Output: dist/MD2HTML.app (dist/ is gitignored - ephemeral build output).
dist-macos:
    @printf "\033[0;34m📦 Building macOS .app bundle...\033[0m\n"
    cargo build --release -p md2html-cli
    ./packaging/macos/bundle.sh target/release/md2html dist/MD2HTML.app
    @printf "\033[0;32m✅ Bundle ready: dist/MD2HTML.app\033[0m\n"

# Install MD2HTML on Linux with desktop integration (requires sudo).
#
# Installs to /usr/local by default; override with `PREFIX=... just install-linux`.
install-linux:
    @printf "\033[0;34m📦 Installing MD2HTML + desktop integration (sudo required)...\033[0m\n"
    cargo build --release -p md2html-cli
    sudo ./packaging/linux/install.sh target/release/md2html
    @printf "\033[0;32m✅ Install complete\033[0m\n"
