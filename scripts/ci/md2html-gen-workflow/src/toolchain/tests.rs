// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Tests of property 11: the toolchain file's shape, and each job's
//! components and targets against what its commands use.

use super::{Patterns, check_file, job_issues, jobs_of};

/// The findings for the jobs of one workflow text.
///
/// # Panics
///
/// When the patterns do not compile.
fn findings(text: &str) -> Vec<String> {
    let patterns = Patterns::new().expect("the patterns compile");
    jobs_of(&patterns, text)
        .iter()
        .flat_map(|job| job_issues("w.yml", job))
        .collect()
}

/// The channel with the minimal profile is clean; the file as it was
/// before (default profile, components, targets) has a finding for each.
///
/// # Panics
///
/// When a fixture does not parse, the lean file has a finding, or the old
/// one has not exactly three naming the profile, the components and the
/// targets.
#[test]
fn the_toolchain_file_names_the_channel_and_the_minimal_profile_alone() {
    let lean = "[toolchain]\nchannel = \"nightly-2026-11-23\"\nprofile = \"minimal\"\n";
    assert!(
        check_file(lean).expect("parses").is_empty(),
        "the lean file is clean"
    );
    let full = "[toolchain]\nchannel = \"nightly-2026-11-23\"\ncomponents = [\"clippy\"]\n\
                targets = [\"x86_64-apple-darwin\"]\nprofile = \"default\"\n";
    let issues = check_file(full).expect("parses");
    assert_eq!(issues.len(), 3, "{issues:?}");
    for word in ["profile", "`components`", "`targets`"] {
        assert!(
            issues.iter().any(|line| line.contains(word)),
            "a finding names {word}: {issues:?}"
        );
    }
    let no_channel = check_file("[toolchain]\nprofile = \"minimal\"\n").expect("parses");
    assert_eq!(no_channel.len(), 1, "the missing channel: {no_channel:?}");
}

/// A job that runs clippy without adding it is named, the same job with
/// `rustup component add clippy` is clean, and a component no command
/// uses is named too.
///
/// # Panics
///
/// When the findings differ: one for the missing clippy, none once it is
/// added, one for the unused miri.
#[test]
fn a_component_is_installed_by_the_job_that_uses_it_and_by_no_other() {
    let missing = "jobs:\n  clippy:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                   - run: rustup toolchain install --no-self-update\n      \
                   - run: cargo clippy --workspace -- -D warnings\n";
    let issues = findings(missing);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(
        issues
            .first()
            .is_some_and(|line| line.contains("job `clippy` uses clippy")),
        "the finding names the job and the component: {issues:?}"
    );
    let added = missing.replace(
        "--no-self-update\n",
        "--no-self-update\n      - run: rustup component add clippy\n",
    );
    assert!(findings(&added).is_empty(), "added, it is clean");
    let unused = "jobs:\n  tests:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                  - run: rustup component add miri\n      - run: cargo test\n";
    let unused_issues = findings(unused);
    assert_eq!(unused_issues.len(), 1, "{unused_issues:?}");
    assert!(
        unused_issues
            .first()
            .is_some_and(|line| line.contains("installs miri but runs nothing")),
        "the unused component is named: {unused_issues:?}"
    );
}

/// Only the commands of `run:` steps count, and the `CodeQL` action's need.
///
/// A step's name and a shell comment that mention a subcommand do not
/// count; a block's continued lines do; `toolchain install --component
/// a,b` installs; and the `CodeQL` action needs `rust-src`.
///
/// # Panics
///
/// When the canary-shaped job or the commented one has a finding, or the
/// `CodeQL` job has not exactly one, for `rust-src`.
#[test]
fn commands_are_read_from_run_steps_only() {
    let canary = "jobs:\n  canary:\n    runs-on: ${{ matrix.runner }}\n    steps:\n      \
                  - name: cargo clippy (floating nightly)\n        run: |\n          \
                  # cargo careful would need rust-src\n          \
                  rustup toolchain install nightly \\\n            --profile minimal \\\n            \
                  --component clippy,rustfmt \\\n            --no-self-update\n          \
                  cargo +nightly clippy --version\n          cargo +nightly fmt --check\n";
    assert!(findings(canary).is_empty(), "{:?}", findings(canary));
    let codeql = "jobs:\n  analyze:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                  - uses: github/codeql-action/init@aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa # v4\n";
    let issues = findings(codeql);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(
        issues
            .first()
            .is_some_and(|line| line.contains("uses rust-src")),
        "the action's need is named: {issues:?}"
    );
}

/// A target is installed by the job that builds for it, unless it is the
/// runner's own.
///
/// A matrix target pairs with a matrix install; what follows `--` is the
/// program's, not cargo's; and a target nothing builds for is named.
///
/// # Panics
///
/// When the clean jobs have a finding, or the others not exactly the two
/// expected: the Intel macOS build without its target, the unused musl
/// install.
#[test]
fn a_target_is_installed_where_it_is_built_unless_it_is_the_host() {
    let clean = "jobs:\n  native:\n    runs-on: windows-latest\n    steps:\n      \
                 - run: cargo build --target x86_64-pc-windows-msvc\n  \
                 cross:\n    runs-on: macos-latest\n    steps:\n      \
                 - run: rustup target add x86_64-apple-darwin\n      \
                 - run: cargo build --target aarch64-apple-darwin\n      \
                 - run: cargo build --target=x86_64-apple-darwin\n  \
                 matrix:\n    runs-on: ${{ matrix.os }}\n    steps:\n      \
                 - run: rustup target add ${{ matrix.target }}\n      \
                 - run: cargo build --target \"$TARGET\"\n  \
                 drift:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                 - run: cargo run -p md2html-gen-hooks -- --target pre-commit --check\n";
    assert!(findings(clean).is_empty(), "{:?}", findings(clean));
    let drifted = "jobs:\n  intel:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                   - run: cargo build --target x86_64-apple-darwin\n  \
                   musl:\n    runs-on: ubuntu-22.04\n    steps:\n      \
                   - run: rustup target add x86_64-unknown-linux-musl\n      \
                   - run: cargo build\n";
    let issues = findings(drifted);
    assert_eq!(issues.len(), 2, "{issues:?}");
    assert!(
        issues
            .iter()
            .any(|line| line.contains("job `intel` builds for x86_64-apple-darwin")),
        "the missing target is named: {issues:?}"
    );
    assert!(
        issues.iter().any(|line| line
            .contains("job `musl` installs x86_64-unknown-linux-musl but builds nothing")),
        "the unused target is named: {issues:?}"
    );
}
