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
/// `md2html-listed` in every escape hatch, so the suppression
/// mechanism stays covered even though the production allow-lists
/// in [`KnownExceptions::new`] start empty in a fresh project.
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

#[test]
fn missing_lints_block_fires_3_7() {
    let text = MEMBER_CLEAN.replace("[lints]\nworkspace = true\n", "");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_lints_inherit_workspace(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.7"]);
}

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

#[test]
fn profile_block_fires_3_8() {
    let text = format!("{MEMBER_CLEAN}\n[profile.release]\nopt-level = 3\n");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_no_workspace_policy_blocks(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.8"]);
}

#[test]
fn non_workspace_dep_fires_3_6() {
    let text = MEMBER_CLEAN.replace("anyhow = { workspace = true }", r#"anyhow = "1.0.0""#);
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let exceptions = KnownExceptions::new();
    let findings = audit_deps_inherit_workspace(&member, "md2html-core", &exceptions);
    assert_eq!(invariants(&findings), ["3.6"]);
}

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

#[test]
fn missing_docs_rs_metadata_fires_3_15() {
    let text = MEMBER_CLEAN.replace("[package.metadata.docs.rs]\nall-features = true\n", "");
    let manifest = parse_member(&text).unwrap();
    let member = disc("crates/md2html-core/Cargo.toml", &manifest);
    let findings = audit_docs_rs_metadata_present(&member, "md2html-core");
    assert_eq!(invariants(&findings), ["3.15"]);
}

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
