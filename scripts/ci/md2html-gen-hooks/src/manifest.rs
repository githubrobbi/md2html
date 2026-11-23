// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
//
// Manifest schema model - mirrors `scripts/ci/gates.toml`.
//
// The `gates.toml` file is the canonical source-of-truth for the
// workspace's PR-time gate set.  See `docs/architecture/gates-manifest-plan.md`
// §3 for the schema spec.  This module does the bare minimum:
//
//   1. Deserialise the file via `serde` + `toml::from_str`.
//   2. Validate the invariants the codegen relies on (no duplicate ids, every
//      `tiers` entry valid, every `bucket` valid, every `gate_when` valid).
//
// The validator is intentionally lightweight - it only catches schema
// bugs that would crash the generator.  Cross-consumer drift detection
// is the Phase-1 `check_gates_drift.sh` script's job and stays
// authoritative there.

use alloc::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use serde::Deserialize;

/// Top-level manifest layout.  TOML key names match `gates.toml`
/// verbatim so `[[gate]]` arrays of tables deserialise straight in.
#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    /// Optional `[manifest]` header table - version + plan-doc
    /// cross-reference.  Surfaced by `--verbose`; Phase 3 will also
    /// gate generator compatibility on `version`.
    #[serde(default, rename = "manifest")]
    pub(crate) header: ManifestMeta,

    /// Optional `[classification]` table - the regex stack used by
    /// `_lint_pre_push.sh` for `RUST_CHANGED` / `DEP_CHANGED` /
    /// `INFRA_CHANGED` detection.  Phase 2 keeps the regexes in the
    /// embedded `preamble` template (§5 of the plan: codegen is
    /// layered, not big-bang); the generator only echoes them via
    /// `--verbose` so contributors can confirm what they parsed.
    /// Phase 3 will wire them through to the preamble.
    #[serde(default)]
    pub(crate) classification: Option<Classification>,

    /// Every `[[gate]]` table in the manifest.  TOML preserves
    /// declaration order on parse; the generator re-sorts before
    /// emit (see `emit.rs`) so the output is deterministic regardless
    /// of declaration order.
    #[serde(default)]
    pub(crate) gate: Vec<Gate>,
}

/// `[manifest]` header table: the manifest's version and plan doc.
///
/// Captures the manifest's own version
/// and the cross-reference back to the plan doc.  All fields are
/// optional so an early manifest snapshot without the header still
/// parses cleanly.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct ManifestMeta {
    /// Schema version.  Surfaced by `--verbose`; Phase 3 will also
    /// reject manifests whose major bumps this past the generator's
    /// known maximum.
    #[serde(default)]
    pub(crate) version: u32,
    /// Path (relative to repo root) of the architecture plan
    /// document this manifest implements.  Surfaced by `--verbose`
    /// and (Phase 2.x) the regen banner.
    #[serde(default)]
    pub(crate) plan_doc: Option<String>,
}

/// `[classification]` is a free-form map of class-name → regex pattern.
///
/// Phase 2 does not consume these for emission (the regexes are
/// embedded literally in `templates/preamble.sh`); the generator only
/// echoes them via `--verbose`.  Phase 3 will tie the two together so
/// the manifest's regex stack drives the preamble's classification
/// block.
#[derive(Debug, Deserialize)]
pub(crate) struct Classification {
    /// Class-name → regex map.  Class names are short identifiers like
    /// `rust`, `dep`, `infra`, `docs`; the values are anchored regex
    /// patterns matched against `git diff --name-only` output.
    #[serde(flatten)]
    pub(crate) patterns: BTreeMap<String, String>,
}

