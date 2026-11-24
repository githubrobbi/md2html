// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Commit-time code classification, from the manifest's path patterns.
//!
//! Using the manifest's existing patterns lets a
//! `gate_when = "code_changed"` gate run at pre-commit (staged-scoped) with the
//! same meaning it has at pre-push (range-scoped): "code" is rust OR dep OR
//! infra, derived from the `[classification]` table, never a second regex to
//! drift.

use crate::manifest::Manifest;

/// The `has_staged_code` helper the pre-commit dispatch calls for every
/// `code_changed` gate.
///
/// No staged files (a manual `just lint-fast` run) conservatively runs the
/// gates: a helper that cannot classify must not skip.
pub(super) fn helpers(manifest: &Manifest) -> String {
    let patterns: Vec<&str> = manifest
        .classification
        .classes()
        .iter()
        .map(|&(_, pattern)| pattern)
        .collect();
    let regex = super::shell::shell_quote(&patterns.join("|"));
    format!(
        "\nhas_staged_code() {{ ! has_any_staged || printf '%s\\n' \"$STAGED_ALL\" | grep -Eq {regex}; }}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Execute the generated dispatch with a recording spawn instead of
    /// running the gate command, so the classification can be asserted
    /// against real staged-path shapes.
    ///
    /// # Panics
    ///
    /// When the real `gates.toml` does not parse; bash cannot be started or
    /// prints output that is not UTF-8; the synthetic `code_changed` gate is
    /// spawned for staged documentation alone, or skipped for a staged Rust
    /// file, `Cargo.lock`, a `Cargo.toml`, `scripts/ci/gates.toml`,
    /// documentation with Rust, or nothing staged; or the pre-commit preamble
    /// does not list staged files with `--diff-filter=ACMRD`, which takes in
    /// deletions.
    #[test]
    fn docs_skip_but_code_dependencies_and_infra_run() {
        let mut manifest: Manifest =
            toml::from_str(include_str!("../../../gates.toml")).expect("real manifest");
        // One synthetic pre-commit gate is enough: the classification
        // helper is what is under test, not the manifest's gate set.
        manifest.gate.clear();
        manifest.gate.push(crate::manifest::Gate {
            id: String::from("code-gate"),
            label: String::from("x"),
            command: vec![String::from("true")],
            tiers: vec![String::from("pre-commit")],
            when: String::from("code_changed"),
            tool: String::from("bash"),
            expected_runtime_secs: 1,
            bucket: Some(String::from("bg")),
            order: 1,
            consumer_names: alloc::collections::BTreeMap::new(),
            notes: String::new(),
        });
        let script = format!(
            "has_any_staged() {{ [[ -n \"$STAGED_ALL\" ]]; }}\nspawn() {{ printf '%s' \"$1\"; }}\n{}{}",
            helpers(&manifest),
            super::super::pre_commit::render_dispatch_fast(&manifest),
        );
        for (paths, runs) in [
            ("docs/forge/COMPONENTS.md\nREADME.md", false),
            ("crates/md2html-core/src/lib.rs", true),
            ("Cargo.lock", true),
            ("crates/md2html-core/Cargo.toml", true),
            ("scripts/ci/gates.toml", true),
            ("README.md\ncrates/md2html-core/src/lib.rs", true),
            ("", true),
        ] {
            let output = Command::new("bash")
                .args(["-c", &script])
                .env("STAGED_ALL", paths)
                .output()
                .expect("bash fixture");
            assert_eq!(
                String::from_utf8(output.stdout).expect("UTF-8 output"),
                if runs { "code-gate" } else { "" },
                "{paths}"
            );
        }
        assert!(
            super::super::PREAMBLE_PRE_COMMIT.contains("--diff-filter=ACMRD"),
            "deletions must also be classified"
        );
    }
}
