// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Tests of the dependency rules (invariants 3.2 and 3.6) across every
//! dependency table, and of the one spelling of an inherited dependency.

use super::{MEMBER_CLEAN, NONE, disc, invariants};
use crate::audit::member::{
    audit_dep_spelling, audit_deps_inherit_workspace, audit_no_path_dep_outside_workspace,
};
use crate::audit::{Finding, KnownExceptions};
use crate::manifest::{parse_member, parse_root};

/// The clean fixture with `table` appended, holding `entry`.
fn with_table(table: &str, entry: &str) -> String {
    format!("{MEMBER_CLEAN}\n[{table}]\n{entry}\n")
}

/// The 3.6 findings for the fixture with `entry` in `table`.
///
/// # Panics
///
/// When the fixture does not parse.
fn inheritance_findings(table: &str, entry: &str) -> Vec<Finding> {
    let manifest = parse_member(&with_table(table, entry)).unwrap();
    let member = disc("crates/md2html-version/Cargo.toml", &manifest);
    audit_deps_inherit_workspace(&member, "md2html-version", &KnownExceptions::new())
}

/// A version written in any dependency table breaks invariant 3.6, and the
/// finding names the table.
///
/// # Panics
///
/// When a fixture does not parse, or a version in `[build-dependencies]`,
/// `[dev-dependencies]` or a target's tables does not give exactly one
/// finding of invariant 3.6 naming that table.
#[test]
fn a_version_in_any_dependency_table_fires_3_6() {
    for table in [
        "build-dependencies",
        "dev-dependencies",
        "target.'cfg(windows)'.dependencies",
        "target.'cfg(unix)'.build-dependencies",
        "target.x86_64-pc-windows-msvc.dev-dependencies",
    ] {
        let findings = inheritance_findings(table, r#"regex = "1""#);
        assert_eq!(invariants(&findings), ["3.6"], "[{table}]: {findings:?}");
        assert!(
            findings
                .iter()
                .all(|finding| finding.detail.starts_with(&format!("[{table}]"))),
            "the finding names [{table}]: {findings:?}"
        );
    }
}

/// An inherited dependency passes in every dependency table, with or without
/// added features.
///
/// # Panics
///
/// When a fixture does not parse, or an inherited entry gives a finding.
#[test]
fn an_inherited_dependency_passes_in_every_table_3_6() {
    for table in [
        "build-dependencies",
        "dev-dependencies",
        "target.'cfg(windows)'.dev-dependencies",
    ] {
        for entry in [
            "regex.workspace = true",
            r#"md2html-version = { workspace = true, features = ["build"] }"#,
        ] {
            let findings = inheritance_findings(table, entry);
            assert_eq!(invariants(&findings), NONE, "[{table}] {entry}");
        }
    }
}

/// A path dependency outside the members breaks invariant 3.2 in a
/// `[dev-dependencies]` table as well.
///
/// # Panics
///
/// When a fixture does not parse, or a path to a folder that is no member
/// does not give exactly one finding of invariant 3.2, or a path to a member
/// gives one.
#[test]
fn a_path_outside_the_members_fires_3_2_in_dev_dependencies() {
    let root = parse_root(
        "[workspace]\nresolver = \"3\"\nmembers = [\"crates/md2html\", \
         \"crates/md2html-version\"]\n",
    )
    .unwrap();
    for (path, expected) in [
        ("../md2html-version", NONE.as_slice()),
        ("../../elsewhere", ["3.2"].as_slice()),
    ] {
        let entry = format!(r#"helper = {{ path = "{path}" }}"#);
        let manifest = parse_member(&with_table("dev-dependencies", &entry)).unwrap();
        let member = disc("crates/md2html/Cargo.toml", &manifest);
        let findings = audit_no_path_dep_outside_workspace(&member, "md2html", &root);
        assert_eq!(invariants(&findings), expected, "{path}: {findings:?}");
    }
}

/// `name = { workspace = true }` in a dependency table breaks invariant 3.6:
/// the workspace spells it `name.workspace = true`.
///
/// # Panics
///
/// When the inline-table spelling, in `[dependencies]`, in
/// `[build-dependencies]` or under a target, does not give exactly one
/// finding of invariant 3.6.
#[test]
fn the_inline_table_spelling_fires_3_6() {
    for table in [
        "dependencies",
        "build-dependencies",
        "target.'cfg(windows)'.dependencies",
    ] {
        let text =
            format!("[package]\nname = \"x\"\n\n[{table}]\nanyhow = {{ workspace = true }}\n");
        let findings = audit_dep_spelling(&text, "md2html-version");
        assert_eq!(invariants(&findings), ["3.6"], "[{table}]: {findings:?}");
    }
}

/// The dotted spelling, a table that adds features, a comment, and the same
/// text outside a dependency table all pass the spelling rule.
///
/// # Panics
///
/// When one of them gives a finding.
#[test]
fn other_spellings_and_places_pass_the_spelling_rule() {
    let text = "\
[package]
name = \"x\"
edition = { workspace = true }

[package.metadata.extra]
anyhow = { workspace = true }

[dependencies]
# anyhow = { workspace = true } was the old spelling
anyhow.workspace = true
clap.workspace = true # inherited
md2html-version = { workspace = true, features = [\"build\"] }

[lints]
workspace = true
";
    let findings = audit_dep_spelling(text, "md2html-version");
    assert_eq!(invariants(&findings), NONE, "{findings:?}");
}
