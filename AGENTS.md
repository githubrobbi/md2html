<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# AGENTS.md: rules for AI coding agents

Read this whole file before doing anything. Follow it literally.

**The first rule of this repository: no exceptions.** A lint finding, a
failing gate or a failing test is a real issue and gets a real fix. Apart
from the one exception the owner approved (`clippy::float_arithmetic` in
`crates/md2html/src/mermaid/geom.rs`, recorded in
`docs/policies/lint-posture.md`), there are no `#[allow(...)]` and no
`#[expect(...)]` attributes in this code, in any form, tests included, and
you add none. No other escape either: no
entry in an exceptions list, no `// AUDIT-OK` marker, no `|| true` or
`continue-on-error` that lets a failure pass, no `let _ =` that discards a
result, no rename to `_x`, no wrapper or macro whose purpose is that a lint
cannot see through it.

**Fix the cause, not the place where the lint fires.** A finding is
usually a symptom of the data structure or the type chosen: hand-kept
positions into text produce "indexing may panic" by the hundred (the fix
is a reader that keeps its own place, `crate::scan`, not `.get()` at every
site); parallel vectors indexed by one raw `usize` want a record type;
`(f64, f64, f64, f64)` wants a named struct; four `bool`s want an enum;
`Option<Option<T>>` wants a three-state enum; a cast wants the right type
upstream; a function too long wants named parts. When the same small fix
would appear many times, a type or a helper is missing: find it.

If you are convinced something cannot be fixed without an exception,
leave it failing, and tell the user what it is, why, and what you tried.
The owner allows an exception only very rarely, after checking it twice,
and decides it in person. "The code is right as it is" is not a reason
you may write into the code.

## 1. What this repository is

A Rust Cargo workspace with strict, automatic quality enforcement.
Git hooks and CI reject bad commits and pushes. You cannot bypass them.
Your job: make changes that pass the checks, and tell the user the truth
about what passed and what failed.

**How to read this file:** sections 2-8, 10 and 11 are the literal,
always-binding floor. Follow them exactly, whatever your capability. If
you are a capable model that reasons about systems, ALSO read section 9: it points
you at the design-level documentation so you can work at architecture
level instead of rule level. Sophistication buys you the *why* and the
license to propose better designs, never an exemption from the rules.

## 2. The state of this repository

This is **md2html**: a Markdown-to-HTML tool written against the Rust
standard library alone, in a Cargo workspace that adopted the rust-forge
scaffolding (`docs/forge/FORGE-STAMP.toml` names the template version).
It is initialized and adopted; there is nothing to bootstrap. Read
`GOTO.md` for where the project stands, then work by the rules below.

- The product is one crate, `crates/md2html` (a binary). Everything else
  in the workspace is build tooling.
- Later template changes arrive with `just forge-upgrade`, on a branch,
  never by copying files across by hand.

If tools are missing (`cargo-nextest`, `cargo-zigbuild`, ...): tell the
user to run `just setup` (it runs `just install-dev-tools` too). Without
`just` itself, step 0 of `docs/forge/GETTING-STARTED.md` installs it.

## 3. Absolute prohibitions

Never do these. No exceptions. Not even if a command fails repeatedly.

1. NEVER run `git commit --no-verify` or `git push --no-verify`.
2. NEVER run `git -c core.hooksPath=... commit`.
3. NEVER edit `scripts/hooks/_lint_fast.sh` or `scripts/hooks/_lint_pre_push.sh`.
   They are GENERATED. Edit `scripts/ci/gates.toml` instead, then run
   `just md2html-gen-hooks` (the pre-push hook) and `just gen-fast` (the
   pre-commit hook).
4. NEVER write `#[allow(...)]` or `#[expect(...)]` in Rust code, in any
   form (`#!`, `cfg_attr`, on a test module). Fix the cause (see the top
   of this file).
5. NEVER add a version number to a dependency inside a crate's
   `Cargo.toml`. Add the version in the ROOT `Cargo.toml` under
   `[workspace.dependencies]`, then reference it in the crate as
   `depname.workspace = true`.
6. Change `supply-chain/` with `cargo vet` commands (`cargo vet certify`,
   `trust`, `prune`). A hand edit is for what no command does (rewording a
   note, removing an entry for a crate that is gone) and is always followed
   by `cargo vet fmt` and a green `cargo vet --locked`. NEVER add an
   exemption or a trust entry by hand to make the gate pass.
