// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Cross-compilation syntax validation for the `cross-check`
//! subcommand (`just check-cross`): a Linux and a Windows check pass,
//! both required.

use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::child::{self, captured};
use crate::context::PipelineContext;
use crate::exec::execute_command;
use crate::out::say;

/// `cross-check` subcommand: Linux and Windows cross-compilation syntax
/// checks.  Both legs run, or the command fails.
///
/// The Linux leg is a `cargo check` for `x86_64-unknown-linux-gnu`, whose
/// standard library `rust-toolchain.toml` installs; checking links
/// nothing, so no cross C toolchain is involved.  The Windows leg is
/// `cargo xwin check` (cargo-xwin brings the MSVC headers and libraries).
///
/// # Errors
///
/// cargo-xwin is not installed (the error names the install command), or
/// either check fails.
pub(crate) fn handle_cross_check(ctx: &PipelineContext) -> Result<()> {
    say!("🔍 Cross-compilation syntax validation...")?;
    say!("⚠️  Note: This checks syntax only (no linking) to catch API compatibility issues")?;

    execute_command(
        "Cross-compile syntax check (Linux x86_64)",
        "cargo",
        &[
            "check",
            "--workspace",
            "--all-features",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--lib",
        ],
        ctx,
    )
    .context("Cross-compilation syntax check failed")?;
    say!("✅ Cross-compilation syntax check passed")?;

    // Windows x86_64 via cargo-xwin (bundles MSVC headers/libs).
    let has_cargo_xwin = captured(
        Command::new("cargo").args(["xwin", "--version"]),
        child::LOCAL,
    )
    .is_ok_and(|output| output.status.success());
    if !has_cargo_xwin {
        bail!(
            "cargo-xwin not found: the Windows cross-check needs it - install it with \
             `just install-dev-tools` (or `cargo install --locked cargo-xwin`)"
        );
    }
    execute_command(
        "Cross-compile syntax check (Windows x86_64)",
        "cargo",
        &[
            "xwin",
            "check",
            "--workspace",
            "--target",
            "x86_64-pc-windows-msvc",
        ],
        ctx,
    )
    .context("Windows cross-compilation syntax check failed")?;
    say!("✅ Windows cross-compilation syntax check passed")?;
    Ok(())
}
