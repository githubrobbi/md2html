// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Tests of the structural validator: the permissive-set lattice, the
//! four properties against a consistent fixture and its mutations, and
//! the reading of the aggregator table.

use super::*;
use crate::{manifest, workflow};

/// Manifest fixture covering all four classes of `gate_when`:
/// `always`, `code_changed`, `rust_changed`, `dep_changed`.
/// Includes folding: `cargo-check` (code) + `vet` (dep) → sanity.
const MANIFEST_FIXTURE: &str = r#"
[[gate]]
id = "fmt"
label = "fmt"
tiers = ["pr-fast"]
gate_when = "rust_changed"

[[gate]]
id = "file-size"
label = "file-size"
tiers = ["pr-fast"]
gate_when = "always"

[[gate]]
id = "cargo-check"
label = "cargo check"
tiers = ["pr-fast"]
gate_when = "code_changed"
consumer_names = { "pr-fast" = "sanity" }

[[gate]]
id = "vet"
label = "cargo vet"
tiers = ["pr-fast"]
gate_when = "dep_changed"
consumer_names = { "pr-fast" = "sanity" }

[[gate]]
id = "lint-prod"
label = "ultra-strict clippy"
tiers = ["pre-commit", "pre-push"]
gate_when = "rust_changed"
"#;

/// Workflow fixture that's CONSISTENT with the manifest above.
/// Tests mutate this string to provoke each drift class.
const WORKFLOW_FIXTURE: &str = "
name: PR Fast CI
on: pull_request
jobs:
  classify:
    name: Classify changes
    runs-on: ubuntu-22.04
    outputs:
      rust: ${{ steps.x.outputs.rust }}
    steps:
      - run: echo

  fmt:
    name: Format check
    runs-on: ubuntu-22.04
    needs: classify
    if: needs.classify.outputs.rust == 'true'
    steps:
      - run: cargo fmt

  file-size:
    name: File size
    runs-on: ubuntu-22.04
    needs: classify
    steps:
      - run: bash check.sh

  sanity:
    name: Sanity
    runs-on: ubuntu-22.04
    needs: classify
    if: needs.classify.outputs.code == 'true'
    steps:
      - run: cargo check

  required:
    name: PR Fast CI / required
    runs-on: ubuntu-22.04
    if: always()
    needs:
      - classify
      - fmt
      - file-size
      - sanity
    steps:
      - run: |
          declare -A R=(
            [fmt]='${{ needs.fmt.result }}'
            [file-size]='${{ needs.file-size.result }}'
            [sanity]='${{ needs.sanity.result }}'
          )
";

/// Helper - parse both fixtures and run the validator.
/// `unwrap()` is allowed in tests via `clippy.toml`'s
/// `allow-unwrap-in-tests = true`.
///
/// # Panics
///
/// When the manifest text or the workflow text does not parse, or the validator
/// fails to compile its aggregator pattern.
fn validate_fixture(manifest_text: &str, workflow_text: &str) -> Vec<String> {
    let manifest = manifest::parse(manifest_text).unwrap();
    let workflow = workflow::parse(workflow_text).unwrap();
    validate(&manifest, &workflow, workflow_text).unwrap()
}

// ─── Permissive-set lattice unit tests ────────────────────────

/// Each `gate_when` value maps to the change classes it runs on.
///
/// # Panics
///
/// When `always`, `code_changed` or `rust_changed` does not map to every class,
/// to rust, dep and infra, or to rust alone, or an unknown value maps to
/// anything but the empty set.
#[test]
fn permissive_set_from_gate_when() {
    assert_eq!(PermissiveSet::from_gate_when("always"), PermissiveSet::ALL);
    assert_eq!(
        PermissiveSet::from_gate_when("code_changed"),
        PermissiveSet::CODE
    );
    assert_eq!(
        PermissiveSet::from_gate_when("rust_changed"),
        PermissiveSet::RUST
    );
    // Unknown gate_when → empty set (drift will surface in the
    // downstream contains() check).
    assert_eq!(
        PermissiveSet::from_gate_when("does_not_exist"),
        PermissiveSet::default()
    );
}

/// Each recognised `if:` shape maps to the change classes it accepts.
///
/// # Panics
///
/// When no `if:` or `always()` does not accept the always class, an `if:` on
/// the `code` output does not accept rust, dep and infra or does accept always,
/// or one on the `rust` output does not accept rust or does accept dep.
#[test]
fn permissive_set_from_if_expr() {
    assert!(PermissiveSet::from_if_expr(None).contains(PermissiveSet::ALWAYS));
    assert!(PermissiveSet::from_if_expr(Some("always()")).contains(PermissiveSet::ALWAYS));
    let code = PermissiveSet::from_if_expr(Some("needs.classify.outputs.code == 'true'"));
    assert!(code.contains(PermissiveSet::CODE));
    assert!(!code.contains(PermissiveSet::ALWAYS));
    let rust = PermissiveSet::from_if_expr(Some("needs.classify.outputs.rust == 'true'"));
    assert!(rust.contains(PermissiveSet::RUST));
    assert!(!rust.contains(PermissiveSet::DEP));
}

