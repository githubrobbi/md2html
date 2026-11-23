// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! The 15 manifest invariants from
//! `docs/dev/architecture/code_clean/phase_1_manifest_implementation_plan.md`
//! §3, encoded as machine-checkable assertions.
//!
//! Each public function in this module corresponds to one or more
//! invariants and returns a `Vec<Finding>` (empty on success).  The
//! top-level [`audit_all`] entry point calls every check and
//! concatenates the results - exit code is derived from
//! `result.is_empty()`.
//!
//! # Design rationale
//!
//! * Findings are **plain text** (no JSON, no enum taxonomy) because the audit
//!   only fires at PR time and a human is always the next reader.
//! * The invariants are **deliberately conservative**: the audit prefers false
//!   negatives (missing a real drift) over false positives (failing on
//!   intentional exceptions).  The hardcoded exception list in
//!   [`KnownExceptions`] documents every accepted deviation with an in-code
//!   citation.

mod member;
#[cfg(test)]
mod tests;

use alloc::collections::BTreeSet;

use member::{
    audit_deps_inherit_workspace, audit_docs_rs_metadata_present, audit_lib_bin_name_convention,
    audit_lints_inherit_workspace, audit_metadata_fields, audit_no_path_dep_outside_workspace,
    audit_no_workspace_policy_blocks, audit_publish_partition, check_inherited,
};

use crate::manifest::{MemberManifest, RootManifest};

/// Single audit finding.  Self-contained diagnostic; the CI log
/// alone tells the contributor what to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// Phase 1 invariant number (e.g. `"3.7"`).
    pub(crate) invariant: &'static str,
    /// Member crate id (e.g. `"md2html-core"`) for member-scoped findings,
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
/// Update this whenever a new exception lands - the audit fires on every new
/// deviation by default, forcing the contributor to either fix the
/// manifest or extend this list with a citation.
struct KnownExceptions {
    /// Members allowed to set `publish = true` explicitly (not via
    /// workspace inheritance).  Source: `release-automation-baseline.md`
    /// §10 deviation row 5.
    publish_explicit: BTreeSet<&'static str>,
    /// Members allowed underscore-style `[[bin]] name = ...` (vs. the
    /// hyphen convention).  Source: md2html-gen-workflow comment in
    /// `scripts/ci/md2html-gen-workflow/src/main.rs`.
    underscore_bin_ok: BTreeSet<&'static str>,
    /// `(member, dep_name)` pairs allowed to bypass workspace
    /// inheritance.  Documented in Phase 1 §3.6.
    nonworkspace_dep_ok: BTreeSet<(&'static str, &'static str)>,
    /// Members allowed to set `readme = "README.md"` directly (not via
    /// workspace inheritance).  This is the implementation of the
    /// "deliberately overridden value with a justification comment"
    /// escape hatch documented in Phase 1 §3.5 - publishable library
    /// crates that ship their own per-crate `README.md` need to point
    /// crates.io / docs.rs at that file rather than at the workspace-
    /// root app-focused README that gets inherited.  The override is
    /// tightened to the literal value `"README.md"` (same-name pattern,
    /// alongside the crate's `Cargo.toml`) so an exception listed crate
    /// can't accidentally point at an arbitrary path.
    readme_override_ok: BTreeSet<&'static str>,
    /// Members allowed to set `keywords = [...]` directly (not via
    /// workspace inheritance).  Same Phase 1 §3.5 "deliberately
    /// overridden with justification" escape hatch as `readme`.
    /// Publishable library crates benefit from per-crate keywords
    /// tailored to their specific domain (e.g. `md2html-time` is a
    /// generic NTFS FILETIME helper that wants `["ntfs", "filetime",
    /// "datetime", ...]` rather than the workspace-app defaults
    /// `["mft", "ntfs", "file-search", "windows", "polars"]` which
    /// describe the *application*, not the library).  The override
    /// is required to be a **non-empty TOML array** - an empty array
    /// still fires (cargo treats `keywords = []` as legal but it
    /// defeats the discoverability purpose of an override).  Per-
    /// keyword shape (max 5, max 20 chars, `^[a-zA-Z][a-zA-Z0-9_-]*$`)
    /// is *not* re-validated here because `cargo publish --dry-run`
    /// in the weekly `crates-io-dry-run.yml` workflow enforces it
    /// authoritatively against crates.io's own validator.
    keywords_override_ok: BTreeSet<&'static str>,
    /// Members allowed to set `categories = [...]` directly (not via
    /// workspace inheritance).  Same rationale as `keywords_override_ok`:
    /// publishable library crates want category slugs that match
    /// their library role (e.g. `md2html-time` → `["date-and-time"]`)
    /// rather than the workspace-app defaults (`["filesystem",
    /// "command-line-utilities"]`).  Same non-empty-array requirement.
    /// Slug validity (must be on `https://crates.io/category_slugs`)
    /// is enforced authoritatively by `cargo publish --dry-run`.
    categories_override_ok: BTreeSet<&'static str>,
}

impl KnownExceptions {
    /// Build the hardcoded exception set.  Update this list whenever a
    /// new intentional deviation lands (with an in-code citation) - the
    /// audit fires on every new deviation by default.
    const fn new() -> Self {
        // The template starts with ZERO deviations: every member inherits
        // every fingerprint field and every dependency from the workspace.
        // Add entries here only when a crate earns a documented exception
        // (e.g. a publishable library crate that ships its own README.md
        // and tailored keywords/categories) - each addition needs an
        // in-code citation, and the audit fires on every new deviation
        // by default.
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
    /// Workspace-relative path (e.g. `"crates/md2html-core/Cargo.toml"`).
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

    for member in members {
        let id = member_id_from_path(member.manifest_path);
        findings.extend(audit_no_path_dep_outside_workspace(member, &id, root));
        findings.extend(check_inherited(
            member.manifest.package.edition.as_ref(),
            "edition",
            "3.3",
            &id,
        ));
        // Invariant 3.4 (`rust-version` consistency) was retired on 2026-05-18
        // when the workspace dropped its `[workspace.package].rust-version`
        // claim (refs issue #267 - the polars/nightly transitive feature
        // makes any stable MSRV claim unverifiable).  Per-crate manifests no
        // longer carry `rust-version.workspace = true`.
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

/// `"crates/md2html-core/Cargo.toml"` → `"md2html-core"`.
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