/// One `[[gate]]` table.
///
/// Field names match `gates.toml` modulo the
/// `when` ↔ `gate_when` rename: the TOML schema spells the trigger
/// classifier `gate_when` (matching the Phase-1 plan doc and the
/// drift-detector script), while the Rust field uses the unprefixed
/// `when` to keep the struct-name-prefix lint clean.  `serde(rename)`
/// bridges the two without requiring the manifest authors to know
/// the Rust spelling.
#[derive(Debug, Deserialize)]
pub(crate) struct Gate {
    /// Kebab-case canonical identifier - the `spawn_bg` / `run_seq`
    /// label emitted into the hook (modulo `consumer_names` overrides).
    pub(crate) id: String,
    /// Human-readable name surfaced by drift-detector messages and
    /// (future) the regen banner.
    pub(crate) label: String,
    /// Command line as a TOML array.  Token zero is the executable;
    /// subsequent tokens are arguments.  See `emit::format_command`
    /// for the bash-quoting rules.
    pub(crate) command: Vec<String>,
    /// Subset of `{pre-commit, pre-push, pr-fast, tier-2}`.  The
    /// generator filters by membership.
    pub(crate) tiers: Vec<String>,
    /// One of `always` / `rust_changed` / `dep_changed` /
    /// `infra_changed` / `code_changed`.  Drives Bucket-2 inner
    /// guards and (future) Bucket-1 conditional emission.
    /// TOML schema name: `gate_when` (preserved via `serde(rename)`).
    #[serde(rename = "gate_when")]
    pub(crate) when: String,
    /// The binary the gate needs.  Every gate is required: a missing
    /// tool fails it.  Tools outside the baseline (`ASSUMED_TOOLS`) are
    /// listed in the hook's `REQUIRED_TOOLS`, and `cargo-vet` also gets
    /// a fail-with-install-hint block.
    pub(crate) tool: String,
    /// Documentary expected runtime in seconds.  Surfaced by
    /// `--verbose`; Phase 2.x's manifest viewer + Tier-2 budget
    /// pre-flight will also read it.
    #[serde(default)]
    pub(crate) expected_runtime_secs: u32,
    /// Bucket assignment - `"bg"` (Bucket 1, fire-and-forget) or
    /// `"seq"` (Bucket 2, sequential / fail-fast).  Optional because
    /// it is only meaningful when the gate participates in the
    /// `pre-push` tier; pr-fast-only gates (e.g. the full `tests` run
    /// that is too slow for pre-push) legitimately omit it.
    #[serde(default)]
    pub(crate) bucket: Option<String>,
    /// Within-bucket ordering hint.  Lower fires first.  Ties
    /// resolve by lexicographic id ordering (see
    /// `Manifest::gates_for_tier`).
    #[serde(default)]
    pub(crate) order: i32,
    /// Per-tier consumer-name override.  When set, the generator
    /// emits the override (e.g. `tests`) as the `spawn_bg` /
    /// `run_seq` label instead of the canonical gate id (e.g.
    /// `test-build`).  Read by `emit::consumer_label`; the same
    /// field shape is consumed by `check_gates_drift.sh` so the
    /// two stay aligned.
    #[serde(default)]
    pub(crate) consumer_names: BTreeMap<String, String>,
    /// Free-form Markdown.  The first non-blank line is surfaced by
    /// `--verbose`.  Phase 2.x will also emit it as a comment block
    /// in the rendered hook (see `emit.rs::render_dispatch`).
    #[serde(default)]
    pub(crate) notes: String,
}

/// The placeholder a pre-push command holds for one pushed commit range
/// (`BASE..OID`); the generated hook runs the command once per range.
pub(crate) const COMMIT_RANGES: &str = "{{COMMIT_RANGES}}";

/// The placeholder a pre-commit command holds for the staged TOML files
/// outside `supply-chain/`.
pub(crate) const STAGED_TOML: &str = "{{STAGED_TOML}}";

impl Gate {
    /// The id becomes a bash function name and a file name in the hook
    /// (lowercase ASCII letters, digits and `-`), the command is not empty,
    /// and a placeholder is a whole token, never part of one (it would
    /// reach the hook as literal text).
    ///
    /// # Errors
    ///
    /// The id is empty or holds anything but lowercase ASCII letters,
    /// digits and `-` (`must be lowercase ASCII letters, digits and -`);
    /// the command is empty (`has an empty command`); or a token holds
    /// `{{` without being exactly `{{COMMIT_RANGES}}` or `{{STAGED_TOML}}`
    /// (the message names the token and the two placeholders).  Each
    /// message names the gate.
    fn check_id_and_command(&self) -> Result<()> {
        ensure!(
            !self.id.is_empty()
                && self
                    .id
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'),
            "gate id `{}` must be lowercase ASCII letters, digits and `-`",
            self.id
        );
        ensure!(
            !self.command.is_empty(),
            "gate `{}` has an empty command",
            self.id
        );
        for token in &self.command {
            ensure!(
                !token.contains("{{") || token == COMMIT_RANGES || token == STAGED_TOML,
                "gate `{}` has `{token}`: a placeholder must be a whole token, \
                 `{COMMIT_RANGES}` or `{STAGED_TOML}`",
                self.id
            );
        }
        Ok(())
    }
}

