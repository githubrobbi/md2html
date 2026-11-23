// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Tests of the manifest invariants.

use alloc::collections::BTreeSet;

use super::member::{
    audit_deps_inherit_workspace, audit_docs_rs_metadata_present, audit_lib_bin_name_convention,
    audit_lints_inherit_workspace, audit_metadata_fields, audit_no_workspace_policy_blocks,
    audit_publish_partition, check_inherited, is_non_empty_array, normalise_path_dep,
};
use super::{DiscoveredMember, Finding, KnownExceptions, member_id_from_path};
use crate::manifest::{MemberManifest, parse_member};

/// The invariant numbers of `findings`, in order.
fn invariants(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|finding| finding.invariant).collect()
}

/// The invariant numbers of the findings whose detail names `field`.
fn invariants_naming(findings: &[Finding], field: &str) -> Vec<&'static str> {
    findings
        .iter()
        .filter(|finding| finding.detail.contains(field))
        .map(|finding| finding.invariant)
        .collect()
}

/// Test-local exception set that lists the fictional crate
/// `md2html-listed` in every escape hatch.
///
/// That keeps the suppression mechanism covered even though the production
/// allow-lists in [`KnownExceptions::new`] start empty in a fresh project.
fn listed_exceptions() -> KnownExceptions {
    let listed: BTreeSet<&'static str> = core::iter::once("md2html-listed").collect();
    KnownExceptions {
        publish_explicit: listed.clone(),
        underscore_bin_ok: listed.clone(),
        nonworkspace_dep_ok: core::iter::once(("md2html-listed", "special-sys")).collect(),
        readme_override_ok: listed.clone(),
        keywords_override_ok: listed.clone(),
        categories_override_ok: listed,
    }
}

/// A member manifest that satisfies every per-member invariant.
const MEMBER_CLEAN: &str = r#"
[package]
name = "md2html-core"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
readme.workspace = true
keywords.workspace = true
categories.workspace = true
documentation.workspace = true
publish.workspace = true
description = "per-crate description"

[package.metadata.docs.rs]
all-features = true

[dependencies]
anyhow = { workspace = true }
serde.workspace = true

[lints]
workspace = true
"#;

/// The member at `path` with the parsed `manifest`.
fn disc<'a>(path: &'a str, manifest: &'a MemberManifest) -> DiscoveredMember<'a> {
    DiscoveredMember {
        manifest_path: path,
        manifest,
    }
}

/// A member that meets every invariant passes every per-member check.
///
/// # Panics
///
/// When the clean fixture does not parse, or one of the six checks (metadata
/// fields, dependencies, lints, policy blocks, publish partition, docs.rs
/// metadata) reports a finding; the message names the check and its findings.
#[test]
fn clean_member_passes_every_per_member_check() {
    let manifest = parse_member(MEMBER_CLEAN).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    // Each check NAMED, and its findings printed on failure: a bare
    // `assert!(x.is_empty())` here reported only that one of six
    // checks fired, never which or why.
    for (check, findings) in [
        (
            "metadata fields",
            audit_metadata_fields(&member, "md2html-core", &exceptions),
        ),
        (
            "deps inherit workspace",
            audit_deps_inherit_workspace(&member, "md2html-core", &exceptions),
        ),
        (
            "lints inherit workspace",
            audit_lints_inherit_workspace(&member, "md2html-core"),
        ),
        (
            "no workspace policy blocks",
            audit_no_workspace_policy_blocks(&member, "md2html-core"),
        ),
        (
            "publish partition",
            audit_publish_partition(&member, "md2html-core", &exceptions),
        ),
        (
            "docs.rs metadata present",
            audit_docs_rs_metadata_present(&member, "md2html-core"),
        ),
    ] {
        assert!(
            findings.is_empty(),
            "{check} fired on a clean member: {findings:?}"
        );
    }
}

