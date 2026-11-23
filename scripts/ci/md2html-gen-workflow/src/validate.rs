// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Structural validation logic.
//!
//! Each of the four properties from plan §4.2 is implemented as a
//! standalone function returning `Vec<String>` of human-readable
//! drift descriptions (empty on success).  [`validate`] runs all
//! four and concatenates results.
//!
//! # Design choices
//!
//! - **No early returns between properties.**  A single drift in Property 1
//!   should not hide drift in Property 3.  Operators debugging a manifest
//!   renumbering want to see all issues at once.
//! - **`required` job's bash table is parsed via regex over the raw workflow
//!   text**, not via the YAML model.  The bash script is a YAML scalar string;
//!   structural validation of *bash code* would need a bash parser, which is
//!   overkill for the `[<job-id>]='${{ ... }}'` line shape.  Regex with a tight
//!   anchor pattern is sufficient and self-documenting.
//! - **Predicate alignment uses a [`PermissiveSet`] lattice.**  The set of
//!   change classes a job must accept is the union of its folded gates'
//!   classes; the set the `if:` predicate accepts is parsed from a small
//!   recognised vocabulary (see [`PermissiveSet::from_if_expr`]).  Drift = any
//!   class needed but not accepted.

use alloc::collections::{BTreeMap, BTreeSet};

use anyhow::{Context as _, Result};
use regex::Regex;

use crate::manifest::{Gate, Manifest};
use crate::workflow::Workflow;

