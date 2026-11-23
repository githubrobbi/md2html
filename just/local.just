# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# md2html from this checkout, on this machine: installed, and built for every
# platform. Both recipes run a workspace binary (`scripts/dev-tools`), so they
# do the same under every shell `just` uses.

# Build md2html (release) from this checkout and install it to ~/bin
# (`MD2HTML_BIN_DIR` names another folder).
use-local:
    cargo run -q -p md2html-dev-tools --bin install-bins

# The release archives for every platform this machine can build, in dist/:
# macOS, Linux (static) and Windows, on ARM64 and x86-64. Nothing is uploaded.
# `just release-local linux-x64 windows-x64` builds only those;
# `just release-local --list` shows each platform and how it is built here.
release-local *ARGS:
    cargo run -q -p md2html-dev-tools --bin release-local -- {{ ARGS }}
