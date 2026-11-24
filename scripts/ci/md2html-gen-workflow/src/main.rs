// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html-gen-workflow` - gate-manifest workflow structural validator.
//!
//! One of the repository's generators and checkers
//! (`docs/policies/build_codegen_policy.md`, "Files the repository
//! generates"); the gates it holds the workflow to are
//! `scripts/ci/gates.toml`.
//!
//! # Design
//!
//! It checks; it does not write.  Generating the per-gate job blocks of
//! `.github/workflows/pr-fast.yml` was the obvious alternative, but every
//! per-gate job in the workflow is bespoke (runner OS, timeout, `needs:`
//! chain, rust-cache key strategy, free-disk preamble, conditional
//! cargo-vet install + run, multi-step commands).  Encoding all of that
//! in TOML so a generator could emit it back is a YAML-in-TOML
//! translation problem with no real upside, AND it would stake branch
//! protection on a hand-rolled YAML emitter.  A `--check`-only validator
//! keeps every drift guarantee at the risk of `gates-drift`: the tool
//! only reads files; it cannot break the workflow.
//!
//! # Properties enforced
//!
//! 1. **Job presence** - every manifest gate with `tier="pr-fast"` has a
//!    corresponding job in `pr-fast.yml` (resolved via
//!    `consumer_names["pr-fast"]` if present, else the gate id). Multiple gates
//!    may fold into one job (e.g. `rustdoc` + `doc-tests` → `docs`); the
//!    validator handles many-to-one.
//! 2. **`if:` predicate alignment** - for each pr-fast job, the job's `if:`
//!    predicate must be at least as permissive as the least-upper-bound of the
//!    `gate_when` classes of the gates folded into it.  Wider is fine
//!    (over-runs); narrower is drift (would block a gate from running on its
//!    trigger).
//! 3. **Aggregator coverage** - every gate's resolved job-id is in
//!    `required.needs:` AND the `declare -A R=(...)` aggregator inside the
//!    `required` job.  This is the rename-bookkeeping failure mode the tool
//!    exists for.  (Failure notification lives in
//!    `.github/workflows/ci-failure-notify.yml`, so no `notify-failure` job
//!    needs every gate.)
//! 4. **Branch-protection guard** - the `required` job's `name:` field is
//!    exactly the literal string `PR Fast CI / required`. This name is in the
//!    repo's branch-protection rule (`required_status_checks`); a refactor that
//!    renamed it would silently break merge for every subsequent PR.
//! 5. to 8. **What every workflow shares** - one pin per action, the toolchain
//!    versions and build-target flags of the manifest, and nextest profiles
//!    that exist; [`consistency`] reads every workflow for them.
//! 9. **Change classes** - the `classify` job computes each class with the
//!    pattern `[classification]` gives it, the patterns the hooks are generated
//!    from ([`classes`]).
//! 10. **Commands** - each gate's job runs the gate's command, or the
//!     `ci_command` the gate declares ([`commands`]).
//!
//! A job's display name (`Clippy`) is not checked: several gates fold into
//! one job, so a name is not derivable from the manifest's fields.

// `BTreeMap`/`BTreeSet` live in `alloc`; the workspace
// `clippy::std_instead_of_alloc` lint correctly prefers the canonical
// path over `std::collections::*` re-exports.  Bin crates need the
// explicit `extern crate alloc;` to make the `alloc::` namespace
// visible (libraries get it for free via the 2024 edition's prelude).
extern crate alloc;

use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use clap::Parser;

mod classes;
mod commands;
mod consistency;
mod manifest;
mod validate;
mod workflow;

/// Default path to the gate manifest, relative to the workspace root.
///
/// Hidden CLI flag `--manifest` overrides for tests.
const DEFAULT_MANIFEST_PATH: &str = "scripts/ci/gates.toml";

/// Default path to the GitHub Actions PR workflow, relative to the
/// workspace root.  Hidden CLI flag `--workflow` overrides for tests.
const DEFAULT_WORKFLOW_PATH: &str = ".github/workflows/pr-fast.yml";

/// Every workflow, for properties 5 to 8.
const DEFAULT_WORKFLOWS_DIR: &str = ".github/workflows";

/// The nextest config the workflows' `--profile` names must exist in.
const DEFAULT_NEXTEST_PATH: &str = ".config/nextest.toml";

/// Validate `.github/workflows/pr-fast.yml` against the gate manifest.
///
/// `--check` is the default and only behaviour; the flag exists for
/// symmetry with `md2html-gen-hooks` so a contributor reading the pre-push
/// hook output sees a consistent CLI shape.  There is no `--write`
/// mode (the module docs say why).
#[derive(Debug, Parser)]
#[command(
    name = "md2html-gen-workflow",
    about = "Validate pr-fast.yml structurally against scripts/ci/gates.toml.",
    long_about = None,
)]
struct Args {
    /// Run in check-only mode (the default; flag retained for CLI symmetry
    /// with `md2html-gen-hooks`).  Exits 1 if drift is detected.
    #[arg(long, default_value_t = true)]
    check: bool,