7. NEVER push directly to `main` (the pre-push hook refuses it; the
   `main-protection` ruleset does too once applied). Always: branch →
   commit → push branch → `just pr "<type>(<scope>): <subject>"` →
   `just merge <PR-number>`. NEVER call `gh pr merge` directly, with ANY
   flags: on a plan without required checks even `--auto` merges
   instantly, before a single check has run. `just merge` watches every
   check to completion, re-reads the head SHA so a mid-watch push
   restarts the wait, requires the `PR Fast CI / required` aggregate to
   be PRESENT and green, and only then merges. It has no bypass flag on
   purpose.
8. NEVER delete or weaken a lint, gate, or workflow to make an error go
   away. Fix the code instead. If you believe a gate is wrong, STOP and
   ask the user.

## 4. The work loop

After EVERY code change, before telling the user anything is done:

```
just check        # fast: compile, format, file size (seconds); `just lint-ci` for clippy
just test         # run the test suite
just go           # the full validation lane: the definition of "done"
```

If `just go` prints green, the change is done. If it prints a failing gate,
find the gate name in section 6 and apply the exact fix. Do not claim
success while anything is red.

Committing (the hooks will run automatically; that is normal):

```
git switch -c feat/short-description
git add -A
git commit -m "feat: what changed"     # types: feat fix perf refactor docs test build ci chore style revert
git push -u origin feat/short-description
just pr "feat: what changed"      # title validated locally first
just merge <PR-number>            # watches ALL checks, then merges; never raw gh pr merge
```

Before pushing a branch for a PR, `just pre-pr` rehearses the whole PR
tier locally (minutes here instead of paid minutes on a red PR).

If a hook rejects the commit or push, that is the system working. Read the
gate name it prints, fix per section 6, retry the same command.

## 5. Writing Rust in this repo: hard rules

- No `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` in
  production code (`src/`). Return `Result` instead. Tests MAY use
  `unwrap`/`expect`.
- EVERY item needs a doc comment (`///` or `//!`), including private ones.
- Every new source file starts with these two lines:
  ```
  // SPDX-License-Identifier: MIT OR Apache-2.0
  // Copyright (c) 2026 Sky, LLC
  ```
- Public functions that can fail document an `# Errors` section.
  Public APIs get a doc-test example.
- Files stay under 800 lines. Split modules instead of growing files.
- Copy the existing style: look at `crates/md2html/src/text.rs` and
  `sanitize.rs` before writing product code, `scripts/dev-tools/src/lib.rs`
  before tooling code.
- All code meets the whole lint table in the root `Cargo.toml`, with no
  exception anywhere. Read `crates/md2html/src/scan.rs` before writing
  code that reads text or a slice front to back.

## 6. When a gate fails: exact fixes

