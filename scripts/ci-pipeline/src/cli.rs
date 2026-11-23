// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
//
//! `clap` command-line parser target for the MD2HTML CI pipeline driver.
//!
//! One `Cli` struct for every global flag the pipeline reads and one
//! `Commands` enum for the subcommand dispatch.  Both types are visible
//! to sibling modules via `pub(crate)`; keep everything else `private`
//! so the public API of the binary crate stays narrow.

use clap::builder::RangedU64ValueParser;
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, Parser, Subcommand};

/// Top-level `clap` parser target: the global switches, the two
/// settings that several flags spell together ([`CleanMode`], [`Start`]),
/// and the subcommand.  See [`Commands`] for the per-subcommand shape.
#[derive(Parser)]
#[command(name = "ci-pipeline")]
#[command(about = "MD2HTML High-Performance CI Pipeline with Parallel Orchestration")]
pub(crate) struct Cli {
    /// Selected subcommand (`ship`, `go`, `check-all`, ...).
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Enable verbose output (show all command details)
    #[arg(short, long, global = true)]
    pub(crate) verbose: bool,

    /// Generate coverage report (slower, but comprehensive)
    #[arg(short, long, global = true)]
    pub(crate) coverage_report: bool,

    /// Whether the run cleans the build cache first (`--clean`,
    /// `--no-clean`, or neither).
    #[command(flatten)]
    pub(crate) clean: CleanMode,

    /// Auto-clean if free disk space (GiB) is below this threshold.
    #[arg(long, global = true, default_value_t = 25)]
    pub(crate) min_free_gb: u64,

    /// Auto-clean if the cargo target directory exceeds this size (GiB).
    /// Best-effort; unix only.
    #[arg(long, global = true, default_value_t = 120)]
    pub(crate) max_target_gb: u64,

    /// Override Cargo build parallelism (`CARGO_BUILD_JOBS` / rustc job count).
    /// Also caps the parallel fan-out of validation commands to this value.
    /// If omitted, `CARGO_BUILD_JOBS` defaults to `min(CPU count, 16)` and
    /// fan-out defaults to `max(CPU count / 4, 2)`.  At least 1.
    #[arg(long, global = true, value_parser = RangedU64ValueParser::<usize>::new().range(1..))]
    pub(crate) jobs: Option<usize>,

    /// Disable sccache auto-detection/integration even if it is installed.
    #[arg(long, global = true)]
    pub(crate) no_sccache: bool,

    /// Where a ship run starts (`--fresh`, `--skip-toolchain-sync`).
    #[command(flatten)]
    pub(crate) start: Start,
}

/// Argument id of `--clean`.
const CLEAN: &str = "clean";
/// Argument id of `--no-clean`.
const NO_CLEAN: &str = "no_clean";
/// Argument id of `--fresh`.
const FRESH: &str = "fresh";
/// Argument id of `--skip-toolchain-sync`.
const SKIP_TOOLCHAIN_SYNC: &str = "skip_toolchain_sync";

/// Whether the run cleans the build cache before anything else, from the
/// `--clean` / `--no-clean` flags (which exclude each other).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanMode {
    /// `--clean`: clean unconditionally.
    Force,
    /// Neither flag set: apply the auto-clean heuristics.
    Auto,
    /// `--no-clean`: never auto-clean (a needed toolchain clean is
    /// downgraded to a warning so the next run can re-detect it).
    Never,
}

/// Where a ship run starts, from `--fresh` and `--skip-toolchain-sync`.
///
/// `--skip-toolchain-sync` only means something on a fresh run: a resumed
/// run always keeps the pinned nightly, so the flag alone is accepted and
/// changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    /// Resume at the first step the state file does not record as done,
    /// with the pinned nightly (`just toolchain-ensure`).
    Resume,
    /// `--fresh`: forget the recorded steps and bump the toolchain to
    /// today's nightly (`just toolchain-sync`).
    Fresh,
    /// `--fresh --skip-toolchain-sync`: forget the recorded steps but keep
    /// the pinned nightly, for when the latest one is known to be broken.
    FreshPinned,
}

