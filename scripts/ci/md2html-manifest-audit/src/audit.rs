// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The workspace's manifest rules, each a check that returns its findings.
//!
//! This list is the contract the gate `manifest-drift` holds; a finding names
//! its rule by number.  (3.12, the grouping of `[workspace.dependencies]`, is
//! read by people, not checked here.)
//!
//! | # | Rule |
//! |---|---|
//! | 3.1 | `[workspace.members]` lists exactly the folders below the root that hold a `Cargo.toml` (a nested workspace aside) |
//! | 3.2 | no path dependency points outside the members |
//! | 3.3 | `edition.workspace = true` |
//! | 3.4 | `rust-version`: md2html (`crates/md2html`) declares its own; no other member and not `[workspace.package]` (the root `Cargo.toml` says why) |
//! | 3.5 | `version`, `license`, `authors`, `repository`, `readme`, `keywords`, `categories` and `documentation` are inherited; `description` is the crate's own |
//! | 3.6 | every `[dependencies]` entry inherits its version from `[workspace.dependencies]` |
//! | 3.7 | `[lints] workspace = true` |
//! | 3.8 | no `[profile]`, `[patch]` or `[workspace]` in a member |
//! | 3.9 | the root is a virtual workspace (no `[package]`) |
//! | 3.10 | `resolver = "3"` |
//! | 3.11 | `default-members` is set, or the root says why not |
//! | 3.13 | a `[lib]` name uses `_`, a `[[bin]]` name `-` |
//! | 3.14 | `publish.workspace = true` |
//! | 3.15 | a `[package.metadata.docs.rs]` table |
//!
//! [`audit_all`] runs every rule and concatenates the findings; none means
//! the workspace passes.
//!
//! # Design rationale
//!
//! * Findings are **plain text** (no JSON, no enum taxonomy) because the audit
//!   only fires at PR time and a human is always the next reader.
//! * A deviation is allowed only through [`KnownExceptions`], which is empty:
//!   its lists exist for `lane:crates-publish` (`docs/forge/COMPONENTS.md`), a
//!   crate published with its own README, keywords and categories.

mod member;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeSet;

use member::{
    audit_deps_inherit_workspace, audit_docs_rs_metadata_present, audit_lib_bin_name_convention,
    audit_lints_inherit_workspace, audit_metadata_fields, audit_no_path_dep_outside_workspace,
    audit_no_workspace_policy_blocks, audit_publish_partition, audit_rust_version, check_inherited,
};

use crate::manifest::{MemberManifest, RootManifest};

/// The manifest of md2html, the product: the one member that declares a
/// `rust-version` (invariant 3.4; the root `Cargo.toml` says why).
const PRODUCT_MANIFEST: &str = "crates/md2html/Cargo.toml";

/// Single audit finding.  Self-contained diagnostic; the CI log
/// alone tells the contributor what to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// Number of the rule broken (e.g. `"3.7"`; the module docs list them).
    pub(crate) invariant: &'static str,
    /// Member crate id (e.g. `"md2html-version"`) for member-scoped findings,
    /// or `"<root>"` for root-scoped ones.
    pub(crate) member: String,
    /// One-line diagnostic.
    pub(crate) detail: String,
}

impl core::fmt::Display for Finding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}: {}", self.invariant, self.member, self.detail)
    }
}

