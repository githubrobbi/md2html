// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Build script: stamps git sha + build metadata for the `md2html-version`
//! macros (`MD2HTML_GIT_SHA`, `MD2HTML_RUSTC`, `MD2HTML_TARGET`,
//! `MD2HTML_PROFILE`).
//!
//! Projects that want Windows resource embedding (icon, version info,
//! app.manifest) add `winresource` here - see the template documentation.

fn main() {
    md2html_version::emit_build_env();
    println!("cargo:rerun-if-changed=build.rs");
}
