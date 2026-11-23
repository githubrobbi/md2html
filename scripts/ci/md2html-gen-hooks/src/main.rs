// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html-gen-hooks` - gate-manifest hook generator.
//!
//! Phase 2/3a of `docs/architecture/gates-manifest-plan.md`.  Reads
//! `scripts/ci/gates.toml` and emits one of two hook files depending
//! on `--target`:
//!
//! * `pre-push`   → `scripts/hooks/_lint_pre_push.sh` (Phase 2)
//! * `pre-commit` → `scripts/hooks/_lint_fast.sh`     (Phase 3a)
//!
//! Both targets share the manifest reader, validator, and verbose
//! dump; they differ only in the embedded preamble/footer templates
//! and the dispatch generator (see [`emit`]).
//!
//! # Usage
//!
//! ```text
//! md2html-gen-hooks [--check] [--target {pre-push|pre-commit}] [--verbose]
//! ```
//!
//! # Exit codes
//!
//! | code | meaning |
//! |---:|---|
//! | 0 | emit succeeded (or no-op with `--check`) |
//! | 1 | diff detected (with `--check`) |
//! | 2 | schema error (manifest invalid) or unknown target |

// `BTreeMap`/`BTreeSet` live in `alloc`; the workspace
// `clippy::std_instead_of_alloc` lint correctly prefers the canonical
// path over `std::collections::*` re-exports.  Bin crates need the
// explicit `extern crate alloc;` to make the `alloc::` namespace
// visible (libraries get it for free via the 2024 edition's prelude).
extern crate alloc;

/// Per-target hook emission - turns a parsed manifest into the bash
/// text of `_lint_pre_push.sh`.
mod emit;
/// Manifest schema model + lightweight invariant validation.
mod manifest;

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use clap::Parser;

use crate::emit::EmitTarget;
use crate::manifest::Manifest;

/// CLI-side spelling of [`EmitTarget::PrePush`].
///
/// Kept as a string-typed `clap` arg (rather than a derive on the emit-side
/// enum) so adding a new target only touches `emit.rs` and the
/// `match` in `run`; the stringly-typed parse keeps clap's
/// value-parser story simple.
const TARGET_PRE_PUSH: &str = "pre-push";
/// CLI-side spelling of [`EmitTarget::PreCommit`].  Sibling of
/// [`TARGET_PRE_PUSH`].
const TARGET_PRE_COMMIT: &str = "pre-commit";

/// CLI arguments for `md2html-gen-hooks`.  Flags follow the pattern set by
/// `scripts/ci-pipeline` and the rest of the workspace's internal
/// tools.  See the file-level doc-comment for exit-code semantics.
#[derive(Parser, Debug)]
#[command(
    name = "md2html-gen-hooks",
    version,
    about = "Generate _lint_pre_push.sh from gates.toml (Phase 2 of gates-manifest-plan.md)"
)]
struct Args {
    /// Diff mode: do not write files; exit 1 if regen would change them.
    /// Used by CI's `hooks-drift` job and the pre-push Bucket-1 step.
    #[arg(long)]
    check: bool,

    /// Which hook file to emit.  `pre-push` writes
    /// `scripts/hooks/_lint_pre_push.sh`; `pre-commit` writes
    /// `scripts/hooks/_lint_fast.sh`.  Defaults to `pre-push` for
    /// backwards compatibility with Phase-2 invocations and the
    /// existing `hooks-drift` gate.
    #[arg(long, default_value = TARGET_PRE_PUSH)]
    target: String,

    /// Print per-gate emit decisions to stderr.
    #[arg(long, short)]
    verbose: bool,

    /// Override the manifest path (default: `scripts/ci/gates.toml`).
    /// Test-only escape hatch.
    #[arg(long, hide = true)]
    manifest: Option<PathBuf>,

    /// Override the output path (default: `scripts/hooks/_lint_pre_push.sh`).
    /// Test-only escape hatch.
    #[arg(long, hide = true)]
    output: Option<PathBuf>,
}

/// CLI entry point.
///
/// Answers `--version`, then delegates to [`run`] so
/// the `Result<ExitCode>` propagation pattern stays composable; an
/// emit failure surfaces as exit code `2` (schema error class)
/// with the underlying error chain on stderr.
fn main() -> ExitCode {
    let outcome = md2html_version::requested_version!("md2html-gen-hooks").map_or_else(
        || run(&Args::parse()),
        |text| {
            writeln!(io::stdout().lock(), "{text}")
                .context("standard output could not be written")
                .map(|()| ExitCode::SUCCESS)
        },
    );
    match outcome {
        Ok(code) => code,
        Err(err) => fail(&err),
    }
}

