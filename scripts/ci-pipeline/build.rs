// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Build script: stamps git sha + build metadata for the `md2html-version`
//! macros (`MD2HTML_GIT_SHA`, `MD2HTML_RUSTC`, `MD2HTML_TARGET`,
//! `MD2HTML_PROFILE`).

use std::io::Write as _;

/// Writes the cargo directives; a failed write fails the build.
fn main() -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    md2html_version::emit_build_env(&mut out)?;
    writeln!(out, "cargo:rerun-if-changed=build.rs")
}