    /// Path to the gate manifest TOML.  Hidden; defaults to the
    /// workspace-relative `scripts/ci/gates.toml`.
    #[arg(long, hide = true, default_value = DEFAULT_MANIFEST_PATH)]
    manifest: PathBuf,

    /// Path to the GitHub Actions workflow YAML.  Hidden; defaults to the
    /// workspace-relative `.github/workflows/pr-fast.yml`.
    #[arg(long, hide = true, default_value = DEFAULT_WORKFLOW_PATH)]
    workflow: PathBuf,

    /// The directory holding every workflow (properties 5 to 8).
    #[arg(long, hide = true, default_value = DEFAULT_WORKFLOWS_DIR)]
    workflows_dir: PathBuf,

    /// The nextest config (property 8).
    #[arg(long, hide = true, default_value = DEFAULT_NEXTEST_PATH)]
    nextest: PathBuf,

    /// Print per-property summary on success (default: silent on success).
    #[arg(long)]
    verbose: bool,
}

/// CLI entry point: answers `--version`, else runs the validator; any
/// failure, drift included, exits `1` with its chain on stderr.
fn main() -> ExitCode {
    let outcome =
        md2html_version::requested_version!("md2html-gen-workflow").map_or_else(run, |text| {
            writeln!(io::stdout().lock(), "{text}").context("standard output could not be written")
        });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        // With standard error closed there is no one left to tell; the
        // exit code says it failed either way.
        Err(err) => match writeln!(io::stderr().lock(), "❌ md2html-gen-workflow: {err:#}") {
            Ok(()) | Err(_) => ExitCode::from(1),
        },
    }
}

/// Writes one line to standard error, where this tool's report goes.
///
/// # Errors
///
/// Standard error is closed or cannot be written.
fn tell(line: core::fmt::Arguments<'_>) -> Result<()> {
    writeln!(io::stderr().lock(), "{line}").context("standard error could not be written")
}

/// Top-level fallible entry point - reads both inputs, runs the
/// validator, exits non-zero on the first drift class detected.
///
/// # Errors
///
/// The manifest or the workflow cannot be read (`read gate manifest at <path>`,
/// `read workflow at <path>`); the manifest does not parse (`parse gate
/// manifest`, from [`manifest::parse`]); `pr-fast.yml` does not parse (`parse
/// pr-fast.yml`, from [`workflow::parse`]); the validator or the cross-workflow
/// checks fail to run (`run structural validator`, `run cross-workflow
/// consistency checks`); standard error cannot be written (from [`tell`]); or
/// drift is found: the issues are listed on standard error and the error is
/// `structural drift detected`.
///
/// A bad command line is not an error here: clap prints the usage and ends the
/// process.
fn run() -> Result<()> {
    let args = Args::parse();

    let manifest_text = std::fs::read_to_string(&args.manifest)
        .with_context(|| format!("read gate manifest at {}", args.manifest.display()))?;
    let workflow_text = std::fs::read_to_string(&args.workflow)
        .with_context(|| format!("read workflow at {}", args.workflow.display()))?;

    let manifest = manifest::parse(&manifest_text).context("parse gate manifest")?;
    let workflow = workflow::parse(&workflow_text).context("parse pr-fast.yml")?;

    let mut report = validate::validate(&manifest, &workflow, &workflow_text)
        .context("run structural validator")?;
    report.extend(
        consistency::check(&manifest, &args.workflows_dir, &args.nextest)
            .context("run cross-workflow consistency checks")?,
    );
    report.extend(classes::check(&manifest, &workflow_text));
    report.extend(commands::check(&manifest, &workflow_text));

    if report.is_empty() {
        if args.verbose {
            tell(format_args!(
                "✅ md2html-gen-workflow: pr-fast.yml is structurally consistent with gates.toml \
                 and computes its change classes, and every workflow agrees on pins, \
                 toolchain, targets and profiles"
            ))?;
            tell(format_args!(
                "   (10 properties checked; see docs/policies/lint-posture.md)"
            ))?;
        }
        Ok(())
    } else {
        tell(format_args!(
            "❌ md2html-gen-workflow: {} structural drift issue(s) detected:",
            report.len()
        ))?;
        for issue in &report {
            tell(format_args!("   • {issue}"))?;
        }
        tell(format_args!(""))?;
        tell(format_args!("   Manifest: {}", args.manifest.display()))?;
        tell(format_args!("   Workflow: {}", args.workflow.display()))?;
        tell(format_args!(
            "   Properties 1-4: pr-fast.yml structure; 5-8: pins, toolchain, targets, nextest \
             profiles across every workflow; 9: pr-fast.yml's change classes; 10: what each \
             gate's job runs \
             (scripts/ci/gates.toml header)."
        ))?;
        anyhow::bail!("structural drift detected");
    }
}