impl Start {
    /// Whether the run forgets the steps an earlier run completed.
    pub(crate) const fn is_fresh(self) -> bool {
        match self {
            Self::Resume => false,
            Self::Fresh | Self::FreshPinned => true,
        }
    }

    /// The toolchain step this start runs: its label and the `just`
    /// recipe.
    pub(crate) const fn toolchain_step(self) -> (&'static str, &'static str) {
        match self {
            Self::Fresh => (
                "Toolchain sync (fresh bump to latest nightly)",
                "toolchain-sync",
            ),
            Self::Resume | Self::FreshPinned => {
                ("Toolchain ensure (pinned nightly)", "toolchain-ensure")
            }
        }
    }
}

/// A global on/off flag named `long` under the id `id`.
fn switch(id: &'static str, long: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .long(long)
        .global(true)
        .action(ArgAction::SetTrue)
        .help(help)
}

/// Whether the on/off flag `id` was given.
///
/// # Errors
///
/// `id` is not an argument of the command, or not an on/off flag (its value
/// is not a `bool`); clap's message saying which is wrapped in a
/// missing-argument error.
fn given(matches: &ArgMatches, id: &str) -> Result<bool, clap::Error> {
    matches
        .try_get_one::<bool>(id)
        .map(|value| value.is_some_and(|set| *set))
        .map_err(|error| clap::Error::raw(ErrorKind::MissingRequiredArgument, error))
}

