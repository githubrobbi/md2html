// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Minimal subset of the gate-manifest schema needed by the workflow
//! structural validator.
//!
//! # Why duplicate (a subset of) `scripts/ci/md2html-gen-hooks/src/manifest.rs`?
//!
//! `md2html-gen-hooks` and `md2html-gen-workflow` both read
//! `scripts/ci/gates.toml`. A purist refactor would extract a shared
//! `md2html-gates-schema` library crate that both binaries depend on.  We
//! deliberately deferred that for this PR for two reasons:
//!
//! 1. The cross-tool dependency graph is currently zero-coupled
//!    (md2html-gen-hooks doesn't depend on md2html-gen-workflow and vice
//!    versa); a shared crate would introduce a new internal dependency just to
//!    hold ~30 lines of struct definitions.
//! 2. Serde's default `deserialize_unknown_fields = false` means additive
//!    changes to the manifest schema (new fields in `[[gate]]`) are silently
//!    absorbed by both deserializers - the common drift mode (md2html-gen-hooks
//!    gains a field, md2html-gen-workflow stays blissfully unaware) is safe.
//!    Removal of a field md2html-gen-workflow uses fails noisily on first
//!    parse; loud regression beats silent corruption every time.
//!
//! When this assumption breaks (e.g. both generators need a complex
//! shared validator), promote this module + the md2html-gen-hooks one into a
//! `scripts/ci/gates-schema` library.  Until then: keep it simple.
//!
//! # Schema subset
//!
//! md2html-gen-workflow only needs:
//! - `id` - gate's stable identifier
//! - `label` - human-readable string (used in error messages)
//! - `tiers` - to filter to `pr-fast`
//! - `gate_when` - the change-classification predicate, mapped to the
//!   workflow's `if:` field
//! - `consumer_names` - per-tier job-id override (e.g. `lint-ci` appears as job
//!   `clippy` in pr-fast.yml)
//!
//! Every other field (`command`, `bucket`, `notes`, etc.) is
//! ignored via `serde(default)` on the surrounding struct + serde's
//! permissive mode for unknown TOML keys.

use alloc::collections::BTreeMap;

use anyhow::{Context as _, Result};
use serde::Deserialize;

/// Top-level gate-manifest document.
///
/// Mirrors the shape of `scripts/ci/gates.toml`'s `[[gate]]` array
/// but only carries the fields the workflow validator inspects.
#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    /// One entry per `[[gate]]` block in the TOML.  Renamed to
    /// `gates` on the Rust side for readability.
    #[serde(rename = "gate")]
    pub(crate) gates: Vec<Gate>,

    /// `[toolchain]`: tool versions every workflow must agree on
    /// (property 6); empty when the table is absent.
    #[serde(default)]
    pub(crate) toolchain: BTreeMap<String, String>,

    /// `[[target]]`: the RUSTFLAGS each build target carries
    /// (property 7); empty when the table is absent.
    #[serde(default, rename = "target")]
    pub(crate) targets: Vec<TargetRow>,

    /// `[classification]`: the change classes' path patterns, which the
    /// `classify` job of `pr-fast.yml` must use (property 9).  Required.
    pub(crate) classification: Classification,
}

/// `[classification]`: one extended regular expression per change class.
/// Closed: a class this tool does not know is refused, as
/// `md2html-gen-hooks` refuses it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Classification {
    /// Rust sources.
    rust: String,
    /// The manifests, the lockfile and the supply-chain store.
    dep: String,
    /// The build, check and CI machinery.
    infra: String,
}

impl Classification {
    /// The three classes by the shell variable the `classify` job sets for
    /// each, with its pattern.
    pub(crate) const fn classes(&self) -> [(&'static str, &str); 3] {
        [
            ("RUST", self.rust.as_str()),
            ("DEP", self.dep.as_str()),
            ("INFRA", self.infra.as_str()),
        ]
    }
}

/// One `[[target]]` row: a build target and its flags.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub(crate) struct TargetRow {
    /// The Rust target triple.
    pub(crate) target: String,
    /// The RUSTFLAGS a job or matrix row for that target must carry.
    pub(crate) rustflags: String,
}