/// Ends a run that stopped on `err`: its chain on standard error, and
/// exit code `2`.
fn fail(err: &anyhow::Error) -> ExitCode {
    // With standard error closed there is no one left to tell; the exit
    // code says it failed either way.
    match writeln!(io::stderr().lock(), "md2html-gen-hooks: {err:#}") {
        Ok(()) | Err(_) => ExitCode::from(2),
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

/// Inner driver: read, validate and render the manifest, then write or check.
///
/// Splits cleanly into: parse args → read manifest → validate → render →
/// (write or check).  Any failure short-circuits via `?`; the return value
/// distinguishes "emit succeeded" (0) from "diff detected in --check mode"
/// (1).  Schema errors propagate up to `main`, which maps them to exit code
/// `2`.
///
/// # Errors
///
/// `--target` is neither `pre-push` nor `pre-commit` (`unknown --target`,
/// naming both); the manifest cannot be read (`reading manifest at`), is
/// not TOML of the manifest's shape (`parsing manifest at`) or breaks a
/// rule of `Manifest::validate` (`validating manifest at`, with the rule);
/// with `--check`, the hook on disk cannot be read (`reading output at`);
/// without it, the hook cannot be written (`writing` and the path); or
/// standard error cannot be written for `--verbose` or the drift report.
/// A hook out of date under `--check` is no error: it is exit code 1.
fn run(args: &Args) -> Result<ExitCode> {
    let target = match args.target.as_str() {
        TARGET_PRE_PUSH => EmitTarget::PrePush,
        TARGET_PRE_COMMIT => EmitTarget::PreCommit,
        other => anyhow::bail!(
            "unknown --target `{other}`; expected one of `{TARGET_PRE_PUSH}` or `{TARGET_PRE_COMMIT}`",
        ),
    };

    let manifest_path = args
        .manifest
        .clone()
        .unwrap_or_else(|| PathBuf::from("scripts/ci/gates.toml"));
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(target.default_output_path()));

    let manifest_text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading manifest at {}", manifest_path.display()))?;
    let manifest: Manifest = toml::from_str(&manifest_text)
        .with_context(|| format!("parsing manifest at {}", manifest_path.display()))?;
    manifest
        .validate()
        .with_context(|| format!("validating manifest at {}", manifest_path.display()))?;

    if args.verbose {
        emit_verbose_dump(&manifest, &manifest_path, target.tier())?;
    }

    let emitted = target.render(&manifest);

    if args.check {
        let on_disk = std::fs::read_to_string(&output_path)
            .with_context(|| format!("reading output at {}", output_path.display()))?;
        if on_disk == emitted {
            if args.verbose {
                tell(format_args!("md2html-gen-hooks: --check passed (no diff)"))?;
            }
            return Ok(ExitCode::SUCCESS);
        }
        let recipe = match target {
            EmitTarget::PrePush => "just md2html-gen-hooks",
            EmitTarget::PreCommit => "just gen-fast",
        };
        tell(format_args!(
            "md2html-gen-hooks: --check FAILED - {} is out of sync with the manifest.\n\
             \n\
             Regenerate it with:\n    {recipe}\n\
             \n\
             (Or run `cargo run -p md2html-gen-hooks -- --target {tier}` directly.)",
            output_path.display(),
            tier = target.tier(),
        ))?;
        return Ok(ExitCode::from(1));
    }

    std::fs::write(&output_path, &emitted)
        .with_context(|| format!("writing {}", output_path.display()))?;
    if args.verbose {
        tell(format_args!(
            "md2html-gen-hooks: wrote {} ({} bytes)",
            output_path.display(),
            emitted.len()
        ))?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Print a human-readable dump of the parsed manifest to stderr.
///
/// Used by `--verbose` to confirm what the generator saw before
/// rendering - header metadata, the `[classification]` regex stack
/// (Phase 3 will wire these into the preamble), and one line per
/// gate that participates in `tier`, including the documentary
/// `expected_runtime_secs` budget and the first line of `notes`.
///
/// # Errors
///
/// Standard error cannot be written.
fn emit_verbose_dump(manifest: &Manifest, manifest_path: &Path, tier: &str) -> Result<()> {
    tell(format_args!(
        "md2html-gen-hooks: parsed manifest at {} (schema v{}{})",
        manifest_path.display(),
        manifest.header.version,
        manifest
            .header
            .plan_doc
            .as_deref()
            .map_or_else(String::new, |plan| format!(", plan: {plan}")),
    ))?;
    if let Some(cls) = manifest.classification.as_ref() {
        let mut keys: Vec<&str> = cls.patterns.keys().map(String::as_str).collect();
        keys.sort_unstable();
        tell(format_args!(
            "md2html-gen-hooks: classification keys ({}): {}",
            keys.len(),
            keys.join(", ")
        ))?;
    }
    tell(format_args!(
        "md2html-gen-hooks: {} gates total, emitting tier `{}`",
        manifest.gate.len(),
        tier
    ))?;
    for gate in &manifest.gate {
        if !gate.tiers.iter().any(|t_name| t_name == tier) {
            continue;
        }
        let first_note = gate
            .notes
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("(no notes)");
        tell(format_args!(
            "  · [{tier}] {id} ({label}) bucket={bucket} when={when} tool={tool} \
             ~{secs}s - {note}",
            tier = tier,
            id = gate.id,
            label = gate.label,
            bucket = gate.bucket.as_deref().unwrap_or("<n/a>"),
            when = gate.when,
            tool = gate.tool,
            secs = gate.expected_runtime_secs,
            note = first_note,
        ))?;
    }
    Ok(())
}
