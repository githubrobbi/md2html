// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Property 11: each job installs the parts of the toolchain it uses, and
//! nothing else.
//!
//! rustup installs whatever `rust-toolchain.toml` lists in every job that
//! touches the toolchain, so a component or a target listed there is paid
//! for by every job, the many that never use it included (20 to 30 seconds
//! a job, measured on the runs of PR #47). The file therefore names the
//! channel and the minimal profile alone, and each job adds what it uses
//! (`rustup component add clippy`, `rustup target add <triple>`). This
//! property holds both sides:
//!
//! - `rust-toolchain.toml` has a `channel`, `profile = "minimal"`, and no
//!   `components` or `targets`;
//! - in every job of every workflow, a component its commands use is one it
//!   installs, and a component it installs is one they use: `cargo fmt` uses
//!   `rustfmt`, `cargo clippy` `clippy`, `cargo miri` `miri` and `rust-src`,
//!   `cargo careful` `rust-src`, `cargo llvm-cov` `llvm-tools`, and the action
//!   `github/codeql-action/init` `rust-src` (its extractor reads the standard
//!   library's sources);
//! - a target a job builds for (`--target <triple>`) is one it installs or the
//!   host of its runner, and a target it installs is one it builds for. A
//!   target named by a variable (`"$TARGET"`, a matrix value) pairs with an
//!   install named by a variable.
//!
//! Like the other properties it only reads: it can fail a push, never
//! rewrite a file.

use alloc::collections::BTreeSet;
use core::str::SplitWhitespace;
use std::path::Path;

use anyhow::{Context as _, Result};
use regex::{NoExpand, Regex};
use serde::Deserialize;

use crate::consistency::WorkflowText;

/// What each cargo subcommand needs beside the minimal profile.
const SUBCOMMAND_NEEDS: [(&str, &[&str]); 5] = [
    ("fmt", &["rustfmt"]),
    ("clippy", &["clippy"]),
    ("miri", &["miri", "rust-src"]),
    ("careful", &["rust-src"]),
    ("llvm-cov", &["llvm-tools"]),
];

/// What an action needs beside the minimal profile.
const ACTION_NEEDS: [(&str, &[&str]); 1] = [("github/codeql-action/init", &["rust-src"])];

/// `rust-toolchain.toml`, as far as this property reads it.
#[derive(Deserialize)]
struct ToolchainFile {
    /// The `[toolchain]` table.
    toolchain: Toolchain,
}

/// The keys of `[toolchain]` this property holds.
#[derive(Deserialize)]
struct Toolchain {
    /// The pinned channel.
    channel: Option<String>,
    /// The rustup profile every install starts from.
    profile: Option<String>,
    /// Components every install would add.
    components: Option<Vec<String>>,
    /// Targets every install would add.
    targets: Option<Vec<String>>,
}

/// A target triple as a workflow writes it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    /// Written out (`x86_64-apple-darwin`).
    Named(String),
    /// Named by a variable (`"$TARGET"`, `${{ matrix.target }}`).
    Variable,
}

impl Target {
    /// The target a command-line value names; quotes are not part of it.
    fn of(value: &str) -> Self {
        let bare = value.trim_matches(['"', '\'']);
        if bare.contains('$') {
            Self::Variable
        } else {
            Self::Named(bare.to_owned())
        }
    }

    /// How a finding names it.
    fn label(&self) -> &str {
        match self {
            Self::Named(name) => name,
            Self::Variable => "the target a variable names",
        }
    }
}

/// The parts of the toolchain beyond the minimal profile, as one job
/// installs them or uses them.
#[derive(Default)]
struct Parts {
    /// Components (`clippy`, `rust-src`).
    components: BTreeSet<String>,
    /// Targets.
    targets: BTreeSet<Target>,
}

/// One job of a workflow, read for this property.
#[derive(Default)]
struct Job {
    /// The job key.
    name: String,
    /// Its `runs-on:` value.
    runner: String,
    /// What its `run:` steps install.
    installs: Parts,
    /// What its `run:` steps and its actions use.
    uses: Parts,
}

/// The patterns a workflow is read with.
struct Patterns {
    /// A job key, two spaces in.
    job_key: Regex,
    /// A job's runner.
    runs_on: Regex,
    /// A step's action.
    action: Regex,
    /// A `run:` key and what follows it.
    run: Regex,
    /// A cargo subcommand that needs a component.
    subcommand: Regex,
    /// A `${{ ... }}` expression, read as one word.
    expression: Regex,
}

