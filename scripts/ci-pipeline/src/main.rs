// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
//
// MD2HTML - md2html: High-Performance File Search Tool
// Contact: 50460704+githubrobbi@users.noreply.github.com for licensing
// inquiries.

//! MD2HTML CI pipeline driver - orchestration of the
//! pre-push, pre-ship, and ship workflows.
//!
//! Module layout (each module is self-contained with its own docs):
//!
//! * [`cli`]         - `clap` parser (`Cli`, `Commands`).
//! * [`context`]     - `PipelineContext`, `PipelineFlags`, and the small
//!   filesystem helpers used by context construction.
//! * [`workflow`]    - resumable-workflow state machine (`WorkflowState`,
//!   `StepTracker`, `WorkflowPhase`, `STEP_*` ids).
//! * [`exec`]        - subprocess execution primitives (`execute_command*`,
//!   `execute_parallel`, `execute_step_with_tracking`).
//! * [`version`]     - version discovery + bump helpers.
//! * [`git_ops`]     - `git` + `gh` CLI orchestration for Phase 2 (commit,
//!   push, open the release PR).
//! * [`phases`]      - non-resumable `phase1_optimized` / `phase2_optimized`
//!   driver functions.
//! * [`ship`]        - resumable ship pipeline (`run_enhanced_phase1`,
//!   `run_enhanced_phase2`, `run_ship_pipeline`).
//! * [`cross_check`] - Linux + Windows cross-compilation syntax validation.
//! * [`out`]         - where the text for people goes (`say!`).
//! * [`style`]       - colour for that text, only on a terminal.
//! * [`child`]       - running a child process with a time limit.
//!
//! `main` itself is just the CLI dispatch: parse → build context →
//! print banner → match subcommand → delegate.  Every subcommand
//! handler that survives in this file (`handle_go`,
//! `handle_check_all`, etc.) is a small coordinator - the real work
//! lives in the modules above.

// `BTreeMap`/`BTreeSet` live in `alloc`; the workspace
// `clippy::std_instead_of_alloc` lint correctly prefers the canonical
// path over `std::collections::*` re-exports.  Bin crates need the
// explicit `extern crate alloc;` to make the `alloc::` namespace
// visible (libraries get it for free via the 2024 edition's prelude).
extern crate alloc;

mod changelog;
mod child;
mod cli;
mod clock;
mod context;
mod cross_check;
mod exec;
mod git_ops;
mod out;
mod phases;
mod ship;
mod style;
mod version;
mod workflow;

use std::process::{Command, Stdio};

use anyhow::{Context as _, Result};
use clap::Parser as _;

use crate::child::finished;
use crate::cli::{Cli, Commands};
use crate::context::PipelineContext;
use crate::cross_check::handle_cross_check;
use crate::exec::execute_parallel;
use crate::out::say;
use crate::phases::{coverage_report_command, phase1_optimized, phase2_optimized};
use crate::ship::{print_sccache_stats_if_verbose, run_ship_pipeline};
use crate::style::{blue, green, yellow};
use crate::workflow::{WorkflowState, print_workflow_status};

