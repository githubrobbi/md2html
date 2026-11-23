// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the ship pipeline tells the person running it around the two
//! phases: the resume banner, the separator, the summary and the next
//! steps.
//!
//! # Errors
//!
//! Every function here fails only when standard output cannot be
//! written.

use core::time::Duration;
use std::process::Command;

use anyhow::Result;

use crate::child::stdout_text;
use crate::context::PipelineContext;
use crate::out::say;
use crate::style::Paint as _;
use crate::workflow::{ALL_STEPS, WorkflowState};

/// Print the resume banner: how many steps are already completed,
/// which ones, and which are still pending.  No-op on `--fresh` runs
/// and on the very first ship attempt (nothing to resume).
pub(super) fn print_ship_resume_banner(state: &WorkflowState, ctx: &PipelineContext) -> Result<()> {
    let completed_count = state.step_tracker.completed_steps.len();
    if completed_count == 0 || ctx.flags.start.is_fresh() {
        return Ok(());
    }
    let total_steps = ALL_STEPS.len();
    let pending_steps = state.get_pending_steps(ALL_STEPS);

    say!(
        "{} Resuming from previous run: {}/{} steps completed",
        "📋".cyan(),
        completed_count,
        total_steps
    )?;
    if !state.step_tracker.completed_steps.is_empty() {
        say!("   Already completed:")?;
        for step in &state.step_tracker.completed_steps {
            say!("     {} {}", "✓".green(), step)?;
        }
    }
    if !pending_steps.is_empty() {
        say!("   Remaining:")?;
        for step in &pending_steps {
            say!("     {} {}", "○".dimmed(), step)?;
        }
    }
    say!()?;
    Ok(())
}

/// Print the post-run summary: version, total wall time, completed
/// step count, and per-step timing breakdown.
pub(super) fn print_ship_summary(state: &WorkflowState, total_time: Duration) -> Result<()> {
    let total_steps = ALL_STEPS.len();
    say!()?;
    say!(
        "{}",
        "═══════════════════════════════════════════════════════════════════".green()
    )?;
    say!("{} Ship Pipeline Complete!", "🎉".green().bold())?;
    say!("═══════════════════════════════════════════════════════════════════")?;
    say!("   Version:    {}", state.current_version.cyan())?;
    say!("   Total time: {}s", total_time.as_secs())?;
    say!(
        "   Steps:      {}/{} completed",
        state.step_tracker.completed_steps.len(),
        total_steps
    )?;

    if !state.step_durations_secs.is_empty() {
        say!()?;
        say!("   Step Timings:")?;
        for (step, secs) in &state.step_durations_secs {
            let mins = secs / 60;
            let remaining_secs = secs % 60;
            if mins > 0 {
                say!("     {step} {mins}m {remaining_secs}s")?;
            } else {
                say!("     {step} {secs}s")?;
            }
        }
    }
    Ok(())
}

/// Print sccache `-s` stats when both sccache was enabled and the user
/// asked for verbose output (or why there are none).  Prints nothing in
/// any other case.
pub(crate) fn print_sccache_stats_if_verbose(ctx: &PipelineContext) -> Result<()> {
    if !(ctx.flags.sccache_enabled && ctx.flags.verbose) {
        return Ok(());
    }
    match Command::new("sccache").arg("-s").output() {
        Ok(out) => say!(
            "{} sccache stats:\n{}",
            "⚡".green(),
            stdout_text("sccache -s", out.stdout)?
        ),
        Err(error) => say!("{} sccache stats unavailable: {error}", "⚡".yellow()),
    }
}

/// Print the terminal "what GitHub does next" hint block that points
/// the developer at the release-workflow run list.
pub(super) fn print_ship_next_steps(state: &WorkflowState) -> Result<()> {
    say!()?;
    say!(
        "{} Release PR opened; land it with `just merge <PR>`",
        "📤".green()
    )?;
    say!(
        "{} GitHub Actions will build + publish v{} on merge",
        "👷".green(),
        state.current_version
    )?;
    say!(
        "   💡 Watch: {}",
        "gh run list --repo <owner>/<repo> --workflow=release.yml --limit 5".cyan()
    )?;
    Ok(())
}

/// Print a horizontal separator used between the two phases of the
/// ship pipeline, for visual scannability in the terminal output.
pub(super) fn print_ship_phase_separator() -> Result<()> {
    say!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed()
    )?;
    Ok(())
}
