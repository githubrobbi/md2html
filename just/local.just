# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# md2html from this checkout, on this machine: installed, and built for every
# platform. Both recipes run a workspace binary (`scripts/dev-tools`), so they
# do the same under every shell `just` uses.

# md2html's own zig, when it is installed (`just install-dev-tools`): 0.17.1,
# the version `[toolchain] ziglang` names in scripts/ci/gates.toml.  Every
# just-driven cargo-zigbuild in this repository uses it, whatever zig is on
# the PATH (other rust-forge repositories keep an older one there).  Without
# it, the zig on the PATH serves.  OS-guarded as in just/shared.just: on
# Windows the backtick would run under PowerShell.
md2html_zig := if os_family() == "windows" { "zig" } else { `if test -x "$HOME/.local/zig/0.17.1/zig"; then echo "$HOME/.local/zig/0.17.1/zig"; else echo zig; fi` }
export CARGO_ZIGBUILD_ZIG_PATH := md2html_zig

# Build md2html (release) from this checkout and install it to ~/bin
# (`MD2HTML_BIN_DIR` names another folder).

# Build md2html (release) from this checkout and install it to ~/bin.
use-local:
    cargo run -q -p md2html-dev-tools --bin install-bins

# The release archives for every platform this machine can build, in dist/:
# macOS, Linux (static) and Windows, on ARM64 and x86-64. Nothing is uploaded.
# `just release-local linux-x64 windows-x64` builds only those;
# `just release-local --list` shows each platform and how it is built here.

# Release archives for every platform this machine can build, in dist/.
release-local *ARGS:
    cargo run -q -p md2html-dev-tools --bin release-local -- {{ ARGS }}
