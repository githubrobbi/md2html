<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Build scripts, macros, generated files and the environment

What runs at build time, what writes files the repository keeps, and what md2html reads from the environment. Each of these is code that a reader of `src/` does not see, so each is kept small and listed here.

## Build scripts

md2html has none. Four tooling crates have one, all the same: `scripts/ci-pipeline`, `scripts/ci/md2html-gen-hooks`, `scripts/ci/md2html-gen-workflow` and `scripts/ci/md2html-manifest-audit`. Each `build.rs` is `fn main() -> io::Result<()>`: it calls `md2html_version::emit_build_env(&mut out)` with its locked standard output, then writes its own `cargo:rerun-if-changed=build.rs`. `emit_build_env` writes the `cargo:rustc-env` lines that stamp the git commit, its date, the compiler, the target and the profile into the build (`MD2HTML_GIT_SHA`, `MD2HTML_COMMIT_DATE`, `MD2HTML_RUSTC`, `MD2HTML_TARGET`, `MD2HTML_PROFILE`) for the binary's `--version`, and declares what it reads (`cargo:rerun-if-changed`, `cargo:rerun-if-env-changed`), so a build is never stale. The git files it watches, `HEAD` and the branch file `HEAD` names, are found with `git rev-parse --git-path`, so a crate at any depth and a git worktree get the right ones. A field it cannot resolve is stamped `unknown`; a failure to write is returned, and fails the build.

A new build script does only what cannot be done in ordinary code, declares every file and variable it reads, and returns its errors like any other code. Checked in review.

## The version text

Each tooling binary answers `--version` (and `-V`; with `-v` or `--verbose` after it, the long build fingerprint) through `md2html_version::requested_version!("name")`. The macro expands to `version_request(args, short, long) -> Option<String>` over the process arguments, with that binary's own short and long text. When it gives text, the binary's `main` writes it to standard output and returns; nothing exits the process from inside the library. Only the first argument counts, so a later `--version` stays an argument of the tool.

## Macros

| Macro | Where | Why a function cannot do it |
| --- | --- | --- |
| `put!` | `crates/md2html/src/text.rs` | It appends formatted text to a `String`; `format_args!` exists only as a macro. It gives back no result, by the owner's decision: appending to a `String` cannot fail ([lint-posture.md](lint-posture.md)). |
| `say!` | `scripts/ci-pipeline/src/out.rs` | The pipeline's line for people, with `format!`'s arguments, on a locked standard output; it returns the write's error for `?`. |
| `version_short!`, `version_long!`, `requested_version!` | `crates/md2html-version/src/lib.rs` | They read `env!` and `option_env!` in the crate that calls them, at its compile time, so the text is that binary's. The logic itself is the function `version_request`. |

A macro is written only where a function cannot do the job, and never to hide control flow or to keep code out of a lint's sight. Checked in review.

## Files the repository generates

| Tool | Reads | Writes or checks | Gate |
| --- | --- | --- | --- |
| `md2html-gen-hooks` | `scripts/ci/gates.toml` | writes `scripts/hooks/_lint_pre_push.sh` and `scripts/hooks/_lint_fast.sh` | `hooks-drift`, `fast-drift` |
| `md2html-gen-workflow` | `scripts/ci/gates.toml` | checks `.github/workflows/pr-fast.yml` and every other workflow, never writes them | `workflow-drift` |
| `md2html-manifest-audit` | the rules in `scripts/ci/md2html-manifest-audit/src/audit.rs` | checks every member's `Cargo.toml` | `manifest-drift` |
| `scripts/ci/check_gates_drift.sh` | `scripts/ci/gates.toml` | checks that the hooks and `pr-fast.yml` name the same gates | `gates-drift` |

A generated file is changed through its source, never by hand ([AGENTS.md](../../AGENTS.md), prohibition 3); a new generator comes with a gate that fails when its output and its source disagree. `scripts/ci-pipeline` writes no file the repository keeps: it runs the gates and the release.

## The environment

md2html reads:

| Variable | What it does | Read in |
| --- | --- | --- |
| `MD2HTML_CHROME` | The Chrome (or Chromium, or Edge) that `--pdf` prints with | `export/print.rs` |
| `PATH` | Where `--pdf` looks for Chrome otherwise | `export/print.rs` |
| `MD2HTML_GLOSSARY` | The shared glossary; otherwise `~/.config/md2html/glossary.md`, found through `HOME` | `document/glossary.rs` |
| `MD2HTML_GLOSSARY_URL` | Where a page's "all terms" link points | `document/glossary.rs` |
| `SOURCE_DATE_EPOCH` | The date on the page, so the same Markdown renders to the same bytes | `text.rs` (`now_secs`) |

The tests read `SPEC_FAILURES` (when set, the CommonMark conformance test lists the examples that fail) and `MD2HTML_TEST_RENDERS` (set only for the runs the determinism test starts, `tests/determinism.rs`).

The tooling reads:

| Variable | What it does | Read in |
| --- | --- | --- |
| `CARGO_TARGET_DIR`, `HOME` | Where the build's files are; `HOME` expands a leading `~` in it | `scripts/ci-pipeline/src/context.rs`, `scripts/dev-tools/src/lib.rs` |
| `CLICOLOR_FORCE` | Any value but `0` colours the pipeline's output even away from a terminal; `just/shared.just` exports `1` | `scripts/ci-pipeline/src/style.rs` |
| `NO_COLOR`, `CLICOLOR` | Set, or `CLICOLOR=0`: no colour, unless `CLICOLOR_FORCE` | `scripts/ci-pipeline/src/style.rs` |
| `MD2HTML_BIN_DIR`, `HOME`, `USERPROFILE`, `PATH` | Where `just use-local` installs the binaries, and whether that folder is on the path | `scripts/dev-tools/src/bin/install-bins.rs` |
| `RUSTC`, and the variables Cargo sets for a build script | The compiler and the target stamped into `--version` | `crates/md2html-version/src/lib.rs`, at build time |

Some names are written, not read: `just/shared.just` exports `RUSTC_WRAPPER` and `CARGO_INCREMENTAL` for every build it starts, and `scripts/ci-pipeline` sets `RUSTC_WRAPPER`, `CARGO_BUILD_JOBS` and `CARGO_TARGET_DIR` for the commands it runs.

The environment, like the command line, belongs to the person running md2html: it may name any file. A document cannot set an environment variable, and nothing a variable names widens what a document may read ([hostile_input_policy.md](hostile_input_policy.md)). A new variable gets a line in this table in the same pull request. Checked in review.