/// A job folding rust and dep gates must accept both classes.
///
/// # Panics
///
/// When the union of `rust_changed` and `dep_changed` lacks rust or dep or
/// holds infra or always, an `if:` on the `code` output does not cover it, or
/// one on the `rust` output does.
#[test]
fn permissive_set_union_handles_mixed_classes() {
    // sanity job folds rust_changed + dep_changed gates → must
    // accept BOTH rust and dep.
    let folded = PermissiveSet::from_gate_when("rust_changed")
        .union(PermissiveSet::from_gate_when("dep_changed"));
    assert!(folded.contains(PermissiveSet::RUST));
    assert!(folded.contains(PermissiveSet::DEP));
    assert!(!folded.contains(PermissiveSet::INFRA));
    assert!(!folded.contains(PermissiveSet::ALWAYS));

    // job's if: code accepts rust + dep + infra → contains folded.
    let job = PermissiveSet::from_if_expr(Some("classify.outputs.code"));
    assert!(job.contains(folded));

    // job's if: rust does NOT contain rust + dep - too narrow.
    let narrow = PermissiveSet::from_if_expr(Some("classify.outputs.rust"));
    assert!(!narrow.contains(folded));
}

// ─── Happy-path: the consistent fixture passes ────────────────

/// The consistent fixtures give no drift.
///
/// # Panics
///
/// When the fixtures do not parse, or the validator reports any issue for them;
/// the message lists the issues.
#[test]
fn consistent_fixture_passes_all_four_properties() {
    let issues = validate_fixture(MANIFEST_FIXTURE, WORKFLOW_FIXTURE);
    assert!(
        issues.is_empty(),
        "expected no drift, got:\n{}",
        issues.join("\n")
    );
}

// ─── Property 1 mutation tests ────────────────────────────────

/// A missing job is reported once for each gate folded into it.
///
/// # Panics
///
/// When removing the `sanity` job does not give exactly two property 1 issues,
/// one naming `cargo-check` and one naming `vet`.
#[test]
fn property1_detects_missing_job() {
    // Remove the `sanity` job - both cargo-check and vet fold
    // into it, so we expect 2 issues (one per gate).
    let workflow = WORKFLOW_FIXTURE.replace(
            "  sanity:\n    name: Sanity\n    runs-on: ubuntu-22.04\n    needs: classify\n    if: needs.classify.outputs.code == 'true'\n    steps:\n      - run: cargo check\n\n",
            "",
        );
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    let p1: Vec<&String> = issues
        .iter()
        .filter(|issue| issue.contains("Property 1"))
        .collect();
    assert_eq!(
        p1.len(),
        2,
        "expected one P1 issue per missing-job gate, got {}: {:?}",
        p1.len(),
        issues
    );
    assert!(p1.iter().any(|issue| issue.contains("`cargo-check`")));
    assert!(p1.iter().any(|issue| issue.contains("`vet`")));
}

// ─── Property 2 mutation tests ────────────────────────────────

/// A predicate narrower than its gates need is reported.
///
/// # Panics
///
/// When narrowing the `if:` of `sanity` from the `code` output to the `rust`
/// output does not give exactly one property 2 issue, or that issue does not
/// name `sanity` and call the predicate too narrow.
#[test]
fn property2_detects_too_narrow_predicate() {
    // sanity folds {cargo-check (code), vet (dep)} → needs code
    // permissions.  Narrow it to `rust` only - should fail.
    let workflow = WORKFLOW_FIXTURE.replace(
        "    if: needs.classify.outputs.code == 'true'\n    steps:\n      - run: cargo check",
        "    if: needs.classify.outputs.rust == 'true'\n    steps:\n      - run: cargo check",
    );
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    let p2: Vec<&String> = issues
        .iter()
        .filter(|issue| issue.contains("Property 2"))
        .collect();
    let [narrow] = p2.as_slice() else {
        panic!("expected one P2 issue, got: {issues:?}");
    };
    assert!(narrow.contains("`sanity`"));
    assert!(narrow.contains("too narrow"));
}

/// A predicate wider than its gates need passes.
///
/// # Panics
///
/// When widening the `if:` of `fmt` from the `rust` output to the `code` output
/// gives any property 2 issue.
#[test]
fn property2_accepts_wider_predicate() {
    // Widen fmt's `if:` from `rust` to `code` - wider is fine.
    let workflow = WORKFLOW_FIXTURE.replace(
        "    if: needs.classify.outputs.rust == 'true'",
        "    if: needs.classify.outputs.code == 'true'",
    );
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    assert!(
        issues.iter().all(|issue| !issue.contains("Property 2")),
        "wider predicate should pass, got: {issues:?}"
    );
}

