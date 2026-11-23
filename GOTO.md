<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# GOTO.md: where md2html stands, what's next, which switch does what

The **living state doc** for this project. `AGENTS.md` holds the rules and `docs/forge/GETTING-STARTED.md` the how-to; this file holds the *state*: where the project stands, the rhythm for daily work, the debt that is recorded, and the runbook for every dormant capability.

**Maintenance rule:** this file is a snapshot, not eternal truth. When a checkbox flips, a lane goes live, or the plan changes, update it **in the same PR**. A stale GOTO.md is worse than none.

---

## 1. Where we are (snapshot: 2026-11-22)

### The product

md2html 0.7.6: Markdown to one self-contained HTML page (`docs/capabilities.md` is the tour, in seventeen languages). One crate, `crates/md2html`, no dependencies, unsafe code forbidden. 640 of the 652 CommonMark examples pass; that number is a floor.

### The machine

- [x] Adopted the rust-forge scaffolding (template 1.1.3) with `adopt.sh`; merged as PR #1 on 2026-11-22
- [x] Gate tools installed, git hooks wired (`core.hooksPath=scripts/hooks`)
- [x] Commit signing: GPG key `E406D32B4736D09F`, repo-local; `just doctor-signing` green
- [x] `just go` green (the definition of done for any change)
- [x] GitHub-side state (`bash scripts/ci/bootstrap-github.sh`): labels, every `LANE_*` variable `false`, merge commits only (squash and rebase off), auto-merge, delete-branch-on-merge, `tag-protection-v-prefix`
- [x] `main-protection` ruleset active: no deletion, no force push, signed commits, a pull request with every thread resolved, merge commits only, and the required checks (`scripts/ci/main-ruleset.json`)
- [x] The whole lint table of the template at its own levels, with the findings that predate it recorded (section 3)
- [x] `just use-local` and `just release-local`: md2html for six platforms from one machine
- [x] The audit store (`supply-chain/`) stands on its own: no import from another repository
- [x] Dependencies current (2026-11-22): toolchain `nightly-2026-11-23` (chosen by `just toolchain-sync`), the tooling's crates at their latest (90 crates: 64 fully audited, 17 partly, 9 exempted; no advisory ignored), every action at its latest release that is a week old, the browser tools at their latest
- [x] Secret scanning with push protection, Dependabot alerts and security updates, private vulnerability reporting: all on

### What this repository cannot have, and what stands in

- **No merge queue.** GitHub offers one only to repositories an organization owns; this one belongs to a user account. So the heavy lanes of `pr-fast.yml` (clippy, tests, rustdoc, deny and vet, Windows clippy) run on the pull request itself, and the ruleset requires their aggregate. Moving the repository to an organization would make the queue available; `scripts/ci/main-ruleset.json` would then gain the `merge_queue` rule.

### Pending (needs a human decision)

- [ ] **Releases.** Every release, packaging and preview lane is installed and switched off (section 5). The owner decides when the first one runs.
- [ ] **A native reader for the translations.** The sixteen translations of the capabilities page were made by translation, and their coined terms want a native speaker's eye.
- [ ] **The nine exemptions** in `supply-chain/config.toml` (`cargo vet suggest` lists them by size): each is a crate version nobody has audited yet. They shrink as audits are written or imported.
- [ ] **EPUB validation.** md2html's EPUBs have not been run through epubcheck.

---

## 2. The rhythm (daily)

```bash
git switch -c feat/<topic>      # never work on main
# edit
just check                      # seconds: compile + lint
just test                       # nextest suite
just go                         # the definition of done
git add -A && git commit -m "feat: ..."   # hooks run, seconds
git push -u origin feat/<topic>           # gate battery, ~30 s
just pr "feat: ..."             # validated title, opens the PR
just merge <PR-number>          # watches every check, then merges (never raw gh pr merge)
```

- Gate fails? `docs/forge/GETTING-STARTED.md` has the fix-it table; `AGENTS.md` section 6 the agent version. Fix the cause, never bypass.
- Conventional Commits, one scope at most; every commit signed; merge commits only.
- Weekly, automatic (Mondays): the tier-2 deep suite (coverage, cargo-careful, Miri on the parsing and text modules, mutation testing of `sanitize.rs` and `text.rs`), the vet refresh, the nightly-toolchain canary. A failure files an issue.
- Dependabot opens grouped pull requests on Mondays. A patch-level bump merges itself once every required check is green; a cargo bump first needs the audit store to know the new versions (`cargo vet`, then commit `supply-chain/imports.lock` to the PR branch).

**Reading order for a session:** `AGENTS.md` → this file → `docs/capabilities.md` for what the product does.

---

## 3. Lint debt (recorded 2026-11-22)

The template's whole lint table is on at the template's levels. New code meets it. md2html's code predates it, and the 2051 findings it had on the day the table was switched on are recorded as `#![expect(...)]` blocks at the top of each file in `crates/md2html/src`, with the reason "adoption debt".

An expectation that is no longer met fails the build, so the debt only goes down: clean a lint out of a file, remove it from that file's block, and the gate holds the file to it from then on. Nothing may be added to a block.

