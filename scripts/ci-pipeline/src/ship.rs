// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Resumable ship pipeline driver.
//!
//! Everything in this module wraps [`crate::exec::execute_step_with_tracking`]
//! around a pipeline step so the on-disk
//! [`crate::workflow::WorkflowState`] records each transition - a
//! re-run after a failure picks up at the first non-completed step.
//!
//! Layout:
//! * `tracked_*_step` - one helper per resumable pipeline step (toolchain,
//!   coverage, parallel validation, format check); the clean step and its
//!   policy live in [`clean`].
//! * `run_enhanced_phase1` / `run_enhanced_phase2` - thin orchestrators over
//!   those tracked helpers.
//! * `load_or_reset_ship_state` - resume or start over; [`report`] holds the
//!   banner / summary / next-steps lines.
//! * `run_ship_pipeline` - the entry point `just ship` ultimately calls.

mod clean;
mod report;

use anyhow::{Context as _, Result};
use clean::tracked_clean_step;
pub(crate) use report::print_sccache_stats_if_verbose;
use report::{
    print_ship_next_steps, print_ship_phase_separator, print_ship_resume_banner, print_ship_summary,
};

use crate::changelog::roll_changelog_file;
use crate::child::{self, captured};
use crate::context::PipelineContext;
use crate::exec::{execute_command, execute_parallel, execute_step_with_tracking};
use crate::git_ops::{count_unpushed_commits, git_commit, git_push};
use crate::out::say;
use crate::style::{blue, cyan, green, yellow};
use crate::version::{bump_workspace_version, get_current_version};
use crate::workflow::{
    STEP_COVERAGE_TESTS, STEP_FORMAT_CHECK, STEP_FORMAT_CODE, STEP_GIT_COMMIT, STEP_GIT_PUSH,
    STEP_PARALLEL_VALIDATION, STEP_TOOLCHAIN_SYNC, WorkflowPhase, WorkflowState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Tracked step helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Tracked toolchain step: sync or ensure the nightly, by the run's start.
///
/// `--fresh` runs bump to today's nightly via
/// `just toolchain-sync` (unless `--skip-toolchain-sync`); resumable
/// runs only ensure the currently pinned nightly is installed so
/// repeat ship invocations don't churn `rust-toolchain.toml`.  The
/// choice is [`crate::cli::Start::toolchain_step`].
///
/// # Errors
///
/// The `just` recipe cannot be run, times out or exits with a failure (from
/// [`execute_command`]), recorded as the step's failure first; the
/// workflow state file cannot be written when the step starts, ends or
/// fails; or standard output cannot be written.  A step already completed
/// in `state` is skipped.
fn tracked_toolchain_step(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    let (step_label, step_recipe) = ctx.flags.start.toolchain_step();
    execute_step_with_tracking(state, STEP_TOOLCHAIN_SYNC, || {
        execute_command(step_label, "just", &[step_recipe], ctx)
    })
}

/// Tracked coverage-tests step: `cargo llvm-cov nextest` across the workspace.
///
/// Uses `--lib --bins --tests` rather than `--all-targets`
/// to exclude benchmarks whose init-time `DataFrame` allocations would
/// SIGKILL `nextest` during test enumeration.
///
/// # Errors
///
/// `cargo llvm-cov nextest` cannot be run, times out or exits with a failure
/// (from [`execute_command`], which also
/// covers a failing test), recorded as the step's failure first; the
/// workflow state file cannot be written when the step starts, ends or
/// fails; or standard output cannot be written.  A step already completed
/// in `state` is skipped.
fn tracked_coverage_tests_step(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    execute_step_with_tracking(state, STEP_COVERAGE_TESTS, || {
        execute_command(
            "Coverage tests",
            "cargo",
            &[
                "llvm-cov",
                "nextest",
                "--workspace",
                "--all-features",
                "--lib",
                "--bins",
                "--tests",
                "--profile",
                "ci",
                "--no-report",
            ],
            ctx,
        )
    })
}

/// Tracked parallel-validation step: doctests + production clippy +
/// test clippy + `cargo deny` + rustdoc link validation, all fanned
/// out with `RUSTDOCFLAGS=-Dwarnings` so rustdoc failures are hard.
///
/// # Errors
///
/// One of the five commands cannot be run, times out or fails (from
/// [`execute_parallel`], which names it), recorded as the step's failure
/// first; the workflow state file cannot be written when the step starts,
/// ends or fails; or standard output cannot be written.  A step already
/// completed in `state` is skipped.
fn tracked_parallel_validation_step(
    state: &mut WorkflowState,
    ctx: &PipelineContext,
) -> Result<()> {
    execute_step_with_tracking(state, STEP_PARALLEL_VALIDATION, || {
        let parallel_commands = vec![
            ("Documentation tests", "cargo", vec![
                "test",
                "--doc",
                "--workspace",
                "--all-features",
            ]),
            // pedantic/nursery/cargo/multiple_crate_versions levels are
            // set in workspace Cargo.toml - only per-target overrides
            // needed here.
            ("Production linting", "cargo", vec![
                "clippy",
                "--workspace",
                "--lib",
                "--bins",
                "--all-features",
                "--no-deps",
                "--",
                "-D",
                "warnings",
                "-W",
                "clippy::panic",
                "-W",
                "clippy::todo",
                "-W",
                "clippy::unimplemented",
            ]),
            ("Test linting", "cargo", vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--tests",
                "--no-deps",
                "--",
                "-D",
                "warnings",
                "-W",
                "clippy::panic",
                "-W",
                "clippy::todo",
                "-W",
                "clippy::unimplemented",
                "-A",
                "clippy::unwrap_used",
                "-A",
                "clippy::expect_used",
                "-A",
                "unused-crate-dependencies",
            ]),
            ("Dependency security", "cargo", vec!["deny", "check"]),
            // `--document-private-items` validates links across the private
            // surface too - see the matching note in `phases.rs`.
            ("Rustdoc link validation", "cargo", vec![
                "doc",
                "--workspace",
                "--all-features",
                "--no-deps",
                "--document-private-items",
            ]),
        ];
        execute_parallel(&parallel_commands, &[("RUSTDOCFLAGS", "-Dwarnings")], ctx)
    })
}