// ─── Property 3 mutation tests ────────────────────────────────

/// A job missing from `required.needs:` is reported.
///
/// # Panics
///
/// When removing `sanity` from the `needs:` of the `required` job gives no
/// property 3 issue naming `required.needs:`.
#[test]
fn property3_detects_missing_required_needs() {
    let workflow = WORKFLOW_FIXTURE.replace("      - sanity\n", "");
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("Property 3") && issue.contains("required.needs:")),
        "expected P3 missing-from-required-needs, got: {issues:?}"
    );
}

/// A job missing from the `declare -A R=(...)` table is reported.
///
/// # Panics
///
/// When removing the `sanity` row from the table gives no property 3 issue
/// naming `declare -A R=`.
#[test]
fn property3_detects_missing_aggregator_entry() {
    let workflow =
        WORKFLOW_FIXTURE.replace("            [sanity]='${{ needs.sanity.result }}'\n", "");
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("Property 3") && issue.contains("declare -A R=")),
        "expected P3 missing-from-aggregator, got: {issues:?}"
    );
}

// There is no notify-failure invariant to test: failure notification
// lives outside pr-fast.yml, in ci-failure-notify.yml (workflow_run-
// triggered, after the auto-rerun window), which doesn't carry a
// `needs:` list of every gate.

// ─── Property 4 mutation tests ────────────────────────────────

/// A renamed `required` job is reported.
///
/// # Panics
///
/// When renaming the job `PR Fast CI / Required` does not give exactly one
/// property 4 issue, or that issue does not quote the name branch protection
/// needs.
#[test]
fn property4_detects_renamed_required() {
    let workflow = WORKFLOW_FIXTURE.replace(
        "    name: PR Fast CI / required",
        "    name: PR Fast CI / Required", // capital R = drift
    );
    let issues = validate_fixture(MANIFEST_FIXTURE, &workflow);
    let p4: Vec<&String> = issues
        .iter()
        .filter(|issue| issue.contains("Property 4"))
        .collect();
    let [renamed] = p4.as_slice() else {
        panic!("expected one P4 issue, got: {issues:?}");
    };
    assert!(renamed.contains("PR Fast CI / required"));
}

/// A missing `required` job is reported.
///
/// # Panics
///
/// When the workflow cut short before the `required` job gives no property 4
/// issue saying it is missing.
#[test]
fn property4_detects_missing_required_job() {
    // Remove the entire `required:` job block.  Property 1
    // fires for every gate (since they all fold INTO required's
    // needs check), but Property 4 also fires once for the
    // missing required job itself.
    let workflow = WORKFLOW_FIXTURE.split("  required:").next().unwrap();
    let issues = validate_fixture(MANIFEST_FIXTURE, workflow);
    assert!(
        issues
            .iter()
            .any(|issue| issue.contains("Property 4") && issue.contains("missing")),
        "expected P4 missing-required-job, got: {issues:?}"
    );
}

// ─── Aggregator-extraction unit tests ─────────────────────────

/// The ids of a realistic `R=(...)` table are read.
///
/// # Panics
///
/// When the pattern does not compile, or the ids read are not exactly
/// `file-size`, `gates-drift` and `hooks-drift`.
#[test]
fn extract_aggregator_ids_handles_realistic_table() {
    let yaml = r#"
      - run: |
          declare -A R=(
            [file-size]='${{ needs.file-size.result }}'
            [gates-drift]='${{ needs.gates-drift.result }}'
            [hooks-drift]='${{ needs.hooks-drift.result }}'
          )
          for j in "${!R[@]}"; do
            echo "$j: ${R[$j]}"
          done
"#;
    let ids = extract_aggregator_ids(yaml).unwrap();
    assert_eq!(
        ids,
        ["file-size", "gates-drift", "hooks-drift"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect()
    );
}

/// Substitutions, and brackets outside the table, are not job ids.
///
/// # Panics
///
/// When the pattern does not compile, or anything but `valid-id` is read: a
/// `${{ ... }}` substitution, or the `[other-bracket]` line after the table.
#[test]
fn extract_aggregator_ids_ignores_unrelated_brackets() {
    // `${{ needs.foo.result }}` patterns inside tables shouldn't
    // be matched as job-ids.  Lines outside the table shouldn't
    // be matched at all.
    let yaml = r#"
      - run: |
          echo "${{ matrix.os }}"
          declare -A R=(
            [valid-id]='something'
          )
          echo "${{ steps.x.outputs.y }}"
          [other-bracket]=ignored
"#;
    let ids = extract_aggregator_ids(yaml).unwrap();
    let expected: BTreeSet<String> = core::iter::once("valid-id".to_owned()).collect();
    assert_eq!(ids, expected);
}