/// A single gate entry - minimal subset.  Field ordering matches the
/// canonical schema in `md2html-gen-hooks/src/manifest.rs::Gate` so a
/// reviewer comparing the two structs sees them line up.
#[derive(Debug, Deserialize)]
pub(crate) struct Gate {
    /// Stable kebab-case identifier; primary key for cross-file
    /// references.
    pub(crate) id: String,

    /// Human-readable name surfaced in error messages.  Free-form;
    /// not enforced by any property.
    pub(crate) label: String,

    /// Set of tiers this gate runs in.  The workflow validator
    /// filters to entries that include `"pr-fast"`.
    tiers: Vec<String>,

    /// Change-classification predicate.  Mapped to the workflow's
    /// `if:` field by the validator's `PermissiveSet::from_gate_when`
    /// (property 2 in the crate docs: the predicate-alignment lattice).
    /// The TOML field is named `gate_when` (as `gates.toml` spells
    /// it) but renamed to `when` on the Rust side to avoid
    /// `clippy::struct_field_names` triggering on the `gate_` prefix.
    /// `serde(rename = "gate_when")` bridges the two.
    #[serde(rename = "gate_when")]
    pub(crate) when: String,

    /// Per-tier job-id override.  Multiple gates can map to the
    /// same workflow job (e.g. `rustdoc` + `doc-tests` both have
    /// `consumer_names = { "pr-fast" = "docs" }`).  The validator
    /// resolves each gate to a job-id via this map; if a tier is
    /// not present, the gate id is used directly.
    #[serde(default)]
    consumer_names: BTreeMap<String, String>,

    /// The argv the hooks run (property 10 looks for it in the gate's job).
    #[serde(default)]
    pub(crate) command: Vec<String>,

    /// The `run:` line the gate's pr-fast job uses when it is not
    /// `command` (another runner, CI's own flags): the difference written
    /// where the gate is, and held to the job by property 10.
    #[serde(default)]
    pub(crate) ci_command: Option<String>,
}

/// Parse the manifest TOML.  Errors propagate with file-level context.
///
/// # Errors
///
/// The text is not TOML, lacks the `[[gate]]` array or a field every gate needs
/// (`id`, `label`, `tiers`, `gate_when`), has a `[[target]]` row without
/// `target` or `rustflags`, or gives a field the wrong type (a `[toolchain]`
/// value that is not a string); the message is `deserialize gates.toml` with
/// the TOML error beneath.
pub(crate) fn parse(text: &str) -> Result<Manifest> {
    toml::from_str::<Manifest>(text).context("deserialize gates.toml")
}

impl Manifest {
    /// Iterator over gates that include `"pr-fast"` in their `tiers`
    /// list, in manifest order.  Order matters for deterministic
    /// error reporting.
    pub(crate) fn pr_fast_gates(&self) -> impl Iterator<Item = &Gate> {
        self.gates
            .iter()
            .filter(|gate| gate.tiers.iter().any(|tier| tier == "pr-fast"))
    }
}