impl FromArgMatches for CleanMode {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if given(matches, CLEAN)? {
            Ok(Self::Force)
        } else if given(matches, NO_CLEAN)? {
            Ok(Self::Never)
        } else {
            Ok(Self::Auto)
        }
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Args for CleanMode {
    fn augment_args(cmd: Command) -> Command {
        cmd.arg(switch(
            CLEAN,
            "clean",
            "Force a full `cargo clean` at the start (slower, but can recover from stale \
             artifacts)",
        ))
        .arg(
            switch(
                NO_CLEAN,
                "no-clean",
                "Force skipping cargo clean even when auto-clean would run (dangerous if disk \
                 is tight)",
            )
            .conflicts_with(CLEAN),
        )
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for Start {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        if !given(matches, FRESH)? {
            Ok(Self::Resume)
        } else if given(matches, SKIP_TOOLCHAIN_SYNC)? {
            Ok(Self::FreshPinned)
        } else {
            Ok(Self::Fresh)
        }
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Args for Start {
    fn augment_args(cmd: Command) -> Command {
        cmd.arg(switch(
            FRESH,
            "fresh",
            "Force a fresh run, ignoring any previously completed steps. Use this to start \
             the pipeline from scratch.",
        ))
        .arg(switch(
            SKIP_TOOLCHAIN_SYNC,
            "skip-toolchain-sync",
            "Skip the nightly toolchain bump even on `--fresh` runs: keep the pinned nightly \
             (`just toolchain-ensure`) when the latest one is known to be broken. Runs \
             without `--fresh` never bump",
        ))
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

/// CLI subcommands.  Each variant maps 1:1 to a sub-entry-point in
/// `main`; see that dispatch for the runtime semantics.
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Safe-by-default validation workflow (no version bump, deploy, commit, or
    /// push)
    Go,
    /// Full ship pipeline: Phase 1 validation + Phase 2 deploy (resumable)
    /// Re-runs skip already-completed steps. Use --fresh to start from scratch.
    Ship,
    /// Comprehensive nightly-grade validation with parallel execution
    CheckAll,
    /// Phase 1 nightly validation gates with maximum parallelism
    Phase1,
    /// Explicit ship lane: version bump, build, deploy, commit, and push
    Phase2,
    /// Generate coverage report from existing data (or run tests if needed)
    CoverageReport,
    /// Multi-tool security audit with parallelism
    AuditComprehensive,
    /// Check current workflow status
    WorkflowStatus,
    /// Reset workflow state (force clean slate)
    WorkflowReset,
    /// Resume incomplete workflow
    WorkflowResume,
    /// Nightly cross-compilation validation
    CrossCheck,
    /// Write an auto-generated draft into CHANGELOG `[Unreleased]` from the
    /// Conventional-Commit subjects since the last release tag. A placeholder
    /// to polish before shipping; never overwrites hand-authored notes.
    ChangelogDraft,
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::{CleanMode, Cli, Start};

    /// The clean mode and start the command line `words` asks for.
    ///
    /// # Panics
    ///
    /// When clap rejects `words` as a command line.
    fn settings(words: &[&str]) -> (CleanMode, Start) {
        let cli = Cli::try_parse_from(words).expect("a valid command line");
        (cli.clean, cli.start)
    }

    /// With no flags, `ship` resumes and cleans when the auto rule says so.
    ///
    /// # Panics
    ///
    /// When the line is rejected, or the mode is not `Auto` or the start not
    /// `Resume`.
    #[test]
    fn no_flags_resume_with_the_auto_clean() {
        assert_eq!(
            settings(&["ci-pipeline", "ship"]),
            (CleanMode::Auto, Start::Resume)
        );
    }

    /// `--clean` and `--no-clean` set their modes on either side of the
    /// subcommand.
    ///
    /// # Panics
    ///
    /// When a line is rejected, `--clean` after `ship` is not `Force`, or
    /// `--no-clean` before `go` is not `Never`.
    #[test]
    fn clean_flags_map_to_their_modes_before_or_after_the_subcommand() {
        assert_eq!(
            settings(&["ci-pipeline", "ship", "--clean"]).0,
            CleanMode::Force
        );
        assert_eq!(
            settings(&["ci-pipeline", "--no-clean", "go"]).0,
            CleanMode::Never
        );
    }

    /// `--clean` and `--no-clean` together are refused.
    ///
    /// # Panics
    ///
    /// When clap accepts both on one line.
    #[test]
    fn clean_and_no_clean_exclude_each_other() {
        assert!(
            Cli::try_parse_from(["ci-pipeline", "ship", "--clean", "--no-clean"]).is_err(),
            "--clean with --no-clean is a contradiction"
        );
    }

    /// `--fresh` and `--skip-toolchain-sync` choose where the run starts.
    ///
    /// # Panics
    ///
    /// When a line is rejected, `--fresh` is not `Fresh`, both flags are not
    /// `FreshPinned`, or `--skip-toolchain-sync` alone is not `Resume`.
    #[test]
    fn fresh_and_skip_toolchain_sync_choose_the_start() {
        assert_eq!(
            settings(&["ci-pipeline", "ship", "--fresh"]).1,
            Start::Fresh
        );
        assert_eq!(
            settings(&["ci-pipeline", "ship", "--fresh", "--skip-toolchain-sync"]).1,
            Start::FreshPinned
        );
        assert_eq!(
            settings(&["ci-pipeline", "ship", "--skip-toolchain-sync"]).1,
            Start::Resume
        );
    }

    /// `--jobs` takes a count of one or more.
    ///
    /// # Panics
    ///
    /// When `--jobs 0` is accepted, or `--jobs 3` is rejected or read as
    /// another count.
    #[test]
    fn jobs_must_be_at_least_one() {
        assert!(Cli::try_parse_from(["ci-pipeline", "go", "--jobs", "0"]).is_err());
        let cli = Cli::try_parse_from(["ci-pipeline", "go", "--jobs", "3"]).expect("valid");
        assert_eq!(cli.jobs, Some(3));
    }

    #[test]
    fn the_command_definition_is_consistent() {
        <Cli as clap::CommandFactory>::command().debug_assert();
    }
}