/// Hardcoded set of intentional manifest deviations.
///
/// Every list is empty: md2html publishes nothing and every member inherits
/// what the workspace decides.  The lists are what `lane:crates-publish`
/// (`docs/forge/COMPONENTS.md`) fills when a crate is published; an entry
/// comes with the reason beside it, and the audit fires on any deviation not
/// listed.
struct KnownExceptions {
    /// Members allowed to set `publish = true` explicitly instead of
    /// inheriting the workspace's `publish = false`: a crate published to
    /// crates.io.
    publish_explicit: BTreeSet<&'static str>,
    /// Members allowed an underscore in a `[[bin]]` name (against the
    /// hyphen convention of invariant 3.13).
    underscore_bin_ok: BTreeSet<&'static str>,
    /// `(member, dep_name)` pairs allowed to write their own version
    /// instead of inheriting it from `[workspace.dependencies]`.
    nonworkspace_dep_ok: BTreeSet<(&'static str, &'static str)>,
    /// Members allowed to set `readme = "README.md"` directly instead of
    /// inheriting the workspace's: a published crate that ships its own
    /// README beside its `Cargo.toml`, so crates.io shows that one rather
    /// than the repository's.  Only the literal `"README.md"` passes, so a
    /// listed crate cannot point at an arbitrary path.
    readme_override_ok: BTreeSet<&'static str>,
    /// Members allowed to set `keywords = [...]` directly: a published
    /// crate whose keywords describe it rather than md2html.  The override
    /// must be a **non-empty** array (`keywords = []` is legal but says
    /// nothing).  The shape of each keyword is crates.io's to check, at
    /// `cargo publish --dry-run` (`crates-io-dry-run.yml`), not this audit's.
    keywords_override_ok: BTreeSet<&'static str>,
    /// Members allowed to set `categories = [...]` directly, on the same
    /// terms as `keywords_override_ok`; crates.io checks the slugs.
    categories_override_ok: BTreeSet<&'static str>,
}

impl KnownExceptions {
    /// Build the hardcoded exception set.  Update this list whenever a
    /// new intentional deviation lands (with an in-code citation) - the
    /// audit fires on every new deviation by default.
    const fn new() -> Self {
        // No deviation: every member inherits every metadata field and
        // every dependency from the workspace.  An entry is added only when
        // a crate is published (lane:crates-publish), with its reason.
        Self {
            publish_explicit: BTreeSet::new(),
            underscore_bin_ok: BTreeSet::new(),
            nonworkspace_dep_ok: BTreeSet::new(),
            readme_override_ok: BTreeSet::new(),
            keywords_override_ok: BTreeSet::new(),
            categories_override_ok: BTreeSet::new(),
        }
    }
}

/// One discovered member: path + parsed manifest + raw source text.
pub(crate) struct DiscoveredMember<'a> {
    /// Workspace-relative path (e.g. `"crates/md2html-version/Cargo.toml"`).
    pub(crate) manifest_path: &'a str,
    /// Parsed shape.
    pub(crate) manifest: &'a MemberManifest,
}

