// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! Property 9: the `classify` job of `pr-fast.yml` computes each change
//! class with the pattern `[classification]` of the manifest gives it.
//!
//! The hooks are generated from that table; the workflow is written by hand,
//! so it is held to the table here.  The job sets one shell variable per
//! class, each on a line of its own:
//!
//! ```text
//! RUST=$(match '<pattern>')
//! ```
//!
//! A class whose line is missing, written twice, or carries another pattern
//! is drift.

use crate::manifest::Manifest;

/// What opens a class's line after the variable's name and `=`.
const CALL_OPEN: &str = "$(match '";

/// What closes it.
const CALL_CLOSE: &str = "')";

/// One issue per class of the manifest whose `classify` line is missing,
/// repeated or different; empty when the workflow agrees.
pub(crate) fn check(manifest: &Manifest, workflow_text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for (class, pattern) in manifest.classification.classes() {
        let found: Vec<&str> = workflow_text
            .lines()
            .filter_map(|line| class_pattern(line, class))
            .collect();
        match found.as_slice() {
            [one] if *one == pattern => {}
            [one] => issues.push(format!(
                "Property 9 (change classes): pr-fast.yml computes {class} with '{one}', \
                 [classification] in gates.toml says '{pattern}'"
            )),
            [] => issues.push(format!(
                "Property 9 (change classes): pr-fast.yml has no `{class}={CALL_OPEN}...{CALL_CLOSE}` \
                 line; [classification] in gates.toml gives '{pattern}'"
            )),
            several => issues.push(format!(
                "Property 9 (change classes): pr-fast.yml computes {class} {} times; once, \
                 with '{pattern}'",
                several.len()
            )),
        }
    }
    issues
}

/// The pattern of `line` when it is `<class>=$(match '<pattern>')`, leading
/// and trailing blanks aside.
fn class_pattern<'line>(line: &'line str, class: &str) -> Option<&'line str> {
    line.trim()
        .strip_prefix(class)?
        .strip_prefix('=')?
        .strip_prefix(CALL_OPEN)?
        .strip_suffix(CALL_CLOSE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest;

    /// A manifest with the three classes and one gate.
    const MANIFEST: &str = r#"
[classification]
rust = '\.rs$'
dep = '^Cargo\.lock$'
infra = '^scripts/'

[[gate]]
id = "fmt"
label = "fmt"
tiers = ["pr-fast"]
gate_when = "rust_changed"
"#;

    /// The classify step's three lines, as the manifest above says.
    const AGREEING: &str = "
      - run: |
          RUST=$(match '\\.rs$')
          DEP=$(match '^Cargo\\.lock$')
          INFRA=$(match '^scripts/')
";

    /// The issues for `workflow` against the manifest above.
    ///
    /// # Panics
    ///
    /// When the manifest fixture does not parse.
    fn issues(workflow: &str) -> Vec<String> {
        check(&manifest::parse(MANIFEST).unwrap(), workflow)
    }

    /// A workflow that uses the manifest's patterns has no issue.
    ///
    /// # Panics
    ///
    /// When an issue is reported.
    #[test]
    fn agreeing_classes_pass() {
        assert_eq!(issues(AGREEING), Vec::<String>::new());
    }

    /// Another pattern, a missing line and a repeated line are each reported,
    /// naming the class.
    ///
    /// # Panics
    ///
    /// When one of the three is not reported, or the report does not name the
    /// class and what was expected.
    #[test]
    fn a_different_missing_or_repeated_class_is_drift() {
        let different = AGREEING.replace("'^Cargo\\.lock$'", "'^(.*Cargo\\.toml$|Cargo\\.lock$)'");
        let [different_issue] = issues(&different).try_into().unwrap();
        assert!(
            different_issue.contains("computes DEP with '^(.*Cargo"),
            "{different_issue}"
        );

        let missing = AGREEING.replace("          INFRA=$(match '^scripts/')\n", "");
        let [missing_issue] = issues(&missing).try_into().unwrap();
        assert!(
            missing_issue.contains("no `INFRA=") && missing_issue.contains("'^scripts/'"),
            "{missing_issue}"
        );

        let repeated = format!("{AGREEING}          RUST=$(match '\\.rs$')\n");
        let [repeated_issue] = issues(&repeated).try_into().unwrap();
        assert!(
            repeated_issue.contains("computes RUST 2 times"),
            "{repeated_issue}"
        );
    }

    /// The real workflow agrees with the real manifest.
    ///
    /// # Panics
    ///
    /// When `gates.toml` does not parse, or `pr-fast.yml` computes a class
    /// with another pattern.
    #[test]
    fn the_repository_agrees() {
        let manifest = manifest::parse(include_str!("../../gates.toml")).unwrap();
        let workflow = include_str!("../../../../.github/workflows/pr-fast.yml");
        assert_eq!(check(&manifest, workflow), Vec::<String>::new());
    }
}
