<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# GOTO.md: where md2html stands, what's next, which switch does what

The **living state doc** for this project. `AGENTS.md` holds the rules and `docs/forge/GETTING-STARTED.md` the how-to; this file holds the *state*: where the project stands, the rhythm for daily work, how the code is held to the lint table, and the runbook for every dormant capability.

**Maintenance rule:** this file is a snapshot, not eternal truth. When a checkbox flips, a lane goes live, or the plan changes, update it **in the same PR**. A stale GOTO.md is worse than none.

---

## 1. Where we are (snapshot: 2026-11-23)

### The product

md2html 0.7.7: Markdown to one self-contained HTML page (`docs/capabilities.md` is the tour, in seventeen languages). One crate, `crates/md2html`, no dependencies, unsafe code forbidden. All 652 CommonMark examples pass; that number is a floor. Its twenty modules stand in nine layers, each using only its own or lower ones and none in a circle (`docs/ARCHITECTURE.md`); a test holds it. Every item is visible only as far as the code that uses it (private, its parent module, or the crate).

### The machine

- [x] Adopted the rust-forge scaffolding (template 1.1.3) with `adopt.sh`; merged as PR #1 on 2026-11-22
- [x] Gate tools installed, git hooks wired (`core.hooksPath=scripts/hooks`)
- [x] Commit signing: GPG key `E406D32B4736D09F`, repo-local; `just doctor-signing` green
- [x] `just go` green (the definition of done for any change)
- [x] GitHub-side state (`bash scripts/ci/bootstrap-github.sh`): labels, every `LANE_*` variable `false`, merge commits only (squash and rebase off), auto-merge, delete-branch-on-merge, `tag-protection-v-prefix`
- [x] `main-protection` ruleset active: no deletion, no force push, signed commits, a pull request with every thread resolved, merge commits only, and the required checks (`scripts/ci/main-ruleset.json`)
- [x] The whole lint table at its own levels, held without exceptions (section 3)
- [x] `just use-local` and `just release-local`: md2html for six platforms from one machine
- [x] The audit store (`supply-chain/`) stands on its own: no import from another repository
- [x] Dependencies current (2026-11-22): toolchain `nightly-2026-11-23` (chosen by `just toolchain-sync`), the tooling's crates at their latest (28 crates since 2026-11-23, all fully audited, none exempted; no advisory ignored), every action at its latest release that is a week old, the browser tools at their latest
- [x] Secret scanning with push protection, Dependabot alerts and security updates, private vulnerability reporting: all on
- [x] Every EPUB md2html writes for the repository's documents passes epubcheck 5.4.0 with no error and no warning: CI's `epub` job checks the 19 books, `just epubcheck` the same locally

### What this repository cannot have, and what stands in

- **No merge queue.** GitHub offers one only to repositories an organization owns; this one belongs to a user account. So the heavy lanes of `pr-fast.yml` (clippy, tests, rustdoc, deny and vet, Windows clippy) run on the pull request itself, and the ruleset requires their aggregate. Moving the repository to an organization would make the queue available; `scripts/ci/main-ruleset.json` would then gain the `merge_queue` rule.

### Pending (needs a human decision)

- [ ] **Releases.** Every release, packaging and preview lane is installed and switched off (section 5). The owner decides when the first one runs.
- [ ] **A native reader for the translations.** The sixteen translations of the capabilities page were made by translation, and their coined terms want a native speaker's eye.
- [ ] **Require the `epub` check on GitHub.** `scripts/ci/main-ruleset.json` lists CI's new `epub` job (the 19 books through epubcheck) among the required checks; `just protect-main` applies it to the live ruleset.

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
- Weekly, automatic (Mondays): the tier-2 deep suite (coverage, cargo-careful, Miri on the parsers' and helpers' unit tests, mutation testing of the sanitizer and the text helpers, every mutant caught), the vet refresh, the nightly-toolchain canary. A failure files an issue.
- Dependabot opens grouped pull requests on Mondays. A patch-level bump merges itself once every required check is green; a cargo bump first needs the audit store to know the new versions (`cargo vet`, then commit `supply-chain/imports.lock` to the PR branch).

**Reading order for a session:** `AGENTS.md` → this file → `docs/capabilities.md` for what the product does.

---

## 3. The lint table, held without exceptions (since 2026-11-23)

Every Rust file of the workspace, the tooling and the tests included, meets the whole lint table in the root `Cargo.toml` and `clippy.toml`, with no `#[allow]` and no `#[expect]`. A finding is fixed at its cause, usually the type or the structure it points at (`AGENTS.md`, `docs/policies/lint-posture.md`).

The one exception, decided by the owner on 2026-11-23: `clippy::float_arithmetic` in `crates/md2html/src/mermaid/geom.rs`, the arithmetic of the diagram geometry. Everything else in the diagrams works through its `Len`, `Point`, `Size` and `Rect` types.

`scripts/ci/anti_pattern_gate.sh` (in `just go` and in CI) refuses any other lint exception in any Rust file, lossy text decoding in production code, and discarded writes. There is no marker that silences it, no exceptions list for file sizes (every Rust file is under 800 lines), no soft gate and no bypass variable.

How the code got there: on 2026-11-22 the table was switched on over 2,051 findings, recorded as debt. On 2026-11-23 every module was rebuilt so that the findings' causes are gone: a shared reader (`crate::scan`) instead of hand-kept positions into text, the page model as one struct per kind (`model.rs`), typed ids and records instead of parallel vectors, named types instead of tuples and `bool` bundles, long functions as named steps, one way to report output. Every change was checked against a differential test of 8,298 documents in six modes (49,788 renders), which came out byte for byte the same apart from one deliberate fix (a stray comma in two button titles).

---

## 4. What's next

1. **Wider mutation testing.** The weekly mutation job is required and covers the sanitizer and the text helpers, where every mutant is caught. Widen it module by module (`-f` in `tier-2.yml`), each once its tests catch every mutant (all of md2html is several thousand mutants).
2. **The first release** with binaries, when the owner decides: section 5, `lane:release`.

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

Before the first release, check the release workflow against md2html once: it was written for the template's sample crates and has never run here. Its SBOM step writes one `sbom-<crate>.cdx.json` per workspace crate (`sbom-md2html.cdx.json` for the product).

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