/// Run every audit invariant and concatenate findings.  An empty
/// return value means the audit passed.
pub(crate) fn audit_all(
    root: &RootManifest,
    root_text: &str,
    members: &[DiscoveredMember<'_>],
    discovered_paths: &BTreeSet<String>,
) -> Vec<Finding> {
    let exc = KnownExceptions::new();
    let mut findings = Vec::new();

    findings.extend(audit_member_list_accuracy(root, discovered_paths));
    findings.extend(audit_root_is_virtual(root));
    findings.extend(audit_resolver_pinned(root));
    findings.extend(audit_default_members_intent(root_text));
    findings.extend(audit_no_workspace_rust_version(root));
    findings.extend(audit_product_present(members));

    for member in members {
        let id = member_id_from_path(member.manifest_path);
        findings.extend(audit_no_path_dep_outside_workspace(member, &id, root));
        findings.extend(check_inherited(
            member.manifest.package.edition.as_ref(),
            "edition",
            "3.3",
            &id,
        ));
        findings.extend(audit_rust_version(member, &id));
        findings.extend(audit_metadata_fields(member, &id, &exc));
        findings.extend(audit_deps_inherit_workspace(member, &id, &exc));
        findings.extend(audit_lints_inherit_workspace(member, &id));
        findings.extend(audit_no_workspace_policy_blocks(member, &id));
        findings.extend(audit_lib_bin_name_convention(member, &id, &exc));
        findings.extend(audit_publish_partition(member, &id, &exc));
        findings.extend(audit_docs_rs_metadata_present(member, &id));
    }

    findings
}

/// `"crates/md2html-version/Cargo.toml"` → `"md2html-version"`.
fn member_id_from_path(path: &str) -> String {
    path.rsplit_once('/')
        .and_then(|(parent, _)| parent.rsplit_once('/').map(|(_, last)| last))
        .unwrap_or(path)
        .to_owned()
}

// ─────────────────────────────────────────────────────────────────────────────
// Root-scoped invariants
// ─────────────────────────────────────────────────────────────────────────────

/// **Invariant 3.1**: declared `[workspace.members]` matches the
/// set of discovered `Cargo.toml` files exactly.
fn audit_member_list_accuracy(root: &RootManifest, discovered: &BTreeSet<String>) -> Vec<Finding> {
    let Some(ws) = root.workspace.as_ref() else {
        return vec![Finding {
            invariant: "3.1",
            member: "<root>".to_owned(),
            detail: "root Cargo.toml has no [workspace] table".to_owned(),
        }];
    };
    let declared: BTreeSet<&str> = ws.members.iter().map(String::as_str).collect();
    let discovered_set: BTreeSet<&str> = discovered.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    for missing in discovered_set.difference(&declared) {
        findings.push(Finding {
            invariant: "3.1",
            member: "<root>".to_owned(),
            detail: format!("discovered `{missing}/Cargo.toml` but not in [workspace.members]"),
        });
    }
    for extra in declared.difference(&discovered_set) {
        findings.push(Finding {
            invariant: "3.1",
            member: "<root>".to_owned(),
            detail: format!("[workspace.members] lists `{extra}` but no Cargo.toml exists there"),
        });
    }
    findings
}

/// **Invariant 3.9**: root `Cargo.toml` is a virtual workspace.
fn audit_root_is_virtual(root: &RootManifest) -> Vec<Finding> {
    if root.package.is_some() {
        vec![Finding {
            invariant: "3.9",
            member: "<root>".to_owned(),
            detail: "root Cargo.toml has a [package] block; MD2HTML uses a virtual workspace"
                .to_owned(),
        }]
    } else {
        Vec::new()
    }
}

/// **Invariant 3.10**: `resolver = "3"` (ed-2024 default).
fn audit_resolver_pinned(root: &RootManifest) -> Vec<Finding> {
    let Some(ws) = root.workspace.as_ref() else {
        return Vec::new();
    };
    match ws.resolver.as_deref() {
        Some("3") => Vec::new(),
        Some(other) => vec![Finding {
            invariant: "3.10",
            member: "<root>".to_owned(),
            detail: format!("[workspace].resolver = {other:?}; MD2HTML requires \"3\""),
        }],
        None => vec![Finding {
            invariant: "3.10",
            member: "<root>".to_owned(),
            detail: "[workspace].resolver is unset; MD2HTML requires \"3\"".to_owned(),
        }],
    }
}

/// **Invariant 3.11**: `default-members` is set or the all-members
/// fallback is documented in a nearby comment.
fn audit_default_members_intent(root_text: &str) -> Vec<Finding> {
    if root_text.contains("default-members") {
        return Vec::new();
    }
    // Look for a comment mentioning default-members as a deliberate choice.
    if root_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('#') && trimmed.to_ascii_lowercase().contains("default-members")
    }) {
        return Vec::new();
    }
    vec![Finding {
        invariant: "3.11",
        member: "<root>".to_owned(),
        detail: "root [workspace] neither sets `default-members` nor documents the all-members \
                 fallback in a comment"
            .to_owned(),
    }]
}

/// **Invariant 3.4**, root half: `[workspace.package]` declares no
/// `rust-version`, so no member can inherit one.
fn audit_no_workspace_rust_version(root: &RootManifest) -> Vec<Finding> {
    let declared = root
        .workspace
        .as_ref()
        .and_then(|ws| ws.package.as_ref())
        .is_some_and(|package| package.contains_key("rust-version"));
    if declared {
        vec![Finding {
            invariant: "3.4",
            member: "<root>".to_owned(),
            detail: "`[workspace.package]` declares `rust-version`; only md2html declares one, \
                     in crates/md2html/Cargo.toml (the root Cargo.toml says why)"
                .to_owned(),
        }]
    } else {
        Vec::new()
    }
}

/// **Invariant 3.4**, product half: md2html is among the members, so the
/// one `rust-version` the workspace promises exists.
fn audit_product_present(members: &[DiscoveredMember<'_>]) -> Vec<Finding> {
    if members
        .iter()
        .any(|member| member.manifest_path == PRODUCT_MANIFEST)
    {
        Vec::new()
    } else {
        vec![Finding {
            invariant: "3.4",
            member: "<root>".to_owned(),
            detail: format!(
                "no `{PRODUCT_MANIFEST}`: the member that declares the workspace's minimum Rust \
                 is missing"
            ),
        }]
    }
}
