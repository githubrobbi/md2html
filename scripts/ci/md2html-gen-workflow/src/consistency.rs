// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Properties 5 to 8: what every workflow shares, checked across all
//! of them.
//!
//! The structural validator (properties 1 to 4) reads `pr-fast.yml`
//! only, because that is the file the gate manifest drives. The
//! release, preview, nightly and dependabot workflows are hand-written
//! and were outside every check, which is exactly where they drift:
//! one action pinned at two SHAs, a zig version bumped in one job and
//! not the other, a target's RUSTFLAGS copied with a typo, a nextest
//! profile renamed in `.config/nextest.toml` and not in the workflow
//! that names it. These four checks read every `*.yml` under
//! `.github/workflows` with the same read-only posture as the rest of
//! the tool: they can fail a push, never rewrite a file.
//!
//! 5. One pin per action: every `uses: owner/name@<sha>` for a given action
//!    resolves to the same SHA across all workflows.
//! 6. Toolchain versions: every `ziglang==<v>`, `cargo-zigbuild@<v>`,
//!    `cargo-vet@<v>` and `cargo-deny@<v>`, and the `CARGO_VET_VERSION: <v>`
//!    and `CARGO_DENY_VERSION: <v>` of a job that downloads the release,
//!    matches `[toolchain]` in the manifest.
//! 7. Build targets: a matrix row `target: T` followed by `rustflags: "X"`, and
//!    a job that names `--target T` under a `RUSTFLAGS: "X"`, carry the flags
//!    `[[target]]` gives for `T`; `target-cpu=native` is refused anywhere.
//! 8. Nextest profiles: every `--profile <name>` names a `[profile.<name>]` in
//!    `.config/nextest.toml`.

use alloc::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, Result};
use regex::Regex;

use crate::manifest::Manifest;

/// One workflow file's text, with its name for messages.
pub(crate) struct WorkflowText {
    /// The file name (`release.yml`).
    pub(crate) name: String,
    /// The file's contents.
    pub(crate) text: String,
}

/// Runs properties 5 to 8 over the workflows (see [`read_workflows`]).
///
/// # Errors
/// An invalid built-in regex.
pub(crate) fn check(
    manifest: &Manifest,
    workflows: &[WorkflowText],
    nextest_toml: &Path,
) -> Result<Vec<String>> {
    let profiles = nextest_profiles(nextest_toml);
    let mut issues = Vec::new();
    issues.extend(check_action_pins(workflows)?);
    issues.extend(check_toolchain(manifest, workflows)?);
    issues.extend(check_targets(manifest, workflows)?);
    issues.extend(check_profiles(&profiles, workflows)?);
    Ok(issues)
}

/// Every `*.yml` under the directory, sorted by name.
///
/// # Errors
///
/// The directory cannot be listed (`read workflows at <dir>`), one of its
/// entries cannot be read, a `.yml` file's name is not UTF-8, or a `.yml`
/// file in it cannot be read as UTF-8 text (`read workflow <path>`); the I/O
/// error follows.
pub(crate) fn read_workflows(dir: &Path) -> Result<Vec<WorkflowText>> {
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("read workflows at {}", dir.display()))?;
    for found in entries {
        let entry = found?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yml") {
            continue;
        }
        let name = entry.file_name().into_string().map_err(|name| {
            anyhow::anyhow!(
                "workflow {} has a name that is not UTF-8",
                Path::new(&name).display()
            )
        })?;
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read workflow {}", path.display()))?;
        out.push(WorkflowText { name, text });
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(out)
}