| Gate name printed | Run / do exactly this |
| --- | --- |
| `fmt` | `just fmt` |
| `lint-prod`, `lint-tests`, `lint-ci`, `lint-ci-no-default` | Read the clippy message, find the structure that causes it, fix that. Never an `#[allow]` or `#[expect]` |
| `lint-ci-windows`, `lint-ci-linux-zig`, `lint-ci-mac-intel` | The same clippy, for another system: run the recipe of the same name (`just lint-ci-windows`, ...); a missing cross tool is `just install-dev-tools` |
| `cargo-check`, `test-build` | The code or a test does not compile: `cargo check --workspace --all-targets --all-features --locked` shows where |
| `tests`, `smoke`, `doc-tests` | `just test` (and `just test-doc`), fix the failing test. The pre-push hook prints `tests` for `test-build` too: a test that does not compile |
| `rustdoc` | `just rustdoc`; fix the doc comment it names (a broken link, a bad code block) |
| `msrv` | md2html must build and pass its tests with its minimum Rust (`rust-version` in `crates/md2html/Cargo.toml`): `just msrv`, then write it with what that Rust has, or fix what its standard library does differently. Ask the user before raising the minimum |
| `anti-pattern` | An `#[allow]`/`#[expect]`, lossy decoding or a discarded write: fix the cause (the top of this file); `bash scripts/ci/anti_pattern_gate.sh` names each place |
| `gate-tests` | A shell gate no longer does what its test says: `bash scripts/ci/run_gate_tests.sh` names the case; fix the gate (or the test, when the rule itself changed) |
| `shellcheck` | Fix the script or recipe line it names (quote the variable, use an array for a list of words); never a `# shellcheck disable=`; `python3 scripts/ci/check_shell.py` lists each |
| `actionlint` | Fix the workflow it names; `actionlint` locally shows the same (with shellcheck over the `run:` blocks) |
| `zizmor` | Fix the workflow it names at the cause it gives (`zizmor --offline --strict-collection .` shows the same; its `help` line links the audit's page). Never a `# zizmor: ignore` or a `zizmor.yml` that turns an audit off |
| `entities-drift` | The table of named character references is generated: never edit `crates/md2html/src/text/entity/names*`; `just entities` writes it again |
| `word-drift` | The table of the characters a heading's anchor keeps is generated: never edit `crates/md2html/src/text/word.rs`; `just word-table` writes it again from `scripts/unicode/DerivedGeneralCategory.txt` |
| `typos` | Fix the spelling. Real jargon: add to `.typos.toml` under `[default.extend-words]` |
| `taplo` | `taplo fmt` |
| `reuse` | Add the 2-line SPDX header from section 5 to the new file |
| `file-size` | Split the file; keep every piece under 800 lines |
| `deny` | `cargo deny check`; usually a new dependency's license; ask the user before changing `deny.toml` |
| `vet` | `cargo vet` and follow its printed suggestions |
| `vet-audit-discipline` | A crate's version changed without its audit: `just vet-bump CRATE NEW OLD`, or `cargo vet certify` with the evidence (`docs/SUPPLY-CHAIN.md`, chapter 5) |
| `vet-fmt` | `cargo vet fmt` |
| `machete` | Remove the unused dependency from that crate's `Cargo.toml` |
| `gates-drift`, `hooks-drift`, `fast-drift` | You edited a generated hook or `gates.toml` inconsistently. Revert hook edits, edit `scripts/ci/gates.toml`, run `just md2html-gen-hooks` and `just gen-fast` |
| `forge-stamp` | `docs/forge/TEMPLATE_VERSION` and `docs/forge/FORGE-STAMP.toml` disagree: never edit either by hand; `just forge-upgrade` changes both |
| `doc-references` | A document names a path, a `just` recipe or a gate that is not there, or this table misses a gate: fix the document (or this table); `python3 scripts/ci/check_doc_references.py` lists each |
| `versions` | The version disagrees between `Cargo.toml`/`Cargo.lock`, the capabilities pages and `CHANGELOG.md`'s newest release (section 11), or a document states another minimum Rust than `rust-version` (the README, `docs/SUPPLY-CHAIN.md`, the release policy): make them agree; for the pages, `python3 scripts/ci/versions.py stamp X.Y.Z`. Never change the version outside a release (`docs/policies/release_policy.md`) |
| `workflow-drift` | Make `.github/workflows/pr-fast.yml` match `scripts/ci/gates.toml`; the error names the mismatch |
| `manifest-drift` | A crate `Cargo.toml` violates inheritance. Use `field.workspace = true` and `dep.workspace = true` (see prohibition 5) |
| `commit-subjects` | Reword: `git commit --amend -m "feat: ..."` with a type from section 4 |
| `commit-signatures` | Tell the user to run `just setup-signing` (needs their passphrase) |

Checks that are not gates of `gates.toml` but fail the same way:

- **`--check` on the repository's pages** (CI's `test` job): run the
  command of section 11 and fix what it prints.