// ─────────────────────────────────────────────────────────────────────────────
// Startup helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Print the startup banner (verbose mode + coverage status + sccache
/// status + log file location), then eagerly start the sccache server
/// if enabled so the first real compile inherits a warm connection.
///
/// # Errors
///
/// Standard output cannot be written.  A `sccache --start-server` that
/// cannot be spawned is said on a line, not returned.
fn print_startup_banner_and_warm_sccache(ctx: &PipelineContext) -> Result<()> {
    if ctx.flags.verbose {
        say!("{} Verbose mode enabled", blue("🔍"))?;
        say!(
            "{} Coverage report: {}",
            blue("📊"),
            if ctx.flags.coverage_report {
                "enabled"
            } else {
                "disabled"
            }
        )?;
    }

    if ctx.flags.sccache_enabled {
        say!("{} sccache: enabled (RUSTC_WRAPPER=sccache)", green("⚡"))?;
    } else if ctx.flags.verbose {
        say!(
            "{} sccache: disabled (install sccache for big CI wins)",
            yellow("⚡")
        )?;
    }

    if let Some(log_path) = &ctx.log_file {
        say!("{} Log file: {}", blue("📝"), log_path.display())?;
    }

    // The exit status is not read: sccache answered `rustc -vV` moments
    // ago (`sccache_is_functional`), so a server runs or starts on the
    // first compile either way; only a failed spawn is worth a line.
    if ctx.flags.sccache_enabled
        && let Err(error) = finished(
            Command::new("sccache")
                .arg("--start-server")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
            child::LOCAL,
        )
    {
        say!("{} sccache server not started: {error}", yellow("⚡"))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Subcommand handlers that stay in `main.rs` (small enough that a
// separate module would just add import noise).
// ─────────────────────────────────────────────────────────────────────────────

/// `go` subcommand: Phase 1 validation + completion footer (total
/// time, coverage tip, next-step hint for `just phase2-ship`).
///
/// # Errors
///
/// A Phase 1 stage fails (`Validation workflow failed`, over the step's
/// own error), the sccache statistics cannot be read in a verbose run, or
/// standard output cannot be written.
fn handle_go(ctx: &PipelineContext) -> Result<()> {
    say!(
        "{}",
        blue("🚀 Safe-by-Default Validation Workflow (OPTIMIZED)").bold()
    )?;
    phase1_optimized(ctx).context("Validation workflow failed")?;

    let total_time = ctx.start_time.elapsed();
    say!(
        "{} Total pipeline time: {}s",
        green("🎉"),
        total_time.as_secs()
    )?;

    print_sccache_stats_if_verbose(ctx)?;

    if !ctx.flags.coverage_report {
        say!(
            "{} Tip: Use --coverage-report to generate HTML coverage report",
            blue("💡")
        )?;
    }
    say!("{} Run 'just phase2-ship' when ready to ship", blue("💡"))?;
    Ok(())
}

/// `check-all` subcommand: comprehensive validation using the same
/// `phase1_optimized` stack as `go`, with a distinct banner.
///
/// # Errors
///
/// A Phase 1 stage fails (`Comprehensive validation failed`, over the
/// step's own error), or standard output cannot be written.
fn handle_check_all(ctx: &PipelineContext) -> Result<()> {
    say!("{}", blue("📋 Comprehensive Validation (PARALLEL)").bold())?;
    phase1_optimized(ctx).context("Comprehensive validation failed")?;
    say!("{} Comprehensive validation complete!", green("✅"))?;
    Ok(())
}

/// `audit-comprehensive` subcommand: parallel multi-tool security
/// audit (cargo audit + cargo deny + cargo audit --deny warnings).
///
/// # Errors
///
/// One of the three commands cannot be run, times out or fails
/// (`Security audit failed`, over the failing command's error), or
/// standard output cannot be written.
fn handle_audit_comprehensive(ctx: &PipelineContext) -> Result<()> {
    say!("{}", blue("🔒 Multi-Tool Security Audit (PARALLEL)").bold())?;
    let audit_commands = vec![
        ("Cargo audit", "cargo", vec!["audit"]),
        ("Cargo deny", "cargo", vec!["deny", "check"]),
        ("Security advisory", "cargo", vec![
            "audit", "--deny", "warnings",
        ]),
    ];
    execute_parallel(&audit_commands, &[], ctx).context("Security audit failed")
}

/// `workflow-reset` subcommand: atomically replace the workflow state
/// file with a fresh default.
///
/// # Errors
///
/// The fresh state cannot be written (`Failed to save reset workflow
/// state`, over the serialising, folder, write or rename error), or
/// standard output cannot be written.
fn handle_workflow_reset() -> Result<()> {
    let state = WorkflowState::default();
    state
        .save()
        .context("Failed to save reset workflow state")?;
    say!("🧹 Workflow state reset to clean slate")?;
    Ok(())
}

/// `workflow-resume` subcommand: print a resumable-phase hint if a
/// partial ship is detected, otherwise fall through to the workflow
/// status display.
///
/// # Errors
///
/// The state file exists but cannot be read or is not valid JSON (`Failed
/// to load workflow state`), or standard output cannot be written.
fn handle_workflow_resume() -> Result<()> {
    let state = WorkflowState::load().context("Failed to load workflow state")?;
    if state.is_resumable() {
        say!("🔄 Resuming workflow from phase: {}", state.phase())?;
        say!("💡 Run 'just phase2-ship' to continue the explicit ship lane")?;
    } else {
        say!("❌ No resumable workflow found")?;
        print_workflow_status(&state)?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

/// CLI entry point.
///
/// Parses args, builds the [`PipelineContext`],
/// prints the startup banner, and dispatches to the `handle_*`
/// sub-function matching the chosen subcommand.  Each handler owns its
/// own footer printing so this dispatch stays a one-arm-per-command
/// match.
///
/// # Errors
///
/// The log folder cannot be created, standard output cannot be written,
/// or the chosen subcommand fails; the error names the subcommand
/// (`Ship pipeline failed`, `PHASE 1 standalone execution failed`, ...)
/// over the cause.  A command line clap rejects ends the process with
/// clap's usage message instead.
fn main() -> Result<()> {
    if let Some(text) = md2html_version::requested_version!("md2html-ci-pipeline") {
        return say!("{text}");
    }
    let cli = Cli::parse();
    let validation_command = matches!(
        cli.command,
        Commands::Go | Commands::CheckAll | Commands::Phase1
    );
    let ctx = PipelineContext::new(&cli, validation_command)?;

    print_startup_banner_and_warm_sccache(&ctx)?;

    match cli.command {
        Commands::Go => handle_go(&ctx)?,
        Commands::Ship => run_ship_pipeline(&ctx).context("Ship pipeline failed")?,
        Commands::CheckAll => handle_check_all(&ctx)?,
        Commands::Phase1 => {
            phase1_optimized(&ctx).context("PHASE 1 standalone execution failed")?;
        }
        Commands::Phase2 => {
            phase2_optimized(&ctx).context("PHASE 2 standalone execution failed")?;
        }
        Commands::CoverageReport => {
            coverage_report_command(&ctx).context("Coverage report generation failed")?;
        }
        Commands::AuditComprehensive => handle_audit_comprehensive(&ctx)?,
        Commands::WorkflowStatus => {
            let state = WorkflowState::load().context("Failed to load workflow state")?;
            print_workflow_status(&state)?;
        }
        Commands::WorkflowReset => handle_workflow_reset()?,
        Commands::WorkflowResume => handle_workflow_resume()?,
        Commands::CrossCheck => handle_cross_check(&ctx)?,
        Commands::ChangelogDraft => changelog::write_draft_into_unreleased()
            .context("Failed to draft CHANGELOG [Unreleased] from commits")?,
    }

    Ok(())
}