/// A member without `[lints] workspace = true` breaks invariant 3.7.
///
/// # Panics
///
/// When the edited fixture does not parse, or the lints check does not give
/// exactly one finding, of invariant 3.7.
#[test]
fn missing_lints_block_fires_3_7() {
    let text = MEMBER_CLEAN.replace("[lints]\nworkspace = true\n", "");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_lints_inherit_workspace(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.7"]);
}

/// An edition set in the member itself breaks invariant 3.3.
///
/// # Panics
///
/// When the edited fixture does not parse, or `edition = "2024"` does not give
/// exactly one finding, of invariant 3.3.
#[test]
fn edition_override_fires_3_3() {
    let text = MEMBER_CLEAN.replace("edition.workspace = true", "edition = \"2024\"");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = check_inherited(
        member.manifest.package.edition.as_ref(),
        "edition",
        "3.3",
        "md2html-core",
    );
    assert_eq!(invariants(&findings), ["3.3"]);
}

/// A `[profile]` table in a member breaks invariant 3.8.
///
/// # Panics
///
/// When the edited fixture does not parse, or `[profile.release]` does not give
/// exactly one finding, of invariant 3.8.
#[test]
fn profile_block_fires_3_8() {
    let text = format!("{MEMBER_CLEAN}\n[profile.release]\nopt-level = 3\n");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_no_workspace_policy_blocks(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.8"]);
}

/// A dependency with its own version breaks invariant 3.6.
///
/// # Panics
///
/// When the edited fixture does not parse, or `anyhow = "1.0.0"` does not give
/// exactly one finding, of invariant 3.6.
#[test]
fn non_workspace_dep_fires_3_6() {
    let text = MEMBER_CLEAN.replace("anyhow = { workspace = true }", r#"anyhow = "1.0.0""#);
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    let findings = audit_deps_inherit_workspace(&member, "md2html-core", &exceptions);
    assert_eq!(invariants(&findings), ["3.6"]);
}

/// A dependency on the exception list is not reported.
///
/// # Panics
///
/// When the edited fixture does not parse, or `special-sys` with its own
/// version in `md2html-listed`, which the exception list allows, gives any
/// finding.
#[test]
fn known_exception_suppresses_3_6() {
    let text = MEMBER_CLEAN.replace(
        "anyhow = { workspace = true }",
        r#"special-sys = { version = "0.1.44", features = ["extended"] }"#,
    );
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_deps_inherit_workspace(&member, "md2html-listed", &exceptions);
    assert!(
        findings.is_empty(),
        "exception should suppress; got: {findings:?}"
    );
}

/// A member without `[package.metadata.docs.rs]` breaks invariant 3.15.
///
/// # Panics
///
/// When the edited fixture does not parse, or the docs.rs check does not give
/// exactly one finding, of invariant 3.15.
#[test]
fn missing_docs_rs_metadata_fires_3_15() {
    let text = MEMBER_CLEAN.replace("[package.metadata.docs.rs]\nall-features = true\n", "");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_docs_rs_metadata_present(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.15"]);
}

/// An underscore in a binary's name breaks invariant 3.13 unless the crate is
/// listed.
///
/// # Panics
///
/// When the fixture with a `[[bin]]` named `foo_bar` does not parse, does not
/// give exactly one finding of invariant 3.13 for `md2html-core`, or gives any
/// finding for the listed `md2html-listed`.
#[test]
fn underscore_bin_name_fires_3_13_except_diag() {
    let text = format!("{MEMBER_CLEAN}\n[[bin]]\nname = \"foo_bar\"\npath = \"src/main.rs\"\n");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    let findings = audit_lib_bin_name_convention(&member, "md2html-core", &exceptions);
    assert_eq!(invariants(&findings), ["3.13"]);

    // Same shape under an allow-listed crate - should be suppressed.
    let listed = listed_exceptions();
    let d_listed = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let suppressed = audit_lib_bin_name_convention(&d_listed, "md2html-listed", &listed);
    assert!(
        suppressed.is_empty(),
        "allow-listed crate should suppress: {suppressed:?}"
    );
}

/// A path dependency resolves to its folder from the workspace root.
///
/// # Panics
///
/// When `../md2html-client` from `crates/md2html-cli` does not resolve to
/// `crates/md2html-client`, or `../../../crates/md2html-core` from
/// `scripts/ci/md2html-gen-hooks` does not resolve to `crates/md2html-core`.
#[test]
fn normalise_path_dep_resolves_relative() {
    assert_eq!(
        normalise_path_dep("crates/md2html-cli/Cargo.toml", "../md2html-client"),
        "crates/md2html-client"
    );
    assert_eq!(
        normalise_path_dep(
            "scripts/ci/md2html-gen-hooks/Cargo.toml",
            "../../../crates/md2html-core"
        ),
        "crates/md2html-core"
    );
}

// ─────────────────────────────────────────────────────────────
// Invariant 3.5 - readme override allow-list
// ─────────────────────────────────────────────────────────────

/// A listed crate may set `readme = "README.md"`.
///
/// # Panics
///
/// When the edited fixture does not parse, or `readme = "README.md"` in
/// `md2html-listed` gives a finding naming `readme`.
#[test]
fn readme_override_on_allowlisted_crate_is_suppressed() {
    // A crate on the allow-list using the conventional
    // `readme = "README.md"` same-name pattern must not fire.
    let text = MEMBER_CLEAN.replace("readme.workspace = true", "readme = \"README.md\"");
    let manifest = parse_member(&text).unwrap();
    let exceptions = listed_exceptions();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert!(
        !findings
            .iter()
            .any(|finding| finding.detail.contains("`readme`")),
        "allow-listed readme override should be suppressed; got: {findings:?}"
    );
}

/// An unlisted crate setting `readme` breaks invariant 3.5.
///
/// # Panics
///
/// When the edited fixture does not parse, or `readme = "README.md"` in
/// `md2html-core` does not give exactly one finding naming `readme`, of
/// invariant 3.5.
#[test]
fn readme_override_on_unlisted_crate_still_fires_3_5() {
    // `md2html-core` is NOT on the allow-list - override must fire.
    let text = MEMBER_CLEAN.replace("readme.workspace = true", "readme = \"README.md\"");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    let findings = audit_metadata_fields(&member, "md2html-core", &exceptions);
    assert_eq!(
        invariants_naming(&findings, "`readme`"),
        ["3.5"],
        "unlisted crate readme override should fire 3.5; got: {findings:?}"
    );
}

/// A listed crate pointing `readme` elsewhere than `README.md` breaks invariant
/// 3.5.
///
/// # Panics
///
/// When the edited fixture does not parse, or `readme = "docs/intro.md"` in
/// `md2html-listed` does not give exactly one finding naming `readme`, of
/// invariant 3.5.
#[test]
fn readme_override_to_non_readme_path_still_fires_even_for_listed_crate() {
    // `md2html-listed` is on the allow-list, but the exception is
    // tightened to the literal `"README.md"`.  An attempt to point
    // at a different path (e.g. `"docs/intro.md"`) must still fire.
    let text = MEMBER_CLEAN.replace("readme.workspace = true", "readme = \"docs/intro.md\"");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert_eq!(
        invariants_naming(&findings, "`readme`"),
        ["3.5"],
        "non-README.md readme override on listed crate must still fire 3.5; got: {findings:?}"
    );
}

/// An inherited `readme` passes for every member.
///
/// # Panics
///
/// When the clean fixture does not parse, or `readme.workspace = true` gives a
/// finding naming `readme` for one of the four member ids tried.
#[test]
fn readme_workspace_inherit_passes_for_every_member() {
    // The default state (workspace inheritance) must pass for both
    // allow-listed and non-allow-listed crates.
    let manifest = parse_member(MEMBER_CLEAN).unwrap();
    let exceptions = KnownExceptions::new();
    for id in [
        "md2html-core",
        "md2html-time",
        "md2html-text",
        "md2html-broker",
    ] {
        let path = format!("crates/{id}/Cargo.toml");
        let member = disc(&path, &manifest);
        let findings = audit_metadata_fields(&member, id, &exceptions);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.detail.contains("`readme`")),
            "`readme.workspace = true` must not fire for `{id}`; got: {findings:?}"
        );
    }
}

/// A member's id is the name of its folder.
///
/// # Panics
///
/// When `crates/md2html-core/Cargo.toml` does not give `md2html-core`, or
/// `scripts/ci/md2html-gen-hooks/Cargo.toml` does not give `md2html-gen-hooks`.
#[test]
fn member_id_from_path_strips_prefix_and_filename() {
    assert_eq!(
        member_id_from_path("crates/md2html-core/Cargo.toml"),
        "md2html-core"
    );
    assert_eq!(
        member_id_from_path("scripts/ci/md2html-gen-hooks/Cargo.toml"),
        "md2html-gen-hooks"
    );
}

// ─────────────────────────────────────────────────────────────
// Invariant 3.5 - keywords / categories override allow-list
// ─────────────────────────────────────────────────────────────

/// A listed crate may set its own non-empty `keywords`.
///
/// # Panics
///
/// When the edited fixture does not parse, or three keywords set in
/// `md2html-listed` give a finding naming `keywords`.
#[test]
fn keywords_override_on_allowlisted_crate_is_suppressed() {
    // A crate on the keywords allow-list using a well-formed
    // non-empty array must not fire.
    let text = MEMBER_CLEAN.replace(
        "keywords.workspace = true",
        r#"keywords = ["ntfs", "filetime", "datetime"]"#,
    );
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert!(
        !findings
            .iter()
            .any(|finding| finding.detail.contains("`keywords`")),
        "allow-listed keywords override should be suppressed; got: {findings:?}"
    );
}

/// A listed crate may set its own non-empty `categories`.
///
/// # Panics
///
/// When the edited fixture does not parse, or `categories = ["date-and-time"]`
/// in `md2html-listed` gives a finding naming `categories`.
#[test]
fn categories_override_on_allowlisted_crate_is_suppressed() {
    let text = MEMBER_CLEAN.replace(
        "categories.workspace = true",
        r#"categories = ["date-and-time"]"#,
    );
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert!(
        !findings
            .iter()
            .any(|finding| finding.detail.contains("`categories`")),
        "allow-listed categories override should be suppressed; got: {findings:?}"
    );
}

/// An unlisted crate setting `keywords` breaks invariant 3.5.
///
/// # Panics
///
/// When the edited fixture does not parse, or a well-formed `keywords` array in
/// `md2html-core` does not give exactly one finding naming `keywords`, of
/// invariant 3.5.
#[test]
fn keywords_override_on_unlisted_crate_still_fires_3_5() {
    // `md2html-core` is NOT on the keywords allow-list - override
    // must fire even with a well-formed array.
    let text = MEMBER_CLEAN.replace("keywords.workspace = true", r#"keywords = ["something"]"#);
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    let findings = audit_metadata_fields(&member, "md2html-core", &exceptions);
    assert_eq!(
        invariants_naming(&findings, "`keywords`"),
        ["3.5"],
        "unlisted crate keywords override must fire 3.5; got: {findings:?}"
    );
}

/// An empty `keywords` array breaks invariant 3.5 even in a listed crate.
///
/// # Panics
///
/// When the edited fixture does not parse, or `keywords = []` in
/// `md2html-listed` does not give exactly one finding naming `keywords`, of
/// invariant 3.5.
#[test]
fn empty_keywords_array_on_allowlisted_crate_still_fires_3_5() {
    // `md2html-listed` is on the allow-list but the override is
    // tightened to **non-empty** arrays.  `keywords = []` is
    // legal TOML but defeats the discoverability purpose of an
    // override; it must still fire.
    let text = MEMBER_CLEAN.replace("keywords.workspace = true", "keywords = []");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert_eq!(
        invariants_naming(&findings, "`keywords`"),
        ["3.5"],
        "empty keywords array on listed crate must still fire 3.5; got: {findings:?}"
    );
}

/// An empty `categories` array breaks invariant 3.5 even in a listed crate.
///
/// # Panics
///
/// When the edited fixture does not parse, or `categories = []` in
/// `md2html-listed` does not give exactly one finding naming `categories`, of
/// invariant 3.5.
#[test]
fn empty_categories_array_on_allowlisted_crate_still_fires_3_5() {
    let text = MEMBER_CLEAN.replace("categories.workspace = true", "categories = []");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-listed/Cargo.toml", &manifest);
    let exceptions = listed_exceptions();
    let findings = audit_metadata_fields(&member, "md2html-listed", &exceptions);
    assert_eq!(
        invariants_naming(&findings, "`categories`"),
        ["3.5"],
        "empty categories array on listed crate must still fire 3.5; got: {findings:?}"
    );
}

/// Inherited `keywords` and `categories` pass for every member.
///
/// # Panics
///
/// When the clean fixture does not parse, or the inherited fields give a
/// finding naming `keywords` or `categories` for one of the eight member ids
/// tried.
#[test]
fn keywords_and_categories_workspace_inherit_passes_for_every_member() {
    // The default state (workspace inheritance) must pass for
    // both allow-listed and non-allow-listed crates.
    let manifest = parse_member(MEMBER_CLEAN).unwrap();
    let exceptions = KnownExceptions::new();
    for id in [
        "md2html-core",
        "md2html-time",
        "md2html-text",
        "md2html-client",
        "md2html-mcp",
        "md2html-mft",
        "md2html-cli",
        "md2html-broker",
    ] {
        let path = format!("crates/{id}/Cargo.toml");
        let member = disc(&path, &manifest);
        let findings = audit_metadata_fields(&member, id, &exceptions);
        assert!(
            !findings
                .iter()
                .any(|finding| finding.detail.contains("`keywords`")
                    || finding.detail.contains("`categories`")),
            "`keywords.workspace = true` / `categories.workspace = true` must not fire for \
             `{id}`; got: {findings:?}"
        );
    }
}

/// Only an array with an element counts as a non-empty array.
///
/// # Panics
///
/// When an empty array, no value or the `{ workspace = true }` table counts as
/// a non-empty array, or a one-element array does not.
#[test]
fn is_non_empty_array_distinguishes_shapes() {
    // Empty array → false.
    let empty = toml::Value::Array(Vec::new());
    assert!(!is_non_empty_array(Some(&empty)));
    // Non-empty array → true.
    let one = toml::Value::Array(vec![toml::Value::String("a".to_owned())]);
    assert!(is_non_empty_array(Some(&one)));
    // None → false.
    assert!(!is_non_empty_array(None));
    // Workspace-inheritance shape (table with `workspace = true`) → false.
    let mut table = toml::value::Table::new();
    table.insert("workspace".to_owned(), toml::Value::Boolean(true));
    let inherit_value = toml::Value::Table(table);
    assert!(!is_non_empty_array(Some(&inherit_value)));
}
