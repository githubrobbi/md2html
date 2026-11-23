// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The per-member invariants: each check reads one member manifest and
//! returns its findings (empty when the member complies).

use alloc::collections::BTreeSet;

use super::{DiscoveredMember, Finding, KnownExceptions};
use crate::manifest::{RootManifest, is_workspace_inherited};

/// **Invariant 3.2**: no `path = "..."` dep points outside the
/// declared `[workspace.members]`.
pub(super) fn audit_no_path_dep_outside_workspace(
    member: &DiscoveredMember<'_>,
    id: &str,
    root: &RootManifest,
) -> Vec<Finding> {
    let Some(ws) = root.workspace.as_ref() else {
        return Vec::new();
    };
    let allowed: BTreeSet<&str> = ws.members.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for (dep_name, dep_value) in &member.manifest.dependencies {
        let Some(table) = dep_value.as_table() else {
            continue;
        };
        let Some(path_str) = table.get("path").and_then(toml::Value::as_str) else {
            continue;
        };
        let resolved = normalise_path_dep(member.manifest_path, path_str);
        if !allowed.contains(resolved.as_str()) {
            findings.push(Finding {
                invariant: "3.2",
                member: id.to_owned(),
                detail: format!(
                    "dep `{dep_name}` uses `path = {path_str:?}` resolving to `{resolved}` - \
                     not in [workspace.members]"
                ),
            });
        }
    }
    findings
}

/// Resolve `../sibling`-style member-relative path against the
/// member's directory.  Returns the path relative to the workspace
/// root (e.g. `"crates/md2html-version"`).
pub(super) fn normalise_path_dep(member_manifest_path: &str, dep_path: &str) -> String {
    let member_dir = member_manifest_path
        .rsplit_once('/')
        .map_or(member_manifest_path, |(parent, _)| parent);
    let mut components: Vec<&str> = member_dir.split('/').collect();
    for segment in dep_path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            other => components.push(other),
        }
    }
    components.join("/")
}

/// Shared 3.3 / 3.5 helper: a finding when `value` is set in the member
/// instead of inherited from the workspace.
pub(super) fn check_inherited(
    value: Option<&toml::Value>,
    field_name: &str,
    invariant: &'static str,
    id: &str,
) -> Vec<Finding> {
    let Some(inner) = value else {
        return Vec::new();
    };
    if is_workspace_inherited(inner) {
        return Vec::new();
    }
    vec![Finding {
        invariant,
        member: id.to_owned(),
        detail: format!(
            "`{field_name}` is overridden per-crate; should be `{field_name}.workspace = true`"
        ),
    }]
}

/// **Invariant 3.5**: every fingerprint metadata field except
/// `description` must inherit from workspace, with three narrowly
/// scoped exceptions:
///
/// * `readme` - see [`KnownExceptions::readme_override_ok`].
/// * `keywords` - see [`KnownExceptions::keywords_override_ok`].
/// * `categories` - see [`KnownExceptions::categories_override_ok`].
///
/// All three are for a crate published on its own (`lane:crates-publish`),
/// which describes itself rather than md2html.
pub(super) fn audit_metadata_fields(
    member: &DiscoveredMember<'_>,
    id: &str,
    exc: &KnownExceptions,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (name, value) in [
        ("version", member.manifest.package.version.as_ref()),
        ("license", member.manifest.package.license.as_ref()),
        ("authors", member.manifest.package.authors.as_ref()),
        ("repository", member.manifest.package.repository.as_ref()),
        ("readme", member.manifest.package.readme.as_ref()),
        ("keywords", member.manifest.package.keywords.as_ref()),
        ("categories", member.manifest.package.categories.as_ref()),
        (
            "documentation",
            member.manifest.package.documentation.as_ref(),
        ),
    ] {
        // `readme` override: tightened to the literal `"README.md"`
        // same-name pattern so a listed crate can't accidentally
        // point at an arbitrary path.  Any other override pattern
        // still fires a finding even for listed crates.
        if name == "readme"
            && exc.readme_override_ok.contains(id)
            && value.and_then(toml::Value::as_str) == Some("README.md")
        {
            continue;
        }
        // `keywords` / `categories` overrides: tightened to non-empty
        // TOML arrays.  An empty array still fires (it says nothing).
        // The shape of each keyword and the validity of each category
        // are crates.io's to check, at `cargo publish --dry-run`
        // (`crates-io-dry-run.yml`); checking them here as well would be
        // a second copy of crates.io's rules.
        if (name == "keywords" && exc.keywords_override_ok.contains(id)
            || name == "categories" && exc.categories_override_ok.contains(id))
            && is_non_empty_array(value)
        {
            continue;
        }
        findings.extend(check_inherited(value, name, "3.5", id));
    }
    findings
}

/// True when `value` is a non-empty TOML array.
///
/// Used by the
/// `keywords` / `categories` override allow-list to ensure listed
/// crates can't accidentally set `keywords = []` (which would defeat
/// the override's discoverability purpose).
pub(super) fn is_non_empty_array(value: Option<&toml::Value>) -> bool {
    value
        .and_then(toml::Value::as_array)
        .is_some_and(|array| !array.is_empty())
}

/// **Invariant 3.6**: every dep should use workspace inheritance,
/// modulo the documented exception list.
pub(super) fn audit_deps_inherit_workspace(
    member: &DiscoveredMember<'_>,
    id: &str,
    exc: &KnownExceptions,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (dep_name, dep_value) in &member.manifest.dependencies {
        if dep_inherits_workspace(dep_value) {
            continue;
        }
        if exc.nonworkspace_dep_ok.contains(&(id, dep_name.as_str())) {
            continue;
        }
        findings.push(Finding {
            invariant: "3.6",
            member: id.to_owned(),
            detail: format!(
                "dep `{dep_name}` is not workspace-inherited and not in the documented \
                 exception list; declare it via `{dep_name}.workspace = true` and add the \
                 corresponding entry under `[workspace.dependencies]` in root Cargo.toml"
            ),
        });
    }
    findings
}

