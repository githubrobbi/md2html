<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# GOTO.md: where md2html stands, what's next, which switch does what

The **living state doc** for this project. `AGENTS.md` holds the rules and `docs/forge/GETTING-STARTED.md` the how-to; this file holds the *state*: where the project stands, the rhythm for daily work, how the code is held to the lint table, and the runbook for every dormant capability.

**Maintenance rule:** this file is a snapshot, not eternal truth. When a checkbox flips, a lane goes live, or the plan changes, update it **in the same PR**. A stale GOTO.md is worse than none.

---

## 1. Where we are (snapshot: 2026-11-24)

### The product

md2html 0.7.7: Markdown to one self-contained HTML page (`docs/capabilities.md` is the tour, in seventeen languages). One crate, `crates/md2html`, no dependencies, unsafe code forbidden. All 652 CommonMark examples pass; that number is a floor. Its twenty-one modules stand in nine layers, each using only its own or lower ones and none in a circle (`docs/ARCHITECTURE.md`); a test holds it. Every item is visible only as far as the code that uses it (private, its parent module, or the crate), and each module's root file is the one place the rest uses it through (its submodules are private; the layer test holds that too). A struct shows its fields only when it is a record; a value with a rule keeps them private behind its methods (`docs/policies/type_policy.md`). Markup is `text::Html`, apart from plain text, so nothing is escaped twice or not at all (a test finds every entry into the page escaped once); a setting with a rule is read once into its type, a heading's level is one to six, and a boolean that picked a way of working is an enum that names it. What the command line promises (options, settings, exit codes, environment variables, what is stable) is `docs/command-line.md`, and a test holds the page and the code to each other; what a version number promises for it, the minimum Rust and when it may rise and the release checklist are `docs/policies/release_policy.md`. A failure is seen, never turned into a quiet default: a folder that cannot be read stops the run, a file that is there but cannot be read is named as such, a setting or variable that cannot be used is a warning; a decision about a file name is made on the name as the file system has it, and a link never names a file under U+FFFD (`docs/policies/panic_policy.md`, hostile-input rule 6). The error strategy is by kind of crate: messages and exit codes in md2html, `anyhow` with context in the tooling binaries, typed errors in the two libraries. Traits and generics do work or are not there: md2html has two traits, each with its reason written down, the tooling none, `dyn` only where the standard library puts it, a sorted type has a total order and a default is a real value (`docs/policies/type_policy.md`). Speed is measured, not guessed: `just measure` times the stages of a render on the repository's documents and on a handbook with a glossary of 2000 terms, and a change made for speed or memory names the numbers it moved (`docs/policies/allocation_policy.md`). A page's time grows in proportion to its text, its glossary and its links included: a text is searched only for the terms it names, and its memory with its bytes, not its lines (the block reader reads a line from the text when it needs one and holds a few a level of nesting, a list item's blank lines as a count; a line borrows the text, a line's end is part of the text around it, a node is 32 bytes); tests hold such bounds by size (`docs/policies/hostile_input_policy.md`). The one threaded part, `--serve`, fits on a page (`docs/policies/concurrency_policy.md`): every thread has an owner that waits for it and a defined end, the server stops with nothing left running, the shared state is one `Site` without `Arc` or atomics, and a request is read by HTTP/1.1's rules or refused with the status that says why (400, 408, 414, 431, 505); tests over real connections hold the limits and the stop. Every child process the release pipeline starts has a time limit. Production code has no `cfg` on an operating system: where Linux, macOS and Windows differ, md2html takes the standard library's answer and keeps its own rules in one module (`system`: the home folder, a path as a `file:` URL, a relative path that stays in its folder), and a test holds each system's rule on that system, where CI runs it (`docs/policies/platform_policy.md`). How it is tested is one page (`docs/TESTING.md`): each kind of test, what it guards and where it runs. Tests run the built program for the command line's contract; a seeded generator holds every page to its invariants; the differential test is in the repository (`just differential` compares 8,300 documents rendered six ways with another build, byte for byte; `just generated-documents` and a weekly job fail on a crash or a hang); the sanitizer, the text helpers and the inline, block and document modules are mutation-tested weekly with every mutant caught.

### The machine