impl Manifest {
    /// Lightweight invariant validation - only the things the codegen
    /// itself depends on.  Cross-consumer drift is Phase 1's job.
    ///
    /// # Errors
    ///
    /// The first broken rule: a gate id seen before (`duplicate gate id`);
    /// then, gate by gate, an id or command `Gate::check_id_and_command`
    /// refuses; a `bucket` other than `bg` or `seq` (`unknown bucket`); a
    /// gate in the `pre-push` tier without a `bucket` (`has no bucket
    /// field`); a `gate_when` other than `always`, `rust_changed`,
    /// `dep_changed`, `infra_changed` or `code_changed` (`unknown
    /// gate_when`); or a tier other than `pre-commit`, `pre-push`,
    /// `pr-fast` or `tier-2` (`unknown tier`).  Each message names the
    /// gate.
    pub(crate) fn validate(&self) -> Result<()> {
        // No duplicate ids.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for gate in &self.gate {
            ensure!(
                seen.insert(gate.id.as_str()),
                "duplicate gate id `{}` in manifest",
                gate.id
            );
        }

        // Every gate has a usable id and command and a known bucket / when.
        for gate in &self.gate {
            gate.check_id_and_command()?;
            // `bucket` is required IFF the gate participates in pre-push.
            // pr-fast-only gates (e.g. full `tests`) legitimately omit it.
            let in_pre_push = gate.tiers.iter().any(|tier_name| tier_name == "pre-push");
            match (in_pre_push, gate.bucket.as_deref()) {
                (true, Some(bucket)) => ensure!(
                    matches!(bucket, "bg" | "seq"),
                    "gate `{}` has unknown bucket `{}` (want `bg` or `seq`)",
                    gate.id,
                    bucket
                ),
                (true, None) => anyhow::bail!(
                    "gate `{}` is in the `pre-push` tier but has no `bucket` field",
                    gate.id
                ),
                (false, Some(bucket)) => ensure!(
                    matches!(bucket, "bg" | "seq"),
                    "gate `{}` has unknown bucket `{}` (want `bg` or `seq`)",
                    gate.id,
                    bucket
                ),
                (false, None) => {}
            }
            ensure!(
                matches!(
                    gate.when.as_str(),
                    "always" | "rust_changed" | "dep_changed" | "infra_changed" | "code_changed"
                ),
                "gate `{}` has unknown gate_when `{}`",
                gate.id,
                gate.when
            );
            for tier in &gate.tiers {
                ensure!(
                    matches!(
                        tier.as_str(),
                        "pre-commit" | "pre-push" | "pr-fast" | "tier-2"
                    ),
                    "gate `{}` has unknown tier `{}`",
                    gate.id,
                    tier
                );
            }
        }
        Ok(())
    }

    /// Filter gates by tier membership - the codegen iterates this
    /// once per emit.  Sorted by `(bucket-as-defined-order, order)` so
    /// Bucket-1 `spawn_bg` lines come before Bucket-2 `run_seq` lines
    /// and within each bucket gates fire in the order the plan declares.
    pub(crate) fn gates_for_tier<'a>(&'a self, tier: &str) -> Vec<&'a Gate> {
        let mut out: Vec<&'a Gate> = self
            .gate
            .iter()
            .filter(|gate| gate.tiers.iter().any(|tier_name| tier_name == tier))
            .collect();
        out.sort_by_key(|gate| {
            (
                bucket_rank(gate.bucket.as_deref().unwrap_or("")),
                gate.order,
                gate.id.as_str().to_owned(),
            )
        });
        out
    }
}