| Lint | Findings |
| --- | ---: |
| `clippy::min_ident_chars` | 559 |
| `clippy::indexing_slicing` | 425 |
| `clippy::float_arithmetic` | 228 |
| `clippy::default_numeric_fallback` | 216 |
| `clippy::string_slice` | 187 |
| `clippy::let_underscore_must_use` | 100 |
| `clippy::shadow_reuse` | 76 |
| `clippy::rest_pattern_accessible_field` | 67 |
| `clippy::unseparated_literal_suffix` | 51 |
| `clippy::shadow_unrelated` | 43 |
| `clippy::let_underscore_untyped` | 18 |
| `clippy::print_stderr` | 15 |
| `clippy::wildcard_enum_match_arm` | 14 |
| `clippy::cognitive_complexity` | 7 |
| `clippy::string_add` | 7 |
| `clippy::print_stdout` | 6 |
| `clippy::std_instead_of_core` | 5 |
| `clippy::map_err_ignore` | 4 |
| `clippy::mixed_read_write_in_expression` | 4 |
| `clippy::redundant_type_annotations` | 4 |
| `clippy::shadow_same` | 4 |
| `clippy::missing_asserts_for_indexing` | 3 |
| `clippy::infinite_loop` | 2 |
| `clippy::std_instead_of_alloc` | 2 |
| `clippy::assertions_on_result_states` | 1 |
| `elided_lifetimes_in_paths` | 1 |
| `clippy::if_then_some_else_none` | 1 |
| `clippy::allow_attributes` | 1 |
| **Total** | **2051** |

More than half is in `mermaid.rs` (the diagram layout: short names for coordinates, indexing, floating-point arithmetic). Some of these lints are a matter of judgement for geometry and scanning code; where the answer is "this is how the domain writes it", the lint leaves the debt block for a scoped `#[expect]` with that reason, or `clippy.toml` gains the allowed names.

---

## 4. What's next

1. **The code**: burn down section 3, module by module, and widen the mutation testing in `tier-2.yml` beyond `sanitize.rs` and `text.rs` as each module's tests are shown to hold (all of md2html is 4,534 mutants).
2. **The first release**, when the owner decides: section 5, `lane:release`.

---

## 5. Which switch does what (dormant lanes)

Everything below is **installed and dormant**; enabling is a repository variable plus, at most, a secret. Full runbooks with verify steps: `docs/forge/COMPONENTS.md`. Nothing here runs, and nothing here costs anything, until its switch is flipped.

| Lane | Turns on | How (short form) | When |
|---|---|---|---|
| `lane:release` | GitHub release binaries (archives + SHA256SUMS) | `gh variable set LANE_RELEASE --body true`, then `just ship` | First release. `just release-local` builds the same archives here, for nothing |
| `lane:slsa` | build provenance attestation on release artifacts (no secrets) | `gh variable set LANE_SLSA --body true` | Same moment as `lane:release` |
| `lane:release-plz` | automated version and changelog PRs on main | `gh variable set LANE_RELEASE_PLZ --body true` | An alternative to `just ship`-driven versioning; pick one driver. md2html's changelog is written by hand today |
| `lane:preview` | real binaries for one PR's commit, with a SHA256 manifest | label the PR `preview-artifacts` (+ `preview-macos`) or dispatch `preview-artifacts.yml` | When someone must try a build before merge |
| `lane:brew` | Homebrew tap formula per release | `LANE_BREW=true`, `HOMEBREW_TAP_TOKEN`, a tap repository | After releases exist |
| `lane:winget` | Windows package manager PRs per release | `WINGET_TOKEN` secret, `LANE_WINGET=true`, one manual first submission | After releases exist |
| `lane:crates-publish` | crates.io publishing + weekly dry-run guard | reserve the name, `CARGO_REGISTRY_TOKEN` secret, `LANE_CRATES=true` | If md2html is to be `cargo install md2html` |
| `lane:codecov` | coverage upload + PR comments | `CODECOV_TOKEN` secret, `LANE_CODECOV=true` | When coverage numbers are wanted on PRs |
| `lane:codesign` | stable macOS binary identity; Developer ID + notarized releases | `just setup-codesign`; for releases the `APPLE_*` secrets (`docs/forge/CODE-SIGNING.md`) | With `lane:release`, for Macs |

Always on: the main guard (`just merge`, the pre-push hook, `main-merge-guard.yml`, the ruleset), CodeQL (the repository is public), the cross-platform lint gates at pre-push.

Before the first release, check the release workflow against md2html once: it was written for the template's sample crates and has never run here (the SBOM step still names `sbom-md2html-cli.cdx.json`).

---

## 6. Doc map (which doc for what)

| Question | Doc |
|---|---|
| What does md2html do? | `docs/capabilities.md`, `README.md` |
| What are the rules? | `AGENTS.md` (agents), `docs/CONTRIBUTING.md` (people) |
| How do I fix a gate? | `docs/forge/GETTING-STARTED.md` |
| Where are we, what's next, which switch? | **this file** |
| How do I grow the project (crates, lanes)? | `docs/forge/COMPONENTS.md` |
| Why is this lint, panic or dependency rule strict? | `docs/policies/*.md` |
| What is promised about safety? | `SECURITY.md`, `docs/SUPPLY-CHAIN.md` |
| Release mechanics | `scripts/ci-pipeline/src/` + `release-plz.toml` |
| Upgrading the scaffolding | `docs/forge/ADOPTING.md` (`just forge-upgrade`) |