- [x] Adopted the rust-forge scaffolding (template 1.1.3) with `adopt.sh`; merged as PR #1 on 2026-11-22
- [x] Gate tools installed, git hooks wired (`core.hooksPath=scripts/hooks`)
- [x] Commit signing: GPG key `E406D32B4736D09F`, repo-local; `just doctor-signing` green
- [x] `just go` green (the definition of done for any change)
- [x] GitHub-side state (`bash scripts/ci/bootstrap-github.sh`): labels, every `LANE_*` variable `false`, merge commits only (squash and rebase off), auto-merge, delete-branch-on-merge, `tag-protection-v-prefix`
- [x] `main-protection` ruleset active: no deletion, no force push, signed commits, a pull request with every thread resolved, merge commits only, and the required checks (`scripts/ci/main-ruleset.json`)
- [x] The whole lint table at its own levels, held with one exception the owner approved (section 3)
- [x] `just use-local` and `just release-local`: md2html for six platforms from one machine
- [x] The audit store (`supply-chain/`) stands on its own: no import from another repository
- [x] Dependencies current (2026-11-23): toolchain `nightly-2026-11-23` (chosen by `just toolchain-sync`), the tooling's crates at their latest (28 crates since 2026-11-23, all fully audited, none exempted; no advisory ignored; each built with only the features it uses, so 27 are built), the one feature of the workspace (`build` in `md2html-version`) built, linted and tested with it on and off, every action at its latest release that is a week old, the browser tools at their latest
- [x] Secret scanning with push protection, Dependabot alerts and security updates, private vulnerability reporting: all on
- [x] Every EPUB md2html writes for the repository's documents passes epubcheck 5.4.0 with no error and no warning: CI's `epub` job checks the 19 books and the book of the balancing's test page (`tests/browser/balance.md`, with `--html`), `just epubcheck` the same locally
- [x] The enforcement checks itself (2026-11-24): every lint the toolchain offers was read and is either in the table or written into `docs/policies/lint-posture.md` with its reason; the change classes, the clippy flags and each pull-request job's command each have one source the drift gates hold (`workflow-drift` properties 9 and 10); the anti-pattern gate runs at every commit and has its own tests (`gate-tests`); every shell script, `just` recipe and workflow is read by a gate (`shellcheck`, `actionlint`); no gate passes without having checked; `just fmt` formats all a gate checks
- [x] Releases have written rules (2026-11-24): what a patch, a minor and a major number may change in the command line's contract, the minimum Rust and when it may rise, and a checklist from the changelog to the downloaded binary (`docs/policies/release_policy.md`). The version agrees in its three places by a gate (`versions`); `CHANGELOG.md` is in Keep a Changelog form, which `just ship`'s roll reads; `just ship --bump minor|major` raises more than the patch and stamps the capabilities pages; `release.yml` refuses a tag that disagrees with the version or a commit not on `main`, checks for an existing release on both paths, and writes its notes from the changelog without `envsubst`
- [x] Security and supply chain reviewed (2026-11-24): `cargo vet --locked` 28 fully audited, none exempted; `cargo deny check` clean, no advisory ignored; no future-incompatibility report; no `[patch]`; secret scanning with push protection, Dependabot alerts and security updates, private vulnerability reporting and CodeQL on, no open alert. md2html was read against every guarantee of `SECURITY.md`, area by area, with hostile input run against the built program and in Chrome; every way around a guarantee it found was closed with a test (`CHANGELOG.md`, Security), the three that needed a change of design among them (2026-11-24): the balancing holds md2html's own elements beside the author's and judges each start tag by HTML's rules, so no tag of the author's ends one of md2html's (Chrome reads every article as md2html nested it: `tests/browser/nesting.js`); memory grows with a text's bytes, not its lines, and no record is kept per line (16 MB of one-letter lines: 181 MB; of blank lines: 37 MB); before md2html's block the author's open phrasing (a `span`, an `em`, a `ruby`) is closed, so a book of such HTML is valid XHTML; and the small ones (Markdown's every extension refused as an output, a setting given twice named, `--embed` naming a folder or a pipe, `--serve` making no book, HTTP/1.0 without `Host` and port 80 answered). A link md2html makes to a file from its names escapes them by one rule (`text::PathUrl`), and `--serve` reads them back the same way
- [x] Keeping md2html is written down (2026-11-24): `docs/MAINTAINING.md` says how to take the project over, what needs an administrator on GitHub, and every recurring chore (weekly, every three months, yearly: the toolchain, the pinned tools and their SHA-256, epubcheck, the runners) with how to do each and when it was last done; `docs/policies/release_policy.md` has the rule for deprecating (nothing is deprecated today)
- [x] The workflows hold no more power than they use: every job has a timeout and its own write permissions, no checkout keeps its token unless it pushes, no `${{ }}` reaches a shell, no release restores a cache (zizmor 1.33.1: 85 findings to 3, the three in section 1's pending list)
- [x] Every generated file is made from what the repository holds and checked by a gate: the git hooks from `scripts/ci/gates.toml` (`hooks-drift`, `fast-drift`), the table of HTML's named character references from the WHATWG's list vendored in `scripts/entities/` (`entities-drift`; any Python from 3.8 writes the same bytes). The tooling's build scripts stamp only what they declare to cargo (`docs/policies/build_codegen_policy.md`)

### What this repository cannot have, and what stands in

- **No merge queue.** GitHub offers one only to repositories an organization owns; this one belongs to a user account. So the heavy lanes of `pr-fast.yml` (clippy, tests, rustdoc, deny and vet, Windows clippy) run on the pull request itself, and the ruleset requires their aggregate. Moving the repository to an organization would make the queue available; `scripts/ci/main-ruleset.json` would then gain the `merge_queue` rule.

### Pending (needs a human decision)

- [ ] **Releases.** Every release, packaging and preview lane is installed and switched off (section 5). The owner decides when the first one runs.
- [ ] **The release tag: signed or the workflow's.** v0.6.0 to v0.7.7 are tags signed by hand. With `LANE_RELEASE` on, merging `just ship`'s `release/vX.Y.Z` pull request makes `release.yml` create the tag itself, unsigned; `just release-tag` makes a signed one, but only when that dispatch did not run (`docs/policies/release_policy.md`, step 8). Keep both, or drop the automatic dispatch so every release tag is signed.
- [ ] **The version promise's open points.** `docs/policies/release_policy.md` writes down what the code and the contract test hold today; for the owner to confirm or change: whether a new `--check` warning may come in a patch release (today: yes, as an addition), whether a page's heading ids and anchors are promised across versions (today: no, they are the page's inside), the minimum Rust's rise rule (a minor release, a Rust stable for six months), and when md2html is 1.0. `cargo install --git` in the README installs `main`, not a release.
- [ ] **A native reader for the translations.** The sixteen translations of the capabilities page were made by translation, and their coined terms want a native speaker's eye.
- [ ] **zizmor's last three findings.** The `workflow_run` trigger of `auto-rerun-transient.yml` and `ci-failure-notify.yml` (nothing else runs after another workflow ends; both read no pull-request code), and `dependabot-auto-merge.yml`'s local reusable-workflow call, whose `$/` spelling actionlint does not accept yet. Until they have an answer, zizmor is run by hand at each workflow change, not as a gate.
- [ ] **`release-cache-warm.yml`.** The release builds no longer restore a cache (a pull request could have written it), so the weekly warm-up no longer shortens a release; it still builds the release targets on `main`. Keep it as that check or retire it.
- [ ] **What the security review left for a decision** (2026-11-24). (1) A document may name files in the folder md2html runs in as well as its own folder (`SECURITY.md`, guarantee 2): run from the home folder, any `.css`, `.html` or `.md` under it may be read into a page. Keep the allowance, drop it, or refuse hidden parts of the path. (2) The shared glossary's link is the absolute `file:` path of the glossary's page, so a published page carries the home folder's path (`/Users/<name>/...`) unless `MD2HTML_GLOSSARY_URL` is set; link it only through that variable, or keep it. (3) Secret scanning's non-provider patterns and validity checks are off. (4) The deprecation rule in the release policy is new: confirm it with the version promise's other open points.
- [ ] **Require the `epub` check on GitHub.** `scripts/ci/main-ruleset.json` lists CI's new `epub` job (the 20 books through epubcheck) among the required checks; `just protect-main` applies it to the live ruleset.

---

## 2. The rhythm (daily)

```bash
git switch -c feat/<topic>      # never work on main
# edit
just check                      # seconds: compile, format, file size
just lint-ci                    # clippy, as CI runs it
just test                       # nextest suite
just go                         # the definition of done
git add -A && git commit -m "feat: ..."   # hooks run, seconds
git push -u origin feat/<topic>           # gate battery, ~30 s
just pr "feat: ..."             # validated title, opens the PR
just merge <PR-number>          # watches every check, then merges (never raw gh pr merge)
```

- Gate fails? `AGENTS.md` section 6 has a row for every gate (the gate `doc-references` holds it to `scripts/ci/gates.toml`); `docs/forge/GETTING-STARTED.md` the same fixes for a person. Fix the cause, never bypass.
- Conventional Commits, one scope at most; every commit signed; merge commits only.
- Weekly, automatic (Mondays): the tier-2 deep suite (coverage, cargo-careful, Miri on the parsers' and helpers' unit tests, mutation testing of the modules in `scripts/ci/mutants-scope.txt`: the sanitizer, the text helpers, the inline parser, the block reader and the document assembly, every mutant caught), the vet refresh, the nightly-toolchain canary. A failure files an issue.
- Dependabot opens grouped pull requests on Mondays. A patch-level bump merges itself once every required check is green; a cargo bump first needs the audit store to know the new versions (`cargo vet`, then commit `supply-chain/imports.lock` to the PR branch).

**Reading order for a session:** `AGENTS.md` → this file → `docs/capabilities.md` for what the product does. For a person new to the code: `README.md` → `docs/CONTRIBUTING.md` (and `docs/forge/GETTING-STARTED.md` to set the machine up) → `docs/ARCHITECTURE.md` → `docs/policies/` → `docs/TESTING.md`.

---

## 3. The lint table, held with one exception (since 2026-11-23)

Every Rust file of the workspace, the tooling and the tests included, meets the whole lint table in the root `Cargo.toml` and `clippy.toml`, with no `#[allow]` and no `#[expect]` but the one below. A finding is fixed at its cause, usually the type or the structure it points at (`AGENTS.md`, `docs/policies/lint-posture.md`).

The one exception, decided by the owner on 2026-11-23: `clippy::float_arithmetic` in `crates/md2html/src/mermaid/geom.rs`, the arithmetic of the diagram geometry. Everything else in the diagrams works through its `Len`, `Point`, `Size` and `Rect` types.

`scripts/ci/anti_pattern_gate.sh` (gate `anti-pattern`: every commit, every push, every pull request; its own cases are gate `gate-tests`) refuses any other lint exception in any Rust file, lossy decoding of bytes or names (`from_utf8_lossy`, `from_utf16_lossy`, `to_string_lossy`) in production code, and discarded writes. There is no marker that silences it, no exceptions list for file sizes (every Rust file is under 800 lines), no soft gate and no bypass variable.

How the code got there: on 2026-11-22 the table was switched on over 2,051 findings, recorded as debt. On 2026-11-23 every module was rebuilt so that the findings' causes are gone: a shared reader (`crate::scan`) instead of hand-kept positions into text, the page model as one struct per kind (`model.rs`), typed ids and records instead of parallel vectors, named types instead of tuples and `bool` bundles, long functions as named steps, one way to report output. Every change was checked against a differential test of 8,298 documents in six modes (49,788 renders), which came out byte for byte the same apart from one deliberate fix (a stray comma in two button titles).

---

## 4. What's next

1. **Wider mutation testing.** The weekly mutation job is required and covers the sanitizer, the text helpers, the inline parser, the block reader and the document assembly, where every mutant is caught. Widen it module by module (`scripts/ci/mutants-scope.txt`), each once its tests catch every mutant: `docs/TESTING.md` has the next modules (`serve`, `render`), their mutant counts, run times and how many are still missed.
2. **The first release** with binaries, when the owner decides: section 5, `lane:release`, and the checklist in `docs/policies/release_policy.md`.
3. **Faster CI, the owner's decision.** A pull request's checks spend most of their runner time before any check runs (measured on the runs of PR #47): clearing disk space takes 50 to 180 seconds in each of six jobs, installing the pinned toolchain with all its components and five targets 20 to 30 seconds in each of thirteen, and the `ci` workflow's supply-chain job compiles cargo-deny and cargo-vet (180 seconds) where PR Fast CI installs them prebuilt. The tooling the drift gates run is built with the release profile and its LTO (12 seconds from clean, 5 without; `docs/policies/build_codegen_policy.md`), and `ci`'s tests run with it too. Each would change what CI runs, so each waits for the owner. The tests themselves take 12 to 17 seconds and gain nothing from being split.

---

## 5. Which switch does what (dormant lanes)

Everything below is **installed and dormant**; enabling is a repository variable plus, at most, a secret. Full runbooks with verify steps: `docs/forge/COMPONENTS.md`. Nothing here runs, and nothing here costs anything, until its switch is flipped.

| Lane | Turns on | How (short form) | When |
|---|---|---|---|
| `lane:release` | GitHub release binaries (archives + SHA256SUMS) | `gh variable set LANE_RELEASE --body true`, then `just ship` | First release. `just release-local` builds the same archives here, for nothing |
| `lane:slsa` | build provenance attestation on release artifacts (no secrets) | `gh variable set LANE_SLSA --body true` | Same moment as `lane:release` |
| `lane:release-plz` | the crates.io publish job, for the tag `v<version>` a release makes; it never bumps a version or writes the changelog (`release-plz.toml`) | `gh variable set LANE_RELEASE_PLZ --body true` (with `lane:crates-publish`) | With `lane:crates-publish` |
| `lane:preview` | real binaries for one PR's commit, with a SHA256 manifest | label the PR `preview-artifacts` (+ `preview-macos`) or dispatch `preview-artifacts.yml` | When someone must try a build before merge |
| `lane:brew` | Homebrew tap formula per release | `LANE_BREW=true`, `HOMEBREW_TAP_TOKEN`, a tap repository | After releases exist |
| `lane:winget` | Windows package manager PRs per release | `WINGET_TOKEN` secret, `LANE_WINGET=true`, one manual first submission | After releases exist |
| `lane:crates-publish` | crates.io publishing + weekly dry-run guard | reserve the name, make this repository its trusted publisher on crates.io (no token is stored: the publish job signs in with OIDC), `LANE_CRATES`, `LANE_RELEASE_PLZ` and `ENABLE_CRATES_IO_PUBLISH` = `true` | If md2html is to be `cargo install md2html` |
| `lane:codecov` | coverage upload + PR comments | `CODECOV_TOKEN` secret, `LANE_CODECOV=true` | When coverage numbers are wanted on PRs |
| `lane:codesign` | stable macOS binary identity; Developer ID + notarized releases | `just setup-codesign`; for releases the `APPLE_*` secrets (`docs/forge/CODE-SIGNING.md`) | With `lane:release`, for Macs |

Always on: the main guard (`just merge`, the pre-push hook, `main-merge-guard.yml`, the ruleset), CodeQL (the repository is public), the cross-platform lint gates at pre-push.

The release workflow has never run here. It was read against the release checklist on 2026-11-24 and what can run without GitHub was run (`docs/policies/release_policy.md`, the last section, says what only a real run proves). Its SBOM step writes one `sbom-<crate>.cdx.json` for each crate under `crates/` (`sbom-md2html.cdx.json` for the product, `sbom-md2html-version.cdx.json` for the tooling's version helper).

---

## 6. Doc map (which doc for what)

| Question | Doc |
|---|---|
| What does md2html do? | `docs/capabilities.md`, `README.md` |
| What does the command line promise (options, settings, exit codes)? | `docs/command-line.md` |
| How is it tested, and where does each test run? | `docs/TESTING.md` |
| How are md2html's modules arranged? | `docs/ARCHITECTURE.md` |
| What are the rules? | `AGENTS.md` (agents), `docs/CONTRIBUTING.md` (people) |
| How do I fix a gate? | `AGENTS.md` section 6, `docs/forge/GETTING-STARTED.md` |
| Where are we, what's next, which switch? | **this file** |
| How do I grow the project (crates, lanes)? | `docs/forge/COMPONENTS.md` |
| Why is this lint, panic or dependency rule strict? | `docs/policies/*.md` |
| What is promised about safety? | `SECURITY.md`, `docs/SUPPLY-CHAIN.md` |
| What does a version promise? How is a release made? | `docs/policies/release_policy.md` |
| Release mechanics | `scripts/ci-pipeline/src/` + `release-plz.toml` |
| Upgrading the scaffolding | `docs/forge/ADOPTING.md` (`just forge-upgrade`); `docs/forge/README.md` lists the template's documents |
| The research behind the design (md2html 0.3.3, before the rebuild) | `reports/Markdown to HTML state of the art.md` and its notes in `research_notes/` |