impl Patterns {
    /// Compiles the patterns.
    ///
    /// # Errors
    ///
    /// A pattern does not compile; the message is the `regex` crate's parse
    /// error. They are fixed literals, so this happens only if one is edited
    /// into an invalid pattern.
    fn new() -> Result<Self> {
        Ok(Self {
            job_key: Regex::new(r"^  ([A-Za-z0-9_-]+):\s*$")?,
            runs_on: Regex::new(r##"^    runs-on:\s*"?([^"#]*?)"?\s*$"##)?,
            action: Regex::new(r"^\s*(?:-\s+)?uses:\s*([^@\s]+)@")?,
            run: Regex::new(r"^(\s*)(?:-\s+)?run:\s*(.*?)\s*$")?,
            subcommand: Regex::new(
                r"(?:^|\s)cargo\s+(?:\+\S+\s+)?(?:xwin\s+)?(fmt|clippy|miri|careful|llvm-cov)(?:\s|$)",
            )?,
            expression: Regex::new(r"\$\{\{[^}]*\}\}")?,
        })
    }
}

/// Runs property 11: the toolchain file and every workflow's jobs.
///
/// # Errors
///
/// `rust-toolchain.toml` cannot be read (`read <path>`) or parsed (`parse
/// <path>`), or a pattern does not compile (see [`Patterns::new`]). A file or
/// a job that breaks the property is a finding in the list, not an error.
pub(crate) fn check(toolchain_file: &Path, workflows: &[WorkflowText]) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(toolchain_file)
        .with_context(|| format!("read {}", toolchain_file.display()))?;
    let mut issues =
        check_file(&text).with_context(|| format!("parse {}", toolchain_file.display()))?;
    let patterns = Patterns::new()?;
    for workflow in workflows {
        for job in jobs_of(&patterns, &workflow.text) {
            issues.extend(job_issues(&workflow.name, &job));
        }
    }
    Ok(issues)
}

/// What `rust-toolchain.toml` names beyond the channel and the minimal
/// profile.
///
/// # Errors
///
/// The text is not TOML with a `[toolchain]` table whose keys have the types
/// rustup reads; the message is the `toml` crate's.
fn check_file(text: &str) -> Result<Vec<String>> {
    let file: ToolchainFile = toml::from_str(text)?;
    let toolchain = file.toolchain;
    let mut issues = Vec::new();
    let head = "property 11 (lean toolchain): rust-toolchain.toml";
    if toolchain.channel.is_none() {
        issues.push(format!("{head} names no channel"));
    }
    if toolchain.profile.as_deref() != Some("minimal") {
        issues.push(format!(
            "{head} must say `profile = \"minimal\"`: every job installs the profile, and \
             most use nothing beyond rustc, cargo and the standard library"
        ));
    }
    for (key, present) in [
        ("components", toolchain.components.is_some()),
        ("targets", toolchain.targets.is_some()),
    ] {
        if present {
            issues.push(format!(
                "{head} lists `{key}`: rustup installs them in every job; a job that uses one \
                 adds it (`rustup component add`, `rustup target add`), and a developer's \
                 machine gets them from `just toolchain-ensure`"
            ));
        }
    }
    Ok(issues)
}

/// The jobs of one workflow, each with what it installs and uses.
fn jobs_of(patterns: &Patterns, text: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let mut current: Option<Job> = None;
    let mut from_jobs = text.lines().skip_while(|line| line.trim_end() != "jobs:");
    from_jobs.next();
    let mut lines = from_jobs.peekable();
    while let Some(line) = lines.next() {
        if let Some(key) = patterns.job_key.captures(line).and_then(|hit| hit.get(1)) {
            jobs.extend(current.take());
            current = Some(Job {
                name: key.as_str().to_owned(),
                ..Job::default()
            });
            continue;
        }
        let Some(job) = current.as_mut() else {
            continue;
        };
        if let Some(runner) = patterns.runs_on.captures(line).and_then(|hit| hit.get(1)) {
            runner.as_str().clone_into(&mut job.runner);
        }
        if let Some(action) = patterns.action.captures(line).and_then(|hit| hit.get(1)) {
            needs_of_action(action.as_str(), &mut job.uses);
        }
        let Some(run) = patterns.run.captures(line) else {
            continue;
        };
        let base = run.get(1).map_or(0, |indent| indent.as_str().len());
        let value = run.get(2).map_or("", |hit| hit.as_str());
        let mut script = String::new();
        if value.starts_with(['|', '>']) {
            while let Some(body) =
                lines.next_if(|next| next.trim().is_empty() || indent_of(next) > base)
            {
                script.push_str(body);
                script.push('\n');
            }
        } else {
            script.push_str(value.trim_matches(['"', '\'']));
        }
        let words = patterns
            .expression
            .replace_all(&script, NoExpand("${{expression}}"));
        for command in shell_commands(&words) {
            read_command(patterns, &command, job);
        }
    }
    jobs.extend(current);
    jobs
}

/// Leading spaces of a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// The commands of a shell script: comments dropped, backslash
/// continuations joined, and each line cut at `|`, `;` and `&`.
fn shell_commands(script: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut joined = String::new();
    for raw in script.lines() {
        let line = raw.trim();
        if line.starts_with('#') {
            continue;
        }
        let code = line.split_once(" #").map_or(line, |(head, _comment)| head);
        if let Some(head) = code.strip_suffix('\\') {
            joined.push_str(head);
            joined.push(' ');
            continue;
        }
        joined.push_str(code);
        commands.extend(
            joined
                .split(['|', ';', '&'])
                .map(str::trim)
                .filter(|command| !command.is_empty())
                .map(str::to_owned),
        );
        joined.clear();
    }
    commands
}

