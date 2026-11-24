// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Build script: stamps the commit, the compiler, the target and the
//! profile for the `md2html-version` macros' `--version` text, through
//! `md2html_version::emit_build_env`, which declares every file and
//! variable it reads.

use std::io::Write as _;

/// Writes the cargo directives; a failed write fails the build.
fn main() -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    md2html_version::emit_build_env(&mut out)?;
    writeln!(out, "cargo:rerun-if-changed=build.rs")
}