/// Sort key for bucket strings: Bucket 1 (`bg`) before Bucket 2
/// (`seq`); pr-fast-only gates without a bucket sink to the bottom.
fn bucket_rank(bucket: &str) -> u8 {
    match bucket {
        "bg" => 0,
        "seq" => 1,
        _ => 99,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        // Minimal valid manifest covering one bg + one seq gate.
        r#"
[manifest]
version = 1
plan_doc = "docs/architecture/gates-manifest-plan.md"

[classification]
rust  = "\\.rs$"
dep   = "^(.*Cargo\\.toml$|Cargo\\.lock$|supply-chain/)"
infra = "^(\\.github/|scripts/|)"

[[gate]]
id        = "fmt"
label     = "cargo fmt --check"
command   = ["cargo", "fmt", "--all", "--", "--check"]
tiers     = ["pre-push"]
gate_when = "always"
tool      = "cargo"
bucket    = "bg"
order     = 10
notes     = "Always-on rustfmt."

[[gate]]
id        = "lint-ci"
label     = "CI-mirror clippy"
command   = ["just", "lint-ci"]
tiers     = ["pre-commit", "pre-push"]
gate_when = "rust_changed"
tool      = "cargo"
bucket    = "seq"
order     = 20
"#
    }

    /// The fixture parses into two gates and schema version 1.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, or holds other than two gates or
    /// version 1.
    #[test]
    fn parses_minimal_manifest() {
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        assert_eq!(manifest.gate.len(), 2);
        assert_eq!(manifest.header.version, 1);
    }

    /// The fixture passes validation.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, or the validator refuses it.
    #[test]
    fn validates_clean() {
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        manifest.validate().unwrap();
    }

    /// A second gate with the id `fmt` is refused.
    ///
    /// # Panics
    ///
    /// When the manifest does not parse, the validator accepts it, or its error
    /// does not say `duplicate gate id`.
    #[test]
    fn rejects_duplicate_ids() {
        let dup = format!(
            "{}\n[[gate]]\nid=\"fmt\"\nlabel=\"x\"\ncommand=[\"true\"]\ntiers=[\"pre-push\"]\ngate_when=\"always\"\ntool=\"bash\"\nbucket=\"bg\"\n",
            fixture()
        );
        let manifest: Manifest = toml::from_str(&dup).unwrap();
        let err = manifest.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate gate id"));
    }

    /// A bucket other than `bg` or `seq` is refused.
    ///
    /// # Panics
    ///
    /// When the manifest with the bucket `nope` does not parse, is accepted, or
    /// its error does not say `unknown bucket`.
    #[test]
    fn rejects_bad_bucket() {
        let bad = fixture().replace("bucket    = \"bg\"", "bucket    = \"nope\"");
        let manifest: Manifest = toml::from_str(&bad).unwrap();
        let err = manifest.validate().unwrap_err();
        assert!(err.to_string().contains("unknown bucket"));
    }

    /// The pre-push gates come back with Bucket 1 before Bucket 2.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, or its pre-push gates are not `fmt`
    /// then `lint-ci`.
    #[test]
    fn gates_for_tier_filters_and_sorts() {
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        let ids: Vec<&str> = manifest
            .gates_for_tier("pre-push")
            .iter()
            .map(|gate| gate.id.as_str())
            .collect();
        // Bucket bg before seq.
        assert_eq!(ids, ["fmt", "lint-ci"]);
    }

    /// An unknown tier has no gates.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, or a gate comes back for the tier
    /// `nonexistent`.
    #[test]
    fn gates_for_tier_empty_for_unknown_tier() {
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        assert!(manifest.gates_for_tier("nonexistent").is_empty());
    }

    /// The TOML key `gate_when` fills the field `when`.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, `fmt` or `lint-ci` is missing from it,
    /// or their `when` is not `always` and `rust_changed`.
    #[test]
    fn gate_when_field_rename_round_trips() {
        // The TOML schema spells the trigger classifier `gate_when`
        // (matching the Phase-1 plan doc and the drift detector); the
        // Rust field is named `when`.  Regression-guard for the
        // `serde(rename = "gate_when")` directive.
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        let fmt_gate = manifest.gate.iter().find(|gate| gate.id == "fmt").unwrap();
        assert_eq!(fmt_gate.when, "always");
        let clippy_gate = manifest
            .gate
            .iter()
            .find(|gate| gate.id == "lint-ci")
            .unwrap();
        assert_eq!(clippy_gate.when, "rust_changed");
    }

    /// The `[classification]` table parses into its class-name → regex map.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse, has no `[classification]` table, does
    /// not hold exactly three classes, maps `rust` to anything but `\.rs$`, or
    /// lacks `dep` or `infra`.
    #[test]
    fn classification_block_parses() {
        // Regression-guard for the free-form `Classification`
        // shape: any class-name → regex map should round-trip
        // cleanly into the BTreeMap.  The verbose-dump path reads
        // these keys; if parsing breaks, --verbose breaks silently.
        let manifest: Manifest = toml::from_str(fixture()).unwrap();
        let cls = manifest
            .classification
            .as_ref()
            .expect("classification block");
        assert_eq!(cls.patterns.len(), 3);
        assert_eq!(cls.patterns.get("rust").map(String::as_str), Some(r"\.rs$"));
        assert!(cls.patterns.contains_key("dep"));
        assert!(cls.patterns.contains_key("infra"));
    }

    /// Per-tier `consumer_names` parse, in tier order.
    ///
    /// # Panics
    ///
    /// When the manifest does not parse or validate, holds other than one gate,
    /// or its names are not `pr-fast` → `test-build` and `pre-push` → `tests`.
    #[test]
    fn consumer_names_round_trips() {
        let toml_text = r#"
[[gate]]
id        = "test-build"
label     = "x"
command   = ["true"]
tiers     = ["pre-push"]
gate_when = "code_changed"
tool      = "cargo-nextest"
bucket    = "seq"
consumer_names = { "pre-push" = "tests", "pr-fast" = "test-build" }
"#;
        let manifest: Manifest = toml::from_str(toml_text).unwrap();
        manifest.validate().unwrap();
        let [gate] = manifest.gate.as_slice() else {
            panic!("one gate in the fixture");
        };
        let names: Vec<(&str, &str)> = gate
            .consumer_names
            .iter()
            .map(|(tier, name)| (tier.as_str(), name.as_str()))
            .collect();
        assert_eq!(names, [("pr-fast", "test-build"), ("pre-push", "tests")]);
    }

    /// A gate only in the `pr-fast` tier may leave out `bucket`.
    ///
    /// # Panics
    ///
    /// When the manifest does not parse, the validator refuses it, it holds
    /// other than one gate, or that gate has a bucket.
    #[test]
    fn pr_fast_only_gate_may_omit_bucket() {
        // Mirrors the real `tests` gate (the full nextest run): it
        // is too slow for pre-push and lives only in the pr-fast
        // tier, where `bucket` is meaningless.  The validator must
        // accept this.
        let toml_text = r#"
[[gate]]
id        = "tests"
label     = "full nextest"
command   = ["cargo", "nextest", "run"]
tiers     = ["pr-fast"]
gate_when = "code_changed"
tool      = "cargo-nextest"
"#;
        let manifest: Manifest = toml::from_str(toml_text).unwrap();
        manifest.validate().unwrap();
        let [gate] = manifest.gate.as_slice() else {
            panic!("one gate in the fixture");
        };
        assert!(gate.bucket.is_none(), "the pr-fast-only gate has no bucket");
    }

    /// A gate in the `pre-push` tier without `bucket` is refused.
    ///
    /// # Panics
    ///
    /// When the manifest does not parse, the validator accepts it, or its error
    /// does not say the gate has no `bucket` field.
    #[test]
    fn pre_push_gate_without_bucket_is_rejected() {
        // Counterpart to the above: if a gate claims the pre-push
        // tier, `bucket` becomes mandatory because emit.rs needs to
        // know whether to spawn_bg or run_seq.
        let toml_text = r#"
[[gate]]
id        = "ghost"
label     = "x"
command   = ["true"]
tiers     = ["pre-push"]
gate_when = "always"
tool      = "bash"
"#;
        let manifest: Manifest = toml::from_str(toml_text).unwrap();
        let err = manifest.validate().unwrap_err();
        assert!(
            err.to_string().contains("no `bucket` field"),
            "expected bucket-required error, got: {err}"
        );
    }

    /// A `gate_when` outside the five known ones is refused.
    ///
    /// # Panics
    ///
    /// When the manifest with `gate_when = "sometimes"` does not parse, is
    /// accepted, or its error does not say `unknown gate_when`.
    #[test]
    fn unknown_gate_when_is_rejected() {
        let bad = fixture().replace("gate_when = \"always\"", "gate_when = \"sometimes\"");
        let manifest: Manifest = toml::from_str(&bad).unwrap();
        let err = manifest.validate().unwrap_err();
        assert!(err.to_string().contains("unknown gate_when"));
    }

    /// A tier outside the four known ones is refused.
    ///
    /// # Panics
    ///
    /// When the manifest with the tier `banana` does not parse, is accepted, or
    /// its error does not say `unknown tier`.
    #[test]
    fn unknown_tier_is_rejected() {
        let bad = fixture().replace("tiers     = [\"pre-push\"]", "tiers     = [\"banana\"]");
        let manifest: Manifest = toml::from_str(&bad).unwrap();
        let err = manifest.validate().unwrap_err();
        assert!(err.to_string().contains("unknown tier"));
    }
}