/// `dep.workspace = true` or `dep = { workspace = true, ... }`?
fn dep_inherits_workspace(value: &toml::Value) -> bool {
    let Some(table) = value.as_table() else {
        return false;
    };
    matches!(table.get("workspace"), Some(toml::Value::Boolean(true)))
}

/// **Invariant 3.7**: every member has `[lints] workspace = true`.
pub(super) fn audit_lints_inherit_workspace(
    member: &DiscoveredMember<'_>,
    id: &str,
) -> Vec<Finding> {
    let Some(lints) = member.manifest.lints.as_ref() else {
        return vec![Finding {
            invariant: "3.7",
            member: id.to_owned(),
            detail: "no [lints] block; should be `[lints] workspace = true`".to_owned(),
        }];
    };
    match lints.get("workspace") {
        Some(toml::Value::Boolean(true)) => Vec::new(),
        _ => vec![Finding {
            invariant: "3.7",
            member: id.to_owned(),
            detail: "[lints] block does not set `workspace = true`".to_owned(),
        }],
    }
}

/// **Invariant 3.8**: members must not declare workspace-policy
/// blocks (`[profile.*]`, `[patch.*]`, `[workspace.*]`).
pub(super) fn audit_no_workspace_policy_blocks(
    member: &DiscoveredMember<'_>,
    id: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if member.manifest.profile.is_some() {
        findings.push(Finding {
            invariant: "3.8",
            member: id.to_owned(),
            detail: "member declares `[profile.*]`; profile policy lives at the workspace root"
                .to_owned(),
        });
    }
    if member.manifest.patch.is_some() {
        findings.push(Finding {
            invariant: "3.8",
            member: id.to_owned(),
            detail: "member declares `[patch.*]`; patch policy lives at the workspace root"
                .to_owned(),
        });
    }
    if member.manifest.workspace.is_some() {
        findings.push(Finding {
            invariant: "3.8",
            member: id.to_owned(),
            detail: "member declares `[workspace]`; only root may declare `[workspace]`".to_owned(),
        });
    }
    findings
}

/// **Invariant 3.13**: lib name uses `_`, bin name uses `-` (unless the
/// member is in [`KnownExceptions::underscore_bin_ok`]).
pub(super) fn audit_lib_bin_name_convention(
    member: &DiscoveredMember<'_>,
    id: &str,
    exc: &KnownExceptions,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(lib) = member.manifest.lib.as_ref()
        && let Some(name) = lib.name.as_deref()
        && name.contains('-')
    {
        findings.push(Finding {
            invariant: "3.13",
            member: id.to_owned(),
            detail: format!("`[lib].name = {name:?}` contains `-`; use underscores"),
        });
    }
    for bin in &member.manifest.bin {
        let Some(name) = bin.name.as_deref() else {
            continue;
        };
        if name.contains('_') && !exc.underscore_bin_ok.contains(id) {
            findings.push(Finding {
                invariant: "3.13",
                member: id.to_owned(),
                detail: format!(
                    "`[[bin]].name = {name:?}` contains `_`; bin names use hyphens.  If this \
                     is intentional, add `{id}` to `KnownExceptions::underscore_bin_ok` with a \
                     citation."
                ),
            });
        }
    }
    findings
}

/// **Invariant 3.14**: `publish` is `publish.workspace = true`, or an
/// explicit `publish = true` for a member in
/// [`KnownExceptions::publish_explicit`].
pub(super) fn audit_publish_partition(
    member: &DiscoveredMember<'_>,
    id: &str,
    exc: &KnownExceptions,
) -> Vec<Finding> {
    let Some(publish) = member.manifest.package.publish.as_ref() else {
        return vec![Finding {
            invariant: "3.14",
            member: id.to_owned(),
            detail: "`publish` field is absent; must be `publish.workspace = true` (or \
                     `publish = true` for a crate listed as published)"
                .to_owned(),
        }];
    };
    if is_workspace_inherited(publish) {
        return Vec::new();
    }
    if matches!(publish, toml::Value::Boolean(true)) && exc.publish_explicit.contains(id) {
        return Vec::new();
    }
    if matches!(publish, toml::Value::Boolean(false)) {
        return vec![Finding {
            invariant: "3.14",
            member: id.to_owned(),
            detail: "`publish = false` is redundant with workspace default; should be \
                     `publish.workspace = true`"
                .to_owned(),
        }];
    }
    vec![Finding {
        invariant: "3.14",
        member: id.to_owned(),
        detail: format!(
            "`publish` is overridden per-crate ({publish:?}) but `{id}` is not listed as \
             published (`KnownExceptions::publish_explicit`, lane:crates-publish in \
             docs/forge/COMPONENTS.md)"
        ),
    }]
}

/// **Invariant 3.15**: every member has a `[package.metadata.docs.rs]`
/// block.
pub(super) fn audit_docs_rs_metadata_present(
    member: &DiscoveredMember<'_>,
    id: &str,
) -> Vec<Finding> {
    if let Some(metadata) = member.manifest.package.metadata.as_ref()
        && metadata
            .get("docs")
            .and_then(|docs| docs.get("rs"))
            .is_some()
    {
        return Vec::new();
    }
    vec![Finding {
        invariant: "3.15",
        member: id.to_owned(),
        detail: "no `[package.metadata.docs.rs]` block; every member carries the same docs.rs \
                 metadata"
            .to_owned(),
    }]
}