- **Browser suite** (CI's `browser` job): `node audit.js`, `node
  interact.js`, `node live.js`, `node nesting.js`, `node handbook.js` in
  `tests/browser`.
- **EPUB check** (CI's `epub` job): `just epubcheck`.
- **Minimum Rust** (CI's `msrv` job): as the `msrv` gate.
- **PR title** (`commitlint.yml`): the title of a pull request follows the
  commit types of section 4; `just pr` checks it before it opens one.

## 7. Adding things

- **New crate**: follow `docs/forge/COMPONENTS.md` → `component:new-crate`. Copy
  `scripts/dev-tools` as the model. Add the crate to `[workspace] members`
  in the root `Cargo.toml` AND a `release = false` block in
  `release-plz.toml`. Then `cargo run -p md2html-manifest-audit` must pass.
- **New dependency of md2html itself**: there are none, and that is a
  promise the README makes. STOP and ask the user before adding one.
- **New dependency of the tooling**: version in root `[workspace.dependencies]`, then
  `depname.workspace = true` in the crate. Then run `just go`; the `vet`
  and `deny` gates will tell you if the supply chain needs attention.
- **Enable releases / publishing / winget**: these are switched-off "lanes".
  Do not improvise. Follow the runbook in `docs/forge/COMPONENTS.md` for the specific
  lane and confirm with the user first.

## 8. Where things are

| Path | What it is |
| --- | --- |
| `crates/md2html` | The product: one binary, no dependencies. `src/page/` holds the page's own CSS and JavaScript; `crates/md2html/tests/commonmark/` the specification's examples, `crates/md2html/tests/command_line/` the tests that run the built program |
| `docs/ARCHITECTURE.md` | md2html's modules in layers and the directions between them; `src/layer_tests.rs` fails when a use goes up a layer or closes a circle |
| `crates/md2html-version` | Version machinery for the tooling crates |
| `scripts/dev-tools/` | `just use-local` and `just release-local` |
| `tests/browser/` | The rendered pages in Chrome: accessibility audit, click-through, live reload (Node, run in CI) |
| `docs/capabilities.md` | The capabilities page and its sixteen translations (`docs/capabilities.<lang>.md`) |
| `docs/command-line.md` | The command line's contract: options, settings, exit codes, environment variables; `cli/contract_tests.rs` holds it to the code |
| `docs/TESTING.md` | Each kind of test, what it guards, where it runs (commit, push, pull request, weekly, by hand) and how to add one |
| `scripts/ci/gates.toml` | Single source of truth for every quality gate |
| `scripts/ci-pipeline/` | The `just go` / `just ship` engine |
| `docs/forge/COMPONENTS.md` | Runbooks for growing the project (crates, lanes) |
| `docs/forge/GETTING-STARTED.md` | Human onboarding guide (send users here) |
| `GOTO.md` | The living project-state doc: where the flow stands, what's next, which dormant lane does what. Read it at session start; update it in the same PR whenever project state changes (lane enabled, milestone done, decision made) |
| `.config/nextest.toml` | Test-runner profiles |
| `docs/policies/` | The ten policies: the reasoning behind the lint rules and the rest of the rules (section 9 lists them) |
| `docs/MAINTAINING.md` | Taking the project over, deprecations, and every recurring chore (the toolchain, the pinned tools and their SHA-256, epubcheck, the runners) with how to do it and when it was last done |
| `docs/CONTRIBUTING.md` | For people: how a change lands, and the reading order of the documents |

## 9. Escalation path for capable agents

Everything above is the safety floor. If you can hold a design in your
head, this is where the repo actually lives:

| Read this | To operate at this level |
| --- | --- |
| `docs/policies/lint-posture.md` | Where the lints are set, which gate runs each, the weekly checks, and the table of findings and the structure that fixes each. Start here. To disagree with a rule, argue against its documented reason with the user, in the PR, never around it. |
| `docs/policies/panic_policy.md` | Why production code returns errors instead of panicking, and what to write instead. |
| `docs/policies/allocation_policy.md` | Why a needless copy costs, and the shapes that avoid it. |
| `docs/policies/type_policy.md` | Types that say what data is; when a trait or a generic earns its place. |
| `docs/policies/concurrency_policy.md` | The one place with threads (`--serve`), and the tooling's scoped threads and child processes with a time limit. |
| `docs/policies/platform_policy.md` | Why there is no unsafe code and no FFI, and what to do the day a crate needs it; every behaviour that differs between Linux, macOS and Windows, the one module that holds md2html's own rules (`system`), and the test that holds each on its system. |
| `docs/policies/hostile_input_policy.md` | Injection, file access, resource exhaustion, determinism: the bugs the compiler does not catch, and the tests that hold each. |
| `docs/policies/dependency_policy.md` | Why md2html has no dependencies; the rules for the tooling's crates and features. |
| `docs/policies/build_codegen_policy.md` | Build scripts, macros, generated files and the environment variables read. |
| `docs/policies/release_policy.md` | What a version number promises for the command line, the minimum Rust and when it may rise, and the release checklist (with what only a real run of the release workflow proves). |
| `scripts/ci/gates.toml` | The machine-readable source of truth for every gate: tiers, budgets, change-classification regexes. Gate changes are made HERE, then `just md2html-gen-hooks` and `just gen-fast` regenerate the hooks and the drift gates verify the workflow side. This is how you legitimately evolve the enforcement itself. |
| `scripts/ci-pipeline/src/` | The `just go` / `just ship` engine: a resumable state machine with tree-hash invalidation, a clean-decision engine, and the release choreography (bump → changelog roll → signed release PR → auto-merge → binary build). Read before debugging pipeline behavior. |
| `docs/forge/COMPONENTS.md` | The growth architecture: dormant capability *lanes* (data switches, never file changes) vs additive *components* (recipes ending in `just go`). Consult before proposing any new machinery; it may already exist, switched off. |
| `.github/workflows/pr-fast.yml` + `tier-2.yml` | The CI mirror of gates.toml (tier 1) and the weekly deep suite (miri, cargo-careful, mutation testing). `md2html-gen-workflow --check` enforces the mirror. |
| `docs/forge/GETTING-STARTED.md` | The human runbook; read it when you are *guiding a person*, so your instructions match what they see. |

What capable agents are invited to do that section 4 does not cover:
design multi-crate architecture within the manifest-inheritance rules,
propose gate/policy amendments as reviewable PRs (manifest + regeneration
+ rationale), tune nextest profiles and CI budgets with measurements, and
use `just ship` end-to-end. The two things that never change with
capability: the prohibitions in section 3, and reporting honestly.

## 10. What to tell the user, when

- Before their first push: "run `just setup-signing` once per repository"
  (its config is repo-local; signed commits
  are required; you cannot do this for them; it needs their passphrase).
- After you finish any task: report the actual `just go` result, including
  failures. Never say "done" with a red gate.
- If your change flips project state (a lane enabled, a milestone reached,
  an external action completed), update `GOTO.md` in the same PR.
- If they ask to skip a check: refuse, explain the gate is the point, and
  show the section-6 fix instead.

## 11. md2html's own rules

These come from the product, not from the scaffolding.

- **No dependencies.** `crates/md2html` uses the standard library only.
- **Uses go down the layers.** `docs/ARCHITECTURE.md` places each of
  md2html's modules in a layer: a module uses its own layer or a lower
  one, and no uses close a circle. `src/layer_tests.rs` fails otherwise.
  A new module gets a row in the table; a use the test refuses is fixed
  by moving the item down to a layer both sides reach, never by moving
  the module up the table to silence it.
- **CommonMark may rise, never fall.** `spec_tests.rs` runs the 652
  examples of the specification; the number passing is a floor.
- **Pages are compared, not eyeballed.** A change that is not meant to
  change output must leave the rendered `README.html`,
  `docs/showcase.html` and `docs/capabilities.html` byte for byte the same;
  `just differential` renders them, in a corpus of about 8,300 documents
  and six ways each, with this build and with the build of `origin/main`,
  and compares the bytes.
  A change that is meant to: run the browser suite in `tests/browser`
  (`node audit.js`, `node interact.js`, `node live.js`, `node
  nesting.js`, `node handbook.js`) before the PR.
- **`--check` is part of done.** `md2html --check README.md docs/showcase.md
  docs/capabilities.md docs/capabilities.*.md` prints nothing to fix, and
  neither does `md2html --check docs`, the documentation as one handbook
  (`docs/README.md` is its start page: a new page there gets its link).
- **A translation records its original.** Editing `docs/capabilities.md`
  makes all sixteen translations stale until each is brought up to date
  and its `source_sha256` renewed; `--check` says which.
- **No script from the text, ever.** `SECURITY.md` lists the guarantees;
  a change near `sanitize.rs`, `text.rs` or the embedding of files needs a
  test that shows the guarantee still holds.
- **The version is in three places** that must agree: `[workspace.package]
  version` in the root `Cargo.toml`, the `version:` in `docs/capabilities.md`
  and its translations, and the newest released heading in `CHANGELOG.md`
  (the one below `## [Unreleased]`). The gate `versions` holds them
  together; `just ship` writes all three, and
  `docs/policies/release_policy.md` has the checklist for a release.