impl Gate {
    /// Resolve the workflow job-id this gate maps to in the pr-fast
    /// tier.  Falls back to the gate id when no override is set.
    pub(crate) fn pr_fast_job_id(&self) -> &str {
        self.consumer_names
            .get("pr-fast")
            .map_or(self.id.as_str(), String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal fixture mirroring two real gates from `gates.toml`:
    /// one with no `consumer_names` (`fmt`) and one with a pr-fast
    /// override (`lint-ci` → `clippy`).
    const FIXTURE: &str = r#"
[classification]
rust = '\.rs$'
dep = '^Cargo\.lock$'
infra = '^scripts/'

[[gate]]
id = "fmt"
label = "cargo fmt --check"
tiers = ["pre-commit", "pre-push", "pr-fast"]
gate_when = "rust_changed"

[[gate]]
id = "lint-ci"
label = "CI-mirror clippy"
tiers = ["pre-commit", "pre-push", "pr-fast"]
gate_when = "rust_changed"
consumer_names = { "pr-fast" = "clippy" }

[[gate]]
id = "lint-prod"
label = "ultra-strict production clippy"
tiers = ["pre-commit", "pre-push"]
gate_when = "rust_changed"
"#;

    /// The fixture parses into its three gates, with their ids and `gate_when`
    /// read.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse or does not hold exactly three gates,
    /// the first gate's id is not `fmt`, or the second gate's `gate_when` is
    /// not `rust_changed`.
    #[test]
    fn parses_minimal_manifest() {
        let manifest = parse(FIXTURE).unwrap();
        let [fmt, lint_ci, _] = manifest.gates.as_slice() else {
            panic!("three gates in the fixture");
        };
        assert_eq!(fmt.id, "fmt");
        assert_eq!(lint_ci.when, "rust_changed");
    }

    /// Only the gates whose tiers include `pr-fast` are listed, in manifest
    /// order.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, or the pr-fast gates are not `fmt` then
    /// `lint-ci`: `lint-prod`, which runs only before a commit and a push,
    /// listed, or the order changed.
    #[test]
    fn pr_fast_gates_filters_correctly() {
        let manifest = parse(FIXTURE).unwrap();
        let ids: Vec<&str> = manifest
            .pr_fast_gates()
            .map(|gate| gate.id.as_str())
            .collect();
        assert_eq!(ids, vec!["fmt", "lint-ci"]); // lint-prod excluded
    }

    /// A gate without a pr-fast override maps to the job of its own id.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse or holds no gate, or `fmt` does not map
    /// to job `fmt`.
    #[test]
    fn pr_fast_job_id_falls_back_to_gate_id() {
        let manifest = parse(FIXTURE).unwrap();
        let [fmt, ..] = manifest.gates.as_slice() else {
            panic!("gates in the fixture");
        };
        assert_eq!(fmt.pr_fast_job_id(), "fmt");
    }

    /// A pr-fast entry in `consumer_names` names the gate's job.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse or holds fewer than two gates, or
    /// `lint-ci` does not map to job `clippy`.
    #[test]
    fn pr_fast_job_id_honors_consumer_override() {
        let manifest = parse(FIXTURE).unwrap();
        // lint-ci has consumer_names = { "pr-fast" = "clippy" }
        let [_, lint_ci, ..] = manifest.gates.as_slice() else {
            panic!("two gates in the fixture");
        };
        assert_eq!(lint_ci.pr_fast_job_id(), "clippy");
    }

    /// The TOML field `gate_when` fills the gate's `when`.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, as it would not without the
    /// `serde(rename)`, or a gate's `when` is not `rust_changed`.
    #[test]
    fn gate_when_field_rename_round_trips() {
        // Regression-guards the `serde(rename = "gate_when")` directive
        // - if someone removed it, this would fail to parse.
        let manifest = parse(FIXTURE).unwrap();
        assert!(
            manifest
                .gates
                .iter()
                .all(|gate| gate.when == "rust_changed")
        );
    }

    /// Fields the validator does not use are ignored.
    ///
    /// # Panics
    ///
    /// When a gate that also has `command`, `bucket`, `order` and `notes` does
    /// not parse, or does not give exactly one gate.
    #[test]
    fn unknown_fields_are_silently_ignored() {
        // Documents the schema-coupling design decision: md2html-gen-hooks
        // can add new manifest fields without breaking md2html-gen-workflow.
        let with_extra = r#"
[classification]
rust = '\.rs$'
dep = '^Cargo\.lock$'
infra = '^scripts/'

[[gate]]
id = "fmt"
label = "fmt"
tiers = ["pr-fast"]
gate_when = "rust_changed"
command = ["cargo", "fmt"]
bucket = "bg"
order = 10
notes = "irrelevant for the validator"
"#;
        let manifest = parse(with_extra).unwrap();
        assert_eq!(manifest.gates.len(), 1);
    }

    /// A gate without a field the validator uses fails to parse.
    ///
    /// # Panics
    ///
    /// When a gate without `label` parses, or the error does not mention
    /// `label`.
    #[test]
    fn missing_required_field_fails_noisily() {
        // The other half of the design: removal of a field this
        // validator depends on fails parse, not silently corrupts.
        let bad = r#"
[classification]
rust = '\.rs$'
dep = '^Cargo\.lock$'
infra = '^scripts/'

[[gate]]
id = "fmt"
tiers = ["pr-fast"]
gate_when = "rust_changed"
"#;
        // Missing `label`.
        let err = parse(bad).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("label"),
            "expected error to mention missing field 'label', got: {msg}"
        );
    }
}
