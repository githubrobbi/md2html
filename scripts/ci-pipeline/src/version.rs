// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Version discovery helpers for the MD2HTML ship pipeline.
//!
//! * [`get_current_version`] - read `[workspace.package].version` out of the
//!   root `Cargo.toml` (simple whole-file scan).
//! * [`extract_version_from_cargo_toml`] - same, but strict: only considers the
//!   `[workspace.package]` section.
//! * [`bump_workspace_version`] - the release's new number, by [`Bump`].
//! * [`stamp_documents`] - the same number on the capabilities pages, and the
//!   three places checked to agree (`scripts/ci/versions.py`).

use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::child::{self, captured, finished};

/// Read the workspace root `Cargo.toml` and return the first
/// `version = "..."` string found.  Used by the push step to build
/// `origin/release/vX.Y.Z`.
///
/// # Errors
///
/// Returns an error if `Cargo.toml` cannot be read, or if no `version`
/// line is present.
pub(crate) fn get_current_version() -> Result<String> {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").context("Failed to read Cargo.toml")?;
    for line in cargo_toml.lines() {
        if line.trim().starts_with("version = ")
            && let Some(version) = line.split('"').nth(1)
        {
            return Ok(version.to_owned());
        }
    }
    bail!("Could not find version in Cargo.toml")
}

/// Which number of the version a release raises
/// (`docs/policies/release_policy.md` says which a change needs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Bump {
    /// `x.y.Z`: what keeps the contract.
    Patch,
    /// `x.Y.0`: before 1.0, what may break the contract.
    Minor,
    /// `X.0.0`: from 1.0, what breaks the contract.
    Major,
}

impl Bump {
    /// The level as `cargo set-version --bump` spells it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

/// Bump the lockstep `[workspace.package].version` (plus the internal
/// `[workspace.dependencies]` version requirements and the lockfile) by
/// `level` via `cargo set-version`.
///
/// It shells out to `cargo-edit`'s `set-version` rather than re-implementing
/// semver math: `set-version` already handles workspace
/// inheritance and the internal dep-requirement updates correctly.  `just ship`
/// is a maintainer-only release command, so requiring `cargo-edit` on the
/// release machine is acceptable.
///
/// # Errors
///
/// Returns an error if `cargo set-version` is not installed or the bump fails.
pub(crate) fn bump_workspace_version(level: Bump) -> Result<()> {
    let available = captured(
        Command::new("cargo").args(["set-version", "--help"]),
        child::LOCAL,
    )
    .is_ok_and(|out| out.status.success());
    if !available {
        bail!(
            "`cargo set-version` not found - install it with `cargo install cargo-edit` \
             (required by `just ship` to bump the release version)."
        );
    }
    let bumped = finished(
        Command::new("cargo").args(["set-version", "--bump", level.as_str()]),
        child::LOCAL,
    )
    .context("running `cargo set-version`")?;
    if !bumped.status.success() {
        bail!("`cargo set-version --bump {}` failed", level.as_str());
    }
    // Refresh the lockfile so the release commit is byte-reproducible.
    let updated = finished(
        Command::new("cargo").args(["update", "--workspace", "--quiet"]),
        child::NETWORK,
    )
    .context("refreshing Cargo.lock after version bump")?;
    if !updated.status.success() {
        bail!(
            "`cargo update --workspace` failed ({}) after the version bump",
            updated.status
        );
    }
    Ok(())
}

/// Write `version` onto the capabilities pages and check the three places
/// agree.
///
/// The version is said in three places (AGENTS.md, section 11): the
/// manifest, which [`bump_workspace_version`] wrote; the changelog, which
/// the roll wrote; and the capabilities page with its sixteen translations,
/// written here by `scripts/ci/versions.py stamp`, which also renews the
/// fingerprint of each translation that was current. The same script's
/// `check` (the gate `versions`) then confirms they agree, so the release
/// commit is never made with one of them behind.
///
/// # Errors
///
/// `python3` cannot be run or does not end within a minute, the stamp
/// fails, or the check finds a place that disagrees (its findings are
/// printed above).
pub(crate) fn stamp_documents(version: &str) -> Result<()> {
    let script = "scripts/ci/versions.py";
    let stamped = finished(
        Command::new("python3").args([script, "stamp", version]),
        child::LOCAL,
    )
    .context("running `versions.py stamp`")?;
    if !stamped.status.success() {
        bail!("`versions.py stamp {version}` failed ({})", stamped.status);
    }
    let checked = finished(Command::new("python3").arg(script), child::LOCAL)
        .context("running `versions.py`")?;
    if !checked.status.success() {
        bail!("the version {version} does not agree in its three places (`versions.py` above)");
    }
    Ok(())
}

/// The `[workspace.package]` version in `content`, a workspace `Cargo.toml`.
///
/// It parses `content` (the text of a workspace root `Cargo.toml`) and
/// extracts the `version = "..."` entry from the `[workspace.package]`
/// table specifically - ignores any unrelated `version = ...` lines in
/// `[dependencies]` or per-crate overrides.
///
/// # Errors
///
/// Returns an error if `[workspace.package]` is missing, or if it does
/// not contain a parseable `version` entry.
pub(crate) fn extract_version_from_cargo_toml(content: &str) -> Result<String> {
    let mut in_workspace_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace.package]" {
            in_workspace_package = true;
            continue;
        }
        if in_workspace_package {
            if trimmed.starts_with('[') && trimmed != "[workspace.package]" {
                break;
            }
            if trimmed.starts_with("version")
                && let Some((_, after_eq)) = trimmed.split_once('=')
            {
                let version = after_eq.trim().trim_matches('"').trim_matches('\'');
                return Ok(version.to_owned());
            }
        }
    }
    bail!("Version extraction failed - no version found in [workspace.package]")
}