/// Run all four structural property checks; return the concatenated
/// list of drift descriptions in property order so reviewers can
/// pattern-match the output regardless of how many issues fire at
/// once.
///
/// # Errors
///
/// Returns an error if a regex used by Property 3 fails to compile;
/// in practice this can only happen if the `regex` crate itself is
/// broken, since the patterns are literal strings tested by the
/// crate's unit tests.
pub(crate) fn validate(
    manifest: &Manifest,
    workflow: &Workflow,
    workflow_text: &str,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();
    issues.extend(check_job_presence(manifest, workflow));
    issues.extend(check_if_predicates(manifest, workflow));
    issues.extend(check_aggregator_coverage(
        manifest,
        workflow,
        workflow_text,
    )?);
    issues.extend(check_required_name_guard(workflow));
    Ok(issues)
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 1 - Job presence
// ─────────────────────────────────────────────────────────────────────────────

/// Every pr-fast-tier gate's job must be in the workflow.
///
/// For every pr-fast-tier gate, the resolved job-id must exist as a
/// key in the workflow's `jobs:` map.  Multiple gates may resolve to
/// the same job-id (e.g. `rustdoc` + `doc-tests` → `docs`); each gate
/// that maps to a missing job is reported separately so a reviewer
/// sees the full picture.
fn check_job_presence(manifest: &Manifest, workflow: &Workflow) -> Vec<String> {
    let mut issues = Vec::new();
    for gate in manifest.pr_fast_gates() {
        let job_id = gate.pr_fast_job_id();
        if !workflow.jobs.contains_key(job_id) {
            issues.push(format!(
                "Property 1 (job presence): manifest gate `{gate_id}` (label={label:?}) resolves to job `{job_id}` which is missing from pr-fast.yml",
                gate_id = gate.id,
                label = gate.label,
            ));
        }
    }
    issues
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 2 - `if:` predicate alignment
// ─────────────────────────────────────────────────────────────────────────────

/// Set of change classes a predicate (or a fold of gates) accepts.
///
/// Implements the Property 2 lattice: a job's `if:` is valid iff its
/// set is a SUPERSET of the union of its folded gates' sets.
///
/// Modeled as a `u8` bitset with one bit per change class
/// (rust / dep / infra / always).  This is materially cleaner than
/// the original `struct { rust: bool, dep: bool, infra: bool,
/// always: bool }` shape - single-byte values, free `Copy`, free
/// `union` via `|`, free `contains` via `&` - and avoids the
/// `clippy::struct_excessive_bools` lint at the root cause (the
/// data shape itself, not via a per-item suppression).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct PermissiveSet(u8);

impl PermissiveSet {
    /// `*.rs` files changed.
    const RUST: Self = Self(1 << 0);
    /// `Cargo.toml` / `Cargo.lock` / `supply-chain/` changed.
    const DEP: Self = Self(1 << 1);
    /// `.github/`, `scripts/`, `.cargo/`, `just/` etc. changed.
    const INFRA: Self = Self(1 << 2);
    /// "Run unconditionally" - `always()` / no `if:` field / no
    /// classification gating.  Strictly stronger than rust+dep+infra
    /// because it also runs on pure docs-only PRs that wouldn't
    /// trigger any of the other three classes.
    const ALWAYS: Self = Self(1 << 3);
    /// rust + dep + infra (matches `gate_when = "code_changed"`).
    const CODE: Self = Self(Self::RUST.0 | Self::DEP.0 | Self::INFRA.0);
    /// rust + dep + infra + always (matches `gate_when = "always"`
    /// and the `always()` workflow predicate).
    const ALL: Self = Self(Self::CODE.0 | Self::ALWAYS.0);

    /// Build the set a single gate's `gate_when` field accepts.
    /// Unknown values yield an empty set, which guarantees a
    /// downstream drift report (the job will fail to satisfy the
    /// superset check, surfacing the unrecognised value).
    fn from_gate_when(when: &str) -> Self {
        match when {
            "always" => Self::ALL,
            "code_changed" => Self::CODE,
            "rust_changed" => Self::RUST,
            "dep_changed" => Self::DEP,
            "infra_changed" => Self::INFRA,
            _ => Self::default(),
        }
    }

    /// Parse the workflow's `if:` predicate string into a
    /// permissive set.  Only the vocabulary actually used in
    /// `pr-fast.yml` is recognised:
    ///
    /// - `always()` or `None`                       → all four bits
    /// - `needs.classify.outputs.code == 'true'`    → rust + dep + infra
    /// - `needs.classify.outputs.rust == 'true'`    → rust
    /// - `needs.classify.outputs.dep == 'true'`     → dep
    /// - `needs.classify.outputs.infra == 'true'`   → infra
    ///
    /// Any other shape falls through to the default empty set,
    /// triggering a drift report on the next superset check.
    fn from_if_expr(expr: Option<&str>) -> Self {
        let Some(text) = expr else {
            // No `if:` field → unconditional → all classes accepted.
            return Self::ALL;
        };
        if text.contains("always()") {
            return Self::ALL;
        }
        if text.contains("classify.outputs.code") {
            return Self::CODE;
        }
        let mut set = Self::default();
        if text.contains("classify.outputs.rust") {
            set = set.union(Self::RUST);
        }
        if text.contains("classify.outputs.dep") {
            set = set.union(Self::DEP);
        }
        if text.contains("classify.outputs.infra") {
            set = set.union(Self::INFRA);
        }
        set
    }

    /// Set union - used to combine multiple folded gates'
    /// requirements into a single demand the job must satisfy.
    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// `true` iff every bit set in `needed` is also set in `self`.
    /// Drift = `!self.contains(needed)`.
    const fn contains(self, needed: Self) -> bool {
        (self.0 & needed.0) == needed.0
    }

    /// Human-readable list of the change classes this set accepts,
    /// for error messages.
    fn describe(self) -> String {
        if self == Self::ALL {
            return "always (rust, dep, infra, docs-only)".to_owned();
        }
        let mut parts = Vec::new();
        if self.contains(Self::RUST) {
            parts.push("rust");
        }
        if self.contains(Self::DEP) {
            parts.push("dep");
        }
        if self.contains(Self::INFRA) {
            parts.push("infra");
        }
        if self.contains(Self::ALWAYS) {
            parts.push("always");
        }
        if parts.is_empty() {
            "(none - predicate not recognised)".to_owned()
        } else {
            parts.join(" + ")
        }
    }
}

/// Group manifest gates by resolved pr-fast job-id, preserving
/// manifest order for deterministic error reporting.
fn group_by_job_id(manifest: &Manifest) -> BTreeMap<&str, Vec<&Gate>> {
    let mut groups: BTreeMap<&str, Vec<&Gate>> = BTreeMap::new();
    for gate in manifest.pr_fast_gates() {
        groups.entry(gate.pr_fast_job_id()).or_default().push(gate);
    }
    groups
}

/// For each job that has manifest gates folded into it, verify the
/// job's `if:` predicate accepts every change class the constituent
/// gates need to run on.
fn check_if_predicates(manifest: &Manifest, workflow: &Workflow) -> Vec<String> {
    let mut issues = Vec::new();
    for (job_id, gates) in group_by_job_id(manifest) {
        let Some(job) = workflow.jobs.get(job_id) else {
            // Job absent → already reported by Property 1.
            continue;
        };
        let needed: PermissiveSet = gates
            .iter()
            .map(|gate| PermissiveSet::from_gate_when(&gate.when))
            .fold(PermissiveSet::default(), PermissiveSet::union);

        let actual = PermissiveSet::from_if_expr(job.if_expr.as_deref());

        if !actual.contains(needed) {
            let gate_summary = gates
                .iter()
                .map(|gate| format!("{}({})", gate.id, gate.when))
                .collect::<Vec<_>>()
                .join(", ");
            issues.push(format!(
                "Property 2 (if: predicate alignment): job `{job_id}` accepts [{actual_desc}] but folded gates [{gate_summary}] need [{needed_desc}]; predicate is too narrow",
                actual_desc = actual.describe(),
                needed_desc = needed.describe(),
            ));
        }
    }
    issues
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 3 - Aggregator coverage
// ─────────────────────────────────────────────────────────────────────────────

/// Pattern for the bash `R=(...)` table line shape:
/// `<whitespace>[<job-id>]=<rest>`.
///
/// Anchored to start-of-line (after whitespace) and an opening `[`
/// so the regex does not match `${{ needs.foo.result }}`
/// substitutions or other bracket-bearing tokens that happen to sit
/// mid-line.
const AGGREGATOR_LINE_PATTERN: &str = r"^\s*\[([a-zA-Z0-9_-]+)\]=";

/// Compile the [`AGGREGATOR_LINE_PATTERN`] regex.
///
/// # Errors
///
/// Returns an error if the `regex` crate fails to compile a literal
/// pattern.  In practice that only happens if the crate itself is
/// broken; the pattern is exercised by every test in this module so
/// any real breakage surfaces immediately.
fn aggregator_table_regex() -> Result<Regex> {
    Regex::new(AGGREGATOR_LINE_PATTERN)
        .with_context(|| format!("compile aggregator regex {AGGREGATOR_LINE_PATTERN:?}"))
}

/// Extract the set of job-ids declared in the `required` job's bash
/// `declare -A R=(...)` table.
///
/// Operates on the raw workflow text because the bash code lives inside a YAML
/// string scalar - pulling it from the YAML model requires schema-modelling
/// step contents, which the validator deliberately avoids (see plan §4.2 "What
/// is explicitly NOT validated").
///
/// # Errors
///
/// Propagates the regex compilation error from
/// [`aggregator_table_regex`].
fn extract_aggregator_ids(workflow_text: &str) -> Result<BTreeSet<String>> {
    let regex = aggregator_table_regex()?;
    let mut inside_table = false;
    let mut ids = BTreeSet::new();
    for line in workflow_text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("declare -A R=(") {
            inside_table = true;
            continue;
        }
        if inside_table {
            // The closing `)` ends the table.  Match it with leading
            // whitespace only to avoid mid-line `)` in expressions.
            if trimmed.starts_with(')') {
                inside_table = false;
                continue;
            }
            if let Some(captures) = regex.captures(line)
                && let Some(id) = captures.get(1)
            {
                ids.insert(id.as_str().to_owned());
            }
        }
    }
    Ok(ids)
}

/// For each manifest gate's resolved job-id, verify presence in:
/// 1. `workflow.jobs["required"].needs`
/// 2. the bash `declare -A R=(...)` table inside `required`
///
/// The third invariant (coverage in `workflow.jobs["notify-failure"].needs:`)
/// was retired in the Design C refactor for #209.  Failure notification
/// is now produced by `.github/workflows/ci-failure-notify.yml`, which
/// triggers off `workflow_run [completed]` rather than via an in-workflow
/// `notify-failure` job needing a `needs:` list of every gate.
///
/// # Errors
///
/// Propagates the regex compilation error from
/// [`extract_aggregator_ids`].
fn check_aggregator_coverage(
    manifest: &Manifest,
    workflow: &Workflow,
    workflow_text: &str,
) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    let required_needs: BTreeSet<String> = workflow
        .jobs
        .get("required")
        .map_or_else(BTreeSet::new, |job| job.needs.iter().cloned().collect());
    let aggregator_ids = extract_aggregator_ids(workflow_text)?;

    // Each manifest job-id must appear in both lists.  We dedupe via
    // BTreeSet to avoid duplicate reports for folded gates (e.g.
    // rustdoc + doc-tests both → docs).
    let needed_ids: BTreeSet<&str> = manifest.pr_fast_gates().map(Gate::pr_fast_job_id).collect();

    for job_id in needed_ids {
        if !required_needs.contains(job_id) {
            issues.push(format!(
                "Property 3 (aggregator coverage): job `{job_id}` is missing from required.needs:"
            ));
        }
        if !aggregator_ids.contains(job_id) {
            issues.push(format!(
                "Property 3 (aggregator coverage): job `{job_id}` is missing from the `declare -A R=(...)` table inside the required job"
            ));
        }
    }

    Ok(issues)
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 4 - Branch-protection guard
// ─────────────────────────────────────────────────────────────────────────────

/// The literal string GitHub branch protection matches against.
/// Source-of-truth: the repository's
/// `required_status_checks.contexts` ruleset.  Plan §4.2 Property 4
/// enforces this string verbatim.
const REQUIRED_JOB_NAME: &str = "PR Fast CI / required";

/// The `required` job's `name:` field MUST be exactly
/// [`REQUIRED_JOB_NAME`].  A rename here silently breaks branch
/// protection for every subsequent PR - there's no other guard.
fn check_required_name_guard(workflow: &Workflow) -> Vec<String> {
    let mut issues = Vec::new();
    let Some(job) = workflow.jobs.get("required") else {
        issues.push(format!(
            "Property 4 (branch-protection guard): job key `required` is missing from pr-fast.yml; branch protection requires a job whose name is exactly `{REQUIRED_JOB_NAME}`"
        ));
        return issues;
    };
    match job.name.as_deref() {
        Some(name) if name == REQUIRED_JOB_NAME => {}
        Some(name) => {
            issues.push(format!(
                "Property 4 (branch-protection guard): required job's name is `{name:?}` but branch protection requires exactly `{REQUIRED_JOB_NAME}`"
            ));
        }
        None => {
            issues.push(format!(
                "Property 4 (branch-protection guard): required job has no `name:` field; branch protection requires exactly `{REQUIRED_JOB_NAME}`"
            ));
        }
    }
    issues
}

#[cfg(test)]
mod tests;