/// The `[profile.<name>]` names in the nextest config (empty when the
/// file is absent: property 8 then reports every named profile).
fn nextest_profiles(path: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("[profile.")
                .and_then(|rest| rest.split(['.', ']']).next())
                .map(str::to_owned)
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// Property 5 - one pin per action
// ─────────────────────────────────────────────────────────────────────

/// Every action pinned at more than one SHA across the workflows.
///
/// # Errors
///
/// The `uses: owner/name@<sha>` pattern does not compile; the message is the
/// `regex` crate's parse error. It is a fixed literal, so this happens only if
/// it is edited into an invalid pattern.
fn check_action_pins(workflows: &[WorkflowText]) -> Result<Vec<String>> {
    let pin = Regex::new(r"uses:\s*([A-Za-z0-9_.-]+/[A-Za-z0-9_./-]+)@([0-9a-f]{40})")?;
    let mut pins: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for workflow in workflows {
        for capture in pin.captures_iter(&workflow.text) {
            let action = capture.get(1).map_or("", |hit| hit.as_str()).to_owned();
            let sha = capture.get(2).map_or("", |hit| hit.as_str()).to_owned();
            pins.entry(action)
                .or_default()
                .entry(sha)
                .or_default()
                .insert(workflow.name.clone());
        }
    }
    Ok(pins
        .iter()
        .filter(|(_action, by_sha)| by_sha.len() > 1)
        .map(|(action, by_sha)| {
            let spread: Vec<String> = by_sha
                .iter()
                .map(|(sha, files)| {
                    format!(
                        "{} in {}",
                        sha.get(..7).unwrap_or(sha),
                        files.iter().cloned().collect::<Vec<_>>().join(", ")
                    )
                })
                .collect();
            format!(
                "property 5 (one pin per action): `{action}` is pinned at {} SHAs: {}",
                by_sha.len(),
                spread.join("; ")
            )
        })
        .collect())
}

// ─────────────────────────────────────────────────────────────────────
// Property 6 - toolchain versions
// ─────────────────────────────────────────────────────────────────────

/// Every toolchain mention that disagrees with `[toolchain]`.
///
/// # Errors
///
/// One of the version patterns (`ziglang==`, `cargo-zigbuild@`, `cargo-vet@`
/// or `CARGO_VET_VERSION:`, `cargo-deny@` or `CARGO_DENY_VERSION:`) does not
/// compile; the message is the `regex` crate's parse error. They are fixed
/// literals, so this happens only if one is edited into an invalid pattern. A
/// `[toolchain]` key with no pattern, like a version that disagrees, is a
/// finding in the list, not an error.
fn check_toolchain(manifest: &Manifest, workflows: &[WorkflowText]) -> Result<Vec<String>> {
    let matchers: [(&str, Regex); 4] = [
        ("ziglang", Regex::new("ziglang==([0-9][0-9.]*)")?),
        (
            "cargo-zigbuild",
            Regex::new("cargo-zigbuild@([0-9][0-9.]*)")?,
        ),
        (
            "cargo-vet",
            Regex::new(r#"(?:cargo-vet@|CARGO_VET_VERSION:\s*"?)([0-9][0-9.]*)"#)?,
        ),
        (
            "cargo-deny",
            Regex::new(r#"(?:cargo-deny@|CARGO_DENY_VERSION:\s*"?)([0-9][0-9.]*)"#)?,
        ),
    ];
    let mut issues = Vec::new();
    for (key, wanted) in &manifest.toolchain {
        let Some((_name, matcher)) = matchers.iter().find(|(name, _re)| name == key) else {
            issues.push(format!(
                "property 6 (toolchain): `[toolchain] {key}` has no matcher in md2html-gen-workflow \
                 (known: ziglang, cargo-zigbuild, cargo-vet, cargo-deny)"
            ));
            continue;
        };
        for workflow in workflows {
            for capture in matcher.captures_iter(&workflow.text) {
                let found = capture.get(1).map_or("", |hit| hit.as_str());
                if found != wanted {
                    issues.push(format!(
                        "property 6 (toolchain): {} pins {key} {found}; the manifest says {wanted}",
                        workflow.name
                    ));
                }
            }
        }
    }
    Ok(issues)
}

// ─────────────────────────────────────────────────────────────────────
// Property 7 - build targets and their flags
// ─────────────────────────────────────────────────────────────────────

/// One job's flags and the targets its steps name.
struct JobTargets {
    /// The job key.
    name: String,
    /// The job-level `RUSTFLAGS`, when set.
    flags: Option<String>,
    /// Every `--target` the steps name.
    targets: BTreeSet<String>,
}

/// Matrix rows and jobs whose flags differ from `[[target]]`, and any
/// `target-cpu=native`.
///
/// # Errors
///
/// The matrix-row patterns (`target:` and `rustflags:`) or those of [`jobs_of`]
/// do not compile; the message is the `regex` crate's parse error. They are
/// fixed literals, so this happens only if one is edited into an invalid
/// pattern.
fn check_targets(manifest: &Manifest, workflows: &[WorkflowText]) -> Result<Vec<String>> {
    let table: BTreeMap<&str, &str> = manifest
        .targets
        .iter()
        .map(|row| (row.target.as_str(), row.rustflags.as_str()))
        .collect();
    let matrix_target = Regex::new(r#"^\s*-?\s*target:\s*"?([A-Za-z0-9_-]+)"?\s*$"#)?;
    let matrix_flags = Regex::new(r#"^\s*rustflags:\s*"([^"]*)"\s*$"#)?;
    let mut issues = Vec::new();
    for workflow in workflows {
        let lines: Vec<&str> = workflow.text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            // A comment saying "never target-cpu=native" is not a use of it.
            if line.trim_start().starts_with('#') {
                continue;
            }
            if let Some(capture) = matrix_target.captures(line) {
                let target = capture.get(1).map_or("", |hit| hit.as_str());
                if let Some(wanted) = table.get(target) {
                    for later in lines.iter().skip(index + 1).take(4) {
                        if let Some(flags) = matrix_flags.captures(later) {
                            let found = flags.get(1).map_or("", |hit| hit.as_str());
                            if found != *wanted {
                                issues.push(format!(
                                    "property 7 (targets): {} matrix row {target} carries \
                                     rustflags \"{found}\"; the manifest says \"{wanted}\"",
                                    workflow.name
                                ));
                            }
                        }
                    }
                }
            }
            if line.contains("target-cpu=native") {
                issues.push(format!(
                    "property 7 (targets): {} line {} sets target-cpu=native (never: the \
                     binary would SIGILL on another runner)",
                    workflow.name,
                    index + 1
                ));
            }
        }
        for job in jobs_of(&lines)? {
            let Some(found) = job.flags else {
                continue;
            };
            for target in job.targets {
                if let Some(wanted) = table.get(target.as_str())
                    && found != *wanted
                {
                    issues.push(format!(
                        "property 7 (targets): {} job `{}` builds {target} under \
                         RUSTFLAGS \"{found}\"; the manifest says \"{wanted}\"",
                        workflow.name, job.name
                    ));
                }
            }
        }
    }
    Ok(issues)
}

/// The jobs of a workflow with their job-level `RUSTFLAGS` and the
/// `--target` triples their steps name.
///
/// # Errors
///
/// One of its three patterns (a job-level `RUSTFLAGS:`, a `--target` argument,
/// a job key two spaces in) does not compile; the message is the `regex`
/// crate's parse error. They are fixed literals, so this happens only if one is
/// edited into an invalid pattern.
fn jobs_of(lines: &[&str]) -> Result<Vec<JobTargets>> {
    let env_flags = Regex::new(r#"^\s*RUSTFLAGS:\s*"([^"]*)"\s*$"#)?;
    let cli_target = Regex::new(r"--target\s+([A-Za-z0-9_-]+)")?;
    let job_key = Regex::new(r"^  ([A-Za-z0-9_-]+):\s*$")?;
    let mut in_jobs = false;
    let mut current: Option<JobTargets> = None;
    let mut jobs: Vec<JobTargets> = Vec::new();
    for line in lines {
        if line.trim_end() == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if let Some(capture) = job_key.captures(line) {
            if let Some(done) = current.take() {
                jobs.push(done);
            }
            current = Some(JobTargets {
                name: capture.get(1).map_or("", |hit| hit.as_str()).to_owned(),
                flags: None,
                targets: BTreeSet::new(),
            });
            continue;
        }
        let Some(job) = current.as_mut() else {
            continue;
        };
        if let Some(capture) = env_flags.captures(line)
            && line_indent(line) <= 6
        {
            job.flags = capture.get(1).map(|hit| hit.as_str().to_owned());
        }
        for capture in cli_target.captures_iter(line) {
            job.targets
                .insert(capture.get(1).map_or("", |hit| hit.as_str()).to_owned());
        }
    }
    if let Some(done) = current.take() {
        jobs.push(done);
    }
    Ok(jobs)
}

/// Leading spaces of a line.
fn line_indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

// ─────────────────────────────────────────────────────────────────────
// Property 8 - nextest profiles
// ─────────────────────────────────────────────────────────────────────

/// Every `--profile <name>` a nextest invocation names that the
/// nextest config does not define.
///
/// `rustup toolchain install --profile minimal` is not nextest's profile;
/// only lines that run nextest count, backslash continuations joined.
///
/// # Errors
///
/// The pattern for `nextest run` or `nextest archive` with a `--profile` does
/// not compile; the message is the `regex` crate's parse error. It is a fixed
/// literal, so this happens only if it is edited into an invalid pattern.
fn check_profiles(profiles: &BTreeSet<String>, workflows: &[WorkflowText]) -> Result<Vec<String>> {
    let named = Regex::new(r"nextest\s+(?:run|archive)\b[^\n]*?--profile\s+([A-Za-z0-9_-]+)")?;
    let mut issues = Vec::new();
    for workflow in workflows {
        let mut seen = BTreeSet::new();
        let joined = logical_lines(&workflow.text);
        for capture in named.captures_iter(&joined) {
            let profile = capture.get(1).map_or("", |hit| hit.as_str());
            if profiles.contains(profile) || !seen.insert(profile.to_owned()) {
                continue;
            }
            issues.push(format!(
                "property 8 (nextest profiles): {} names `--profile {profile}`, which \
                 .config/nextest.toml does not define",
                workflow.name
            ));
        }
    }
    Ok(issues)
}

/// The text with backslash-continued lines joined, so a command split
/// over several lines reads as one.
fn logical_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if let Some(head) = line.trim_end().strip_suffix('\\') {
            out.push_str(head);
            out.push(' ');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use super::{
        WorkflowText, check, check_action_pins, check_profiles, check_targets, check_toolchain,
        read_workflows,
    };
    use crate::manifest::Manifest;

    /// A scratch directory under the system temp dir, unique per test
    /// process.
    ///
    /// It is removed by [`ScratchDir::remove`] at the end of the test that
    /// made it (a failing test leaves it for a look). (No `tempfile`
    /// dependency: the workspace keeps its dev-dependency tree lean and
    /// vetted.)
    struct ScratchDir(std::path::PathBuf);

    impl ScratchDir {
        /// Creates `<temp>/md2html-gen-workflow-<pid>-<name>`.
        ///
        /// # Panics
        ///
        /// When the directory cannot be created (`scratch dir`), as when the
        /// temp dir is not writable.
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "md2html-gen-workflow-{}-{name}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("scratch dir");
            Self(path)
        }

        /// Removes the directory and everything in it.
        ///
        /// # Panics
        ///
        /// When the directory or something in it cannot be removed (`remove the
        /// scratch dir`).
        fn remove(self) {
            std::fs::remove_dir_all(&self.0).expect("remove the scratch dir");
        }
    }

    /// Only `*.yml` files are read, in name order, and the full check
    /// over a consistent directory reports nothing.
    ///
    /// # Panics
    ///
    /// When a fixture file cannot be written or the scratch dir removed, the
    /// directory cannot be read, the names read are not `a.yml` then `b.yml`
    /// (`notes.md` read, or the order wrong), the full check fails to run, or
    /// it reports any issue for two workflows that pin `actions/checkout` at
    /// one SHA.
    #[test]
    fn a_workflows_directory_is_read_in_name_order_and_checks_clean() {
        let dir = ScratchDir::new("workflows");
        std::fs::write(
            dir.0.join("b.yml"),
            "jobs:\n  x:\n    steps:\n      - uses: actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa # v7\n",
        )
        .expect("write b");
        std::fs::write(
            dir.0.join("a.yml"),
            "jobs:\n  y:\n    steps:\n      - uses: actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa # v7\n",
        )
        .expect("write a");
        std::fs::write(dir.0.join("notes.md"), "not a workflow\n").expect("write md");
        let names: Vec<String> = read_workflows(&dir.0)
            .expect("readable")
            .into_iter()
            .map(|workflow| workflow.name)
            .collect();
        assert_eq!(names, ["a.yml", "b.yml"], "yml only, sorted by name");
        let workflows = read_workflows(&dir.0).expect("readable");
        let issues =
            check(&manifest(), &workflows, &dir.0.join("absent-nextest.toml")).expect("checks run");
        assert!(
            issues.is_empty(),
            "a consistent directory is clean: {issues:?}"
        );
        dir.remove();
    }

    /// A workflow fixture by name and body.
    fn workflow(name: &str, text: &str) -> WorkflowText {
        WorkflowText {
            name: name.to_owned(),
            text: text.to_owned(),
        }
    }

    /// A manifest fixture with one toolchain pin and one target row.
    ///
    /// # Panics
    ///
    /// When the fixture does not parse.
    fn manifest() -> Manifest {
        crate::manifest::parse(
            r#"
gate = []

[toolchain]
ziglang = "0.14.1"

[[target]]
target = "x86_64-pc-windows-msvc"
rustflags = "-C target-cpu=x86-64-v3"

[classification]
rust = '\.rs$'
dep = '^Cargo\.lock$'
infra = '^scripts/'
"#,
        )
        .unwrap()
    }

    /// Two pins of one action across two files is one finding naming both.
    ///
    /// # Panics
    ///
    /// When the pin pattern does not compile, or `actions/checkout` pinned at
    /// two SHAs in `a.yml` and `b.yml` does not give exactly one finding naming
    /// the action and `a.yml`.
    #[test]
    fn two_pins_of_one_action_are_named() {
        let first = workflow(
            "a.yml",
            "      - uses: actions/checkout@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa # v7\n",
        );
        let second = workflow(
            "b.yml",
            "      - uses: actions/checkout@bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb # v7\n",
        );
        let issues = check_action_pins(&[first, second]).expect("regex");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues
                .first()
                .is_some_and(|line| line.contains("actions/checkout") && line.contains("a.yml")),
            "the finding names the action and the first file: {issues:?}"
        );
    }

    /// A zig version that disagrees with the manifest is named with both
    /// values.
    ///
    /// # Panics
    ///
    /// When the patterns do not compile, or `ziglang==0.13.0` against the
    /// manifest's `0.14.1` does not give exactly one finding naming both
    /// versions.
    #[test]
    fn a_toolchain_version_off_the_manifest_is_named() {
        let stale = workflow(
            "release.yml",
            "          python3 -m pip install --user \"ziglang==0.13.0\"\n",
        );
        let issues = check_toolchain(&manifest(), &[stale]).expect("regex");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues
                .first()
                .is_some_and(|line| line.contains("0.13.0") && line.contains("0.14.1")),
            "the finding names both versions: {issues:?}"
        );
    }

    /// A version a job downloads by, in its `env:`, is held to the manifest
    /// like one it installs by: an old `CARGO_VET_VERSION` is named, the
    /// manifest's `cargo-deny` in either spelling is not.
    ///
    /// # Panics
    ///
    /// When the patterns do not compile, or the findings are not exactly one,
    /// naming `0.10.2` and the manifest's `0.10.3`.
    #[test]
    fn a_downloaded_version_off_the_manifest_is_named() {
        let mut manifest = manifest();
        manifest
            .toolchain
            .insert(String::from("cargo-vet"), String::from("0.10.3"));
        manifest
            .toolchain
            .insert(String::from("cargo-deny"), String::from("0.22.0"));
        let ci = workflow(
            "ci.yml",
            "    env:\n      CARGO_DENY_VERSION: 0.22.0\n      CARGO_VET_VERSION: \"0.10.2\"\n\
             run: cargo install --locked cargo-deny@0.22.0\n",
        );
        let issues = check_toolchain(&manifest, &[ci]).expect("regex");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues
                .first()
                .is_some_and(|line| line.contains("0.10.2") && line.contains("0.10.3")),
            "the finding names both versions: {issues:?}"
        );
    }

    /// A matrix row with the wrong flags, a job whose RUSTFLAGS disagree
    /// with the target it builds, and a native target-cpu are all named;
    /// a job that matches is not.
    ///
    /// # Panics
    ///
    /// When the patterns do not compile, or the findings are not exactly three:
    /// the matrix row with `x86-64-v2`, the `preview` job that builds the
    /// Windows target under other flags, and its `target-cpu=native`. A finding
    /// for the `good` job, whose flags match the manifest, makes four and fails
    /// it too.
    #[test]
    fn target_flags_are_checked_in_matrix_rows_and_jobs() {
        let text = "jobs:\n  build:\n    strategy:\n      matrix:\n        include:\n          - target: x86_64-pc-windows-msvc\n            rustflags: \"-C target-cpu=x86-64-v2\"\n  preview:\n    env:\n      RUSTFLAGS: \"-C target-cpu=native\"\n    steps:\n      - run: cargo build --target x86_64-pc-windows-msvc\n  good:\n    env:\n      RUSTFLAGS: \"-C target-cpu=x86-64-v3\"\n    steps:\n      - run: cargo build --target x86_64-pc-windows-msvc\n";
        let issues = check_targets(&manifest(), &[workflow("w.yml", text)]).expect("regex");
        assert_eq!(issues.len(), 3, "{issues:?}");
        assert!(
            issues.iter().any(|line| line.contains("matrix row")),
            "the matrix row finding is present: {issues:?}"
        );
        assert!(
            issues.iter().any(|line| line.contains("job `preview`")),
            "the job finding is present: {issues:?}"
        );
        assert!(
            issues.iter().any(|line| line.contains("target-cpu=native")),
            "the native target-cpu finding is present: {issues:?}"
        );
    }

    /// A profile the nextest config lacks is named once per workflow.
    ///
    /// # Panics
    ///
    /// When the pattern does not compile, or the findings are not exactly one
    /// naming `gone`: a finding for the defined `ci` profile, for `rustup`'s
    /// `--profile minimal`, or for the second mention of `gone` fails it.
    #[test]
    fn an_unknown_nextest_profile_is_named_once() {
        let profiles: BTreeSet<String> = core::iter::once(String::from("ci")).collect();
        let text = "run: cargo nextest run --profile ci
run: cargo nextest run \\
  --profile gone
run: cargo nextest run --profile gone
rustup toolchain install nightly --profile minimal
";
        let issues = check_profiles(&profiles, &[workflow("w.yml", text)]).expect("regex");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert!(
            issues.first().is_some_and(|line| line.contains("gone")),
            "the finding names the missing profile: {issues:?}"
        );
    }
}