/// Adds what one command installs or uses to its job.
fn read_command(patterns: &Patterns, command: &str, job: &mut Job) {
    let mut words = command.split_whitespace();
    if words.any(|word| word == "rustup") {
        rustup_installs(words, &mut job.installs);
        return;
    }
    if let Some(subcommand) = patterns
        .subcommand
        .captures(command)
        .and_then(|hit| hit.get(1))
    {
        for (name, needs) in SUBCOMMAND_NEEDS {
            if name == subcommand.as_str() {
                job.uses
                    .components
                    .extend(needs.iter().map(|need| (*need).to_owned()));
            }
        }
    }
    if command.split_whitespace().any(|word| word == "cargo") {
        // What follows a lone `--` is the program's, not cargo's.
        let mut flags = command.split_whitespace().take_while(|word| *word != "--");
        while let Some(flag) = flags.next() {
            if let Some(value) = flag.strip_prefix("--target=") {
                job.uses.targets.insert(Target::of(value));
            } else if flag == "--target"
                && let Some(value) = flags.next()
            {
                job.uses.targets.insert(Target::of(value));
            }
        }
    }
}

/// What an action's step needs.
fn needs_of_action(action: &str, uses: &mut Parts) {
    for (name, needs) in ACTION_NEEDS {
        if name == action {
            uses.components
                .extend(needs.iter().map(|need| (*need).to_owned()));
        }
    }
}

/// What a `rustup` command installs, from the words after `rustup`:
/// `component add <names>`, `target add <names>`, and the `--component`
/// and `--target` of `toolchain install`.
fn rustup_installs(mut words: SplitWhitespace<'_>, installs: &mut Parts) {
    let verb = (words.next(), words.next());
    if verb == (Some("component"), Some("add")) {
        installs
            .components
            .extend(positional(words).map(str::to_owned));
    } else if verb == (Some("target"), Some("add")) {
        installs.targets.extend(positional(words).map(Target::of));
    } else if verb == (Some("toolchain"), Some("install")) {
        while let Some(word) = words.next() {
            let (flag, inline) = word
                .split_once('=')
                .map_or((word, None), |(flag, value)| (flag, Some(value)));
            let is_component = flag == "--component" || flag == "-c";
            let is_target = flag == "--target" || flag == "-t";
            if !is_component && !is_target {
                continue;
            }
            let Some(value) = inline.or_else(|| words.next()) else {
                continue;
            };
            for name in value.split(',').filter(|name| !name.is_empty()) {
                if is_component {
                    installs.components.insert(name.to_owned());
                } else {
                    installs.targets.insert(Target::of(name));
                }
            }
        }
    }
}

/// The names among the words of `rustup ... add`: what is not a flag, nor
/// the value of `--toolchain`.
fn positional(words: SplitWhitespace<'_>) -> impl Iterator<Item = &str> {
    let mut after_toolchain = false;
    words.filter(move |word| {
        if after_toolchain {
            after_toolchain = false;
            return false;
        }
        if *word == "--toolchain" {
            after_toolchain = true;
        }
        !word.starts_with('-')
    })
}

/// The target a runner builds for without adding one, when its name says.
fn host_of(runner: &str) -> Option<&'static str> {
    if runner.starts_with("ubuntu-") {
        Some(if runner.ends_with("-arm") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        })
    } else if runner.starts_with("windows-") {
        Some("x86_64-pc-windows-msvc")
    } else if runner == "macos-13" || runner.ends_with("-intel") {
        Some("x86_64-apple-darwin")
    } else if runner.starts_with("macos-") {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

/// The findings for one job: what it uses and does not install, and what
/// it installs and does not use.
fn job_issues(workflow: &str, job: &Job) -> Vec<String> {
    let head = format!(
        "property 11 (lean toolchain): {workflow} job `{}`",
        job.name
    );
    let mut issues = Vec::new();
    for component in job.uses.components.difference(&job.installs.components) {
        issues.push(format!(
            "{head} uses {component} but does not install it: add `rustup component add \
             {component}` after its toolchain install"
        ));
    }
    for component in job.installs.components.difference(&job.uses.components) {
        issues.push(format!(
            "{head} installs {component} but runs nothing that uses it"
        ));
    }
    let host = host_of(&job.runner).map(|triple| Target::Named(triple.to_owned()));
    for target in job.uses.targets.difference(&job.installs.targets) {
        if host.as_ref() != Some(target) {
            issues.push(format!(
                "{head} builds for {} but does not install it: add `rustup target add`",
                target.label()
            ));
        }
    }
    for target in job.installs.targets.difference(&job.uses.targets) {
        issues.push(format!(
            "{head} installs {} but builds nothing for it",
            target.label()
        ));
    }
    issues
}

#[cfg(test)]
mod tests;