/// Tracked format-check step: `cargo fmt --all -- --check` on the
/// *post-format* tree - catches any files rustfmt can't stabilise in
/// one pass (extremely rare, but worth gating before shipping).
///
/// # Errors
///
/// `cargo fmt --all -- --check` cannot be run, times out or exits with a
/// failure (from [`execute_command`], which also
/// covers a file that is not formatted), recorded as the step's failure first;
/// the workflow state file cannot be written when the step starts, ends or
/// fails; or standard output cannot be written.  A step already completed
/// in `state` is skipped.
fn tracked_format_check_step(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    execute_step_with_tracking(state, STEP_FORMAT_CHECK, || {
        execute_command(
            "Format check",
            "cargo",
            &["fmt", "--all", "--", "--check"],
            ctx,
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Resumable Phase 1 / Phase 2
// ─────────────────────────────────────────────────────────────────────────────

/// Phase 1 of the ship pipeline: the full validation sweep.
///
/// The sweep is toolchain +
/// polars + clean + format + coverage tests + parallel validation +
/// format check).  Mutates `state` so every step's started / completed
/// / failed transition is persisted - a re-run after a failure picks
/// up at the first non-completed step.
///
/// Thin orchestrator: each `STEP_*` gets a named `tracked_*_step`
/// helper, so the control flow is readable and a backtrace on failure
/// points at the specific tracked step that failed.
///
/// # Errors
///
/// Propagates any failure from the wrapped tracked steps.
fn run_enhanced_phase1(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    say!(
        "{}",
        blue("🧪 PHASE 1: Optimized Testing & Validation Pipeline").bold()
    )?;
    say!(
        "ℹ️  Running all validation with CURRENT version - version increment happens AFTER validation passes"
    )?;
    say!("🚀 Using safe parallel optimization: format → compile → parallel(doc tests + linting)")?;

    // ── Tree-hash invalidation ──
    // A resumed `just ship` skips a validation step only when the code it
    // validated is byte-identical.  If the working tree changed since Phase 1
    // last passed (you edited a tracked file or committed between runs), the
    // cached "completed" flags for the code-dependent steps are invalidated so
    // they re-run on the new code.  An unchanged tree (e.g. resuming after a
    // Phase-2 push failure) keeps the fast skip.  The authoritative gate is
    // still the uncacheable PR CI; this closes the local "passed step skipped
    // after a code edit" gap.
    let fingerprint = working_tree_fingerprint();
    if state.validated_fingerprint() != fingerprint {
        if !state.validated_fingerprint().is_empty() {
            say!(
                "{}",
                yellow("↻ Working tree changed since last validation - re-running Phase 1 checks")
            )?;
        }
        for step in [
            STEP_FORMAT_CODE,
            STEP_COVERAGE_TESTS,
            STEP_PARALLEL_VALIDATION,
            STEP_FORMAT_CHECK,
        ] {
            state.invalidate_step(step)?;
        }
    }

    tracked_toolchain_step(state, ctx)?;

    say!("{}", yellow("📋 Stage 1: Sequential Prerequisites").bold())?;

    tracked_clean_step(state, ctx)?;
    execute_step_with_tracking(state, STEP_FORMAT_CODE, || {
        execute_command("Format code", "cargo", &["fmt", "--all"], ctx)
    })?;

    // File size policy runs inline (not resumable) because it's
    // ~100 ms and a stale "completed" state could mask a new
    // violation.
    execute_command(
        "File size policy",
        "bash",
        &["scripts/ci/check_file_size_policy.sh"],
        ctx,
    )?;

    tracked_coverage_tests_step(state, ctx)?;
    tracked_parallel_validation_step(state, ctx)?;
    tracked_format_check_step(state, ctx)?;

    // Record the (post-`cargo fmt`) tree this Phase-1 run validated, so a
    // resume on an unchanged tree skips the checks and any later edit re-runs
    // them.  Recomputed here (not reused from the top) because `03-format-code`
    // may have rewritten files.
    state.record_validated(working_tree_fingerprint());
    state.save()?;

    say!(
        "{}",
        green("✅ PHASE 1 COMPLETE - All testing and validation passed!").bold()
    )?;
    Ok(())
}

/// Fingerprint of the working tree that validation actually sees.
///
/// It hashes the `HEAD`
/// commit plus the full diff of tracked changes against it.  A new commit or
/// any edit to a tracked file changes the hash; a brand-new *untracked* file
/// is the one gap, and the uncacheable PR CI still covers it.
///
/// Uses `DefaultHasher` (fixed-key `SipHash` - deterministic across processes,
/// unlike `HashMap`'s randomized state); non-cryptographic but ample for
/// change detection.  On any `git` error the components default to empty, which
/// simply makes the fingerprint conservative (more likely to differ → re-run).
fn working_tree_fingerprint() -> String {
    use core::hash::{Hash as _, Hasher as _};

    fn git(args: &[&str]) -> Vec<u8> {
        match captured(std::process::Command::new("git").args(args), child::LOCAL) {
            Ok(out) => out.stdout,
            // No git: an empty component, see the conservative note above.
            Err(_) => Vec::new(),
        }
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    git(&["rev-parse", "HEAD"]).hash(&mut hasher);
    git(&["diff", "HEAD"]).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Phase 2 of the ship pipeline: version bump, commit, push, release PR.
///
/// Same resumable-state semantics as
/// [`run_enhanced_phase1`].  The push step participates in the Phase 6
/// `invalidate_step` fix so a follow-up commit after a cached
/// "push completed" state re-runs the push rather than silently
/// skipping.
///
/// # Errors
///
/// Propagates any failure from the wrapped tracked steps or from the
/// workflow-state writes surrounding them.
pub(crate) fn run_enhanced_phase2(state: &mut WorkflowState, ctx: &PipelineContext) -> Result<()> {
    say!(
        "{}",
        blue("📦 PHASE 2: version bump → commit → release PR").bold()
    )?;

    // Version increment (Path B, 2026-06-10): restored after R5 retired it.
    // release-plz only versions the 2 publishable leaf libs - it cannot drive
    // binary releases - so the lockstep workspace bump happens HERE, at the end
    // of `just ship`, gated behind the resumable `version_incremented` flag so
    // a re-run after a mid-ship failure never double-bumps.  Default level:
    // patch.
    if !state.version_incremented() {
        bump_workspace_version("patch").context("Failed to bump workspace version")?;
        let new_version = get_current_version().context("Failed to read bumped version")?;
        say!("🔖 Bumped workspace version → v{new_version}")?;
        // Roll CHANGELOG `## [Unreleased]` into the dated `## [vX.Y.Z]` section
        // BEFORE the commit so it lands in the same release commit (`git add .`
        // in `git_commit` stages it).  This is what stops the changelog from
        // drifting - every ship records its release notes (see PR #490 / the
        // `changelog` module).  Idempotent: a resumed ship after this point is
        // gated by `version_incremented`, and the roll itself is a no-op on an
        // already-rolled file.
        roll_changelog_file(&new_version).context("Failed to roll CHANGELOG [Unreleased]")?;
        state.record_bump(new_version);
        state.save()?;
    }

    // Step 10: Git commit (signed commit on the working branch).
    execute_step_with_tracking(state, STEP_GIT_COMMIT, || git_commit(ctx))?;

    // Step 11: Git push -- opens the release/vX.Y.Z PR; the operator
    // lands it with `just merge <PR>` (a merge commit, never squash).
    //
    // Binaries are NOT built here.  Once the PR merges to main,
    // release-auto-trigger.yml dispatches `release.yml`,
    // which produces the reproducible cross-platform binaries on
    // GitHub-hosted runners.
    //
    // Phase 6 resumable-push fix (docs/architecture/dev-flow.md §
    // 5.1 / dev-flow-implementation-plan.md § 6.3): if the developer
    // committed locally since the previous ship run (e.g. to fix a
    // CI-detected audit failure), HEAD will be ahead of the already-
    // pushed release branch on origin.  The cached "completed" state
    // from the prior run would otherwise silently skip this step and
    // the new commits would never land.  Detect that condition and
    // invalidate the cached state so the step re-runs.
    let release_branch_peek = format!("release/v{}", get_current_version()?);
    let unpushed = count_unpushed_commits(&release_branch_peek)?;
    if unpushed > 0 && state.is_step_completed(STEP_GIT_PUSH) {
        say!(
            "↻ {} unpushed commit(s) on HEAD vs origin/{} - re-running step {}",
            yellow(&unpushed.to_string()),
            cyan(&release_branch_peek),
            cyan(STEP_GIT_PUSH),
        )?;
        state.invalidate_step(STEP_GIT_PUSH)?;
    }
    execute_step_with_tracking(state, STEP_GIT_PUSH, || git_push(ctx))?;

    say!(
        "{}",
        green("✅ PHASE 2 COMPLETE - Release PR opened; GitHub Actions will produce binaries.")
            .bold()
    )?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Ship-entry helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Load the on-disk workflow state for a ship run.
///
/// The state is reset to a
/// fresh default when `--fresh` is in effect.  Non-fresh runs that
/// cannot read the state file say so and start a fresh default, so a
/// corrupt state can't strand the pipeline.
///
/// # Errors
///
/// Returns an error if the version cannot be read from `Cargo.toml`, if
/// `--fresh` cannot persist the reset state to disk, or if standard
/// output cannot be written.
fn load_or_reset_ship_state(ctx: &PipelineContext) -> Result<WorkflowState> {
    if ctx.flags.start.is_fresh() {
        say!(
            "{} Fresh run requested - resetting workflow state",
            yellow("🔄")
        )?;
        let new_state = WorkflowState::new_workflow(get_current_version()?);
        new_state
            .save()
            .context("Failed to save fresh workflow state")?;
        Ok(new_state)
    } else {
        let loaded = match WorkflowState::load() {
            Ok(state) => Some(state),
            Err(error) => {
                say!(
                    "{} Workflow state unreadable ({error:#}) - starting a new workflow",
                    yellow("⚠️")
                )?;
                None
            }
        };
        // A COMPLETED cycle must not be "resumed": its steps are all marked
        // done and `version_incremented` is set, so resuming would skip both
        // Phase 1 validation AND the Phase 2 version bump - the next release
        // would commit nothing.  `is_resumable()` already excludes Completed,
        // so a finished state starts a fresh cycle.  (`--fresh` is still the
        // explicit reset; this just stops a stale terminal state from wedging
        // the next `just ship`.)
        match loaded {
            Some(state) if state.phase() != WorkflowPhase::Completed => Ok(state),
            Some(_) | None => Ok(WorkflowState::new_workflow(get_current_version()?)),
        }
    }
}

/// Combined ship pipeline: Phase 1 (validation) + Phase 2 (deploy).
///
/// Supports resumable execution - re-runs skip already-completed
/// steps.  Pass `--fresh` on the CLI to reset state and start over.
///
/// Thin orchestrator: state hydration, progress banner, phase calls,
/// summary, sccache stats, and next-steps hint each live in their own
/// named helper so the ship entry point reads as a single recipe.
///
/// # Errors
///
/// Propagates any failure from the two phase functions or from the
/// workflow-state writes surrounding them.
pub(crate) fn run_ship_pipeline(ctx: &PipelineContext) -> Result<()> {
    say!(
        "{}",
        blue("🚢 MD2HTML Ship Pipeline (Phase 1 + Phase 2, Resumable)").bold()
    )?;
    say!("═══════════════════════════════════════════════════════════════════")?;

    let mut state = load_or_reset_ship_state(ctx)?;
    print_ship_resume_banner(&state, ctx)?;

    print_ship_phase_separator()?;
    run_enhanced_phase1(&mut state, ctx).context("Phase 1 (validation) failed")?;

    print_ship_phase_separator()?;
    run_enhanced_phase2(&mut state, ctx).context("Phase 2 (release PR) failed")?;

    state.advance_phase(WorkflowPhase::Completed)?;

    let total_time = ctx.start_time.elapsed();
    print_ship_summary(&state, total_time)?;
    print_sccache_stats_if_verbose(ctx)?;
    print_ship_next_steps(&state)
}

#[cfg(test)]
mod tests {
    use super::working_tree_fingerprint;

    /// Two fingerprints of an unchanged tree agree and are 16 hex digits.
    ///
    /// # Panics
    ///
    /// When two back-to-back fingerprints differ (every `just ship` would then
    /// re-run Phase 1), or one is not 16 characters, all hex digits.
    #[test]
    fn tree_fingerprint_is_deterministic_and_nonempty() {
        // Two back-to-back calls observe the same working tree, so they MUST
        // agree - if the fingerprint were unstable, every `just ship` would
        // needlessly re-run all of Phase 1.  16 hex chars (u64) is the shape.
        let first = working_tree_fingerprint();
        let second = working_tree_fingerprint();
        assert_eq!(
            first, second,
            "fingerprint must be stable for an unchanged tree"
        );
        assert_eq!(first.len(), 16, "fingerprint is the 16-hex-char u64 hash");
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
