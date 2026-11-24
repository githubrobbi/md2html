// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! Property 10: the job of each pull-request gate runs that gate's command.
//!
//! Properties 1 to 4 hold that a gate has a job; this one holds what the
//! job runs.  The job must have a one-line `run:` step that is the gate's
//! command as the hooks run it, compared word by word: a `{{...}}`
//! placeholder (the hooks' staged files or commit ranges) is left out, and
//! so is a leading `env NAME=value` (the job sets its environment in
//! `env:`).  Where CI runs the gate otherwise (another runner, CI's own
//! flags), the gate says so in `ci_command`, the exact `run:` line, and that
//! line is what must be there: a difference is written beside the gate, not
//! only in the workflow.

use crate::manifest::{Gate, Manifest};

/// One issue per pull-request gate whose job has no `run:` line equal to
/// what the gate runs; empty when every job agrees.
pub(crate) fn check(manifest: &Manifest, workflow_text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for gate in manifest.pr_fast_gates() {
        let job = gate.pr_fast_job_id();
        let Some(expected) = expected_run(gate) else {
            issues.push(format!(
                "Property 10 (commands): gate `{}` has no command and no ci_command",
                gate.id
            ));
            continue;
        };
        let runs = job_runs(workflow_text, job);
        if !runs.iter().any(|run| words(run) == words(&expected)) {
            issues.push(format!(
                "Property 10 (commands): job `{job}` of pr-fast.yml has no `run: {expected}` \
                 for gate `{}`; make the job run the gate's command, or write the line it \
                 runs as the gate's `ci_command` in gates.toml",
                gate.id
            ));
        }
    }
    issues
}

/// The `run:` line the gate's job must have: its `ci_command`, else its
/// command without placeholders and without a leading `env NAME=value`.
fn expected_run(gate: &Gate) -> Option<String> {
    if let Some(line) = &gate.ci_command {
        return Some(line.clone());
    }
    let mut tokens = gate.command.iter().map(String::as_str).peekable();
    if tokens.next_if_eq(&"env").is_some() {
        while tokens.next_if(|token| token.contains('=')).is_some() {}
    }
    let kept: Vec<&str> = tokens.filter(|token| !token.starts_with("{{")).collect();
    (!kept.is_empty()).then(|| kept.join(" "))
}

/// The words of a command line, blanks of any length between them.
fn words(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

/// The one-line `run:` steps of `job`: from its `  <job>:` line to the next
/// line at the same indentation.  A `run: |` block is not a one-line step.
fn job_runs<'text>(workflow_text: &'text str, job: &str) -> Vec<&'text str> {
    let header = format!("  {job}:");
    let mut lines = workflow_text
        .lines()
        .skip_while(|line| line.trim_end() != header);
    if lines.next().is_none() {
        return Vec::new();
    }
    lines
        .take_while(|line| line.is_empty() || line.starts_with("   "))
        .filter_map(|line| {
            let indented = line.trim_start();
            let step = indented.strip_prefix("- ").unwrap_or(indented);
            step.strip_prefix("run:").map(str::trim)
        })
        .filter(|run| !run.is_empty() && !run.starts_with('|') && !run.starts_with('>'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest;

    /// A manifest with a plain gate, one with an `env` prefix and a
    /// placeholder, and one CI runs otherwise.
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
command = ["cargo", "fmt", "--all", "--", "--check"]

[[gate]]
id = "rustdoc"
label = "rustdoc"
tiers = ["pr-fast"]
gate_when = "rust_changed"
command = ["env", "RUSTDOCFLAGS=-Dwarnings", "cargo", "doc", "{{STAGED_TOML}}", "--no-deps"]
consumer_names = { "pr-fast" = "docs" }

[[gate]]
id = "lint-ci"
label = "clippy"
tiers = ["pr-fast"]
gate_when = "rust_changed"
command = ["just", "lint-ci"]
ci_command = "cargo clippy -- -D warnings"
consumer_names = { "pr-fast" = "clippy" }
"#;

    /// Jobs that run each gate's line.
    const AGREEING: &str = "
jobs:
  fmt:
    steps:
      - run: cargo fmt --all -- --check
  docs:
    env:
      RUSTDOCFLAGS: -Dwarnings
    steps:
      - run: cargo doc  --no-deps
  clippy:
    steps:
      - name: lint
        run: cargo clippy -- -D warnings
  required:
    steps:
      - run: |
          cargo fmt --all -- --check
";

    /// The issues for `workflow` against the manifest above.
    ///
    /// # Panics
    ///
    /// When the manifest fixture does not parse.
    fn issues(workflow: &str) -> Vec<String> {
        check(&manifest::parse(MANIFEST).unwrap(), workflow)
    }

    /// Jobs that run the gates' lines, blanks aside, agree.
    ///
    /// # Panics
    ///
    /// When an issue is reported.
    #[test]
    fn jobs_that_run_their_gates_pass() {
        assert_eq!(issues(AGREEING), Vec::<String>::new());
    }

    /// A job that runs other flags, a gate whose line is only in another
    /// job or only inside a `run: |` block, and a `ci_command` the job does
    /// not run are each drift.
    ///
    /// # Panics
    ///
    /// When one of these is not reported, naming the gate.
    #[test]
    fn a_job_that_runs_something_else_is_drift() {
        let other_flags = AGREEING.replace("cargo doc  --no-deps", "cargo doc --no-deps --locked");
        let [flags_issue] = issues(&other_flags).try_into().unwrap();
        assert!(flags_issue.contains("gate `rustdoc`"), "{flags_issue}");

        let elsewhere = AGREEING.replace("      - run: cargo fmt --all -- --check\n", "");
        let [elsewhere_issue] = issues(&elsewhere).try_into().unwrap();
        assert!(elsewhere_issue.contains("gate `fmt`"), "{elsewhere_issue}");

        let unlisted = AGREEING.replace("cargo clippy -- -D warnings", "just lint-ci");
        let [unlisted_issue] = issues(&unlisted).try_into().unwrap();
        assert!(
            unlisted_issue.contains("gate `lint-ci`"),
            "{unlisted_issue}"
        );
    }

    /// The real workflow runs the real manifest's gates.
    ///
    /// # Panics
    ///
    /// When `gates.toml` does not parse, or a job of `pr-fast.yml` does not
    /// run its gate.
    #[test]
    fn the_repository_agrees() {
        let manifest = manifest::parse(include_str!("../../gates.toml")).unwrap();
        let workflow = include_str!("../../../../.github/workflows/pr-fast.yml");
        assert_eq!(check(&manifest, workflow), Vec::<String>::new());
    }
}
